//! AST node traits for typed CST node casting.

use crate::{SyntaxKind, SyntaxNode};

/// Trait for typed AST nodes that can be cast from `SyntaxNode`.
///
/// This allows safe downcasting from generic `SyntaxNode` to specific
/// AST node types.
pub trait AstNode<'tree>: Sized {
    /// Returns the expected `SyntaxKind` for this AST node type.
    fn can_cast(kind: SyntaxKind) -> bool;

    /// Attempts to cast a `SyntaxNode` to this AST node type.
    ///
    /// Returns `Some(Self)` if the node's kind matches, `None` otherwise.
    fn cast(node: SyntaxNode<'tree>) -> Option<Self>;

    /// Returns the underlying `SyntaxNode`.
    fn syntax(&self) -> SyntaxNode<'tree>;
}

/// Extension trait for `SyntaxNode` to provide casting methods.
pub trait SyntaxNodeExt<'tree> {
    /// Attempts to cast this node to a specific AST node type.
    fn try_into<T: AstNode<'tree>>(self) -> Option<T>;
}

impl<'tree> SyntaxNodeExt<'tree> for SyntaxNode<'tree> {
    fn try_into<T: AstNode<'tree>>(self) -> Option<T> {
        T::cast(self)
    }
}

// Example AST node implementations

/// Represents a function definition in the AST.
#[derive(Debug, Clone, Copy)]
pub struct FunctionDef<'tree> {
    syntax: SyntaxNode<'tree>,
}

impl<'tree> AstNode<'tree> for FunctionDef<'tree> {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::FUNCTION_DEFINITION
    }

    fn cast(node: SyntaxNode<'tree>) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> SyntaxNode<'tree> {
        self.syntax
    }
}

impl<'tree> FunctionDef<'tree> {
    /// Returns the function signature node.
    pub fn signature(&self) -> Option<SyntaxNode<'tree>> {
        self.syntax.child_by_field("name")
    }

    /// Returns the function body node.
    pub fn body(&self) -> Option<SyntaxNode<'tree>> {
        self.syntax.child_by_field("body")
    }
}

/// Represents an element in the AST.
#[derive(Debug, Clone, Copy)]
pub struct Element<'tree> {
    syntax: SyntaxNode<'tree>,
}

impl<'tree> AstNode<'tree> for Element<'tree> {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(kind, SyntaxKind::ELEMENT | SyntaxKind::SELF_CLOSING_ELEMENT)
    }

    fn cast(node: SyntaxNode<'tree>) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> SyntaxNode<'tree> {
        self.syntax
    }
}

impl<'tree> Element<'tree> {
    /// Returns the open tag node.
    pub fn open_tag(&self) -> Option<SyntaxNode<'tree>> {
        self.syntax.child_by_field("open_tag")
    }

    /// Returns the close tag node.
    pub fn close_tag(&self) -> Option<SyntaxNode<'tree>> {
        self.syntax.child_by_field("close_tag")
    }

    /// Returns an iterator over child elements.
    pub fn children(&self) -> impl Iterator<Item = SyntaxNode<'tree>> + 'tree {
        self.syntax.children()
    }
}

/// Represents a type definition in the AST.
#[derive(Debug, Clone, Copy)]
pub struct TypeDef<'tree> {
    syntax: SyntaxNode<'tree>,
}

impl<'tree> AstNode<'tree> for TypeDef<'tree> {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::TYPE_DEFINITION
    }

    fn cast(node: SyntaxNode<'tree>) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> SyntaxNode<'tree> {
        self.syntax
    }
}

impl<'tree> TypeDef<'tree> {
    /// Returns the type name.
    pub fn name(&self) -> Option<SyntaxNode<'tree>> {
        self.syntax.child_by_field("name")
    }

    /// Returns the type definition.
    pub fn type_node(&self) -> Option<SyntaxNode<'tree>> {
        self.syntax.child_by_field("type")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    #[test]
    fn test_function_def_cast() {
        let mut parser = parser();
        let source = "let <Button text: string /> = <button>{text}</button>";
        let tree = parser.parse(source, None).unwrap();
        let root = SyntaxNode::new(tree.root_node(), source);

        let func = root.children().next().unwrap();
        assert!(FunctionDef::can_cast(func.kind()));

        let func_def = FunctionDef::cast(func);
        assert!(func_def.is_some());
    }

    #[test]
    fn test_type_def_cast() {
        let mut parser = parser();
        let source = "type UserId = int";
        let tree = parser.parse(source, None).unwrap();
        let root = SyntaxNode::new(tree.root_node(), source);

        let type_node = root.children().next().unwrap();
        assert!(TypeDef::can_cast(type_node.kind()));

        let type_def = TypeDef::cast(type_node);
        assert!(type_def.is_some());
    }

    #[test]
    fn test_syntax_node_ext() {
        let mut parser = parser();
        let source = "type UserId = int";
        let tree = parser.parse(source, None).unwrap();
        let root = SyntaxNode::new(tree.root_node(), source);

        let type_node = root.children().next().unwrap();
        let type_def = <SyntaxNode as SyntaxNodeExt>::try_into::<TypeDef>(type_node);
        assert!(type_def.is_some());
    }
}
