//! Type-checking behavior for the empty braced list `{}`.
//!
//! One case per scenario in the `braced-value-sequences` capability's typing requirement.

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

fn assert_clean(source: &str) {
    let errors = errors(source);
    assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
}

fn binding_type(source: &str, name: &str) -> Type {
    check(source)
        .type_env
        .lookup(&Name::new(name))
        .unwrap_or_else(|| panic!("no binding named {name:?}"))
        .clone()
}

const LINK: &str = "type ChatBrandLink = { label:string }\n";

// ---------------------------------------------------------------------------
// The element type comes from the site
// ---------------------------------------------------------------------------

#[test]
fn empty_list_takes_its_element_type_from_an_annotation() {
    let source = "let value:string[] = {}";
    assert_clean(source);
    assert_eq!(binding_type(source, "value"), Type::array(Type::string()));
}

#[test]
fn empty_list_takes_its_element_type_from_a_property_site() {
    assert_clean("type Fit = fill | contain | cover\ntype Img = { fits:Fit[] }\n<Img fits={} />");
}

#[test]
fn empty_list_is_accepted_as_a_field_default() {
    assert_clean(&format!(
        "{LINK}type Brand = {{ links:ChatBrandLink[] = {{}} }}\n<Brand />"
    ));
}

#[test]
fn empty_list_is_accepted_as_a_component_property_default() {
    assert_clean("external component <C xs:string[] = {} />");
}

#[test]
fn empty_list_is_accepted_as_element_body_content() {
    assert_clean(
        "external component <List content items:object[] />\ncomponent <N /> = { <List>{}</List> }",
    );
}

#[test]
fn empty_list_at_a_nullable_list_site_is_a_non_null_empty_list() {
    // The `T[]?` versus `T?[]` distinction: the field is a list that may be absent, so supplying
    // `{}` supplies the list, not the absence of one.
    let source = format!("{LINK}type Brand = {{ links:ChatBrandLink[]? }}\n<Brand links={{}} />");
    assert_clean(&source);
}

#[test]
fn empty_list_is_accepted_as_a_function_body_with_a_declared_list_return() {
    // A function body is a values brace too, so the empty form reaches it. It is accepted only
    // where the return type says what the elements are.
    assert_clean("let f():string[] = {}");
}

#[test]
fn empty_function_body_with_no_declared_return_type_is_reported() {
    // This source was a parse error before `{}` was admitted. It is still rejected, now by the
    // element-type rule rather than by the parser.
    let errors = errors("let <f /> = { }");
    assert!(
        errors
            .iter()
            .any(|message| message.contains("element type")),
        "expected an element-type diagnostic, got: {errors:?}"
    );
}

// ---------------------------------------------------------------------------
// No expected type is an error, not `object[]`
// ---------------------------------------------------------------------------

#[test]
fn empty_list_with_no_expected_type_is_reported() {
    let source = "let value = {}";
    let errors = errors(source);
    assert!(
        errors
            .iter()
            .any(|message| message.contains("element type") && message.contains("value")),
        "expected an element-type diagnostic naming the binding, got: {errors:?}"
    );
}

#[test]
fn empty_list_with_no_expected_type_does_not_infer_object_list() {
    // Asserted explicitly: an `object[]` fallback would silently pass a test that only checked for
    // the absence of errors, and would let the list flow to any list-typed site.
    let source = "let value = {}";
    assert_ne!(
        binding_type(source, "value"),
        Type::array(Type::named("object")),
        "an empty list with no expected type must not fall back to object[]"
    );
}

/// A binding that is annotated with a non-list type reports one thing: the mismatch.
///
/// The element-type diagnostic is for a site that supplies no expected type. Here the site supplied
/// one and it was not a list, so the annotation is not the thing to change, and telling the author
/// to write an annotation they already wrote points at the wrong half of the line.
#[test]
fn an_annotated_non_list_binding_reports_only_the_mismatch() {
    let source = "let value:string = {}";
    let messages = errors(source);
    assert_eq!(
        messages.len(),
        1,
        "expected only the type mismatch, got: {messages:?}"
    );
    assert!(
        messages[0].contains("expects string"),
        "expected the mismatch to name the declared type, got: {messages:?}"
    );
    assert!(
        !messages[0].contains("annotate"),
        "the binding is already annotated, got: {messages:?}"
    );
}

