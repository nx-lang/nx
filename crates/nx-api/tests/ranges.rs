//! Behavior of the `..` and `..=` operators, against real builds that carry the prelude.
//!
//! One case per scenario in the `range-expressions` capability. These live in `nx-api` rather than
//! `nx-types` because a range expression means the prelude's `Range`, and only a real build has a
//! prelude.

use nx_api::{
    eval_source, evaluate_component_source, validate_workspace, ComponentEvaluateEvalResult,
    EvalResult, NxDiagnostic, NxWorkspace, NxWorkspaceModule, ProgramBuildContext,
};
use nx_value::NxValue;
use std::collections::BTreeMap;

fn diagnostics(source: &str) -> Vec<NxDiagnostic> {
    let workspace = NxWorkspace::new(vec![
        NxWorkspaceModule::from_source("ranges.nx", source).expect("workspace module")
    ])
    .expect("workspace");
    validate_workspace(&workspace, &ProgramBuildContext::empty())
}

fn messages(source: &str) -> Vec<String> {
    diagnostics(source)
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect()
}

#[track_caller]
fn assert_clean(source: &str) {
    let diagnostics = diagnostics(source);
    assert!(
        diagnostics.is_empty(),
        "expected no diagnostics for:\n{source}\ngot: {diagnostics:?}"
    );
}

#[track_caller]
fn assert_reports(source: &str, needle: &str) {
    let messages = messages(source);
    assert!(
        messages.iter().any(|message| message.contains(needle)),
        "expected a diagnostic containing {needle:?} for:\n{source}\ngot: {messages:?}"
    );
}

#[track_caller]
fn evaluates(source: &str) -> NxValue {
    match eval_source(source, "ranges.nx", &ProgramBuildContext::empty()) {
        EvalResult::Ok(value) => value,
        EvalResult::Err(diagnostics) => {
            panic!("expected evaluation to succeed for:\n{source}\ngot: {diagnostics:?}")
        }
    }
}

