//! Type reference AST nodes.
//!
//! These represent type annotations in the source code before type checking.
//! They are resolved to concrete types during type checking.

use super::expr::Literal;
use crate::Name;
use nx_diagnostics::TextSpan;
use smol_str::SmolStr;

/// How many values a type admits: the occurrence an `?`, `+` or `*` suffix spells.
///
/// <para>Two flags rather than an enum, so that every lattice operation is a bit operation. The
/// three language suffixes are the three combinations with at least one flag set; both flags
/// clear is exactly one, which a type spells by carrying no suffix at all. `join` takes the least
/// upper bound, `sum` is what two items contribute side by side in a braced value, `product` is
/// what a `for` over one occurrence yields with a body of another, and `without_zero` is what
/// `??` does to its left operand.</para>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Occurrence {
    /// The type admits no value: `?` or `*`.
    pub may_be_empty: bool,
    /// The type admits more than one value: `+` or `*`.
    pub may_be_many: bool,
}

impl Occurrence {
    /// Exactly one value: no suffix.
    pub const ONE: Self = Self {
        may_be_empty: false,
        may_be_many: false,
    };
    /// Zero or one value: `?`.
    pub const OPTIONAL: Self = Self {
        may_be_empty: true,
        may_be_many: false,
    };
    /// One or more values: `+`.
    pub const ONE_OR_MORE: Self = Self {
        may_be_empty: false,
        may_be_many: true,
    };
    /// Zero or more values: `*`.
    pub const ZERO_OR_MORE: Self = Self {
        may_be_empty: true,
        may_be_many: true,
    };

    /// The occurrence a suffix character spells, or `None` for anything else.
    pub fn from_suffix(suffix: char) -> Option<Self> {
        match suffix {
            '?' => Some(Self::OPTIONAL),
            '+' => Some(Self::ONE_OR_MORE),
            '*' => Some(Self::ZERO_OR_MORE),
            _ => None,
        }
    }

    /// True for exactly one: the absence of a suffix.
    pub fn is_one(self) -> bool {
        self == Self::ONE
    }

    /// True when the type admits no value.
    pub fn admits_zero(self) -> bool {
        self.may_be_empty
    }

    /// True when the type admits more than one value.
    pub fn admits_many(self) -> bool {
        self.may_be_many
    }

    /// The least upper bound in the lattice `1 ⊂ ?`, `1 ⊂ +`, `? ⊂ *`, `+ ⊂ *`: what the arms of a
    /// conditional, or the operands of `??`, admit together.
    pub fn join(self, other: Self) -> Self {
        Self {
            may_be_empty: self.may_be_empty || other.may_be_empty,
            may_be_many: self.may_be_many || other.may_be_many,
        }
    }

    /// What two items contribute side by side in a collecting position: at least one when either
    /// is, and always possibly many.
    pub fn sum(self, other: Self) -> Self {
        Self {
            may_be_empty: self.may_be_empty && other.may_be_empty,
            may_be_many: true,
        }
    }

    /// What a `for` over `self` yields with a body of `other`: at least one only when both are,
    /// bounded by one only when both are.
    pub fn product(self, other: Self) -> Self {
        self.join(other)
    }

    /// The occurrence with zero removed: `?` becomes exactly one, `*` becomes `+`.
    pub fn without_zero(self) -> Self {
        Self {
            may_be_empty: false,
            may_be_many: self.may_be_many,
        }
    }

    /// True when a value of occurrence `self` satisfies a site of occurrence `expected`: `self` is
    /// at or below `expected` in the lattice.
    pub fn satisfies(self, expected: Self) -> bool {
        (!self.may_be_empty || expected.may_be_empty) && (!self.may_be_many || expected.may_be_many)
    }

    /// The suffix that spells this occurrence: `""`, `"?"`, `"+"` or `"*"`.
    pub fn suffix(self) -> &'static str {
        match (self.may_be_empty, self.may_be_many) {
            (false, false) => "",
            (true, false) => "?",
            (false, true) => "+",
            (true, true) => "*",
        }
    }
}

impl std::fmt::Display for Occurrence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.suffix())
    }
}

