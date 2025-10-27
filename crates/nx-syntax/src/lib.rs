//! NX syntax parsing and CST (Concrete Syntax Tree) representation.
//!
//! This crate provides tree-sitter-based parsing for the NX language,
//! with typed wrappers and a high-level API for parsing files.

mod ast;
mod syntax_kind;
mod syntax_node;
mod validation;

pub use ast::{AstNode, Element, FunctionDef, SyntaxNodeExt, TypeDef};
pub use syntax_kind::{syntax_kind_from_str, SyntaxKind};
pub use syntax_node::SyntaxNode;
pub use validation::validate;

use nx_diagnostics::{Diagnostic, Severity};
use std::fs;
use std::io;
use std::path::Path;
use std::sync::Arc;
use text_size::TextRange;
use tree_sitter::{Language, Parser, Tree};

extern "C" {
    fn tree_sitter_nx() -> Language;
}

/// Returns the tree-sitter Language for NX.
pub fn language() -> Language {
    unsafe { tree_sitter_nx() }
}

/// Creates a new parser configured for NX.
pub fn parser() -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(language())
        .expect("Failed to set NX language");
    parser
}

/// Unique identifier for a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceId(u32);

impl SourceId {
    /// Creates a new source ID.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Returns the inner ID value.
    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

/// An immutable syntax tree from tree-sitter.
pub struct SyntaxTree {
    tree: Tree,
    source: Arc<String>,
    source_id: SourceId,
}

impl SyntaxTree {
    /// Creates a new syntax tree.
    fn new(tree: Tree, source: String, source_id: SourceId) -> Self {
        Self {
            tree,
            source: Arc::new(source),
            source_id,
        }
    }

    /// Returns the root syntax node.
    pub fn root(&self) -> SyntaxNode {
        SyntaxNode::new(self.tree.root_node(), &self.source)
    }

    /// Returns the text for a given span.
    pub fn text(&self, range: TextRange) -> &str {
        let start = range.start().into();
        let end = range.end().into();
        &self.source[start..end]
    }

    /// Returns the full source code.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Finds the node at the given byte offset.
    pub fn node_at(&self, offset: usize) -> Option<SyntaxNode> {
        let node = self
            .tree
            .root_node()
            .descendant_for_byte_range(offset, offset)?;
        Some(SyntaxNode::new(node, &self.source))
    }

    /// Returns the source ID.
    pub fn source_id(&self) -> SourceId {
        self.source_id
    }
}

/// Error types for parsing operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Invalid UTF-8 in source file")]
    InvalidUtf8,

    #[error("File not found: {0}")]
    FileNotFound(std::path::PathBuf),

    #[error("Parse error: {0}")]
    Parse(String),
}

/// Result of parsing NX source code.
pub struct ParseResult {
    /// The parsed syntax tree (None if fatal parse error)
    pub tree: Option<SyntaxTree>,

    /// Parse errors and warnings
    pub errors: Vec<Diagnostic>,

    /// Source file identifier
    pub source_id: SourceId,
}

impl ParseResult {
    /// Returns true if parsing succeeded (no errors).
    pub fn is_ok(&self) -> bool {
        self.errors
            .iter()
            .all(|d| d.severity() != Severity::Error)
    }

    /// Returns the root syntax node if available.
    pub fn root(&self) -> Option<SyntaxNode> {
        self.tree.as_ref().map(|t| t.root())
    }

    /// Returns true if there are any errors.
    pub fn has_errors(&self) -> bool {
        !self.is_ok()
    }
}

/// Parses NX source code into a syntax tree.
///
/// # Examples
///
/// ```
/// use nx_syntax::parse_str;
///
/// let source = "let x = 42";
/// let result = parse_str(source, "example.nx");
///
/// assert!(result.tree.is_some());
/// ```
pub fn parse_str(source: &str, file_name: &str) -> ParseResult {
    // Validate UTF-8
    if !source.is_utf8() {
        return ParseResult {
            tree: None,
            errors: vec![Diagnostic::error("invalid-utf8")
                .with_message("Source file contains invalid UTF-8")
                .build()],
            source_id: SourceId::new(0),
        };
    }

    let mut parser = parser();
    let tree = parser.parse(source, None);

    let source_id = SourceId::new(file_name.as_bytes().iter().fold(0u32, |acc, &b| {
        acc.wrapping_mul(31).wrapping_add(b as u32)
    }));

    match tree {
        Some(tree) => {
            // Collect parse errors from the tree with enhanced messages
            let mut errors = validation::collect_enhanced_errors(&tree, source, file_name);

            // Create the syntax tree
            let syntax_tree = SyntaxTree::new(tree, source.to_string(), source_id);

            // Run post-parse validation (e.g., tag matching)
            let validation_errors = validation::validate(&syntax_tree, file_name);
            errors.extend(validation_errors);

            ParseResult {
                tree: Some(syntax_tree),
                errors,
                source_id,
            }
        }
        None => ParseResult {
            tree: None,
            errors: vec![Diagnostic::error("parse-failed")
                .with_message("Failed to parse source")
                .build()],
            source_id,
        },
    }
}

