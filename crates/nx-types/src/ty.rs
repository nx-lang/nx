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
    /// The bottom type: the type of a value that does not exist.
    ///
    /// <para>Inference-internal: it can appear in a diagnostic but is never written in source, and
    /// it has no runtime representation, because no value has bottom type. What it exists for is
    /// the empty list: `{}` is a `never[]`, and `never` being below every type is what makes that
    /// one value usable at every list-typed site without the site having to be consulted.</para>
    Never,
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
            Primitive::Never => "never",
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

    /// Whether a value of this type has a canonical text form: the numeric types and `boolean`.
    ///
    /// <para>These are the types `+` joins to a string and a text body may embed. `string` is
    /// already text and needs no conversion, and `void` and `never` name no value.</para>
    pub fn is_stringifiable(&self) -> bool {
        self.is_numeric() || matches!(self, Primitive::Boolean)
    }

    /// This primitive as a HIR node names it, or `None` for the inference-internal ones.
    pub fn hir_type(&self) -> Option<nx_hir::ast::PrimitiveType> {
        use nx_hir::ast::PrimitiveType;
        Some(match self {
            Primitive::Int => PrimitiveType::Int,
            Primitive::Int32 => PrimitiveType::Int32,
            Primitive::Int64 => PrimitiveType::Int64,
            Primitive::Float32 => PrimitiveType::Float32,
            Primitive::Float64 => PrimitiveType::Float64,
            Primitive::String => PrimitiveType::String,
            Primitive::Boolean => PrimitiveType::Boolean,
            Primitive::Never => return None,
        })
    }

    /// The primitive a HIR node names.
    pub fn from_hir_type(ty: nx_hir::ast::PrimitiveType) -> Self {
        use nx_hir::ast::PrimitiveType;
        match ty {
            PrimitiveType::Int => Primitive::Int,
            PrimitiveType::Int32 => Primitive::Int32,
            PrimitiveType::Int64 => Primitive::Int64,
            PrimitiveType::Float32 => Primitive::Float32,
            PrimitiveType::Float64 => Primitive::Float64,
            PrimitiveType::String => Primitive::String,
            PrimitiveType::Boolean => Primitive::Boolean,
        }
    }

    /// Whether this floating-point primitive represents `value` exactly.
    ///
    /// <para>Always false for a non-floating-point primitive: the question is whether converting an
    /// integer to *this* float type loses nothing, and there is no conversion to ask about
    /// otherwise.</para>
    ///
    /// <para>The comparison is made in `i128` rather than by casting the float back to `i64`,
    /// because a float-to-integer `as` cast saturates. `i64::MAX` rounds to 2^63 as an `f64`, which
    /// saturates back to `i64::MAX` and would report an exact conversion that did not happen. Every
    /// `f64` reachable from an `i64` fits an `i128` unrounded, so that cast is the honest one.</para>
    pub fn represents_integer_exactly(&self, value: i64) -> bool {
        match self {
            Primitive::Float32 => (value as f32) as i128 == value as i128,
            Primitive::Float64 => (value as f64) as i128 == value as i128,
            _ => false,
        }
    }

    /// Whether a value of this primitive type is accepted, unchanged in meaning, at a site of
    /// `target`'s type.
    ///
    /// <para>This is the implicit numeric conversion lattice, written once: `int32 → int → int64`,
    /// `float32 → float64`, and the two exact crossings into floating point, `int32 → float64` and
    /// `int → float64`. A conversion is on the list only when it is total, exact, and has one
    /// obvious result. `int` is exact over ±(2^53−1), precisely the integer range a `float64`
    /// holds without loss, so those two crossings qualify; `int64` exceeds it and `float32` is
    /// exact only to ±2^24, so `int64` to any float and any integer to `float32` do not. Every
    /// primitive widens to itself. The relation runs one way: nothing narrows.</para>
    pub fn widens_to(self, target: Primitive) -> bool {
        if self == target {
            return true;
        }
        match self {
            Primitive::Int32 => matches!(
                target,
                Primitive::Int | Primitive::Int64 | Primitive::Float64
            ),
            Primitive::Int => matches!(target, Primitive::Int64 | Primitive::Float64),
            Primitive::Float32 => matches!(target, Primitive::Float64),
            _ => false,
        }
    }

    /// The narrowest numeric primitive both operands widen to, or `None` when there is none.
    ///
    /// <para>This is what a mixed arithmetic or comparison operation is typed at. It is computed
    /// from [`Primitive::widens_to`] over the rank order `int32, int, int64, float32, float64`
    /// rather than written as its own table, so the lattice has one home. The results that fall
    /// out: `int32 + int → int`, `int + int64 → int64`, `float32 + float64 → float64`,
    /// `int + float64 → float64`, `int32 + float32 → float64` (neither widens to `float32`, both
    /// widen exactly to `float64`), and `int64` with either float has no common type.</para>
    pub fn numeric_promotion(a: Primitive, b: Primitive) -> Option<Primitive> {
        const RANK: [Primitive; 5] = [
            Primitive::Int32,
            Primitive::Int,
            Primitive::Int64,
            Primitive::Float32,
            Primitive::Float64,
        ];
        if !a.is_numeric() || !b.is_numeric() {
            return None;
        }
        RANK.into_iter()
            .find(|candidate| a.widens_to(*candidate) && b.widens_to(*candidate))
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
    /// Primitive type (int, int32, int64, float32, float64, string, boolean, never)
    Primitive(Primitive),

    /// Sequence type: T[]
    ///
    /// <para>A sequence is flat: its element type is an item type, never another sequence. No
    /// type reference, alias chain, type-argument substitution or inference result produces an
    /// `Array` whose element is an `Array`.</para>
    ///
    /// Example: `int[]`, `string?[]`, `string[]?`
    Array(Box<Type>),

    /// Nullable type: T?
    ///
    /// Example: `int?`, `string?`
    Nullable(Box<Type>),

    /// Function type: an element function's signature with `function` in the name slot.
    ///
    /// Example: `<function Item:Contact Index:int />: DrawnNode`
    Function {
        /// Parameters, in declared order. A function satisfies a function type by parameter
        /// name, so the order is display information.
        params: Vec<FunctionParam>,
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

    /// A component type parameter, rigid within the declaration that introduced it.
    ///
    /// Satisfied only by itself and the bottom type; satisfies itself and `object`. Identified by
    /// the declaring component and the parameter's position there, so two parameters spelled alike
    /// on two components are two types. Never reaches a value: a use site replaces it with the
    /// argument it bound, or with the bottom type when it bound none.
    Parameter(TypeParameterRef),

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

/// One parameter of a function type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionParam {
    /// Parameter name, which arguments bind to.
    pub name: Name,
    /// Parameter type.
    pub ty: Type,
    /// Whether the parameter receives markup body content.
    pub is_content: bool,
}

impl FunctionParam {
    /// Creates a plain (non-content) parameter.
    pub fn new(name: impl Into<Name>, ty: Type) -> Self {
        Self {
            name: name.into(),
            ty,
            is_content: false,
        }
    }

    /// Creates the content parameter.
    pub fn content(name: impl Into<Name>, ty: Type) -> Self {
        Self {
            name: name.into(),
            ty,
            is_content: true,
        }
    }
}

/// Why a function fails to satisfy a function type.
///
/// A plain "expects F, found G" leaves the reader comparing two signatures by eye; the reason
/// names the parameter that decided it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionMismatch {
    /// The function declares a parameter the type does not supply. A function may declare fewer
    /// parameters than its type, never more.
    UnsuppliedParameter(Name),
    /// A parameter is the content parameter on one side only.
    ContentMismatch(Name),
    /// What the type supplies for a parameter is not acceptable to the function.
    ParameterType {
        name: Name,
        supplied: Type,
        declared: Type,
    },
    /// The function's result is not acceptable where the type's result is expected.
    Result { returned: Type, expected: Type },
}

