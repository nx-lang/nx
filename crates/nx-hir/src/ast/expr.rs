//! Expression AST nodes.

use super::Occurrence;
use crate::{ElementId, ExprId, LocalDefinitionId, Name};
use nx_diagnostics::{TextSize, TextSpan};
use smol_str::SmolStr;

/// Literal value in source code.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Literal {
    /// String literal.
    ///
    /// Example: `"hello world"`
    String(SmolStr),

    /// Integer literal of type `int` or `int64`.
    ///
    /// Lowering gives every integer literal this form. Type analysis rewrites one written at an
    /// `int32` site to [`Literal::Int32`] and one written at a floating-point site to a real
    /// literal, so below analysis a literal's variant is its width. `int64` keeps this variant:
    /// it has no distinct runtime carrier yet.
    ///
    /// Example: `42`, `-10`
    Int(i64),

    /// Integer literal of type `int32`.
    ///
    /// Never produced by lowering: type analysis rewrites an [`Literal::Int`] written at an
    /// `int32` site into this, once the value is known to fit.
    Int32(i32),

    /// Floating-point literal of type `float64`.
    ///
    /// Example: `3.14`, `-0.5`
    Float(OrderedFloat),

    /// Floating-point literal of type `float32`.
    ///
    /// Never produced by lowering: type analysis rewrites a literal written at a `float32` site
    /// into this. The carried value is already rounded to the nearest `float32`, widened back to
    /// `f64` so the literal can be compared and hashed like [`Literal::Float`].
    Float32(OrderedFloat),

    /// Boolean literal.
    ///
    /// Example: `true`, `false`
    Boolean(bool),
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

/// Property assignment inside a record literal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecordLiteralProperty {
    /// Property key.
    pub name: Name,
    /// Property value expression.
    pub value: ExprId,
    /// Source span for the property assignment.
    pub span: TextSpan,
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
}

/// A primitive type as a HIR node names it after type analysis.
///
/// The type checker decides that a `+` concatenates and which operand needs its text form, and
/// records that decision on the module as [`Expr::ToText`] naming the operand's type with one of
/// these. It is also the width a numeric literal took from its site. Only the primitives a value
/// can have are here: `void` and `never` name no value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    Int,
    Int32,
    Int64,
    Float32,
    Float64,
    Boolean,
    String,
}

impl PrimitiveType {
    /// The type's source spelling, which is also its name in NX IR.
    pub fn as_str(self) -> &'static str {
        match self {
            PrimitiveType::Int => "int",
            PrimitiveType::Int32 => "int32",
            PrimitiveType::Int64 => "int64",
            PrimitiveType::Float32 => "float32",
            PrimitiveType::Float64 => "float64",
            PrimitiveType::Boolean => "boolean",
            PrimitiveType::String => "string",
        }
    }

    /// The primitive with this source spelling, if any.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "int" => PrimitiveType::Int,
            "int32" => PrimitiveType::Int32,
            "int64" => PrimitiveType::Int64,
            "float32" => PrimitiveType::Float32,
            "float64" => PrimitiveType::Float64,
            "boolean" => PrimitiveType::Boolean,
            "string" => PrimitiveType::String,
            _ => return None,
        })
    }
}

impl std::fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Unary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnOp {
    /// Negation: `-`
    Neg,
    /// Logical NOT: `!`
    Not,
}

