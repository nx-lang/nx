//! End-to-end component lifecycles driven through the public `Interpreter` API: initialize, then
//! dispatch handler invocations and emitted actions against each returned snapshot in turn.
//!
//! Every source is type checked first and run from the checked module, so these are the programs
//! a host would actually run. Covers the component-runtime-bindings requirements for dispatch,
//! handler tokens, and batch atomicity.

use nx_hir::{LoweredModule, Name};
use nx_interpreter::{
    Interpreter, ResolvedProgram, RuntimeErrorKind, Value, HANDLER_INVOCATION_TYPE_NAME,
};
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::sync::Arc;

/// A type-checked program and an interpreter bound to it.
struct Runtime {
    module: Arc<LoweredModule>,
    interpreter: Interpreter,
}

impl Runtime {
    fn new(source: &str) -> Self {
        let result = nx_types::check_str(source, "update-records.nx");
        assert!(
            result.errors().is_empty(),
            "Expected source to pass analysis, got {:?}",
            result
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message().to_string())
                .collect::<Vec<_>>()
        );
        let module = result.lowered_module.expect("lowered module");
        let interpreter = Interpreter::from_resolved_program(ResolvedProgram::single_root_module(
            source.len() as u64,
            "update-records.nx",
            module.clone(),
        ));
        Self {
            module,
            interpreter,
        }
    }

    fn init(&self, component: &str, props: Value) -> Lifecycle {
        let result = self
            .interpreter
            .initialize_component(self.module.as_ref(), component, props)
            .expect("Expected initialization to succeed");
        Lifecycle {
            rendered: result.rendered,
            effects: Vec::new(),
            snapshot: result.state_snapshot,
        }
    }

    fn dispatch(&self, lifecycle: &Lifecycle, batch: Vec<Value>) -> Lifecycle {
        let result = self
            .try_dispatch(&lifecycle.snapshot, batch)
            .expect("Expected dispatch to succeed");
        Lifecycle {
            rendered: result.rendered,
            effects: result.effects,
            snapshot: result.state_snapshot,
        }
    }

    fn try_dispatch(
        &self,
        snapshot: &[u8],
        batch: Vec<Value>,
    ) -> Result<nx_interpreter::ComponentDispatchResult, nx_interpreter::RuntimeError> {
        self.interpreter
            .dispatch_component_actions(self.module.as_ref(), snapshot, batch)
    }

    fn call(&self, function: &str) -> Value {
        self.interpreter
            .execute_function(self.module.as_ref(), function, vec![])
            .expect("Expected function to evaluate")
    }
}

/// One step of a component instance: what it rendered, what it emitted, and its next snapshot.
struct Lifecycle {
    rendered: Value,
    effects: Vec<Value>,
    snapshot: Vec<u8>,
}

fn record(type_name: &str, fields: &[(&str, Value)]) -> Value {
    Value::Record {
        type_name: Name::new(type_name),
        fields: fields
            .iter()
            .map(|(name, value)| (SmolStr::new(*name), value.clone()))
            .collect(),
    }
}

fn string(value: &str) -> Value {
    Value::String(SmolStr::new(value))
}

fn no_props() -> Value {
    record("object", &[])
}

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    let Value::Record { fields, .. } = value else {
        panic!("Expected a record, got {:?}", value);
    };
    fields
        .get(name)
        .unwrap_or_else(|| panic!("Expected field '{}' on {:?}", name, value))
}

fn token(rendered: &Value, handler_property: &str) -> SmolStr {
    match field(rendered, handler_property) {
        Value::ActionHandler {
            token: Some(token),
            action_name,
            ..
        } => {
            assert!(!token.is_empty());
            assert!(!action_name.as_str().is_empty());
            token.clone()
        }
        other => panic!("Expected a tokened handler, got {:?}", other),
    }
}

fn invoke(token: &SmolStr, action: Value) -> Value {
    record(
        HANDLER_INVOCATION_TYPE_NAME,
        &[("token", Value::String(token.clone())), ("action", action)],
    )
}

fn tapped() -> Value {
    record("Button.Tapped", &[])
}

const COUNTER: &str = r#"
    external component <Button value:int = 0 emits { Tapped { } } />
    component <Counter /> = {
      state { count:int = 0 }
      <Button value={count} onTapped=<Update count={count + 1} /> />
    }
