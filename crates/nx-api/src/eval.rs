use crate::artifacts::{
    build_library_artifact_from_directory, build_program_artifact_from_source, LibraryArtifact,
    ProgramArtifact, ProgramBuildContext,
};
use crate::diagnostics::diagnostics_to_api;
use crate::value::to_nx_value;
use crate::NxDiagnostic;
use nx_diagnostics::{Diagnostic, Label, Severity};
use nx_hir::Item;
use nx_interpreter::{Interpreter, RuntimeError};
use nx_value::NxValue;
use std::fs;
use std::path::Path;
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

pub(crate) fn runtime_error_diagnostics(source: &str, error: RuntimeError) -> Vec<NxDiagnostic> {
    let diag = Diagnostic::error("runtime-error")
        .with_message(error.to_string())
        .build();
    diagnostics_to_api(&[diag], source)
}

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
    has_error_diagnostics(&program.diagnostics)
        .then(|| diagnostics_to_api(&program.diagnostics, fallback_source))
}

pub(crate) fn program_root_source(program: &ProgramArtifact) -> String {
    let Some(root_module) = program.root_modules.first() else {
        return String::new();
    };

    if Path::new(&root_module.file_name).is_file() {
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

fn no_root_diagnostics(file_name: &str, source: &str) -> Vec<NxDiagnostic> {
    let diag = Diagnostic::error("no-root")
        .with_message("No root element found in source")
        .with_label(Label::primary(file_name, full_source_span(source)))
        .with_help("Add a top-level element to create an implicit root function.")
        .build();
    diagnostics_to_api(&[diag], source)
}

fn eval_program_artifact_with_source(program: &ProgramArtifact, source: &str) -> EvalResult {
    if let Some(diagnostics) = program_artifact_error_diagnostics(program, source) {
        return EvalResult::Err(diagnostics);
    }

    let Some(root_module) = program.root_modules.first() else {
        return EvalResult::Err(no_root_diagnostics("input.nx", source));
    };
    let Some(module) = root_module.lowered_module.as_ref() else {
        return EvalResult::Err(no_root_diagnostics(&root_module.file_name, source));
    };

    let has_root = module
        .items()
        .iter()
        .any(|item| matches!(item, Item::Function(f) if f.name.as_str() == "root"));
    if !has_root {
        return EvalResult::Err(no_root_diagnostics(&root_module.file_name, source));
    }

    let interpreter = Interpreter::from_resolved_program(program.resolved_program.clone());
    match interpreter.execute_resolved_program_function("root", vec![]) {
        Ok(value) => EvalResult::Ok(to_nx_value(&value)),
        Err(error) => EvalResult::Err(runtime_error_diagnostics(source, error)),
    }
}

/// Evaluates the `root()` entrypoint of a previously built [`ProgramArtifact`].
///
/// The supplied program artifact should already be free of static-analysis errors.
pub fn eval_program_artifact(program: &ProgramArtifact) -> EvalResult {
    let source = program_root_source(program);
    eval_program_artifact_with_source(program, &source)
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

/// Runs shared static analysis and then evaluates a self-contained NX source string, returning the
/// result as an [`NxValue`].
///
/// The source must define a zero-argument `root()` function. That function is called and its
/// return value is converted to [`NxValue`] via [`to_nx_value`](crate::to_nx_value).
///
/// `file_name` is used for diagnostic labels. Callers must supply the `ProgramBuildContext` used
/// to resolve any imported libraries during program construction.
///
/// # Errors
///
/// Returns [`EvalResult::Err`] with diagnostics when:
/// - Static analysis reports errors
/// - No `root()` function is defined
/// - A runtime error occurs during evaluation
pub fn eval_source(
    source: &str,
    file_name: &str,
    build_context: &ProgramBuildContext,
) -> EvalResult {
    let program = match load_program_artifact_from_source(source, file_name, build_context) {
        Ok(program) => program,
        Err(diagnostics) => return EvalResult::Err(diagnostics),
    };

    eval_program_artifact_with_source(&program, source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifacts::{build_program_artifact_from_source, LibraryRegistry};
    use std::collections::BTreeMap;
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

        let EvalResult::Err(diagnostics) = eval_source(
            source,
            "eval-static-errors.nx",
            &ProgramBuildContext::empty(),
        ) else {
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

        let EvalResult::Err(diagnostics) =
            eval_source(source, "virtual/main.nx", &ProgramBuildContext::empty())
        else {
            panic!("Expected virtual import source to fail");
        };

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_deref() == Some("library-imports-require-path")));
    }

    #[test]
    fn eval_source_resolves_preloaded_local_imports() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("button.nx"),
            r#"export let <Button /> = <button />"#,
        )
        .expect("ui file");
        let main_path = app_dir.join("main.nx");
        let source = r#"import { Button as Layout.Button } from "../ui"
let root() = { <Layout.Button /> }"#;
        fs::write(&main_path, source).expect("main file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected registry preload");
        let build_context = registry.build_context();

        let EvalResult::Ok(value) =
            eval_source(source, &main_path.display().to_string(), &build_context)
        else {
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

    #[test]
    fn eval_source_preserves_external_component_identity_props_and_handlers() {
        let source = r#"
            action SearchRequested = { query:string }
            action DoSearch = { query:string }

            external component <SearchBox placeholder:string = "Find docs" emits { SearchRequested } />
            let root() = { <SearchBox onSearchRequested=<DoSearch query={action.query} /> /> }
        "#;

        let EvalResult::Ok(value) = eval_source(
            source,
            "external-component-root.nx",
            &ProgramBuildContext::empty(),
        ) else {
            panic!("Expected external component evaluation to succeed");
        };

        assert_eq!(
            value,
            NxValue::Record {
                type_name: Some("SearchBox".to_string()),
                properties: BTreeMap::from([
                    (
                        "onSearchRequested".to_string(),
                        NxValue::Record {
                            type_name: Some("ActionHandler".to_string()),
                            properties: BTreeMap::from([
                                (
                                    "action".to_string(),
                                    NxValue::String("SearchRequested".to_string()),
                                ),
                                (
                                    "component".to_string(),
                                    NxValue::String("SearchBox".to_string()),
                                ),
                                (
                                    "emit".to_string(),
                                    NxValue::String("SearchRequested".to_string()),
                                ),
                            ]),
                        },
                    ),
                    (
                        "placeholder".to_string(),
                        NxValue::String("Find docs".to_string()),
                    ),
                ]),
            }
        );
    }

    #[test]
    fn eval_program_artifact_applies_imported_abstract_base_component_defaults() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("base.nx"),
            r#"
                export let defaultPlaceholder(): string = { "Find docs" }
                export abstract component <SearchBase prefix:string = {defaultPlaceholder()} placeholder:string = {prefix} />
            "#,
        )
        .expect("ui file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected registry preload");
        let build_context = registry.build_context();

        let main_path = app_dir.join("main.nx");
        let source = r#"
            import "../ui"

            let defaultPlaceholder(): string = { "Wrong" }

            component <SearchBox extends SearchBase /> = {
              state { query:string = {placeholder} }
              <TextInput value={query} placeholder={placeholder} />
            }

            let root() = { <SearchBox /> }
        "#;
        fs::write(&main_path, source).expect("main file");

        let program = build_program_artifact_from_source(
            source,
            &main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected program artifact");

        let EvalResult::Ok(value) = eval_program_artifact(&program) else {
            panic!("Expected imported inherited component defaults to evaluate");
        };

        assert_eq!(
            value,
            NxValue::Record {
                type_name: Some("SearchBox".to_string()),
                properties: BTreeMap::from([
                    (
                        "placeholder".to_string(),
                        NxValue::String("Find docs".to_string()),
                    ),
                    (
                        "prefix".to_string(),
                        NxValue::String("Find docs".to_string()),
                    ),
                ]),
            }
        );
    }

    #[test]
    fn eval_program_artifact_applies_imported_abstract_record_defaults() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("base.nx"),
            r#"
                export let defaultPlaceholder(): string = { "Find docs" }
                export abstract type SearchConfigBase = {
                  prefix: string = {defaultPlaceholder()}
                  placeholder: string = {prefix}
                }
            "#,
        )
        .expect("ui file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected registry preload");
        let build_context = registry.build_context();

        let main_path = app_dir.join("main.nx");
        let source = r#"
            import "../ui"

            let defaultPlaceholder(): string = { "Wrong" }

            type SearchConfig extends SearchConfigBase = {
              showSearchIcon: bool = true
            }

            let root() = { SearchConfig() }
        "#;
        fs::write(&main_path, source).expect("main file");

        let program = build_program_artifact_from_source(
            source,
            &main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected program artifact");

        let EvalResult::Ok(value) = eval_program_artifact(&program) else {
            panic!("Expected imported inherited record defaults to evaluate");
        };

        assert_eq!(
            value,
            NxValue::Record {
                type_name: Some("SearchConfig".to_string()),
                properties: BTreeMap::from([
                    (
                        "placeholder".to_string(),
                        NxValue::String("Find docs".to_string())
                    ),
                    (
                        "prefix".to_string(),
                        NxValue::String("Find docs".to_string())
                    ),
                    ("showSearchIcon".to_string(), NxValue::Bool(true)),
                ]),
            }
        );
    }

    #[test]
    fn eval_program_artifact_executes_root_across_imported_modules() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("answer.nx"),
            r#"export let answer(): int = { 42 }"#,
        )
        .expect("ui file");
        let main_path = app_dir.join("main.nx");
        let source = r#"import "../ui"
let root() = { answer() }"#;
        fs::write(&main_path, source).expect("main file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected registry preload");
        let build_context = registry.build_context();

        let program = build_program_artifact_from_source(
            source,
            &main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected program artifact");

        let EvalResult::Ok(value) = eval_program_artifact(&program) else {
            panic!("Expected program artifact evaluation to succeed");
        };

        assert_eq!(value, NxValue::Int(42));
    }
}
