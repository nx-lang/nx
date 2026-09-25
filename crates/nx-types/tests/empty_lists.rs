//! Type-checking behavior for the empty braced value `{}`.
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
// The empty value satisfies every site that admits zero
// ---------------------------------------------------------------------------

#[test]
fn empty_value_takes_its_type_from_an_annotation() {
    let source = "let value:string* = {}";
    assert_clean(source);
    assert_eq!(
        binding_type(source, "value"),
        Type::zero_or_more(Type::string())
    );
}

#[test]
fn empty_value_is_accepted_at_an_optional_property_site() {
    assert_clean("type Fit = fill | contain | cover\ntype Img = { fits?:Fit+ }\n<Img fits={} />");
}

#[test]
fn empty_value_is_not_a_field_default() {
    // A field that may be empty is marked optional on its name, and an optional field has no
    // default: omitting it binds the empty value. Writing `{}` as the default of a `+` field is
    // the mismatch it looks like.
    assert_clean(&format!(
        "{LINK}type Brand = {{ links?:ChatBrandLink+ }}\n<Brand />"
    ));
    let errors = errors(&format!(
        "{LINK}type Brand = {{ links:ChatBrandLink+ = {{}} }}"
    ));
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("ChatBrandLink+") && errors[0].contains("found {}"),
        "{errors:?}"
    );
}

#[test]
fn empty_value_is_not_a_component_property_default() {
    assert_clean("external component <C xs?:string+ />\nlet c = <C />");
    let errors = errors("external component <C xs:string+ = {} />");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("string+") && errors[0].contains("found {}"),
        "{errors:?}"
    );
}

#[test]
fn empty_value_is_accepted_as_element_body_content() {
    assert_clean(
        "external component <List content items?:object+ />\ncomponent <N /> = { <List>{}</List> }",
    );
    // A body was written, so the content property does not fall back to its default: `{}` is
    // bound, and a `+` property rejects it.
    let errors = errors("type Def = { content items:string+ = {\"d\"} }\nlet d = <Def>{}</Def>");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("string+") && errors[0].contains("found {}"),
        "{errors:?}"
    );
}

#[test]
fn empty_value_at_an_optional_sequence_site_is_the_same_as_omitting_it() {
    // There is no absent sequence distinct from the empty one: `links={}` and no `links` at all
    // both leave the field holding the empty value.
    let source = format!("{LINK}type Brand = {{ links?:ChatBrandLink+ }}\n<Brand links={{}} />");
    assert_clean(&source);
}

#[test]
fn empty_value_is_accepted_as_a_function_body_with_a_declared_return_type() {
    // A function body is a values brace too, so the empty form reaches it. It is accepted where
    // the return type admits zero.
    assert_clean("let f():string* = {}");
    assert_clean("let g():string? = {}");
}

#[test]
fn empty_function_body_with_no_declared_return_type_is_reported() {
    // This source was a parse error before `{}` was admitted. It is still rejected, now by the
    // item-type rule rather than by the parser.
    let errors = errors("let <f /> = { }");
    assert!(
        errors.iter().any(|message| message.contains("item type")),
        "expected an item-type diagnostic, got: {errors:?}"
    );
}

// ---------------------------------------------------------------------------
// No expected type is an error, not `object*`
// ---------------------------------------------------------------------------

#[test]
fn empty_value_with_no_expected_type_is_reported() {
    let source = "let value = {}";
    let errors = errors(source);
    assert!(
        errors
            .iter()
            .any(|message| message.contains("item type") && message.contains("'value'")),
        "expected an item-type diagnostic naming the binding, got: {errors:?}"
    );
}

#[test]
fn empty_value_with_no_expected_type_is_the_empty_type() {
    // Asserted explicitly: an `object*` fallback would silently pass a test that only checked for
    // the absence of errors, and would let the value flow to any sequence-typed site.
    let source = "let value = {}";
    assert_eq!(
        binding_type(source, "value"),
        Type::empty(),
        "an empty value with no expected type is the empty type, not object*"
    );
}

