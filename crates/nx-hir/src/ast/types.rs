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

    /// Sequence type.
    ///
    /// <para>A sequence is flat, so a well-formed reference never wraps an `Array` in another
    /// one. `string[][]` is rejected by post-parse validation and `Names[]` where `Names` is a
    /// sequence alias is rejected when the reference is resolved.</para>
    ///
    /// Example: `int[]`, `string?[]`
    Array(Box<TypeRef>),

    /// Nullable type.
    ///
    /// Example: `int?`, `string?`
    Nullable(Box<TypeRef>),

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
}

impl FunctionParam {
    /// Creates a plain (non-content) parameter.
    pub fn new(name: impl Into<Name>, ty: TypeRef) -> Self {
        Self {
            name: name.into(),
            ty,
            is_content: false,
        }
    }

    /// Creates the content parameter.
    pub fn content(name: impl Into<Name>, ty: TypeRef) -> Self {
        Self {
            name: name.into(),
            ty,
            is_content: true,
        }
    }
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
}

/// One parameter of a function type, already spelled: whether it takes body content, its name,
/// and its type as the caller renders types.
pub struct SpelledParam<'a> {
    /// Whether the parameter receives markup body content.
    pub is_content: bool,
    /// Parameter name.
    pub name: &'a str,
    /// The parameter's type, spelled by the caller.
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
        out.push(':');
        out.push_str(param.ty);
    }
    out.push_str(" />: ");
    out.push_str(result);
    out
}

/// Spells a type reference as source does, parenthesizing a function type under a `[]` or `?`
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
        TypeRef::Array(inner) => format!("{}[]", spell_type_ref_under_suffix(inner)),
        TypeRef::Nullable(inner) => format!("{}?", spell_type_ref_under_suffix(inner)),
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
                    ty,
                }),
                &spell_type_ref(return_type),
            )
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

/// A type reference under a `[]` or `?` suffix: a function type is parenthesized, anything else
/// is spelled as it stands.
pub fn spell_type_ref_under_suffix(ty: &TypeRef) -> String {
    match ty {
        TypeRef::Function { .. } => format!("({})", spell_type_ref(ty)),
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
                FunctionParam::content("Children", TypeRef::array(TypeRef::name("Node"))),
            ],
            TypeRef::name("Node"),
        );
        assert_eq!(
            spell_type_ref(&ty),
            "<function Item:Contact content Children:Node[] />: Node"
        );
        assert_eq!(
            spell_type_ref(&TypeRef::nullable(ty.clone())),
            "(<function Item:Contact content Children:Node[] />: Node)?"
        );
        assert_eq!(
            spell_type_ref(&TypeRef::array(TypeRef::nullable(TypeRef::name("int")))),
            "int?[]"
        );
    }

    #[test]
    fn an_applied_type_is_spelled_as_source_writes_it() {
        let range = TypeRef::applied("Range", vec![(Name::new("T"), TypeRef::name("int"))]);
        assert_eq!(spell_type_ref(&range), "<Range T=int/>");
        // `/>` closes the applied type, so a suffix after it needs no parentheses.
        assert_eq!(
            spell_type_ref(&TypeRef::nullable(TypeRef::array(range.clone()))),
            "<Range T=int/>[]?"
        );
        assert_eq!(
            spell_type_ref(&TypeRef::applied(
                "Box",
                vec![(Name::new("T"), TypeRef::array(range))]
            )),
            "<Box T=<Range T=int/>[]/>"
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