/// Reference to a type in source code.
///
/// This is the syntactic representation of types before type checking.
/// During type checking, these are resolved to concrete `Type` values
/// in the nx-types crate.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeRef {
    /// Named type reference (primitive or user-defined).
    ///
    /// Examples: `int`, `string`, `MyType`
    Name(Name),

    /// A type under an occurrence suffix: zero or one (`?`), one or more (`+`) or zero or more
    /// (`*`) values of the inner type.
    ///
    /// <para>The inner type is an exactly-one type, never another `Seq`: `string??` and
    /// `(string+)*` are rejected by post-parse validation, and `Names*` where `Names` is a
    /// suffixed alias is rejected when the reference is resolved. Exactly one is the absence of
    /// this wrapper, so `occ` is never [`Occurrence::ONE`].</para>
    ///
    /// Example: `int?`, `string+`, `Person*`
    Seq {
        /// The item type.
        inner: Box<TypeRef>,
        /// How many items the type admits.
        occ: Occurrence,
    },

    /// Applied type: one instantiation of a generic record, spelled as the element that
    /// constructs it with only its type arguments.
    ///
    /// <para>Arguments are kept in source order and matched to the record's type parameters by
    /// name; the checker reorders them.</para>
    ///
    /// Example: `<Range T=int/>`, `<Range.Update T=int/>`
    Applied {
        /// The generic record's name, possibly qualified.
        name: Name,
        /// Type arguments as source wrote them, each binding a parameter name to a type.
        args: Vec<(Name, TypeRef)>,
    },

    /// Function type: an element function's signature with `function` in the name slot.
    ///
    /// Example: `<function Item:Contact Index:int />: DrawnNode`
    Function {
        /// Parameters, in declared order. A function is matched against the type by parameter
        /// name, so the order is display information.
        params: Vec<FunctionParam>,
        /// Return type
        return_type: Box<TypeRef>,
    },

    /// Function reference type: a function type whose parameter list is `...`, the type of a
    /// function of any parameters whose result satisfies the stated one.
    ///
    /// <para>A variant of its own rather than a flag on [`TypeRef::Function`], so that code which
    /// reads a function type's parameters cannot mistake this for a function of no parameters.
    /// </para>
    ///
    /// Example: `<function ... />: HttpArguments`, `<function ... />: object*`
    AnyFunction {
        /// Return type
        return_type: Box<TypeRef>,
    },

    /// A type with a `where` clause: the base type's values that match a numeric value pattern.
    ///
    /// <para>The clause follows the whole base, suffix included, so `int+ where >= 0` lowers to a
    /// constraint over `int+`, which the checker rejects with the `Score+` fix-it. A constraint
    /// over a constraint arises only through an alias, which the checker intersects.</para>
    ///
    /// Example: `float32 where in 0.0..=1.0`, `int where >= 1`
    Constrained {
        /// The constrained type.
        base: Box<TypeRef>,
        /// The pattern its values must match.
        pattern: NumericPattern,
    },
}

/// A numeric value pattern with no operand: range and comparison terms joined by `&&`.
///
/// <para>One structure serves a `where` clause and a match arm, since they are the same
/// patterns. Terms the language does not support (`||`, `!=`, an expression with an operand) are
/// kept as [`PatternTerm::Unsupported`] so that the checker can name them; lowering reports
/// nothing about them.</para>
///
/// <para>Equality and hashing read only the pattern's spelling, so the same constraint written
/// in two places is the same, as a type is.</para>
#[derive(Debug, Clone)]
pub struct NumericPattern {
    /// The terms, in source order.
    pub terms: Vec<PatternTerm>,
    /// The pattern's span, from its first term to its last.
    pub span: TextSpan,
}

impl NumericPattern {
    /// The pattern as source spells it, normalized: terms joined by ` && `.
    pub fn spelling(&self) -> String {
        let mut out = String::new();
        let mut previous_or = true;
        for term in &self.terms {
            let is_or = matches!(
                term,
                PatternTerm::Unsupported {
                    kind: UnsupportedTerm::Or,
                    ..
                }
            );
            if !out.is_empty() {
                out.push_str(if is_or || previous_or { " " } else { " && " });
            }
            out.push_str(&term.spelling());
            previous_or = is_or;
        }
        out
    }
}

