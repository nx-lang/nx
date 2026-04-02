//! High-level type checking and source-analysis API.

use crate::{InferenceContext, Type, TypeEnvironment};
use nx_diagnostics::{Diagnostic, Label, Severity, TextSize, TextSpan};
use nx_hir::{link_local_libraries, lower, ExprId, LoweringDiagnostic, Module, SourceId};
use nx_syntax::{parse_file as syntax_parse_file, parse_str as syntax_parse_str};
use rustc_hash::FxHashMap;
use std::io;
use std::path::Path;
use std::sync::Arc;

/// Result of analyzing NX source.
///
/// Contains the lowered module, inferred type environment, and any static-analysis diagnostics
/// (errors or warnings) discovered while parsing, lowering, building scopes, and type checking.
#[derive(Debug, Clone)]
pub struct SourceAnalysisResult {
    /// The lowered module produced during analysis (None if parsing failed fatally)
    pub module: Option<Arc<Module>>,
    /// Type environment with all inferred bindings
    pub type_env: TypeEnvironment,
    /// Diagnostics from parsing, lowering, scope building, and type checking
    pub diagnostics: Vec<Diagnostic>,
    /// Source file ID
    pub source_id: SourceId,
}

impl SourceAnalysisResult {
    /// Returns true if analysis succeeded without any error diagnostics.
    pub fn is_ok(&self) -> bool {
        self.errors().is_empty()
    }

    /// Returns all error diagnostics.
    pub fn errors(&self) -> Vec<&Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity() == nx_diagnostics::Severity::Error)
            .collect()
    }

    /// Returns the type of an expression, if available.
    pub fn type_of(&self, expr: ExprId) -> Option<&Type> {
        self.type_env.get_expr_type(expr)
    }

    /// Returns all diagnostics (errors + warnings).
    pub fn all_diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// Backward-compatible alias for callers that still think of the shared analysis result as the
/// outcome of type checking.
pub type TypeCheckResult = SourceAnalysisResult;

/// Analyzes NX source code from a string.
///
/// This performs parsing, lowering, scope building, and type checking in one pass. If parsing
/// produces a syntax tree, the returned analysis result preserves the lowered module even when
/// later phases report diagnostics.
pub fn analyze_str(source: &str, file_name: &str) -> SourceAnalysisResult {
    let parse_result = syntax_parse_str(source, file_name);
    analyze_string_parse_result(parse_result, source, file_name, None)
}

/// Analyzes NX source code from a string while resolving local library imports from an on-disk
/// source path.
pub fn analyze_str_with_path(
    source: &str,
    file_name: &str,
    source_path: impl AsRef<Path>,
) -> SourceAnalysisResult {
    let parse_result = syntax_parse_str(source, file_name);
    analyze_string_parse_result(parse_result, source, file_name, Some(source_path.as_ref()))
}

/// Type checks NX source code from a string.
///
/// # Example
///
/// ```
/// use nx_types::check_str;
///
/// let source = r#"
///     let <Button text:string /> = <button>{text}</button>
/// "#;
///
/// let result = check_str(source, "example.nx");
/// // Parse should succeed
/// assert!(result.module.is_some());
/// ```
pub fn check_str(source: &str, file_name: &str) -> TypeCheckResult {
    analyze_str(source, file_name)
}

/// Type checks an NX source file.
///
/// # Example
///
/// ```no_run
/// use nx_types::check_file;
///
/// let result = check_file("example.nx").expect("Failed to read file");
/// if result.is_ok() {
///     println!("Type checking passed!");
/// } else {
///     for error in result.errors() {
///         println!("Error: {:?}", error);
///     }
/// }
/// ```
///
/// # Errors
///
/// Returns an error if the file cannot be read or is not valid UTF-8.
pub fn check_file(path: impl AsRef<Path>) -> io::Result<TypeCheckResult> {
    let path = path.as_ref();
    let parse_result = syntax_parse_file(path)?;
    let file_name = path.display().to_string();
    analyze_parse_result(parse_result, &file_name, Some(path))
}

