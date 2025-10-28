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
        let start = TextSize::from(self.node.start_byte() as u32);
        let end = TextSize::from(self.node.end_byte() as u32);
        TextRange::new(start, end)
    }

    /// Returns an iterator over the named child nodes.
    pub fn children(&self) -> impl Iterator<Item = SyntaxNode<'tree>> {
        let node = self.node;
        let source = self.source;
        (0..node.named_child_count())
            .filter_map(move |i| node.named_child(i))
            .map(move |n| SyntaxNode::new(n, source))
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

    /// Returns the next sibling node, if any.
    pub fn next_sibling(&self) -> Option<SyntaxNode<'tree>> {
        self.node
            .next_named_sibling()
            .map(|node| SyntaxNode::new(node, self.source))
    }

    /// Returns the previous sibling node, if any.
    pub fn prev_sibling(&self) -> Option<SyntaxNode<'tree>> {
        self.node
            .prev_named_sibling()
            .map(|node| SyntaxNode::new(node, self.source))
    }

    /// Returns true if this node represents an error.
    pub fn is_error(&self) -> bool {
        self.node.is_error() || self.node.is_missing() || self.kind() == SyntaxKind::ERROR
    }

    /// Returns true if this node has any errors in its subtree.
    pub fn has_error(&self) -> bool {
        self.node.has_error()
    }

    /// Returns the number of named children.
    pub fn child_count(&self) -> usize {
        self.node.named_child_count()
    }

    /// Returns a specific named child by index.
    pub fn child(&self, index: usize) -> Option<SyntaxNode<'tree>> {
        self.node
            .named_child(index)
            .map(|node| SyntaxNode::new(node, self.source))
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
        let source = "import foo\nimport bar";
        let tree = parser.parse(source, None).unwrap();
        let root = SyntaxNode::new(tree.root_node(), source);

        let children: Vec<_> = root.children().collect();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].kind(), SyntaxKind::IMPORT_STATEMENT);
        assert_eq!(children[1].kind(), SyntaxKind::IMPORT_STATEMENT);
    }

    #[test]
    fn test_syntax_node_span() {
        let mut parser = parser();
        let source = "import foo";
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
}
