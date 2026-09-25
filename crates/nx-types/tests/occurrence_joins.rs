//! How a join combines occurrences: the item types join as they always did, and the occurrences
//! take their least upper bound in the lattice `1 ⊂ ? ⊂ *`, `1 ⊂ + ⊂ *`.
//!
//! The empty value `{}` contributes no item type and admits zero, so joining it with `T` is `T?`.
//! A join whose item types climb to `object` stays `object`, under whatever occurrence the sides
//! agree on.

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

fn errors(source: &str) -> Vec<String> {
    check_str(source, "test.nx")
        .errors()
        .iter()
        .map(|diagnostic| diagnostic.message().to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// The occurrence is the least upper bound
// ---------------------------------------------------------------------------

#[test]
fn a_value_joined_with_the_empty_value_is_optional() {
    assert_eq!(
        body_type("let f(b:boolean, n:int) = { if b { n } else {} }"),
        Type::optional(Type::int())
    );
    assert_eq!(
        body_type("let f(b:boolean, x:float64) = { if b {} else { x } }"),
        Type::optional(Type::float64())
    );
    // A missing `else` is the same join.
    assert_eq!(
        body_type("let f(b:boolean, n:int) = { if b { n } }"),
        Type::optional(Type::int())
    );
}

#[test]
fn exactly_one_joined_with_one_or_more_is_one_or_more() {
    assert_eq!(
        body_type("let f(b:boolean, n:int, ns:int+) = { if b { n } else { ns } }"),
        Type::one_or_more(Type::int())
    );
    assert_eq!(
        body_type("let f(b:boolean, n:int, ns:int+) = { if b { ns } else { n } }"),
        Type::one_or_more(Type::int())
    );
}

#[test]
fn optional_joined_with_one_or_more_is_zero_or_more() {
    // `?` and `+` are incomparable, so the join climbs to the top of the lattice.
    assert_eq!(
        body_type("let f(b:boolean, ns:int+, o?:int) = { if b { o } else { ns } }"),
        Type::zero_or_more(Type::int())
    );
}

#[test]
fn one_or_more_joined_with_optional_is_zero_or_more() {
    assert_eq!(
        body_type("let f(b:boolean, ns:int+, o?:int) = { if b { ns } else { o } }"),
        Type::zero_or_more(Type::int())
    );
}

// ---------------------------------------------------------------------------
// The item types join on their own, under the joined occurrence
// ---------------------------------------------------------------------------

#[test]
fn the_occurrence_is_lifted_out_of_a_numeric_join_in_either_order() {
    assert_eq!(
        body_type("let f(b:boolean, x:float64, n?:int) = { if b { n } else { x } }"),
        Type::optional(Type::float64())
    );
    assert_eq!(
        body_type("let f(b:boolean, n:int, x?:float64) = { if b { n } else { x } }"),
        Type::optional(Type::float64())
    );
    assert_eq!(
        body_type(
            "let f(a:boolean, b:boolean, n:int, x:float64) = \
             { if a { n } else { if b { x } else {} } }"
        ),
        Type::optional(Type::float64())
    );
}

#[test]
fn the_occurrence_is_lifted_out_of_a_record_join() {
    assert_eq!(
        body_type(
            "abstract type Animal = { name:string }\n\
             type Cat extends Animal = { lives:int }\n\
             type Dog extends Animal = { good:boolean }\n\
             let f(b:boolean, dog:Dog, cat?:Cat) = { if b { cat } else { dog } }"
        )
        .to_string(),
        "Animal?"
    );
}

#[test]
fn a_join_that_climbs_to_object_keeps_the_occurrence() {
    assert_eq!(
        body_type("let f(b:boolean, x:float64, s?:string) = { if b { s } else { x } }"),
        Type::optional(Type::named("object"))
    );
}

// ---------------------------------------------------------------------------
// Satisfaction follows the same lattice
// ---------------------------------------------------------------------------

#[test]
fn an_optional_is_rejected_where_exactly_one_is_expected() {
    let errors = errors("let f(o?:int): int = { o }");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("expects int,") && errors[0].contains("found int?"),
        "{errors:?}"
    );
}

#[test]
fn zero_or_more_is_rejected_where_one_or_more_is_expected() {
    let errors = errors("let f(xs?:int+): int+ = { xs }");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("expects int+") && errors[0].contains("found int*"),
        "{errors:?}"
    );
}

#[test]
fn one_or_more_is_rejected_where_optional_is_expected() {
    let errors = errors("let f(xs:int+): int? = { xs }");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("expects int?") && errors[0].contains("found int+"),
        "{errors:?}"
    );
}
