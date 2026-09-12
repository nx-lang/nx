//! End-to-end component lifecycle for a component whose state holds a property reference: the
//! `T.Property` case round-trips through initialization, a state snapshot, and a dispatched
//! update, and the rendered output carries the case. Its wire form is covered in `nx-api`.

use nx_hir::{LoweredModule, Name};
use nx_interpreter::{Interpreter, ResolvedProgram, Value, HANDLER_INVOCATION_TYPE_NAME};
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::sync::Arc;

struct Runtime {
    module: Arc<LoweredModule>,
    interpreter: Interpreter,
}

struct Lifecycle {
    rendered: Value,
    snapshot: Vec<u8>,
}

impl Runtime {
    fn new(source: &str) -> Self {
        let result = nx_types::check_str(source, "property-references.nx");
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
            "property-references.nx",
            module.clone(),
        ));
        Self {
            module,
            interpreter,
        }
    }

    fn init(&self, component: &str) -> Lifecycle {
        let result = self
            .interpreter
            .initialize_component(self.module.as_ref(), component, record("object", &[]))
            .expect("Expected initialization to succeed");
        Lifecycle {
            rendered: result.rendered,
            snapshot: result.state_snapshot,
        }
    }

    fn dispatch(&self, lifecycle: &Lifecycle, batch: Vec<Value>) -> Lifecycle {
        let result = self
            .interpreter
            .dispatch_component_actions(self.module.as_ref(), &lifecycle.snapshot, batch)
            .expect("Expected dispatch to succeed");
        Lifecycle {
            rendered: result.rendered,
            snapshot: result.state_snapshot,
        }
    }
}

fn record(type_name: &str, fields: &[(&str, Value)]) -> Value {
    Value::Record {
        type_name: Name::new(type_name),
        fields: fields
            .iter()
            .map(|(name, value)| (SmolStr::new(*name), value.clone()))
            .collect::<FxHashMap<_, _>>(),
    }
}

fn case(union: &str, case: &str) -> Value {
    Value::UnionCase {
        union: Name::new(union),
        case: SmolStr::new(case),
    }
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
            token: Some(token), ..
        } => token.clone(),
        other => panic!("Expected a tokened handler, got {:?}", other),
    }
}

fn invoke(token: &SmolStr, action: Value) -> Value {
    record(
        HANDLER_INVOCATION_TYPE_NAME,
        &[("token", Value::String(token.clone())), ("action", action)],
    )
}

const PEOPLE: &str = r#"
    type User = { name:string email:string? }
    external component <Grid sortBy:User.Property keys:People.Property[] emits { Sorted { } } />
    component <People /> = {
      state { sortBy:User.Property = {User.Property.name} keys:People.Property[] = { Property.sortBy } }
      <Grid sortBy={sortBy} keys={keys} onSorted=<Update sortBy={User.Property.email} keys={ Property.sortBy Property.keys } /> />
    }
"#;

#[test]
fn a_property_case_in_state_round_trips_through_init_snapshot_and_dispatch() {
    let runtime = Runtime::new(PEOPLE);
    let init = runtime.init("People");
    assert_eq!(
        field(&init.rendered, "sortBy"),
        &case("User.Property", "name")
    );
    assert_eq!(
        field(&init.rendered, "keys"),
        &Value::Array(vec![case("People.Property", "sortBy")])
    );

    let next = runtime.dispatch(
        &init,
        vec![invoke(
            &token(&init.rendered, "onSorted"),
            record("Grid.Sorted", &[]),
        )],
    );
    assert_eq!(
        field(&next.rendered, "sortBy"),
        &case("User.Property", "email")
    );
    assert_eq!(
        field(&next.rendered, "keys"),
        &Value::Array(vec![
            case("People.Property", "sortBy"),
            case("People.Property", "keys"),
        ])
    );

    // The snapshot carries the case forward unchanged.
    let again = runtime.dispatch(
        &next,
        vec![invoke(
            &token(&next.rendered, "onSorted"),
            record("Grid.Sorted", &[]),
        )],
    );
    assert_eq!(
        field(&again.rendered, "sortBy"),
        &case("User.Property", "email")
    );
}
