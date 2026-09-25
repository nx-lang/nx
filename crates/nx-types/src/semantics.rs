use crate::{FunctionParam, Primitive, Type};
use nx_hir::{ast, Name};
use rustc_hash::FxHashSet;

/// The structural join of two types: the least type above both.
///
/// <para>The item types join by the rules below — equality, numeric promotion, one satisfying the
/// other, else `object` — and the occurrences join by the lattice, so `int?` with `int+` is
/// `int*` and `string?` with `float64` is `object?`. The bottom item type is the identity, which
/// is what makes `{}` joined with `T` a `T?`: the empty type contributes its `?` and no item.
/// The inference context overrides this with a join that knows about records and unions, and
/// splits the occurrence off the same way.</para>
pub fn common_supertype(lhs: &Type, rhs: &Type) -> Type {
    if lhs.is_error() || rhs.is_error() {
        return Type::Error;
    }

    if lhs == rhs {
        return lhs.clone();
    }

    let (lhs_item, lhs_occ) = lhs.split();
    let (rhs_item, rhs_occ) = rhs.split();
    let item = common_item_supertype(lhs_item, rhs_item);
    Type::seq(item, lhs_occ.join(rhs_occ))
}

/// The join of two item types, with the bottom type as the identity.
pub(crate) fn common_item_supertype(lhs: &Type, rhs: &Type) -> Type {
    if lhs == rhs {
        return lhs.clone();
    }

    if let Type::Primitive(Primitive::Never) = lhs {
        return rhs.clone();
    }
    if let Type::Primitive(Primitive::Never) = rhs {
        return lhs.clone();
    }

    if let (Type::Primitive(a), Type::Primitive(b)) = (lhs, rhs) {
        if let Some(promoted) = Primitive::numeric_promotion(*a, *b) {
            return Type::Primitive(promoted);
        }
    }

    if type_satisfies_expected(lhs, rhs) {
        return rhs.clone();
    }

    if type_satisfies_expected(rhs, lhs) {
        return lhs.clone();
    }

    Type::named("object")
}

pub fn is_object_type(ty: &Type) -> bool {
    matches!(ty, Type::Named(named) if named.name.as_str() == "object")
}

/// The numeric primitive a numeric literal written at this site would take, if any.
///
/// <para>`None` for every other expected type, and that breadth is the point: `object` accepts any
/// value, and an unresolved type variable has not decided what it accepts yet. Converting a literal
/// on either basis would change the value a host receives on the strength of an expectation that
/// was never a numeric one.</para>
///
/// <para>A suffixed site answers with its item type because a scalar binds there by the one-level
/// lift, so the item type is the expectation a literal written at that site actually meets.</para>
pub fn numeric_literal_target(expected: &Type) -> Option<Primitive> {
    match expected.item() {
        Type::Primitive(primitive) if primitive.is_numeric() => Some(*primitive),
        _ => None,
    }
}

/// Whether a value of type `actual` satisfies a site of type `expected`.
///
/// <para>`object` is the top item type: it admits every item, under the occurrence the site
/// declares. A sequence does not satisfy a plain `object` site, since `object` is exactly one
/// value; it satisfies `object*`.</para>
pub fn type_satisfies_expected(actual: &Type, expected: &Type) -> bool {
    actual.is_compatible_with(expected)
        || (is_object_type(expected.item()) && actual.occurrence().satisfies(expected.occurrence()))
}

pub fn resolve_type_ref_with<F>(type_ref: &ast::TypeRef, resolve_named: &mut F) -> Type
where
    F: FnMut(&Name, &mut FxHashSet<Name>) -> Type,
{
    let mut seen = FxHashSet::default();
    resolve_type_ref_with_seen(type_ref, &mut seen, resolve_named)
}

pub fn resolve_type_ref_with_seen<F>(
    type_ref: &ast::TypeRef,
    seen: &mut FxHashSet<Name>,
    resolve_named: &mut F,
) -> Type
where
    F: FnMut(&Name, &mut FxHashSet<Name>) -> Type,
{
    match type_ref {
        ast::TypeRef::Name(name) => builtin_type(name).unwrap_or_else(|| resolve_named(name, seen)),
        // An applied type reads as its record here: this walk is the one below the type checker,
        // where type arguments are erased. The checker resolves the arguments itself, in
        // `InferenceContext`, because only it knows the record's parameters.
        ast::TypeRef::Applied { name, .. } => {
            builtin_type(name).unwrap_or_else(|| resolve_named(name, seen))
        }
        ast::TypeRef::Seq { inner, occ } => {
            Type::seq(resolve_type_ref_with_seen(inner, seen, resolve_named), *occ)
        }
        ast::TypeRef::Function {
            params,
            return_type,
        } => {
            let params = params
                .iter()
                .map(|param| FunctionParam {
                    name: param.name.clone(),
                    ty: resolve_type_ref_with_seen(&param.ty, seen, resolve_named),
                    is_content: param.is_content,
                    optional: param.optional,
                })
                .collect();
            let ret = resolve_type_ref_with_seen(return_type, seen, resolve_named);
            Type::function(params, ret)
        }
    }
}