#[track_caller]
fn int_list(value: &NxValue) -> Vec<i64> {
    match value {
        NxValue::Array(items) => items
            .iter()
            .map(|item| match item {
                NxValue::Int(n) => *n,
                other => panic!("expected an integer item, got {other:?}"),
            })
            .collect(),
        other => panic!("expected a list, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------------------------
// A range expression constructs the prelude's `Range`
// ---------------------------------------------------------------------------------------------

#[test]
fn integer_bounds_give_an_integer_range() {
    assert_clean(
        "let r = {1..5}\n\
         let typed:<Range T=int/> = {r}\n\
         let first:int = {r.start}\n\
         let root() = { first }\n",
    );
}

#[test]
fn the_expected_type_decides_the_argument() {
    assert_clean(
        "type Slider = { range:<Range T=float64/> }\n\
         let s = <Slider range={0..1} />\n\
         let bounds:<Range T=float64/> = {s.range}\n\
         let low:float64 = {bounds.start}\n\
         let root() = { low }\n",
    );
}

#[test]
fn mixed_operands_take_their_common_type() {
    assert_clean(
        "let small:int32 = 3\n\
         let a:<Range T=int/> = {small..10}\n\
         let b:<Range T=float64/> = {0..2.5}\n\
         let root() = { a }\n",
    );
}

#[test]
fn operands_with_no_common_type_are_rejected() {
    assert_reports(
        "let big:int64 = 3\n\
         let bad = {big..2.5}\n\
         let root() = { 1 }\n",
        "neither converts to the other",
    );
}

#[test]
fn a_non_numeric_operand_is_rejected_and_points_at_the_element_form() {
    let messages = messages(
        "let bad = {\"a\"..\"f\"}\n\
         let root() = { 1 }\n",
    );
    let message = messages
        .iter()
        .find(|message| message.contains("range operand must be numeric"))
        .unwrap_or_else(|| panic!("expected a numeric-operand error, got: {messages:?}"));
    assert!(
        message.contains("<Range T=string"),
        "the diagnostic points at the element form: {message}"
    );
}

/// `1..5..9` parses left-associatively, so the checker gets to say a `Range` is not a numeric bound.
#[test]
fn a_range_of_a_range_is_a_type_error() {
    assert_reports(
        "let bad = {1..5..9}\n\
         let root() = { 1 }\n",
        "range operand must be numeric",
    );
}

#[test]
fn a_range_in_a_braced_list_is_parenthesized() {
    assert_clean(
        "let rs:<Range T=int/>[] = { (0..5) (5..=9) }\n\
         let root() = { rs }\n",
    );
    assert!(
        !diagnostics(
            "let rs:<Range T=int/>[] = { 0..5 5..=9 }\n\
             let root() = { rs }\n"
        )
        .is_empty(),
        "an unparenthesized binary expression in a list is rejected, as every other one is"
    );
}

/// The documented pair in `types.md`: a list of one instantiation keeps its arguments, and a list
/// of two different ones is a list of `object`, because an applied type is invariant.
#[test]
fn a_list_of_ranges_takes_the_join_of_its_instantiations() {
    assert_reports(
        "let same = { (0..5) (5..=9) }\n\
         let bad:int = {same}\n\
         let root() = { 1 }\n",
        "found list <Range T=int/>[]",
    );
    assert_reports(
        "let mixed = { (0..5) (0.0..1.0) }\n\
         let bad:int = {mixed}\n\
         let root() = { 1 }\n",
        "found list object[]",
    );
}

#[test]
fn a_reversed_range_is_a_value() {
    assert_eq!(
        evaluates(
            "let r = {5..2}\n\
             let root() = { r.start }\n"
        ),
        NxValue::Int(5)
    );
}

#[test]
fn the_operator_and_the_element_build_equal_values() {
    assert_eq!(
        evaluates(
            "let a = {1..=5}\n\
             let b = <Range T=int start={1} end={5} endInclusive={true} />\n\
             let root() = { a == b }\n"
        ),
        NxValue::Bool(true)
    );
    assert_eq!(
        evaluates(
            "let a = {1..5}\n\
             let b = {1..=4}\n\
             let root() = { a == b }\n"
        ),
        NxValue::Bool(false),
        "a half-open range and a closed one are different records"
    );
}

// ---------------------------------------------------------------------------------------------
// A range expression is rejected where `Range` is not the prelude's
// ---------------------------------------------------------------------------------------------

#[test]
fn a_local_range_disables_the_operator_in_that_module() {
    let diagnostics = diagnostics(
        "type Range = { low:int high:int }\n\
         let r = {1..5}\n\
         let root() = { 1 }\n",
    );
    let hidden = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code.as_deref() == Some("range-hidden"))
        .unwrap_or_else(|| panic!("expected a range-hidden diagnostic, got: {diagnostics:?}"));
    assert!(
        hidden.message.contains("declares its own 'Range'"),
        "the diagnostic names the file's own declaration: {}",
        hidden.message
    );
    assert!(
        hidden
            .labels
            .iter()
            .any(|label| !label.primary && label.span.start_line == 1),
        "a secondary label points at the hiding declaration: {:?}",
        hidden.labels
    );
}

/// A declaration hides the prelude's `Range` whatever namespace it occupies. A value, a function
/// and a component leave the type namespace free, and a prelude record bound there while the
/// module's own name holds another namespace would let the operator check against a declaration
/// that the construction below the checker — the interpreter's, codegen's — never resolves to.
#[test]
fn a_local_range_in_any_namespace_disables_the_operator() {
    for declaration in [
        "let Range = 5",
        "let Range() = 5",
        "component <Range /> = { <span>hi</span> }",
        "type Range = int",
        "type Range = | only",
    ] {
        let source = format!("{declaration}\nlet r = {{1..5}}\nlet root() = {{ 1 }}\n");
        let diagnostics = diagnostics(&source);
        let hidden = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code.as_deref() == Some("range-hidden"))
            .unwrap_or_else(|| {
                panic!(
                    "expected a range-hidden diagnostic for {declaration:?}, got: {diagnostics:?}"
                )
            });
        assert!(
            hidden.message.contains("declares its own 'Range'"),
            "the diagnostic names the module's own declaration for {declaration:?}: {}",
            hidden.message
        );
        assert!(
            hidden
                .labels
                .iter()
                .any(|label| !label.primary && label.span.start_line == 1),
            "a secondary label points at the hiding declaration for {declaration:?}: {:?}",
            hidden.labels
        );
    }
}

/// A `Range` a module does not import is a `Range` it does not have: the operator still means the
/// prelude's.
#[test]
fn another_modules_range_does_not_matter() {
    let workspace = NxWorkspace::new(vec![
        NxWorkspaceModule::from_source("shapes.nx", "export type Range = { low:int high:int }\n")
            .expect("shapes module"),
        NxWorkspaceModule::from_source(
            "main.nx",
            "let r:<Range T=int/> = {1..5}\nlet root() = { r }\n",
        )
        .expect("main module"),
    ])
    .expect("workspace");

    let diagnostics = validate_workspace(&workspace, &ProgramBuildContext::empty());
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

// ---------------------------------------------------------------------------------------------
// `for` iterates a range of an integer type
// ---------------------------------------------------------------------------------------------

#[test]
fn a_half_open_range_counts_up_to_its_end() {
    let squares = evaluates(
        "let squares = { for i in 0..4 { i * i } }\n\
         let root() = { squares }\n",
    );
    assert_eq!(int_list(&squares), vec![0, 1, 4, 9]);
}

#[test]
fn a_closed_range_includes_its_end() {
    let pages = evaluates(
        "let pages = { for page in 1..=3 { page } }\n\
         let root() = { pages }\n",
    );
    assert_eq!(int_list(&pages), vec![1, 2, 3]);
}

#[test]
fn the_index_counts_from_zero() {
    let pairs = evaluates(
        "let pairs = { for value, index in 5..8 { value * 10 + index } }\n\
         let root() = { pairs }\n",
    );
    assert_eq!(int_list(&pairs), vec![50, 61, 72]);
}

#[test]
fn empty_and_reversed_ranges_yield_nothing() {
    for iterable in ["0..count", "5..2", "1..=0"] {
        let source = format!(
            "let count = 0\n\
             let xs = {{ for i in {iterable} {{ i }} }}\n\
             let root() = {{ xs }}\n"
        );
        assert_eq!(
            int_list(&evaluates(&source)),
            Vec::<i64>::new(),
            "{iterable} yields no items"
        );
    }
}

#[test]
fn a_one_element_closed_range() {
    let one = evaluates(
        "let one = { for i in 3..=3 { i } }\n\
         let root() = { one }\n",
    );
    assert_eq!(int_list(&one), vec![3]);
}

#[test]
fn a_range_value_from_anywhere_iterates() {
    let items = evaluates(
        "let span:<Range T=int/> = <Range T=int start={2} end={4} endInclusive={true} />\n\
         let items = { for i in span { i } }\n\
         let root() = { items }\n",
    );
    assert_eq!(int_list(&items), vec![2, 3, 4]);
}

#[test]
fn a_component_renders_one_element_per_integer() {
    let stars = evaluates(
        "let <Star /> = <star />\n\
         let <Stars count:int /> = { for i in 0..count { <Star /> } }\n\
         let root() = <Stars count={5} />\n",
    );
    let children = match &stars {
        NxValue::Array(children) => children.len(),
        other => panic!("expected a list of rendered elements, got {other:?}"),
    };
    assert_eq!(children, 5);
}

/// The count is computed once, so a closed range ending at the carrier's maximum does not overflow
/// on its way to the loop.
#[test]
fn a_closed_range_at_the_maximum_value_terminates() {
    let last = evaluates(
        "let top:int = 9007199254740991\n\
         let xs = { for i in top..=top { i } }\n\
         let root() = { xs }\n",
    );
    assert_eq!(int_list(&last), vec![9007199254740991]);
}

/// A range the host built, not the operator, iterates the same way.
#[test]
fn a_host_supplied_range_iterates() {
    let mut range = BTreeMap::new();
    range.insert("start".to_string(), NxValue::Int(2));
    range.insert("end".to_string(), NxValue::Int(4));
    range.insert("endInclusive".to_string(), NxValue::Bool(true));

    let mut props = BTreeMap::new();
    props.insert(
        "span".to_string(),
        NxValue::Record {
            type_name: Some("Range".to_string()),
            properties: range,
        },
    );

    let result = evaluate_component_source(
        "component <Counted span:<Range T=int/> /> = { for i in span { <tick value={i} /> } }\n",
        "ranges.nx",
        &ProgramBuildContext::empty(),
        "Counted",
        &NxValue::Record {
            type_name: None,
            properties: props,
        },
        &NxValue::Null,
    );
    let rendered = match result {
        ComponentEvaluateEvalResult::Ok(result) => result.rendered,
        ComponentEvaluateEvalResult::Err(diagnostics) => {
            panic!("expected the host's range to iterate: {diagnostics:?}")
        }
    };

    let ticks = match &rendered {
        NxValue::Array(items) => items.len(),
        other => panic!("expected a list of rendered ticks, got {other:?}"),
    };
    assert_eq!(ticks, 3, "the body runs for 2, 3 and 4");
}

/// The budget, not a pre-allocation, is what stops a range that is merely enormous.
#[test]
fn an_enormous_range_exceeds_the_operation_budget() {
    let EvalResult::Err(diagnostics) = eval_source(
        "let root() = { for i in 0..2000000 { i } }\n",
        "ranges.nx",
        &ProgramBuildContext::empty(),
    ) else {
        panic!("expected the operation budget to stop the loop");
    };
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("Operation limit exceeded")),
        "{diagnostics:?}"
    );
}

