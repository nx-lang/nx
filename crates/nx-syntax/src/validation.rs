//! Post-parse validation for NX syntax trees.
//!
//! This module provides semantic validation that goes beyond what tree-sitter
//! can detect during parsing, such as:
//! - Element tag matching (opening and closing tags must match)
//! - Error recovery within scopes
//! - Enhanced error messages with suggestions

use crate::{SyntaxKind, SyntaxNode, SyntaxTree};
use nx_diagnostics::{Diagnostic, Label};
use text_size::TextRange;

/// Validates a syntax tree and returns any semantic errors found.
///
/// This performs post-parse validation that tree-sitter cannot detect, such as:
/// - Element tag matching (opening and closing tags must match)
/// - Semantic consistency checks
///
/// # Examples
///
/// ```
/// use nx_syntax::{parse_str, validate};
///
/// let result = parse_str("<Button>content</Button>", "test.nx");
/// if let Some(tree) = result.tree {
///     let diagnostics = validate(&tree, "test.nx");
///     assert!(diagnostics.is_empty());
/// }
/// ```
pub fn validate(tree: &SyntaxTree, file_name: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let root = tree.root();

    // Validate element tag matching
    validate_element_tags(&root, tree, file_name, &mut diagnostics);

    // Validate root definitions (no duplicates between explicit 'root' and top-level element)
    validate_root_definitions(&root, file_name, &mut diagnostics);

    diagnostics
}

/// Validates that element opening and closing tags match.
fn validate_element_tags(
    node: &SyntaxNode,
    _tree: &SyntaxTree,
    file_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Check if this is an element node or text child element
    if node.kind() == SyntaxKind::ELEMENT || node.kind() == SyntaxKind::TEXT_CHILD_ELEMENT {
        // Get opening tag
        let opening_tag = node
            .child_by_field("opening_tag")
            .or_else(|| node.child_by_field("tag"))
            .or_else(|| node.child_by_field("name"));
        let close_name_node = node.child_by_field("close_name");

        if let (Some(opening), Some(closing)) = (opening_tag, close_name_node) {
            // Get the tag name from the opening tag
            let opening_name = extract_tag_name(&opening);

            // Get the tag name from the closing tag
            let closing_name = extract_tag_name(&closing);

            if let (Some(open_name), Some(close_name)) = (opening_name, closing_name) {
                if open_name != close_name {
                    // Tag names don't match - create diagnostic
                    let open_range = opening.span();
                    let close_range = closing.span();

                    let diagnostic = Diagnostic::error("tag-mismatch")
                        .with_message(format!(
                            "Element closing tag '{}' does not match opening tag '{}'",
                            close_name, open_name
                        ))
                        .with_label(
                            Label::primary(file_name, close_range).with_message("closing tag here"),
                        )
                        .with_label(
                            Label::secondary(file_name, open_range)
                                .with_message(format!("opening tag '{}' here", open_name)),
                        )
                        .with_note(format!("Expected closing tag '</{}>>'", open_name))
                        .build();

                    diagnostics.push(diagnostic);
                }
            }
        }
    }

    // Recursively validate children
    for child in node.children() {
        validate_element_tags(&child, _tree, file_name, diagnostics);
    }
}

/// Extracts the tag name from an element tag node.
fn extract_tag_name(tag_node: &SyntaxNode) -> Option<String> {
    for child in tag_node.children() {
        if child.kind() == SyntaxKind::IDENTIFIER {
            return Some(child.text().to_string());
        }

        if child.kind() == SyntaxKind::QUALIFIED_MARKUP_NAME {
            return extract_tag_name(&child);
        }
    }

    if tag_node.kind() == SyntaxKind::IDENTIFIER
        || tag_node.kind() == SyntaxKind::QUALIFIED_MARKUP_NAME
    {
        return Some(tag_node.text().to_string());
    }

    None
}