/// The eight primitive type names NX source can write.
///
/// <para>`object` is the top type and is a `Named` type rather than a `Primitive`, so
/// [`builtin_type`] does not answer for it. A resolver deciding whether a bare name denotes a
/// type, or listing the type names an author could have meant, consults this list so that the
/// two cannot disagree about `object`.</para>
pub(crate) use nx_syntax::PRIMITIVE_TYPE_NAMES;

pub(crate) fn builtin_type(name: &Name) -> Option<Type> {
    match name.as_str() {
        "string" => Some(Type::string()),
        "int" => Some(Type::int()),
        "int32" => Some(Type::int32()),
        "int64" => Some(Type::int64()),
        "float32" => Some(Type::float32()),
        "float64" => Some(Type::float64()),
        "boolean" => Some(Type::boolean()),
        // `void` is deliberately absent. The unit type is inference-internal: it is what an `if`
        // with no `else` takes, and nothing in NX source needs to name it.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_supertype_promotes_integer_widths() {
        assert_eq!(
            common_supertype(&Type::int32(), &Type::int64()),
            Type::int64()
        );
        assert_eq!(
            common_supertype(&Type::float32(), &Type::float64()),
            Type::float64()
        );
    }

    #[test]
    fn test_common_supertype_promotes_sequence_items() {
        assert_eq!(
            common_supertype(
                &Type::one_or_more(Type::int32()),
                &Type::one_or_more(Type::int64())
            ),
            Type::one_or_more(Type::int64())
        );
        assert_eq!(
            common_supertype(
                &Type::zero_or_more(Type::float32()),
                &Type::one_or_more(Type::float64())
            ),
            Type::zero_or_more(Type::float64())
        );
    }

    #[test]
    fn test_common_supertype_joins_occurrences_by_the_lattice() {
        // `{}` joined with `T` is `T?`: the empty type contributes `?` and no item.
        assert_eq!(
            common_supertype(&Type::empty(), &Type::float64()),
            Type::optional(Type::float64())
        );
        assert_eq!(
            common_supertype(&Type::optional(Type::int()), &Type::float64()),
            Type::optional(Type::float64())
        );
        assert_eq!(
            common_supertype(&Type::int(), &Type::one_or_more(Type::int())),
            Type::one_or_more(Type::int())
        );
        assert_eq!(
            common_supertype(
                &Type::optional(Type::int()),
                &Type::one_or_more(Type::int())
            ),
            Type::zero_or_more(Type::int())
        );
        // A join with no common item type still joins its occurrence.
        assert_eq!(
            common_supertype(&Type::optional(Type::string()), &Type::float64()),
            Type::optional(Type::named("object"))
        );
    }

    #[test]
    fn test_type_satisfies_expected_lifts_a_scalar_to_a_sequence() {
        assert!(type_satisfies_expected(
            &Type::int(),
            &Type::one_or_more(Type::int())
        ));
        assert!(type_satisfies_expected(
            &Type::int(),
            &Type::zero_or_more(Type::int())
        ));
        assert!(type_satisfies_expected(
            &Type::one_or_more(Type::int()),
            &Type::zero_or_more(Type::int())
        ));
    }

    #[test]
    fn test_type_satisfies_expected_follows_the_lattice_downward_never() {
        assert!(!type_satisfies_expected(
            &Type::zero_or_more(Type::int()),
            &Type::one_or_more(Type::int())
        ));
        assert!(!type_satisfies_expected(
            &Type::one_or_more(Type::int()),
            &Type::optional(Type::int())
        ));
        assert!(!type_satisfies_expected(
            &Type::optional(Type::int()),
            &Type::int()
        ));
        assert!(!type_satisfies_expected(
            &Type::one_or_more(Type::int()),
            &Type::int()
        ));
    }

    #[test]
    fn test_object_is_the_top_item_type_under_the_site_occurrence() {
        assert!(type_satisfies_expected(
            &Type::int(),
            &Type::named("object")
        ));
        assert!(type_satisfies_expected(
            &Type::one_or_more(Type::int()),
            &Type::zero_or_more(Type::named("object"))
        ));
        assert!(!type_satisfies_expected(
            &Type::one_or_more(Type::int()),
            &Type::named("object")
        ));
        assert!(!type_satisfies_expected(
            &Type::empty(),
            &Type::named("object")
        ));
    }

    #[test]
    fn test_common_supertype_follows_the_integer_rank_order() {
        assert_eq!(common_supertype(&Type::int32(), &Type::int()), Type::int());
        assert_eq!(common_supertype(&Type::int(), &Type::int32()), Type::int());
        assert_eq!(
            common_supertype(&Type::int(), &Type::int64()),
            Type::int64()
        );
        assert_eq!(
            common_supertype(&Type::int64(), &Type::int()),
            Type::int64()
        );
        assert_eq!(
            common_supertype(
                &Type::one_or_more(Type::int32()),
                &Type::one_or_more(Type::int())
            ),
            Type::one_or_more(Type::int())
        );
    }

    #[test]
    fn test_former_spellings_are_not_builtin_types() {
        for name in ["i32", "i64", "f32", "f64", "float", "bool"] {
            assert_eq!(
                builtin_type(&Name::new(name)),
                None,
                "'{}' must no longer resolve to a primitive type",
                name
            );
        }
    }

    #[test]
    fn test_canonical_names_are_builtin_types() {
        assert_eq!(builtin_type(&Name::new("int")), Some(Type::int()));
        assert_eq!(builtin_type(&Name::new("int32")), Some(Type::int32()));
        assert_eq!(builtin_type(&Name::new("int64")), Some(Type::int64()));
        assert_eq!(builtin_type(&Name::new("float32")), Some(Type::float32()));
        assert_eq!(builtin_type(&Name::new("float64")), Some(Type::float64()));
        assert_eq!(builtin_type(&Name::new("boolean")), Some(Type::boolean()));
        assert_eq!(builtin_type(&Name::new("string")), Some(Type::string()));
    }

    #[test]
    fn test_void_is_not_a_builtin_type() {
        // There is no unit type any more: `void` is a name a declaration may take, and nothing in
        // the checker renders a type as `void`.
        assert_eq!(builtin_type(&Name::new("void")), None);
    }

    #[test]
    fn test_capitalized_spellings_are_not_builtin_types() {
        for name in [
            "String", "Int", "INT", "INT64", "Int64", "Boolean", "Float64", "Void",
        ] {
            assert_eq!(
                builtin_type(&Name::new(name)),
                None,
                "'{}' must not resolve to a primitive type; primitive names are case-sensitive",
                name
            );
        }
    }

    #[test]
    fn test_object_is_matched_case_sensitively() {
        assert!(is_object_type(&Type::named("object")));
        assert!(!is_object_type(&Type::named("Object")));
        assert!(!is_object_type(&Type::named("OBJECT")));
    }

    #[test]
    fn test_numeric_literal_target_finds_each_numeric_width() {
        for (ty, primitive) in [
            (Type::int(), Primitive::Int),
            (Type::int32(), Primitive::Int32),
            (Type::int64(), Primitive::Int64),
            (Type::float32(), Primitive::Float32),
            (Type::float64(), Primitive::Float64),
        ] {
            assert_eq!(numeric_literal_target(&ty), Some(primitive));
        }
    }

    #[test]
    fn test_numeric_literal_target_sees_through_an_occurrence() {
        assert_eq!(
            numeric_literal_target(&Type::optional(Type::float64())),
            Some(Primitive::Float64)
        );
        assert_eq!(
            numeric_literal_target(&Type::one_or_more(Type::int32())),
            Some(Primitive::Int32)
        );
        assert_eq!(
            numeric_literal_target(&Type::zero_or_more(Type::float64())),
            Some(Primitive::Float64)
        );
    }

    #[test]
    fn test_numeric_literal_target_declines_every_non_numeric_expectation() {
        // `object` accepts anything and a type variable has not decided yet; converting on either
        // basis would change the value on the strength of an expectation nobody made.
        assert_eq!(numeric_literal_target(&Type::named("object")), None);
        assert_eq!(numeric_literal_target(&Type::Variable(0)), None);
        assert_eq!(numeric_literal_target(&Type::string()), None);
        assert_eq!(numeric_literal_target(&Type::boolean()), None);
        assert_eq!(numeric_literal_target(&Type::named("Thickness")), None);
        assert_eq!(
            numeric_literal_target(&Type::one_or_more(Type::string())),
            None
        );
    }

    #[test]
    fn test_resolve_type_ref_with_uses_builtin_and_callback_resolution() {
        let type_ref = ast::TypeRef::function(
            vec![
                ast::FunctionParam::new("Label", ast::TypeRef::name("string")),
                ast::FunctionParam::content(
                    "Items",
                    ast::TypeRef::one_or_more(ast::TypeRef::name("Custom")),
                ),
            ],
            ast::TypeRef::optional(ast::TypeRef::name("boolean")),
        );

        let resolved =
            resolve_type_ref_with(&type_ref, &mut |name, _seen| Type::named(name.clone()));

        assert_eq!(
            resolved,
            Type::function(
                vec![
                    FunctionParam::new("Label", Type::string()),
                    FunctionParam::content("Items", Type::one_or_more(Type::named("Custom"))),
                ],
                Type::optional(Type::boolean())
            )
        );
    }
}
