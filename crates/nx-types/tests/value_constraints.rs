//! Type checking of value constraints, `float32 where in 0.0..=1.0`, and of numeric match patterns.
//!
//! One case per type-checking scenario in the `value-constraints` capability. Parsing is covered in
//! `nx-syntax`, the checks the emitter writes in `nx-codegen`, and evaluation in the runtimes.

use nx_diagnostics::Severity;
use nx_hir::Item;
use nx_types::{check_str, Type};

/// Each diagnostic of `source` as its code, its severity, its message and the text it points at.
fn diagnostics(source: &str) -> Vec<(String, Severity, String, String)> {
    check_str(source, "test.nx")
        .all_diagnostics()
        .iter()
        .map(|diagnostic| {
            let at = diagnostic
                .labels()
                .first()
                .and_then(|label| source.get(std::ops::Range::<usize>::from(label.range)))
                .unwrap_or_default()
                .to_string();
            (
                diagnostic.code().unwrap_or_default().to_string(),
                diagnostic.severity(),
                diagnostic.message().to_string(),
                at,
            )
        })
        .collect()
}

fn assert_clean(source: &str) {
    let found = diagnostics(source);
    assert!(found.is_empty(), "`{source}` reported {found:#?}");
}

/// The one diagnostic with `code` in `source`, which must point at `at`.
fn reported(source: &str, code: &str, at: &str) -> String {
    let found = diagnostics(source);
    let matching = found
        .iter()
        .filter(|(found_code, ..)| found_code == code)
        .collect::<Vec<_>>();
    assert_eq!(matching.len(), 1, "`{source}`: {found:#?}");
    assert_eq!(matching[0].3, at, "`{source}`: {found:#?}");
    matching[0].2.clone()
}

/// The type of the body of the function named `name` in `source`, which must check cleanly.
fn body_type(source: &str, name: &str) -> Type {
    let checked = check_str(source, "test.nx");
    let messages: Vec<_> = checked
        .errors()
        .iter()
        .map(|diagnostic| diagnostic.message().to_string())
        .collect();
    assert!(messages.is_empty(), "`{source}` reported {messages:?}");
    let module = checked.lowered_module.as_ref().expect("lowered module");
    let body = module
        .items()
        .iter()
        .find_map(|item| match item {
            Item::Function(function) if function.name.as_str() == name => Some(function.body),
            _ => None,
        })
        .unwrap_or_else(|| panic!("a function named {name}"));
    checked
        .type_env
        .get_expr_type(body)
        .cloned()
        .expect("a type for the body")
}

const RATIO: &str = "type Ratio = float32 where in 0.0..=1.0\n";

// ---------------------------------------------------------------------------
// Where a constraint is written
// ---------------------------------------------------------------------------

#[test]
fn a_constrained_field_takes_a_default_after_its_constraint() {
    assert_clean(
        "type Paint = { opacity: float64 where in 0.0..=1.0 = 1.0  layer: int }\n\
         let p = <Paint layer=1 />",
    );
}

#[test]
fn a_named_constrained_type() {
    assert_clean(&format!(
        "{RATIO}type Fade = {{ amount: Ratio }}\nlet f = <Fade amount=0.5 />"
    ));
    reported(
        &format!("{RATIO}type Fade = {{ amount: Ratio }}\nlet f = <Fade amount=1.5 />"),
        "constraint-violation",
        "1.5",
    );
}

#[test]
fn parameters_results_and_annotations() {
    assert_clean(
        "let fade(amount: float32 where in 0.0..=1.0, steps: int where >= 1): int where >= 0 = { steps }\n\
         let half: float64 where <= 1.0 = 0.5",
    );
}

#[test]
fn a_field_may_be_named_where_or_value() {
    assert_clean(
        "type Slider = { value: float64 where in 0.0..=1.0  where: string }\n\
         type Query = { limit: int  where: string  order?: string }\n\
         let <Filter count: int where?: string /> = <p />",
    );
}

// ---------------------------------------------------------------------------
// Patterns
// ---------------------------------------------------------------------------

#[test]
fn terms_intersect() {
    let unit = "type Unit = float64 where >= 0.0 && < 1.0\n";
    assert_clean(&format!("{unit}let a: Unit = 0.0\nlet b: Unit = 0.5"));
    reported(
        &format!("{unit}let c: Unit = 1.0"),
        "constraint-violation",
        "1.0",
    );
}

#[test]
fn a_negative_bound() {
    assert_clean("type Pan = float32 where in -1.0..=1.0\nlet p: Pan = -0.5");
}