"#;

const FORM: &str = r#"
    action Saved = { }
    external component <Button emits { Tapped { } } />
    component <Form emits { Saved } /> = {
      state { dirty:boolean = true count:int = 5 }
      <Panel dirty={dirty} count={count}>
        <Button onTapped={<Update dirty=false /> <Saved />} />
        <Button onTapped=<Saved /> />
      </Panel>
    }
"#;

fn panel_buttons(rendered: &Value) -> &[Value] {
    match field(rendered, "content") {
        Value::Array(buttons) => buttons,
        other => panic!("Expected buttons, got {:?}", other),
    }
}

// ============================================================================
// Handler invocation patches state and returns rendered output
// ============================================================================

#[test]
fn counter_counts_across_init_dispatch_dispatch() {
    let runtime = Runtime::new(COUNTER);
    let init = runtime.init("Counter", no_props());
    assert_eq!(field(&init.rendered, "value"), &Value::Int(0));

    let first = runtime.dispatch(
        &init,
        vec![invoke(&token(&init.rendered, "onTapped"), tapped())],
    );
    assert_eq!(field(&first.rendered, "value"), &Value::Int(1));
    assert!(first.effects.is_empty());

    let second = runtime.dispatch(
        &first,
        vec![invoke(&token(&first.rendered, "onTapped"), tapped())],
    );
    assert_eq!(field(&second.rendered, "value"), &Value::Int(2));
    assert!(second.effects.is_empty());
}

#[test]
fn initialization_output_names_the_action_each_handler_accepts() {
    let runtime = Runtime::new(COUNTER);
    let init = runtime.init("Counter", no_props());
    match field(&init.rendered, "onTapped") {
        Value::ActionHandler {
            action_name, token, ..
        } => {
            assert_eq!(action_name.as_str(), "Button.Tapped");
            assert!(token.as_ref().is_some_and(|token| !token.is_empty()));
        }
        other => panic!("Expected a handler, got {:?}", other),
    }
}

#[test]
fn pure_evaluation_output_carries_no_tokens() {
    let runtime = Runtime::new(COUNTER);
    let evaluated = runtime
        .interpreter
        .evaluate_component(
            runtime.module.as_ref(),
            "Counter",
            no_props(),
            record("object", &[("count", Value::Int(4))]),
        )
        .expect("Expected evaluation to succeed");
    assert_eq!(field(&evaluated.rendered, "value"), &Value::Int(4));
    assert!(matches!(
        field(&evaluated.rendered, "onTapped"),
        Value::ActionHandler { token: None, .. }
    ));
}

#[test]
fn two_invocations_in_one_batch_both_apply() {
    let runtime = Runtime::new(COUNTER);
    let init = runtime.init("Counter", no_props());
    let tap = token(&init.rendered, "onTapped");
    let next = runtime.dispatch(&init, vec![invoke(&tap, tapped()), invoke(&tap, tapped())]);
    assert_eq!(field(&next.rendered, "value"), &Value::Int(2));
}

// ============================================================================
// Results route by type: state, parent, host
// ============================================================================

#[test]
fn form_applies_its_update_and_re_emits_saved() {
    let runtime = Runtime::new(FORM);
    let init = runtime.init("Form", no_props());
    let buttons = panel_buttons(&init.rendered);
    let save_and_clean = token(&buttons[0], "onTapped");
    let save_only = token(&buttons[1], "onTapped");

    assert_eq!(field(&init.rendered, "dirty"), &Value::Boolean(true));

    let first = runtime.dispatch(&init, vec![invoke(&save_and_clean, tapped())]);
    assert_eq!(first.effects, vec![record("Saved", &[])]);
    assert_eq!(field(&first.rendered, "dirty"), &Value::Boolean(false));
    assert_eq!(
        field(&first.rendered, "count"),
        &Value::Int(5),
        "Absent fields are unchanged"
    );

    // The second button only emits, so the state carries forward as it was.
    let buttons = panel_buttons(&first.rendered);
    let second = runtime.dispatch(
        &first,
        vec![invoke(&token(&buttons[1], "onTapped"), tapped())],
    );
    assert_eq!(second.effects, vec![record("Saved", &[])]);
    assert_eq!(field(&second.rendered, "dirty"), &Value::Boolean(false));

    // The stale token from initialization is refused against the later snapshot.
    let error = runtime
        .try_dispatch(&second.snapshot, vec![invoke(&save_only, tapped())])
        .expect_err("Expected a stale token to be refused");
    assert!(matches!(
        error.kind(),
        RuntimeErrorKind::UnknownHandlerToken { .. }
    ));
}

