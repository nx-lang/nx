//! Type representation.
//!
//! Defines the core `Type` enum and related types.

use nx_hir::Name;
use std::fmt;
use std::hash::{Hash, Hasher};

/// Arena index for types (for future interning/arena allocation).
pub type TypeId = u32;

/// Canonical discriminant for numeric primitive equality.
///
/// `Int` and `I64` map to the same canonical value, as do `Float` and `F64`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CanonicalPrimitive {
    I32,
    I64,
    F32,
    F64,
    String,
    Bool,
    Void,
}

/// Primitive type kinds.
///
/// `Int` is a display-preserving synonym for `I64`, and `Float` for `F64`.
/// They compare equal and hash identically via custom impls.
#[derive(Debug, Clone, Copy)]
pub enum Primitive {
    /// 32-bit signed integer
    I32,
    /// 64-bit signed integer
    I64,
    /// Synonym for I64 (displays as "int")
    Int,
    /// 32-bit floating-point
    F32,
    /// 64-bit floating-point
    F64,
    /// Synonym for F64 (displays as "float")
    Float,
    /// String type
    String,
    /// Boolean type
    Bool,
    /// Void/unit type (functions with no return value)
    Void,
}

impl Primitive {
    /// Returns the canonical discriminant for equality and hashing.
    fn canonical(&self) -> CanonicalPrimitive {
        match self {
            Primitive::I32 => CanonicalPrimitive::I32,
            Primitive::I64 | Primitive::Int => CanonicalPrimitive::I64,
            Primitive::F32 => CanonicalPrimitive::F32,
            Primitive::F64 | Primitive::Float => CanonicalPrimitive::F64,
            Primitive::String => CanonicalPrimitive::String,
            Primitive::Bool => CanonicalPrimitive::Bool,
            Primitive::Void => CanonicalPrimitive::Void,
        }
    }

    /// Returns the name of this primitive type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Primitive::I32 => "i32",
            Primitive::I64 => "i64",
            Primitive::Int => "int",
            Primitive::F32 => "f32",
            Primitive::F64 => "f64",
            Primitive::Float => "float",
            Primitive::String => "string",
            Primitive::Bool => "bool",
            Primitive::Void => "void",
        }
    }

    /// Returns true if this is any integer type (i32, i64, int).
    pub fn is_integer(&self) -> bool {
        matches!(self, Primitive::I32 | Primitive::I64 | Primitive::Int)
    }

    /// Returns true if this is any float type (f32, f64, float).
    pub fn is_float(&self) -> bool {
        matches!(self, Primitive::F32 | Primitive::F64 | Primitive::Float)
    }

    /// Returns true if this is any numeric type.
    pub fn is_numeric(&self) -> bool {
        self.is_integer() || self.is_float()
    }

    /// Returns the promoted type when combining two numeric primitives of the
    /// same category (both integer or both float). Returns `None` for
    /// cross-category combinations (e.g. i32 + f64).
    ///
    /// Promotion rules:
    /// - i32 + i32 → i32
    /// - i32 + i64/int → i64/int (preserves the wider operand's display)
    /// - f32 + f32 → f32
    /// - f32 + f64/float → f64/float (preserves the wider operand's display)
    pub fn numeric_promotion(a: Primitive, b: Primitive) -> Option<Primitive> {
        if a.is_integer() && b.is_integer() {
            if matches!(a, Primitive::I32) && matches!(b, Primitive::I32) {
                Some(Primitive::I32)
            } else if matches!(a, Primitive::I32) {
                Some(b)
            } else {
                Some(a)
            }
        } else if a.is_float() && b.is_float() {
            if matches!(a, Primitive::F32) && matches!(b, Primitive::F32) {
                Some(Primitive::F32)
            } else if matches!(a, Primitive::F32) {
                Some(b)
            } else {
                Some(a)
            }
        } else {
            None
        }
    }
}

impl PartialEq for Primitive {
    fn eq(&self, other: &Self) -> bool {
        self.canonical() == other.canonical()
    }
}

