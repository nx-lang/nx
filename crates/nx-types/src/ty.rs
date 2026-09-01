//! Type representation.
//!
//! Defines the core `Type` enum and related types.

use nx_hir::{same_declaration, Name};
use std::fmt;
use std::hash::{Hash, Hasher};

pub use nx_hir::DeclaringOrigin;

/// Arena index for types (for future interning/arena allocation).
pub type TypeId = u32;

/// Primitive type kinds.
///
/// Each primitive has exactly one spelling; there are no aliases or synonyms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Primitive {
    /// Default signed integer, exact over ±(2^53−1)
    ///
    /// `int` is the integer type NX programs use unless they have a specific reason not to. Its
    /// range is the widest that every NX backend represents exactly and cheaply: it fits in a C#
    /// `long`, a Rust `i64`, and — critically — a JavaScript `number`, which is exact only to
    /// 2^53−1. Backends are free to store it in whatever 64-bit-or-wider slot is natural, because
    /// the specified range makes that choice unobservable.
    Int,
    /// 32-bit signed integer
    Int32,
    /// 64-bit signed integer
    Int64,
    /// 32-bit floating-point
    Float32,
    /// 64-bit floating-point
    Float64,
    /// String type
    String,
    /// Boolean type
    Boolean,
    /// Void/unit type (functions with no return value)
    Void,
}

impl Primitive {
    /// Returns the name of this primitive type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Primitive::Int => "int",
            Primitive::Int32 => "int32",
            Primitive::Int64 => "int64",
            Primitive::Float32 => "float32",
            Primitive::Float64 => "float64",
            Primitive::String => "string",
            Primitive::Boolean => "boolean",
            Primitive::Void => "void",
        }
    }

    /// Returns true if this is any integer type (int, int32, int64).
    pub fn is_integer(&self) -> bool {
        matches!(self, Primitive::Int | Primitive::Int32 | Primitive::Int64)
    }

    /// Returns true if this is any float type (float32, float64).
    pub fn is_float(&self) -> bool {
        matches!(self, Primitive::Float32 | Primitive::Float64)
    }

    /// Returns true if this is any numeric type.
    pub fn is_numeric(&self) -> bool {
        self.is_integer() || self.is_float()
    }

    /// Returns the promoted type when combining two numeric primitives of the
    /// same category (both integer or both float). Returns `None` for
    /// cross-category combinations (e.g. int32 + float64).
    ///
    /// Promotion rules follow the integer rank order int32 < int < int64, so the wider operand
    /// wins:
    /// - int32 + int32 → int32
    /// - int32 + int → int
    /// - int + int → int
    /// - int64 with any integer → int64
    /// - float32 + float32 → float32
    /// - float32 + float64 → float64 (the wider operand wins)
    pub fn numeric_promotion(a: Primitive, b: Primitive) -> Option<Primitive> {
        if a.is_integer() && b.is_integer() {
            if matches!(a, Primitive::Int64) || matches!(b, Primitive::Int64) {
                Some(Primitive::Int64)
            } else if matches!(a, Primitive::Int) || matches!(b, Primitive::Int) {
                Some(Primitive::Int)
            } else {
                Some(Primitive::Int32)
            }
        } else if a.is_float() && b.is_float() {
            if matches!(a, Primitive::Float32) && matches!(b, Primitive::Float32) {
                Some(Primitive::Float32)
            } else {
                Some(Primitive::Float64)
            }
        } else {
            None
        }
    }
}

