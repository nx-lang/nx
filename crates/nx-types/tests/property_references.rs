//! Type checking of derived `T.Property` unions: their cases, how they behave as constant unions in
//! type, expression, pattern, and bare-name positions, and the contextual bare `Property` inside a
//! component. Covers the first three requirements of the property-references capability.
//!
//! The imported-record scenario needs a library on disk and lives with the other library tests in
//! `nx-api`, which owns the library registry.

use nx_types::{check_str, TypeCheckResult};

fn diagnostics(result: &TypeCheckResult) -> Vec<(Option<&str>, String)> {
    result
        .diagnostics
        .iter()
        .map(|diagnostic| (diagnostic.code(), diagnostic.message().to_string()))
        .collect()
}

fn assert_ok(source: &str, file_name: &str) -> TypeCheckResult {
    let result = check_str(source, file_name);
    assert!(
        result.errors().is_empty(),
        "Expected '{}' to type check, got {:?}",
        file_name,
        diagnostics(&result)
    );
    result
}

fn assert_error(source: &str, file_name: &str, code: &str, message_part: &str) -> TypeCheckResult {
    let result = check_str(source, file_name);
    assert!(
        result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code() == Some(code) && diagnostic.message().contains(message_part)
        }),
        "Expected '{}' to report {} containing {:?}, got {:?}",
        file_name,
        code,
        message_part,
        diagnostics(&result)
    );
    result
}

fn assert_message(source: &str, file_name: &str, message_part: &str) {
    let result = check_str(source, file_name);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message().contains(message_part)),
        "Expected '{}' to report a diagnostic containing {:?}, got {:?}",
        file_name,
        message_part,
        diagnostics(&result)
    );
}

fn property_union_cases(result: &TypeCheckResult, name: &str) -> Vec<String> {
    let module = result.lowered_module.as_ref().expect("lowered module");
    let Some(nx_hir::Item::Union(union)) = module.find_item(name) else {
        panic!("Expected a {} union", name);
    };
    assert!(union.property_target().is_some());
    assert!(union.is_constant_union());
    union
        .cases
        .iter()
        .map(|case| case.name.as_str().to_string())
        .collect()
}

fn value_type(result: &TypeCheckResult, name: &str) -> String {
    result
        .type_env
        .lookup(&nx_hir::Name::new(name))
        .map(|ty| ty.to_string())
        .unwrap_or_else(|| panic!("Expected '{}' to be typed", name))
}

// ============================================================================
// Every record-shaped declaration has a derived property union
// ============================================================================

#[test]
fn property_union_of_a_plain_record_lists_its_fields() {
    let result = assert_ok(
        r#"
        type User = { name:string email:string? }
        let key: User.Property = {User.Property.email}
        "#,
        "property-plain.nx",
    );
    assert_eq!(
        property_union_cases(&result, "User.Property"),
        ["name", "email"]
    );
    assert_eq!(value_type(&result, "key"), "User.Property");
}

#[test]
fn property_union_of_a_component_is_derived_from_state() {
    let result = assert_ok(
        r#"
        component <Counter step:int = 1 /> = { state { count:int = 0 } <Label text={count} /> }
        let key: Counter.Property = {Counter.Property.count}
        "#,
        "property-component.nx",
    );
    assert_eq!(property_union_cases(&result, "Counter.Property"), ["count"]);

    assert_error(
        r#"
        component <Counter step:int = 1 /> = { state { count:int = 0 } <Label text={count} /> }
        let key = {Counter.Property.step}
        "#,
        "property-component-prop.nx",
        "undefined-union-case",
        "step",
    );
}

#[test]
fn property_union_includes_inherited_fields_in_declaration_order() {
    let result = assert_ok(
        r#"
        abstract type Named = { name:string }
        type User extends Named = { email:string }
        let keys: User.Property[] = { User.Property.name User.Property.email }
        "#,
        "property-inherited.nx",
    );
    assert_eq!(
        property_union_cases(&result, "User.Property"),
        ["name", "email"]
    );
}