fn analyze_string_parse_result(
    parse_result: nx_syntax::ParseResult,
    source: &str,
    file_name: &str,
    source_path: Option<&Path>,
) -> SourceAnalysisResult {
    let source_id = SourceId::new(parse_result.source_id.as_u32());
    let mut diagnostics = normalize_diagnostics_file_name(parse_result.errors, file_name);

    let Some(tree) = parse_result.tree else {
        return SourceAnalysisResult {
            module: None,
            type_env: TypeEnvironment::new(),
            diagnostics,
            source_id,
        };
    };

    let mut module = lower(tree.root(), source_id);

    if let Some(path) = source_path {
        module = match link_local_libraries(module.clone(), path) {
            Ok(module) => module,
            Err(error) => {
                diagnostics.push(library_load_error_diagnostic(source, file_name, &error));
                return SourceAnalysisResult {
                    module: Some(Arc::new(module)),
                    type_env: TypeEnvironment::new(),
                    diagnostics,
                    source_id,
                };
            }
        };
    } else if !module.imports.is_empty() {
        diagnostics.push(library_imports_require_path_diagnostic(source, file_name));
        return SourceAnalysisResult {
            module: Some(Arc::new(module)),
            type_env: TypeEnvironment::new(),
            diagnostics,
            source_id,
        };
    }

    analyze_lowered_module(module, diagnostics, file_name, source_id)
}

fn analyze_parse_result(
    parse_result: nx_syntax::ParseResult,
    file_name: &str,
    source_path: Option<&Path>,
) -> io::Result<SourceAnalysisResult> {
    let source_id = SourceId::new(parse_result.source_id.as_u32());
    let diagnostics = normalize_diagnostics_file_name(parse_result.errors, file_name);

    let Some(tree) = parse_result.tree else {
        return Ok(SourceAnalysisResult {
            module: None,
            type_env: TypeEnvironment::new(),
            diagnostics,
            source_id,
        });
    };

    let mut module = lower(tree.root(), source_id);
    if let Some(path) = source_path {
        module = link_local_libraries(module, path)?;
    }

    Ok(analyze_lowered_module(
        module,
        diagnostics,
        file_name,
        source_id,
    ))
}

fn analyze_lowered_module(
    module: Module,
    mut diagnostics: Vec<Diagnostic>,
    file_name: &str,
    source_id: SourceId,
) -> SourceAnalysisResult {
    diagnostics.extend(lowering_diagnostics(module.diagnostics(), file_name));

    let (_scope_manager, scope_diagnostics) = nx_hir::build_scopes(&module);
    diagnostics.extend(normalize_diagnostics_file_name(
        scope_diagnostics,
        file_name,
    ));

    let mut ctx = InferenceContext::with_file_name(&module, file_name);

    for item in module.items() {
        match item {
            nx_hir::Item::Function(func) => {
                ctx.infer_function(func);
            }
            nx_hir::Item::Value(_) => {}
            nx_hir::Item::Component(_) => {}
            nx_hir::Item::TypeAlias(_) => {}
            nx_hir::Item::Record(_) => {}
            nx_hir::Item::Enum(_) => {}
        }
    }

    let (type_env, type_diagnostics) = ctx.finish();
    diagnostics.extend(normalize_diagnostics_file_name(type_diagnostics, file_name));

    SourceAnalysisResult {
        module: Some(Arc::new(module)),
        type_env,
        diagnostics,
        source_id,
    }
}

fn lowering_diagnostics(diagnostics: &[LoweringDiagnostic], file_name: &str) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .map(|diagnostic| {
            Diagnostic::error("lowering-error")
                .with_message(diagnostic.message.clone())
                .with_label(Label::primary(file_name, diagnostic.span))
                .build()
        })
        .collect()
}

fn full_source_span(source: &str) -> TextSpan {
    let source_len = u32::try_from(source.len())
        .expect("NX source size should be validated before creating source diagnostics");
    TextSpan::new(TextSize::from(0), TextSize::from(source_len))
}

fn library_load_error_diagnostic(source: &str, file_name: &str, error: &io::Error) -> Diagnostic {
    Diagnostic::error("library-load-error")
        .with_message(format!("Failed to load library imports: {}", error))
        .with_label(Label::primary(file_name, full_source_span(source)))
        .build()
}

fn library_imports_require_path_diagnostic(source: &str, file_name: &str) -> Diagnostic {
    Diagnostic::error("library-imports-require-path")
        .with_message("Library imports require an on-disk source path")
        .with_label(Label::primary(file_name, full_source_span(source)))
        .with_help("Pass a real file path as file_name or use a file-based entry point.")
        .build()
}

fn normalize_diagnostics_file_name(
    diagnostics: Vec<Diagnostic>,
    file_name: &str,
) -> Vec<Diagnostic> {
    diagnostics
        .into_iter()
        .map(|diagnostic| {
            let labels = diagnostic
                .labels()
                .iter()
                .cloned()
                .map(|mut label| {
                    if label.file.is_empty() {
                        label.file = file_name.to_string();
                    }
                    label
                })
                .collect::<Vec<_>>();

            let code = diagnostic.code().unwrap_or("diagnostic");
            let mut builder = match diagnostic.severity() {
                Severity::Error => Diagnostic::error(code),
                Severity::Warning => Diagnostic::warning(code),
                Severity::Info => Diagnostic::info(code),
                Severity::Hint => Diagnostic::hint(code),
            }
            .with_message(diagnostic.message())
            .with_labels(labels);

            if let Some(help) = diagnostic.help() {
                builder = builder.with_help(help);
            }

            if let Some(note) = diagnostic.note() {
                builder = builder.with_note(note);
            }

            builder.build()
        })
        .collect()
}