impl Eq for Primitive {}

impl Hash for Primitive {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.canonical().hash(state);
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
    /// Primitive type (i32, i64, int, f32, f64, float, string, bool, void)
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
    /// Example: `(int, string) => bool`
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
    /// Creates a primitive i32 type.
    pub fn i32() -> Self {
        Type::Primitive(Primitive::I32)
    }

    /// Creates a primitive i64 type.
    pub fn i64() -> Self {
        Type::Primitive(Primitive::I64)
    }

    /// Creates a primitive int type (synonym for i64, displays as "int").
    pub fn int() -> Self {
        Type::Primitive(Primitive::Int)
    }

    /// Creates a primitive f32 type.
    pub fn f32() -> Self {
        Type::Primitive(Primitive::F32)
    }

    /// Creates a primitive f64 type.
    pub fn f64() -> Self {
        Type::Primitive(Primitive::F64)
    }

    /// Creates a primitive float type (synonym for f64, displays as "float").
    pub fn float() -> Self {
        Type::Primitive(Primitive::Float)
    }

    /// Creates a primitive string type.
    pub fn string() -> Self {
        Type::Primitive(Primitive::String)
    }

    /// Creates a primitive bool type.
    pub fn bool() -> Self {
        Type::Primitive(Primitive::Bool)
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
    /// - Numeric width promotion within the same category (i32 ↔ i64, f32 ↔ f64)
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
            Type::Array(elem) => write!(f, "{}[]", elem),
            Type::Nullable(inner) => write!(f, "{}?", inner),
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
            Type::Variable(id) => write!(f, "T{}", id),
            Type::Unknown => write!(f, "?"),
            Type::Error => write!(f, "<error>"),
        }
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
        assert_eq!(Type::float(), Type::Primitive(Primitive::Float));
        assert_eq!(Type::string(), Type::Primitive(Primitive::String));
        assert_eq!(Type::bool(), Type::Primitive(Primitive::Bool));
        assert_eq!(Type::void(), Type::Primitive(Primitive::Void));
    }

    #[test]
    fn test_new_numeric_types() {
        assert_eq!(Type::i32(), Type::Primitive(Primitive::I32));
        assert_eq!(Type::i64(), Type::Primitive(Primitive::I64));
        assert_eq!(Type::f32(), Type::Primitive(Primitive::F32));
        assert_eq!(Type::f64(), Type::Primitive(Primitive::F64));
    }

    #[test]
    fn test_synonym_equality() {
        // int == i64 semantically
        assert_eq!(Type::int(), Type::i64());
        assert_eq!(Primitive::Int, Primitive::I64);

        // float == f64 semantically
        assert_eq!(Type::float(), Type::f64());
        assert_eq!(Primitive::Float, Primitive::F64);
    }

    #[test]
    fn test_synonym_display_preserving() {
        // Even though int == i64, they display differently
        assert_eq!(Type::int().to_string(), "int");
        assert_eq!(Type::i64().to_string(), "i64");
        assert_eq!(Type::float().to_string(), "float");
        assert_eq!(Type::f64().to_string(), "f64");
    }

    #[test]
    fn test_width_inequality() {
        // i32 != i64
        assert_ne!(Type::i32(), Type::i64());
        assert_ne!(Type::i32(), Type::int());
        // f32 != f64
        assert_ne!(Type::f32(), Type::f64());
        assert_ne!(Type::f32(), Type::float());
    }

    #[test]
    fn test_cross_category_inequality() {
        assert_ne!(Type::i32(), Type::f32());
        assert_ne!(Type::i64(), Type::f64());
        assert_ne!(Type::int(), Type::float());
    }

    #[test]
    fn test_primitive_is_integer() {
        assert!(Primitive::I32.is_integer());
        assert!(Primitive::I64.is_integer());
        assert!(Primitive::Int.is_integer());
        assert!(!Primitive::F32.is_integer());
        assert!(!Primitive::F64.is_integer());
        assert!(!Primitive::Float.is_integer());
        assert!(!Primitive::String.is_integer());
    }

    #[test]
    fn test_primitive_is_float() {
        assert!(Primitive::F32.is_float());
        assert!(Primitive::F64.is_float());
        assert!(Primitive::Float.is_float());
        assert!(!Primitive::I32.is_float());
        assert!(!Primitive::I64.is_float());
        assert!(!Primitive::Int.is_float());
    }

    #[test]
    fn test_numeric_promotion() {
        // Same width
        assert_eq!(Primitive::numeric_promotion(Primitive::I32, Primitive::I32), Some(Primitive::I32));
        assert_eq!(Primitive::numeric_promotion(Primitive::F32, Primitive::F32), Some(Primitive::F32));

        // Cross width, same category
        let result = Primitive::numeric_promotion(Primitive::I32, Primitive::I64);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), Primitive::I64); // wider wins

        let result = Primitive::numeric_promotion(Primitive::I32, Primitive::Int);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), Primitive::Int); // preserves display

        let result = Primitive::numeric_promotion(Primitive::F32, Primitive::F64);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), Primitive::F64);

        // Cross category: error
        assert_eq!(Primitive::numeric_promotion(Primitive::I32, Primitive::F32), None);
        assert_eq!(Primitive::numeric_promotion(Primitive::I64, Primitive::F64), None);
        assert_eq!(Primitive::numeric_promotion(Primitive::Int, Primitive::Float), None);
    }

    #[test]
    fn test_is_compatible_same_category_widths() {
        // i32 compatible with i64 (same category, different width)
        assert!(Type::i32().is_compatible_with(&Type::i64()));
        assert!(Type::i64().is_compatible_with(&Type::i32()));
        assert!(Type::i32().is_compatible_with(&Type::int()));

        // f32 compatible with f64
        assert!(Type::f32().is_compatible_with(&Type::f64()));
        assert!(Type::f64().is_compatible_with(&Type::f32()));
        assert!(Type::f32().is_compatible_with(&Type::float()));
    }

    #[test]
    fn test_is_not_compatible_cross_category() {
        // i32 not compatible with f32
        assert!(!Type::i32().is_compatible_with(&Type::f32()));
        assert!(!Type::i64().is_compatible_with(&Type::f64()));
        assert!(!Type::int().is_compatible_with(&Type::float()));
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
        let func = Type::function(vec![Type::int(), Type::string()], Type::bool());
        assert_eq!(func.to_string(), "(int, string) => bool");
    }

    #[test]
    fn test_type_equality() {
        assert_eq!(Type::int(), Type::int());
        assert_ne!(Type::int(), Type::float());
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
        // i32 should be compatible with i64? (via promotion + nullable)
        assert!(Type::i32().is_compatible_with(&Type::nullable(Type::i64())));
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
        assert_eq!(Type::int().to_string(), "int");
        assert_eq!(Type::i32().to_string(), "i32");
        assert_eq!(Type::i64().to_string(), "i64");
        assert_eq!(Type::f32().to_string(), "f32");
        assert_eq!(Type::f64().to_string(), "f64");
        assert_eq!(Type::float().to_string(), "float");
        assert_eq!(Type::array(Type::string()).to_string(), "string[]");
        assert_eq!(Type::nullable(Type::bool()).to_string(), "bool?");
        assert_eq!(
            Type::function(vec![Type::int(), Type::int()], Type::int()).to_string(),
            "(int, int) => int"
        );
        assert_eq!(
            Type::enum_type(Name::new("Direction"), vec![Name::new("North")]).to_string(),
            "Direction"
        );
    }

    #[test]
    fn test_nested_types() {
        let nested = Type::array(Type::nullable(Type::int()));
        assert_eq!(nested.to_string(), "int?[]");

        let func_array = Type::array(Type::function(vec![Type::int()], Type::string()));
        assert_eq!(func_array.to_string(), "(int) => string[]");
    }
}
