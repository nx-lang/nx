use crate::diagnostics::diagnostics_to_api;
use crate::value::to_nx_value;
use crate::NxDiagnostic;
use nx_diagnostics::{Diagnostic, Label};
use nx_hir::{Item, Module};
use nx_interpreter::{Interpreter, RuntimeError};
use nx_types::{analyze_str, analyze_str_with_path};
use nx_value::NxValue;
use std::path::Path;
use std::sync::Arc;
use text_size::{TextRange, TextSize};

/// The result of evaluating NX source code.
///
/// This is intentionally not a `Result` alias so that both variants carry domain-specific
/// types rather than a generic error trait.
pub enum EvalResult {
    /// Evaluation succeeded, producing an [`NxValue`].
    Ok(NxValue),
    /// Evaluation failed with one or more diagnostics (static analysis, missing root, runtime
    /// errors).
    Err(Vec<NxDiagnostic>),
}

pub(crate) fn analyze_source_module(
    source: &str,
    file_name: &str,
) -> Result<Arc<Module>, Vec<NxDiagnostic>> {
    let source_path = Path::new(file_name);
    let analysis = if source_path.exists() {
        analyze_str_with_path(source, file_name, source_path)
    } else {
        analyze_str(source, file_name)
    };

    if !analysis.is_ok() || analysis.module.is_none() {
        return Err(diagnostics_to_api(&analysis.diagnostics, source));
    }

    Ok(analysis.module.expect("checked above"))
}

pub(crate) fn runtime_error_diagnostics(source: &str, error: RuntimeError) -> Vec<NxDiagnostic> {
    let diag = Diagnostic::error("runtime-error")
        .with_message(error.to_string())
        .build();
    diagnostics_to_api(&[diag], source)
}

/// Runs shared static analysis and then evaluates a self-contained NX source string, returning the
/// result as an [`NxValue`].
///
/// The source must define a zero-argument `root()` function. That function is called and its
/// return value is converted to [`NxValue`] via [`to_nx_value`](crate::to_nx_value).
///
/// `file_name` is used for diagnostic labels. If it points to an on-disk source file, local
/// library imports are resolved relative to that path before runtime execution.
///
/// # Errors
///
/// Returns [`EvalResult::Err`] with diagnostics when:
/// - Static analysis reports errors
/// - No `root()` function is defined
/// - A runtime error occurs during evaluation
pub fn eval_source(source: &str, file_name: &str) -> EvalResult {
    let module = match analyze_source_module(source, file_name) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn eval_source_returns_aggregated_static_diagnostics_before_runtime_execution() {
        let source = r#"
            abstract type Entity = {
              id: int
            }

            type User extends Entity = {
              name: string
            }

            type Admin extends User = {
              level: int
            }

            let broken(): int = "oops"
            let root(): int = { 1 / 0 }
        "#;

        let EvalResult::Err(diagnostics) = eval_source(source, "eval-static-errors.nx") else {
            panic!("Expected evaluation to stop on static analysis diagnostics");
        };

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_deref() == Some("lowering-error")));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_deref() == Some("return-type-mismatch")));
        assert!(!diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_deref() == Some("runtime-error")));
    }

    #[test]
    fn eval_source_reports_imports_require_path_when_source_is_not_on_disk() {
        let source = r#"import { Button as Layout.Button } from "../ui"
let root() = { <Layout.Button /> }"#;

        let EvalResult::Err(diagnostics) = eval_source(source, "virtual/main.nx") else {
            panic!("Expected virtual import source to fail");
        };

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_deref() == Some("library-imports-require-path")));
    }

    #[test]
    fn eval_source_resolves_local_imports_when_file_name_points_to_real_path() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(ui_dir.join("button.nx"), r#"let <Button /> = <button />"#).expect("ui file");
        let main_path = app_dir.join("main.nx");
        let source = r#"import { Button as Layout.Button } from "../ui"
let root() = { <Layout.Button /> }"#;
        fs::write(&main_path, source).expect("main file");

        let EvalResult::Ok(value) = eval_source(source, &main_path.display().to_string()) else {
            panic!("Expected import-backed source evaluation to succeed");
        };

        assert_eq!(
            value,
            NxValue::Record {
                type_name: Some("button".to_string()),
                properties: Default::default(),
            }
        );
    }
}
