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
    matches!(ty, Type::Named(name) if name.as_str().eq_ignore_ascii_case("object"))
}

pub fn type_satisfies_expected(actual: &Type, expected: &Type) -> bool {
    actual.is_compatible_with(expected) || is_object_type(expected)
}

pub fn type_satisfies_expected_with_coercion(actual: &Type, expected: &Type) -> bool {
    match (actual, expected) {
        (Type::Array(actual_inner), Type::Array(expected_inner)) => {
            type_satisfies_expected(actual_inner, expected_inner)
        }
        (Type::Array(_), _) if is_object_type(expected) => true,
        (Type::Array(_), _) => false,
        (_, Type::Array(expected_inner)) => type_satisfies_expected(actual, expected_inner),
        _ => type_satisfies_expected(actual, expected),
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
    match name.as_str().to_ascii_lowercase().as_str() {
        "string" => Some(Type::string()),
        "i32" => Some(Type::i32()),
        "i64" => Some(Type::i64()),
        "int" => Some(Type::int()),
        "f32" => Some(Type::f32()),
        "f64" => Some(Type::f64()),
        "float" => Some(Type::float()),
        "bool" => Some(Type::bool()),
        "void" => Some(Type::void()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_supertype_promotes_integer_widths() {
        assert_eq!(common_supertype(&Type::i32(), &Type::int()), Type::int());
        assert_eq!(
            common_supertype(&Type::f32(), &Type::float()),
            Type::float()
        );
    }

    #[test]
    fn test_common_supertype_promotes_nested_array_items() {
        assert_eq!(
            common_supertype(&Type::array(Type::i32()), &Type::array(Type::int())),
            Type::array(Type::int())
        );
        assert_eq!(
            common_supertype(&Type::array(Type::f32()), &Type::array(Type::float())),
            Type::array(Type::float())
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
    fn test_type_satisfies_expected_with_coercion_rejects_list_to_scalar() {
        assert!(!type_satisfies_expected_with_coercion(
            &Type::array(Type::int()),
            &Type::int()
        ));
    }

    #[test]
    fn test_builtin_type_is_case_insensitive() {
        assert_eq!(builtin_type(&Name::new("String")), Some(Type::string()));
        assert_eq!(builtin_type(&Name::new("INT")), Some(Type::int()));
        assert_eq!(builtin_type(&Name::new("Bool")), Some(Type::bool()));
    }

    #[test]
    fn test_resolve_type_ref_with_uses_builtin_and_callback_resolution() {
        let type_ref = ast::TypeRef::function(
            vec![
                ast::TypeRef::name("String"),
                ast::TypeRef::array(ast::TypeRef::name("Custom")),
            ],
            ast::TypeRef::nullable(ast::TypeRef::name("BOOL")),
        );

        let resolved =
            resolve_type_ref_with(&type_ref, &mut |name, _seen| Type::named(name.clone()));

        assert_eq!(
            resolved,
            Type::function(
                vec![Type::string(), Type::array(Type::named("Custom"))],
                Type::nullable(Type::bool())
            )
        );
    }
}
