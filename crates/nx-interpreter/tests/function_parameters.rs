//! A parameter a call leaves out is filled by the function: with its default, evaluated after
//! the parameters before it, or with empty when it is optional. This holds for a positional call,
//! an element call, and a host call alike.

use nx_hir::{lower, SourceId};
use nx_interpreter::{Interpreter, RuntimeErrorKind, Value};
use nx_syntax::parse_str;

fn call(
    source: &str,
    function: &str,
    args: Vec<Value>,
) -> Result<Value, nx_interpreter::RuntimeError> {
    let parsed = parse_str(source, "test.nx");
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    let module = lower(parsed.root().expect("root"), SourceId::new(0));
    Interpreter::new().execute_function(&module, function, args)
}

const ADD: &str = "let scale = 10\n\
                   let add(a:int, b:int = { a * scale }, c?:int) = { a + b + (c ?? 0) }\n";

#[test]
fn a_positional_call_leaves_out_trailing_parameters() {
    let source = format!(
        "{ADD}let one() = {{ add(1) }}\nlet two() = {{ add(1, 2) }}\nlet three() = {{ add(1, 2, 3) }}"
    );
    assert_eq!(call(&source, "one", vec![]).unwrap(), Value::Int(11));
    assert_eq!(call(&source, "two", vec![]).unwrap(), Value::Int(3));
    assert_eq!(call(&source, "three", vec![]).unwrap(), Value::Int(6));
}

#[test]
fn an_element_call_leaves_out_a_parameter_in_the_middle() {
    let source = format!("{ADD}let skip() = {{ <add a=1 c=5 /> }}");
    assert_eq!(call(&source, "skip", vec![]).unwrap(), Value::Int(16));
}

#[test]
fn an_element_style_default_reads_the_parameters_before_it() {
    let source = "let <Area w:int h:int = { w } /> = { w * h }\nlet square() = { <Area w=3 /> }";
    assert_eq!(call(source, "square", vec![]).unwrap(), Value::Int(9));
}

#[test]
fn a_host_call_leaves_out_trailing_parameters() {
    assert_eq!(
        call(ADD, "add", vec![Value::Int(2)]).unwrap(),
        Value::Int(22)
    );
    assert_eq!(
        call(
            ADD,
            "add",
            vec![Value::Int(2), Value::Int(1), Value::Int(4)]
        )
        .unwrap(),
        Value::Int(7)
    );
}

#[test]
fn a_host_call_must_reach_every_required_parameter() {
    let error = call(ADD, "add", vec![]).expect_err("a is required");
    match error.kind() {
        RuntimeErrorKind::ParameterCountMismatch {
            required,
            expected,
            actual,
            ..
        } => assert_eq!((*required, *expected, *actual), (1, 3, 0)),
        other => panic!("expected ParameterCountMismatch, got {other:?}"),
    }
    assert_eq!(
        error.to_string().lines().next().unwrap_or_default(),
        "Function add expects 1 to 3 parameter(s), got 0"
    );
}

#[test]
fn a_default_is_lifted_to_its_parameters_occurrence() {
    let source = "let count(xs:int+ = { 7 }) = { xs }\nlet lifted() = { count() }";
    assert_eq!(
        call(source, "lifted", vec![]).unwrap(),
        Value::Array(vec![Value::Int(7)])
    );
}