impl fmt::Display for FunctionMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FunctionMismatch::UnsuppliedParameter(name) => write!(
                f,
                "the function declares parameter '{name}', which the function type does not supply"
            ),
            FunctionMismatch::ContentMismatch(name) => write!(
                f,
                "parameter '{name}' is the content parameter on one side only"
            ),
            FunctionMismatch::ParameterType {
                name,
                supplied,
                declared,
            } => {
                let (supplied, declared) = display_type_pair(supplied, declared);
                write!(
                    f,
                    "parameter '{name}' is supplied as {supplied}, which is not {declared}"
                )
            }
            FunctionMismatch::Result { returned, expected } => {
                let (returned, expected) = display_type_pair(returned, expected);
                write!(f, "the result {returned} is not {expected}")
            }
        }
    }
}

/// Checks a function's type against a function type by parameter name.
///
/// <para>`satisfies(value, target)` decides each pairing: for a parameter, whether what the type
/// supplies satisfies what the function declares (contravariance); for the result, whether what
/// the function returns satisfies what the type expects (covariance). Every parameter the
/// function declares must be one the type supplies, under the same name and the same `content`
/// marking; the function may leave parameters of the type undeclared, and a caller supplying them
/// is what makes that safe. Parameter order is not compared.</para>
// `FunctionMismatch` carries the two types its message prints, so the `Err` is large by design.
// The caller turns it straight into a diagnostic and there is one per mismatched binding.
#[allow(clippy::result_large_err)]
pub fn check_function_satisfies(
    actual_params: &[FunctionParam],
    actual_ret: &Type,
    expected_params: &[FunctionParam],
    expected_ret: &Type,
    satisfies: &mut dyn FnMut(&Type, &Type) -> bool,
) -> Result<(), FunctionMismatch> {
    for declared in actual_params {
        let Some(supplied) = expected_params
            .iter()
            .find(|param| param.name == declared.name)
        else {
            return Err(FunctionMismatch::UnsuppliedParameter(declared.name.clone()));
        };
        if supplied.is_content != declared.is_content {
            return Err(FunctionMismatch::ContentMismatch(declared.name.clone()));
        }
        if !satisfies(&supplied.ty, &declared.ty) {
            return Err(FunctionMismatch::ParameterType {
                name: declared.name.clone(),
                supplied: supplied.ty.clone(),
                declared: declared.ty.clone(),
            });
        }
    }
    if !satisfies(actual_ret, expected_ret) {
        return Err(FunctionMismatch::Result {
            returned: actual_ret.clone(),
            expected: expected_ret.clone(),
        });
    }
    Ok(())
}

