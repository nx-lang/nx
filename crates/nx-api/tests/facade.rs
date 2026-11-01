//! An external caller can compile and execute with nx-api as its only NX import.
use nx_api::{
    build_program_artifact_from_source, codegen, eval_program_artifact, ir_runtime, EvalResult,
    ProgramBuildContext,
};
use std::collections::BTreeMap;

#[test]
fn facade_builds_emits_and_runs_both_execution_engines() {
    let artifact = build_program_artifact_from_source(
        "let root(): int = { 6 * 7 }",
        "main.nx",
        &ProgramBuildContext::empty(),
    )
    .expect("build program");
    let images = codegen::emit_nx_ir(
        &artifact,
        &codegen::NxIrEmitOptions {
            modules: Some(Vec::new()),
            ..Default::default()
        },
    )
    .expect("emit all modules, including the prelude");
    let modules: BTreeMap<_, _> = images
        .into_iter()
        .map(|image| {
            let module = ir_runtime::PreparedModule::prepare(image.bytes).expect("prepare image");
            (image.identity, module)
        })
        .collect();
    let program = ir_runtime::Program::link(
        &modules["main.nx"],
        |identity| modules.get(identity).cloned(),
        &ir_runtime::LinkOptions::default(),
    )
    .expect("link program");
    let compiled = program
        .evaluate_function("root", &[], &ir_runtime::RuntimeOptions::default())
        .expect("execute IR");
    let EvalResult::Ok(interpreted) = eval_program_artifact(&artifact) else {
        panic!("execute through the interpreter-backed API");
    };
    assert_eq!(compiled.to_json_string().unwrap(), "42");
    assert_eq!(compiled, interpreted);
}