/// A binding that is annotated with an exactly-one type reports one thing: the mismatch.
///
/// The item-type diagnostic is for a site that supplies no expected type. Here the site supplied
/// one and it admits no zero, so the annotation is not the thing to change, and telling the author
/// to write an annotation they already wrote points at the wrong half of the line.
#[test]
fn an_annotated_exactly_one_binding_reports_only_the_mismatch() {
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

/// The mismatch names the empty value as the author wrote it.
///
/// The empty type is the bottom item type under the zero-or-one occurrence, and the bottom type
/// has no source spelling, so rendering it by its structure would put a name the author cannot
/// write in front of someone who wrote `{}`.
#[test]
fn a_mismatch_on_an_empty_value_renders_it_as_written() {
    let source = "let value:string = {}";
    let messages = errors(source);
    assert!(
        messages.iter().any(|message| message.contains("found {}")),
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

const ECHO: &str = "let echo(xs?:string+): string* = {xs}\n";
const ECHO_PLUS: &str = "let echo(xs:string+): string+ = {xs}\n";

#[test]
fn empty_value_takes_its_type_from_a_parameter() {
    let source = format!("{ECHO}let value = {{echo({{}})}}");
    assert_clean(&source);
    assert_eq!(
        binding_type(&source, "value"),
        Type::zero_or_more(Type::string())
    );
}

#[test]
fn braced_list_argument_is_accepted() {
    assert_clean(&format!("{ECHO_PLUS}let value = {{echo({{\"a\" \"b\"}})}}"));
}

/// A one-item braced argument is an exactly-one value that the parameter's `+` type lifts, exactly
/// as a property binding does. The brace is an expression escape at arity one, not a sequence.
#[test]
fn singleton_braced_argument_lifts_to_the_parameter_sequence() {
    assert_clean(&format!("{ECHO_PLUS}let value = {{echo({{\"only\"}})}}"));
}

#[test]
fn empty_value_at_an_optional_sequence_parameter_is_accepted() {
    assert_clean(&format!(
        "{LINK}let count(links?:ChatBrandLink+): int = 1\nlet value = {{count({{}})}}"
    ));
}

#[test]
fn braced_values_are_accepted_in_every_argument_position() {
    assert_clean(&format!(
        "{ECHO_PLUS}let pick(b:int, c:string+, a?:string+): string* = {{a}}\n\
         let value = {{pick(1, {{\"a\" \"b\"}}, {{}})}}"
    ));
}

/// Record-constructor arguments are not type checked at all, so this pins the gap rather than the
/// empty value.
///
/// <para>`Row({})` at a `string+` field is accepted — but so is every one of the neighbours below,
/// including a `string` at an `int` field and two arguments to a one-field record. Asserting only
/// the first would be vacuous: it would stay green under an implementation that rejected the empty
/// value everywhere. The gap is pre-existing and out of scope here; the empty value at an argument
/// position is covered against real functions by `braced_values_are_accepted_in_every_argument_position`
/// and `empty_value_at_an_exactly_one_parameter_reports_only_the_mismatch`. When record-constructor
/// arguments do get checked, this test is expected to fail and should become the real assertions.</para>
#[test]
fn record_constructor_arguments_are_unchecked_including_the_empty_value() {
    assert_clean("type Row = { cells:string+ }\nlet value = {Row({})}");
    assert_clean("type Row = { n:int }\nlet value = {Row({})}");
    assert_clean("type Row = { n:int }\nlet value = {Row(\"x\")}");
    assert_clean("type Row = { n:int }\nlet value = {Row(1, 2)}");
}

/// A parameter that admits no zero reports the mismatch, and only the mismatch.
#[test]
fn empty_value_at_an_exactly_one_parameter_reports_only_the_mismatch() {
    let messages = errors("let f(s:string): int = 1\nlet value = {f({})}");
    assert_eq!(
        messages.len(),
        1,
        "expected only the argument mismatch, got: {messages:?}"
    );
    assert!(
        messages[0].contains("expects string") && messages[0].contains("found {}"),
        "expected the mismatch to name the parameter type and the value, got: {messages:?}"
    );
    assert!(
        !messages[0].contains("annotate"),
        "the parameter already declares a type, got: {messages:?}"
    );
}

/// A call that cannot be checked has no parameter type to give, and has already said so.
///
/// Reporting the empty value on top of it would ask the author to fix a second thing that is not
/// wrong: once the call is repaired the argument has a site again.
#[test]
fn an_uncheckable_call_does_not_add_an_item_type_diagnostic() {
    for source in [
        "let value = {nope({})}",
        "let f(xs:string+, n:int): int = 1\nlet value = {f({})}",
    ] {
        let messages = errors(source);
        assert_eq!(
            messages.len(),
            1,
            "expected only the call's own diagnostic, got: {messages:?}"
        );
        assert!(
            !messages[0].contains("item type"),
            "the empty value is not the thing to fix here, got: {messages:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Sites that conclude a binding some other way than by admitting zero
// ---------------------------------------------------------------------------

const BADGE: &str = "external component <Badge />\n";

#[test]
fn braced_value_beside_sibling_body_content_is_accepted() {
    // The content property is the site for every item, not only for a body of one. With a sibling
    // the single-expression path is not taken, and the empty value has to be resolved against the
    // content property anyway.
    let source =
        format!("{BADGE}external component <List content items:object+ />\ncomponent <N /> = {{ <List>{{}}<Badge/></List> }}");
    assert_clean(&source);
}

#[test]
fn an_empty_value_contributes_no_items_to_the_sequence_it_sits_in() {
    // The empty type contributes no item type and admits zero, so joining it with `Badge` yields
    // `Badge` and the sibling decides the item type and the occurrence on its own. Absorbing the
    // empty value into the join as `object` instead would report a mismatch at a `Badge+` site.
    let source = format!(
        "{BADGE}external component <List content items:Badge+ />\ncomponent <N /> = {{ <List>{{}}<Badge/></List> }}"
    );
    assert_clean(&source);
}

#[test]
fn empty_value_does_not_satisfy_object() {
    // `object` is exactly one value. The empty value admits zero, so it does not satisfy `object`;
    // it satisfies `object?`. A sequence does not satisfy `object` either: `object` admits a
    // sequence only as an opaque held value, never by a `+` value being lifted into it.
    let errors = errors("type Box = { thing:object }\n<Box thing={} />");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("expects object") && errors[0].contains("found {}"),
        "{errors:?}"
    );
    assert_clean("type Maybe = { thing?:object }\n<Maybe thing={} />");

    let errors = self::errors("type Box = { thing:object }\n<Box thing={\"a\" \"b\"} />");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("expects object") && errors[0].contains("found string+"),
        "{errors:?}"
    );
}

// ---------------------------------------------------------------------------
// Positions the empty form reaches through control flow
// ---------------------------------------------------------------------------

#[test]
fn empty_value_in_a_condition_arm_takes_its_type_from_the_other_arm() {
    // An arm body declares no type of its own, so what the arms join to is the only expected type
    // it has. An empty value contributes no item type, and admits zero: `{}` with `string+` is
    // `string*`.
    assert_clean("let pick(c:boolean): string* = {if { c => {} else => {\"a\" \"b\"} }}");
    let errors = errors("let pick(c:boolean): string+ = {if { c => {} else => {\"a\" \"b\"} }}");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("expects string+") && errors[0].contains("found string*"),
        "{errors:?}"
    );
}

#[test]
fn empty_value_in_an_if_branch_takes_its_type_from_the_other_branch() {
    assert_clean("let pick(c:boolean): string* = {if c {\"a\" \"b\"} else {}}");
}

#[test]
fn empty_value_as_a_for_body_yields_the_empty_value() {
    // A `for` concatenates what its body yields, so a body of `{}` yields nothing at all. The
    // result is `never*`, which satisfies every `*` type.
    assert_clean("let ys:string+ = {\"q\"}\nlet xs:string* = {for y in ys {}}");
}

#[test]
fn empty_value_as_a_for_body_still_needs_an_annotation_to_name_its_item_type() {
    let source = "let ys:string+ = {\"q\"}\nlet a = {for y in ys {}}";
    let messages = errors(source);
    assert_eq!(
        messages.len(),
        1,
        "the missing annotation is the whole story, got: {messages:?}"
    );
    assert!(
        !messages[0].contains("T0")
            && !messages[0].contains("T1")
            && !messages[0].contains("never"),
        "a type the author cannot write must not reach a diagnostic, got: {messages:?}"
    );
}

#[test]
fn an_unannotated_binding_of_a_for_over_empty_bodies_is_still_named() {
    // The named diagnostic must survive the empty value being one level down: the `for` multiplies
    // the body's occurrence, and the bottom item type inside what it builds still reaches the
    // binding.
    let messages = errors("let ys:string+ = {\"q\"}\nlet a = {for y in ys {}}");
    assert!(
        messages.iter().any(|message| message.contains("'a'")),
        "expected the binding to be named, got: {messages:?}"
    );
}

// ---------------------------------------------------------------------------
// A join of empty values is still empty, and the binding it fixes is still named
// ---------------------------------------------------------------------------

#[test]
fn an_unannotated_function_whose_arms_are_all_empty_is_reported() {
    // Joining `{}` with `{}` is `{}`, so the whole `if` is the empty value and the function's
    // inferred return type has no item type. The one diagnostic is at the function, which is the
    // binding an author can annotate.
    let messages = errors("let f(c:boolean) = {if { c => {} else => {} }}");
    assert_eq!(
        messages.len(),
        1,
        "expected exactly one item-type diagnostic, got: {messages:?}"
    );
    assert!(
        messages[0].contains("item type") && messages[0].contains("'f'"),
        "expected an item-type diagnostic naming the function, got: {messages:?}"
    );
}

#[test]
fn an_unannotated_function_whose_for_body_is_empty_is_reported() {
    let messages = errors("let f(ys:string+) = {for y in ys {}}");
    assert_eq!(
        messages.len(),
        1,
        "expected exactly one item-type diagnostic, got: {messages:?}"
    );
    assert!(messages[0].contains("'f'"), "{messages:?}");
}

/// The empty value satisfies every `?` and `*` type it meets, and that is accepted: after the
/// diagnostic, one `{}` inhabiting both `string*` and `int*` is correct, not a leak. What must not
/// happen is the inferred signature going out *unreported*, so this pins the diagnostic, not the
/// two uses.
#[test]
fn an_empty_value_in_an_inferred_signature_is_reported_at_the_function() {
    let source = concat!(
        "type Box = { items?: string+ ns?: int+ }\n",
        "let f(c:boolean) = {if { c => {} else => {} }}\n",
        "<Box items={f(true)} ns={f(false)} />"
    );
    let messages = errors(source);
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert!(
        messages[0].contains("'f'"),
        "one value must not inhabit both `string*` and `int*` unreported: {messages:?}"
    );
}

#[test]
fn all_empty_alternatives_still_take_the_type_from_the_binding() {
    // Reporting at the outer binding must not cost the case where the site does supply a type:
    // the whole `if` is the empty value, and any annotation that admits zero types it with no
    // diagnostic at all.
    assert_clean("let c:boolean = true\nlet both:string* = {if c {} else {}}");
    assert_clean("let c:boolean = true\nlet one:string? = {if c {} else {}}");
    assert_clean("let arms(x:boolean): string* = {if { x => {} else => {} }}");
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
fn multi_item_braced_value_still_infers_one_or_more() {
    let source = "let value = {1 2 3}";
    assert_clean(source);
    assert_eq!(
        binding_type(source, "value"),
        Type::one_or_more(Type::int())
    );
}

#[test]
fn scalar_lift_at_a_sequence_typed_site_is_unaffected() {
    assert_clean("let value:string+ = {\"only\"}");
}