impl fmt::Display for Primitive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A type in the NX type system.
///
/// Types are immutable and can be shared via `Arc` for efficiency.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    /// Primitive type (int, int32, int64, float32, float64, string, boolean, void)
    Primitive(Primitive),

    /// Array type: T[]
    ///
    /// Example: `int[]`, `string[][]`
    Array(Box<Type>),

    /// Nullable type: T?
    ///
    /// Example: `int?`, `string?`
    Nullable(Box<Type>),

    /// Function type: (T1, T2, ...) => R
    ///
    /// Example: `(int, string) => boolean`
    Function {
        /// Parameter types
        params: Vec<Type>,
        /// Return type
        ret: Box<Type>,
    },

    /// A nominal type reached by name: a record, a component, a built-in like `Element`, or a
    /// name resolution reached no declaration for.
    ///
    /// A record carries the declaration it resolved to, so two same-named records in different
    /// modules are two types. A `Named` with no origin is one resolution reached nothing for, or
    /// one of the built-in names that has no declaration to point at.
    ///
    /// Example: `MyType`, `Person`
    Named(NamedType),

    /// Discriminated union type (nominal with fixed set of cases)
    Union(UnionType),

    /// Discriminated union case type scoped to an owning union.
    UnionCase(UnionCaseType),

    /// Type variable for inference (e.g., T0, T1, T2)
    ///
    /// Used during type inference before the concrete type is known.
    Variable(TypeId),

    /// A bare name awaiting resolution against the expected type of its binding site.
    ///
    /// Produced by inference for `Expr::ContextualName`, which has no context-free type. It is
    /// replaced by the resolved union case type at the binding site, and reaching a
    /// site that supplies no expected type is a diagnostic rather than a silent success.
    ContextualName(Name),

    /// Unknown type (inference failed or error)
    ///
    /// Used as a placeholder when type checking fails.
    Unknown,

    /// Error type (for error recovery)
    ///
    /// Used to continue type checking despite errors.
    Error,
}

impl Type {
    /// Creates a primitive int type, the default integer type.
    pub fn int() -> Self {
        Type::Primitive(Primitive::Int)
    }

    /// Creates a primitive int32 type.
    pub fn int32() -> Self {
        Type::Primitive(Primitive::Int32)
    }

    /// Creates a primitive int64 type.
    pub fn int64() -> Self {
        Type::Primitive(Primitive::Int64)
    }

    /// Creates a primitive float32 type.
    pub fn float32() -> Self {
        Type::Primitive(Primitive::Float32)
    }

    /// Creates a primitive float64 type.
    pub fn float64() -> Self {
        Type::Primitive(Primitive::Float64)
    }

    /// Creates a primitive string type.
    pub fn string() -> Self {
        Type::Primitive(Primitive::String)
    }

    /// Creates a primitive boolean type.
    pub fn boolean() -> Self {
        Type::Primitive(Primitive::Boolean)
    }

    /// Creates a primitive void type.
    pub fn void() -> Self {
        Type::Primitive(Primitive::Void)
    }

    /// Creates an array type.
    pub fn array(element: Type) -> Self {
        Type::Array(Box::new(element))
    }

    /// Creates a nullable type.
    pub fn nullable(inner: Type) -> Self {
        Type::Nullable(Box::new(inner))
    }

    /// Creates a function type.
    pub fn function(params: Vec<Type>, ret: Type) -> Self {
        Type::Function {
            params,
            ret: Box::new(ret),
        }
    }

    /// Creates a named type that resolution reached no declaration for.
    pub fn named(name: impl Into<Name>) -> Self {
        Type::Named(NamedType::new(name.into(), None))
    }

    /// Creates a named type for the declaration at `origin`.
    pub fn named_at(name: impl Into<Name>, origin: Option<DeclaringOrigin>) -> Self {
        Type::Named(NamedType::new(name.into(), origin))
    }

    /// Creates a discriminated union type declared at `origin`.
    pub fn union_type(
        name: impl Into<Name>,
        cases: Vec<Name>,
        base: Option<Name>,
        origin: Option<DeclaringOrigin>,
    ) -> Self {
        Type::Union(UnionType::new(name.into(), cases, base, origin))
    }

    /// Creates a discriminated union case type whose owning union is declared at `origin`.
    pub fn union_case_type(
        union: impl Into<Name>,
        case: impl Into<Name>,
        origin: Option<DeclaringOrigin>,
    ) -> Self {
        Type::UnionCase(UnionCaseType::new(union.into(), case.into(), origin))
    }

    /// Creates a type variable.
    pub fn var(id: TypeId) -> Self {
        Type::Variable(id)
    }

    /// Returns true if this is an error type.
    pub fn is_error(&self) -> bool {
        matches!(self, Type::Error)
    }

    /// Returns true if this is an unknown type.
    pub fn is_unknown(&self) -> bool {
        matches!(self, Type::Unknown)
    }

    /// Returns true if this is a type variable.
    pub fn is_variable(&self) -> bool {
        matches!(self, Type::Variable(_))
    }

    /// Returns true if this type is nullable.
    pub fn is_nullable(&self) -> bool {
        matches!(self, Type::Nullable(_))
    }

    /// Returns true if this is a primitive type.
    pub fn is_primitive(&self) -> bool {
        matches!(self, Type::Primitive(_))
    }