#[test]
fn an_effect_only_batch_carries_state_forward_and_still_renders() {
    let runtime = Runtime::new(
        r#"
        action Saved = { }
        external component <Button value:int = 0 emits { Tapped { } } />
        component <Form emits { Saved } /> = {
          state { count:int = 5 }
          <Button value={count} onTapped=<Saved /> />
        }
        "#,
    );
    let init = runtime.init("Form", no_props());
    let next = runtime.dispatch(
        &init,
        vec![invoke(&token(&init.rendered, "onTapped"), tapped())],
    );
    assert_eq!(next.effects, vec![record("Saved", &[])]);
    assert_eq!(field(&next.rendered, "value"), &Value::Int(5));
}

#[test]
fn a_parent_bound_update_is_an_effect_for_the_host() {
    let runtime = Runtime::new(
        r#"
        component <SearchBox emits { SearchSubmitted { searchString:string } } /> = {
          state { query:string = "" }
          <TextInput value={query} />
        }
        component <Page /> = {
          state { query:string = "" }
          <SearchBox onSearchSubmitted=<Update query={action.searchString} /> />
        }
        "#,
    );
    let page = runtime.init("Page", no_props());
    let search_box = runtime.init("SearchBox", page.rendered.clone());

    let next = runtime.dispatch(
        &search_box,
        vec![record(
            "SearchBox.SearchSubmitted",
            &[("searchString", string("docs"))],
        )],
    );
    assert_eq!(
        next.effects,
        vec![record("Page.Update", &[("query", string("docs"))])]
    );
    assert_eq!(field(&next.rendered, "value"), &string(""));
}

#[test]
fn host_actions_run_in_order() {
    let runtime = Runtime::new(
        r#"
        action DoSearch = { search:string }
        component <SearchBox emits { SearchSubmitted { searchString:string } } /> = {
          <TextInput />
        }
        let withHandler() = <SearchBox onSearchSubmitted=<DoSearch search={action.searchString} /> />
        "#,
    );
    let init = runtime.init("SearchBox", runtime.call("withHandler"));
    let next = runtime.dispatch(
        &init,
        vec![
            record(
                "SearchBox.SearchSubmitted",
                &[("searchString", string("docs"))],
            ),
            record(
                "SearchBox.SearchSubmitted",
                &[("searchString", string("guides"))],
            ),
        ],
    );
    assert_eq!(
        next.effects,
        vec![
            record("DoSearch", &[("search", string("docs"))]),
            record("DoSearch", &[("search", string("guides"))]),
        ]
    );
}

#[test]
fn an_update_record_is_the_only_way_state_changes() {
    let runtime = Runtime::new(
        r#"
        action Cleared = { }
        external component <TextInput value:string emits { TextChanged { text:string } Cleared { } } />
        component <SearchBox placeholder:string emits { Cleared } /> = {
          state { query:string = {placeholder} typed:int = 0 }
          <TextInput
            value={query}
            onTextChanged=<Update query={action.text} />
            onCleared={<Update query="" /> <Cleared />}
          />
        }
        "#,
    );
    let init = runtime.init(
        "SearchBox",
        record("object", &[("placeholder", string("Find"))]),
    );
    assert_eq!(field(&init.rendered, "value"), &string("Find"));

    let typed = runtime.dispatch(
        &init,
        vec![invoke(
            &token(&init.rendered, "onTextChanged"),
            record("TextInput.TextChanged", &[("text", string("docs"))]),
        )],
    );
    assert_eq!(field(&typed.rendered, "value"), &string("docs"));

    let cleared = runtime.dispatch(
        &typed,
        vec![invoke(
            &token(&typed.rendered, "onCleared"),
            record("TextInput.Cleared", &[]),
        )],
    );
    assert_eq!(field(&cleared.rendered, "value"), &string(""));
    assert_eq!(cleared.effects, vec![record("Cleared", &[])]);
}

