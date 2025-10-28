//! Expression AST nodes.

use crate::{ElementId, ExprId, Name};
use nx_diagnostics::{TextSize, TextSpan};
use smol_str::SmolStr;

/// Literal value in source code.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Literal {
    /// String literal.
    ///
    /// Example: `"hello world"`
    String(SmolStr),

    /// Integer literal.
    ///
    /// Example: `42`, `-10`
    Int(i64),

    /// Floating-point literal.
    ///
    /// Example: `3.14`, `-0.5`
    Float(OrderedFloat),

    /// Boolean literal.
    ///
    /// Example: `true`, `false`
    Bool(bool),

    /// Null literal.
    ///
    /// Example: `null`
    Null,
}

/// Wrapper for f64 that implements Eq and Hash by treating NaN values as equal.
///
/// This is needed because f64 doesn't implement Eq/Hash due to NaN != NaN in IEEE 754.
/// For AST comparison purposes, we treat all NaN values as equivalent.
#[derive(Debug, Clone, Copy)]
pub struct OrderedFloat(pub f64);

impl PartialEq for OrderedFloat {
    fn eq(&self, other: &Self) -> bool {
        if self.0.is_nan() && other.0.is_nan() {
            true
        } else {
            self.0 == other.0
        }
    }
}

impl Eq for OrderedFloat {}

impl std::hash::Hash for OrderedFloat {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if self.0.is_nan() {
            // Hash all NaN values the same way
            state.write_u64(0);
        } else {
            state.write_u64(self.0.to_bits());
        }
    }
}

/// Binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    // Arithmetic
    Add, // +
    Sub, // -
    Mul, // *
    Div, // /
    Mod, // %

    // Comparison
    Eq, // ==
    Ne, // !=
    Lt, // <
    Le, // <=
    Gt, // >
    Ge, // >=

    // Logical
    And, // &&
    Or,  // ||

    // String
    Concat, // + (for strings)
}

/// Unary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnOp {
    /// Negation: `-`
    Neg,
    /// Logical NOT: `!`
    Not,
}

/// Expression AST node.
///
/// All expressions are stored in an arena and referenced by `ExprId`.
/// This enables efficient memory management and supports cyclic references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// Literal value.
    ///
    /// Example: `42`, `"hello"`, `true`
    Literal(Literal),

    /// Identifier reference.
    ///
    /// Example: `x`, `myVar`
    Ident(Name),

    /// Binary operation.
    ///
    /// Example: `a + b`, `x == y`
    BinaryOp {
        lhs: ExprId,
        op: BinOp,
        rhs: ExprId,
        span: TextSpan,
    },

    /// Unary operation.
    ///
    /// Example: `-x`, `!flag`
    UnaryOp {
        op: UnOp,
        expr: ExprId,
        span: TextSpan,
    },

    /// Function call.
    ///
    /// Example: `foo(1, 2)`, `bar()`
    Call {
        func: ExprId,
        args: Vec<ExprId>,
        span: TextSpan,
    },

    /// If expression.
    ///
    /// Example: `if x { y } else { z }`
    If {
        condition: ExprId,
        then_branch: ExprId,
        else_branch: Option<ExprId>,
        span: TextSpan,
    },

    /// Block expression.
    ///
    /// Example: `{ let x = 1; x + 2 }`
    Block {
        stmts: Vec<super::Stmt>,
        expr: Option<ExprId>,
        span: TextSpan,
    },

    /// Array literal.
    ///
    /// Example: `[1, 2, 3]`
    Array {
        elements: Vec<ExprId>,
        span: TextSpan,
    },

    /// Array index operation.
    ///
    /// Example: `arr[0]`, `matrix[i][j]`
    Index {
        base: ExprId,
        index: ExprId,
        span: TextSpan,
    },

    /// Member access.
    ///
    /// Example: `obj.field`
    Member {
        base: ExprId,
        member: Name,
        span: TextSpan,
    },

    /// Element literal expression.
    ///
    /// Example: `<button class="primary" />`
    Element { element: ElementId, span: TextSpan },

    /// Error placeholder for malformed expressions.
    ///
    /// This is used during lowering when the CST contains errors.
    Error(TextSpan),
}

impl Expr {
    /// Get the source span of this expression.
    pub fn span(&self) -> TextSpan {
        match self {
            Expr::Literal(_) => TextSpan::new(TextSize::from(0), TextSize::from(0)), // Literals don't track spans yet
            Expr::Ident(_) => TextSpan::new(TextSize::from(0), TextSize::from(0)),
            Expr::BinaryOp { span, .. } => *span,
            Expr::UnaryOp { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::If { span, .. } => *span,
            Expr::Block { span, .. } => *span,
            Expr::Array { span, .. } => *span,
            Expr::Index { span, .. } => *span,
            Expr::Member { span, .. } => *span,
            Expr::Element { span, .. } => *span,
            Expr::Error(span) => *span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_int() {
        let lit = Literal::Int(42);
        assert_eq!(lit, Literal::Int(42));
    }

    #[test]
    fn test_literal_string() {
        let lit = Literal::String(SmolStr::new("hello"));
        assert_eq!(lit, Literal::String(SmolStr::new("hello")));
    }

    #[test]
    fn test_literal_bool() {
        assert_eq!(Literal::Bool(true), Literal::Bool(true));
        assert_ne!(Literal::Bool(true), Literal::Bool(false));
    }

    #[test]
    fn test_literal_null() {
        assert_eq!(Literal::Null, Literal::Null);
    }

    #[test]
    fn test_ordered_float_equality() {
        let f1 = OrderedFloat(3.14);
        let f2 = OrderedFloat(3.14);
        assert_eq!(f1, f2);

        let nan1 = OrderedFloat(f64::NAN);
        let nan2 = OrderedFloat(f64::NAN);
        assert_eq!(nan1, nan2);
    }

    #[test]
    fn test_binop_variants() {
        assert_eq!(BinOp::Add, BinOp::Add);
        assert_ne!(BinOp::Add, BinOp::Sub);
    }

    #[test]
    fn test_unop_variants() {
        assert_eq!(UnOp::Neg, UnOp::Neg);
        assert_ne!(UnOp::Neg, UnOp::Not);
    }
}
