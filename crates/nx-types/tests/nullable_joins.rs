//! How a join types branches when one of them is nullable or `null`.
//!
//! Nullability is lifted out of the join: `A?` with `B` is the join of `A` and `B`, made nullable,
//! and `null` adds nullability and nothing else. A join that climbs to `object` stays `object`.

use nx_hir::Item;
use nx_types::{check_str, Type};

/// The type of `f`'s body in `source`, which must check cleanly.
fn body_type(source: &str) -> Type {
    let checked = check_str(source, "test.nx");
    let messages: Vec<_> = checked
        .errors()
        .iter()
        .map(|diagnostic| diagnostic.message().to_string())
        .collect();
    assert!(messages.is_empty(), "`{source}` reported {messages:?}");
    let module = checked.lowered_module.as_ref().expect("lowered module");
    let body = module
        .items()
        .iter()
        .find_map(|item| match item {
            Item::Function(function) if function.name.as_str() == "f" => Some(function.body),
            _ => None,
        })
        .expect("a function named f");
    checked
        .type_env
        .get_expr_type(body)
        .cloned()
        .expect("a type for f's body")
}

#[test]
fn a_value_joined_with_null_is_nullable() {
    assert_eq!(
        body_type("let f(b:boolean, n:int) = { if b { n } else { null } }"),
        Type::nullable(Type::int())
    );
    assert_eq!(
        body_type("let f(b:boolean, x:float64) = { if b { null } else { x } }"),
        Type::nullable(Type::float64())
    );
    assert_eq!(
        body_type("let f(n:int) = { n null }"),
        Type::array(Type::nullable(Type::int()))
    );
}

#[test]
fn nullability_is_lifted_out_of_a_numeric_join_in_either_order() {
    assert_eq!(
        body_type("let f(b:boolean, n:int?, x:float64) = { if b { n } else { x } }"),
        Type::nullable(Type::float64())
    );
    assert_eq!(
        body_type("let f(b:boolean, n:int, x:float64?) = { if b { n } else { x } }"),
        Type::nullable(Type::float64())
    );
    assert_eq!(
        body_type(
            "let f(a:boolean, b:boolean, n:int, x:float64) = \
             { if a { n } else { if b { x } else { null } } }"
        ),
        Type::nullable(Type::float64())
    );
}

#[test]
fn nullability_is_lifted_out_of_a_record_join() {
    assert_eq!(
        body_type(
            "abstract type Animal = { name:string }\n\
             type Cat extends Animal = { lives:int }\n\
             type Dog extends Animal = { good:boolean }\n\
             let f(b:boolean, cat:Cat?, dog:Dog) = { if b { cat } else { dog } }"
        )
        .to_string(),
        "Animal?"
    );
}

#[test]
fn a_join_that_climbs_to_object_stays_object() {
    assert_eq!(
        body_type("let f(b:boolean, s:string?, x:float64) = { if b { s } else { x } }"),
        Type::named("object")
    );
}