#[test]
fn property_union_of_an_action_and_an_inline_emit_exist() {
    assert_ok(
        r#"
        action Rename = { name:string }
        component <Form emits { Submit { value:string } } /> = { <Panel /> }
        let a: Rename.Property = {Rename.Property.name}
        let b: Form.Submit.Property = {Form.Submit.Property.value}
        "#,
        "property-action.nx",
    );
}

#[test]
fn nested_derived_names_do_not_exist() {
    assert_error(
        r#"
        type User = { name:string }
        let a = {User.Property.Property.name}
        "#,
        "property-property.nx",
        "unknown-property-union",
        "'User.Property' is a property union, and property unions have no property union of their own",
    );
    assert_error(
        r#"
        type User = { name:string }
        let b = {User.Update.Property.name}
        "#,
        "update-property.nx",
        "unknown-property-union",
        "'User.Update' is an update record, and update records have no property union",
    );
    assert_error(
        r#"
        type User = { name:string }
        let c = <User.Property.Update name="Ada" />
        "#,
        "property-update.nx",
        "unknown-update-record",
        "'User.Property' is a property union, and property unions have no update record",
    );
}

#[test]
fn property_union_for_an_unknown_target_is_rejected() {
    assert_error(
        r#"
        let key = {Icons.Property.size}
        "#,
        "property-unknown.nx",
        "unknown-property-union",
        "'Icons' is not a record, action, or component with state",
    );
}

#[test]
fn a_component_without_state_has_no_property_union() {
    assert_error(
        r#"
        component <Form /> = { <Panel /> }
        let key = {Form.Property.value}
        "#,
        "property-stateless.nx",
        "unknown-property-union",
        "component 'Form' declares no state, so it has no property union",
    );
}

#[test]
fn property_union_cannot_be_extended() {
    assert_message(
        r#"
        type User = { name:string }
        type Extra extends User.Property = | more
        "#,
        "property-extends.nx",
        "'User.Property' is a derived property union and cannot be extended",
    );
}

// ============================================================================
// A property union behaves as a constant union everywhere
// ============================================================================

#[test]
fn bare_case_name_resolves_at_a_property_typed_site() {
    let result = assert_ok(
        r#"
        type Contact = { title:string subtitle:string }
        component <Table sortBy:Contact.Property /> = { <div /> }
        let v = <Table sortBy=subtitle />
        "#,
        "property-bare.nx",
    );
    // The bare name was rewritten to the qualified case, so nothing downstream sees it.
    let module = result.lowered_module.expect("lowered module");
    let resolved = module.exprs().any(|(_, expr)| {
        matches!(
            expr,
            nx_hir::ast::Expr::ResolvedUnionCase { union, case, .. }
                if union.as_str() == "Contact.Property" && case.as_str() == "subtitle"
        )
    });
    assert!(
        resolved,
        "Expected the bare 'subtitle' to resolve to Contact.Property.subtitle"
    );
}

#[test]
fn bare_case_names_resolve_at_a_list_typed_site() {
    assert_ok(
        r#"
        type Contact = { title:string subtitle:string }
        component <Table columns:Contact.Property[] /> = { <div /> }
        let v = <Table columns=title />
        "#,
        "property-bare-list.nx",
    );
}

#[test]
fn unknown_case_at_a_property_typed_site_names_the_candidates() {
    let result = assert_error(
        r#"
        type Contact = { title:string subtitle:string }
        component <Table sortBy:Contact.Property /> = { <div /> }
        let v = <Table sortBy=titel />
        "#,
        "property-bare-unknown.nx",
        "unresolved-contextual-name",
        "titel",
    );
    let message = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == Some("unresolved-contextual-name"))
        .map(|diagnostic| diagnostic.message().to_string())
        .expect("diagnostic");
    assert!(
        message.contains("title") && message.contains("subtitle"),
        "{}",
        message
    );
}

