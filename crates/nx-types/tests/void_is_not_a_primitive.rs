//! `void` has no source spelling.
//!
//! One case per scenario in the `primitive-type-names` capability's requirements about the
//! primitive set and the inference-internal unit type.

use nx_hir::Name;
use nx_types::{check_str, Type, TypeCheckResult};

fn check(source: &str) -> TypeCheckResult {
    check_str(source, "test.nx")
}

fn errors(source: &str) -> Vec<String> {
    check(source)
        .errors()
        .iter()
        .map(|diagnostic| diagnostic.message().to_string())
        .collect()
}

#[test]
fn void_in_type_position_is_an_ordinary_named_type() {
    // Not a primitive: a string does not satisfy it, exactly as it would not satisfy any other
    // undeclared name. NX reports nothing at the declaration for an undeclared name — the same
    // for `void` as for `Undeclared` — so the observable difference is at the binding.
    let named = errors("type Holder = { n:void }\n<Holder n=\"x\" />");
    assert!(
        named.iter().any(|message| message.contains("expects void")),
        "expected `void` to behave as a named type, got: {named:?}"
    );

    let undeclared = errors("type Holder = { n:Undeclared }\n<Holder n=\"x\" />");
    assert!(
        undeclared
            .iter()
            .any(|message| message.contains("expects Undeclared")),
        "an undeclared name should behave the same way, got: {undeclared:?}"
    );
}

#[test]
fn a_user_declaration_may_take_the_name_void() {
    let source =
        "type void = { value:int }\ntype Holder = { n:void }\n<Holder n=<void value=1 /> />";
    let errors = errors(source);
    assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
}

#[test]
fn no_inferred_type_renders_as_void() {
    // The unit type is gone: an `if` with no `else` carries an implicit `else { }` and is an
    // optional, an uncovered match path follows the same rule, and nothing else constructed one.
    // So no diagnostic can name a type `void` except the user's own declaration of that name.
    let messages = errors("let c = true\nlet v:string = { if c { 1 } }");
    assert!(!messages.is_empty());
    assert!(
        messages.iter().all(|message| !message.contains("void")),
        "got: {messages:?}"
    );
}

#[test]
fn a_no_else_conditional_is_an_optional_rather_than_the_unit_type() {
    // An `if` with no `else` carries an implicit `else { }`, so its type is the join of the branch
    // with the empty value -- the branch's type admitting zero, not the unit type.
    let source = "let c = true\nlet v = { if c { 1 } }";
    let ty = check(source)
        .type_env
        .lookup(&Name::new("v"))
        .cloned()
        .expect("binding v");
    assert_eq!(ty, Type::optional(Type::int()), "got: {ty}");
}

#[test]
fn a_diagnostic_naming_a_user_declared_void_names_only_that_type() {
    // With no unit type left, `void` in a message can only be the declaration the author wrote,
    // so one rendering cannot stand for two types.
    let messages =
        errors("type void = { value:int }\ntype Holder = { n:void }\nlet h = <Holder n=\"x\" />");
    assert!(
        messages
            .iter()
            .any(|message| message.contains("expects void")),
        "got: {messages:?}"
    );
    assert!(
        messages
            .iter()
            .all(|message| !message.contains("found void")),
        "no inferred type is rendered as `void`: {messages:?}"
    );
}
