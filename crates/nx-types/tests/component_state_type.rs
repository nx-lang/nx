//! Type checking of a component's derived `T.State` record, the bare `State` name inside a
//! component body, and the props-based `T.Update` and `T.Property` beside them. Covers the
//! component-state-type capability.
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

// ============================================================================
// A component with state has a derived state record
// ============================================================================

#[test]
fn state_record_has_the_state_fields_and_not_the_props() {
    let result = assert_ok(
        r#"
        component <Counter step:int = 1 /> = { state { count:int = 0 note?:string } <Label text={count} /> }
        let s = <Counter.State count=3 />
        "#,
        "state-fields.nx",
    );
    assert_eq!(
        record_fields(&result, "Counter.State"),
        vec![("count".to_string(), false), ("note".to_string(), true)]
    );
    assert_eq!(value_type(&result, "s"), "Counter.State");

    assert_error(
        r#"
        component <Counter step:int = 1 /> = { state { count:int = 0 note?:string } <Label text={count} /> }
        let s = <Counter.State count=3 step=2 />
        "#,
        "state-fields-prop.nx",
        "unknown-record-field",
        "step",
    );
}

#[test]
fn state_record_carries_no_defaults() {
    assert_message(
        r#"
        component <Counter /> = { state { count:int = 0 } <Label text={count} /> }
        let s = <Counter.State />
        "#,
        "state-no-defaults.nx",
        "count",
    );
}

#[test]
fn state_record_has_its_own_update_record_and_property_union() {
    let result = assert_ok(
        r#"
        component <Counter /> = { state { count:int = 0 label:string = "" } <Label text={label} /> }
        let p = <Counter.State.Update count=1 />
        let k: Counter.State.Property = {Counter.State.Property.label}
        "#,
        "state-derived.nx",
    );
    assert_eq!(
        record_fields(&result, "Counter.State.Update"),
        vec![("count".to_string(), false), ("label".to_string(), false)]
    );
    assert_eq!(
        union_cases(&result, "Counter.State.Property"),
        vec!["count", "label"]
    );
}

#[test]
fn intrinsics_work_on_a_state_record() {
    let result = assert_ok(
        r#"
        component <Counter /> = { state { count:int = 0 } <Label text={count} /> }
        let after = {apply(<Counter.State count=1 />, <Counter.State.Update count=2 />)}
        let d = {diff(<Counter.State count=1 />, <Counter.State count=4 />)}
        let m = {merge(<Counter.State.Update count=1 />, <Counter.State.Update />)}
        let keys = {changed(d)}
        "#,
        "state-intrinsics.nx",
    );
    assert_eq!(value_type(&result, "after"), "Counter.State");
    assert_eq!(value_type(&result, "d"), "Counter.State.Update");
    assert_eq!(value_type(&result, "m"), "Counter.State.Update");
    assert_eq!(value_type(&result, "keys"), "Counter.State.Property*");
}

#[test]
fn intrinsics_merge_and_list_a_components_props_patches() {
    let result = assert_ok(
        r#"
        component <Counter step:int = 1 label?:string /> = { <Label /> }
        let m = {merge(<Counter.Update step=2 />, <Counter.Update label={} />)}
        let keys = {changed(m)}
        "#,
        "props-intrinsics.nx",
    );
    assert_eq!(value_type(&result, "m"), "Counter.Update");
    assert_eq!(value_type(&result, "keys"), "Counter.Property*");

    assert_error(
        r#"
        component <Counter step:int = 1 /> = { <Label /> }
        let a = {apply(<Counter step=1 />, <Counter.Update step=2 />)}
        "#,
        "props-apply.nx",
        "intrinsic-argument-type",
        "must be a record or action",
    );
}

#[test]
fn a_component_without_state_has_no_state_record() {
    assert_error(
        r#"
        component <Form /> = { <Panel /> }
        let s: Form.State = <Form.State />
        "#,
        "stateless-state.nx",
        "unknown-state-record",
        "component 'Form' declares no state, so it has no state record",
    );
}

#[test]
fn only_a_component_has_a_derived_state_record() {
    assert_error(
        r#"
        type User = { name:string }
        let a: User.State = <User.State />
        "#,
        "record-state.nx",
        "unknown-state-record",
        "'User' is not a component with state",
    );
    assert_error(
        r#"
        component <Counter /> = { state { count:int = 0 } <Label /> }
        let b: Counter.State.State = <Counter.State.State />
        "#,
        "state-state.nx",
        "unknown-state-record",
        "'Counter.State' is a state record, and state records have no state record of their own",
    );
    assert_error(
        r#"
        type User = { name:string }
        let c = <User.Update.State />
        "#,
        "update-state.nx",
        "unknown-state-record",
        "'User.Update' is an update record",
    );
    assert_error(
        r#"
        type User = { name:string }
        let d = <User.Property.State />
        "#,
        "property-state.nx",
        "unknown-state-record",
        "'User.Property' is a property union",
    );
}

