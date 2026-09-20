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

const FIT: &str = "type Fit = fill | contain | cover\n";
const LOAD_STATE: &str = "type LoadState = idle | loading\n";

#[test]
fn bare_name_resolves_to_a_case_at_a_union_typed_property() {
    assert_clean(&format!(
        "{FIT}type Box = {{ fit: Fit }}\n<Box fit=cover />"
    ));
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
    assert_clean(&format!(
        "{FIT}type Box = {{ fit: Fit? }}\n<Box fit=cover />"
    ));
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
    assert_clean(&format!(
        "{FIT}type Opts = {{ fit: Fit = contain }}\n<Opts />"
    ));
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
        "type LoadState = idle | failed { message: string }\n\
         let view(s: LoadState) = {if s is { idle => \"idle\" failed => s.message }}\n\
         let v = {view(LoadState.idle)}",
    );
    // Constructing one still requires the element-style form.
    assert_reports(
        "type LoadState = idle | failed { message: string }\n\
         type V = { state: LoadState }\n\
         <V state=failed />",
        "requires element-style payload construction",
    );
}

#[test]
fn bare_pattern_from_another_type_is_rejected() {
    assert_reports(
        &format!(
            "{FIT}type Align = start | center\nlet label(f: Fit) = {{if f is {{ center => \"c\" else => \"\" }}}}\nlet v = {{label(Fit.fill)}}"
        ),
        "is not a case of union 'Fit'",
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
fn quoted_string_at_a_union_typed_property_is_rejected() {
    assert_reports(
        &format!("{FIT}type Box = {{ fit: Fit }}\n<Box fit=\"cover\" />"),
        "a quoted string is never a member of Fit",
    );
}

#[test]
fn bare_name_at_a_string_typed_property_is_rejected() {
    assert_reports(
        "type Box = { alt: string }\n<Box alt=cover />",
        "a bare name resolves only against a union's cases",
    );
}

/// The fix the message names has to be one the site accepts. An author who writes `let span = 1..=5`
/// meets this diagnostic first, and being sent to `"span"` at a numeric site only produces a second
/// error.
#[test]
fn a_visible_binding_is_offered_in_its_braced_form() {
    let messages = errors("let r = 5\nlet x:int = r\n");
    assert!(
        messages.iter().any(|message| message.contains("write {r}")),
        "expected the braced form to be suggested, got: {messages:?}"
    );
    assert!(
        !messages.iter().any(|message| message.contains("\"r\"")),
        "an int site does not take a string, so the quoted form must not be offered: {messages:?}"
    );
}

#[test]
fn the_quoted_form_is_offered_only_where_a_string_fits() {
    assert_reports("let x:string = hello\n", "for a string value write \"hello\"");

    let numeric = errors("let x:int = hello\n");
    assert!(
        !numeric.iter().any(|message| message.contains("for a string value")),
        "a string does not satisfy an int site: {numeric:?}"
    );

    let record = errors("type P = { a:int }\nlet p = <P a={1} />\nlet q:P = p\n");
    assert!(
        record.iter().any(|message| message.contains("write {p}")),
        "a record site with a visible binding should name the braced form: {record:?}"
    );
    assert!(
        !record.iter().any(|message| message.contains("for a string value")),
        "a record site does not take a string: {record:?}"
    );
}

/// A binding is only a fix where its own type fits: `let r = 5` does not make `r` a string, so a
/// `string` site is sent to the quoted form rather than to `{r}`, which it would reject in turn.
#[test]
fn a_visible_binding_of_the_wrong_type_is_not_offered() {
    let messages = errors("let r = 5\nlet x:string = r\n");
    assert!(
        messages
            .iter()
            .any(|message| message.contains("for a string value write \"r\"")),
        "a string site should be offered the quoted form, got: {messages:?}"
    );
    assert!(
        !messages.iter().any(|message| message.contains("write {r}")),
        "an int binding does not satisfy a string site, so the braced form must not be offered: {messages:?}"
    );

    let function = errors("let f() = 5\nlet x:int = f\n");
    assert!(
        !function.iter().any(|message| message.contains("write {f}")),
        "a function value does not satisfy an int site: {function:?}"
    );
}

/// Neither form fits and no binding is visible, and the message still names what the site takes:
/// stopping at "a bare name resolves only against a union's cases" leaves the author nowhere.
#[test]
fn a_site_with_nothing_to_point_at_still_names_the_accepted_form() {
    assert_reports(
        "let x:int = nosuch\n",
        "a value here is written as a literal, or in braces as `{...}`",
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
    let messages = errors(&format!(
        "{FIT}type Box = {{ fit: Fit }}\n<Box fitt=cover />"
    ));
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

// ---------------------------------------------------------------------------------------------
// A type-parameter site resolves a bare name against the visible type names
// ---------------------------------------------------------------------------------------------

#[test]
fn bare_name_resolves_to_a_visible_type_at_a_type_parameter_site() {
    // A parameter named like the type is in lexical scope at the use site, and a type parameter
    // site still reads the type. (A second top-level `Contact` is not the way to shadow it: a
    // module's top-level names are unique.)
    assert_clean(
        "type Contact = { name:string }\n\
         external component <List TItem:type items:TItem[]? />\n\
         let v(Contact:string) = <List TItem=Contact />",
    );
}

#[test]
fn unknown_type_name_at_a_type_parameter_site_suggests_a_near_match() {
    let errors = errors(
        "type Contact = { name:string }\n\
         external component <List TItem:type />\n\
         let v = <List TItem=Contatc />",
    );
    assert!(
        errors.iter().any(|message| {
            message.contains("'TItem'")
                && message.contains("expects a type name")
                && message.contains("did you mean `Contact`")
        }),
        "expected the type-name diagnostic with a suggestion, got: {errors:?}"
    );
}

/// Every numeric literal of the analyzed module with the type recorded for it.
fn numeric_literals(source: &str) -> Vec<(nx_hir::ast::Literal, String)> {
    use nx_hir::ast::{Expr, Literal};

    let checked = check_str(source, "test.nx");
    assert!(checked.errors().is_empty(), "{:?}", errors(source));
    let module = checked.lowered_module.as_ref().expect("lowered module");
    module
        .exprs()
        .filter_map(|(id, expr)| match expr {
            Expr::Literal(
                literal @ (Literal::Int(_)
                | Literal::Int32(_)
                | Literal::Float(_)
                | Literal::Float32(_)),
            ) => Some((
                literal.clone(),
                checked
                    .type_env
                    .get_expr_type(id)
                    .map(|ty| ty.to_string())
                    .unwrap_or_default(),
            )),
            _ => None,
        })
        .collect()
}

#[test]
fn a_real_literal_binds_at_a_float32_site_as_float32() {
    use nx_hir::ast::{Literal, OrderedFloat};

    assert_eq!(
        numeric_literals("external component <B v:float32 />\nlet root() = { <B v=1.5 /> }"),
        vec![(Literal::Float32(OrderedFloat(1.5)), "float32".to_string())]
    );
}

#[test]
fn an_integer_literal_binds_at_a_float32_property_as_the_literal_a_real_one_is_there() {
    let converted =
        numeric_literals("external component <B v:float32 />\nlet root() = { <B v=1 /> }");
    let written =
        numeric_literals("external component <B v:float32 />\nlet root() = { <B v=1.0 /> }");

    assert_eq!(
        converted, written,
        "the two spellings stay indistinguishable"
    );
    assert_eq!(converted[0].1, "float32");
}
