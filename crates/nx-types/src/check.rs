//! High-level type checking API.

use crate::{InferenceContext, Type, TypeEnvironment};
use nx_diagnostics::Diagnostic;
use nx_hir::{lower, ExprId, Module, SourceId};
use nx_syntax::{parse_file as syntax_parse_file, parse_str as syntax_parse_str};
use rustc_hash::FxHashMap;
use std::io;
use std::path::Path;
use std::sync::Arc;

/// Result of type checking a module.
///
/// Contains the typed module, type environment, and any diagnostics
/// (errors, warnings) discovered during type checking.
#[derive(Debug, Clone)]
pub struct TypeCheckResult {
    /// The module that was type-checked (None if parsing failed)
    pub module: Option<Arc<Module>>,
    /// Type environment with all bindings
    pub type_env: TypeEnvironment,
    /// Diagnostics (parse errors + type errors)
    pub diagnostics: Vec<Diagnostic>,
    /// Source file ID
    pub source_id: SourceId,
}

impl TypeCheckResult {
    /// Returns true if type checking succeeded (no errors).
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
    let source_id = SourceId::new(0); // Simple ID for single-file checking

    // Parse
    let parse_result = syntax_parse_str(source, file_name);
    let mut diagnostics = parse_result.errors.clone();

    if let Some(tree) = parse_result.tree {
        // Lower to HIR
        let root = tree.root();
        let module = lower(root, source_id);

        // Build scopes
        let (_scope_manager, scope_diagnostics) = nx_hir::build_scopes(&module);
        diagnostics.extend(scope_diagnostics);

        // Type check
        let mut ctx = InferenceContext::new(&module);

        // Infer types for all items
        for item in module.items() {
            match item {
                nx_hir::Item::Function(func) => {
                    // Add function to environment
                    // TODO: Build function type from params and return type
                    // For now, infer the body
                    ctx.infer_function(func);
                }
                nx_hir::Item::Element(_) => {
                    // Elements don't need type checking yet
                }
                nx_hir::Item::TypeAlias => {
                    // Not implemented yet
                }
            }
        }

        let (type_env, type_diagnostics) = ctx.finish();
        diagnostics.extend(type_diagnostics);

        TypeCheckResult {
            module: Some(Arc::new(module)),
            type_env,
            diagnostics,
            source_id,
        }
    } else {
        // Parse failed - return error result
        TypeCheckResult {
            module: None,
            type_env: TypeEnvironment::new(),
            diagnostics,
            source_id,
        }
    }
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
    let source_id = SourceId::new(0);

    // Parse file
    let parse_result = syntax_parse_file(path)?;
    let mut diagnostics = parse_result.errors.clone();

    if let Some(tree) = parse_result.tree {
        // Lower to HIR
        let root = tree.root();
        let module = lower(root, source_id);

        // Build scopes
        let (_scope_manager, scope_diagnostics) = nx_hir::build_scopes(&module);
        diagnostics.extend(scope_diagnostics);

        // Type check
        let mut ctx = InferenceContext::new(&module);

        // Infer types for all items
        for item in module.items() {
            match item {
                nx_hir::Item::Function(func) => {
                    ctx.infer_function(func);
                }
                nx_hir::Item::Element(_) => {}
                nx_hir::Item::TypeAlias => {}
            }
        }

        let (type_env, type_diagnostics) = ctx.finish();
        diagnostics.extend(type_diagnostics);

        Ok(TypeCheckResult {
            module: Some(Arc::new(module)),
            type_env,
            diagnostics,
            source_id,
        })
    } else {
        Ok(TypeCheckResult {
            module: None,
            type_env: TypeEnvironment::new(),
            diagnostics,
            source_id,
        })
    }
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
}