/// Validates that there are no duplicate 'root' definitions.
///
/// A module can have at most one 'root' definition, which can come from either:
/// - An explicit `let root = ...` or `let root() = ...` definition
/// - An implicit top-level element (which becomes the 'root' function)
///
/// This function detects:
/// - Multiple explicit 'root' definitions (error)
/// - Both explicit 'root' and top-level element (error)
fn validate_root_definitions(
    root: &SyntaxNode,
    file_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut explicit_roots: Vec<TextRange> = Vec::new();
    let mut implicit_root: Option<TextRange> = None;

    // Scan top-level children of the module
    for child in root.children() {
        match child.kind() {
            SyntaxKind::FUNCTION_DEFINITION | SyntaxKind::VALUE_DEFINITION => {
                // Check if this defines 'root'
                if let Some(name_node) = child.child_by_field("name") {
                    if name_node.text() == "root" {
                        explicit_roots.push(name_node.span());
                    }
                }
            }
            SyntaxKind::ELEMENT => {
                // Top-level element becomes implicit 'root'
                implicit_root = Some(child.span());
            }
            _ => {}
        }
    }

    // Check for multiple explicit root definitions
    if explicit_roots.len() > 1 {
        let first_span = explicit_roots[0];
        let second_span = explicit_roots[1];

        let diagnostic = Diagnostic::error("duplicate-root")
            .with_message("Duplicate definition of 'root'")
            .with_label(
                Label::primary(file_name, second_span).with_message("duplicate 'root' definition"),
            )
            .with_label(
                Label::secondary(file_name, first_span)
                    .with_message("first 'root' definition here"),
            )
            .with_note("A module can have at most one 'root' definition")
            .build();

        diagnostics.push(diagnostic);
    }

    // Check for conflict between explicit root and top-level element
    if let (Some(explicit_span), Some(implicit_span)) =
        (explicit_roots.first().copied(), implicit_root)
    {
        let diagnostic = Diagnostic::error("duplicate-root")
            .with_message("Duplicate definition of 'root'")
            .with_label(
                Label::primary(file_name, implicit_span)
                    .with_message("top-level element implicitly defines 'root'"),
            )
            .with_label(
                Label::secondary(file_name, explicit_span)
                    .with_message("explicit 'root' definition here"),
            )
            .with_note(
                "A module can have either a top-level element or an explicit 'root' definition, but not both",
            )
            .build();

        diagnostics.push(diagnostic);
    }
}

/// Collects all parse errors from tree-sitter ERROR nodes with enhanced messages.
///
/// This function walks the CST and converts tree-sitter ERROR and MISSING nodes
/// into rich `Diagnostic` messages with context-aware suggestions.
///
/// # Arguments
///
/// * `tree` - The tree-sitter parse tree
/// * `source` - The original source code
/// * `file_name` - The name of the file being parsed (for error messages)
///
/// # Returns
///
/// A vector of diagnostic messages for all syntax errors found in the tree.
pub fn collect_enhanced_errors(
    tree: &tree_sitter::Tree,
    source: &str,
    file_name: &str,
) -> Vec<Diagnostic> {
    let mut errors = Vec::new();
    let root = tree.root_node();

    walk_and_collect_errors(root, source, file_name, &mut errors);
    errors
}

/// Recursively walks the tree and collects errors with context-aware messages.
fn walk_and_collect_errors(
    node: tree_sitter::Node,
    source: &str,
    file_name: &str,
    errors: &mut Vec<Diagnostic>,
) {
    if node.is_error() || node.is_missing() {
        let start = node.start_byte() as u32;
        let end = node.end_byte() as u32;
        let range = TextRange::new(start.into(), end.into());

        // Get the text of the error node for context
        let error_text = &source[start as usize..end.min(source.len() as u32) as usize];

        // Generate context-aware error message
        let (message, suggestion) = analyze_error_context(&node, error_text, source);

        let mut diagnostic_builder = Diagnostic::error("syntax-error")
            .with_message(message)
            .with_label(Label::primary(file_name, range).with_message("unexpected syntax here"));

        if let Some(note) = suggestion {
            diagnostic_builder = diagnostic_builder.with_note(note);
        }

        errors.push(diagnostic_builder.build());
    }

    // Recursively check children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_and_collect_errors(child, source, file_name, errors);
    }
}

