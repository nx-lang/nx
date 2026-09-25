//! The occurrence model: one suffix with four cardinalities, the lattice that decides
//! satisfaction, the empty value, and how a collecting position and a `for` compute occurrences.
//!
//! One case per scenario in the `occurrence-types` capability.

use nx_hir::{Item, Name};
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

// ---------------------------------------------------------------------------
// A type reference carries at most one occurrence suffix
// ---------------------------------------------------------------------------

#[test]
fn each_suffix_is_accepted_once() {
    assert_clean("type A = string?\ntype B = string+\ntype C = string*\ntype D = (string)+");
    assert_clean("type B = string+\ntype D = (string)+\nlet b:B = {\"x\"}\nlet d:D = {b}");
}

#[test]
fn a_second_suffix_through_an_alias_is_rejected() {
    let errors = errors("type A = string?\ntype I = A?");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("already carries an occurrence") && errors[0].contains("A"),
        "{errors:?}"
    );
    let alias_errors = self::errors("type Ints = int+\ntype Grid = Ints*\ntype MaybeInts = Ints?");
    assert_eq!(alias_errors.len(), 2, "{alias_errors:?}");
    assert!(
        alias_errors.iter().all(|e| e.contains("Ints")),
        "{alias_errors:?}"
    );
}

#[test]
fn null_is_not_a_literal() {
    let errors = errors("let x = null");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("{}"), "{errors:?}");
}

// ---------------------------------------------------------------------------
// Occurrences form a lattice that decides satisfaction
// ---------------------------------------------------------------------------

#[test]
fn exactly_one_satisfies_every_occurrence() {
    assert_clean("let a:int? = 1\nlet b:int+ = 1\nlet c:int* = 1");
}

#[test]
fn optional_and_one_or_more_each_satisfy_zero_or_more_and_nothing_narrower() {
    assert_clean("let o:int? = {}\nlet p:int+ = {1 2}\nlet s:int* = {o}\nlet t:int* = {p}");
    let errors = errors("let o:int? = {}\nlet p:int+ = {1 2}\nlet u:int+ = {o}\nlet v:int? = {p}");
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(
        errors[0].contains("int+") && errors[0].contains("int?"),
        "{errors:?}"
    );
    assert!(
        errors[1].contains("int?") && errors[1].contains("int+"),
        "{errors:?}"
    );
}

#[test]
fn an_optional_does_not_satisfy_exactly_one() {
    let errors = errors("let o:string? = {}\nlet s:string = {o}");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("string?") && errors[0].contains("string"),
        "{errors:?}"
    );
}

// ---------------------------------------------------------------------------
// The empty sequence is the absent value
// ---------------------------------------------------------------------------

#[test]
fn the_empty_value_satisfies_optional_and_zero_or_more_sites() {
    assert_clean(
        "let a:string? = {}\nlet b:string* = {}\ntype Box = { items?:string+ }\nlet c = <Box items={} />",
    );
}

#[test]
fn the_empty_value_is_rejected_where_at_least_one_is_required() {
    let errors = errors("let a:string = {}\nlet b:string+ = {}");
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(errors.iter().all(|e| e.contains("found {}")), "{errors:?}");
    assert!(errors.iter().all(|e| !e.contains("never")), "{errors:?}");
}

#[test]
fn every_source_of_emptiness_is_the_same_value() {
    assert_clean(
        "let f(c:boolean): int? = { if c { 1 } }\nlet g(): int? = {}\nlet h(ns?:int+): int* = { for n in ns {} }",
    );
}

// ---------------------------------------------------------------------------
// Only an exactly-one type is an item type
// ---------------------------------------------------------------------------

#[test]
fn a_type_argument_that_carries_an_occurrence_is_rejected() {
    let errors = errors(
        "type Box = { T:type value:T }\ntype Maybe = int?\nlet a = <Box T=int? value=1 />\nlet b:<Box T=Maybe/> = <Box T=Maybe value=1 />",
    );
    assert!(
        errors.iter().any(|e| e
            == "A type argument must be exactly one value; 'int?' carries an occurrence"),
        "{errors:?}"
    );
    assert!(
        errors.iter().all(|e| !e.contains("Syntax error")),
        "{errors:?}"
    );
    assert!(
        errors
            .iter()
            .any(|e| e.contains("Maybe") && e.contains("carries an occurrence")),
        "{errors:?}"
    );
}

#[test]
fn optionality_moves_to_the_slot() {
    let source = "type Box = { T:type value?:T }\nlet b = <Box T=int />\nlet v = { b.value }";
    assert_clean(source);
    assert_eq!(binding_type(source, "v"), Type::optional(Type::int()));
}

// ---------------------------------------------------------------------------
// The join takes the least upper bound of the occurrences
// ---------------------------------------------------------------------------

#[test]
fn a_conditional_with_no_else_is_optional() {
    let source = "let c = true\nlet v = { if c { 1 } }";
    assert_clean(source);
    assert_eq!(binding_type(source, "v"), Type::optional(Type::int()));
}

#[test]
fn a_conditional_joins_occurrences() {
    let source = "let c = true\nlet xs:int+ = {1 2}\nlet a = { if c { 1 } else { xs } }\nlet b = { if c { xs } }\nlet d = { if c { 1 } else { 2 } }";
    assert_clean(source);
    assert_eq!(binding_type(source, "a"), Type::one_or_more(Type::int()));
    assert_eq!(binding_type(source, "b"), Type::zero_or_more(Type::int()));
    assert_eq!(binding_type(source, "d"), Type::int());
}

