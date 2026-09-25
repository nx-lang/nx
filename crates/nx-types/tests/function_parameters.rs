//! How a `let` function is called: a paren-style function by position or as an element, an
//! element-style function only as an element, and a parameter a call leaves out filled by the
//! function with its default, or with empty.
//!
//! One case per scenario in the `function-parameters` capability.

use nx_types::check_str;

fn errors(source: &str) -> Vec<String> {
    check_str(source, "test.nx")
        .errors()
        .iter()
        .map(|diagnostic| diagnostic.message().to_string())
        .collect()
}

fn assert_clean(source: &str) {
    let errors = errors(source);
    assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
}

fn assert_one_error(source: &str, expected: &str) {
    let errors = errors(source);
    assert_eq!(errors.len(), 1, "expected one error, got: {errors:?}");
    assert!(
        errors[0].contains(expected),
        "expected an error containing {expected:?}, got: {errors:?}"
    );
}

#[test]
fn a_required_parameter_after_an_omissible_one_is_rejected() {
    assert_one_error(
        "let f(a?:int, b:int) = { b }",
        "Required parameter 'b' follows 'a', which a caller may omit",
    );
    assert_one_error(
        "let f(a:int = 1, b:int) = { b }",
        "Required parameter 'b' follows 'a', which a caller may omit",
    );
}

#[test]
fn an_element_style_function_orders_its_parameters_freely() {
    assert_clean("let <F a?:int b:int /> = { b }\nlet x = <F b=1 />");
}

#[test]
fn a_positional_call_leaves_out_trailing_parameters() {
    let source = "let f(a:int, b:int = 2, c?:int) = { a + b + (c ?? 0) }\n\
                  let one = { f(1) }\n\
                  let two = { f(1, 2) }\n\
                  let three = { f(1, 2, 3) }";
    assert_clean(source);
}

#[test]
fn a_positional_call_must_reach_every_required_parameter() {
    assert_one_error(
        "let f(a:int, b:int = 2, c?:int) = { a }\nlet x = { f() }",
        "Function expects 1 to 3 arguments, got 0",
    );
    assert_one_error(
        "let f(a:int, b:int = 2) = { a }\nlet x = { f(1, 2, 3) }",
        "Function expects 1 to 2 arguments, got 3",
    );
    assert_one_error(
        "let f(a:int, b:int) = { a }\nlet x = { f(1) }",
        "Function expects 2 arguments, got 1",
    );
}

#[test]
fn an_element_call_of_a_paren_style_function_leaves_out_any_omissible_parameter() {
    assert_clean(
        "let f(a:int, b:int = 2, c?:int) = { a }\n\
         let x = <f a=1 c=3 />\n\
         let y = <f a=1 />",
    );
    assert_one_error(
        "let f(a:int, b:int = 2) = { a }\nlet x = <f b=1 />",
        "Element 'f' requires property 'a'",
    );
}

#[test]
fn an_element_style_function_cannot_be_called_by_position() {
    assert_one_error(
        "let <F a:int b:int /> = { a + b }\nlet x = { F(1, 2) }",
        "'F' is declared in element style, so its arguments bind by name; call it as an element, <F a=... b=... />",
    );
}

#[test]
fn a_default_is_checked_against_its_parameter() {
    assert_one_error(
        "let f(a:int, b:int = \"two\") = { a }",
        "Default value for parameter 'b' expects int, found string",
    );
    assert_one_error(
        "let <F a:int b:string = { 2 } /> = { a }",
        "Default value for parameter 'b' expects string, found int",
    );
}

#[test]
fn a_default_reads_the_parameters_before_it() {
    assert_clean("let f(a:int, b:int = { a * 10 }) = { a + b }");
    assert_clean("let <F a:int b:int = { a + 1 } /> = { a + b }");
    assert_one_error(
        "let f(a:int = { b }, b:int = 1) = { a }",
        "Undefined identifier 'b'",
    );
}

#[test]
fn a_default_on_an_optional_parameter_is_still_rejected() {
    assert_one_error(
        "let f(a:int, b?:int = 2) = { a }",
        "write `b:int = 2` or `b?:int`",
    );
}
