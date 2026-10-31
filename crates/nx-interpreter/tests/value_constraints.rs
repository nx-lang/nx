//! The interpreter evaluates numeric match patterns as the IR runtimes do. That it runs a
//! constrained program, checking it only when it is compiled, is tested in `nx-api`, which builds the
//! program as the CLI does.

use nx_hir::{lower, SourceId};
use nx_interpreter::{Interpreter, Value};
use nx_syntax::parse_str;

fn run(source: &str, function: &str, args: Vec<Value>) -> Value {
    let parsed = parse_str(source, "test.nx");
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    let module = lower(parsed.root().expect("root"), SourceId::new(0));
    Interpreter::new()
        .execute_function(&module, function, args)
        .unwrap_or_else(|error| panic!("{function}: {error}"))
}

fn text(value: Value) -> String {
    match value {
        Value::String(text) => text.to_string(),
        other => format!("{other:?}"),
    }
}

#[test]
fn the_interpreter_matches_a_range_pattern() {
    let source = "let root() = { if 0.5 is { in 0.0..1.0 => \"valid\" else => \"no\" } }";
    assert_eq!(text(run(source, "root", vec![])), "valid");
}

#[test]
fn the_interpreter_dispatches_on_ranges_alternatives_and_conjunctions() {
    let source = "let describe(x: float64) = { if x is { in 0.0..1.0 => \"valid\" > 1.0 => \"too big\" < 0.0 => \"too small\" else => \"one\" } }\n\
                  let flag(n: int) = { if n is { < 0, > 100 => \"out\" > 0 && <= 10 => \"low\" } }";
    for (x, expected) in [
        (0.5, "valid"),
        (2.0, "too big"),
        (-1.0, "too small"),
        (1.0, "one"),
    ] {
        assert_eq!(
            text(run(source, "describe", vec![Value::Float(x)])),
            expected,
            "{x}"
        );
    }
    for (n, expected) in [(-5, "out"), (200, "out"), (3, "low")] {
        assert_eq!(
            text(run(source, "flag", vec![Value::Int(n)])),
            expected,
            "{n}"
        );
    }
    let none = run(source, "flag", vec![Value::Int(50)]);
    assert!(
        matches!(&none, Value::Array(items) if items.is_empty()),
        "no arm taken is the empty value: {none:?}"
    );
    assert_eq!(
        text(run(source, "describe", vec![Value::Float(f64::NAN)])),
        "one"
    );
}

#[test]
fn a_body_is_not_continued_by_the_next_arm() {
    let source = "let g(n: int) = { if n is { 1 => 5 -1 => 6 > 9 => 7 else => 8 } }";
    assert!(matches!(
        run(source, "g", vec![Value::Int(-1)]),
        Value::Int(6)
    ));
    assert!(matches!(
        run(source, "g", vec![Value::Int(10)]),
        Value::Int(7)
    ));
}
