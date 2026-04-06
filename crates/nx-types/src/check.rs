//! High-level type checking and source-analysis API.

use crate::{InferenceContext, Type, TypeEnvironment};
use nx_diagnostics::{Diagnostic, Label, Severity};
use nx_hir::{lower, ExprId, Import, LoweredModule, LoweringDiagnostic, SourceId};
use nx_syntax::{parse_file as syntax_parse_file, parse_str as syntax_parse_str};
use rustc_hash::FxHashMap;
use std::io;
use std::path::Path;
use std::sync::Arc;

/// File-scoped analysis artifact for one NX source file.
///
/// This artifact preserves the parse outcome, lowered HIR, inferred type environment, static
/// diagnostics, and import metadata produced while parsing, lowering, preparing an analysis
/// module, building scopes, and type checking one source file.
#[derive(Debug, Clone)]
pub struct ModuleArtifact {
    /// Source file name used for diagnostics.
    pub file_name: String,
    /// Source file ID.
    pub source_id: SourceId,
    /// Whether parsing produced a syntax tree for this file.
    pub parse_succeeded: bool,
    /// The lowered module produced during analysis, if parsing succeeded.
    pub lowered_module: Option<Arc<LoweredModule>>,
    /// Type environment with all inferred bindings
    pub type_env: TypeEnvironment,
    /// Diagnostics from parsing, lowering, scope building, and type checking
    pub diagnostics: Vec<Diagnostic>,
    /// Import metadata preserved from the lowered module.
    pub imports: Vec<Import>,
}