    /// Unwraps the inner type if this is nullable, otherwise returns self.
    pub fn strip_nullable(&self) -> &Type {
        match self {
            Type::Nullable(inner) => inner,
            _ => self,
        }
    }

    /// Checks if this type is compatible with another type.
    ///
    /// Compatibility includes:
    /// - Exact equality
    /// - Numeric width promotion within the same category (int32 ↔ int64, float32 ↔ float64)
    /// - Subtyping (e.g., T is compatible with T?)
    /// - Error types are compatible with everything (for error recovery)
    pub fn is_compatible_with(&self, other: &Type) -> bool {
        // Exact equality
        if self == other {
            return true;
        }

        // Error types are compatible with everything
        if self.is_error() || other.is_error() {
            return true;
        }

        // Unknown types are compatible with everything
        if self.is_unknown() || other.is_unknown() {
            return true;
        }

        // Numeric width promotion within the same category
        if let (Type::Primitive(a), Type::Primitive(b)) = (self, other) {
            if a.is_integer() && b.is_integer() {
                return true;
            }
            if a.is_float() && b.is_float() {
                return true;
            }
        }

        // T is compatible with T?
        if let Type::Nullable(inner) = other {
            if self.is_compatible_with(inner.as_ref()) {
                return true;
            }
        }

        // A case satisfies a union only when it is a case of *that* union — the one declared at
        // the same origin. Comparing names alone would let a same-named local declaration's case
        // stand in for a foreign union's, and comparing case lists as well still would where the
        // two declarations happen to agree on them.
        if let (Type::UnionCase(case), Type::Union(union)) = (self, other) {
            return case.is_case_of(union);
        }

        // Arrays: T[] is compatible with U[] if T is compatible with U
        if let (Type::Array(t1), Type::Array(t2)) = (self, other) {
            return t1.is_compatible_with(t2);
        }

        // Functions: (T1, T2) => R1 is compatible with (U1, U2) => R2
        // if U1 is compatible with T1, U2 is compatible with T2 (contravariant params)
        // and R1 is compatible with R2 (covariant return)
        if let (
            Type::Function {
                params: p1,
                ret: r1,
            },
            Type::Function {
                params: p2,
                ret: r2,
            },
        ) = (self, other)
        {
            if p1.len() != p2.len() {
                return false;
            }

            // Check parameters (contravariant)
            for (t1, t2) in p1.iter().zip(p2.iter()) {
                if !t2.is_compatible_with(t1) {
                    return false;
                }
            }

            // Check return type (covariant)
            return r1.is_compatible_with(r2);
        }

        false
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Primitive(p) => write!(f, "{}", p),
            Type::Array(elem) => write_postfix_type(f, elem, "[]"),
            Type::Nullable(inner) => write_postfix_type(f, inner, "?"),
            Type::Function { params, ret } => {
                write!(f, "(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ") => {}", ret)
            }
            Type::Named(named) => write!(f, "{}", named.name),
            Type::Union(union_ty) => write!(f, "{}", union_ty.name),
            Type::UnionCase(case_ty) => write!(f, "{}.{}", case_ty.union, case_ty.case),
            Type::Variable(id) => write!(f, "T{}", id),
            Type::ContextualName(name) => write!(f, "{}", name),
            Type::Unknown => write!(f, "?"),
            Type::Error => write!(f, "<error>"),
        }
    }
}

/// Renders two types for one diagnostic, qualifying them by declaring module only when needed.
///
/// <para>`expects Fit, found Fit` says nothing about why the two do not match. Qualifying every
/// nominal type in every message would be noise, so the declaring module is added exactly where the
/// display name alone cannot tell two different declarations apart.</para>
///
/// <para>Whether it can is decided on the nominal parts, not on the rendered strings. `expects Fit,
/// found Fit.cover` renders as two different strings and is exactly as ambiguous as the identical
/// pair: one `Fit` is the expectation and the other is the author's, and nothing on the line says
/// so.</para>
pub fn display_type_pair(lhs: &Type, rhs: &Type) -> (String, String) {
    if nominal_parts_collide(lhs, rhs) {
        return (qualified_display(lhs), qualified_display(rhs));
    }
    (lhs.to_string(), rhs.to_string())
}

