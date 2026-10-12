//! Type checking of a component's derived `T.Props` record: its fields, its own update record and
//! property union, and the names it reserves. Covers the component-props-type capability.
//!
//! The imported-component scenario needs a library on disk and lives with the other library tests
//! in `nx-api`, which owns the library registry.

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

fn value_type(result: &TypeCheckResult, name: &str) -> String {
    result
        .type_env
        .lookup(&nx_hir::Name::new(name))
        .map(|ty| ty.to_string())
        .unwrap_or_else(|| panic!("Expected '{}' to be typed", name))
}

fn record_fields(result: &TypeCheckResult, name: &str) -> Vec<(String, bool)> {
    let module = result.lowered_module.as_ref().expect("lowered module");
    let Some(nx_hir::Item::Record(record)) = module.find_item(name) else {
        panic!("Expected a {} record", name);
    };
    record
        .properties
        .iter()
        .map(|field| (field.name.as_str().to_string(), field.optional))
        .collect()
}

fn union_cases(result: &TypeCheckResult, name: &str) -> Vec<String> {
    let module = result.lowered_module.as_ref().expect("lowered module");
    let Some(nx_hir::Item::Union(union)) = module.find_item(name) else {
        panic!("Expected a {} union", name);
    };
    union
        .cases
        .iter()
        .map(|case| case.name.as_str().to_string())
        .collect()
}

#[test]
fn props_record_has_the_props_and_not_the_state() {
    let result = assert_ok(
        r#"
        component <Counter step:int = 1 label?:string /> = { state { count:int = 0 } <Label text={count} /> }
        let p = <Counter.Props step=2 />
        "#,
        "props-fields.nx",
    );
    assert_eq!(
        record_fields(&result, "Counter.Props"),
        vec![("step".to_string(), false), ("label".to_string(), true)]
    );
    assert_eq!(value_type(&result, "p"), "Counter.Props");

    assert_error(
        r#"
        component <Counter step:int = 1 label?:string /> = { state { count:int = 0 } <Label text={count} /> }
        let p = <Counter.Props step=2 count=1 />
        "#,
        "props-fields-state.nx",
        "unknown-record-field",
        "count",
    );
}

#[test]
fn props_record_carries_no_defaults() {
    assert_message(
        r#"
        component <Counter step:int = 1 /> = { <Label /> }
        let p = <Counter.Props />
        "#,
        "props-no-defaults.nx",
        "step",
    );
}

#[test]
fn inherited_props_come_first() {
    let result = assert_ok(
        r#"
        abstract component <Question label:string help?:string />
        component <SingleChoice extends Question allowsOther:boolean = false /> = { <Panel /> }
        let keys: SingleChoice.Props.Property+ = { SingleChoice.Props.Property.label SingleChoice.Props.Property.allowsOther }
        let p = <SingleChoice.Props label="Pick" allowsOther=true />
        "#,
        "props-inherited.nx",
    );
    assert_eq!(
        union_cases(&result, "SingleChoice.Props.Property"),
        vec!["label", "help", "allowsOther"]
    );

    assert_message(
        r#"
        abstract component <Question label:string help?:string />
        component <SingleChoice extends Question allowsOther:boolean = false /> = { <Panel /> }
        let p = <SingleChoice.Props allowsOther=true />
        "#,
        "props-inherited-required.nx",
        "label",
    );
}

#[test]
fn props_record_has_its_own_update_record_and_property_union() {
    let result = assert_ok(
        r#"
        component <Card title:string subtitle?:string /> = { <Panel /> }
        let patch = <Card.Props.Update title="New" />
        let k: Card.Props.Property = {Card.Props.Property.subtitle}
        let after = {apply(<Card.Props title="Old" />, patch)}
        "#,
        "props-derived.nx",
    );
    assert_eq!(value_type(&result, "patch"), "Card.Props.Update");
    assert_eq!(value_type(&result, "after"), "Card.Props");
    assert_eq!(
        union_cases(&result, "Card.Props.Property"),
        vec!["title", "subtitle"]
    );
}

#[test]
fn a_component_with_no_props_has_an_empty_props_record() {
    assert_ok(
        r#"
        component <Divider /> = { <Line /> }
        let p = <Divider.Props />
        "#,
        "props-empty.nx",
    );
}

#[test]
fn an_external_component_has_a_props_record() {
    let result = assert_ok(
        r#"
        external component <SearchBox placeholder?:string /> = { state { query:string } }
        let p = <SearchBox.Props placeholder="Search" />
        "#,
        "props-external.nx",
    );
    assert_eq!(
        record_fields(&result, "SearchBox.Props"),
        vec![("placeholder".to_string(), true)]
    );
}

#[test]
fn a_generic_components_props_record_keeps_its_type_parameters() {
    let result = assert_ok(
        r#"
        component <List TItem:type items:TItem+ /> = { <Panel /> }
        let p:<List.Props TItem=int/> = <List.Props TItem=int items={ 1 2 } />
        "#,
        "props-generic.nx",
    );
    assert_eq!(union_cases(&result, "List.Props.Property"), vec!["items"]);
}

#[test]
fn only_a_component_has_a_derived_props_record() {
    assert_error(
        r#"
        type User = { name:string }
        let a: User.Props = <User.Props />
        "#,
        "props-record.nx",
        "unresolved-type",
        "only a component has a props record",
    );
    assert_message(
        r#"
        component <Card /> = { <Panel /> }
        let b: Card.Props.Props = <Card.Props.Props />
        "#,
        "props-props.nx",
        "Card.Props.Props",
    );
    assert_message(
        r#"
        component <Card /> = { state { open:boolean = false } <Panel /> }
        let c = <Card.State.Props />
        "#,
        "state-props.nx",
        "Card.State.Props",
    );
}

#[test]
fn a_props_record_cannot_be_extended() {
    assert_error(
        r#"
        component <Card title:string /> = { <Panel /> }
        type Wider extends Card.Props = { more:int }
        "#,
        "props-extends.nx",
        "lowering-error",
        "derived props record",
    );
}

#[test]
fn a_component_without_state_suggests_its_props_record() {
    assert_error(
        r#"
        component <Form title:string /> = { <Panel /> }
        let patch = <Form.Update title="Sign in" />
        "#,
        "form-update.nx",
        "unknown-update-record",
        "write 'Form.Props.Update' to patch its props",
    );
    assert_error(
        r#"
        component <Form title:string /> = { <Panel /> }
        let key = {Form.Property.title}
        "#,
        "form-property.nx",
        "unknown-property-union",
        "write 'Form.Props.Property' to name a prop",
    );
}

#[test]
fn a_prop_named_props_stays_legal() {
    assert_ok(
        r#"
        component <Wrapper Props:string /> = { <Label text={Props} /> }
        let w = <Wrapper Props="x" />
        "#,
        "prop-named-props.nx",
    );
}

#[test]
fn props_is_a_reserved_emitted_action_name() {
    assert_message(
        r#"
        component <Form emits { Props { value:string } } /> = { <Panel /> }
        "#,
        "emit-props.nx",
        "'Form.Props' is reserved for the component's derived props record",
    );
    assert_message(
        r#"
        action Props = { value:string }
        component <Form emits { Props } /> = { <Panel /> }
        "#,
        "emit-shared-props.nx",
        "would collide with the component's derived props record",
    );
}