/// The mismatch names the empty list as the author wrote it.
///
/// The empty list's type is `never[]`, and `never` has no source spelling, so rendering the type
/// directly would put a name the author cannot write in front of someone who wrote `{}`.
#[test]
fn a_mismatch_on_an_empty_list_does_not_name_the_element_type() {
    let source = "let value:string = {}";
    let messages = errors(source);
    assert!(
        messages.iter().any(|message| message.contains("{}")),
        "expected the diagnostic to spell the value as `{{}}`, got: {messages:?}"
    );
    assert!(
        !messages
            .iter()
            .any(|message| message.contains("T0") || message.contains("never")),
        "a type the author cannot write must not reach a diagnostic, got: {messages:?}"
    );
}

// ---------------------------------------------------------------------------
// A call argument is a binding site like any other
// ---------------------------------------------------------------------------

const ECHO: &str = "let echo(xs:string[]): string[] = {xs}\n";

#[test]
fn empty_list_takes_its_element_type_from_a_parameter() {
    let source = format!("{ECHO}let value = {{echo({{}})}}");
    assert_clean(&source);
    assert_eq!(binding_type(&source, "value"), Type::array(Type::string()));
}

#[test]
fn braced_list_argument_is_accepted() {
    assert_clean(&format!("{ECHO}let value = {{echo({{\"a\" \"b\"}})}}"));
}

/// A one-item braced argument is a scalar that the parameter's list type coerces, exactly as a
/// property binding does. The brace is an expression escape at arity one, not a list.
#[test]
fn singleton_braced_argument_coerces_to_the_parameter_list() {
    assert_clean(&format!("{ECHO}let value = {{echo({{\"only\"}})}}"));
}

#[test]
fn empty_list_at_a_nullable_list_parameter_is_accepted() {
    assert_clean(&format!(
        "{LINK}let count(links:ChatBrandLink[]?): int = 1\nlet value = {{count({{}})}}"
    ));
}

#[test]
fn braced_values_are_accepted_in_every_argument_position() {
    assert_clean(&format!(
        "{ECHO}let pick(a:string[], b:int, c:string[]): string[] = {{a}}\n\
         let value = {{pick({{}}, 1, {{\"a\" \"b\"}})}}"
    ));
}

/// Record-constructor arguments are not type checked at all, so this pins the gap rather than the
/// empty list.
///
/// <para>`Row({})` at a `string[]` field is accepted — but so is every one of the neighbours below,
/// including a `string` at an `int` field and two arguments to a one-field record. Asserting only
/// the first would be vacuous: it would stay green under an implementation that rejected the empty
/// list everywhere. The gap is pre-existing and out of scope here; the empty list at an argument
/// position is covered against real functions by `braced_values_are_accepted_in_every_argument_position`
/// and `empty_list_at_a_non_list_parameter_reports_only_the_mismatch`. When record-constructor
/// arguments do get checked, this test is expected to fail and should become the real assertions.</para>
#[test]
fn record_constructor_arguments_are_unchecked_including_the_empty_list() {
    assert_clean("type Row = { cells:string[] }\nlet value = {Row({})}");
    assert_clean("type Row = { n:int }\nlet value = {Row({})}");
    assert_clean("type Row = { n:int }\nlet value = {Row(\"x\")}");
    assert_clean("type Row = { n:int }\nlet value = {Row(1, 2)}");
}