/// Returns true when the two types spell one display name for two different declarations.
fn nominal_parts_collide(lhs: &Type, rhs: &Type) -> bool {
    let (mut lhs_parts, mut rhs_parts) = (Vec::new(), Vec::new());
    collect_nominal_parts(lhs, &mut lhs_parts);
    collect_nominal_parts(rhs, &mut rhs_parts);
    lhs_parts.iter().any(|(lhs_name, lhs_origin)| {
        rhs_parts.iter().any(|(rhs_name, rhs_origin)| {
            lhs_name == rhs_name && !same_declaration(*lhs_origin, lhs_name, *rhs_origin, rhs_name)
        })
    })
}

/// Collects every nominal declaration a type names, as `(display name, declaration)`.
///
/// A union case contributes its *union's* name, because that is the name the reader has to tell
/// apart — `Fit.cover` and `Fit` collide on `Fit`.
fn collect_nominal_parts<'ty>(
    ty: &'ty Type,
    parts: &mut Vec<(&'ty Name, Option<&'ty DeclaringOrigin>)>,
) {
    match ty {
        Type::Named(named) => parts.push((&named.name, named.origin())),
        Type::Union(union_ty) => parts.push((&union_ty.name, union_ty.origin())),
        Type::UnionCase(case_ty) => parts.push((&case_ty.union, case_ty.origin())),
        Type::Array(inner) | Type::Nullable(inner) => collect_nominal_parts(inner, parts),
        Type::Function { params, ret } => {
            for param in params {
                collect_nominal_parts(param, parts);
            }
            collect_nominal_parts(ret, parts);
        }
        _ => {}
    }
}

/// Renders a type with each nominal part prefixed by the module that declares it.
fn qualified_display(ty: &Type) -> String {
    match ty {
        Type::Union(union_ty) => match union_ty.origin() {
            Some(origin) => format!("{}:{}", origin.module_identity(), union_ty.name),
            None => union_ty.name.to_string(),
        },
        Type::UnionCase(case_ty) => match case_ty.origin() {
            Some(origin) => format!(
                "{}:{}.{}",
                origin.module_identity(),
                case_ty.union,
                case_ty.case
            ),
            None => format!("{}.{}", case_ty.union, case_ty.case),
        },
        Type::Named(named) => match named.origin() {
            Some(origin) => format!("{}:{}", origin.module_identity(), named.name),
            None => named.name.to_string(),
        },
        Type::Array(inner) => format!("{}[]", qualified_display(inner)),
        Type::Nullable(inner) => format!("{}?", qualified_display(inner)),
        Type::Function { params, ret } => {
            let params = params
                .iter()
                .map(qualified_display)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({}) => {}", params, qualified_display(ret))
        }
        _ => ty.to_string(),
    }
}

fn write_postfix_type(f: &mut fmt::Formatter<'_>, inner: &Type, suffix: &str) -> fmt::Result {
    match inner {
        Type::Function { .. } => write!(f, "({inner}){suffix}"),
        _ => write!(f, "{inner}{suffix}"),
    }
}

/// Hashes a nominal type consistently with [`same_declaration`].
fn hash_declaration<H: Hasher>(origin: Option<&DeclaringOrigin>, name: &Name, state: &mut H) {
    match origin {
        Some(origin) => origin.hash(state),
        None => name.hash(state),
    }
}

/// A nominal type reached by name, with the declaration that name reached.
#[derive(Debug, Clone)]
pub struct NamedType {
    /// The name, as displayed. Two declarations sharing one are still two types.
    pub name: Name,
    /// The declaration this name reached, where the resolving context reached one.
    origin: Option<DeclaringOrigin>,
}

impl NamedType {
    /// Creates a named type for the declaration at `origin`.
    pub fn new(name: Name, origin: Option<DeclaringOrigin>) -> Self {
        Self { name, origin }
    }

    /// Returns the declaration this name reached, if the building context reached one.
    pub fn origin(&self) -> Option<&DeclaringOrigin> {
        self.origin.as_ref()
    }

    /// Returns true when both names reached the same declaration.
    pub fn is_same_declaration_as(&self, other: &NamedType) -> bool {
        same_declaration(
            self.origin.as_ref(),
            &self.name,
            other.origin.as_ref(),
            &other.name,
        )
    }
}

impl PartialEq for NamedType {
    fn eq(&self, other: &Self) -> bool {
        self.is_same_declaration_as(other)
    }
}

impl Eq for NamedType {}

impl Hash for NamedType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_declaration(self.origin.as_ref(), &self.name, state);
    }
}