impl NumericPattern {
    /// The interval the terms admit, read without a base type: bounds as written, no rounding
    /// and no integer normalization. `None` when a term is unsupported or unreadable.
    ///
    /// <para>What an engine with no type information tests a value against; for an integer value
    /// it admits the same set as the normalized interval.</para>
    pub fn interval_as_written(&self) -> Option<Interval> {
        let value = |bound: &PatternBound| match bound.value.as_ref()? {
            Literal::Int(value) => Some(*value as f64),
            Literal::Float(value) => Some(value.0),
            _ => None,
        };
        let mut interval = Interval::ALL;
        for term in &self.terms {
            let term = match term {
                PatternTerm::Finite => Interval {
                    low: Some(IntervalEnd::inclusive(-f64::MAX)),
                    high: Some(IntervalEnd::inclusive(f64::MAX)),
                },
                PatternTerm::Range {
                    start,
                    end,
                    inclusive,
                } => Interval {
                    low: Some(IntervalEnd::inclusive(value(start)?)),
                    high: Some(if *inclusive {
                        IntervalEnd::inclusive(value(end)?)
                    } else {
                        IntervalEnd::exclusive(value(end)?)
                    }),
                },
                PatternTerm::Compare { op, bound } => {
                    let bound = value(bound)?;
                    match op {
                        CompareOp::Lt => Interval {
                            low: None,
                            high: Some(IntervalEnd::exclusive(bound)),
                        },
                        CompareOp::Le => Interval {
                            low: None,
                            high: Some(IntervalEnd::inclusive(bound)),
                        },
                        CompareOp::Gt => Interval {
                            low: Some(IntervalEnd::exclusive(bound)),
                            high: None,
                        },
                        CompareOp::Ge => Interval {
                            low: Some(IntervalEnd::inclusive(bound)),
                            high: None,
                        },
                    }
                }
                PatternTerm::Unsupported { .. } => return None,
            };
            interval = interval.intersect(&term);
        }
        Some(interval)
    }
}

impl PartialEq for NumericPattern {
    fn eq(&self, other: &Self) -> bool {
        self.spelling() == other.spelling()
    }
}

impl Eq for NumericPattern {}

impl std::hash::Hash for NumericPattern {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.spelling().hash(state);
    }
}

/// One term of a [`NumericPattern`].
#[derive(Debug, Clone)]
pub enum PatternTerm {
    /// The finite values of a floating-point base.
    Finite,
    /// `in a..b` or `in a..=b`.
    Range {
        start: PatternBound,
        end: PatternBound,
        /// Whether the end is included: `..=`.
        inclusive: bool,
    },
    /// `< a`, `<= a`, `> a` or `>= a`.
    Compare { op: CompareOp, bound: PatternBound },
    /// Something a numeric pattern does not support, kept for the checker to name.
    Unsupported {
        /// What was written.
        kind: UnsupportedTerm,
        /// Where it was written.
        span: TextSpan,
        /// Its source text.
        text: SmolStr,
    },
}

impl PatternTerm {
    /// The term as source spells it.
    pub fn spelling(&self) -> String {
        match self {
            PatternTerm::Range {
                start,
                end,
                inclusive,
            } => format!(
                "in {}{}{}",
                start.spelling(),
                if *inclusive { "..=" } else { ".." },
                end.spelling()
            ),
            PatternTerm::Finite => "finite".to_owned(),
            PatternTerm::Compare { op, bound } => format!("{} {}", op.as_str(), bound.spelling()),
            PatternTerm::Unsupported { text, .. } => text.to_string(),
        }
    }
}

/// The comparison a [`PatternTerm::Compare`] makes against its bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompareOp {
    Lt,
    Le,
    Gt,
    Ge,
}

impl CompareOp {
    /// The operator's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            CompareOp::Lt => "<",
            CompareOp::Le => "<=",
            CompareOp::Gt => ">",
            CompareOp::Ge => ">=",
        }
    }
}

/// What a [`PatternTerm::Unsupported`] holds.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UnsupportedTerm {
    /// `||` between terms. Alternatives are commas, and only in a match arm.
    Or,
    /// `!= a`.
    NotEqual,
    /// An expression. When it is a comparison with a leading operand (`value >= 1`), the
    /// suggestion is the pattern without the operand (`>= 1`).
    Expression { suggestion: Option<SmolStr> },
}

/// One bound of a pattern term: a numeric literal, possibly negated.
#[derive(Debug, Clone)]
pub struct PatternBound {
    /// The literal's value: [`Literal::Int`] for an integer or hex literal, [`Literal::Float`] for
    /// a real one. `None` when the literal could not be read.
    pub value: Option<Literal>,
    /// The literal as written, sign included.
    pub text: SmolStr,
    /// Where it was written.
    pub span: TextSpan,
}

