//! The presence test `x?`, the step `x?.m`, the fallback `x ?? y` and the `{}` pattern: typing,
//! the diagnostics for receivers that cannot be empty, and the narrowing they induce.
//!
//! One case per static scenario in the `presence-operators` capability.

use nx_diagnostics::Severity;
use nx_hir::Item;
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

fn warnings(source: &str) -> Vec<String> {
    check(source)
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity() == Severity::Warning)
        .map(|diagnostic| diagnostic.message().to_string())
        .collect()
}

fn assert_clean(source: &str) {
    let errors = errors(source);
    assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
}

fn body_type(source: &str, function: &str) -> Type {
    let checked = check(source);
    let module = checked.lowered_module.as_ref().expect("lowered module");
    let body = module
        .items()
        .iter()
        .find_map(|item| match item {
            Item::Function(f) if f.name.as_str() == function => Some(f.body),
            _ => None,
        })
        .unwrap_or_else(|| panic!("a function named {function}"));
    checked
        .type_env
        .get_expr_type(body)
        .cloned()
        .expect("a type for the body")
}

const BOOK: &str = "type Person = { name:string }\ntype Book = { title:string author?:Person }\n";

// ---------------------------------------------------------------------------
// Postfix `?` tests presence
// ---------------------------------------------------------------------------

#[test]
fn a_presence_test_is_a_boolean() {
    let source = format!("{BOOK}let has(b:Book): boolean = {{ b.author? }}");
    assert_clean(&source);
    assert_eq!(body_type(&source, "has"), Type::boolean());
}

#[test]
fn a_test_on_a_value_that_cannot_be_empty_is_reported() {
    let source = "let f(n:int, xs:int+) = { n? && xs? }";
    assert_clean(source);
    let warnings = warnings(source);
    assert_eq!(warnings.len(), 2, "{warnings:?}");
    assert!(
        warnings.iter().all(|w| w.contains("always present")),
        "{warnings:?}"
    );
}

#[test]
fn a_test_on_zero_or_more_is_non_emptiness() {
    assert_clean("let any(xs?:int+): boolean = { xs? }");
}

#[test]
fn a_test_composes_with_negation_and_conjunction() {
    assert_clean("let f(a?:int, b?:int): boolean = { !a? || (a? && b?) }");
}

// ---------------------------------------------------------------------------
// A presence test narrows the tested path
// ---------------------------------------------------------------------------

#[test]
fn a_tested_field_is_read_as_present_in_the_branch() {
    let source = format!(
        "{BOOK}let byline(b:Book): string = {{ if b.author? {{ \"by \" + b.author.name }} else {{ \"anonymous\" }} }}"
    );
    assert_clean(&source);
    let unnarrowed = self::errors(&format!(
        "{BOOK}let byline(b:Book): string = {{ if b.author? {{ \"by \" }} else {{ b.author.name }} }}"
    ));
    assert_eq!(unnarrowed.len(), 1, "{unnarrowed:?}");
    assert!(unnarrowed[0].contains("?.name"), "{unnarrowed:?}");
}

#[test]
fn a_negated_test_narrows_the_else_branch() {
    assert_clean("let f(o?:string): string = { if !o? { \"none\" } else { o } }");
}

#[test]
fn a_conjunction_narrows_each_tested_operand() {
    assert_clean("let f(a?:string, b?:string): string = { if a? && b? { a + b } else { \"\" } }");
}

#[test]
fn a_disjunction_does_not_narrow() {
    let errors =
        errors("let f(a?:string, b?:string): string = { if a? || b? { a } else { \"\" } }");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("string?"), "{errors:?}");
}

#[test]
fn a_tested_chain_narrows_each_receiver() {
    assert_clean(
        "type Name = { first:string }\ntype Person = { name?:Name }\ntype Book = { author?:Person }\n\
         let f(b:Book): string = { if b.author?.name? { b.author.name.first } else { \"\" } }",
    );
}

#[test]
fn narrowing_does_not_escape_its_branch() {
    // The body collects the `if` and the trailing `o`; read as `string?` outside the branch, `o`
    // makes the collection `string*`. Narrowed, it would have been `string+`.
    let errors = errors("let f(o?:string): string = { if o? { \"x\" } o }");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("string*"), "{errors:?}");
    assert_clean("let g(o?:string): string* = { if o? { \"x\" } o }");
}