#[test]
fn the_item_has_the_ranges_integer_type() {
    assert_clean(
        "let lo:int32 = 1\n\
         let hi:int32 = 3\n\
         let xs:int32[] = { for i in lo..hi { i } }\n\
         let root() = { xs }\n",
    );
}

/// A look-alike does not iterate. The item's carrier and the declaring module are both absent from
/// the value a backend sees — `Range`'s `T` erases to `object` in an image, and a canonical record
/// names its declaration without its module — so which `Range` a loop counts over is settled here,
/// at the checker, and nowhere below it.
#[test]
fn a_modules_own_range_does_not_iterate() {
    let messages = messages(
        "type Range = { start:int end:int endInclusive:boolean }\n         let mine = <Range start={0} end={4} endInclusive={false} />\n         let xs = { for i in mine { i } }\n         let root() = { xs }\n",
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("For iterable must be an array, found Range")),
        "expected the loop over the module's own Range to be refused, got: {messages:?}"
    );
}

#[test]
fn a_non_integer_range_does_not_iterate() {
    let messages = messages(
        "let bad = { for x in 0..2.5 { x } }\n\
         let root() = { 1 }\n",
    );
    let message = messages
        .iter()
        .find(|message| message.contains("only a range of an integer type"))
        .unwrap_or_else(|| panic!("expected a range-not-iterable error, got: {messages:?}"));
    assert!(
        message.contains("<Range T=float64/>"),
        "the diagnostic names the type: {message}"
    );
}

