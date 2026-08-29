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

pub fn is_object_type(ty: &Type) -> bool {
    matches!(ty, Type::Named(name) if name.as_str() == "object")
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

fn builtin_type(name: &Name) -> Option<Type> {
    match name.as_str() {
        "string" => Some(Type::string()),
        "int" => Some(Type::int()),
        "int32" => Some(Type::int32()),
        "int64" => Some(Type::int64()),
        "float32" => Some(Type::float32()),
        "float64" => Some(Type::float64()),
        "boolean" => Some(Type::boolean()),
        "void" => Some(Type::void()),
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
        assert_eq!(builtin_type(&Name::new("void")), Some(Type::void()));
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