/// Describes a discriminated union type with its cases.
#[derive(Debug, Clone)]
pub struct UnionType {
    /// Union name, as displayed. Two unions sharing one are still two types.
    pub name: Name,
    /// Ordered case names
    pub cases: Vec<Name>,
    /// Optional abstract record base.
    pub base: Option<Name>,
    /// The declaration this union comes from, where the building context could name one.
    origin: Option<DeclaringOrigin>,
}

impl UnionType {
    /// Creates a new discriminated union type definition declared at `origin`.
    pub fn new(
        name: Name,
        cases: Vec<Name>,
        base: Option<Name>,
        origin: Option<DeclaringOrigin>,
    ) -> Self {
        Self {
            name,
            cases,
            base,
            origin,
        }
    }

    /// Returns the declaration this union comes from.
    pub fn origin(&self) -> Option<&DeclaringOrigin> {
        self.origin.as_ref()
    }

    /// Returns true when both denote the same declared union.
    pub fn is_same_union_as(&self, other: &UnionType) -> bool {
        same_declaration(
            self.origin.as_ref(),
            &self.name,
            other.origin.as_ref(),
            &other.name,
        )
    }
}

impl PartialEq for UnionType {
    fn eq(&self, other: &Self) -> bool {
        self.is_same_union_as(other)
    }
}

impl Eq for UnionType {}

impl Hash for UnionType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_declaration(self.origin.as_ref(), &self.name, state);
    }
}

/// Describes a discriminated union case type.
#[derive(Debug, Clone)]
pub struct UnionCaseType {
    /// Owning union name, as displayed.
    pub union: Name,
    /// Case name scoped under the owning union.
    pub case: Name,
    /// The declaration the owning union comes from.
    origin: Option<DeclaringOrigin>,
}

impl UnionCaseType {
    /// Creates a new union case type whose owning union is declared at `origin`.
    pub fn new(union: Name, case: Name, origin: Option<DeclaringOrigin>) -> Self {
        Self {
            union,
            case,
            origin,
        }
    }

    /// Returns the declaration the owning union comes from.
    pub fn origin(&self) -> Option<&DeclaringOrigin> {
        self.origin.as_ref()
    }

    /// Returns true when this case's owning union is the union `other` denotes.
    pub fn is_same_union_as(&self, other: &UnionType) -> bool {
        same_declaration(
            self.origin.as_ref(),
            &self.union,
            other.origin.as_ref(),
            &other.name,
        )
    }

    /// Returns true when both cases are scoped under the same declared union.
    pub fn shares_union_with(&self, other: &UnionCaseType) -> bool {
        same_declaration(
            self.origin.as_ref(),
            &self.union,
            other.origin.as_ref(),
            &other.union,
        )
    }

    /// Returns true when `union` is this case's owning union and declares it.
    pub fn is_case_of(&self, union: &UnionType) -> bool {
        self.is_same_union_as(union) && union.cases.contains(&self.case)
    }
}

impl PartialEq for UnionCaseType {
    fn eq(&self, other: &Self) -> bool {
        self.case == other.case && self.shares_union_with(other)
    }
}

impl Eq for UnionCaseType {}

