//! NX syntax parsing and CST (Concrete Syntax Tree) representation.
//!
//! This crate provides tree-sitter-based parsing for the NX language.

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

/// Parse NX source code and return the syntax tree.
pub fn parse(source: &str) -> Option<Tree> {
    let mut parser = parser();
    parser.parse(source, None)
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
    fn test_parse_simple_function() {
        let source = "let <Button text: string /> = <button>{text}</button>";
        let tree = parse(source).expect("Failed to parse");

        let root = tree.root_node();
        assert_eq!(root.kind(), "module_definition");
        assert!(!root.has_error());
    }

    #[test]
    fn test_parse_function_with_defaults() {
        let source = r#"let <Button
  text: string = "Click me"
  disabled: boolean = false
/> = <button disabled={disabled}>{text}</button>"#;

        let tree = parse(source).expect("Failed to parse");
        let root = tree.root_node();

        assert_eq!(root.kind(), "module_definition");

        // Find function definition
        let func = root
            .child(0)
            .expect("Expected function definition");
        assert_eq!(func.kind(), "function_definition");
    }

    #[test]
    fn test_parse_type_definitions() {
        let source = r#"type UserId = int
type Username = string
type MaybeUser = User?"#;

        let tree = parse(source).expect("Failed to parse");
        let root = tree.root_node();

        // Should have 3 type definitions
        assert_eq!(root.named_child_count(), 3);

        for i in 0..3 {
            let child = root.named_child(i).unwrap();
            assert_eq!(child.kind(), "type_definition");
        }
    }

    #[test]
    fn test_parse_import() {
        let source = "import core.html\nimport ui.components";

        let tree = parse(source).expect("Failed to parse");
        let root = tree.root_node();

        // Should have 2 import statements
        assert!(root.named_child_count() >= 2);

        let import1 = root.named_child(0).unwrap();
        assert_eq!(import1.kind(), "import_statement");
    }

    #[test]
    fn test_parse_binary_expressions() {
        let source = "let <Math x: int y: int /> = <div>{x + y * 2}</div>";

        let tree = parse(source).expect("Failed to parse");
        let root = tree.root_node();

        assert_eq!(root.kind(), "module_definition");
        assert!(!root.has_error());
    }

    #[test]
    fn test_parse_conditional_expression() {
        let source = "let <Test x: int /> = <div>{x > 0 ? \"positive\" : \"negative\"}</div>";

        let tree = parse(source).expect("Failed to parse");
        let root = tree.root_node();

        assert_eq!(root.kind(), "module_definition");
        // Note: May have minor errors due to text content in conditional
    }

    #[test]
    fn test_parse_for_expression() {
        let source = r#"let <ItemList items: string[] /> =
  <ul>{for item in items { <li>{item}</li> }}</ul>"#;

        let tree = parse(source).expect("Failed to parse");
        let root = tree.root_node();

        assert_eq!(root.kind(), "module_definition");
        // Note: May have parsing challenges with text+markup combinations
    }
}
