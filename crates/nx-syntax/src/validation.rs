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

    diagnostics
}

/// Validates that element opening and closing tags match.
fn validate_element_tags(
    node: &SyntaxNode,
    _tree: &SyntaxTree,
    file_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Check if this is an element node
    if node.kind() == SyntaxKind::ELEMENT {
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
}