// ============================================================================
// Rejections and atomicity
// ============================================================================

#[test]
fn an_invocation_with_the_wrong_action_type_names_the_expected_action() {
    let runtime = Runtime::new(COUNTER);
    let init = runtime.init("Counter", no_props());
    let error = runtime
        .try_dispatch(
            &init.snapshot,
            vec![invoke(
                &token(&init.rendered, "onTapped"),
                record("Slider.EndChanged", &[("value", Value::Float(1.0))]),
            )],
        )
        .expect_err("Expected the wrong action type to be refused");
    assert!(
        matches!(error.kind(), RuntimeErrorKind::TypeMismatch { expected, .. } if expected == "Button.Tapped"),
        "got {:?}",
        error
    );
}

#[test]
fn a_failing_entry_discards_every_earlier_patch() {
    let runtime = Runtime::new(COUNTER);
    let init = runtime.init("Counter", no_props());
    let tap = token(&init.rendered, "onTapped");

    let error = runtime
        .try_dispatch(
            &init.snapshot,
            vec![
                invoke(&tap, tapped()),
                invoke(&SmolStr::new("h7-1"), tapped()),
            ],
        )
        .expect_err("Expected the batch to fail");
    assert!(error.to_string().contains("'h7-1'"), "got {}", error);

    let retry = runtime.dispatch(&init, vec![]);
    assert_eq!(field(&retry.rendered, "value"), &Value::Int(0));
}

#[test]
fn an_update_that_fails_validation_aborts_the_batch() {
    // Lowered but deliberately not type checked: the field type is wrong.
    let source = r#"
        external component <Button value:int = 0 emits { Tapped { } } />
        component <Counter /> = {
          state { count:int = 0 }
          <Button value={count} onTapped=<Update count="many" /> />
        }
    "#;
    let module = Arc::new(nx_hir::lower_source_module(source, "unchecked.nx").expect("lowers"));
    let interpreter = Interpreter::from_resolved_program(ResolvedProgram::single_root_module(
        1,
        "unchecked.nx",
        module.clone(),
    ));
    let init = interpreter
        .initialize_component(module.as_ref(), "Counter", no_props())
        .expect("Expected initialization to succeed");
    let error = interpreter
        .dispatch_component_actions(
            module.as_ref(),
            &init.state_snapshot,
            vec![invoke(&token(&init.rendered, "onTapped"), tapped())],
        )
        .expect_err("Expected the mistyped update to abort the batch");
    assert!(error.to_string().contains("count"), "got {}", error);
}

#[test]
fn update_records_constructed_in_functions_keep_absence() {
    let runtime = Runtime::new(
        r#"
        type User = { name:string = "anon" email:string? }
        let clearEmail(): User.Update = <User.Update email={null} />
        "#,
    );
    let mut expected = FxHashMap::default();
    expected.insert(SmolStr::new("email"), Value::Null);
    assert_eq!(
        runtime.call("clearEmail"),
        Value::Record {
            type_name: Name::new("User.Update"),
            fields: expected,
        }
    );
}

// ============================================================================
// Handlers bound elsewhere: captures only, every result an effect
// ============================================================================

