//! The interpreter runs programs with value constraints, through a real build: it reports every
//! diagnostic the type checker reports and does not check values when the program runs.

use nx_api::{eval_source, EvalResult, ProgramBuildContext};
use nx_value::NxValue;

fn evaluate(source: &str) -> Result<NxValue, Vec<String>> {
    match eval_source(source, "constraints.nx", &ProgramBuildContext::empty()) {
        EvalResult::Ok(value) => Ok(value),
        EvalResult::Err(diagnostics) => Err(diagnostics
            .iter()
            .map(|diagnostic| format!("{:?}: {}", diagnostic.code, diagnostic.message))
            .collect()),
    }
}

#[test]
fn the_interpreter_accepts_a_constrained_program() {
    let source = "type Ratio = float32 where in 0.0..=1.0\nlet r: Ratio = 0.5\nlet root() = { r }";
    match evaluate(source) {
        Ok(NxValue::Float32(value)) => assert_eq!(value, 0.5),
        Ok(NxValue::Float(value)) => assert_eq!(value, 0.5),
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_interpreter_reports_a_constraint_violation_before_it_runs() {
    let source = "type Ratio = float32 where in 0.0..=1.0\nlet r: Ratio = 1.5\nlet root() = { r }";
    match evaluate(source) {
        Err(diagnostics) => assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("constraint-violation")),
            "{diagnostics:?}"
        ),
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_interpreter_matches_a_range_pattern() {
    let source = "let root() = { if 0.5 is { in 0.0..1.0 => \"valid\" else => \"no\" } }";
    match evaluate(source) {
        Ok(NxValue::String(text)) => assert_eq!(text, "valid"),
        other => panic!("{other:?}"),
    }
}