impl PatternBound {
    /// The bound as written.
    pub fn spelling(&self) -> &str {
        &self.text
    }
}

/// One end of an [`Interval`]: a bound and whether the bound itself is excluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntervalEnd {
    /// The bound. Every bound a constraint can state is exact in an `f64`: integer bounds lie
    /// within ±(2^53−1), and a `float32` bound is already rounded to `float32`.
    pub value: super::OrderedFloat,
    /// Whether the bound is excluded.
    pub exclusive: bool,
}

impl IntervalEnd {
    /// An included bound.
    pub fn inclusive(value: f64) -> Self {
        Self {
            value: super::OrderedFloat(value),
            exclusive: false,
        }
    }

    /// An excluded bound.
    pub fn exclusive(value: f64) -> Self {
        Self {
            value: super::OrderedFloat(value),
            exclusive: true,
        }
    }
}

/// The values a numeric constraint or pattern admits: one interval whose ends are each
/// inclusive, exclusive or open (`None`).
///
/// <para>The type checker builds one from a pattern's terms, intersecting them, and from an
/// alias chain, intersecting its constraints. On an integer base the checker normalizes an
/// exclusive end to the inclusive one next to it, so equal sets of integers are equal
/// intervals.</para>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Interval {
    /// The low end, or `None` when unbounded below.
    pub low: Option<IntervalEnd>,
    /// The high end, or `None` when unbounded above.
    pub high: Option<IntervalEnd>,
}

impl Interval {
    /// The interval with no ends: every value.
    pub const ALL: Self = Self {
        low: None,
        high: None,
    };

    /// Whether `value` lies in the interval. `NaN` lies in none.
    pub fn contains(&self, value: f64) -> bool {
        if value.is_nan() {
            return false;
        }
        let above_low = match self.low {
            None => true,
            Some(end) if end.exclusive => value > end.value.0,
            Some(end) => value >= end.value.0,
        };
        let below_high = match self.high {
            None => true,
            Some(end) if end.exclusive => value < end.value.0,
            Some(end) => value <= end.value.0,
        };
        above_low && below_high
    }

    /// The values both intervals admit.
    pub fn intersect(&self, other: &Self) -> Self {
        let low = match (self.low, other.low) {
            (None, end) | (end, None) => end,
            (Some(a), Some(b)) => Some(if a.value.0 > b.value.0 {
                a
            } else if b.value.0 > a.value.0 {
                b
            } else {
                IntervalEnd {
                    value: a.value,
                    exclusive: a.exclusive || b.exclusive,
                }
            }),
        };
        let high = match (self.high, other.high) {
            (None, end) | (end, None) => end,
            (Some(a), Some(b)) => Some(if a.value.0 < b.value.0 {
                a
            } else if b.value.0 < a.value.0 {
                b
            } else {
                IntervalEnd {
                    value: a.value,
                    exclusive: a.exclusive || b.exclusive,
                }
            }),
        };
        Self { low, high }
    }

    /// The interval with each exclusive end replaced by the inclusive integer next to it, as an
    /// integer base reads it: `> 0` is `>= 1`, `< 0.5` is `<= 0`.
    pub fn normalized_for_integers(&self) -> Self {
        let low = self.low.map(|end| {
            let floor = end.value.0.floor();
            if end.exclusive || floor != end.value.0 {
                IntervalEnd::inclusive(floor + 1.0)
            } else {
                IntervalEnd::inclusive(floor)
            }
        });
        let high = self.high.map(|end| {
            let ceil = end.value.0.ceil();
            if end.exclusive || ceil != end.value.0 {
                IntervalEnd::inclusive(ceil - 1.0)
            } else {
                IntervalEnd::inclusive(ceil)
            }
        });
        Self { low, high }
    }

    /// Whether no value lies in the interval. On an integer base, normalize first.
    pub fn is_empty(&self) -> bool {
        match (self.low, self.high) {
            (Some(low), Some(high)) => {
                low.value.0 > high.value.0
                    || (low.value.0 == high.value.0 && (low.exclusive || high.exclusive))
            }
            _ => false,
        }
    }