#[test]
fn a_narrowed_zero_or_more_is_one_or_more() {
    assert_clean("let first(xs?:int+): int+ = { if xs? { xs } else { 0 } }");
}

// ---------------------------------------------------------------------------
// `?.` steps through an optional receiver
// ---------------------------------------------------------------------------

#[test]
fn a_step_through_an_absent_receiver_is_empty() {
    let source = format!("{BOOK}let name(b:Book): string? = {{ b.author?.name }}");
    assert_clean(&source);
}

#[test]
fn a_step_joins_the_members_occurrence() {
    let source = "type Person = { nick?:string tags:string+ }\ntype Book = { author?:Person }\nlet f(b:Book) = { b.author?.nick }\nlet g(b:Book) = { b.author?.tags }";
    assert_clean(source);
    assert_eq!(body_type(source, "f"), Type::optional(Type::string()));
    assert_eq!(body_type(source, "g"), Type::zero_or_more(Type::string()));
}

#[test]
fn a_plain_dot_on_an_optional_receiver_is_rejected_with_the_fix() {
    let errors = errors(&format!("{BOOK}let f(b:Book) = {{ b.author.name }}"));
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("b.author?.name"), "{errors:?}");
}

#[test]
fn a_step_on_a_receiver_that_is_always_present_is_reported() {
    let source = "type Person = { name:string }\ntype Book = { author:Person }\nlet f(b:Book) = { b.author?.name }";
    assert_clean(source);
    let warnings = warnings(source);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("unnecessary"), "{warnings:?}");
}

#[test]
fn a_step_on_a_sequence_is_rejected() {
    let errors = errors("type Person = { name:string }\nlet f(ps:Person+) = { ps?.name }");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("no members") && errors[0].contains("for"),
        "{errors:?}"
    );
    let dot = self::errors("type Person = { name:string }\nlet f(ps:Person+) = { ps.name }");
    assert_eq!(dot.len(), 1, "{dot:?}");
    assert!(dot[0].contains("no members"), "{dot:?}");
}

#[test]
fn a_member_of_a_presence_test_names_the_step_spelling() {
    // A space splits `?.`, so `b.author? .name` reads a member of the boolean test.
    let errors = errors(&format!(
        "{BOOK}let f(b:Book): boolean = {{ b.author? .name }}"
    ));
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("presence test is a boolean") && errors[0].contains("`b.author?.name`"),
        "{errors:?}"
    );
}

#[test]
fn a_member_of_a_value_with_no_members_names_its_type() {
    let errors = errors("let f(s:string): int = { s.length }");
    assert_eq!(
        errors,
        vec!["A value of type string has no member 'length'".to_string()]
    );
}

// ---------------------------------------------------------------------------
// `??` supplies a fallback for an empty value
// ---------------------------------------------------------------------------

#[test]
fn a_fallback_makes_an_optional_exactly_one() {
    let source =
        "type Book = { subtitle?:string }\nlet sub(b:Book): string = { b.subtitle ?? \"none\" }";
    assert_clean(source);
    assert_eq!(body_type(source, "sub"), Type::string());
}

#[test]
fn a_fallback_binds_tighter_than_concatenation() {
    let source = format!(
        "{BOOK}let byline(b:Book): string = {{ \"Author: \" + b.author?.name ?? \"Anonymous\" }}"
    );
    assert_clean(&source);
}

#[test]
fn the_right_operand_decides_how_many_the_result_admits() {
    let source = "let a(ys:int+, xs?:int+) = { xs ?? ys }\nlet b(xs?:int+, ys?:int+) = { xs ?? ys }\nlet c(o?:int, p?:int) = { o ?? p }";
    assert_clean(source);
    assert_eq!(body_type(source, "a"), Type::one_or_more(Type::int()));
    assert_eq!(body_type(source, "b"), Type::zero_or_more(Type::int()));
    assert_eq!(body_type(source, "c"), Type::optional(Type::int()));
}

#[test]
fn a_fallback_on_a_value_that_cannot_be_empty_is_reported() {
    let source = "let f(n:int) = { n ?? 0 }";
    assert_clean(source);
    let warnings = warnings(source);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("never taken"), "{warnings:?}");
}

#[test]
fn fallback_chains_associate_to_the_right() {
    let source = "let f(c:int, a?:int, b?:int): int = { a ?? b ?? c }";
    assert_clean(source);
    assert_eq!(body_type(source, "f"), Type::int());
}

