//! Type-checking behavior for function types and function values.
//!
//! One case per scenario in the `function-types` and `function-values` capabilities, plus the
//! `component-type-parameters` scenarios about a function-typed prop.

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

fn assert_reports(source: &str, needle: &str) -> Vec<String> {
    let errors = errors(source);
    assert!(
        errors.iter().any(|message| message.contains(needle)),
        "expected an error containing {needle:?}, got: {errors:?}"
    );
    errors
}

// `DrawnNode` is abstract, so a template body renders the concrete `SkiaLabel`.
const DRAWN_NODE: &str = "abstract external component <DrawnNode />\nexternal component <SkiaLabel extends DrawnNode Text?:string />\n";
const CONTACT: &str = "type Contact = { name:string }\n";
const SKIA_LAYOUT: &str = "external component <SkiaLayout TItem:type ItemsSource?:TItem+ ItemTemplate?:<function Item:TItem Index:int />: DrawnNode />\n";
const CONTACT_ROW: &str = "let <ContactRow Item:Contact Index:int />: DrawnNode = <SkiaLabel />\n";

// ---------------------------------------------------------------------------------------------
// A function type is spelled as an element function signature
// ---------------------------------------------------------------------------------------------

#[test]
fn a_property_is_declared_at_a_function_type() {
    assert_clean(&format!(
        "{DRAWN_NODE}external component <DataTable RowTemplate: <function Item:object Index:int />: DrawnNode />"
    ));
}

#[test]
fn a_function_type_takes_no_parameters() {
    assert_clean(&format!(
        "{DRAWN_NODE}external component <DataTable HeaderTemplate: <function />: DrawnNode />"
    ));
}

#[test]
fn an_aliased_function_type_is_the_inline_one() {
    assert_clean(&format!(
        "{DRAWN_NODE}{CONTACT}type RowTemplate = <function Item:Contact Index:int />: DrawnNode\n\
         {CONTACT_ROW}\
         component <A extends DrawnNode Row:RowTemplate /> = {{ <SkiaLabel /> }}\n\
         component <B extends DrawnNode Row:<function Item:Contact Index:int />: DrawnNode /> = {{ <SkiaLabel /> }}\n\
         let a = <A Row={{ContactRow}} />\n\
         let b = <B Row={{ContactRow}} />"
    ));
}

#[test]
fn a_suffix_after_the_result_binds_to_the_result() {
    // `Maybe` is exactly one function returning `string?`: a function returning `string`
    // satisfies it, and the empty value does not.
    assert_clean(
        "type Maybe = <function Count:int />: string?\n\
         let <Count Count:int />: string = \"x\"\n\
         let m: Maybe = {Count}",
    );
    assert_reports(
        "type Maybe = <function Count:int />: string?\nlet m: Maybe = {}",
        "expects <function Count:int />: string?, found {}",
    );
    // `Many` and `Rows` are likewise exactly one function each, returning `string+` and
    // `DrawnNode*`; a suffix on a parenthesized function type counts functions instead.
    assert_clean(&format!(
        "{DRAWN_NODE}type Many = <function />: string+\ntype Rows = <function />: DrawnNode*\n\
         let <Two />: string+ = {{ \"a\" \"b\" }}\n\
         let <None />: DrawnNode* = {{}}\n\
         let many: Many = {{Two}}\n\
         let rows: Rows = {{None}}\n\
         let <One />: string = \"x\"\n\
         let fns: (<function />: string)+ = {{ One One }}"
    ));
    assert_reports(
        "let <One />: string = \"x\"\nlet fns: (<function />: string)+ = {}",
        "expects (<function />: string)+, found {}",
    );
}

#[test]
fn a_content_parameter_is_accepted_once() {
    assert_clean(
        "type Wrap = <function content Children:object+ />: string\n\
         let <Box content Children:object+ />: string = \"x\"\n\
         let w: Wrap = {Box}",
    );
}

#[test]
fn the_word_function_is_still_an_identifier_elsewhere() {
    assert_clean("let function = 1\nlet v = {function}");
}

// ---------------------------------------------------------------------------------------------
// A function type is displayed in NX spelling
// ---------------------------------------------------------------------------------------------

#[test]
fn a_mismatch_diagnostic_shows_the_function_type() {
    let errors = assert_reports(
        &format!(
            "{DRAWN_NODE}{CONTACT}component <Section extends DrawnNode Row:<function Item:Contact />: DrawnNode /> = {{ <SkiaLabel /> }}\n\
             let s = <Section Row=\"text\" />"
        ),
        "expects <function Item:Contact />: DrawnNode, found string",
    );
    assert!(
        errors.iter().all(|message| !message.contains("=>")),
        "{errors:?}"
    );
}

#[test]
fn an_optional_function_type_is_parenthesized() {
    assert_reports(
        &format!(
            "{DRAWN_NODE}component <Section extends DrawnNode Row?:<function />: DrawnNode /> = {{ <SkiaLabel /> }}\n\
             let s = <Section Row=\"text\" />"
        ),
        "expects (<function />: DrawnNode)?, found string",
    );
}

