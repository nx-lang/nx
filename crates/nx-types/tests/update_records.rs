//! Type checking of derived `T.Update` records and of the action handlers that return them.
//!
//! Covers the update-records capability (shape, absent-versus-null construction, the bare
//! `Update` tag) and the component-action-handlers requirements that handler bodies are checked
//! and their results routed by type.

use nx_hir::ast;
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

fn assert_error(source: &str, file_name: &str, code: &str, message_part: &str) {
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

// ============================================================================
// Every record-shaped declaration has a derived update record
// ============================================================================

#[test]
fn update_record_of_a_plain_record_has_the_same_fields() {
    let result = assert_ok(
        r#"
        type User = { name:string email:string? }
        let patch = <User.Update name="Ada" />
        let typed(): User.Update = <User.Update email="ada@example.com" />
        "#,
        "update-plain.nx",
    );
    let module = result.lowered_module.expect("lowered module");
    let Some(nx_hir::Item::Record(record)) = module.find_item("User.Update") else {
        panic!("Expected a User.Update record");
    };
    assert_eq!(
        record.update_target().map(|name| name.as_str()),
        Some("User")
    );
}

#[test]
fn update_record_of_a_component_is_derived_from_state() {
    let source = r#"
        component <Counter step:int = 1 /> = { state { count:int = 0 } <Label text={count} /> }
        let reset(): Counter.Update = <Counter.Update count=0 />
    "#;
    assert_ok(source, "update-component.nx");

    assert_error(
        r#"
        component <Counter step:int = 1 /> = { state { count:int = 0 } <Label text={count} /> }
        let bad = <Counter.Update step=2 />
        "#,
        "update-component-prop.nx",
        "unknown-record-field",
        "step",
    );
}

#[test]
fn update_record_includes_inherited_fields() {
    assert_ok(
        r#"
        abstract type Named = { name:string }
        type User extends Named = { email:string }
        let patch = <User.Update name="Ada" email="ada@example.com" />
        "#,
        "update-inherited.nx",
    );
}

#[test]
fn an_abstract_record_has_an_update_record() {
    assert_ok(
        r#"
        abstract type Named = { name:string }
        let patch = <Named.Update name="Ada" />
        "#,
        "abstract-update.nx",
    );
}

#[test]
fn update_record_cannot_be_extended() {
    assert_message(
        r#"
        type User = { name:string }
        action Rename extends User.Update = { }
        "#,
        "update-extends.nx",
        "'User.Update' is a derived update record and cannot be extended",
    );
}

#[test]
fn update_record_of_an_update_record_does_not_exist() {
    assert_error(
        r#"
        type User = { name:string }
        let patch = <User.Update.Update name="Ada" />
        "#,
        "update-update.nx",
        "unknown-update-record",
        "User.Update.Update",
    );
    assert_error(
        r#"
        component <Counter /> = { state { count:int = 0 } <Label /> }
        let patch = <Counter.Update.Update count=1 />
        "#,
        "component-update-update.nx",
        "unknown-update-record",
        "Counter.Update.Update",
    );
}

#[test]
fn an_update_tag_for_an_unknown_target_is_rejected() {
    assert_error(
        r#"
        let icon = <Icons.Update size=3 />
        "#,
        "unknown-update-target.nx",
        "unknown-update-record",
        "'Icons' is not a record, action, or component with state",
    );
}

#[test]
fn a_stateless_component_has_no_update_record() {
    let result = check_str(
        r#"
        external component <Button emits { Tapped } />
        component <Form /> = { <Button onTapped=<Update /> /> }
        "#,
        "stateless-bare-update.nx",
    );
    assert_eq!(
        diagnostics(&result),
        vec![(
            Some("unknown-update-record"),
            "Unknown type 'Form.Update': component 'Form' declares no state, so it has no update record"
                .to_string()
        )]
    );

    assert_error(
        r#"
        component <Form /> = { <Panel /> }
        let patch = <Form.Update />
        "#,
        "stateless-qualified-update.nx",
        "unknown-update-record",
        "component 'Form' declares no state",
    );

    assert_error(
        r#"
        type User = { name:string }
        external component <Button emits { Tapped } />
        component <Form /> = { <Button onTapped=<User.Update name="Ada" /> /> }
        "#,
        "stateless-other-update.nx",
        "handler-update-wrong-target",
        "'Form' declares no state to change",
    );
}

#[test]
fn update_record_is_not_the_record_it_patches() {
    assert_error(
        r#"
        type User = { name:string }
        let user(): User = <User.Update name="Ada" />
        "#,
        "update-not-target.nx",
        "return-type-mismatch",
        "User.Update",
    );
}

// ============================================================================
// An absent field means unchanged and null means set to null
// ============================================================================

#[test]
fn unsupplied_fields_are_not_required_and_defaults_do_not_apply() {
    assert_ok(
        r#"
        type User = { name:string = "anon" email:string? }
        let patch = <User.Update email="ada@example.com" />
        "#,
        "update-absent.nx",
    );
}

#[test]
fn null_is_accepted_for_a_nullable_field() {
    assert_ok(
        r#"
        type User = { name:string email:string? }
        let patch = <User.Update email={null} />
        "#,
        "update-null-nullable.nx",
    );
}

#[test]
fn null_is_rejected_for_a_non_nullable_field() {
    assert_error(
        r#"
        type User = { name:string }
        let patch = <User.Update name={null} />
        "#,
        "update-null-non-nullable.nx",
        "record-field-type-mismatch",
        "name",
    );
}

#[test]
fn unknown_and_mistyped_fields_are_rejected() {
    let source = r#"
        type User = { name:string }
        let a = <User.Update nickname="A" />
        let b = <User.Update name=3 />
    "#;
    assert_error(
        source,
        "update-unknown.nx",
        "unknown-record-field",
        "nickname",
    );
    assert_error(
        source,
        "update-mistyped.nx",
        "record-field-type-mismatch",
        "name",
    );
}

#[test]
fn empty_update_record_is_valid() {
    assert_ok(
        r#"
        type User = { name:string }
        let none = <User.Update />
        "#,
        "update-empty.nx",
    );
}

// ============================================================================
// The bare `Update` tag resolves to the enclosing component's update record
// ============================================================================

/// Returns the record name the first handler bound in `component`'s body constructs.
fn handler_body_record(result: &TypeCheckResult, component: &str) -> String {
    let module = result.lowered_module.as_ref().expect("lowered module");
    let Some(nx_hir::Item::Component(component)) = module.find_item(component) else {
        panic!("Expected component '{}'", component);
    };
    let ast::Expr::Element { element, .. } = module.expr(component.body.expect("body")) else {
        panic!("Expected an element body");
    };
    let value = module.element(*element).properties[0].value;
    let ast::Expr::ActionHandler { body, .. } = module.expr(value) else {
        panic!("Expected an action handler");
    };
    match module.expr(*body) {
        ast::Expr::RecordLiteral { record, .. } => record.as_str().to_string(),
        other => panic!("Expected a record literal, got {:?}", other),
    }
}

#[test]
fn bare_update_in_a_handler_body_resolves_to_the_component() {
    let result = assert_ok(
        r#"
        external component <Button emits { Tapped } />
        component <Counter /> = { state { count:int = 0 } <Button onTapped=<Update count={count + 1} /> /> }
        "#,
        "bare-update-handler.nx",
    );
    assert_eq!(handler_body_record(&result, "Counter"), "Counter.Update");

    assert_error(
        r#"
        external component <Button emits { Tapped } />
        component <Counter /> = { state { count:int = 0 } <Button onTapped=<Update count="one" /> /> }
        "#,
        "bare-update-handler-mismatch.nx",
        "record-field-type-mismatch",
        "count",
    );
}

#[test]
fn bare_update_in_a_state_default_resolves_to_the_component() {
    assert_ok(
        r#"
        component <Editor /> = { state { pending:Editor.Update = <Update /> } <Panel /> }
        "#,
        "bare-update-state-default.nx",
    );
}

#[test]
fn bare_update_outside_a_component_is_rejected_with_the_qualified_spelling() {
    assert_error(
        r#"
        type User = { name:string }
        let patch = <Update name="Ada" />
        "#,
        "bare-update-root.nx",
        "bare-update-outside-component",
        "<Type.Update ... />",
    );
}

#[test]
fn bare_update_takes_precedence_over_a_same_named_declaration_inside_a_component() {
    let result = assert_ok(
        r#"
        type Update = { note:string }
        external component <Button emits { Tapped } />
        component <Counter /> = { state { count:int = 0 } <Button onTapped=<Update count=1 /> /> }
        "#,
        "bare-update-precedence.nx",
    );
    assert_eq!(handler_body_record(&result, "Counter"), "Counter.Update");
}

// ============================================================================
// Handler bodies are type checked in the scope of their binding site
// ============================================================================

#[test]
fn action_payload_fields_are_typed_inside_the_body() {
    assert_ok(
        r#"
        external component <Slider emits { EndChanged { value:float64 } } />
        component <Volume /> = { state { level:float64 = 0.5 } <Slider onEndChanged=<Update level={action.value} /> /> }
        "#,
        "handler-payload.nx",
    );
}

#[test]
fn a_mistyped_update_field_in_a_handler_is_rejected() {
    assert_error(
        r#"
        external component <Slider emits { EndChanged { value:float64 } } />
        component <Volume /> = { state { level:int = 0 } <Slider onEndChanged=<Update level={action.value} /> /> }
        "#,
        "handler-mistyped.nx",
        "record-field-type-mismatch",
        "level",
    );
}

#[test]
fn a_misspelled_state_field_in_a_handler_is_rejected() {
    assert_error(
        r#"
        external component <Button emits { Tapped } />
        component <Counter /> = { state { count:int = 0 } <Button onTapped=<Update cont={count + 1} /> /> }
        "#,
        "handler-misspelled.nx",
        "unknown-record-field",
        "cont",
    );
}

#[test]
fn a_handler_whose_result_is_not_an_action_or_update_is_rejected() {
    assert_error(
        r#"
        external component <Button emits { Tapped } />
        component <Counter /> = { state { count:int = 0 } <Button onTapped={count + 1} /> }
        "#,
        "handler-int-result.nx",
        "handler-result-not-action",
        "int",
    );
}

#[test]
fn enclosing_loop_variables_are_visible_inside_a_handler() {
    assert_ok(
        r#"
        external component <Row emits { Tapped } />
        let remove(items:string[], item:string): string[] = { items }
        component <List /> = {
          state { items:string[] = {} }
          <Column>
            {for item in items { <Row onTapped=<Update items={remove(items, item)} /> /> }}
          </Column>
        }
        "#,
        "handler-loop-variable.nx",
    );
}

#[test]
fn enclosing_let_bindings_are_visible_inside_a_handler() {
    // A block has no local `let`, so the enclosing bindings a handler can see are the module's.
    assert_ok(
        r#"
        external component <Row emits { Tapped } />
        let remove(items:string[], item:string): string[] = { items }
        let first = "x"
        component <List /> = {
          state { items:string[] = {} }
          <Row onTapped=<Update items={remove(items, first)} /> />
        }
        "#,
        "handler-let-binding.nx",
    );
}

#[test]
fn a_match_result_is_routed_arm_by_arm() {
    assert_ok(
        r#"
        action Saved = { }
        action Cleared = { }
        external component <Button emits { Tapped } />
        component <Form emits { Saved Cleared } /> = {
          state { mode:string = "a" }
          <Button onTapped={if mode is { "a" => <Saved /> "b" => <Update mode="a" /> else => <Cleared /> }} />
        }
        "#,
        "handler-match-result.nx",
    );

    assert_error(
        r#"
        action Saved = { }
        action Cleared = { }
        external component <Button emits { Tapped } />
        component <Form emits { Saved } /> = {
          state { mode:string = "a" }
          <Button onTapped={if mode is { "a" => <Saved /> else => <Cleared /> }} />
        }
        "#,
        "handler-match-result-not-emitted.nx",
        "handler-action-not-emitted",
        "Cleared",
    );
}

#[test]
fn an_empty_list_result_is_rejected() {
    assert_error(
        r#"
        external component <Button emits { Tapped } />
        component <Counter /> = { state { count:int = 0 } <Button onTapped={} /> }
        "#,
        "handler-empty-list.nx",
        "handler-result-empty",
        "empty list",
    );
}

// ============================================================================
// Handler results are routed by type to state, parent, or host
// ============================================================================

#[test]
fn update_of_the_enclosing_component_is_accepted() {
    assert_ok(
        r#"
        external component <Button emits { Tapped } />
        component <Counter /> = { state { count:int = 0 } <Button onTapped=<Update count={count + 1} /> /> }
        "#,
        "route-own-update.nx",
    );
}

#[test]
fn emitted_action_is_accepted_inside_a_component() {
    assert_ok(
        r#"
        action Saved = { }
        external component <Button emits { Tapped } />
        component <Form emits { Saved } /> = { <Button onTapped=<Saved /> /> }
        "#,
        "route-emitted.nx",
    );
}

#[test]
fn action_the_component_does_not_emit_is_rejected_with_a_fix() {
    assert_error(
        r#"
        action Saved = { }
        external component <Button emits { Tapped } />
        component <Form /> = { <Button onTapped=<Saved /> /> }
        "#,
        "route-not-emitted.nx",
        "handler-action-not-emitted",
        "add 'Saved' to the component's emits",
    );
}

#[test]
fn update_record_of_another_type_is_rejected_inside_a_component() {
    assert_error(
        r#"
        type User = { name:string }
        external component <Button emits { Tapped } />
        component <Form /> = { state { draft:string = "" } <Button onTapped=<User.Update name="Ada" /> /> }
        "#,
        "route-other-update.nx",
        "handler-update-wrong-target",
        "only 'Form.Update'",
    );
}

#[test]
fn any_action_or_update_record_is_a_host_effect_at_the_root() {
    assert_ok(
        r#"
        type User = { name:string }
        action DoSearch = { search:string }
        component <SearchBox emits { SearchSubmitted { searchString:string } } /> = { <TextInput /> }
        let render() = <SearchBox onSearchSubmitted={<DoSearch search={action.searchString} /> <User.Update name="Ada" />} />
        "#,
        "route-root.nx",
    );
}

#[test]
fn a_plain_record_is_rejected_even_at_the_root() {
    assert_error(
        r#"
        type User = { name:string }
        component <SearchBox emits { SearchSubmitted { searchString:string } } /> = { <TextInput /> }
        let render() = <SearchBox onSearchSubmitted=<User name="Ada" /> />
        "#,
        "route-root-plain.nx",
        "handler-result-not-action",
        "User",
    );
}

#[test]
fn a_mixed_result_list_is_routed_item_by_item() {
    assert_ok(
        r#"
        action Saved = { }
        external component <Button emits { Tapped } />
        component <Form emits { Saved } /> = { state { dirty:boolean = true } <Button onTapped={<Update dirty=false /> <Saved />} /> }
        "#,
        "route-mixed.nx",
    );

    assert_error(
        r#"
        action Saved = { }
        external component <Button emits { Tapped } />
        component <Form /> = { state { dirty:boolean = true } <Button onTapped={<Update dirty=false /> <Saved />} /> }
        "#,
        "route-mixed-not-emitted.nx",
        "handler-action-not-emitted",
        "Saved",
    );
}

#[test]
fn an_inherited_emit_can_be_re_emitted() {
    assert_ok(
        r#"
        action Saved = { }
        external component <Button emits { Tapped } />
        abstract component <FormBase emits { Saved } />
        component <Form extends FormBase /> = { <Button onTapped=<Saved /> /> }
        "#,
        "route-inherited-emit.nx",
    );
}

#[test]
fn an_inline_emit_can_be_re_emitted() {
    assert_ok(
        r#"
        external component <Button emits { Tapped } />
        component <Form emits { Cleared { } } /> = { <Button onTapped=<Form.Cleared /> /> }
        "#,
        "route-inline-emit.nx",
    );
}
