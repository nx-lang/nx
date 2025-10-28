//! Statement AST nodes.

use crate::{Name, ExprId};
use super::TypeRef;
use nx_diagnostics::{TextSpan, TextSize};

/// Statement AST node.
///
/// Statements are used within blocks and function bodies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    /// Let binding (variable declaration).
    ///
    /// Example: `let x = 42`, `let name: string = "Alice"`
    Let {
        /// Variable name
        name: Name,
        /// Optional type annotation (None means inferred)
        ty: Option<TypeRef>,
        /// Initializer expression
        init: ExprId,
        /// Source location
        span: TextSpan,
    },

    /// Expression statement.
    ///
    /// Example: `foo();`, `x + 1;`
    Expr(ExprId, TextSpan),
}

impl Stmt {
    /// Get the source span of this statement.
    pub fn span(&self) -> TextSpan {
        match self {
            Stmt::Let { span, .. } => *span,
            Stmt::Expr(_, span) => *span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Expr;
    use la_arena::{Arena, Idx};

    #[test]
    fn test_stmt_variants() {
        let mut arena = Arena::new();
        let expr_id = arena.alloc(Expr::Literal(super::super::Literal::Int(42)));

        let let_stmt = Stmt::Let {
            name: Name::new("x"),
            ty: None,
            init: expr_id,
            span: TextSpan::new(TextSize::from(0), TextSize::from(10)),
        };

        assert_eq!(let_stmt.span(), TextSpan::new(TextSize::from(0), TextSize::from(10)));

        let expr_stmt = Stmt::Expr(expr_id, TextSpan::new(TextSize::from(10), TextSize::from(15)));
        assert_eq!(expr_stmt.span(), TextSpan::new(TextSize::from(10), TextSize::from(15)));
    }
}