// ---------------------------------------------------------------------------
// `{}` is a pattern that matches the empty value
// ---------------------------------------------------------------------------

#[test]
fn the_empty_pattern_matches_absence() {
    assert_clean(&format!(
        "{BOOK}let f(b:Book): string = {{ if b.author is {{ {{}} => \"anonymous\" else => b.author.name }} }}"
    ));
}

#[test]
fn the_empty_pattern_completes_exhaustiveness_over_an_optional_union() {
    assert_clean(
        "type State = idle | busy\nlet f(s?:State): string = { if s is { {} => \"none\" idle => \"idle\" busy => \"busy\" } }",
    );
    let missing = self::errors("type State = idle | busy\nlet f(s?:State): string = { if s is { idle => \"idle\" busy => \"busy\" } }");
    assert!(
        missing
            .iter()
            .any(|e| e.contains("missing cases") && e.contains("{}")),
        "{missing:?}"
    );
}

#[test]
fn the_empty_pattern_on_a_scrutinee_that_cannot_be_empty_is_reported() {
    let source = "let f(n:int) = { if n is { {} => 0 else => n } }";
    let warnings = warnings(source);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("never matches"), "{warnings:?}");
}

// ---------------------------------------------------------------------------
// Narrowing scope and sites
// ---------------------------------------------------------------------------

#[test]
fn a_shadowing_binding_does_not_inherit_a_narrowing() {
    // The loop's `o` is an `int`, not the narrowed `string` parameter it shadows.
    let errors = errors(
        "let f(xs:int+, o?:string): string+ = { if o? { for o in xs { o } } else { \"a\" } }",
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_clean("let f(xs:int+, o?:string): int+ = { if o? { for o in xs { o } } else { 1 } }");
}

#[test]
fn a_shadowing_binding_does_not_inherit_a_case_narrowing() {
    assert_clean(
        "type Shape = | circle { r:float64 } | square { s:float64 }\n\
         let f(sh:Shape, xs:int+): int+ = { if sh is { circle => for sh in xs { sh } else => 1 } }",
    );
}

#[test]
fn a_property_list_if_narrows_its_branches() {
    assert_clean(
        "type Person = { nick?:string }\nexternal component <Label text:string />\n\
         let a(p:Person) = { <Label if p.nick? { text={p.nick} } else { text=\"x\" } /> }\n\
         let b(p:Person) = { <Label if !p.nick? { text=\"x\" } else { text={p.nick} } /> }",
    );
}

#[test]
fn a_property_list_condition_list_narrows_later_arms_by_earlier_refuted_tests() {
    assert_clean(
        "type Person = { nick?:string }\nexternal component <Label text:string />\n\
         let a(p:Person) = { <Label if { !p.nick? => text=\"x\" else => text={p.nick} } /> }",
    );
}

// ---------------------------------------------------------------------------
// The removed conditional operator is reported once
// ---------------------------------------------------------------------------

#[test]
fn a_conditional_operator_brings_no_diagnostics_about_its_salvaged_test() {
    for source in [
        "let f(n:int) = { n > 0 ? n * 2 : -1 }",
        "let f(c:boolean) = { c ? 1 : 2 }",
        "let f(c:boolean) = { c ? \"a\" : \"b\" }",
        "let f(c:boolean) = { \"x\" c ? 1 : 2 }",
        "let f(c:boolean) = c ? \"a\" : \"b\"",
        "let <A x:int /> = { x }\nlet f(c:boolean) = <A x={c ? 1 : 2} />",
    ] {
        let diagnostics: Vec<_> = check(source)
            .diagnostics
            .iter()
            .map(|diagnostic| {
                (
                    diagnostic.code().map(str::to_string),
                    diagnostic.message().to_string(),
                )
            })
            .collect();
        assert!(
            diagnostics
                .iter()
                .all(|(code, _)| code.as_deref() == Some("removed-conditional-operator")),
            "{source}: {diagnostics:?}"
        );
        assert!(
            !diagnostics.is_empty(),
            "{source}: expected the ternary diagnostic"
        );
    }

    // Only the salvaged test and its consequent are dropped; an earlier item is still checked.
    let errors = errors("let f(c:boolean) = { missing c ? 1 : 2 }");
    assert!(
        errors
            .iter()
            .any(|error| error.contains("Undefined identifier 'missing'")),
        "{errors:?}"
    );
}
