//! Type reference AST nodes.
//!
//! These represent type annotations in the source code before type checking.
//! They are resolved to concrete types during type checking.

use crate::Name;

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

    /// Array type.
    ///
    /// Example: `int[]`, `string[]`
    Array(Box<TypeRef>),

    /// Nullable type.
    ///
    /// Example: `int?`, `string?`
    Nullable(Box<TypeRef>),

    /// Function type.
    ///
    /// Example: `(int, string) => bool`
    Function {
        /// Parameter types
        params: Vec<TypeRef>,
        /// Return type
        return_type: Box<TypeRef>,
    },
}

impl TypeRef {
    /// Create a named type reference.
    pub fn name(name: impl Into<Name>) -> Self {
        Self::Name(name.into())
    }

    /// Create an array type reference.
    pub fn array(element_type: TypeRef) -> Self {
        Self::Array(Box::new(element_type))
    }

    /// Create a nullable type reference.
    pub fn nullable(inner_type: TypeRef) -> Self {
        Self::Nullable(Box::new(inner_type))
    }

    /// Create a function type reference.
    pub fn function(params: Vec<TypeRef>, return_type: TypeRef) -> Self {
        Self::Function {
            params,
            return_type: Box::new(return_type),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_named_type() {
        let ty = TypeRef::name("int");
        match ty {
            TypeRef::Name(name) => assert_eq!(name.as_str(), "int"),
            _ => panic!("Expected Name variant"),
        }
    }

    #[test]
    fn test_array_type() {
        let ty = TypeRef::array(TypeRef::name("int"));
        match ty {
            TypeRef::Array(inner) => match *inner {
                TypeRef::Name(name) => assert_eq!(name.as_str(), "int"),
                _ => panic!("Expected Name variant"),
            },
            _ => panic!("Expected Array variant"),
        }
    }

    #[test]
    fn test_nullable_type() {
        let ty = TypeRef::nullable(TypeRef::name("string"));
        match ty {
            TypeRef::Nullable(inner) => match *inner {
                TypeRef::Name(name) => assert_eq!(name.as_str(), "string"),
                _ => panic!("Expected Name variant"),
            },
            _ => panic!("Expected Nullable variant"),
        }
    }

    #[test]
    fn test_function_type() {
        let ty = TypeRef::function(
            vec![TypeRef::name("int"), TypeRef::name("string")],
            TypeRef::name("bool"),
        );
        match ty {
            TypeRef::Function { params, return_type } => {
                assert_eq!(params.len(), 2);
                match &return_type.as_ref() {
                    TypeRef::Name(name) => assert_eq!(name.as_str(), "bool"),
                    _ => panic!("Expected Name variant"),
                }
            }
            _ => panic!("Expected Function variant"),
        }
    }
}
