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
fn the_unit_type_still_renders_as_void() {
    // Removing the spelling does not remove the type or its name in diagnostics: the author
    // receives the name, they do not supply it.
    assert_eq!(Type::void().to_string(), "void");
}

#[test]
fn a_no_else_conditional_still_takes_the_unit_type() {
    // `if` with no `else` is one of the sites inference assigns the unit type. The observable
    // consequence here is that its type is not the then-branch's.
    let source = "let c = true\nlet v = { if c { 1 } }";
    let ty = check(source)
        .type_env
        .lookup(&Name::new("v"))
        .cloned()
        .expect("binding v");
    assert_eq!(ty, Type::void(), "expected the unit type, got: {ty}");
}