/// Parses NX source from a file.
///
/// # Errors
///
/// Returns `Err` if the file cannot be read or is not valid UTF-8.
///
/// # Examples
///
/// ```no_run
/// use nx_syntax::parse_file;
///
/// let result = parse_file("example.nx").unwrap();
/// assert!(result.is_ok());
/// ```
pub fn parse_file(path: impl AsRef<Path>) -> io::Result<ParseResult> {
    let path = path.as_ref();

    // Check if file exists
    if !path.exists() {
        return Ok(ParseResult {
            tree: None,
            errors: vec![Diagnostic::error("file-not-found")
                .with_message(format!("File not found: {}", path.display()))
                .build()],
            source_id: SourceId::new(0),
        });
    }

    // Read file contents
    let source = fs::read_to_string(path)?;

    // Validate UTF-8 (already validated by read_to_string, but explicit check)
    if !source.is_utf8() {
        return Ok(ParseResult {
            tree: None,
            errors: vec![Diagnostic::error("invalid-utf8")
                .with_message(format!(
                    "File contains invalid UTF-8: {}",
                    path.display()
                ))
                .build()],
            source_id: SourceId::new(0),
        });
    }

    let file_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    Ok(parse_str(&source, file_name))
}


/// Extension trait for `&str` to check UTF-8 validity.
trait Utf8Ext {
    fn is_utf8(&self) -> bool;
}

impl Utf8Ext for str {
    fn is_utf8(&self) -> bool {
        // str is always valid UTF-8 in Rust
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_can_be_loaded() {
        let lang = language();
        assert!(lang.node_kind_count() > 0);
    }

    #[test]
    fn test_parser_creation() {
        let parser = parser();
        assert!(parser.language().is_some());
    }

    #[test]
    fn test_parse_str_simple() {
        let source = "let <Button text: string /> = <button>{text}</button>";
        let result = parse_str(source, "test.nx");

        assert!(result.tree.is_some());
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_str_with_error() {
        let source = "let x = ";  // Incomplete
        let result = parse_str(source, "test.nx");

        // Should still have a tree, but with errors
        assert!(result.tree.is_some());
        assert!(!result.is_ok());
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_syntax_tree_root() {
        let source = "import foo";
        let result = parse_str(source, "test.nx");

        let tree = result.tree.unwrap();
        let root = tree.root();

        assert_eq!(root.kind(), SyntaxKind::MODULE_DEFINITION);
        assert_eq!(root.text(), source);
    }

    #[test]
    fn test_syntax_tree_source() {
        let source = "import foo";
        let result = parse_str(source, "test.nx");

        let tree = result.tree.unwrap();
        assert_eq!(tree.source(), source);
    }

    #[test]
    fn test_syntax_tree_node_at() {
        let source = "import foo";
        let result = parse_str(source, "test.nx");

        let tree = result.tree.unwrap();
        let node = tree.node_at(7);  // Middle of "foo"

        assert!(node.is_some());
    }

    #[test]
    fn test_parse_result_is_ok() {
        let source = "import foo";
        let result = parse_str(source, "test.nx");

        assert!(result.is_ok());
        assert!(!result.has_errors());
    }

    #[test]
    fn test_parse_result_with_errors() {
        let source = "let x = ";  // Incomplete
        let result = parse_str(source, "test.nx");

        assert!(!result.is_ok());
        assert!(result.has_errors());
    }

    #[test]
    fn test_tree_sitter_error_nodes() {
        // Test if tree-sitter is really producing ERROR nodes ANYWHERE
        let source = "let <Button text: string /> = <button>{text}</button>";
        let mut p = parser();
        let tree = p.parse(source, None).unwrap();
        let root = tree.root_node();

        // Recursively check for ANY error nodes in the entire tree
        fn find_errors(node: tree_sitter::Node, path: &str, errors: &mut Vec<String>) {
            if node.is_error() {
                errors.push(format!("{} -> {} (error)", path, node.kind()));
            }

            let mut cursor = node.walk();
            for (i, child) in node.children(&mut cursor).enumerate() {
                let child_path = format!("{}/{}", path, child.kind());
                find_errors(child, &child_path, errors);
            }
        }

        let mut errors = Vec::new();
        find_errors(root, "root", &mut errors);

        if !errors.is_empty() {
            println!("Found {} ERROR nodes:", errors.len());
            for err in &errors {
                println!("  {}", err);
            }
        }

        // This will fail if there are ANY error nodes anywhere
        assert!(errors.is_empty(), "Tree should not contain any ERROR nodes! Found: {:?}", errors);
    }
}