#[test]
fn an_empty_arm_takes_its_item_type_from_the_other_arm() {
    let source = "type A = { n:int = 1 }\nexternal component <Box content items:A+ />\nlet c = true\nlet v = <Box><A/>{if c { <A n=2 /> } else { }}</Box>";
    let errors = errors(source);
    assert!(errors.is_empty(), "{errors:?}");
}

// ---------------------------------------------------------------------------
// A collecting position adds occurrences and a for multiplies them
// ---------------------------------------------------------------------------

#[test]
fn two_exactly_one_items_make_one_or_more() {
    let source = "let xs = {1 2}";
    assert_clean(source);
    assert_eq!(binding_type(source, "xs"), Type::one_or_more(Type::int()));
}

#[test]
fn an_optional_beside_an_item_makes_one_or_more() {
    let source = "let o:int? = {}\nlet xs = {o 2}";
    assert_clean(source);
    assert_eq!(binding_type(source, "xs"), Type::one_or_more(Type::int()));
}

#[test]
fn two_optionals_make_zero_or_more() {
    let source = "let o:int? = {}\nlet p:int? = 3\nlet xs = {o p}";
    assert_clean(source);
    assert_eq!(binding_type(source, "xs"), Type::zero_or_more(Type::int()));
}

#[test]
fn a_single_item_is_itself() {
    let source = "let o:int? = 1\nlet v = {o}";
    assert_clean(source);
    assert_eq!(binding_type(source, "v"), Type::optional(Type::int()));
}

#[test]
fn a_for_over_one_or_more_with_an_exactly_one_body_yields_one_or_more() {
    assert_clean("let squares(ns:int+): int+ = { for n in ns { n * n } }");
}

#[test]
fn a_for_whose_body_may_yield_nothing_yields_zero_or_more() {
    assert_clean("let evens(ns:int+): int* = { for n in ns { if (n % 2 == 0) { n } } }");
    let bad = self::errors("let bad(ns:int+): int+ = { for n in ns { if (n % 2 == 0) { n } } }");
    assert_eq!(bad.len(), 1, "{bad:?}");
    assert!(bad[0].contains("int*"), "{bad:?}");
}

#[test]
fn a_for_over_an_optional_yields_an_optional() {
    assert_clean(
        "type Person = { name:string }\nlet name(p?:Person): string? = { for x in p { x.name } }",
    );
    assert_eq!(
        body_type(
            "type Person = { name:string }\nlet name(p?:Person) = { for x in p { x.name } }",
            "name"
        ),
        Type::optional(Type::string())
    );
}

// ---------------------------------------------------------------------------
// Types render by their NX spelling
// ---------------------------------------------------------------------------

#[test]
fn a_mismatch_names_both_occurrences() {
    let errors =
        errors("type Box = { items:string+ }\nlet o:string? = {}\nlet b = <Box items={o} />");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("string+") && errors[0].contains("string?"),
        "{errors:?}"
    );
    assert!(
        !errors[0].contains("nullable") && !errors[0].contains("null") && !errors[0].contains("[]"),
        "{errors:?}"
    );
}

#[test]
fn the_empty_type_renders_as_its_spelling() {
    let errors = errors("let s:string = {}");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("found {}"), "{errors:?}");
}

// ---------------------------------------------------------------------------
// Operators at the lattice
// ---------------------------------------------------------------------------

#[test]
fn ordering_takes_exactly_one_value_on_each_side() {
    let errors = errors("let a(n?:int): boolean = { n > 1 }\nlet b(ns:int+): boolean = { ns < 1 }");
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(errors[0].contains("??"), "{errors:?}");
    assert!(errors[1].contains("for"), "{errors:?}");
}

#[test]
fn equality_compares_across_occurrences() {
    assert_clean(
        "let one:int? = 1\nlet ones:int* = { 1 }\nlet two:int+ = { 1 2 }\n\
         let a = { one == ones }\nlet b = { one == two }\nlet c = { one != two }",
    );
    let errors =
        errors("let one:int? = 1\nlet names:string+ = { \"a\" }\nlet a = { one == names }");
    assert_eq!(errors.len(), 1, "{errors:?}");
}

#[test]
fn a_for_whose_body_yields_nothing_is_the_empty_type() {
    assert_clean("let h(xs?:int+): int? = { for x in xs { } }");
}

#[test]
fn an_operand_that_is_not_exactly_one_is_pointed_at_the_fix() {
    let errors = errors("let g(s?:string) = { \"v: \" + s }\nlet h(n?:int) = { n + 1 }");
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(
        errors.iter().all(|error| error.contains("??")),
        "{errors:?}"
    );
}

#[test]
fn null_at_a_typed_site_names_the_empty_value_not_a_string() {
    let errors = errors("let x: string? = null");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("`{}`"), "{errors:?}");
    assert!(!errors[0].contains("\"null\""), "{errors:?}");
}

#[test]
fn a_literal_compared_with_an_occurrence_is_typed_by_the_item_type() {
    let errors = errors("let i:int32? = 1\nlet a = { i == 9999999999 }");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("out of range for int32"), "{errors:?}");
}

#[test]
fn a_unary_operand_that_is_not_exactly_one_is_pointed_at_the_fix() {
    let errors = errors("let f(n?:int) = { -n }\nlet g(b?:boolean) = { !b }");
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(
        errors.iter().all(|error| error.contains("??")),
        "{errors:?}"
    );
}