/// An optional range is not an iterable, exactly as an optional list is not: the iterable is "a
/// list or a `<Range T=X/>`", and accepting `<Range T=int/>?` would only move the failure to
/// evaluation.
#[test]
fn an_optional_range_does_not_iterate() {
    assert_reports(
        "let maybe:<Range T=int/>? = null\n         let xs:int[] = { for i in maybe { i } }\n         let root() = { 1 }\n",
        "found <Range T=int/>?",
    );
}

/// A range at an optional binding site is still ordinary: the nullability is stripped there, not
/// at the iterable.
#[test]
fn a_range_binds_to_an_optional_range_field() {
    assert_clean(
        "type Slider = { range:<Range T=float64/>? }\n         let s = <Slider range={0..1} />\n         let root() = { s }\n",
    );
}

/// The shipped example is checked the way a reader runs it, so the page and the compiler cannot
/// drift apart. It lives here rather than in `nx-types` because it needs a real build's prelude.
#[test]
fn the_ranges_example_checks_and_evaluates() {
    let source = include_str!("../../../examples/nx/ranges.nx");
    let diagnostics = {
        let workspace = NxWorkspace::new(vec![
            NxWorkspaceModule::from_source("ranges.nx", source).expect("workspace module")
        ])
        .expect("workspace");
        validate_workspace(&workspace, &ProgramBuildContext::empty())
    };
    assert!(diagnostics.is_empty(), "{diagnostics:?}");

    match eval_source(source, "ranges.nx", &ProgramBuildContext::empty()) {
        EvalResult::Ok(value) => assert_eq!(int_list(&value), vec![0, 1, 4, 9]),
        EvalResult::Err(diagnostics) => panic!("the example should evaluate: {diagnostics:?}"),
    }
}