impl Hash for UnionCaseType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_declaration(self.origin.as_ref(), &self.union, state);
        self.case.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_types() {
        assert_eq!(Type::int(), Type::Primitive(Primitive::Int));
        assert_eq!(Type::int32(), Type::Primitive(Primitive::Int32));
        assert_eq!(Type::int64(), Type::Primitive(Primitive::Int64));
        assert_eq!(Type::float32(), Type::Primitive(Primitive::Float32));
        assert_eq!(Type::float64(), Type::Primitive(Primitive::Float64));
        assert_eq!(Type::string(), Type::Primitive(Primitive::String));
        assert_eq!(Type::boolean(), Type::Primitive(Primitive::Boolean));
        assert_eq!(Type::void(), Type::Primitive(Primitive::Void));
    }

    #[test]
    fn test_each_primitive_has_one_name() {
        assert_eq!(Type::int().to_string(), "int");
        assert_eq!(Type::int32().to_string(), "int32");
        assert_eq!(Type::int64().to_string(), "int64");
        assert_eq!(Type::float32().to_string(), "float32");
        assert_eq!(Type::float64().to_string(), "float64");
        assert_eq!(Type::string().to_string(), "string");
        assert_eq!(Type::boolean().to_string(), "boolean");
        assert_eq!(Type::void().to_string(), "void");
    }

    #[test]
    fn test_width_inequality() {
        assert_ne!(Type::int32(), Type::int64());
        assert_ne!(Type::float32(), Type::float64());
    }

    #[test]
    fn test_cross_category_inequality() {
        assert_ne!(Type::int32(), Type::float32());
        assert_ne!(Type::int64(), Type::float64());
    }

    #[test]
    fn test_primitive_is_integer() {
        assert!(Primitive::Int.is_integer());
        assert!(Primitive::Int32.is_integer());
        assert!(Primitive::Int64.is_integer());
        assert!(!Primitive::Float32.is_integer());
        assert!(!Primitive::Float64.is_integer());
        assert!(!Primitive::String.is_integer());
    }

    #[test]
    fn test_primitive_is_float() {
        assert!(Primitive::Float32.is_float());
        assert!(Primitive::Float64.is_float());
        assert!(!Primitive::Int.is_float());
        assert!(!Primitive::Int32.is_float());
        assert!(!Primitive::Int64.is_float());
    }

    #[test]
    fn test_int_is_a_distinct_integer_primitive() {
        assert!(Primitive::Int.is_integer());
        assert!(Primitive::Int.is_numeric());
        assert_eq!(Primitive::Int.as_str(), "int");
        assert_eq!(Type::int(), Type::Primitive(Primitive::Int));

        // `int` is its own type, not a spelling of `int64`.
        assert_ne!(Primitive::Int, Primitive::Int64);
        assert_ne!(Type::int(), Type::int64());
    }

    #[test]
    fn test_numeric_promotion_integer_rank_order() {
        // int32 < int < int64, in both operand orders.
        for (a, b) in [
            (Primitive::Int32, Primitive::Int),
            (Primitive::Int, Primitive::Int32),
            (Primitive::Int, Primitive::Int),
        ] {
            assert_eq!(Primitive::numeric_promotion(a, b), Some(Primitive::Int));
        }
        for (a, b) in [
            (Primitive::Int, Primitive::Int64),
            (Primitive::Int64, Primitive::Int),
        ] {
            assert_eq!(Primitive::numeric_promotion(a, b), Some(Primitive::Int64));
        }

        // `int` stays in its own category.
        assert_eq!(
            Primitive::numeric_promotion(Primitive::Int, Primitive::Float64),
            None
        );
    }

    #[test]
    fn test_int_is_compatible_with_the_other_integer_widths() {
        assert!(Type::int().is_compatible_with(&Type::int32()));
        assert!(Type::int32().is_compatible_with(&Type::int()));
        assert!(Type::int().is_compatible_with(&Type::int64()));
        assert!(Type::int64().is_compatible_with(&Type::int()));
        assert!(!Type::int().is_compatible_with(&Type::float64()));
    }

    #[test]
    fn test_numeric_promotion() {
        // Same width
        assert_eq!(
            Primitive::numeric_promotion(Primitive::Int32, Primitive::Int32),
            Some(Primitive::Int32)
        );
        assert_eq!(
            Primitive::numeric_promotion(Primitive::Float32, Primitive::Float32),
            Some(Primitive::Float32)
        );

        // Cross width, same category: the wider operand wins in both orders
        assert_eq!(
            Primitive::numeric_promotion(Primitive::Int32, Primitive::Int64),
            Some(Primitive::Int64)
        );
        assert_eq!(
            Primitive::numeric_promotion(Primitive::Int64, Primitive::Int32),
            Some(Primitive::Int64)
        );
        assert_eq!(
            Primitive::numeric_promotion(Primitive::Float32, Primitive::Float64),
            Some(Primitive::Float64)
        );

        // Cross category: error
        assert_eq!(
            Primitive::numeric_promotion(Primitive::Int32, Primitive::Float32),
            None
        );
        assert_eq!(
            Primitive::numeric_promotion(Primitive::Int64, Primitive::Float64),
            None
        );
    }

    #[test]
    fn test_is_compatible_same_category_widths() {
        // int32 compatible with int64 (same category, different width)
        assert!(Type::int32().is_compatible_with(&Type::int64()));
        assert!(Type::int64().is_compatible_with(&Type::int32()));

        // float32 compatible with float64
        assert!(Type::float32().is_compatible_with(&Type::float64()));
        assert!(Type::float64().is_compatible_with(&Type::float32()));
    }

    #[test]
    fn test_is_not_compatible_cross_category() {
        // int32 not compatible with float32
        assert!(!Type::int32().is_compatible_with(&Type::float32()));
        assert!(!Type::int64().is_compatible_with(&Type::float64()));
    }

    #[test]
    fn test_array_type() {
        let arr = Type::array(Type::int());
        assert_eq!(arr, Type::Array(Box::new(Type::int())));
        assert_eq!(arr.to_string(), "int[]");
    }

    #[test]
    fn test_nullable_type() {
        let nullable = Type::nullable(Type::string());
        assert!(nullable.is_nullable());
        assert_eq!(nullable.to_string(), "string?");
    }

    #[test]
    fn test_function_type() {
        let func = Type::function(vec![Type::int(), Type::string()], Type::boolean());
        assert_eq!(func.to_string(), "(int, string) => boolean");
    }

    #[test]
    fn test_type_equality() {
        assert_eq!(Type::int(), Type::int());
        assert_ne!(Type::int(), Type::float64());
        assert_ne!(Type::int(), Type::nullable(Type::int()));
    }

    #[test]
    fn test_is_compatible_exact() {
        let t1 = Type::int();
        let t2 = Type::int();
        assert!(t1.is_compatible_with(&t2));
    }

    #[test]
    fn test_is_compatible_nullable() {
        let t = Type::int();
        let nullable_t = Type::nullable(Type::int());

        // T is compatible with T?
        assert!(t.is_compatible_with(&nullable_t));

        // But T? is not compatible with T
        assert!(!nullable_t.is_compatible_with(&t));
    }

    #[test]
    fn test_is_compatible_nullable_with_width_promotion() {
        // int32 should be compatible with int64? (via promotion + nullable)
        assert!(Type::int32().is_compatible_with(&Type::nullable(Type::int64())));
    }

    #[test]
    fn test_is_compatible_error() {
        let error = Type::Error;
        let int = Type::int();

        // Error types are compatible with everything
        assert!(error.is_compatible_with(&int));
        assert!(int.is_compatible_with(&error));
    }

    #[test]
    fn test_is_compatible_arrays() {
        let arr_int = Type::array(Type::int());
        let arr_int2 = Type::array(Type::int());
        let arr_string = Type::array(Type::string());

        assert!(arr_int.is_compatible_with(&arr_int2));
        assert!(!arr_int.is_compatible_with(&arr_string));
    }

    #[test]
    fn test_is_compatible_functions() {
        let f1 = Type::function(vec![Type::int()], Type::string());
        let f2 = Type::function(vec![Type::int()], Type::string());
        let f3 = Type::function(vec![Type::string()], Type::string());

        assert!(f1.is_compatible_with(&f2));
        assert!(!f1.is_compatible_with(&f3));
    }

    #[test]
    fn test_strip_nullable() {
        let nullable = Type::nullable(Type::int());
        assert_eq!(nullable.strip_nullable(), &Type::int());

        let non_nullable = Type::string();
        assert_eq!(non_nullable.strip_nullable(), &Type::string());
    }

    #[test]
    fn test_type_display() {
        assert_eq!(Type::array(Type::string()).to_string(), "string[]");
        assert_eq!(Type::nullable(Type::boolean()).to_string(), "boolean?");
        assert_eq!(
            Type::function(vec![Type::int(), Type::int()], Type::int()).to_string(),
            "(int, int) => int"
        );
        assert_eq!(
            Type::union_type(Name::new("Direction"), vec![Name::new("north")], None, None)
                .to_string(),
            "Direction"
        );
    }

    #[test]
    fn test_nested_types() {
        let nested = Type::array(Type::nullable(Type::int()));
        assert_eq!(nested.to_string(), "int?[]");

        let nullable_list = Type::nullable(Type::array(Type::string()));
        assert_eq!(nullable_list.to_string(), "string[]?");
        assert!(!nested.is_compatible_with(&nullable_list));
        assert!(!nullable_list.is_compatible_with(&nested));

        let func_array = Type::array(Type::function(vec![Type::int()], Type::string()));
        assert_eq!(func_array.to_string(), "((int) => string)[]");

        let nullable_func = Type::nullable(Type::function(vec![Type::int()], Type::string()));
        assert_eq!(nullable_func.to_string(), "((int) => string)?");
    }
}
