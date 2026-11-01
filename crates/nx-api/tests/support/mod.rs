//! Source execution is test composition, never a production facade API.
#![allow(dead_code)]
use nx_api::codegen::{emit_nx_ir, NxIrEmitOptions};
use nx_api::ir_runtime::{LinkOptions, PreparedModule, Program, RuntimeOptions};
use nx_api::{NxDiagnostic, ProgramArtifact, ProgramBuildContext};
use nx_value::NxValue;
use std::collections::BTreeMap;

#[derive(Debug)]
pub enum EvalResult {
    Ok(NxValue),
    Err(Vec<NxDiagnostic>),
}

pub fn link(program: &ProgramArtifact, identity: &str) -> Result<Program, Vec<NxDiagnostic>> {
    if program
        .diagnostics
        .iter()
        .any(|d| d.severity() == nx_diagnostics::Severity::Error)
    {
        return Err(nx_api::program_diagnostics_to_api(
            program,
            &program.diagnostics,
            "",
        ));
    }
    let images = emit_nx_ir(
        program,
        &NxIrEmitOptions {
            modules: Some(Vec::new()),
            debug: true,
        },
    )
    .map_err(|error| nx_api::program_diagnostics_to_api(program, &error.diagnostics, ""))?;
    let modules = images
        .into_iter()
        .map(|image| PreparedModule::prepare(image.bytes).map(|m| (image.identity, m)))
        .collect::<Result<BTreeMap<_, _>, _>>()
        .map_err(|e| diagnostics(program, e))?;
    let entry = modules
        .get(identity)
        .ok_or_else(|| diagnostic("no-root", "No root module found"))?;
    Program::link(
        entry,
        |id| modules.get(id).cloned(),
        &LinkOptions::default(),
    )
    .map_err(|e| diagnostics(program, e))
}

pub fn diagnostic(code: &str, message: &str) -> Vec<NxDiagnostic> {
    vec![nx_api::NxDiagnostic {
        severity: nx_api::NxSeverity::Error,
        code: Some(code.to_string()),
        message: message.to_string(),
        labels: Vec::new(),
        note: None,
        help: None,
    }]
}

pub fn diagnostics(
    program: &ProgramArtifact,
    error: nx_api::ir_runtime::NxIrRuntimeError,
) -> Vec<NxDiagnostic> {
    error
        .diagnostics
        .into_iter()
        .flat_map(|d| {
            let mut diag = nx_diagnostics::Diagnostic::error(d.code).with_message(d.message);
            if let Some(source) = d.source {
                diag = diag.with_label(nx_diagnostics::Label::primary(
                    source.identity,
                    text_size::TextRange::new(source.start.into(), source.end.into()),
                ));
            }
            nx_api::program_diagnostics_to_api(program, &[diag.build()], "")
        })
        .collect()
}

pub fn eval_program_artifact_function(
    program: &ProgramArtifact,
    identity: &str,
    name: &str,
) -> EvalResult {
    let result = link(program, identity).and_then(|p| {
        p.evaluate_function(name, &[], &RuntimeOptions::default())
            .map_err(|e| diagnostics(program, e))
    });
    match result {
        Ok(v) => EvalResult::Ok(v),
        Err(d) => EvalResult::Err(d),
    }
}
pub fn eval_program_artifact(program: &ProgramArtifact) -> EvalResult {
    eval_program_artifact_function(program, &program.entry_identity, "root")
}
pub fn eval_source(source: &str, file: &str, context: &ProgramBuildContext) -> EvalResult {
    match nx_api::load_program_artifact_from_source(source, file, context) {
        Ok(p) => eval_program_artifact(&p),
        Err(d) => EvalResult::Err(d),
    }
}
pub fn eval_program_artifact_nx_text(
    program: &ProgramArtifact,
) -> Result<nx_api::NxValueText, Vec<NxDiagnostic>> {
    let p = link(program, &program.entry_identity)?;
    let value = p
        .evaluate_function_value("root", &[], &RuntimeOptions::default())
        .map_err(|e| diagnostics(program, e))?;
    nx_api::format_program_artifact_function_value(program, "root", &value)
        .map_err(|message| diagnostic("nx-text-unspellable", &message))
}

pub mod source;
