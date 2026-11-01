//! The read-only IR result view keeps identities erased by canonical output.
use nx_api::codegen::{emit_nx_ir, NxIrEmitOptions};
use nx_api::ir_runtime::{
    FunctionValue, LinkOptions, PreparedModule, Program, RuntimeOptions, ValueKind,
};
use nx_api::{load_program_artifact_from_source, ProgramBuildContext};
use nx_value::NxValue;
use std::collections::BTreeMap;

fn program(source: &str) -> Program {
    let artifact =
        load_program_artifact_from_source(source, "main.nx", &ProgramBuildContext::empty())
            .unwrap();
    let modules = emit_nx_ir(
        &artifact,
        &NxIrEmitOptions {
            modules: Some(Vec::new()),
            debug: true,
        },
    )
    .unwrap()
    .into_iter()
    .map(|image| {
        (
            image.identity,
            PreparedModule::prepare(image.bytes).unwrap(),
        )
    })
    .collect::<BTreeMap<_, _>>();
    Program::link(
        &modules["main.nx"],
        |identity| modules.get(identity).cloned(),
        &LinkOptions::default(),
    )
    .unwrap()
}

fn evaluate(source: &str) -> FunctionValue {
    program(source)
        .evaluate_function_value("root", &[], &RuntimeOptions::default())
        .unwrap()
}

#[test]
fn a_case_at_an_object_site_keeps_its_union_identity() {
    let value = evaluate("type Status = ready | waiting\nlet root(): object = {Status.ready}");
    assert_eq!(value.canonical(), &NxValue::String("ready".into()));
    let ValueKind::Case { union, case } = value.view().kind() else {
        panic!("expected a case")
    };
    assert_eq!((union, case), ("Status", "ready"));
}

#[test]
fn nested_results_expose_cases_functions_and_sequences_without_captures() {
    let value = evaluate(
        r#"
        type Status = ready | waiting
        let label(s:Status): string = { "label" }
        type Result = { status:object callback:object items:int+ }
        let root() = { <Result status={Status.ready} callback={label} items={1 2} /> }
    "#,
    );
    let ValueKind::Record { type_name, fields } = value.view().kind() else {
        panic!("record")
    };
    assert_eq!(type_name, Some("Result"));
    let fields: BTreeMap<_, _> = fields.collect();
    assert!(matches!(
        fields["status"].kind(),
        ValueKind::Case {
            union: "Status",
            case: "ready"
        }
    ));
    assert!(matches!(
        fields["callback"].kind(),
        ValueKind::Function {
            module: "main.nx",
            name: "label"
        }
    ));
    let ValueKind::Sequence(items) = fields["items"].kind() else {
        panic!("sequence")
    };
    assert_eq!(
        items
            .map(|item| match item.kind() {
                ValueKind::Int(n) => n,
                _ => panic!("int"),
            })
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    let canonical = value.canonical().to_json_string().unwrap();
    assert!(canonical.contains("\"$type\":\"Function\""));
}

#[test]
fn float32_is_the_existing_rounded_float64_carrier() {
    let value = evaluate("let narrow(): float32 = {0.1}\nlet root(): object = {narrow()}");
    let ValueKind::Float(number) = value.view().kind() else {
        panic!("float")
    };
    assert_eq!(number, f64::from(0.1_f32));
    assert_eq!(value.canonical(), &NxValue::Float(number));
}

#[test]
fn canonical_optional_absence_still_differs_from_the_empty_sequence_view() {
    let value = evaluate("let root(): int? = {} ");
    assert_eq!(value.canonical(), &NxValue::Null);
    let ValueKind::Sequence(mut items) = value.view().kind() else {
        panic!("empty sequence")
    };
    assert!(items.next().is_none());
}

#[test]
fn rich_and_canonical_calls_share_resource_accounting_and_reset() {
    let program = program("let root() = {1 + 2}");
    let limited = RuntimeOptions {
        max_operations: Some(0),
        ..RuntimeOptions::default()
    };
    let canonical_error = program
        .evaluate_function("root", &[], &limited)
        .unwrap_err();
    let rich_error = program
        .evaluate_function_value("root", &[], &limited)
        .unwrap_err();
    assert_eq!(rich_error, canonical_error);
    assert_eq!(rich_error.code(), "nx-ir-resource-limit");
    let value = program
        .evaluate_function_value("root", &[], &RuntimeOptions::default())
        .unwrap();
    assert_eq!(value.canonical(), &NxValue::Int(3));
}

fn formatted(source: &str) -> nx_api::NxValueText {
    let artifact =
        load_program_artifact_from_source(source, "main.nx", &ProgramBuildContext::empty())
            .unwrap();
    nx_api::format_program_artifact_function_value(&artifact, "root", &evaluate(source)).unwrap()
}

#[test]
fn rich_formatting_preserves_cases_and_functions_at_object_sites() {
    assert_eq!(
        formatted("type Status = ready | waiting\nlet root(): object = {Status.ready}").text,
        "Status.ready"
    );
    assert_eq!(
        formatted("let answer() = {42}\nlet root(): object = {answer}").text,
        "answer"
    );
}

#[test]
fn rich_formatting_reads_numeric_width_from_the_declared_site() {
    assert_eq!(formatted("let root(): float32 = {0.1}").text, "0.1");
    assert_eq!(
        formatted("let narrow(): float32 = {0.1}\nlet root(): object = {narrow()}").text,
        "0.10000000149011612"
    );
    assert_eq!(
        formatted("type Size = { ratio:float32 }\nlet root() = <Size ratio=0.1 />").text,
        "<Size ratio=0.1 />"
    );
}

#[test]
fn int32_arithmetic_uses_the_existing_wider_integer_carrier() {
    let value = evaluate("let twice(n:int32): int = {n + n}\nlet root() = {twice(2000000000)}");
    assert_eq!(value.canonical(), &NxValue::Int(4_000_000_000));
    assert_eq!(
        formatted("let root(): int32 = {1}").nodes[0].ty.as_deref(),
        Some("int32")
    );
}

#[test]
fn constrained_float32_sites_keep_their_numeric_spelling() {
    let source = "type Ratio = float32 where in 0.0..=1.0\nlet root(): Ratio = {0.1}";
    assert_eq!(formatted(source).text, "0.1");
    let artifact =
        load_program_artifact_from_source(source, "main.nx", &ProgramBuildContext::empty())
            .unwrap();
    let request = serde_json::json!({"value": f64::from(0.1_f32), "type": "Ratio"});
    assert_eq!(
        nx_api::format_program_artifact_value(&artifact, &request.to_string())
            .unwrap()
            .text,
        "0.1"
    );
    assert_eq!(
        formatted(
            "type Size = { ratio:float32 where in 0.0..=1.0 }\nlet root() = <Size ratio=0.1 />"
        )
        .text,
        "<Size ratio=0.1 />"
    );
}
