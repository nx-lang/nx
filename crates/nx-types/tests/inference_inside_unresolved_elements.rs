//! Type analysis of the content and property values of an element whose tag resolves to nothing.
//!
//! One case per scenario in the `source-analysis-pipeline` addition made by the
//! `improve-hover-content` change. The four resolved-tag paths in `infer_element_expression` infer
//! their children as a side effect of checking bindings against a declared contract. The
//! unresolved-tag fallthrough had no contract to check, so it returned a nominal type for the tag
//! without visiting the element at all, and every expression written inside an intrinsic element
//! went unchecked and untyped.

use nx_hir::ast::Expr;
use nx_types::analyze_str;

fn errors(source: &str) -> Vec<String> {
    analyze_str(source, "test.nx")
        .errors()
        .iter()
        .map(|diagnostic| diagnostic.message().to_string())
        .collect()
}

fn assert_reports(source: &str, needle: &str) {
    let errors = errors(source);
    assert!(
        errors.iter().any(|message| message.contains(needle)),
        "expected an error containing {needle:?}, got: {errors:?}"
    );
}

#[test]
fn a_type_error_in_an_unresolved_elements_content_is_reported() {
    let in_element = errors("let root() = <div>{1 + \"a\"}</div>\n");
    let at_top = errors("let root() = { 1 + \"a\" }\n");

    assert!(
        !at_top.is_empty(),
        "the addition of int and string is expected to be an error outside an element"
    );
    assert_eq!(
        in_element, at_top,
        "an element whose tag resolves to nothing should report what the same content reports \
         written outside one"
    );
}

#[test]
fn a_type_error_in_an_unresolved_elements_property_value_is_reported() {
    assert_reports("let root() = <div class={1 + \"a\"} />\n", "string");
}

#[test]
fn an_element_nested_inside_an_unresolved_element_is_still_checked() {
    let preamble = "let <Wrap content:string /> = <div />\n";
    let nested = errors(&format!(
        "{preamble}let root() = <div><Wrap content={{42}} /></div>\n"
    ));
    let at_top = errors(&format!("{preamble}let root() = <Wrap content={{42}} />\n"));

    assert!(
        !at_top.is_empty(),
        "a string property given an int is expected to be an error outside an element"
    );
    assert_eq!(
        nested, at_top,
        "nesting an element inside an unresolved one should not change what it reports"
    );
}

#[test]
fn an_unresolved_tag_reports_nothing_beyond_the_tag() {
    let errors = errors("let root() = <div>{\"ok\"}</div>\n");
    assert!(
        errors.is_empty(),
        "well-typed content inside an unresolved element should report nothing, got: {errors:?}"
    );
}

/// Duplicate detection reads the property paths and needs no binding contract, so an absent tag
/// does not excuse it — the delta says the tag is the only thing left unchecked.
#[test]
fn a_property_supplied_twice_on_an_unresolved_tag_is_reported() {
    let unresolved = errors("let root() = <div class=1 class=2 />\n");
    assert!(
        unresolved.iter().any(|message| message.contains("class")),
        "expected a duplicate-property error, got: {unresolved:?}"
    );
}

#[test]
fn the_type_of_an_expression_inside_an_unresolved_element_is_recorded() {
    let artifact = analyze_str("let <Row count:int /> = <div>{count}</div>\n", "test.nx");
    let lowered = artifact
        .lowered_module
        .as_ref()
        .expect("the module is expected to lower");

    let (id, _) = lowered
        .exprs()
        .find(|(_, expr)| matches!(expr, Expr::Ident(name) if name.as_str() == "count"))
        .expect("the interpolated `count` is expected to be lowered");

    let ty = artifact
        .type_env
        .get_expr_type(id)
        .expect("the type of an expression inside an unresolved element should be recorded");
    assert_eq!(ty.to_string(), "int");
}