#[test]
fn match_over_a_property_union_is_checked_for_exhaustiveness() {
    assert_ok(
        r#"
        type User = { name:string email:string? }
        let label(key:User.Property) = {if key is { name => "Name" email => "Email" }}
        "#,
        "property-match.nx",
    );
    assert_message(
        r#"
        type User = { name:string email:string? }
        let label(key:User.Property) = {if key is { name => "Name" }}
        "#,
        "property-match-partial.nx",
        "email",
    );
}

#[test]
fn qualified_case_pattern_is_accepted_in_a_match() {
    assert_ok(
        r#"
        type User = { name:string }
        let label(key:User.Property) = {if key is { User.Property.name => "Name" }}
        "#,
        "property-match-qualified.nx",
    );
}

#[test]
fn property_unions_of_different_records_are_distinct_types() {
    assert_error(
        r#"
        type User = { name:string }
        type Team = { name:string }
        let key: User.Property = {Team.Property.name}
        "#,
        "property-distinct.nx",
        "value-type-mismatch",
        "Team.Property",
    );
}

#[test]
fn property_union_values_can_be_compared() {
    let result = assert_ok(
        r#"
        type User = { name:string email:string? }
        let same = {User.Property.name == User.Property.name}
        let other = {User.Property.name == User.Property.email}
        "#,
        "property-compare.nx",
    );
    assert_eq!(value_type(&result, "same"), "boolean");
    assert_eq!(value_type(&result, "other"), "boolean");
}

/// Two cases of one union compare for equality only: a union has no order for `<` to follow.
#[test]
fn property_union_values_cannot_be_ordered() {
    for op in ["<", "<=", ">", ">="] {
        assert_error(
            &format!(
                r#"
                type User = {{ name:string email:string? }}
                let ordered = {{User.Property.name {op} User.Property.email}}
                "#
            ),
            "property-ordered.nx",
            "type-mismatch",
            "Cannot compare types",
        );
    }
}

// ============================================================================
// The bare `Property` name resolves to the enclosing component's property union
// ============================================================================

#[test]
fn bare_property_in_a_handler_body_resolves_to_the_component() {
    assert_ok(
        r#"
        external component <Grid emits { Sorted { by:string } } />
        component <People /> = {
          state { sortBy:People.Property = {Property.name} name:string = "" }
          <Grid onSorted=<Update sortBy={Property.name} /> />
        }
        "#,
        "property-handler.nx",
    );
    assert_error(
        r#"
        external component <Grid emits { Sorted { by:string } } />
        component <People /> = {
          state { sortBy:People.Property = {Property.name} name:string = "" }
          <Grid onSorted=<Update sortBy={Property.nickname} /> />
        }
        "#,
        "property-handler-unknown.nx",
        "undefined-union-case",
        "nickname",
    );
}

#[test]
fn bare_property_outside_a_component_is_rejected_with_the_qualified_spelling() {
    assert_error(
        r#"
        type User = { name:string }
        let key = {Property.name}
        "#,
        "property-root.nx",
        "bare-property-outside-component",
        "Type.Property.field",
    );
}

/// The one diagnostic names the qualified spelling; the head `Property` is not also reported as
/// an undefined identifier.
#[test]
fn bare_property_outside_a_component_reports_exactly_one_diagnostic() {
    let result = check_str(
        r#"
        type User = { name:string }
        let key = {Property.name}
        "#,
        "property-root-once.nx",
    );
    let codes = result
        .errors()
        .iter()
        .map(|diagnostic| diagnostic.code())
        .collect::<Vec<_>>();
    assert_eq!(
        codes,
        vec![Some("bare-property-outside-component")],
        "{:?}",
        diagnostics(&result)
    );
}

#[test]
fn bare_property_takes_precedence_over_a_same_named_declaration_inside_a_component() {
    assert_ok(
        r#"
        type Property = { note:string }
        component <Counter /> = {
          state { count:int = 0 key:Counter.Property = {Property.count} }
          <Label />
        }
        "#,
        "property-precedence.nx",
    );
}