/// Every dispatch token in `value`, in the order the token walk assigned them.
fn tokens(value: &Value) -> Vec<SmolStr> {
    fn walk(value: &Value, out: &mut Vec<SmolStr>) {
        match value {
            Value::ActionHandler {
                token: Some(token), ..
            } => out.push(token.clone()),
            Value::Array(items) => items.iter().for_each(|item| walk(item, out)),
            Value::Record { fields, .. } => {
                let mut names = fields.keys().collect::<Vec<_>>();
                names.sort();
                for name in names {
                    walk(&fields[name], out);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(value, &mut out);
    out
}

#[test]
fn a_loop_variable_that_shadows_a_state_field_keeps_its_captured_value() {
    let runtime = Runtime::new(
        r#"
        external component <Row emits { Tapped { } } />
        component <Picker /> = {
          state { count:int = 0 items:int[] = {10 20 30} }
          <Panel count={count}>
            {for count in items { <Row onTapped=<Update count={count} /> /> }}
          </Panel>
        }
        "#,
    );
    let init = runtime.init("Picker", no_props());
    let rows = tokens(&init.rendered);
    assert_eq!(rows.len(), 3);

    let next = runtime.dispatch(&init, vec![invoke(&rows[1], record("Row.Tapped", &[]))]);
    assert_eq!(field(&next.rendered, "count"), &Value::Int(20));
    assert!(next.effects.is_empty());
}

#[test]
fn a_root_bound_handler_in_the_output_runs_and_its_results_are_effects() {
    let runtime = Runtime::new(
        r#"
        action Saved = { }
        external component <Button emits { Tapped { } } />
        let saveButton() = <Button onTapped={<Saved /> <Form.Update dirty=false />} />
        component <Form /> = {
          state { dirty:boolean = true }
          <Panel dirty={dirty}>{saveButton()}</Panel>
        }
        "#,
    );
    let init = runtime.init("Form", no_props());
    let save = tokens(&init.rendered);
    assert_eq!(save.len(), 1);

    let next = runtime.dispatch(&init, vec![invoke(&save[0], tapped())]);
    // The handler was bound at the root, so its update record is the host's to apply.
    assert_eq!(
        next.effects,
        vec![
            record("Saved", &[]),
            record("Form.Update", &[("dirty", Value::Boolean(false))]),
        ]
    );
    assert_eq!(field(&next.rendered, "dirty"), &Value::Boolean(true));
}

#[test]
fn a_parent_bound_handler_on_an_external_instance_runs_and_its_results_are_effects() {
    let runtime = Runtime::new(
        r#"
        external component <SearchBox emits { SearchSubmitted { searchString:string } } />
        component <Page /> = {
          state { query:string = "" }
          <SearchBox onSearchSubmitted=<Update query={action.searchString} /> />
        }
        "#,
    );
    let page = runtime.init("Page", no_props());
    let search_box = runtime.init("SearchBox", page.rendered.clone());
    let submit = token(&search_box.rendered, "onSearchSubmitted");

    let next = runtime.dispatch(
        &search_box,
        vec![invoke(
            &submit,
            record(
                "SearchBox.SearchSubmitted",
                &[("searchString", string("docs"))],
            ),
        )],
    );
    assert_eq!(
        next.effects,
        vec![record("Page.Update", &[("query", string("docs"))])]
    );
}

#[test]
fn any_action_or_update_record_from_a_root_bound_handler_is_an_effect() {
    let runtime = Runtime::new(
        r#"
        type User = { name:string }
        action DoSearch = { search:string }
        external component <SearchBox emits { SearchSubmitted { searchString:string } } />
        let render() = <SearchBox onSearchSubmitted={<DoSearch search={action.searchString} /> <User.Update name="Ada" />} />
        "#,
    );
    let search_box = runtime.init("SearchBox", runtime.call("render"));

    let next = runtime.dispatch(
        &search_box,
        vec![record(
            "SearchBox.SearchSubmitted",
            &[("searchString", string("docs"))],
        )],
    );
    assert_eq!(
        next.effects,
        vec![
            record("DoSearch", &[("search", string("docs"))]),
            record("User.Update", &[("name", string("Ada"))]),
        ]
    );
}

// ============================================================================
// The documented example
// ============================================================================

#[test]
fn the_component_example_counts_and_resets() {
    let runtime = Runtime::new(include_str!("../../../examples/nx/component.nx"));
    let init = runtime.init("Counter", record("object", &[("step", Value::Int(2))]));
    let row = |lifecycle: &Lifecycle| -> Vec<Value> {
        match field(&lifecycle.rendered, "content") {
            Value::Array(items) => items.clone(),
            other => panic!("Expected the row's children, got {:?}", other),
        }
    };
    let children = row(&init);
    assert_eq!(field(&children[0], "text"), &Value::Int(0));

    let add = token(&children[1], "onTapped");
    let added = runtime.dispatch(&init, vec![invoke(&add, tapped()), invoke(&add, tapped())]);
    assert_eq!(field(&row(&added)[0], "text"), &Value::Int(4));
    assert!(added.effects.is_empty());

    let reset = token(&row(&added)[2], "onTapped");
    let reset = runtime.dispatch(&added, vec![invoke(&reset, tapped())]);
    assert_eq!(field(&row(&reset)[0], "text"), &Value::Int(0));
    assert_eq!(reset.effects, vec![record("Reset", &[])]);
}

// ============================================================================
// Host-supplied records are constructed at every depth
// ============================================================================

/// A component whose props, action payload, and state each carry a `User.Update`.
const HOST_BOUNDARY: &str = r#"
    type User = { name:string email:string? }
    external component <Button emits { Tapped { patch:User.Update } } />
    component <Editor pending:User.Update drafts:User.Update[] = {} /> = {
      state { last:User.Update = <User.Update /> }
      <Panel pending={pending} drafts={drafts} last={last}>
        <Button onTapped=<Update last={action.patch} /> />
      </Panel>
    }
"#;

fn editor_props(pending: Value) -> Value {
    record("object", &[("pending", pending)])
}

fn try_init(runtime: &Runtime, props: Value) -> Result<Value, nx_interpreter::RuntimeError> {
    runtime
        .interpreter
        .initialize_component(runtime.module.as_ref(), "Editor", props)
        .map(|result| result.rendered)
}

#[test]
fn a_host_update_record_in_a_prop_keeps_only_the_fields_it_carries() {
    let runtime = Runtime::new(HOST_BOUNDARY);
    let rendered = try_init(
        &runtime,
        editor_props(record("User.Update", &[("email", Value::Null)])),
    )
    .expect("Expected a well-formed update record to initialize");
    assert_eq!(
        field(&rendered, "pending"),
        &record("User.Update", &[("email", Value::Null)])
    );
    assert_eq!(field(&rendered, "last"), &record("User.Update", &[]));
}

#[test]
fn a_host_update_record_in_a_prop_with_an_unknown_field_is_rejected() {
    let runtime = Runtime::new(HOST_BOUNDARY);
    let error = try_init(
        &runtime,
        editor_props(record("User.Update", &[("nick", string("a"))])),
    )
    .expect_err("Expected the unknown field to be rejected at initialization");
    assert!(
        matches!(error.kind(), RuntimeErrorKind::UnknownRecordField { field, .. } if field == "nick"),
        "got {}",
        error
    );
}

#[test]
fn a_host_update_record_in_a_prop_with_null_for_a_non_nullable_field_is_rejected() {
    let runtime = Runtime::new(HOST_BOUNDARY);
    let error = try_init(
        &runtime,
        editor_props(record("User.Update", &[("name", Value::Null)])),
    )
    .expect_err("Expected null for a non-nullable field to be rejected at initialization");
    assert!(error.to_string().contains("name"), "got {}", error);
}

#[test]
fn a_host_update_record_inside_a_prop_array_is_checked_too() {
    let runtime = Runtime::new(HOST_BOUNDARY);
    let props = record(
        "object",
        &[
            ("pending", record("User.Update", &[])),
            (
                "drafts",
                Value::Array(vec![
                    record("User.Update", &[("name", string("Ada"))]),
                    record("User.Update", &[("nick", string("a"))]),
                ]),
            ),
        ],
    );
    let error =
        try_init(&runtime, props).expect_err("Expected the second array element to be rejected");
    assert!(
        matches!(error.kind(), RuntimeErrorKind::UnknownRecordField { field, .. } if field == "nick"),
        "got {}",
        error
    );
}

#[test]
fn a_host_update_record_inside_an_action_payload_is_checked_too() {
    let runtime = Runtime::new(HOST_BOUNDARY);
    let init = runtime.init("Editor", editor_props(record("User.Update", &[])));
    let tapped = token(field(&init.rendered, "content"), "onTapped");

    let ok = runtime.dispatch(
        &init,
        vec![invoke(
            &tapped,
            record(
                "Button.Tapped",
                &[("patch", record("User.Update", &[("email", Value::Null)]))],
            ),
        )],
    );
    assert_eq!(
        field(&ok.rendered, "last"),
        &record("User.Update", &[("email", Value::Null)])
    );

    let error = runtime
        .try_dispatch(
            &init.snapshot,
            vec![invoke(
                &tapped,
                record(
                    "Button.Tapped",
                    &[("patch", record("User.Update", &[("name", Value::Null)]))],
                ),
            )],
        )
        .expect_err("Expected null for a non-nullable field inside the payload to be rejected");
    assert!(error.to_string().contains("name"), "got {}", error);
}

#[test]
fn a_host_update_record_in_explicit_state_is_checked_too() {
    let runtime = Runtime::new(HOST_BOUNDARY);
    let error = runtime
        .interpreter
        .evaluate_component(
            runtime.module.as_ref(),
            "Editor",
            editor_props(record("User.Update", &[])),
            record(
                "object",
                &[("last", record("User.Update", &[("nick", string("a"))]))],
            ),
        )
        .expect_err("Expected the unknown field in explicit state to be rejected");
    assert!(
        matches!(error.kind(), RuntimeErrorKind::UnknownRecordField { field, .. } if field == "nick"),
        "got {}",
        error
    );
}

#[test]
fn a_host_plain_record_nested_in_a_prop_is_checked_too() {
    let runtime = Runtime::new(
        r#"
        type Address = { city:string }
        type User = { name:string address:Address }
        component <Card user:User /> = { <Panel city={user.address.city} /> }
        "#,
    );
    let props = |address: Value| {
        record(
            "object",
            &[(
                "user",
                record("User", &[("name", string("Ada")), ("address", address)]),
            )],
        )
    };
    let rendered = runtime
        .interpreter
        .initialize_component(
            runtime.module.as_ref(),
            "Card",
            props(record("Address", &[("city", string("Paris"))])),
        )
        .expect("Expected a well-formed nested record to initialize")
        .rendered;
    assert_eq!(field(&rendered, "city"), &string("Paris"));

    let error = runtime
        .interpreter
        .initialize_component(
            runtime.module.as_ref(),
            "Card",
            props(record("Address", &[])),
        )
        .expect_err("Expected the nested record's missing field to be rejected");
    assert!(error.to_string().contains("city"), "got {}", error);
}

#[test]
fn a_host_update_record_under_a_component_typed_prop_is_checked_too() {
    let runtime = Runtime::new(
        r#"
        type User = { name:string email:string? }
        component <Editor pending:User.Update /> = { <Panel pending={pending} /> }
        component <Wrap inner:Editor /> = { <Frame inner={inner} /> }
        "#,
    );
    let init_wrap = |pending: Value| {
        runtime.interpreter.initialize_component(
            runtime.module.as_ref(),
            "Wrap",
            record(
                "object",
                &[("inner", record("Editor", &[("pending", pending)]))],
            ),
        )
    };

    let rendered = init_wrap(record("User.Update", &[("email", Value::Null)]))
        .expect("Expected a well-formed component value to initialize")
        .rendered;
    assert_eq!(
        field(&rendered, "inner"),
        &record(
            "Editor",
            &[("pending", record("User.Update", &[("email", Value::Null)]))]
        )
    );

    let error = init_wrap(record("User.Update", &[("nick", string("a"))]))
        .expect_err("Expected the unknown field under the component value to be rejected");
    assert!(
        matches!(error.kind(), RuntimeErrorKind::UnknownRecordField { field, .. } if field == "nick"),
        "got {}",
        error
    );

    let error = init_wrap(record("User.Update", &[("name", Value::Null)]))
        .expect_err("Expected null for a non-nullable field under the component value to fail");
    assert!(error.to_string().contains("name"), "got {}", error);
}

#[test]
fn a_host_action_entry_with_no_bound_handler_is_still_checked() {
    let runtime = Runtime::new(
        r#"
        type User = { name:string email:string? }
        component <Editor emits { Apply { patch:User.Update } } /> = { <Panel /> }
        "#,
    );
    let init = runtime.init("Editor", no_props());
    let apply = |patch: Value| record("Editor.Apply", &[("patch", patch)]);

    let ok = runtime.dispatch(
        &init,
        vec![apply(record("User.Update", &[("email", Value::Null)]))],
    );
    assert!(ok.effects.is_empty());

    let error = runtime
        .try_dispatch(
            &init.snapshot,
            vec![apply(record("User.Update", &[("name", Value::Null)]))],
        )
        .expect_err("Expected the unbound entry's payload to be constructed and rejected");
    assert!(error.to_string().contains("name"), "got {}", error);
}
