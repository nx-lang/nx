//! Evaluation of generic records: type arguments leave no trace at runtime.
//!
//! Covers the evaluation scenarios of the `record-type-parameters` capability — a constructed
//! value has no field for a type parameter, its fields read and patch as any record's do, and two
//! instantiations with equal fields are equal values.

use nx_hir::{LoweredModule, Name};
use nx_interpreter::{Interpreter, ResolvedProgram, Value};
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::sync::Arc;

struct Runtime {
    module: Arc<LoweredModule>,
    interpreter: Interpreter,
}

impl Runtime {
    fn new(source: &str) -> Self {
        let result = nx_types::check_str(source, "record-type-parameters.nx");
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
            "record-type-parameters.nx",
            module.clone(),
        ));
        Self {
            module,
            interpreter,
        }
    }

    fn call(&self, function: &str) -> Value {
        self.interpreter
            .execute_function(self.module.as_ref(), function, vec![])
            .expect("Expected function to evaluate")
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

const RANGE: &str = "type Range = { T:type start:T end:T }\n";

#[test]
fn a_constructed_generic_record_has_no_type_parameter_field() {
    let runtime = Runtime::new(&format!(
        "{RANGE}let r() = {{<Range T=int start={{1}} end={{5}} />}}"
    ));
    assert_eq!(
        runtime.call("r"),
        record("Range", &[("start", Value::Int(1)), ("end", Value::Int(5))])
    );
}

#[test]
fn a_field_of_a_generic_record_reads_and_patches() {
    let runtime = Runtime::new(&format!(
        "{RANGE}let r = <Range T=int start={{1}} end={{5}} />\n\
         let s() = {{r.start}}\n\
         let moved() = {{apply(r, <Range.Update T=int end={{9}} />)}}"
    ));
    assert_eq!(runtime.call("s"), Value::Int(1));
    assert_eq!(
        runtime.call("moved"),
        record("Range", &[("start", Value::Int(1)), ("end", Value::Int(9))])
    );
}

#[test]
fn two_instantiations_with_equal_fields_are_equal_values() {
    // `Range` of `int` and `Range` of `object` are two types to the checker and one value to the
    // runtime, which is what erasing the arguments below the checker means.
    let runtime = Runtime::new(&format!(
        "{RANGE}let a() = {{<Range T=int start={{1}} end={{5}} />}}\n\
         let b() = {{<Range T=object start={{1}} end={{5}} />}}"
    ));
    assert_eq!(runtime.call("a"), runtime.call("b"));
}