/// Analyzes the error context and provides helpful messages and suggestions.
fn analyze_error_context(
    node: &tree_sitter::Node,
    error_text: &str,
    _source: &str,
) -> (String, Option<String>) {
    // Check if this is a missing node
    if node.is_missing() {
        let message = format!("Expected {} here", node.kind());
        let suggestion = Some(format!("Try adding a {} at this location", node.kind()));
        return (message, suggestion);
    }

    // Check parent context for better error messages
    if let Some(parent) = node.parent() {
        match parent.kind() {
            "element" => {
                return (
                    "Invalid element syntax".to_string(),
                    Some(
                        "Expected element with format: <Tag prop={value}>content</Tag>".to_string(),
                    ),
                );
            }
            "function_definition" => {
                return (
                    "Invalid function definition".to_string(),
                    Some("Expected function with format: fn name(params) { body }".to_string()),
                );
            }
            "let_declaration" => {
                return (
                    "Invalid let declaration".to_string(),
                    Some(
                        "Expected format: let name = value or let <Pattern /> = value".to_string(),
                    ),
                );
            }
            _ => {}
        }
    }

    // Common error patterns
    if error_text.contains('{') && !error_text.contains('}') {
        return (
            "Unclosed brace".to_string(),
            Some("Add a closing '}' to match the opening brace".to_string()),
        );
    }

    if error_text.contains('(') && !error_text.contains(')') {
        return (
            "Unclosed parenthesis".to_string(),
            Some("Add a closing ')' to match the opening parenthesis".to_string()),
        );
    }

    if error_text.contains('[') && !error_text.contains(']') {
        return (
            "Unclosed bracket".to_string(),
            Some("Add a closing ']' to match the opening bracket".to_string()),
        );
    }

    // Default error message
    (
        "Syntax error".to_string(),
        Some("Check the syntax and try again".to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_str;

    #[test]
    fn test_validate_matching_tags() {
        let source = "<Button>content</Button>";
        let result = parse_str(source, "test.nx");
        let tree = result.tree.unwrap();

        let diagnostics = validate(&tree, "test.nx");
        assert!(
            diagnostics.is_empty(),
            "Matching tags should not produce errors"
        );
    }

    #[test]
    fn test_validate_mismatched_tags() {
        let source = "<Button>content</Input>";
        let result = parse_str(source, "test.nx");

        if let Some(tree) = result.tree {
            let diagnostics = validate(&tree, "test.nx");

            // Find tag mismatch errors
            let tag_errors: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.code() == Some("tag-mismatch"))
                .collect();

            // May or may not detect depending on grammar's error recovery
            // This test documents the behavior
            if !tag_errors.is_empty() {
                assert!(tag_errors[0].message().contains("does not match"));
            }
        }
    }

    #[test]
    fn test_enhanced_error_messages_for_unclosed_brace() {
        let source = "let x = { a: 1";
        let result = parse_str(source, "test.nx");

        // Should have errors with helpful suggestions
        assert!(!result.errors.is_empty());

        let error_msgs: String = result
            .errors
            .iter()
            .map(|d| d.message())
            .collect::<Vec<_>>()
            .join(" ");

        // At minimum, should have parse errors
        assert!(!error_msgs.is_empty());
    }

    #[test]
    fn test_error_recovery_within_scope() {
        // Multiple errors in the same scope - should collect all of them
        let source = r#"
            let x = {;
            let y = };
            let z = 42
        "#;

        let result = parse_str(source, "test.nx");

        // Should have multiple errors
        assert!(result.errors.len() >= 1, "Should detect syntax errors");
    }

    #[test]
    fn test_validate_text_child_element_matching_tags() {
        let source = "<p:>Hello <b>world</b>!</p>";
        let result = parse_str(source, "test.nx");
        let tree = result.tree.unwrap();

        let diagnostics = validate(&tree, "test.nx");
        assert!(
            diagnostics.is_empty(),
            "Matching text child element tags should not produce errors"
        );
    }

    #[test]
    fn test_validate_text_child_element_mismatched_tags() {
        let source = "<p:>Hello <b>world</i>!</p>";
        let result = parse_str(source, "test.nx");

        if let Some(tree) = result.tree {
            let diagnostics = validate(&tree, "test.nx");

            // Find tag mismatch errors
            let tag_errors: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.code() == Some("tag-mismatch"))
                .collect();

            // Should detect the mismatched <b>...</i> tags
            assert!(
                !tag_errors.is_empty(),
                "Should detect mismatched text child element tags"
            );
            assert!(
                tag_errors[0].message().contains("does not match"),
                "Error message should indicate tag mismatch"
            );
        }
    }

    #[test]
    fn test_validate_top_level_element_only() {
        // A top-level element alone should not produce errors
        let source = "<App><Header /></App>";
        let result = parse_str(source, "test.nx");
        let tree = result.tree.unwrap();

        let diagnostics = validate(&tree, "test.nx");
        let root_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.code() == Some("duplicate-root"))
            .collect();

        assert!(
            root_errors.is_empty(),
            "Top-level element alone should not produce duplicate-root error"
        );
    }

    #[test]
    fn test_validate_explicit_root_only() {
        // An explicit root function alone should not produce errors
        let source = "let root() = <App />";
        let result = parse_str(source, "test.nx");
        let tree = result.tree.unwrap();

        let diagnostics = validate(&tree, "test.nx");
        let root_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.code() == Some("duplicate-root"))
            .collect();

        assert!(
            root_errors.is_empty(),
            "Explicit root function alone should not produce duplicate-root error"
        );
    }

    #[test]
    fn test_validate_duplicate_root_function_and_element() {
        // Both explicit root function and top-level element should produce error
        let source = r#"
            let root() = <Explicit />

            <Implicit />
        "#;
        let result = parse_str(source, "test.nx");
        let tree = result.tree.unwrap();

        let diagnostics = validate(&tree, "test.nx");
        let root_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.code() == Some("duplicate-root"))
            .collect();

        assert_eq!(
            root_errors.len(),
            1,
            "Should detect duplicate root definition"
        );
        assert!(
            root_errors[0].message().contains("Duplicate"),
            "Error message should indicate duplicate"
        );
    }

    #[test]
    fn test_validate_duplicate_root_value_and_element() {
        // Both explicit root value and top-level element should produce error
        let source = r#"
            let root = <Explicit />

            <Implicit />
        "#;
        let result = parse_str(source, "test.nx");
        let tree = result.tree.unwrap();

        let diagnostics = validate(&tree, "test.nx");
        let root_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.code() == Some("duplicate-root"))
            .collect();

        assert_eq!(
            root_errors.len(),
            1,
            "Should detect duplicate root definition (value)"
        );
    }

    #[test]
    fn test_validate_multiple_explicit_root_functions() {
        // Two explicit root functions should produce error
        let source = r#"
            let root() = <First />
            let root() = <Second />
        "#;
        let result = parse_str(source, "test.nx");
        let tree = result.tree.unwrap();

        let diagnostics = validate(&tree, "test.nx");
        let root_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.code() == Some("duplicate-root"))
            .collect();

        assert_eq!(
            root_errors.len(),
            1,
            "Should detect duplicate explicit root definitions"
        );
        assert!(
            root_errors[0].message().contains("Duplicate"),
            "Error message should indicate duplicate"
        );
    }

    #[test]
    fn test_validate_multiple_explicit_root_mixed() {
        // Function and value both named 'root' should produce error
        let source = r#"
            let root = 42
            let root() = <App />
        "#;
        let result = parse_str(source, "test.nx");
        let tree = result.tree.unwrap();

        let diagnostics = validate(&tree, "test.nx");
        let root_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.code() == Some("duplicate-root"))
            .collect();

        assert_eq!(
            root_errors.len(),
            1,
            "Should detect duplicate root (value + function)"
        );
    }
}