#[test]
fn an_unsupported_pattern_is_named_not_a_parse_error() {
    let source = "type Edge = int where < 0 || > 9";
    let message = reported(source, "invalid-constraint", "||");
    for term in ["`in a..b`", "`<`", "`<=`", "`>`", "`>=`", "`&&`"] {
        assert!(message.contains(term), "{term} in {message}");
    }
    assert!(
        diagnostics(source)
            .iter()
            .all(|(code, ..)| code == "invalid-constraint"),
        "no parse error: {:#?}",
        diagnostics(source)
    );
}

#[test]
fn a_comma_in_a_constraint_is_named_not_a_parse_error() {
    for source in [
        "type Edge = int where < 0, > 9",
        "type Box = { a: int where < 0, > 9 }",
        "let edge: int where < 0, > 9 = 5",
        "let f(x: int where < 0, > 9) = { x }",
        "type Edge = int where < 0,\n  > 9",
    ] {
        let message = reported(source, "invalid-constraint", ",");
        assert!(message.contains("one interval"), "{message}");
        assert!(
            diagnostics(source)
                .iter()
                .all(|(code, ..)| code != "syntax-error"),
            "`{source}` gave a bare parse error: {:#?}",
            diagnostics(source)
        );
    }
}

#[test]
fn a_default_is_not_typed_by_a_top_level_value_sharing_a_field_name() {
    // The default reads the sibling `a`, an `int`, not the top-level `string`.
    assert_clean("let a: string = \"x\"\ntype Box = { a: int  c: int = {a} }");
    assert_clean("let a: string = \"x\"\ntype Shape =\n  | dot { a: int  c: int = {a} }");
    // A field the case inherits from the union's abstract base.
    assert_clean(
        "let a: string = \"x\"\nabstract type Base = { a: int }\n\
         type Shape extends Base =\n  | dot { c: int = {a} }",
    );
}

#[test]
fn a_violation_spells_the_sites_resolved_interval() {
    let message = reported(
        "type Small = int where < 10\nlet s: Small = 10",
        "constraint-violation",
        "10",
    );
    assert!(message.contains("`s`"), "{message}");
    assert!(message.contains("`int where <= 9`"), "{message}");
}

#[test]
fn a_separator_comma_after_a_constraint_is_a_plain_syntax_error() {
    for source in [
        "type Box = { a: int where >= 0, b: int }",
        "type Box = { a: int where in 0..=9, b: string }",
        "type Box = { a: int where in 0..=9, }",
        "let f(x: int where >= 0,) = { x }",
        "let f(x: int where >= 0, , y: int) = { x }",
        "type Box = {\n  a: int where >= 0,\n  b: int\n}",
    ] {
        let found = diagnostics(source);
        assert!(
            found.iter().all(|(code, ..)| code != "invalid-constraint"),
            "`{source}` named a separator as alternatives: {found:#?}"
        );
        assert!(
            found.iter().any(|(code, ..)| code == "syntax-error"),
            "`{source}` gave no syntax error: {found:#?}"
        );
    }
    // Alternatives that start with `in`, a number or a negative number are still named.
    for source in [
        "type Edge = int where < 0, in 5..=9",
        "type Edge = int where < 0, -1..=2",
    ] {
        let found = diagnostics(source);
        assert!(
            found.iter().any(|(code, ..)| code == "invalid-constraint"),
            "`{source}`: {found:#?}"
        );
    }
}

#[test]
fn a_constraint_empty_only_at_its_bases_limits_is_reported() {
    for (source, at) in [
        ("type Under = int32 where < -2147483648", "< -2147483648"),
        ("type Over = int32 where > 2147483647", "> 2147483647"),
        (
            "type Huge = int where > 9007199254740991",
            "> 9007199254740991",
        ),
        (
            "type Gap = float32 where > 0.99999994 && < 1.0",
            "> 0.99999994 && < 1.0",
        ),
    ] {
        reported(source, "empty-constraint", at);
    }
    // The base's own limits narrow only the emptiness test, not the interval.
    assert_clean("type Small = int32 where >= 0\nlet s: Small = 2147483647");
    assert_clean("type Edge = int32 where > 2147483646\nlet e: Edge = 2147483647");
    assert_clean("type Below = float32 where > 0.99999994 && <= 1.0\nlet b: Below = 1.0");
}

#[test]
fn an_integer_bound_a_float32_cannot_hold_is_rejected() {
    let message = reported(
        "type Big = float32 where <= 16777217",
        "invalid-constraint",
        "16777217",
    );
    assert!(message.contains("`16777216`"), "{message}");
    assert_clean("type Fits = float32 where <= 16777216");
}

#[test]
fn a_float32_constraint_is_spelled_as_written() {
    let message = reported(
        "type Tenth = float32 where in 0.0..=0.1\nlet t: Tenth = 0.2",
        "constraint-violation",
        "0.2",
    );
    assert_eq!(
        message,
        "`0.2` is outside the constraint for `t`: `float32 where in 0.0..=0.1`"
    );
}

