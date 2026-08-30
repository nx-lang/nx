//! Type-checking behavior for unbraced value forms: contextual names and signed literals.
//!
//! One case per scenario in the `unbraced-literal-forms` capability.

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

fn assert_reports(source: &str, needle: &str) {
    let errors = errors(source);
    assert!(
        errors.iter().any(|message| message.contains(needle)),
        "expected an error containing {needle:?}, got: {errors:?}"
    );
}

const FIT: &str = "enum Fit = fill | contain | cover\n";
const LOAD_STATE: &str = "type LoadState = | idle | loading\n";

#[test]
fn bare_name_resolves_to_an_enum_member_at_an_enum_typed_property() {
    assert_clean(&format!("{FIT}type Box = {{ fit: Fit }}\n<Box fit=cover />"));
}

#[test]
fn bare_name_resolves_to_a_payloadless_union_case() {
    assert_clean(&format!(
        "{LOAD_STATE}type V = {{ state: LoadState }}\n<V state=idle />"
    ));
}

#[test]
fn a_lexical_binding_of_the_same_name_does_not_shadow_the_member() {
    assert_clean(&format!(
        "{FIT}let cover = \"something else\"\ntype Box = {{ fit: Fit }}\n<Box fit=cover />"
    ));
}

#[test]
fn nullable_expected_type_accepts_a_bare_name() {
    assert_clean(&format!("{FIT}type Box = {{ fit: Fit? }}\n<Box fit=cover />"));
}

#[test]
fn list_typed_site_accepts_a_bare_name() {
    assert_clean(&format!(
        "{FIT}type Box = {{ fits: Fit[] }}\n<Box fits=cover />"
    ));
}

#[test]
fn qualified_member_access_inside_braces_remains_accepted() {
    assert_clean(&format!(
        "{FIT}type Box = {{ fit: Fit }}\n<Box fit={{Fit.cover}} />"
    ));
}

#[test]
fn property_and_record_defaults_accept_a_bare_name() {
    assert_clean(&format!("{FIT}type Opts = {{ fit: Fit = contain }}\n<Opts />"));
    assert_clean(&format!(
        "{FIT}external component <Img fit:Fit = cover />\nlet v = 1"
    ));
}

#[test]
fn annotated_value_definition_accepts_a_bare_name() {
    assert_clean(&format!("{FIT}let chosen: Fit = cover\nlet v = {{chosen}}"));
}

#[test]
fn match_pattern_accepts_a_bare_name() {
    assert_clean(&format!(
        "{FIT}let label(f: Fit) = {{if f is {{ cover => \"C\" contain => \"N\" fill => \"F\" }}}}\nlet v = {{label(Fit.fill)}}"
    ));
}

#[test]
fn payload_case_is_matchable_by_bare_name_but_not_constructible() {
    // A pattern matches on the discriminator, so a payload case name is a valid pattern.
    assert_clean(
        "type LoadState = | idle | failed { message: string }\n\
         let view(s: LoadState) = {if s is { idle => \"idle\" failed => s.message }}\n\
         let v = {view(LoadState.idle)}",
    );
    // Constructing one still requires the element-style form.
    assert_reports(
        "type LoadState = | idle | failed { message: string }\n\
         type V = { state: LoadState }\n\
         <V state=failed />",
        "requires element-style payload construction",
    );
}

#[test]
fn bare_pattern_from_another_type_is_rejected() {
    assert_reports(
        &format!(
            "{FIT}enum Align = start | center\nlet label(f: Fit) = {{if f is {{ center => \"c\" else => \"\" }}}}\nlet v = {{label(Fit.fill)}}"
        ),
        "is not a member of enum 'Fit'",
    );
}

#[test]
fn nominal_resolution_in_pattern_position_reports_a_displaced_binding() {
    assert_reports(
        &format!(
            "{LOAD_STATE}let idle = \"shadow\"\nlet view(s: LoadState) = {{if s is {{ idle => \"i\" loading => \"l\" }}}}\nlet v = {{view(LoadState.idle)}}"
        ),
        "not as the binding named 'idle'",
    );
}

#[test]
fn quoted_string_at_an_enum_typed_property_is_rejected() {
    assert_reports(
        &format!("{FIT}type Box = {{ fit: Fit }}\n<Box fit=\"cover\" />"),
        "a quoted string is never a member of Fit",
    );
}

#[test]
fn bare_name_at_a_string_typed_property_is_rejected() {
    assert_reports(
        "type Box = { alt: string }\n<Box alt=cover />",
        "a bare name resolves only against an enum or union",
    );
}

#[test]
fn unknown_member_suggests_a_near_match() {
    assert_reports(
        &format!("{FIT}type Box = {{ fit: Fit }}\n<Box fit=containt />"),
        "did you mean `contain`",
    );
}

#[test]
fn unknown_property_does_not_cascade_into_a_contextual_name_error() {
    let messages = errors(&format!("{FIT}type Box = {{ fit: Fit }}\n<Box fitt=cover />"));
    assert!(
        !messages.iter().any(|message| message.contains("'cover'")),
        "the bare name should not be reported when the property is unknown: {messages:?}"
    );
}

#[test]
fn signed_numeric_literals_need_no_braces() {
    assert_clean("type Opts = { x: float64 = -1.0  n: int = -7 }\n<Opts />");
    assert_clean("type Box = { x: float64 }\n<Box x=-1.5 />");
}

#[test]
fn negative_match_pattern_is_accepted() {
    assert_clean(
        "let classify(n: int) = {if n is { -1 => \"neg one\" 0 => \"zero\" else => \"other\" }}\n\
         let v = {classify(-1)}",
    );
}

#[test]
fn binary_subtraction_is_unaffected() {
    assert_clean("let a = {10}\nlet r1 = {a-1}\nlet r2 = {a - 1}\nlet r3 = {-90 + a}");
}