    /// Whether every value `self` admits, `other` admits too.
    pub fn is_within(&self, other: &Self) -> bool {
        let low_ok = match (self.low, other.low) {
            (_, None) => true,
            (None, Some(_)) => false,
            (Some(a), Some(b)) => {
                a.value.0 > b.value.0 || (a.value.0 == b.value.0 && (a.exclusive || !b.exclusive))
            }
        };
        let high_ok = match (self.high, other.high) {
            (_, None) => true,
            (None, Some(_)) => false,
            (Some(a), Some(b)) => {
                a.value.0 < b.value.0 || (a.value.0 == b.value.0 && (a.exclusive || !b.exclusive))
            }
        };
        low_ok && high_ok
    }

    /// Whether the interval has no ends.
    pub fn is_all(&self) -> bool {
        self.low.is_none() && self.high.is_none()
    }

    /// Spells the interval as a pattern: `in 0.0..=1.0` when both ends are present and the low one
    /// is inclusive, comparison terms joined by `&&` otherwise. `float` decides whether a whole
    /// bound is written with a fraction, `1.0` rather than `1`.
    pub fn spelling(&self, float: bool) -> String {
        self.spelled_with(|value| spell_bound(value, float))
    }

    /// The interval spelled on a `float32` base: each bound in the shortest digits that round to
    /// the same `float32`, `0.1` rather than the widening's `0.10000000149011612`.
    pub fn spelling_float32(&self) -> String {
        self.spelled_with(spell_float32_bound)
    }

    fn spelled_with(&self, bound: impl Fn(f64) -> String) -> String {
        match (self.low, self.high) {
            (Some(low), Some(high)) if !low.exclusive => format!(
                "in {}{}{}",
                bound(low.value.0),
                if high.exclusive { ".." } else { "..=" },
                bound(high.value.0)
            ),
            (low, high) => {
                let mut terms = Vec::new();
                if let Some(low) = low {
                    let op = if low.exclusive { ">" } else { ">=" };
                    terms.push(format!("{op} {}", bound(low.value.0)));
                }
                if let Some(high) = high {
                    let op = if high.exclusive { "<" } else { "<=" };
                    terms.push(format!("{op} {}", bound(high.value.0)));
                }
                terms.join(" && ")
            }
        }
    }
}

/// Spells a `float32` bound as a literal: the shortest digits that round to the same `float32`, with
/// at least one digit after the point.
pub fn spell_float32_bound(value: f64) -> String {
    let text = format!("{}", value as f32);
    if text.contains(['.', 'e', 'E']) || !value.is_finite() {
        text
    } else {
        format!("{text}.0")
    }
}

/// Spells a bound as a literal: an integer without a fraction, a float with at least one digit
/// after the point.
pub fn spell_bound(value: f64, float: bool) -> String {
    if !float {
        return format!("{}", value as i64);
    }
    let text = format!("{value}");
    if text.contains(['.', 'e', 'E']) || !value.is_finite() {
        text
    } else {
        format!("{text}.0")
    }
}

/// One parameter of a function type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionParam {
    /// Parameter name, which arguments bind to.
    pub name: Name,
    /// Parameter type.
    pub ty: TypeRef,
    /// Whether the parameter receives markup body content.
    pub is_content: bool,
    /// Whether the parameter carries the `?` mark (`Index?:int`): it may be omitted, and reads
    /// as a type that admits zero.
    pub optional: bool,
}

impl FunctionParam {
    /// Creates a plain (non-content) parameter.
    pub fn new(name: impl Into<Name>, ty: TypeRef) -> Self {
        Self {
            name: name.into(),
            ty,
            is_content: false,
            optional: false,
        }
    }

    /// Creates the content parameter.
    pub fn content(name: impl Into<Name>, ty: TypeRef) -> Self {
        Self {
            name: name.into(),
            ty,
            is_content: true,
            optional: false,
        }
    }

    /// Marks the parameter optional.
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }
}

impl TypeRef {
    /// Create a named type reference.
    pub fn name(name: impl Into<Name>) -> Self {
        Self::Name(name.into())
    }

    /// Puts `inner` under an occurrence. [`Occurrence::ONE`] returns `inner` itself, since exactly
    /// one is spelled by the absence of a suffix.
    pub fn seq(inner: TypeRef, occ: Occurrence) -> Self {
        if occ.is_one() {
            inner
        } else {
            Self::Seq {
                inner: Box::new(inner),
                occ,
            }
        }
    }

    /// Create a `T?` type reference.
    pub fn optional(inner: TypeRef) -> Self {
        Self::seq(inner, Occurrence::OPTIONAL)
    }

