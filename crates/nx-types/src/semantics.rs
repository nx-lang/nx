use crate::{Primitive, Type};
use nx_hir::{ast, Name};
use rustc_hash::FxHashSet;

pub fn common_supertype(lhs: &Type, rhs: &Type) -> Type {
    if lhs.is_error() || rhs.is_error() {
        return Type::Error;
    }

    if lhs == rhs {
        return lhs.clone();
    }

    if let Some(joined) = nullable_join(lhs, rhs, common_supertype) {
        return joined;
    }

    if let (Type::Primitive(a), Type::Primitive(b)) = (lhs, rhs) {
        if let Some(promoted) = Primitive::numeric_promotion(*a, *b) {
            return Type::Primitive(promoted);
        }
    }

    if let (Type::Array(lhs_inner), Type::Array(rhs_inner)) = (lhs, rhs) {
        return Type::array(common_supertype(lhs_inner, rhs_inner));
    }

    if type_satisfies_expected(lhs, rhs) {
        return rhs.clone();
    }

    if type_satisfies_expected(rhs, lhs) {
        return lhs.clone();
    }

    Type::named("object")
}

/// The join of two types when either is nullable, or `None` when neither is.
///
/// <para>Nullability is lifted out of the join: `A?` with `B`, or with `B?`, is the join of `A`
/// and `B`, made nullable. The `null` literal is typed `T?` for a `T` nothing has decided, so it
/// adds nullability and nothing else: `null` with `float64` is `float64?`. A join that climbs to
/// `object` stays `object`, which already admits `null`. `join` is the join the caller applies to
/// the inner types, so a join that knows about records and unions keeps knowing inside `?`.</para>
pub(crate) fn nullable_join(
    lhs: &Type,
    rhs: &Type,
    mut join: impl FnMut(&Type, &Type) -> Type,
) -> Option<Type> {
    let joined = match (lhs, rhs) {
        (Type::Nullable(inner), other) | (other, Type::Nullable(inner)) if inner.is_variable() => {
            other.clone()
        }
        (Type::Nullable(lhs), Type::Nullable(rhs)) => join(lhs, rhs),
        (Type::Nullable(inner), other) | (other, Type::Nullable(inner)) => join(inner, other),
        _ => return None,
    };
    Some(match joined {
        Type::Nullable(_) | Type::Error => joined,
        joined if is_object_type(&joined) => joined,
        joined => Type::nullable(joined),
    })
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
/// <para>A list-typed site answers with its element type because a scalar binds there by coercion,
/// so the element type is the expectation a literal written at that site actually meets.</para>
pub fn numeric_literal_target(expected: &Type) -> Option<Primitive> {
    match expected.strip_nullable() {
        Type::Primitive(primitive) if primitive.is_numeric() => Some(*primitive),
        Type::Array(element) => numeric_literal_target(element),
        _ => None,
    }
}

pub fn type_satisfies_expected(actual: &Type, expected: &Type) -> bool {
    actual.is_compatible_with(expected) || is_object_type(expected)
}

pub fn type_satisfies_expected_with_coercion(actual: &Type, expected: &Type) -> bool {
    if type_satisfies_expected(actual, expected) {
        return true;
    }

    let coercion_target = expected.strip_nullable();

    match (actual, coercion_target) {
        (Type::Array(actual_inner), Type::Array(expected_inner)) => {
            type_satisfies_expected(actual_inner, expected_inner)
        }
        (Type::Array(_), _) if is_object_type(coercion_target) => true,
        (Type::Array(_), _) => false,
        (_, Type::Array(expected_inner)) => type_satisfies_expected(actual, expected_inner),
        _ => false,
    }
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
        ast::TypeRef::Array(inner) => {
            Type::array(resolve_type_ref_with_seen(inner, seen, resolve_named))
        }
        ast::TypeRef::Nullable(inner) => {
            Type::nullable(resolve_type_ref_with_seen(inner, seen, resolve_named))
        }
        ast::TypeRef::Function {
            params,
            return_type,
        } => {
            let params = params
                .iter()
                .map(|param| resolve_type_ref_with_seen(param, seen, resolve_named))
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
    fn test_common_supertype_promotes_nested_array_items() {
        assert_eq!(
            common_supertype(&Type::array(Type::int32()), &Type::array(Type::int64())),
            Type::array(Type::int64())
        );
        assert_eq!(
            common_supertype(&Type::array(Type::float32()), &Type::array(Type::float64())),
            Type::array(Type::float64())
        );
    }

    #[test]
    fn test_common_supertype_lifts_nullability_out_of_the_join() {
        let null = Type::nullable(Type::var(0));
        assert_eq!(
            common_supertype(&null, &Type::float64()),
            Type::nullable(Type::float64())
        );
        assert_eq!(
            common_supertype(&Type::nullable(Type::int()), &Type::float64()),
            Type::nullable(Type::float64())
        );
        assert_eq!(
            common_supertype(&Type::nullable(Type::string()), &Type::float64()),
            Type::named("object")
        );
    }

    #[test]
    fn test_type_satisfies_expected_with_coercion_allows_scalar_to_list() {
        assert!(type_satisfies_expected_with_coercion(
            &Type::int(),
            &Type::array(Type::int())
        ));
    }

    #[test]
    fn test_type_satisfies_expected_with_coercion_allows_array_to_nullable_array() {
        assert!(type_satisfies_expected_with_coercion(
            &Type::array(Type::int()),
            &Type::nullable(Type::array(Type::int()))
        ));
    }

    #[test]
    fn test_type_satisfies_expected_with_coercion_allows_scalar_to_nullable_array() {
        assert!(type_satisfies_expected_with_coercion(
            &Type::int(),
            &Type::nullable(Type::array(Type::int()))
        ));
    }

    #[test]
    fn test_type_satisfies_expected_with_coercion_rejects_nullable_array_to_array() {
        assert!(!type_satisfies_expected_with_coercion(
            &Type::nullable(Type::array(Type::int())),
            &Type::array(Type::int())
        ));
    }

    #[test]
    fn test_type_satisfies_expected_with_coercion_rejects_nullable_items_for_nullable_array() {
        assert!(!type_satisfies_expected_with_coercion(
            &Type::array(Type::nullable(Type::int())),
            &Type::nullable(Type::array(Type::int()))
        ));
    }

    #[test]
    fn test_type_satisfies_expected_with_coercion_rejects_list_to_scalar() {
        assert!(!type_satisfies_expected_with_coercion(
            &Type::array(Type::int()),
            &Type::int()
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
            common_supertype(&Type::array(Type::int32()), &Type::array(Type::int())),
            Type::array(Type::int())
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
        // The unit type still exists and still renders as `void`; it is only unspellable.
        assert_eq!(builtin_type(&Name::new("void")), None);
        assert_eq!(Type::void().to_string(), "void");
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
    fn test_numeric_literal_target_sees_through_nullable_and_list() {
        assert_eq!(
            numeric_literal_target(&Type::nullable(Type::float64())),
            Some(Primitive::Float64)
        );
        assert_eq!(
            numeric_literal_target(&Type::array(Type::int32())),
            Some(Primitive::Int32)
        );
        assert_eq!(
            numeric_literal_target(&Type::nullable(Type::array(
                Type::nullable(Type::float64())
            ))),
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
        assert_eq!(numeric_literal_target(&Type::array(Type::string())), None);
    }

    #[test]
    fn test_resolve_type_ref_with_uses_builtin_and_callback_resolution() {
        let type_ref = ast::TypeRef::function(
            vec![
                ast::TypeRef::name("string"),
                ast::TypeRef::array(ast::TypeRef::name("Custom")),
            ],
            ast::TypeRef::nullable(ast::TypeRef::name("boolean")),
        );

        let resolved =
            resolve_type_ref_with(&type_ref, &mut |name, _seen| Type::named(name.clone()));

        assert_eq!(
            resolved,
            Type::function(
                vec![Type::string(), Type::array(Type::named("Custom"))],
                Type::nullable(Type::boolean())
            )
        );
    }
}
