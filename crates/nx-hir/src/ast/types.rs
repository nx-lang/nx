//! Type reference AST nodes.
//!
//! These represent type annotations in the source code before type checking.
//! They are resolved to concrete types during type checking.

use crate::Name;

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