/// A parameter that is not a list reports the mismatch, and only the mismatch.
#[test]
fn empty_list_at_a_non_list_parameter_reports_only_the_mismatch() {
    let messages = errors("let f(s:string): int = 1\nlet value = {f({})}");
    assert_eq!(
        messages.len(),
        1,
        "expected only the argument mismatch, got: {messages:?}"
    );
    assert!(
        messages[0].contains("expects string"),
        "expected the mismatch to name the parameter type, got: {messages:?}"
    );
    assert!(
        !messages[0].contains("annotate"),
        "the parameter already declares a type, got: {messages:?}"
    );
}

/// A call that cannot be checked has no parameter type to give, and has already said so.
///
/// Reporting the empty list on top of it would ask the author to fix a second thing that is not
/// wrong: once the call is repaired the argument has a site again.
#[test]
fn an_uncheckable_call_does_not_add_an_element_type_diagnostic() {
    for source in [
        "let value = {nope({})}",
        "let f(xs:string[], n:int): int = 1\nlet value = {f({})}",
    ] {
        let messages = errors(source);
        assert_eq!(
            messages.len(),
            1,
            "expected only the call's own diagnostic, got: {messages:?}"
        );
        assert!(
            !messages[0].contains("element type"),
            "the empty list is not the thing to fix here, got: {messages:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Sites that conclude a binding some other way than by being list-typed
// ---------------------------------------------------------------------------

const BADGE: &str = "external component <Badge />\n";

#[test]
fn braced_value_beside_sibling_body_content_is_accepted() {
    // The content property is the site for every item, not only for a body of one. With a sibling
    // the single-expression path is not taken, and the empty list has to be resolved against the
    // content property anyway.
    let source =
        format!("{BADGE}external component <List content items:object[] />\ncomponent <N /> = {{ <List>{{}}<Badge/></List> }}");
    assert_clean(&source);
}

#[test]
fn an_empty_list_contributes_no_items_to_the_sequence_it_sits_in() {
    // `never` is the identity of the join, so joining it with `Badge` yields `Badge` and the
    // sibling decides the item type on its own. Absorbing the empty list into the join as `object`
    // instead would report a mismatch at a `Badge[]` site.
    let source = format!(
        "{BADGE}external component <List content items:Badge[] />\ncomponent <N /> = {{ <List>{{}}<Badge/></List> }}"
    );
    assert_clean(&source);
}

#[test]
fn empty_list_at_a_site_that_accepts_a_list_without_being_one() {
    // `object` admits a list without declaring an element type, and there is none to declare: a
    // list with no elements has no element type this site can observe. Accepting it keeps `{}`
    // writable wherever `{"a" "b"}` is.
    assert_clean("type Box = { thing:object }\n<Box thing={} />");
    assert_clean("type Box = { thing:object }\n<Box thing={\"a\" \"b\"} />");
}

// ---------------------------------------------------------------------------
// Positions the empty form reaches through control flow
// ---------------------------------------------------------------------------

#[test]
fn empty_list_in_a_condition_arm_takes_its_type_from_the_other_arm() {
    // An arm body declares no type of its own, so what the arms join to is the only expected type
    // it has. An empty list constrains that join not at all.
    assert_clean("let pick(c:boolean): string[] = {if { c => {} else => {\"a\" \"b\"} }}");
}

#[test]
fn empty_list_in_an_if_branch_takes_its_type_from_the_other_branch() {
    assert_clean("let pick(c:boolean): string[] = {if c {\"a\" \"b\"} else {}}");
}

#[test]
fn empty_list_as_a_for_body_is_accepted_where_a_nested_list_is_declared() {
    // A `for` wraps its body in another list, so a body of `{}` is a list of empty lists. That is
    // what a `string[][]` site declares, and the inner element type is unobservable either way.
    assert_clean("let ys:string[] = {\"q\"}\nlet xs:string[][] = {for y in ys {}}");
}

#[test]
fn empty_list_as_a_for_body_at_a_flat_list_site_reports_only_the_mismatch() {
    let source = "let ys:string[] = {\"q\"}\nlet xs:string[] = {for y in ys {}}";
    let messages = errors(source);
    assert_eq!(
        messages.len(),
        1,
        "the site's own mismatch is the whole story, got: {messages:?}"
    );
    assert!(
        messages[0].contains("{}[]"),
        "expected the type to be spelled as the source reads, got: {messages:?}"
    );
    assert!(
        !messages[0].contains("T0")
            && !messages[0].contains("T1")
            && !messages[0].contains("never"),
        "a type the author cannot write must not reach a diagnostic, got: {messages:?}"
    );
}

#[test]
fn an_unannotated_binding_of_a_list_of_empty_lists_is_still_named() {
    // The named diagnostic must survive the empty list being one level down: `mentions_never`
    // looks through the list the `for` builds, so the `never` inside it still reaches the binding.
    let messages = errors("let ys:string[] = {\"q\"}\nlet a = {for y in ys {}}");
    assert!(
        messages.iter().any(|message| message.contains("'a'")),
        "expected the binding to be named, got: {messages:?}"
    );
}

// ---------------------------------------------------------------------------
// A join of empty lists is still empty, and the binding it fixes is still named
// ---------------------------------------------------------------------------

#[test]
fn an_unannotated_function_whose_arms_are_all_empty_is_reported() {
    // Joining `never[]` with `never[]` is `never[]`, so the whole `if` is an empty list and the
    // function's inferred return type mentions `never`. The one diagnostic is at the function,
    // which is the binding an author can annotate.
    let messages = errors("let f(c:boolean) = {if { c => {} else => {} }}");
    assert_eq!(
        messages.len(),
        1,
        "expected exactly one element-type diagnostic, got: {messages:?}"
    );
    assert!(
        messages[0].contains("element type"),
        "expected an element-type diagnostic, got: {messages:?}"
    );
}

#[test]
fn an_unannotated_function_whose_for_body_is_empty_is_reported() {
    let messages = errors("let f(ys:string[]) = {for y in ys {}}");
    assert_eq!(
        messages.len(),
        1,
        "expected exactly one element-type diagnostic, got: {messages:?}"
    );
}

/// `never[]` satisfies every list type it meets, and D3 accepts that: after the diagnostic, one
/// `{}` inhabiting both `string[]` and `int[]` is correct, not a leak. What must not happen is the
/// inferred signature going out *unreported*, so this pins the diagnostic, not the two uses.
#[test]
fn an_empty_list_in_an_inferred_signature_is_reported_at_the_function() {
    let source = concat!(
        "type Box = { items: string[] ns: int[] }\n",
        "let f(c:boolean) = {if { c => {} else => {} }}\n",
        "<Box items={f(true)} ns={f(false)} />"
    );
    let messages = errors(source);
    assert!(
        !messages.is_empty(),
        "one value must not inhabit both `string[]` and `int[]` unreported"
    );
}

#[test]
fn all_empty_alternatives_still_take_the_element_type_from_the_binding() {
    // Reporting at the outer binding must not cost the case where the site does supply a type:
    // the whole `if` is an empty list, and the annotation types it with no diagnostic at all.
    assert_clean("let c:boolean = true\nlet both:string[] = {if c {} else {}}");
    assert_clean("let arms(x:boolean): string[] = {if { x => {} else => {} }}");
}

// ---------------------------------------------------------------------------
// The other arities are unchanged
// ---------------------------------------------------------------------------

#[test]
fn singleton_braced_value_still_infers_a_scalar() {
    let source = "let value = {1}";
    assert_clean(source);
    assert_eq!(binding_type(source, "value"), Type::int());
}

#[test]
fn multi_item_braced_value_still_infers_a_list() {
    let source = "let value = {1 2 3}";
    assert_clean(source);
    assert_eq!(binding_type(source, "value"), Type::array(Type::int()));
}

#[test]
fn scalar_to_list_coercion_at_a_list_typed_site_is_unaffected() {
    assert_clean("let value:string[] = {\"only\"}");
}