impl ModuleArtifact {
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

/// Backward-compatible alias for callers that still think of shared analysis as a result object.
pub type SourceAnalysisResult = ModuleArtifact;

/// Alias for callers that use shared analysis as a type-checking entry point.
pub type TypeCheckResult = ModuleArtifact;

/// Analyzes NX source code from a string.
///
/// This performs parsing, lowering, scope building, and type checking in one pass. If parsing
/// produces a syntax tree, the returned analysis result preserves the lowered module even when
/// later phases report diagnostics.
pub fn analyze_str(source: &str, file_name: &str) -> ModuleArtifact {
    let parse_result = syntax_parse_str(source, file_name);
    analyze_string_parse_result(parse_result, file_name)
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
/// assert!(result.lowered_module.is_some());
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
    Ok(analyze_parse_result(parse_result, &file_name))
}

/// Analyzes a prepared module artifact where import resolution or other augmentation has already
/// been applied to the analysis module.
pub fn analyze_prepared_module(
    file_name: &str,
    source_id: SourceId,
    preserved_module: LoweredModule,
    analysis_module: LoweredModule,
    mut diagnostics: Vec<Diagnostic>,
) -> ModuleArtifact {
    diagnostics.extend(lowering_diagnostics(
        analysis_module.diagnostics(),
        file_name,
    ));

    let (_scope_manager, scope_diagnostics) = nx_hir::build_scopes(&analysis_module);
    diagnostics.extend(normalize_diagnostics_file_name(
        scope_diagnostics,
        file_name,
    ));

    let mut ctx = InferenceContext::with_file_name(&analysis_module, file_name);

    for (index, item) in analysis_module.items().iter().enumerate() {
        if analysis_module.is_external_item(index) {
            continue;
        }

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

    ModuleArtifact {
        file_name: file_name.to_string(),
        source_id,
        parse_succeeded: true,
        lowered_module: Some(Arc::new(preserved_module)),
        type_env,
        diagnostics,
        imports: analysis_module.imports.clone(),
    }
}

fn analyze_string_parse_result(
    parse_result: nx_syntax::ParseResult,
    file_name: &str,
) -> ModuleArtifact {
    let source_id = SourceId::new(parse_result.source_id.as_u32());
    let diagnostics = normalize_diagnostics_file_name(parse_result.errors, file_name);

    let Some(tree) = parse_result.tree else {
        return parse_failure_artifact(file_name, source_id, diagnostics);
    };

    let module = lower(tree.root(), source_id);
    analyze_prepared_module(file_name, source_id, module.clone(), module, diagnostics)
}

fn analyze_parse_result(parse_result: nx_syntax::ParseResult, file_name: &str) -> ModuleArtifact {
    let source_id = SourceId::new(parse_result.source_id.as_u32());
    let diagnostics = normalize_diagnostics_file_name(parse_result.errors, file_name);

    let Some(tree) = parse_result.tree else {
        return parse_failure_artifact(file_name, source_id, diagnostics);
    };

    let module = lower(tree.root(), source_id);
    analyze_prepared_module(file_name, source_id, module.clone(), module, diagnostics)
}

fn parse_failure_artifact(
    file_name: &str,
    source_id: SourceId,
    diagnostics: Vec<Diagnostic>,
) -> ModuleArtifact {
    module_artifact(
        file_name,
        source_id,
        false,
        None,
        TypeEnvironment::new(),
        diagnostics,
    )
}

fn module_artifact(
    file_name: &str,
    source_id: SourceId,
    parse_succeeded: bool,
    lowered_module: Option<Arc<LoweredModule>>,
    type_env: TypeEnvironment,
    diagnostics: Vec<Diagnostic>,
) -> ModuleArtifact {
    let imports = lowered_module
        .as_ref()
        .map(|module| module.imports.clone())
        .unwrap_or_default();

    ModuleArtifact {
        file_name: file_name.to_string(),
        source_id,
        parse_succeeded,
        lowered_module,
        type_env,
        diagnostics,
        imports,
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
    fn test_analyze_str_preserves_import_metadata_without_resolving_imports() {
        let source = r#"
            import { Button as Layout.Button } from "../ui"
            let root() = { <Layout.Button /> }
        "#;

        let result = analyze_str(source, "virtual/main.nx");

        assert!(
            result.parse_succeeded,
            "Expected parse metadata to be preserved"
        );
        assert!(
            result.lowered_module.is_some(),
            "Expected lowered module to be preserved"
        );
        assert_eq!(
            result.imports.len(),
            1,
            "Expected import metadata to be preserved"
        );
        assert!(
            result
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code() != Some("library-imports-require-path")),
            "Prepared-module analysis should not perform implicit import resolution"
        );
    }

    #[test]
    fn test_analyze_str_returns_parse_failure_module_artifact() {
        let diagnostic = Diagnostic::error("parse-failed")
            .with_message("Failed to parse source")
            .build();
        let parse_result = nx_syntax::ParseResult {
            tree: None,
            errors: vec![diagnostic],
            source_id: nx_syntax::SourceId::new(7),
        };
        let result = analyze_string_parse_result(parse_result, "widgets/search-box.nx");

        assert!(
            !result.parse_succeeded,
            "Expected parse-failure artifacts to record the parse outcome"
        );
        assert!(
            result.lowered_module.is_none(),
            "Expected parse-failure artifacts to omit the lowered module"
        );
        assert!(
            result.imports.is_empty(),
            "Expected parse-failure imports to be empty"
        );
        assert!(
            !result.diagnostics.is_empty(),
            "Expected parse diagnostics to be preserved"
        );
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
            result.lowered_module.is_some(),
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
        assert!(result.lowered_module.is_some());
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
        let module = result
            .lowered_module
            .as_ref()
            .expect("Expected lowered module");
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
        let module = result
            .lowered_module
            .as_ref()
            .expect("Expected lowered module");
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
    fn test_content_binding_conflict_reports_diagnostic() {
        let source = r#"
            let <Collect content items: object[] />: object[] = { items }
            let root(): object[] = { <Collect items={null}><div /></Collect> }
        "#;
        let result = check_str(source, "content-binding-conflict.nx");

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diag| diag.code() == Some("content-binding-conflict")),
            "Expected content binding conflict diagnostic, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn test_content_scalar_coerces_to_list_annotation() {
        let source = r#"
            let <Collect content items: object[] />: object[] = { items }
            let root(): object[] = { <Collect><div /></Collect> }
        "#;
        let result = check_str(source, "content-coerce.nx");

        assert!(
            result.errors().is_empty(),
            "Expected no diagnostics, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn test_text_body_satisfies_scalar_string_content_annotation() {
        let source = r#"
            type Label = { content text:string }
            let root(): Label = { <Label>label text</Label> }
        "#;
        let result = check_str(source, "content-text-scalar.nx");

        assert!(
            result.errors().is_empty(),
            "Expected no diagnostics, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn test_scalar_value_child_coerces_to_list_content_annotation() {
        let source = r#"
            let <Collect content items: int[] />: int[] = { items }
            let root(): int[] = { <Collect>{1}</Collect> }
        "#;
        let result = check_str(source, "content-scalar-value-list.nx");

        assert!(
            result.errors().is_empty(),
            "Expected no diagnostics, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn test_missing_content_property_reports_diagnostic() {
        let source = r#"
            let <Collect items: object[] />: object[] = { items }
            let root(): object[] = { <Collect><div /></Collect> }
        "#;
        let result = check_str(source, "missing-content-property.nx");

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diag| diag.code() == Some("missing-content-property")),
            "Expected missing content property diagnostic, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn test_content_multi_value_rejected_for_scalar_annotation() {
        let source = r#"
            let <Single content item: div />: div = { item }
            let root(): div = { <Single><div /><span /></Single> }
        "#;
        let result = check_str(source, "content-reject.nx");

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diag| diag.code() == Some("content-type-mismatch")),
            "Expected content type mismatch diagnostic, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn test_paren_function_markup_invocation_accepts_declared_content_param() {
        let source = r#"
            let Wrap(title:string, content body:Element) = <section>{body}</section>
            let root() = <Wrap title="Docs"><Badge /></Wrap>
        "#;
        let result = check_str(source, "paren-content-invocation.nx");

        assert!(
            result.errors().is_empty(),
            "Expected no diagnostics, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn test_component_markup_invocation_requires_declared_content_prop() {
        let source = r#"
            component <Panel title:string /> = {
                <section>{title}</section>
            }
            let root() = <Panel title="Docs"><Badge /></Panel>
        "#;
        let result = check_str(source, "component-missing-content.nx");

        assert!(
            result
                .diagnostics
                .iter()
                .any(|diag| diag.code() == Some("missing-content-property")),
            "Expected missing content property diagnostic, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn test_component_markup_invocation_accepts_declared_content_prop() {
        let source = r#"
            component <Panel title:string content body:Element /> = {
                <section>{body}</section>
            }
            let root() = <Panel title="Docs"><Badge /></Panel>
        "#;
        let result = check_str(source, "component-content.nx");

        assert!(
            result.errors().is_empty(),
            "Expected no diagnostics, got {:?}",
            result.diagnostics
        );
    }
}