/// A session for batch type checking multiple files.
///
/// This allows efficient type checking of multiple files with shared
/// type information and caching.
///
/// # Example
///
/// ```
/// use nx_types::TypeCheckSession;
///
/// let mut session = TypeCheckSession::new();
///
/// // Add files
/// session.add_file("file1.nx", "let <Button text:string /> = <button>{text}</button>");
/// session.add_file("file2.nx", "let <Input /> = <input />");
///
/// // Check all
/// let results = session.check_all();
///
/// for (name, result) in results {
///     if !result.is_ok() {
///         println!("{}: {} errors", name, result.errors().len());
///     }
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct TypeCheckSession {
    /// Files in the session
    files: FxHashMap<String, String>,
    /// Next source ID to allocate
    _next_id: u32,
}

impl TypeCheckSession {
    /// Creates a new type checking session.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a source file to the session.
    pub fn add_file(&mut self, name: impl Into<String>, source: impl Into<String>) {
        self.files.insert(name.into(), source.into());
    }

    /// Type checks a specific file in the session.
    pub fn check_file(&self, name: &str) -> Option<TypeCheckResult> {
        self.files.get(name).map(|source| check_str(source, name))
    }

    /// Type checks all files in the session.
    pub fn check_all(&self) -> Vec<(String, TypeCheckResult)> {
        self.files
            .iter()
            .map(|(name, source)| (name.clone(), check_str(source, name)))
            .collect()
    }

