//! Typed wrappers around tree-sitter nodes.

use crate::syntax_kind::{syntax_kind_from_str, SyntaxKind};
use text_size::{TextRange, TextSize};
use tree_sitter::Node;

/// A typed wrapper around a tree-sitter `Node`.
///
/// This provides a more ergonomic API and integrates with the NX type system.
#[derive(Clone, Copy)]
pub struct SyntaxNode<'tree> {
    node: Node<'tree>,
    source: &'tree str,
}

impl<'tree> SyntaxNode<'tree> {
    /// Creates a new `SyntaxNode` from a tree-sitter `Node` and source text.
    pub fn new(node: Node<'tree>, source: &'tree str) -> Self {
        Self { node, source }
    }

    /// Returns the kind of this syntax node.
    pub fn kind(&self) -> SyntaxKind {
        syntax_kind_from_str(self.node.kind())
    }

    /// Returns the source text for this node.
    pub fn text(&self) -> &'tree str {
        self.node.utf8_text(self.source.as_bytes()).unwrap_or("")
    }

    /// Returns the text range (span) of this node in the source.
    pub fn span(&self) -> TextRange {
        let start = TextSize::from(
            u32::try_from(self.node.start_byte())
                .expect("NX source size should be validated before building syntax spans"),
        );
        let end = TextSize::from(
            u32::try_from(self.node.end_byte())
                .expect("NX source size should be validated before building syntax spans"),
        );
        TextRange::new(start, end)
    }

    /// Returns an iterator over the named child nodes, skipping comments.
    ///
    /// A comment is a named node in `extras`, so it appears among any node's named children. Every
    /// caller here reads children as the constructs a node is made of, and a comment is not one of
    /// those: a wrapper whose first child is a comment would otherwise lower the comment as the
    /// expression it wraps. `children_with_tokens` keeps them, for the prose scan that needs to know
    /// where they are.
    ///
    /// `child`, `child_count`, `next_sibling` and `prev_sibling` read the same view, so the i-th
    /// child is the i-th construct however the source is commented.
    pub fn children(&self) -> impl Iterator<Item = SyntaxNode<'tree>> {
        let node = self.node;
        let source = self.source;
        (0..node.named_child_count())
            .filter_map(move |i| node.named_child(i))
            .map(move |n| SyntaxNode::new(n, source))
            .filter(|child| !child.kind().is_comment())
    }

    /// Returns an iterator over all child nodes (including anonymous nodes).
    pub fn children_with_tokens(&self) -> impl Iterator<Item = SyntaxNode<'tree>> {
        let node = self.node;
        let source = self.source;
        (0..node.child_count())
            .filter_map(move |i| node.child(i))
            .map(move |n| SyntaxNode::new(n, source))
    }

    /// Returns a child node by its field name.
    pub fn child_by_field(&self, field: &str) -> Option<SyntaxNode<'tree>> {
        self.node
            .child_by_field_name(field)
            .map(|node| SyntaxNode::new(node, self.source))
    }

    /// Returns the parent node, if any.
    pub fn parent(&self) -> Option<SyntaxNode<'tree>> {
        self.node
            .parent()
            .map(|node| SyntaxNode::new(node, self.source))
    }

    /// Returns the next sibling node, skipping comments.
    pub fn next_sibling(&self) -> Option<SyntaxNode<'tree>> {
        let mut node = self.node;
        loop {
            node = node.next_named_sibling()?;
            let sibling = SyntaxNode::new(node, self.source);
            if !sibling.kind().is_comment() {
                return Some(sibling);
            }
        }
    }

    /// Returns the previous sibling node, skipping comments.
    pub fn prev_sibling(&self) -> Option<SyntaxNode<'tree>> {
        let mut node = self.node;
        loop {
            node = node.prev_named_sibling()?;
            let sibling = SyntaxNode::new(node, self.source);
            if !sibling.kind().is_comment() {
                return Some(sibling);
            }
        }
    }

    /// Returns true if this node represents an error.
    pub fn is_error(&self) -> bool {
        self.node.is_error() || self.node.is_missing() || self.kind() == SyntaxKind::ERROR
    }

    /// Returns true if this node has any errors in its subtree.
    pub fn has_error(&self) -> bool {
        self.node.has_error()
    }

    /// Returns the number of named children, not counting comments.
    pub fn child_count(&self) -> usize {
        self.children().count()
    }

    /// Returns a specific named child by index, not counting comments, so index `i` is the i-th
    /// construct the node is made of.
    pub fn child(&self, index: usize) -> Option<SyntaxNode<'tree>> {
        self.children().nth(index)
    }

    /// Returns the start byte position.
    pub fn start_byte(&self) -> usize {
        self.node.start_byte()
    }

    /// Returns the end byte position.
    pub fn end_byte(&self) -> usize {
        self.node.end_byte()
    }

    /// Returns the start position (line, column).
    pub fn start_position(&self) -> (usize, usize) {
        let pos = self.node.start_position();
        (pos.row, pos.column)
    }

    /// Returns the end position (line, column).
    pub fn end_position(&self) -> (usize, usize) {
        let pos = self.node.end_position();
        (pos.row, pos.column)
    }

    /// Returns the underlying tree-sitter node.
    ///
    /// This is provided for interoperability but should be used sparingly.
    pub fn raw(&self) -> Node<'tree> {
        self.node
    }
}