#[test]
fn an_optional_constrained_property_fix_names_the_alias() {
    let message = reported(
        &format!("{RATIO}type Paint = {{ r: Ratio? }}"),
        "optional-in-type-slot",
        "r: Ratio?",
    );
    assert!(message.contains("`r?:Ratio`"), "{message}");
}

#[test]
fn a_leading_operand_gets_a_fix() {
    let message = reported(
        "type Count = int where value >= 1",
        "invalid-constraint",
        "value >= 1",
    );
    assert!(message.contains("`int where >= 1`"), "{message}");
}

// ---------------------------------------------------------------------------
// What may be constrained
// ---------------------------------------------------------------------------

#[test]
fn a_real_bound_on_an_integer() {
    reported("type Half = int where <= 0.5", "invalid-constraint", "0.5");
}

#[test]
fn a_bound_outside_the_base_range() {
    let message = reported(
        "type Big = int32 where <= 3000000000",
        "invalid-constraint",
        "3000000000",
    );
    assert!(message.contains("int32"), "{message}");
}

#[test]
fn an_int64_bound_outside_the_safe_range() {
    reported(
        "type Huge = int64 where <= 9007199254740992",
        "invalid-constraint",
        "9007199254740992",
    );
    assert_clean("type Safe = int64 where <= 9007199254740991");
}

#[test]
fn a_constraint_on_a_sequence_suggests_naming_the_item() {
    let message = reported(
        "type Scores = int+ where >= 0",
        "invalid-constraint",
        ">= 0",
    );
    assert!(
        message.contains("`type Score = int where >= 0`") && message.contains("`Score+`"),
        "{message}"
    );
}

#[test]
fn a_constraint_through_an_alias_with_an_occurrence() {
    let found = diagnostics("type Many = int+\ntype Bad = Many where >= 0");
    assert!(
        found.iter().any(|(code, ..)| code == "invalid-constraint"),
        "{found:#?}"
    );
}

#[test]
fn integer_exclusive_ends_are_inclusive_ends() {
    let source = "type A = int where > 0\ntype B = int where >= 1\n\
                  let f(a: A) = { a }\nlet g(b: B) = { f(b) }";
    assert_clean(source);
    assert_eq!(body_type(source, "g").to_string(), "int where >= 1");
}

#[test]
fn a_constraint_on_a_string() {
    let message = reported(
        "type Name = string where >= 1",
        "invalid-constraint",
        ">= 1",
    );
    assert!(message.contains("numeric types"), "{message}");
}

#[test]
fn a_constrained_type_argument() {
    let found = diagnostics(&format!(
        "{RATIO}type Span = {{ T: type low: T high: T }}\ntype Band = {{ r: <Span T=Ratio/> }}"
    ));
    assert!(
        found.iter().any(|(code, ..)| code == "invalid-constraint"),
        "{found:#?}"
    );
}

#[test]
fn a_constraint_inside_a_function_type() {
    let found =
        diagnostics("type Row = { render: <function value: float64 where >= 0.0 />: string }");
    assert!(
        found.iter().any(|(code, ..)| code == "invalid-constraint"),
        "{found:#?}"
    );
}

#[test]
fn an_empty_constraint() {
    reported(
        "type None = int where > 5 && < 6",
        "empty-constraint",
        "> 5 && < 6",
    );
}

// ---------------------------------------------------------------------------
// A constrained type is its base type
// ---------------------------------------------------------------------------

#[test]
fn a_constrained_value_passes_where_its_base_is_expected() {
    assert_clean(&format!(
        "{RATIO}let wide(x: float64) = {{ x }}\nlet f(r: Ratio) = {{ wide(r) }}"
    ));
}

#[test]
fn a_narrower_interval_satisfies_a_wider_one() {
    assert_clean(&format!(
        "{RATIO}let g(x: float64 where >= 0.0) = {{ x }}\nlet f(r: Ratio) = {{ g(r) }}"
    ));
}

#[test]
fn arithmetic_yields_the_base_type() {
    let source = format!("{RATIO}let f(r: Ratio) = {{ r * 2 }}");
    assert_eq!(body_type(&source, "f"), Type::float32());
}

#[test]
fn a_join_is_the_narrowest_type_every_branch_satisfies() {
    let source = format!(
        "{RATIO}type Low = Ratio where <= 0.5\n\
         let a(c: boolean, x: Ratio, y: Ratio) = {{ if c {{ x }} else {{ y }} }}\n\
         let b(c: boolean, x: Ratio, n: float32) = {{ if c {{ x }} else {{ n }} }}\n\
         let lowered(c: boolean, x: Ratio, l: Low) = {{ if c {{ x }} else {{ l }} }}\n\
         let apart(c: boolean, l: Low, h: float32 where >= 0.75) = {{ if c {{ l }} else {{ h }} }}"
    );
    assert_eq!(
        body_type(&source, "a").to_string(),
        "float32 where in 0.0..=1.0"
    );
    assert_eq!(body_type(&source, "b"), Type::float32());
    assert_eq!(
        body_type(&source, "lowered").to_string(),
        "float32 where in 0.0..=1.0"
    );
    assert_eq!(body_type(&source, "apart"), Type::float32());
}

