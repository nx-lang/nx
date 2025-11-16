//! Type representation.
//!
//! Defines the core `Type` enum and related types.

use nx_hir::Name;
use std::fmt;

/// Arena index for types (for future interning/arena allocation).
pub type TypeId = u32;

/// Primitive type kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Primitive {
    /// Integer type
    Int,
    /// Floating-point type
    Float,
    /// String type
    String,
    /// Boolean type
    Bool,
    /// Void/unit type (functions with no return value)
    Void,
}

impl Primitive {
    /// Returns the name of this primitive type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Primitive::Int => "int",
            Primitive::Float => "float",
            Primitive::String => "string",
            Primitive::Bool => "bool",
            Primitive::Void => "void",
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
    /// Primitive type (int, float, string, bool, void)
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
    /// Creates a primitive int type.
    pub fn int() -> Self {
        Type::Primitive(Primitive::Int)
    }

    /// Creates a primitive float type.
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

        // T is compatible with T?
        if let Type::Nullable(inner) = other {
            if self == inner.as_ref() {
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