/// One arm of a match-style `if value is { ... }` expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchArm {
    /// Patterns accepted by this arm.
    pub patterns: Vec<ExprId>,
    /// Body evaluated when any pattern matches.
    pub body: ExprId,
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

    /// Unresolved contextual literal: a bare name written where a literal is required.
    ///
    /// Resolved against the declared type of the binding site rather than against lexical scope,
    /// so it is deliberately a different node from [`Expr::Ident`]. Type analysis replaces it with
    /// a constant union case; reaching a site with no expected type is a
    /// diagnostic.
    ///
    /// <para>`occurrence` is the suffix glued to a name written as a property value: `int?` in
    /// `<Box T=int? />`. It is always an error, since a type argument is exactly one value and a
    /// value takes no suffix, but it is kept so the checker can say which. Only the first suffix
    /// is kept; post-parse validation reports a second, and a suffix outside a property value.</para>
    ///
    /// Example: `cover` in `<Img fit=cover />`
    ContextualName {
        name: Name,
        occurrence: Option<Occurrence>,
        span: TextSpan,
    },

    /// A union case reached by the declaring origin of its union rather than by a visible name.
    ///
    /// Produced by the rewrite that replaces a resolved [`Expr::ContextualName`]. The origin is the
    /// `(module identity, definition id)` pair a resolved program already addresses items by, so
    /// code generation and evaluation reach the declaration without the union being nameable at the
    /// use site. `union` is display information: the name the declaring module gave the type.
    ///
    /// Example: what `cover` becomes in `<Img fit=cover />` when `Fit` is not imported here
    ResolvedUnionCase {
        union: Name,
        case: Name,
        module_identity: String,
        definition_id: LocalDefinitionId,
        span: TextSpan,
    },

    /// Binary operation.
    ///
    /// Example: `a + b`, `x == y`
    BinaryOp {
        lhs: ExprId,
        op: BinOp,
        rhs: ExprId,
        span: TextSpan,
    },

    /// A range expression: `a..b` or `a..=b`.
    ///
    /// <para>Sugar for constructing the prelude's `Range`, and no more than that: the checker types
    /// it, and a post-check rewrite replaces it with the record construction it means, so nothing
    /// below the checker — the interpreter, NX IR, generated code — ever sees this node.</para>
    Range {
        start: ExprId,
        end: ExprId,
        /// `true` for `..=`, whose end is part of the range, and `false` for `..`.
        inclusive: bool,
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

    /// String concatenation: a `+` that type analysis found to have a string operand.
    ///
    /// Never produced by lowering, which emits [`BinOp::Add`] for every `+`; the type checker is
    /// the one place every operand's type is known, so it decides which additions concatenate and
    /// rewrites them to this. Both operands are strings by the time evaluation sees the node: a
    /// non-string operand was wrapped in an [`Expr::ToText`] by the same rewrite. A joined text
    /// body at a `string` content property is a chain of these too.
    ///
    /// Example: what `"Total: " + count` becomes
    Concat {
        lhs: ExprId,
        rhs: ExprId,
        span: TextSpan,
    },

    /// Conversion of a primitive value to its canonical text form.
    ///
    /// Produced by the same rewrite as [`Expr::Concat`], around each operand that is not already
    /// a string. The type named is the operand's static type, which is what lets a runtime that
    /// carries every number the same way still print a `float32` as a `float32`.
    ToText {
        expr: ExprId,
        ty: PrimitiveType,
        span: TextSpan,
    },

    /// Numeric widening of one branch of a join: an `if` or `match` result, or a list element.
    ///
    /// Never produced by lowering. When type analysis joins an `int` branch with a `float64` one,
    /// the join is a `float64`, and the type checker wraps the narrower branch in this node so the
    /// value it produces is one too. The type named is the numeric type the join has; a sequence
    /// or an optional value is widened item by item, and the empty value is left alone. A runtime
    /// that carries every number the same way has nothing to do here.
    ///
    /// Example: what `n` becomes in `if b { n } else { x }`, with `n:int` and `x:float64`
    Widen {
        expr: ExprId,
        ty: PrimitiveType,
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

    /// Match-style if expression.
    ///
    /// Example: `if state is { LoadState.failed => state.message else => "" }`
    Match {
        scrutinee: ExprId,
        arms: Vec<MatchArm>,
        else_branch: Option<ExprId>,
        span: TextSpan,
    },

    /// Let binding expression.
    ///
    /// Evaluates the value expression once and binds it to a name,
    /// then evaluates the body with that binding in scope.
    ///
    /// Example: `let x = expensive() in x + x`
    Let {
        name: Name,
        value: ExprId,
        body: ExprId,
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

    /// A sequence of items, written as a braced value list.
    ///
    /// <para>An element whose own type is a sequence contributes its items rather than nesting,
    /// so `{xs ys}` with two `string+` values is one `string+`. With no elements this is `{}`,
    /// the empty value — the one spelling of "no value", which the `{}` match pattern also
    /// lowers to.</para>
    ///
    /// Example: `{1 2 3}`, `{}`
    Array {
        elements: Vec<ExprId>,
        span: TextSpan,
    },

    /// Index operation. NX source has no way to write one; the variant is unreached.
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

    /// A step through a receiver that may be empty: `{}` when `base` is empty, `base.member`
    /// otherwise. The receiver is evaluated once.
    ///
    /// Example: `book.author?.name`
    OptionalMember {
        base: ExprId,
        member: Name,
        span: TextSpan,
    },

    /// The presence test: `true` when the operand holds at least one item.
    ///
    /// Example: `book.author?`
    Exists { operand: ExprId, span: TextSpan },

    /// The fallback: `left` when it holds an item, `right` otherwise. `right` is evaluated only
    /// then.
    ///
    /// Example: `book.subtitle ?? "none"`
    Coalesce {
        left: ExprId,
        right: ExprId,
        span: TextSpan,
    },

    /// Record literal instantiation.
    ///
    /// Example: `<User name="Bob" />`
    RecordLiteral {
        /// Record type name
        record: Name,
        /// Property assignments
        properties: Vec<RecordLiteralProperty>,
        /// Source span
        span: TextSpan,
    },

    /// Element literal expression.
    ///
    /// Example: `<button class="primary" />`
    Element { element: ElementId, span: TextSpan },

    /// Lazy component action handler callback.
    ///
    /// The body is lowered with an implicit `action` binding but is not evaluated
    /// until the handler is invoked by the interpreter.
    ActionHandler {
        /// Component the handler belongs to
        component: Name,
        /// Local emitted action name used by the component
        emit: Name,
        /// Exported action type name expected at invocation time
        action_name: Name,
        /// Stable identity of the module that declared the emit this handler binds to.
        ///
        /// <para>The action record `action_name` names is resolved here rather than in the module
        /// that wrote the binding, so a host-supplied action value is checked against the
        /// declaration the component actually emits.</para>
        ///
        /// <para>`None` means the emit is declared in the same module as the handler. Only the
        /// prepared-module rewrite can bind a handler to a component in another module; a binding
        /// lowered directly from source reaches a component declared alongside it, so there is no
        /// other module to name.</para>
        action_module_identity: Option<String>,
        /// Component whose declaration the binding was written in, or `None` at the root.
        ///
        /// <para>The owner decides where the handler's results go: its own update record patches
        /// the owner's state, and an action it emits goes to the owner's parent.</para>
        owner: Option<Name>,
        /// Handler body expression
        body: ExprId,
        /// Source span
        span: TextSpan,
    },

    /// For loop expression.
    ///
    /// Example: `for item in items { item * 2 }`
    /// Example with index: `for item, index in items { item + index }`
    For {
        /// Loop variable (item)
        item: Name,
        /// Optional index variable
        index: Option<Name>,
        /// Iterable expression
        iterable: ExprId,
        /// Loop body expression
        body: ExprId,
        span: TextSpan,
    },

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
            Expr::ContextualName { span, .. } => *span,
            Expr::ResolvedUnionCase { span, .. } => *span,
            Expr::BinaryOp { span, .. } => *span,
            Expr::Range { span, .. } => *span,
            Expr::UnaryOp { span, .. } => *span,
            Expr::Concat { span, .. } => *span,
            Expr::ToText { span, .. } => *span,
            Expr::Widen { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::If { span, .. } => *span,
            Expr::Match { span, .. } => *span,
            Expr::Let { span, .. } => *span,
            Expr::Block { span, .. } => *span,
            Expr::Array { span, .. } => *span,
            Expr::Index { span, .. } => *span,
            Expr::Member { span, .. } => *span,
            Expr::OptionalMember { span, .. } => *span,
            Expr::Exists { span, .. } => *span,
            Expr::Coalesce { span, .. } => *span,
            Expr::RecordLiteral { span, .. } => *span,
            Expr::Element { span, .. } => *span,
            Expr::ActionHandler { span, .. } => *span,
            Expr::For { span, .. } => *span,
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
        assert_eq!(Literal::Boolean(true), Literal::Boolean(true));
        assert_ne!(Literal::Boolean(true), Literal::Boolean(false));
    }

    #[test]
    fn test_ordered_float_equality() {
        let f1 = OrderedFloat(2.75);
        let f2 = OrderedFloat(2.75);
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