    /// Create a `T+` type reference.
    pub fn one_or_more(inner: TypeRef) -> Self {
        Self::seq(inner, Occurrence::ONE_OR_MORE)
    }

    /// Create a `T*` type reference.
    pub fn zero_or_more(inner: TypeRef) -> Self {
        Self::seq(inner, Occurrence::ZERO_OR_MORE)
    }

    /// The occurrence this reference carries: [`Occurrence::ONE`] unless it is a `Seq`.
    pub fn occurrence(&self) -> Occurrence {
        match self {
            Self::Seq { occ, .. } => *occ,
            _ => Occurrence::ONE,
        }
    }

    /// The type a property declared with this type reads at: the reference itself, or, when the
    /// property carries the `?` mark, the reference under the join of its occurrence with `?` —
    /// `T?` for `p?:T`, `T*` for `p?:T+`.
    pub fn read_type(&self, optional: bool) -> TypeRef {
        if !optional {
            return self.clone();
        }
        match self {
            Self::Seq { inner, occ } => {
                Self::seq((**inner).clone(), occ.join(Occurrence::OPTIONAL))
            }
            other => Self::optional(other.clone()),
        }
    }

    /// Create an applied type reference.
    pub fn applied(name: impl Into<Name>, args: Vec<(Name, TypeRef)>) -> Self {
        Self::Applied {
            name: name.into(),
            args,
        }
    }

    /// Create a function type reference.
    pub fn function(params: Vec<FunctionParam>, return_type: TypeRef) -> Self {
        Self::Function {
            params,
            return_type: Box::new(return_type),
        }
    }

    /// Create a function reference type, `<function ... />: R`.
    pub fn any_function(return_type: TypeRef) -> Self {
        Self::AnyFunction {
            return_type: Box::new(return_type),
        }
    }

    /// The reference lowering stands in for a type it could not lower: a syntax error, a missing
    /// type, or a construct post-parse validation has already rejected.
    ///
    /// <para>Its name is empty, which no source can spell, so the checker tells it apart from a
    /// written name and resolves it to an error without a report of its own: the syntax or
    /// validation error already says what is wrong there.</para>
    pub fn recovery() -> Self {
        Self::Name(Name::new(""))
    }

    /// Whether `name` is the one [`Self::recovery`] carries rather than one the source wrote.
    pub fn is_recovery_name(name: &Name) -> bool {
        name.as_str().is_empty()
    }
}

/// One parameter of a function type, already spelled: whether it takes body content, its name,
/// and its type as the caller renders types.
pub struct SpelledParam<'a> {
    /// Whether the parameter receives markup body content.
    pub is_content: bool,
    /// Parameter name.
    pub name: &'a str,
    /// Whether the parameter carries the `?` mark, spelled `name?:type`.
    pub optional: bool,
    /// The parameter's declared type (not its read type), spelled by the caller.
    pub ty: &'a str,
}

/// Spells a function type the way source writes one, `<function Item:Contact Index:int />: Node`.
///
/// <para>Diagnostics, hovers and the explained form of an IR artifact all show a function type,
/// and they read from three different representations — a checked `Type`, a `TypeRef`, and type
/// table indices — so each renders the parts itself and this assembles them. One spelling, in one
/// place, is what keeps the three from drifting.</para>
pub fn spell_function_type<'a>(
    params: impl IntoIterator<Item = SpelledParam<'a>>,
    result: &str,
) -> String {
    let mut out = String::from("<function");
    for param in params {
        out.push(' ');
        if param.is_content {
            out.push_str("content ");
        }
        out.push_str(param.name);
        if param.optional {
            out.push('?');
        }
        out.push(':');
        out.push_str(param.ty);
    }
    out.push_str(" />: ");
    out.push_str(result);
    out
}

/// Spells a function reference type the way source writes one, `<function ... />: Node`, for the
/// same three callers as [`spell_function_type`].
pub fn spell_function_reference_type(result: &str) -> String {
    format!("<function ... />: {result}")
}

