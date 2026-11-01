//! Program loading and independent value formatting.
use crate::nx_text::{
    annotate_canonical_nx_text, annotate_runtime_nx_text, FormatOptions, NxValueText,
};
use crate::NxDiagnostic;
use nx_diagnostics::{Diagnostic, Label, Severity};
use nx_hir::LoweredModule;
use nx_program::{
    build_library_artifact_from_directory, build_program_artifact_from_source, diagnostics_to_api,
    LibraryArtifact, ProgramArtifact, ProgramBuildContext,
};
use std::{fs, path::Path};
use text_size::{TextRange, TextSize};
pub(crate) fn build_source_program_artifact(
    source: &str,
    file_name: &str,
    build_context: &ProgramBuildContext,
) -> Result<ProgramArtifact, Vec<NxDiagnostic>> {
    let program =
        build_program_artifact_from_source(source, file_name, build_context).map_err(|error| {
            let diag = Diagnostic::error("library-load-error")
                .with_message(format!("Failed to load library imports: {}", error))
                .with_label(Label::primary(file_name, full_source_span(source)))
                .build();
            diagnostics_to_api(&[diag], source)
        })?;

    match program_artifact_error_diagnostics(&program, source) {
        Some(diagnostics) => Err(diagnostics),
        None => Ok(program),
    }
}

pub(crate) fn library_artifact_error_diagnostics(
    library: &LibraryArtifact,
) -> Option<Vec<NxDiagnostic>> {
    has_error_diagnostics(&library.diagnostics)
        .then(|| diagnostics_to_api(&library.diagnostics, ""))
}

/// Builds a reusable [`LibraryArtifact`] from a local directory and returns public diagnostics if
/// static analysis fails.
pub fn load_library_artifact_from_directory(
    root_path: impl AsRef<Path>,
) -> Result<LibraryArtifact, Vec<NxDiagnostic>> {
    let root_path = root_path.as_ref();
    let library = build_library_artifact_from_directory(root_path).map_err(|error| {
        let diagnostic = Diagnostic::error("library-load-error")
            .with_message(format!(
                "Failed to load library artifact from '{}': {}",
                root_path.display(),
                error
            ))
            .build();
        diagnostics_to_api(&[diagnostic], "")
    })?;

    match library_artifact_error_diagnostics(&library) {
        Some(diagnostics) => Err(diagnostics),
        None => Ok(library),
    }
}

pub(crate) fn program_artifact_error_diagnostics(
    program: &ProgramArtifact,
    fallback_source: &str,
) -> Option<Vec<NxDiagnostic>> {
    has_error_diagnostics(&program.diagnostics).then(|| {
        nx_program::program_diagnostics_to_api(program, &program.diagnostics, fallback_source)
    })
}

pub(crate) fn program_root_source(program: &ProgramArtifact) -> String {
    let Some(root_module) = program
        .root_modules
        .iter()
        .find(|module| module.file_name == program.entry_identity)
        .or_else(|| program.root_modules.first())
    else {
        return String::new();
    };

    if let Some(source) = program.source_text(&root_module.file_name) {
        source.to_string()
    } else if Path::new(&root_module.file_name).is_file() {
        fs::read_to_string(&root_module.file_name).unwrap_or_default()
    } else {
        String::new()
    }
}

fn has_error_diagnostics(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity() == Severity::Error)
}

fn full_source_span(source: &str) -> TextRange {
    let source_len = u32::try_from(source.len())
        .expect("NX source size should be validated before evaluation diagnostics are created");
    TextRange::new(TextSize::from(0), TextSize::from(source_len))
}

/// Why a program artifact could not spell a value.
#[derive(Debug, Clone, PartialEq)]
pub enum FormatValueError {
    /// The request was not a `{ value, type?, state? }` object, or the artifact has no entry
    /// module to read declarations from.
    Request(String),
    /// The value is not in the canonical format, or has no NX spelling: an `nx-value-invalid` or
    /// `nx-text-unspellable` diagnostic that names the value's pointer.
    Value(Vec<NxDiagnostic>),
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct FormatValueRequest {
    value: serde_json::Value,
    #[serde(rename = "type")]
    ty: Option<String>,
    state: Option<String>,
}

/// Spells a value in canonical JSON, as the IR runtimes return it, as NX text annotated against
/// the artifact's entry module, from a `{ value, type?, state? }` JSON request.
///
/// <para>The shared formatter spells canonical host values independently of execution. An
/// `ActionHandler` record is spelled as the record it is. `type` names the NX type of the whole value; `state` names the component whose
/// state the value is, which is spelled one `name=value` per line. The program is not
/// evaluated.</para>
pub fn format_program_artifact_value(
    program: &ProgramArtifact,
    request: &str,
) -> Result<NxValueText, FormatValueError> {
    let request: FormatValueRequest = serde_json::from_str(request).map_err(|error| {
        FormatValueError::Request(format!("Format request is not valid: {error}"))
    })?;
    let module = program
        .entry_module_id
        .and_then(|id| program.resolved_program.module(id))
        .map(|module| module.lowered_module.as_ref())
        .ok_or_else(|| FormatValueError::Request("The artifact has no entry module".to_string()))?;
    let source = program_root_source(program);
    let options = FormatOptions {
        ty: request.ty,
        state: request.state,
    };
    let libraries = library_modules(program, module);
    annotate_canonical_nx_text(&request.value, &options, module, &libraries, &source).map_err(
        |error| {
            let at = if error.pointer.is_empty() {
                "the value".to_string()
            } else {
                format!("`{}`", error.pointer)
            };
            let diag = Diagnostic::error(error.code)
                .with_message(format!("{} (at {at})", error.message))
                .build();
            FormatValueError::Value(diagnostics_to_api(&[diag], &source))
        },
    )
}

/// Every module of the program but `entry`: where the declarations of what `entry` imports are.
fn library_modules<'a>(
    program: &'a ProgramArtifact,
    entry: &LoweredModule,
) -> Vec<&'a LoweredModule> {
    program
        .resolved_program
        .modules()
        .iter()
        .map(|module| module.lowered_module.as_ref())
        .filter(|module| !std::ptr::eq(*module, entry))
        .collect()
}

/// Builds a reusable [`ProgramArtifact`] from source text and returns public diagnostics if static
/// analysis fails.
pub fn load_program_artifact_from_source(
    source: &str,
    file_name: &str,
    build_context: &ProgramBuildContext,
) -> Result<ProgramArtifact, Vec<NxDiagnostic>> {
    build_source_program_artifact(source, file_name, build_context)
}

/// Spells an already evaluated IR function result, preserving cases and function references.
/// This performs no execution. Number spelling follows the entry function's declared result type.
pub fn format_program_artifact_function_value(
    program: &ProgramArtifact,
    function: &str,
    value: &nx_ir_runtime::FunctionValue,
) -> Result<NxValueText, String> {
    let module = program
        .root_modules
        .iter()
        .find(|m| m.file_name == program.entry_identity)
        .ok_or("The program has no entry module")?;
    let lowered = module
        .lowered_module
        .as_ref()
        .ok_or("The entry module was not lowered")?;
    let ty = module
        .type_env
        .lookup(&nx_hir::Name::new(function))
        .and_then(nx_types::Type::function_parts)
        .map(|(_, result)| result.unconstrained().to_string());
    annotate_runtime_nx_text(
        value.view(),
        ty.as_deref(),
        lowered,
        &library_modules(program, lowered),
        &program_root_source(program),
    )
}
