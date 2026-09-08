//! `never` has no source spelling.
//!
//! One case per scenario in the `primitive-type-names` capability's requirement about the
//! inference-internal bottom type. The type itself is exercised by `empty_lists.rs`, which is
//! where every value of a type mentioning `never` comes from.

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

/// The bottom type is below every type, which is the whole of what it is for: one empty list is a
/// member of every list type, with no site consulted to make it one.
#[test]
fn never_satisfies_every_expected_type() {
    assert!(Type::never().is_compatible_with(&Type::string()));
    assert!(Type::never().is_compatible_with(&Type::int()));
    assert!(Type::never().is_compatible_with(&Type::named("object")));
    assert!(Type::array(Type::never()).is_compatible_with(&Type::array(Type::string())));
    assert!(Type::array(Type::never()).is_compatible_with(&Type::array(Type::int())));
}

/// Nothing is below the bottom type, so the relation does not run the other way.
#[test]
fn nothing_else_satisfies_never() {
    assert!(!Type::string().is_compatible_with(&Type::never()));
    assert!(!Type::named("object").is_compatible_with(&Type::never()));
}

#[test]
fn never_is_not_a_primitive_in_type_position() {
    // Resolved by the rules that govern any undeclared name, exactly as `void` is now.
    let source = "type Handler = { result:never }\nlet h = <Handler result=1 />";
    let messages = errors(source);
    assert!(
        !messages.is_empty(),
        "`never` must not resolve to the bottom type and accept an int, got: {messages:?}"
    );
}

#[test]
fn a_user_declaration_may_take_the_name_never() {
    let source = "type never = { value:int }\ntype Holder = { n:never }\n\
                  let h = <Holder n={<never value=1 />} />";
    let messages = errors(source);
    assert!(
        messages.is_empty(),
        "a user type named `never` should resolve like any other, got: {messages:?}"
    );
}

/// The type renders under its own name, on the same terms as the unit type: an author receives
/// the name in a diagnostic, they never write it.
#[test]
fn never_renders_under_its_own_name() {
    assert_eq!(Type::never().to_string(), "never");
    assert_eq!(Type::array(Type::never()).to_string(), "never[]");
}

/// A diagnostic about a value the author wrote as `{}` spells it `{}`, not `never[]`. The bottom
/// type is accurate and is still not the form the author can act on.
#[test]
fn a_diagnostic_spells_the_empty_list_as_the_source_does() {
    let messages = errors("let value:string = {}");
    assert!(
        messages.iter().any(|message| message.contains("{}")),
        "expected the value spelled as `{{}}`, got: {messages:?}"
    );
    assert!(
        !messages.iter().any(|message| message.contains("never")),
        "a type with no source spelling should not be named here, got: {messages:?}"
    );
}