impl<'tree> std::fmt::Debug for SyntaxNode<'tree> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyntaxNode")
            .field("kind", &self.kind())
            .field("text", &self.text())
            .field("span", &self.span())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    #[test]
    fn test_syntax_node_kind() {
        let mut parser = parser();
        let source = "let x = 42";
        let tree = parser.parse(source, None).unwrap();
        let root = SyntaxNode::new(tree.root_node(), source);

        assert_eq!(root.kind(), SyntaxKind::MODULE_DEFINITION);
    }

    #[test]
    fn test_syntax_node_text() {
        let mut parser = parser();
        let source = "let x = 42";
        let tree = parser.parse(source, None).unwrap();
        let root = SyntaxNode::new(tree.root_node(), source);

        assert_eq!(root.text(), source);
    }

    #[test]
    fn test_syntax_node_children() {
        let mut parser = parser();
        let source = r#"import "./foo"
import { Bar } from "./bar""#;
        let tree = parser.parse(source, None).unwrap();
        let root = SyntaxNode::new(tree.root_node(), source);

        let children: Vec<_> = root.children().collect();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].kind(), SyntaxKind::IMPORT_STATEMENT);
        assert_eq!(children[1].kind(), SyntaxKind::IMPORT_STATEMENT);
    }

    /// A comment is a named node in `extras`, so it lands among a node's named children. Lowering
    /// reads a wrapper's first child as the expression it wraps, so a comment there is not a child
    /// it can use; `children_with_tokens` still sees everything, which is what prose scanning wants.
    #[test]
    fn children_skips_comments_and_children_with_tokens_keeps_them() {
        let mut parser = parser();
        let source = "let items = {\n  // leading\n  1 2\n}";
        let tree = parser.parse(source, None).unwrap();
        let root = SyntaxNode::new(tree.root_node(), source);
        let braced = find_kind(&root, SyntaxKind::VALUES_BRACED_EXPRESSION).expect("braced body");

        assert!(
            braced.children().all(|child| !child.kind().is_comment()),
            "children(): {:?}",
            braced.children().map(|c| c.kind()).collect::<Vec<_>>()
        );
        assert!(
            braced
                .children_with_tokens()
                .any(|child| child.kind().is_comment()),
            "children_with_tokens() should still see the comment"
        );
    }

    /// `child`, `child_count` and the sibling walkers read the same view as `children()`, so a
    /// comment between two constructs never shifts an index or stands between two siblings.
    #[test]
    fn index_and_sibling_access_skip_comments() {
        let mut parser = parser();
        let source = "let items = {\n  // leading\n  1\n  // between\n  2\n}";
        let tree = parser.parse(source, None).unwrap();
        let root = SyntaxNode::new(tree.root_node(), source);
        let braced = find_kind(&root, SyntaxKind::VALUES_BRACED_EXPRESSION).expect("braced body");

        assert_eq!(
            braced.child_count(),
            2,
            "child_count() should not count comments"
        );
        let first = braced.child(0).expect("first construct");
        let second = braced.child(1).expect("second construct");
        assert_eq!(first.text(), "1");
        assert_eq!(second.text(), "2");
        assert_eq!(
            first.next_sibling().map(|node| node.text()),
            Some("2"),
            "next_sibling() should step over the comment between them"
        );
        assert_eq!(
            second.prev_sibling().map(|node| node.text()),
            Some("1"),
            "prev_sibling() should step over the comment between them"
        );
    }

    fn find_kind<'tree>(node: &SyntaxNode<'tree>, kind: SyntaxKind) -> Option<SyntaxNode<'tree>> {
        if node.kind() == kind {
            return Some(*node);
        }
        node.children_with_tokens()
            .find_map(|child| find_kind(&child, kind))
    }

    #[test]
    fn test_syntax_node_span() {
        let mut parser = parser();
        let source = r#"import "./foo""#;
        let tree = parser.parse(source, None).unwrap();
        let root = SyntaxNode::new(tree.root_node(), source);

        let span = root.span();
        assert_eq!(span.start(), TextSize::from(0));
        assert_eq!(span.end(), TextSize::from(source.len() as u32));
    }

    #[test]
    fn test_syntax_node_is_error() {
        let mut parser = parser();
        let source = "let x = "; // Incomplete expression
        let tree = parser.parse(source, None).unwrap();
        let root = SyntaxNode::new(tree.root_node(), source);

        // The root might have errors in its subtree
        assert!(root.has_error());
    }

    #[test]
    fn test_import_statement_cst_structure() {
        let mut parser = parser();
        let source = r#"import { Button as Ui.Button, Input } from "./ui""#;
        let tree = parser.parse(source, None).unwrap();
        let root = SyntaxNode::new(tree.root_node(), source);

        let import = root
            .children()
            .find(|child| child.kind() == SyntaxKind::IMPORT_STATEMENT)
            .expect("Expected import_statement");

        let import_kind = import
            .child_by_field("kind")
            .expect("Import should expose kind field");
        assert_eq!(import_kind.kind(), SyntaxKind::SELECTIVE_IMPORT_LIST);

        let library_path = import
            .child_by_field("path")
            .expect("Import should expose library path field");
        assert_eq!(library_path.kind(), SyntaxKind::LIBRARY_PATH);

        let path_value = library_path
            .child_by_field("value")
            .expect("library_path should expose value field");
        assert_eq!(path_value.kind(), SyntaxKind::STRING_LITERAL);
        assert_eq!(path_value.text(), r#""./ui""#);

        let selective_imports: Vec<_> = import_kind
            .children()
            .filter(|child| child.kind() == SyntaxKind::SELECTIVE_IMPORT)
            .collect();
        assert_eq!(selective_imports.len(), 2);

        let first = selective_imports[0];
        let first_name = first
            .child_by_field("name")
            .expect("selective_import should expose name field");
        let first_alias = first
            .child_by_field("alias")
            .expect("selective_import should expose alias field");
        assert_eq!(first_name.text(), "Button");
        assert_eq!(first_alias.kind(), SyntaxKind::QUALIFIED_NAME);
        assert_eq!(first_alias.text(), "Ui.Button");

        let second = selective_imports[1];
        assert_eq!(
            second
                .child_by_field("name")
                .expect("second selective import should expose name")
                .text(),
            "Input"
        );
        assert!(
            second.child_by_field("alias").is_none(),
            "Second selective import should not have alias"
        );
    }

    #[test]
    fn test_visibility_modifier_cst_structure() {
        let mut parser = parser();
        let source = r#"private let title = "NX"
export component <Button/> = { <button/> }"#;
        let tree = parser.parse(source, None).unwrap();
        let root = SyntaxNode::new(tree.root_node(), source);

        let first = root.children().next().expect("Expected first module child");
        assert_eq!(first.kind(), SyntaxKind::VALUE_DEFINITION);
        assert_eq!(
            first
                .child_by_field("visibility")
                .expect("value_definition should expose visibility")
                .text(),
            "private"
        );

        let second = root
            .children()
            .nth(1)
            .expect("Expected second module child");
        assert_eq!(second.kind(), SyntaxKind::COMPONENT_DEFINITION);
        assert_eq!(
            second
                .child_by_field("visibility")
                .expect("component_definition should expose visibility")
                .text(),
            "export"
        );
    }
}