/// Spells a type reference as source does, parenthesizing a function type under an occurrence
/// suffix, because a suffix written after the result would bind to the result instead.
pub fn spell_type_ref(ty: &TypeRef) -> String {
    match ty {
        TypeRef::Name(name) => name.as_str().to_string(),
        TypeRef::Applied { name, args } => spell_applied_type(
            name.as_str(),
            args.iter()
                .map(|(arg, ty)| (arg.as_str(), spell_type_ref(ty)))
                .collect::<Vec<_>>(),
        ),
        TypeRef::Seq { inner, occ } => format!("{}{occ}", spell_type_ref_under_suffix(inner)),
        TypeRef::Function {
            params,
            return_type,
        } => {
            let types: Vec<String> = params
                .iter()
                .map(|param| spell_type_ref(&param.ty))
                .collect();
            spell_function_type(
                params.iter().zip(&types).map(|(param, ty)| SpelledParam {
                    is_content: param.is_content,
                    name: param.name.as_str(),
                    optional: param.optional,
                    ty,
                }),
                &spell_type_ref(return_type),
            )
        }
        TypeRef::AnyFunction { return_type } => {
            spell_function_reference_type(&spell_type_ref(return_type))
        }
        TypeRef::Constrained { base, pattern } => {
            format!("{} where {}", spell_type_ref(base), pattern.spelling())
        }
    }
}

/// Spells an applied type as source does, `<Range T=int/>`.
///
/// <para>A checked type, a `TypeRef` and a diagnostic that names a missing argument all show one,
/// so the spelling lives here beside `spell_function_type` rather than in each of them.</para>
pub fn spell_applied_type<'a>(
    name: &str,
    args: impl IntoIterator<Item = (&'a str, String)>,
) -> String {
    let mut out = String::from("<");
    out.push_str(name);
    for (arg, ty) in args {
        out.push(' ');
        out.push_str(arg);
        out.push('=');
        out.push_str(&ty);
    }
    out.push_str("/>");
    out
}

