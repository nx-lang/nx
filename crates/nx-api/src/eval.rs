use crate::diagnostics::diagnostics_to_api;
use crate::value::to_nx_value;
use crate::NxDiagnostic;
use nx_diagnostics::{Diagnostic, Label, Severity};
use nx_hir::{lower, Item, Module, SourceId};
use nx_interpreter::{Interpreter, RuntimeError};
use nx_syntax::parse_str;
use nx_value::NxValue;
use text_size::{TextRange, TextSize};

/// The result of evaluating NX source code.
///
/// This is intentionally not a `Result` alias so that both variants carry domain-specific
/// types rather than a generic error trait.
pub enum EvalResult {
    /// Evaluation succeeded, producing an [`NxValue`].
    Ok(NxValue),
    /// Evaluation failed with one or more diagnostics (parse errors, missing root, runtime errors).
    Err(Vec<NxDiagnostic>),
}

pub(crate) fn lower_source_module(
    source: &str,
    file_name: &str,
) -> Result<Module, Vec<NxDiagnostic>> {
    let parse_result = parse_str(source, file_name);

    if parse_result
        .errors
        .iter()
        .any(|d| d.severity() == Severity::Error)
    {
        return Err(diagnostics_to_api(&parse_result.errors, source));
    }

    let tree = match parse_result.tree {
        Some(t) => t,
        None => {
            let diag = Diagnostic::error("parse-failed")
                .with_message("Failed to parse source")
                .build();
            return Err(diagnostics_to_api(&[diag], source));
        }
    };

    let source_id = SourceId::new(parse_result.source_id.as_u32());
    let module = lower(tree.root(), source_id);
    if !module.diagnostics().is_empty() {
        let diagnostics: Vec<_> = module
            .diagnostics()
            .iter()
            .map(|diagnostic| {
                Diagnostic::error("lowering-error")
                    .with_message(diagnostic.message.clone())
                    .with_label(Label::primary(file_name, diagnostic.span))
                    .build()
            })
            .collect();
        return Err(diagnostics_to_api(&diagnostics, source));
    }

    Ok(module)
}

pub(crate) fn runtime_error_diagnostics(source: &str, error: RuntimeError) -> Vec<NxDiagnostic> {
    let diag = Diagnostic::error("runtime-error")
        .with_message(error.to_string())
        .build();
    diagnostics_to_api(&[diag], source)
}

/// Parses and evaluates a self-contained NX source string, returning the result as an [`NxValue`].
///
/// The source must define a zero-argument `root()` function. That function is called and its
/// return value is converted to [`NxValue`] via [`to_nx_value`](crate::to_nx_value).
///
/// `file_name` is used only for display in diagnostic labels — it does not trigger any file I/O
/// or module resolution.
///
/// # Errors
///
/// Returns [`EvalResult::Err`] with diagnostics when:
/// - The source contains syntax errors
/// - No `root()` function is defined
/// - A runtime error occurs during evaluation
pub fn eval_source(source: &str, file_name: &str) -> EvalResult {
    let module = match lower_source_module(source, file_name) {
        Ok(module) => module,
        Err(diagnostics) => return EvalResult::Err(diagnostics),
    };

    let has_root = module
        .items()
        .iter()
        .any(|item| matches!(item, Item::Function(f) if f.name.as_str() == "root"));
    if !has_root {
        let source_len = u32::try_from(source.len())
            .expect("NX source size should be validated before evaluation diagnostics are created");
        let range = TextRange::new(TextSize::from(0), TextSize::from(source_len));
        let diag = Diagnostic::error("no-root")
            .with_message("No root element found in source")
            .with_label(Label::primary(file_name, range))
            .with_help("Add a top-level element to create an implicit root function.")
            .build();
        return EvalResult::Err(diagnostics_to_api(&[diag], source));
    }

    let interpreter = Interpreter::new();
    match interpreter.execute_function(&module, "root", vec![]) {
        Ok(value) => EvalResult::Ok(to_nx_value(&value)),
        Err(e) => EvalResult::Err(runtime_error_diagnostics(source, e)),
    }
}