#[test]
fn an_optional_constrained_property() {
    let fade = format!("{RATIO}type Fade = {{ amount?: Ratio }}\n");
    assert_clean(&format!("{fade}let a = <Fade />"));
    reported(
        &format!("{fade}let b = <Fade amount=2.0 />"),
        "constraint-violation",
        "2.0",
    );
}

#[test]
fn an_alias_adds_a_term() {
    let low = format!("{RATIO}type Low = Ratio where <= 0.5\n");
    assert_clean(&format!("{low}let a: Low = 0.25"));
    reported(
        &format!("{low}let b: Low = 0.75"),
        "constraint-violation",
        "0.75",
    );
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

#[test]
fn a_property_value_out_of_range() {
    let message = reported(
        "type Paint = { opacity: float64 where in 0.0..=1.0 }\nlet p = <Paint opacity=1.5 />",
        "constraint-violation",
        "1.5",
    );
    assert!(
        message.contains("0.0..=1.0") && message.contains("opacity"),
        "{message}"
    );
}

#[test]
fn a_default_out_of_range() {
    reported(
        "type Paint = { opacity: float64 where in 0.0..=1.0 = 2.0 }",
        "constraint-violation",
        "2.0",
    );
}

#[test]
fn an_argument_and_a_constant_expression() {
    reported(
        "let fade(steps: int where >= 1) = { steps }\nlet n = { fade(1 - 1) }",
        "constraint-violation",
        "1 - 1",
    );
}

#[test]
fn an_exclusive_end() {
    reported(
        "type Unit = float64 where in 0.0..1.0\nlet u: Unit = 1.0",
        "constraint-violation",
        "1.0",
    );
}

#[test]
fn a_returned_value_and_a_sequence_item() {
    reported(
        "let clampLow(x: int): int where >= 0 = { -1 }",
        "constraint-violation",
        "-1",
    );
    reported(
        "type Score = int where >= 0\nlet scores: Score+ = { 1 2 (0 - 3) }",
        "constraint-violation",
        "0 - 3",
    );
}

// ---------------------------------------------------------------------------
// Numeric match patterns
// ---------------------------------------------------------------------------

#[test]
fn dispatch_on_ranges_and_alternatives() {
    assert_clean(
        "let describe(x: float64) = { if x is { in 0.0..1.0 => \"valid\" > 1.0 => \"too big\" < 0.0 => \"too small\" else => \"one\" } }\n\
         let flag(n: int) = { if n is { < 0, > 100 => \"out\" > 0 && <= 10 => \"low\" } }",
    );
}

#[test]
fn an_unreachable_pattern_and_an_empty_one() {
    let source = "type Ratio = float64 where in 0.0..=1.0\n\
                  let f(r: Ratio) = { if r is { > 2.0 => \"big\" > 1.0 && < 0.0 => \"never\" else => \"ok\" } }";
    let found = diagnostics(source);
    let unreachable = found
        .iter()
        .filter(|(code, ..)| code == "unreachable-pattern")
        .collect::<Vec<_>>();
    assert_eq!(unreachable.len(), 1, "{found:#?}");
    assert_eq!(unreachable[0].1, Severity::Warning);
    assert_eq!(unreachable[0].3, "> 2.0");
    reported(source, "empty-constraint", "> 1.0 && < 0.0");
}

#[test]
fn a_numeric_pattern_on_a_string() {
    let found = diagnostics("let bad(s: string) = { if s is { > 3 => 1 else => 0 } }");
    assert!(
        found
            .iter()
            .any(|(_, severity, _, at)| *severity == Severity::Error && at == "> 3"),
        "{found:#?}"
    );
}

#[test]
fn a_bound_is_checked_at_the_scrutinee_type() {
    let found = diagnostics("let f(n: int) = { if n is { > 0.5 => 1 else => 0 } }");
    assert!(
        found
            .iter()
            .any(|(code, _, _, at)| code == "invalid-constraint" && at == "0.5"),
        "{found:#?}"
    );
}

#[test]
fn an_optional_scrutinee() {
    assert_clean(
        "let f(n?: int) = { if n is { {} => \"none\" > 0 => \"some\" else => \"other\" } }",
    );
}

#[test]
fn an_arm_narrows_its_scrutinee() {
    let source = "let fade(amount: float64 where in 0.0..=1.0) = { amount }\n\
                  let safe(x: float64) = { if x is { in 0.0..=1.0 => fade(x) else => 0.0 } }";
    assert_clean(source);
}