// ---------------------------------------------------------------------------------------------
// A function satisfies a function type by parameter name
// ---------------------------------------------------------------------------------------------

#[test]
fn an_exact_match_is_compatible() {
    assert_clean(&format!(
        "{CONTACT}let <Row Item:Contact Index:int />: string = {{Item.name}}\n\
         let r: <function Item:Contact Index:int />: string = {{Row}}"
    ));
}

#[test]
fn a_function_that_ignores_a_parameter_is_compatible() {
    assert_clean(&format!(
        "{CONTACT}let <Compact Item:Contact />: string = {{Item.name}}\n\
         let r: <function Item:Contact Index:int />: string = {{Compact}}"
    ));
}

#[test]
fn a_function_that_needs_a_parameter_the_type_lacks_is_rejected() {
    let errors = assert_reports(
        &format!(
            "{CONTACT}let <Row Item:Contact Index:int />: string = {{Item.name}}\n\
             let r: <function Item:Contact />: string = {{Row}}"
        ),
        "'Index', which the function type does not supply",
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
}

/// Parameters match by name, so an omissible parameter the type does not supply is still refused:
/// accepting it would let a misspelled name compile and always read its default.
#[test]
fn a_function_with_an_extra_omissible_parameter_is_rejected() {
    for parameter in ["Idx:int = 0", "Idx?:int"] {
        let errors = assert_reports(
            &format!(
                "let <Row Item:object {parameter} />: string = \"x\"\n\
                 let r: <function Item:object Index:int />: string = {{Row}}"
            ),
            "'Idx', which the function type does not supply",
        );
        assert_eq!(errors.len(), 1, "{parameter}: {errors:?}");
    }
}

#[test]
fn a_parameter_type_is_checked_contravariantly() {
    assert_clean(
        "let <Show Value:object />: string = \"x\"\n\
         let r: <function Value:int />: string = {Show}",
    );
    assert_reports(
        "let <Count Value:int />: string = \"x\"\n\
         let bad: <function Value:object />: string = {Count}",
        "parameter 'Value' is supplied as object, which is not int",
    );
}

#[test]
fn a_result_type_is_checked_covariantly() {
    assert_clean(&format!(
        "{DRAWN_NODE}let <Row Item:object />: SkiaLabel = <SkiaLabel />\n\
         let r: <function Item:object />: DrawnNode = {{Row}}"
    ));
    assert_reports(
        &format!(
            "{DRAWN_NODE}let <Row Item:object />: string = \"x\"\n\
             let r: <function Item:object />: DrawnNode = {{Row}}"
        ),
        "the result string is not DrawnNode",
    );
}

#[test]
fn a_parameter_name_mismatch_is_rejected() {
    assert_reports(
        "let <Row Item:object />: string = \"x\"\n\
         let r: <function Entry:object />: string = {Row}",
        "'Item', which the function type does not supply",
    );
}

#[test]
fn parameter_order_does_not_matter() {
    assert_clean(
        "let <Row Index:int Item:object />: string = \"x\"\n\
         let r: <function Item:object Index:int />: string = {Row}",
    );
}

// ---------------------------------------------------------------------------------------------
// A visible function name is a value of its function type
// ---------------------------------------------------------------------------------------------

#[test]
fn a_function_is_bound_to_a_function_typed_property() {
    assert_clean(&format!(
        "{DRAWN_NODE}{CONTACT}{SKIA_LAYOUT}{CONTACT_ROW}\
         let contacts:Contact* = {{}}\n\
         let v = <SkiaLayout TItem=Contact ItemsSource={{contacts}} ItemTemplate={{ContactRow}} />"
    ));
}

#[test]
fn a_function_is_forwarded_through_an_authored_component() {
    assert_clean(&format!(
        "{DRAWN_NODE}{CONTACT}type RowTemplate = <function Item:Contact Index:int />: DrawnNode\n\
         {SKIA_LAYOUT}\
         component <Section extends DrawnNode Items?:Contact+ Row:RowTemplate /> = {{ <SkiaLayout TItem=Contact ItemsSource={{Items}} ItemTemplate={{Row}} /> }}\n\
         {CONTACT_ROW}\
         let s = <Section Items={{}} Row={{ContactRow}} />"
    ));
}

#[test]
fn a_paren_function_is_a_value_too() {
    assert_clean("let double(n:int): int = {n * 2}\nlet f: <function n:int />: int = {double}");
}

#[test]
fn a_function_bound_where_a_non_function_is_expected_is_a_mismatch() {
    assert_reports(
        "let <Row Item:object />: string = \"x\"\nlet s:string = {Row}",
        "expects string, found <function Item:object />: string",
    );
}

#[test]
fn an_undefined_name_is_still_undefined() {
    assert_reports("let v = {NoSuchFunction}", "NoSuchFunction");
}

#[test]
fn a_lexical_binding_shadows_a_function_of_the_same_name() {
    assert_clean("let <Row Item:object />: string = \"x\"\nlet pick(Row:string): string = {Row}");
}

// ---------------------------------------------------------------------------------------------
// A function-typed value is invocable as an element
// ---------------------------------------------------------------------------------------------

#[test]
fn a_function_typed_prop_is_invoked_as_an_element() {
    assert_clean(&format!(
        "{DRAWN_NODE}{CONTACT}component <Section extends DrawnNode Item:Contact Row:<function Item:Contact Index:int />: DrawnNode /> = {{ <Row Item={{Item}} Index=0 /> }}\n\
         {CONTACT_ROW}\
         let s = <Section Item=<Contact name=\"a\" /> Row={{ContactRow}} />"
    ));
}

#[test]
fn a_function_typed_parameter_of_a_paren_function_is_invoked_as_an_element() {
    let checked = check_str(
        "let invoke(f: <function n:int />: int, n:int): int = <f n={n} />\n\
         let double(n:int): int = {n * 2}\n\
         let v = {invoke(double, 4)}",
        "test.nx",
    );
    let errors: Vec<_> = checked
        .errors()
        .iter()
        .map(|d| d.message().to_string())
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    let v = checked
        .type_env
        .lookup(&nx_hir::Name::new("v"))
        .expect("v is bound");
    assert_eq!(v.to_string(), "int");
}

#[test]
fn a_paren_style_call_on_a_function_typed_value_is_rejected() {
    assert_reports(
        "let invoke(f: <function n:int />: int, n:int): int = {f(n)}",
        "<f n=... />",
    );
}

#[test]
fn a_missing_argument_is_reported_against_the_type() {
    assert_reports(
        "component <Section Row:<function Item:object Index:int />: string /> = { <Row Item=\"a\" /> }",
        "'Row' requires property 'Index'",
    );
}

#[test]
fn an_argument_the_type_does_not_declare_is_reported() {
    assert_reports(
        "component <Section Row:<function Item:object />: string /> = { <Row Item=\"a\" Extra=1 /> }",
        "Element 'Row' has no property 'Extra'",
    );
}

#[test]
fn a_non_function_binding_is_not_a_tag() {
    // `Label` the prop is a string; the tag resolves to the declared component, so the element
    // is accepted as a `Label` instance rather than rejected as a call of a string.
    assert_clean(
        "external component <Label />\ncomponent <Section Label:string /> = { <Label /> }",
    );
}

// ---------------------------------------------------------------------------------------------
// A type argument is substituted through a function-typed prop
// ---------------------------------------------------------------------------------------------

#[test]
fn a_template_is_checked_at_the_substituted_item_type() {
    let shared = format!(
        "{DRAWN_NODE}{CONTACT}type Post = {{ title:string }}\n{SKIA_LAYOUT}{CONTACT_ROW}\
         let <PostRow Item:Post Index:int />: DrawnNode = <SkiaLabel />\n\
         let contacts:Contact* = {{}}\n"
    );
    assert_clean(&format!(
        "{shared}let ok = <SkiaLayout TItem=Contact ItemsSource={{contacts}} ItemTemplate={{ContactRow}} />"
    ));
    assert_reports(
        &format!(
            "{shared}let bad = <SkiaLayout TItem=Contact ItemsSource={{contacts}} ItemTemplate={{PostRow}} />"
        ),
        "expects (<function Item:Contact Index:int />: DrawnNode)?, found <function Item:Post Index:int />: DrawnNode",
    );
}

#[test]
fn a_template_bound_without_a_type_argument_names_the_parameter() {
    let errors = assert_reports(
        &format!(
            "{DRAWN_NODE}{CONTACT}external component <SkiaLayout TItem:type ItemTemplate?:<function Item:TItem />: DrawnNode />\n\
             let <ContactRow Item:Contact />: DrawnNode = <SkiaLabel />\n\
             let v = <SkiaLayout ItemTemplate={{ContactRow}} />"
        ),
        "'TItem', which was not specified",
    );
    assert!(errors.iter().any(|m| m.contains("TItem=")), "{errors:?}");
}

#[test]
fn a_forwarded_parameter_reaches_a_function_typed_prop() {
    assert_clean(&format!(
        "{DRAWN_NODE}{SKIA_LAYOUT}\
         component <Section extends DrawnNode TItem:type Items:TItem+ Row:<function Item:TItem Index:int />: DrawnNode /> = {{ <SkiaLayout TItem=TItem ItemsSource={{Items}} ItemTemplate={{Row}} /> }}"
    ));
}

#[test]
fn a_function_typed_parameter_shadows_a_declared_function_as_a_tag() {
    // `Row` the parameter is the callee inside `Section`, not `Row` the declaration, so the
    // call is checked against the parameter's type, which has no `Index`.
    assert_clean(&format!(
        "{CONTACT}let <Section Item:Contact Row:<function Item:Contact />: string />: string = <Row Item={{Item}} />\n\
         let <Row Item:Contact Index:int />: string = {{Item.name}}\n\
         let <Compact Item:Contact />: string = {{Item.name}}\n\
         let s = <Section Item=<Contact name=\"a\" /> Row={{Compact}} />"
    ));
}