#[test]
fn an_external_component_with_state_has_a_state_record() {
    assert_ok(
        r#"
        external component <SearchBox /> = { state { query:string } }
        let s = <SearchBox.State query="docs" />
        "#,
        "external-state.nx",
    );
}

#[test]
fn a_state_record_cannot_be_extended() {
    assert_message(
        r#"
        component <Counter /> = { state { count:int = 0 } <Label /> }
        type More extends Counter.State = { extra:int }
        "#,
        "state-extends.nx",
        "'Counter.State' is a derived state record and cannot be extended",
    );
}

// ============================================================================
// The bare `State` name resolves to the enclosing component's state record
// ============================================================================

#[test]
fn bare_state_update_in_a_handler_patches_the_component() {
    assert_ok(
        r#"
        external component <Button emits { Tapped { } } />
        component <Counter /> = { state { count:int = 0 } <Button onTapped=<State.Update count={count + 1} /> /> }
        "#,
        "bare-state-handler.nx",
    );
    assert_error(
        r#"
        external component <Button emits { Tapped { } } />
        component <Counter /> = { state { count:int = 0 } <Button onTapped=<State.Update count="one" /> /> }
        "#,
        "bare-state-handler-mismatch.nx",
        "record-field-type-mismatch",
        "count",
    );
}

#[test]
fn bare_state_property_names_a_state_field() {
    assert_ok(
        r#"
        external component <Grid emits { Sorted { by:string } } />
        component <People /> = {
          state { sortBy:State.Property = {State.Property.name} name:string = "" }
          <Grid onSorted=<State.Update sortBy=name /> />
        }
        "#,
        "bare-state-property.nx",
    );
}

#[test]
fn bare_state_as_a_type() {
    assert_ok(
        r#"
        component <Counter /> = { state { count:int = 0 saved?:State } <Label /> }
        let s: Counter.State = <Counter.State count=1 saved=<Counter.State count=0 /> />
        "#,
        "bare-state-type.nx",
    );
}

#[test]
fn bare_state_takes_precedence_over_a_same_named_declaration_inside_a_component() {
    assert_ok(
        r#"
        type State = idle | busy
        external component <Button emits { Tapped { } } />
        component <Counter /> = { state { count:int = 0 } <Button onTapped=<State.Update count=1 /> /> }
        let s: State = idle
        "#,
        "bare-state-precedence.nx",
    );
}

#[test]
fn bare_state_in_a_component_without_state_is_one_diagnostic() {
    let result = check_str(
        r#"
        external component <Button emits { Tapped { } } />
        component <Form /> = { <Button onTapped=<State.Update /> /> }
        "#,
        "bare-state-stateless.nx",
    );
    assert_eq!(
        diagnostics(&result),
        vec![(
            Some("unknown-update-record"),
            "Unknown type 'Form.State.Update': component 'Form' declares no state, so it has no state record"
                .to_string()
        )]
    );
}

// ============================================================================
// A bare `Update` or `Property` inside a component points to the `State` spelling
// ============================================================================

#[test]
fn a_bare_update_inside_a_component_suggests_state_update() {
    assert_error(
        r#"
        external component <Button emits { Tapped { } } />
        component <Counter /> = { state { count:int = 0 } <Button onTapped=<Update count=1 /> /> }
        "#,
        "bare-update-suggest.nx",
        "unknown-bare-update",
        "<State.Update ... />",
    );
}

#[test]
fn a_bare_property_inside_a_component_suggests_state_property() {
    assert_message(
        r#"
        component <Counter /> = { state { count:int = 0 key?:Property } <Label /> }
        "#,
        "bare-property-suggest.nx",
        "did you mean `State.Property`?",
    );
}

#[test]
fn a_declaration_named_update_is_an_ordinary_record_inside_a_component() {
    assert_ok(
        r#"
        type Update = { note:string }
        component <Notes /> = { state { last?:Update } <Panel /> }
        "#,
        "ordinary-update.nx",
    );
    assert_error(
        r#"
        type Update = { note:string }
        component <Notes /> = { state { last?:Update = <Update note=1 /> } <Panel /> }
        "#,
        "ordinary-update-mismatch.nx",
        "record-field-type-mismatch",
        "note",
    );
}

// ============================================================================
// Handler results are routed to state, parent, or host
// ============================================================================

#[test]
fn a_components_own_props_patch_is_rejected_inside_its_body() {
    assert_error(
        r#"
        external component <Button emits { Tapped { } } />
        component <Counter step:int = 1 /> = { state { count:int = 0 } <Button onTapped=<Counter.Update step=2 /> /> }
        "#,
        "own-props-patch.nx",
        "handler-update-props",
        "write 'State.Update'",
    );
}