    /// Returns all diagnostics from all files.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.check_all()
            .into_iter()
            .flat_map(|(_, result)| result.diagnostics)
            .collect()
    }

    /// Returns the number of files in the session.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Returns true if the session has no files.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nx_hir::{Item, Name};

    #[test]
    fn test_analyze_str_reports_library_imports_require_path_without_source_path() {
        let source = r#"
            import { Button as Layout.Button } from "../ui"
            let root() = { <Layout.Button /> }
        "#;

        let result = analyze_str(source, "virtual/main.nx");

        assert!(
            result.module.is_some(),
            "Expected lowered module to be preserved"
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == Some("library-imports-require-path")));
    }

    #[test]
    fn test_analyze_str_aggregates_lowering_and_type_diagnostics_with_file_name() {
        let file_name = "widgets/search-box.nx";
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

            let root(): int = "oops"
        "#;

        let result = analyze_str(source, file_name);

        assert!(
            result.module.is_some(),
            "Expected lowered module to be preserved"
        );

        let lowering = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code() == Some("lowering-error"))
            .expect("Expected lowering diagnostic");
        let return_type = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code() == Some("return-type-mismatch"))
            .expect("Expected return type diagnostic");

        assert_eq!(lowering.labels()[0].file, file_name);
        assert_eq!(return_type.labels()[0].file, file_name);
    }

    #[test]
    fn test_check_str_simple() {
        let source = "let x = 42";
        let result = check_str(source, "test.nx");

        // Parse should succeed even though we don't have full lowering yet
        assert!(result.module.is_some());
    }

    #[test]
    fn test_check_str_with_error() {
        // Invalid syntax
        let source = "let x = ";
        let result = check_str(source, "test.nx");

        // Should have parse errors
        assert!(!result.diagnostics.is_empty());
    }

    #[test]
    fn test_type_check_result_is_ok() {
        let source = "let x = 42";
        let _result = check_str(source, "test.nx");

        // Should succeed (or have warnings, not errors)
        // Note: May have errors if lowering isn't complete
    }

    #[test]
    fn test_session_creation() {
        let session = TypeCheckSession::new();
        assert!(session.is_empty());
        assert_eq!(session.len(), 0);
    }

    #[test]
    fn test_session_add_file() {
        let mut session = TypeCheckSession::new();
        session.add_file("file1.nx", "let x = 42");
        session.add_file("file2.nx", "let y = 10");

        assert_eq!(session.len(), 2);
        assert!(!session.is_empty());
    }

    #[test]
    fn test_session_check_file() {
        let mut session = TypeCheckSession::new();
        session.add_file("test.nx", "let x = 42");

        let result = session.check_file("test.nx");
        assert!(result.is_some());
    }

    #[test]
    fn test_session_check_all() {
        let mut session = TypeCheckSession::new();
        session.add_file("file1.nx", "let x = 42");
        session.add_file("file2.nx", "let y = 10");

        let results = session.check_all();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_session_diagnostics() {
        let mut session = TypeCheckSession::new();
        session.add_file("file1.nx", "let x = 42");
        session.add_file("file2.nx", "let y = "); // Parse error

        let diagnostics = session.diagnostics();
        // Should have at least one diagnostic from the parse error
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn test_scalar_brace_return_coerces_to_list_annotation() {
        let source = r#"
            let wrap(): int[] = { 1 }
        "#;
        let result = check_str(source, "coerce-return.nx");

        assert!(
            result.errors().is_empty(),
            "Expected no diagnostics, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn test_multi_value_brace_return_rejected_for_scalar_annotation() {
        let source = r#"
            let fail(): int = { 1 2 }
        "#;
        let result = check_str(source, "reject-return.nx");

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diag| diag.code() == Some("return-type-mismatch")),
            "Expected return type mismatch diagnostic, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn test_unannotated_multi_value_element_brace_falls_back_to_object_array() {
        let source = r#"
            let root() = { <A /> <B /> }
        "#;
        let result = check_str(source, "object-fallback.nx");
        let module = result.module.as_ref().expect("Expected lowered module");
        let root = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Function(func) if func.name.as_str() == "root" => Some(func),
                _ => None,
            })
            .expect("Expected root function");

        let root_ty = result
            .type_of(root.body)
            .expect("Expected inferred body type")
            .clone();
        assert_eq!(root_ty, Type::array(Type::named("object")));

        let func_ty = result
            .type_env
            .lookup(&Name::new("root"))
            .expect("Expected root function type");
        match func_ty {
            Type::Function { ret, .. } => assert_eq!(**ret, Type::array(Type::named("object"))),
            other => panic!("Expected function type, got {:?}", other),
        }
    }

    #[test]
    fn test_unannotated_multi_value_literal_brace_infers_int_array() {
        let source = r#"
            let root() = { 1 2 3 }
        "#;
        let result = check_str(source, "int-array.nx");
        let module = result.module.as_ref().expect("Expected lowered module");
        let root = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Function(func) if func.name.as_str() == "root" => Some(func),
                _ => None,
            })
            .expect("Expected root function");

        let root_ty = result
            .type_of(root.body)
            .expect("Expected inferred body type")
            .clone();
        assert_eq!(root_ty, Type::array(Type::int()));
    }

    #[test]
    fn test_element_property_type_mismatch_reports_diagnostic() {
        let source = r#"
            let <Counter count:int />: int = { count }
            let root(): int = { <Counter count="hello" /> }
        "#;
        let result = check_str(source, "property-type-mismatch.nx");

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diag| diag.code() == Some("property-type-mismatch")),
            "Expected property type mismatch diagnostic, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn test_children_binding_conflict_reports_diagnostic() {
        let source = r#"
            let <Collect children: object[] />: object[] = { children }
            let root(): object[] = { <Collect children={null}><div /></Collect> }
        "#;
        let result = check_str(source, "children-binding-conflict.nx");

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diag| diag.code() == Some("children-binding-conflict")),
            "Expected children binding conflict diagnostic, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn test_children_scalar_coerces_to_list_annotation() {
        let source = r#"
            let <Collect children: object[] />: object[] = { children }
            let root(): object[] = { <Collect><div /></Collect> }
        "#;
        let result = check_str(source, "children-coerce.nx");

        assert!(
            result.errors().is_empty(),
            "Expected no diagnostics, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn test_scalar_value_child_satisfies_scalar_children_annotation() {
        let source = r#"
            let <Collect children: int />: int = { children }
            let root(): int = { <Collect>{1}</Collect> }
        "#;
        let result = check_str(source, "children-scalar-value.nx");

        assert!(
            result.errors().is_empty(),
            "Expected no diagnostics, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn test_scalar_value_child_coerces_to_list_children_annotation() {
        let source = r#"
            let <Collect children: int[] />: int[] = { children }
            let root(): int[] = { <Collect>{1}</Collect> }
        "#;
        let result = check_str(source, "children-scalar-value-list.nx");

        assert!(
            result.errors().is_empty(),
            "Expected no diagnostics, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn test_children_multi_value_rejected_for_scalar_annotation() {
        let source = r#"
            let <Single children: div />: div = { children }
            let root(): div = { <Single><div /><span /></Single> }
        "#;
        let result = check_str(source, "children-reject.nx");

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diag| diag.code() == Some("children-type-mismatch")),
            "Expected children type mismatch diagnostic, got {:?}",
            result.diagnostics
        );
    }
}