/// Renders a function type in NX spelling, with `render` spelling each part.
///
/// <para>The spelling itself belongs to [`nx_hir::ast::spell_function_type`], which the hover and
/// the explained form of an IR artifact render through too; only the parts are this crate's.</para>
fn format_function_type(
    params: &[FunctionParam],
    ret: &Type,
    render: &dyn Fn(&Type) -> String,
) -> String {
    let types: Vec<String> = params.iter().map(|param| render(&param.ty)).collect();
    nx_hir::ast::spell_function_type(
        params
            .iter()
            .zip(&types)
            .map(|(param, ty)| nx_hir::ast::SpelledParam {
                is_content: param.is_content,
                name: param.name.as_str(),
                ty,
            }),
        &render(ret),
    )
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

    /// Creates the bottom type, which is below every type and which no value inhabits.
    pub fn never() -> Self {
        Type::Primitive(Primitive::Never)
    }

    /// Creates the rigid type of a component type parameter.
    pub fn parameter(
        name: impl Into<Name>,
        owner: Option<DeclaringOrigin>,
        ordinal: usize,
    ) -> Self {
        Type::Parameter(TypeParameterRef {
            name: name.into(),
            owner,
            ordinal,
        })
    }

    /// The first type parameter in this type that `matches`, looking through lists, nullables,
    /// and function signatures.
    pub fn find_parameter(
        &self,
        matches: &impl Fn(&TypeParameterRef) -> bool,
    ) -> Option<&TypeParameterRef> {
        match self {
            Type::Parameter(param) if matches(param) => Some(param),
            Type::Named(named) => named
                .args()
                .iter()
                .find_map(|(_, arg)| arg.find_parameter(matches)),
            Type::Array(inner) | Type::Nullable(inner) => inner.find_parameter(matches),
            Type::Function { params, ret } => params
                .iter()
                .find_map(|param| param.ty.find_parameter(matches))
                .or_else(|| ret.find_parameter(matches)),
            _ => None,
        }
    }

    /// Replaces each type parameter `substitute` answers for, keeping every list, nullable, and
    /// function layer around it. A parameter it answers `None` for is left as it is.
    pub fn substitute_parameters(
        &self,
        substitute: &impl Fn(&TypeParameterRef) -> Option<Type>,
    ) -> Type {
        match self {
            Type::Parameter(param) => substitute(param).unwrap_or_else(|| self.clone()),
            Type::Named(named) if !named.args().is_empty() => Type::Named(
                named.with_args(
                    named
                        .args()
                        .iter()
                        .map(|(param, ty)| (param.clone(), ty.substitute_parameters(substitute)))
                        .collect(),
                ),
            ),
            Type::Array(inner) => Type::array(inner.substitute_parameters(substitute)),
            Type::Nullable(inner) => Type::nullable(inner.substitute_parameters(substitute)),
            Type::Function { params, ret } => Type::function(
                params
                    .iter()
                    .map(|param| FunctionParam {
                        name: param.name.clone(),
                        ty: param.ty.substitute_parameters(substitute),
                        is_content: param.is_content,
                    })
                    .collect(),
                ret.substitute_parameters(substitute),
            ),
            _ => self.clone(),
        }
    }

    /// Replaces each type parameter `substitute` answers for, telling it whether the parameter
    /// stands in a covariant position (a value of the type is produced there) or a contravariant
    /// one (a value is consumed there: a function type's parameter). Nesting a function type's
    /// parameter flips the polarity; its result and every list or nullable layer keep it.
    pub fn substitute_parameters_by_variance(
        &self,
        covariant: bool,
        substitute: &impl Fn(&TypeParameterRef, bool) -> Option<Type>,
    ) -> Type {
        match self {
            Type::Parameter(param) => substitute(param, covariant).unwrap_or_else(|| self.clone()),
            // A type argument is invariant, so it is neither produced nor consumed by the
            // instantiation; it keeps the polarity of the position the applied type stands in.
            Type::Named(named) if !named.args().is_empty() => Type::Named(
                named.with_args(
                    named
                        .args()
                        .iter()
                        .map(|(param, ty)| {
                            (
                                param.clone(),
                                ty.substitute_parameters_by_variance(covariant, substitute),
                            )
                        })
                        .collect(),
                ),
            ),
            Type::Array(inner) => {
                Type::array(inner.substitute_parameters_by_variance(covariant, substitute))
            }
            Type::Nullable(inner) => {
                Type::nullable(inner.substitute_parameters_by_variance(covariant, substitute))
            }
            Type::Function { params, ret } => Type::function(
                params
                    .iter()
                    .map(|param| FunctionParam {
                        name: param.name.clone(),
                        ty: param
                            .ty
                            .substitute_parameters_by_variance(!covariant, substitute),
                        is_content: param.is_content,
                    })
                    .collect(),
                ret.substitute_parameters_by_variance(covariant, substitute),
            ),
            _ => self.clone(),
        }
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
    pub fn function(params: Vec<FunctionParam>, ret: Type) -> Self {
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
    /// The parameters and result of a function type, or `None` for any other type.
    pub fn function_parts(&self) -> Option<(&[FunctionParam], &Type)> {
        match self {
            Type::Function { params, ret } => Some((params.as_slice(), ret.as_ref())),
            _ => None,
        }
    }

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
    /// - The bottom type, which is compatible with every type and which nothing else is compatible
    ///   with
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

        // A type parameter is rigid: exact equality above is the only way to satisfy one, and
        // the only thing it satisfies besides itself is `object`, which the callers that admit the
        // top type handle. It is also the only way a value of parameter type can arise, so no
        // other case below needs to know the variant exists.
        //
        // The bottom type is below every type, so it satisfies every expectation. Nothing is below
        // it, so the relation deliberately does not run the other way.
        //
        // NX carries two compatibility relations — this structural one and the richer
        // `InferenceContext::type_satisfies_expected`, which knows about unions and records and
        // does not delegate here. Both need this case, and `common_supertype` in `semantics.rs`
        // inherits it through this one.
        if matches!(self, Type::Primitive(Primitive::Never)) {
            return true;
        }

        // Numeric widening, one way only: `int32 → int → int64`, `float32 → float64`, and the
        // exact crossings `int32`/`int → float64`. A narrowing is never compatible.
        if let (Type::Primitive(a), Type::Primitive(b)) = (self, other) {
            if a.widens_to(*b) {
                return true;
            }
        }

        // T? is compatible with U? when T is compatible with U: `null` stays `null`, and anything
        // else converts as T to U does.
        if let (Type::Nullable(a), Type::Nullable(b)) = (self, other) {
            if a.is_compatible_with(b) {
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

        // Functions match by parameter name: every parameter the value declares must be one the
        // expected type supplies, contravariantly; the result is covariant. See
        // `check_function_satisfies`.
        if let (
            Type::Function {
                params: actual_params,
                ret: actual_ret,
            },
            Type::Function {
                params: expected_params,
                ret: expected_ret,
            },
        ) = (self, other)
        {
            return check_function_satisfies(
                actual_params,
                actual_ret,
                expected_params,
                expected_ret,
                &mut |value, target| value.is_compatible_with(target),
            )
            .is_ok();
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
                f.write_str(&format_function_type(params, ret, &|ty: &Type| {
                    ty.to_string()
                }))
            }
            Type::Named(named) if named.args().is_empty() => write!(f, "{}", named.name),
            Type::Named(named) => f.write_str(&format_applied_type(
                &named.name,
                named.args(),
                &|ty: &Type| ty.to_string(),
            )),
            Type::Union(union_ty) => write!(f, "{}", union_ty.name),
            Type::UnionCase(case_ty) => write!(f, "{}.{}", case_ty.union, case_ty.case),
            Type::Parameter(param) => write!(f, "{}", param.name),
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
        Type::Named(named) => {
            parts.push((&named.name, named.origin()));
            for (_, arg) in named.args() {
                collect_nominal_parts(arg, parts);
            }
        }
        Type::Union(union_ty) => parts.push((&union_ty.name, union_ty.origin())),
        Type::UnionCase(case_ty) => parts.push((&case_ty.union, case_ty.origin())),
        Type::Array(inner) | Type::Nullable(inner) => collect_nominal_parts(inner, parts),
        Type::Function { params, ret } => {
            for param in params {
                collect_nominal_parts(&param.ty, parts);
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
        Type::Named(named) => {
            let name = match named.origin() {
                Some(origin) => Name::new(&format!("{}:{}", origin.module_identity(), named.name)),
                None => named.name.clone(),
            };
            if named.args().is_empty() {
                name.to_string()
            } else {
                format_applied_type(&name, named.args(), &qualified_display)
            }
        }
        Type::Array(inner) => qualified_postfix_display(inner, "[]"),
        Type::Nullable(inner) => qualified_postfix_display(inner, "?"),
        Type::Function { params, ret } => format_function_type(params, ret, &qualified_display),
        _ => ty.to_string(),
    }
}

/// A suffix after a function type's result would bind to the result, so a function type under a
/// suffix is parenthesized, as source spells it.
fn qualified_postfix_display(inner: &Type, suffix: &str) -> String {
    match inner {
        Type::Function { .. } => format!("({}){suffix}", qualified_display(inner)),
        _ => format!("{}{suffix}", qualified_display(inner)),
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

/// One component type parameter, as a type.
///
/// Equality is what makes the parameter rigid: it is equal only to itself, which is the same
/// name at the same position on the same declaring component.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeParameterRef {
    /// The name, as the signature declares it.
    pub name: Name,
    /// The component whose signature declares it, where the resolving context reached one.
    pub owner: Option<DeclaringOrigin>,
    /// Its position among the component's effective type parameters.
    pub ordinal: usize,
}

/// A nominal type reached by name, with the declaration that name reached.
#[derive(Debug, Clone)]
pub struct NamedType {
    /// The name, as displayed. Two declarations sharing one are still two types.
    pub name: Name,
    /// The declaration this name reached, where the resolving context reached one.
    origin: Option<DeclaringOrigin>,
    /// Type arguments in the declaration's parameter order, empty for a non-generic type.
    ///
    /// <para>Each argument carries the parameter name it binds, because NX names type arguments
    /// and every rendering of an applied type — a mismatch diagnostic, a hover, an explained IR
    /// artifact — spells `<Range T=int/>` rather than a positional form.</para>
    args: Vec<(Name, Type)>,
}

impl NamedType {
    /// Creates a named type for the declaration at `origin`.
    pub fn new(name: Name, origin: Option<DeclaringOrigin>) -> Self {
        Self {
            name,
            origin,
            args: Vec::new(),
        }
    }

    /// Creates one instantiation of the generic declaration at `origin`.
    ///
    /// <para>`args` are in the declaration's parameter order, each paired with the parameter it
    /// binds.</para>
    pub fn applied(name: Name, origin: Option<DeclaringOrigin>, args: Vec<(Name, Type)>) -> Self {
        Self { name, origin, args }
    }

    /// Returns the declaration this name reached, if the building context reached one.
    pub fn origin(&self) -> Option<&DeclaringOrigin> {
        self.origin.as_ref()
    }

    /// Returns the type arguments, in the declaration's parameter order.
    pub fn args(&self) -> &[(Name, Type)] {
        &self.args
    }

    /// Returns the same declaration with `args` as its type arguments.
    pub fn with_args(&self, args: Vec<(Name, Type)>) -> Self {
        Self {
            name: self.name.clone(),
            origin: self.origin.clone(),
            args,
        }
    }

    /// Returns true when both names reached the same declaration, whatever they apply it to.
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
    /// Two applied types are one type exactly when they apply one declaration to equal arguments.
    /// This is what makes an instantiation distinct per argument and invariant: there is no arm
    /// anywhere that widens an argument, because equality never offered one.
    fn eq(&self, other: &Self) -> bool {
        self.is_same_declaration_as(other) && self.args == other.args
    }
}

impl Eq for NamedType {}

impl Hash for NamedType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_declaration(self.origin.as_ref(), &self.name, state);
        self.args.hash(state);
    }
}

/// Spells an applied type as source writes one, with each argument rendered by `render`.
fn format_applied_type(
    name: &Name,
    args: &[(Name, Type)],
    render: &impl Fn(&Type) -> String,
) -> String {
    nx_hir::ast::spell_applied_type(
        name.as_str(),
        args.iter()
            .map(|(param, ty)| (param.as_str(), render(ty)))
            .collect::<Vec<_>>(),
    )
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
        assert_eq!(Type::never().to_string(), "never");
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

        // Cross-category promotion follows the exact crossings into `float64`.
        assert_eq!(
            Primitive::numeric_promotion(Primitive::Int, Primitive::Float64),
            Some(Primitive::Float64)
        );
        assert_eq!(
            Primitive::numeric_promotion(Primitive::Float32, Primitive::Int32),
            Some(Primitive::Float64)
        );
        assert_eq!(
            Primitive::numeric_promotion(Primitive::Float32, Primitive::Float64),
            Some(Primitive::Float64)
        );
        assert_eq!(
            Primitive::numeric_promotion(Primitive::Float32, Primitive::Float32),
            Some(Primitive::Float32)
        );
        // `int64` is lossy in every float, so it has no common type with either.
        assert_eq!(
            Primitive::numeric_promotion(Primitive::Int64, Primitive::Float64),
            None
        );
        assert_eq!(
            Primitive::numeric_promotion(Primitive::Float32, Primitive::Int64),
            None
        );
        assert_eq!(
            Primitive::numeric_promotion(Primitive::String, Primitive::Int),
            None
        );
    }

    #[test]
    fn test_widens_to_admits_exactly_the_six_widenings() {
        use Primitive::*;
        for (from, to) in [
            (Int32, Int),
            (Int32, Int64),
            (Int, Int64),
            (Float32, Float64),
            (Int32, Float64),
            (Int, Float64),
        ] {
            assert!(from.widens_to(to), "{from} should widen to {to}");
            assert!(
                Type::Primitive(from).is_compatible_with(&Type::Primitive(to)),
                "{from} should be compatible with {to}"
            );
        }
        for p in [Int32, Int, Int64, Float32, Float64, String, Boolean] {
            assert!(p.widens_to(p), "{p} should widen to itself");
        }
    }

    #[test]
    fn test_widens_to_rejects_every_narrowing_and_lossy_crossing() {
        use Primitive::*;
        for (from, to) in [
            (Int64, Int),
            (Int64, Int32),
            (Int, Int32),
            (Float64, Float32),
            (Int64, Float64),
            (Int, Float32),
            (Int32, Float32),
            (Int64, Float32),
            (Float32, Int),
            (Float64, Int),
            (String, Int),
            (Boolean, Int32),
        ] {
            assert!(!from.widens_to(to), "{from} should not widen to {to}");
            assert!(
                !Type::Primitive(from).is_compatible_with(&Type::Primitive(to)),
                "{from} should not be compatible with {to}"
            );
        }
    }

    #[test]
    fn test_represents_integer_exactly_at_the_float64_boundary() {
        let exact = 1i64 << 53;
        assert!(Primitive::Float64.represents_integer_exactly(exact));
        assert!(!Primitive::Float64.represents_integer_exactly(exact + 1));
        assert!(Primitive::Float64.represents_integer_exactly(-exact));
        assert!(!Primitive::Float64.represents_integer_exactly(-exact - 1));
    }

    #[test]
    fn test_represents_integer_exactly_at_the_float32_boundary() {
        let exact = 1i64 << 24;
        assert!(Primitive::Float32.represents_integer_exactly(exact));
        assert!(!Primitive::Float32.represents_integer_exactly(exact + 1));
        assert!(Primitive::Float32.represents_integer_exactly(-exact));
        assert!(!Primitive::Float32.represents_integer_exactly(-exact - 1));
    }

    #[test]
    fn test_represents_integer_exactly_does_not_saturate_at_the_i64_extremes() {
        // The trap this guards: `i64::MAX as f64` rounds up to 2^63, and casting that back to i64
        // saturates to i64::MAX again, which would look like a lossless round trip.
        assert!(!Primitive::Float64.represents_integer_exactly(i64::MAX));
        assert!(!Primitive::Float32.represents_integer_exactly(i64::MAX));
        // i64::MIN is a power of two, so it genuinely is exact.
        assert!(Primitive::Float64.represents_integer_exactly(i64::MIN));
        assert!(Primitive::Float32.represents_integer_exactly(i64::MIN));
    }

    #[test]
    fn test_represents_integer_exactly_is_false_for_non_float_primitives() {
        assert!(!Primitive::Int.represents_integer_exactly(1));
        assert!(!Primitive::Int32.represents_integer_exactly(1));
        assert!(!Primitive::Int64.represents_integer_exactly(1));
        assert!(!Primitive::String.represents_integer_exactly(1));
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
        let func = Type::function(
            vec![
                FunctionParam::new("Count", Type::int()),
                FunctionParam::content("Children", Type::array(Type::string())),
            ],
            Type::boolean(),
        );
        assert_eq!(
            func.to_string(),
            "<function Count:int content Children:string[] />: boolean"
        );
        assert_eq!(
            Type::function(vec![], Type::named("DrawnNode")).to_string(),
            "<function />: DrawnNode"
        );
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

    fn function(params: &[(&str, Type)], ret: Type) -> Type {
        Type::function(
            params
                .iter()
                .map(|(name, ty)| FunctionParam::new(*name, ty.clone()))
                .collect(),
            ret,
        )
    }

    #[test]
    fn test_is_compatible_functions() {
        let f1 = function(&[("n", Type::int())], Type::string());
        let f2 = function(&[("n", Type::int())], Type::string());
        let f3 = function(&[("n", Type::string())], Type::string());

        assert!(f1.is_compatible_with(&f2));
        assert!(!f1.is_compatible_with(&f3));
    }

    #[test]
    fn a_function_may_declare_fewer_parameters_than_its_type_but_not_more() {
        let full = function(
            &[("Item", Type::named("Contact")), ("Index", Type::int())],
            Type::string(),
        );
        let compact = function(&[("Item", Type::named("Contact"))], Type::string());
        assert!(
            compact.is_compatible_with(&full),
            "ignoring Index is allowed"
        );
        assert!(
            !full.is_compatible_with(&compact),
            "needing Index where none is supplied is not"
        );
        let mismatch = check_function_satisfies(
            full.function_parts().unwrap().0,
            full.function_parts().unwrap().1,
            compact.function_parts().unwrap().0,
            compact.function_parts().unwrap().1,
            &mut |value, target| value.is_compatible_with(target),
        )
        .unwrap_err();
        assert_eq!(
            mismatch,
            FunctionMismatch::UnsuppliedParameter(Name::new("Index"))
        );
        assert!(mismatch.to_string().contains("'Index'"), "{mismatch}");
    }

    #[test]
    fn function_parameters_match_by_name_not_position() {
        let declared = function(
            &[("Index", Type::int()), ("Item", Type::named("object"))],
            Type::string(),
        );
        let expected = function(
            &[("Item", Type::named("object")), ("Index", Type::int())],
            Type::string(),
        );
        assert!(declared.is_compatible_with(&expected));

        let renamed = function(&[("Entry", Type::named("object"))], Type::string());
        assert!(!renamed.is_compatible_with(&expected));
    }

    #[test]
    fn function_parameters_are_contravariant_and_results_covariant() {
        let takes_object = function(&[("Value", Type::named("object"))], Type::string());
        let takes_int = function(&[("Value", Type::int())], Type::string());
        // `object` is not below `int`, and this structural relation does not know `int` is below
        // `object`; the checker's richer relation does. Contravariance shows through widening.
        let takes_int32 = function(&[("Value", Type::int32())], Type::string());
        assert!(
            takes_int.is_compatible_with(&takes_int32),
            "a function taking int accepts the int32 the type supplies"
        );
        assert!(!takes_int32.is_compatible_with(&takes_int));
        assert!(!takes_object.is_compatible_with(&takes_int));

        let returns_int32 = function(&[], Type::int32());
        let returns_int = function(&[], Type::int());
        assert!(returns_int32.is_compatible_with(&returns_int));
        assert!(!returns_int.is_compatible_with(&returns_int32));
    }

    #[test]
    fn a_content_parameter_pairs_only_with_a_content_parameter() {
        let content = Type::function(
            vec![FunctionParam::content(
                "Children",
                Type::array(Type::string()),
            )],
            Type::string(),
        );
        let plain = function(&[("Children", Type::array(Type::string()))], Type::string());
        assert!(content.is_compatible_with(&content));
        assert!(!content.is_compatible_with(&plain));
        assert!(!plain.is_compatible_with(&content));
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
            function(&[("a", Type::int()), ("b", Type::int())], Type::int()).to_string(),
            "<function a:int b:int />: int"
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

        let func_array = Type::array(function(&[("n", Type::int())], Type::string()));
        assert_eq!(func_array.to_string(), "(<function n:int />: string)[]");

        let nullable_func = Type::nullable(function(&[("n", Type::int())], Type::string()));
        assert_eq!(nullable_func.to_string(), "(<function n:int />: string)?");

        // A suffix on the result is written on the result, so no parentheses appear.
        let returns_nullable = function(&[("n", Type::int())], Type::nullable(Type::string()));
        assert_eq!(returns_nullable.to_string(), "<function n:int />: string?");
        assert!(!nullable_func.to_string().contains("=>"));
    }

    #[test]
    fn a_type_parameter_is_rigid() {
        let param = Type::parameter("TItem", None, 0);
        let other = Type::parameter("TOther", None, 1);

        assert!(param.is_compatible_with(&param));
        assert!(Type::never().is_compatible_with(&param));
        assert!(crate::type_satisfies_expected(
            &param,
            &Type::named("object")
        ));
        assert!(!Type::string().is_compatible_with(&param));
        assert!(!param.is_compatible_with(&Type::string()));
        assert!(!param.is_compatible_with(&other));
        assert!(!Type::named("TItem").is_compatible_with(&param));

        // Composes under `?` and `[]` like any other type.
        assert!(param.is_compatible_with(&Type::nullable(param.clone())));
        assert!(Type::array(Type::never()).is_compatible_with(&Type::array(param.clone())));
        assert!(!Type::array(Type::int()).is_compatible_with(&Type::array(param.clone())));

        // Joining with anything else is a mismatch: the join is the top type.
        assert_eq!(
            crate::common_supertype(&param, &Type::string()),
            Type::named("object")
        );
        assert_eq!(crate::common_supertype(&param, &param), param);
    }

    #[test]
    fn a_type_parameter_renders_as_its_name_and_substitutes_under_wrappers() {
        let param = Type::parameter("TItem", None, 0);
        let ty = Type::nullable(Type::array(param.clone()));
        assert_eq!(ty.to_string(), "TItem[]?");

        let found = ty.find_parameter(&|candidate| candidate.name.as_str() == "TItem");
        assert_eq!(found.map(|candidate| candidate.ordinal), Some(0));
        assert!(ty
            .find_parameter(&|candidate| candidate.name.as_str() == "TOther")
            .is_none());

        let substituted = ty.substitute_parameters(&|candidate| {
            (candidate.name.as_str() == "TItem").then(Type::string)
        });
        assert_eq!(substituted, Type::nullable(Type::array(Type::string())));
        let untouched = ty.substitute_parameters(&|_| None);
        assert_eq!(untouched, ty);
    }

    #[test]
    fn two_instantiations_of_one_record_are_different_types() {
        let range = |arg: Type| {
            Type::Named(NamedType::applied(
                Name::new("Range"),
                None,
                vec![(Name::new("T"), arg)],
            ))
        };
        let ints = range(Type::int());
        let strings = range(Type::string());

        assert_eq!(ints, range(Type::int()));
        assert_ne!(ints, strings);
        assert!(!ints.is_compatible_with(&strings));
        assert!(!strings.is_compatible_with(&ints));
        // Neither is the argument-less type, which is what makes a bare `Range` unusable.
        assert_ne!(ints, Type::named("Range"));

        assert_eq!(ints.to_string(), "<Range T=int/>");
        assert_eq!(
            Type::nullable(Type::array(ints.clone())).to_string(),
            "<Range T=int/>[]?"
        );
    }

    #[test]
    fn a_type_argument_substitutes_and_is_found_inside_an_instantiation() {
        let param = Type::parameter("TItem", None, 0);
        let ty = Type::array(Type::Named(NamedType::applied(
            Name::new("Range"),
            None,
            vec![(Name::new("T"), param)],
        )));

        let found = ty.find_parameter(&|candidate| candidate.name.as_str() == "TItem");
        assert_eq!(found.map(|candidate| candidate.ordinal), Some(0));

        let substituted = ty.substitute_parameters(&|candidate| {
            (candidate.name.as_str() == "TItem").then(Type::int)
        });
        assert_eq!(
            substituted,
            Type::array(Type::Named(NamedType::applied(
                Name::new("Range"),
                None,
                vec![(Name::new("T"), Type::int())]
            )))
        );
    }
}
