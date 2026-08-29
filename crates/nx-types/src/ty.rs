//! Type representation.
//!
//! Defines the core `Type` enum and related types.

use nx_hir::Name;
use std::fmt;

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

    /// User-defined type (nominal type by name)
    ///
    /// Example: `MyType`, `Person`
    Named(Name),

    /// Enum type (nominal with fixed set of members)
    Enum(EnumType),

    /// Discriminated union type (nominal with fixed set of cases)
    Union(UnionType),

    /// Discriminated union case type scoped to an owning union.
    UnionCase(UnionCaseType),

    /// Type variable for inference (e.g., T0, T1, T2)
    ///
    /// Used during type inference before the concrete type is known.
    Variable(TypeId),

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

    /// Creates a named type.
    pub fn named(name: impl Into<Name>) -> Self {
        Type::Named(name.into())
    }

    /// Creates an enum type.
    pub fn enum_type(name: impl Into<Name>, members: Vec<Name>) -> Self {
        Type::Enum(EnumType::new(name.into(), members))
    }

    /// Creates a discriminated union type.
    pub fn union_type(name: impl Into<Name>, cases: Vec<Name>, base: Option<Name>) -> Self {
        Type::Union(UnionType::new(name.into(), cases, base))
    }

    /// Creates a discriminated union case type.
    pub fn union_case_type(union: impl Into<Name>, case: impl Into<Name>) -> Self {
        Type::UnionCase(UnionCaseType::new(union.into(), case.into()))
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

        if let (Type::UnionCase(case), Type::Union(union)) = (self, other) {
            return case.union == union.name;
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
            Type::Named(name) => write!(f, "{}", name),
            Type::Enum(enum_ty) => write!(f, "{}", enum_ty.name),
            Type::Union(union_ty) => write!(f, "{}", union_ty.name),
            Type::UnionCase(case_ty) => write!(f, "{}.{}", case_ty.union, case_ty.case),
            Type::Variable(id) => write!(f, "T{}", id),
            Type::Unknown => write!(f, "?"),
            Type::Error => write!(f, "<error>"),
        }
    }
}

fn write_postfix_type(f: &mut fmt::Formatter<'_>, inner: &Type, suffix: &str) -> fmt::Result {
    match inner {
        Type::Function { .. } => write!(f, "({inner}){suffix}"),
        _ => write!(f, "{inner}{suffix}"),
    }
}

/// Describes an enum type with its members.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumType {
    /// Enum name
    pub name: Name,
    /// Ordered member names
    pub members: Vec<Name>,
}

/// Describes a discriminated union type with its cases.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnionType {
    /// Union name
    pub name: Name,
    /// Ordered case names
    pub cases: Vec<Name>,
    /// Optional abstract record base.
    pub base: Option<Name>,
}

impl UnionType {
    /// Creates a new discriminated union type definition.
    pub fn new(name: Name, cases: Vec<Name>, base: Option<Name>) -> Self {
        Self { name, cases, base }
    }
}

/// Describes a discriminated union case type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnionCaseType {
    /// Owning union name.
    pub union: Name,
    /// Case name scoped under the owning union.
    pub case: Name,
}

impl UnionCaseType {
    /// Creates a new union case type.
    pub fn new(union: Name, case: Name) -> Self {
        Self { union, case }
    }
}

impl EnumType {
    /// Creates a new enum type definition.
    pub fn new(name: Name, members: Vec<Name>) -> Self {
        Self { name, members }
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
            Type::enum_type(Name::new("Direction"), vec![Name::new("north")]).to_string(),
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