/// A type reference under an occurrence suffix: a function type is parenthesized, anything else
/// is spelled as it stands.
pub fn spell_type_ref_under_suffix(ty: &TypeRef) -> String {
    match ty {
        TypeRef::Function { .. } | TypeRef::AnyFunction { .. } | TypeRef::Constrained { .. } => {
            format!("({})", spell_type_ref(ty))
        }
        _ => spell_type_ref(ty),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_function_type_is_spelled_as_source_writes_it() {
        let ty = TypeRef::function(
            vec![
                FunctionParam::new("Item", TypeRef::name("Contact")),
                FunctionParam::content("Children", TypeRef::one_or_more(TypeRef::name("Node"))),
            ],
            TypeRef::name("Node"),
        );
        assert_eq!(
            spell_type_ref(&ty),
            "<function Item:Contact content Children:Node+ />: Node"
        );
        assert_eq!(
            spell_type_ref(&TypeRef::optional(ty.clone())),
            "(<function Item:Contact content Children:Node+ />: Node)?"
        );
        assert_eq!(
            spell_type_ref(&TypeRef::one_or_more(ty.clone())),
            "(<function Item:Contact content Children:Node+ />: Node)+"
        );
        assert_eq!(
            spell_type_ref(&TypeRef::zero_or_more(TypeRef::name("int"))),
            "int*"
        );
    }

    #[test]
    fn exactly_one_is_no_wrapper() {
        assert_eq!(
            TypeRef::seq(TypeRef::name("int"), Occurrence::ONE),
            TypeRef::name("int")
        );
        assert_eq!(TypeRef::name("int").occurrence(), Occurrence::ONE);
        assert_eq!(
            TypeRef::one_or_more(TypeRef::name("int")).occurrence(),
            Occurrence::ONE_OR_MORE
        );
    }

    #[test]
    fn the_occurrence_lattice() {
        use Occurrence as O;
        // join: least upper bound
        assert_eq!(O::ONE.join(O::ONE), O::ONE);
        assert_eq!(O::ONE.join(O::OPTIONAL), O::OPTIONAL);
        assert_eq!(O::ONE.join(O::ONE_OR_MORE), O::ONE_OR_MORE);
        assert_eq!(O::OPTIONAL.join(O::ONE_OR_MORE), O::ZERO_OR_MORE);
        assert_eq!(O::ZERO_OR_MORE.join(O::ONE), O::ZERO_OR_MORE);
        // sum: two items side by side
        assert_eq!(O::ONE.sum(O::ONE), O::ONE_OR_MORE);
        assert_eq!(O::OPTIONAL.sum(O::ONE), O::ONE_OR_MORE);
        assert_eq!(O::OPTIONAL.sum(O::OPTIONAL), O::ZERO_OR_MORE);
        assert_eq!(O::ONE_OR_MORE.sum(O::OPTIONAL), O::ONE_OR_MORE);
        // product: a for over one with a body of the other
        assert_eq!(O::ONE_OR_MORE.product(O::ONE), O::ONE_OR_MORE);
        assert_eq!(O::ONE_OR_MORE.product(O::OPTIONAL), O::ZERO_OR_MORE);
        assert_eq!(O::OPTIONAL.product(O::ONE), O::OPTIONAL);
        assert_eq!(O::OPTIONAL.product(O::ONE_OR_MORE), O::ZERO_OR_MORE);
        // without zero
        assert_eq!(O::OPTIONAL.without_zero(), O::ONE);
        assert_eq!(O::ZERO_OR_MORE.without_zero(), O::ONE_OR_MORE);
        // satisfaction follows the order
        assert!(O::ONE.satisfies(O::OPTIONAL) && O::ONE.satisfies(O::ONE_OR_MORE));
        assert!(
            O::OPTIONAL.satisfies(O::ZERO_OR_MORE) && O::ONE_OR_MORE.satisfies(O::ZERO_OR_MORE)
        );
        assert!(!O::OPTIONAL.satisfies(O::ONE));
        assert!(!O::ZERO_OR_MORE.satisfies(O::ONE_OR_MORE));
        assert!(!O::ONE_OR_MORE.satisfies(O::OPTIONAL));
        // spelling
        assert_eq!(O::ONE.to_string(), "");
        assert_eq!(O::OPTIONAL.to_string(), "?");
        assert_eq!(O::ONE_OR_MORE.to_string(), "+");
        assert_eq!(O::ZERO_OR_MORE.to_string(), "*");
    }

    #[test]
    fn an_applied_type_is_spelled_as_source_writes_it() {
        let range = TypeRef::applied("Range", vec![(Name::new("T"), TypeRef::name("int"))]);
        assert_eq!(spell_type_ref(&range), "<Range T=int/>");
        // `/>` closes the applied type, so a suffix after it needs no parentheses.
        assert_eq!(
            spell_type_ref(&TypeRef::zero_or_more(range.clone())),
            "<Range T=int/>*"
        );
        assert_eq!(
            spell_type_ref(&TypeRef::applied(
                "Box",
                vec![(Name::new("T"), TypeRef::one_or_more(range))]
            )),
            "<Box T=<Range T=int/>+/>"
        );
        assert_eq!(
            spell_type_ref(&TypeRef::applied(
                "Pair",
                vec![
                    (Name::new("TKey"), TypeRef::name("string")),
                    (Name::new("TValue"), TypeRef::name("int")),
                ]
            )),
            "<Pair TKey=string TValue=int/>"
        );
        assert_eq!(
            spell_type_ref(&TypeRef::applied("Range", Vec::new())),
            "<Range/>"
        );
    }

    #[test]
    fn test_named_type() {
        let ty = TypeRef::name("int");
        match ty {
            TypeRef::Name(name) => assert_eq!(name.as_str(), "int"),
            _ => panic!("Expected Name variant"),
        }
    }

    #[test]
    fn test_seq_type() {
        let ty = TypeRef::one_or_more(TypeRef::name("int"));
        match ty {
            TypeRef::Seq { inner, occ } => {
                assert_eq!(occ, Occurrence::ONE_OR_MORE);
                match *inner {
                    TypeRef::Name(name) => assert_eq!(name.as_str(), "int"),
                    _ => panic!("Expected Name variant"),
                }
            }
            _ => panic!("Expected Seq variant"),
        }
    }

    #[test]
    fn test_function_type() {
        let ty = TypeRef::function(
            vec![
                FunctionParam::new("Count", TypeRef::name("int")),
                FunctionParam::content("Children", TypeRef::name("string")),
            ],
            TypeRef::name("boolean"),
        );
        match ty {
            TypeRef::Function {
                params,
                return_type,
            } => {
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].name.as_str(), "Count");
                assert!(!params[0].is_content);
                assert_eq!(params[1].name.as_str(), "Children");
                assert!(params[1].is_content);
                match &return_type.as_ref() {
                    TypeRef::Name(name) => assert_eq!(name.as_str(), "boolean"),
                    _ => panic!("Expected Name variant"),
                }
            }
            _ => panic!("Expected Function variant"),
        }
    }
}
