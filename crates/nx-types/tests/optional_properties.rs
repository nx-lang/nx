//! The `name?:type` property form: where it is taken, what the slot admits, the no-default rule,
//! the read type, and construction with an omitted, written or empty value.
//!
//! One case per scenario in the `optional-properties` capability.

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

fn binding_type(source: &str, name: &str) -> Type {
    check(source)
        .type_env
        .lookup(&Name::new(name))
        .unwrap_or_else(|| panic!("no binding named {name:?}"))
        .clone()
}

// ---------------------------------------------------------------------------
// A property admits zero only through a mark on its name
// ---------------------------------------------------------------------------

#[test]
fn the_four_property_shapes_are_accepted() {
    assert_clean(
        "type Person = { name:string }\ntype Book = { title:string subtitle?:string authors:Person+ tags?:string+ }",
    );
}

#[test]
fn an_occurrence_that_admits_zero_is_rejected_in_the_type_slot() {
    let errors = errors("type Book = { subtitle:string? tags:string* }");
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(errors[0].contains("subtitle?:string"), "{errors:?}");
    assert!(errors[1].contains("tags?:string+"), "{errors:?}");
}

#[test]
fn an_alias_that_carries_zero_is_rejected_in_the_type_slot() {
    let errors = errors("type Maybe = string?\ntype Book = { subtitle:Maybe }");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("subtitle")
            && errors[0].contains("Maybe")
            && errors[0].contains("string?"),
        "{errors:?}"
    );
    assert_clean(
        "type Names = string+\ntype Person = { name:string }\ntype Book = { authors:Names }",
    );
}

/// The fix-it says to mark the name, so the property reads as marked: a use that leaves it out is
/// not reported again. A file that wrote the old spelling on five properties, used a dozen times,
/// would otherwise report sixty errors for five mistakes.
#[test]
fn a_property_rejected_in_the_type_slot_is_not_reported_again_where_it_is_left_out() {
    for source in [
        "abstract external component <Element />\n\
         external component <Label text:string />\n\
         component <Box width:float64? /> = { <Label text=\"x\" /> }\n\
         let a = <Box />\nlet b = <Box />",
        "let <Row gap:float64? /> = { \"row\" }\nlet a = <Row />\nlet b = <Row />",
        "type Book = { title:string tags:string* }\nlet a = <Book title=\"a\" />\nlet b = <Book title=\"b\" />",
        "let f(a:int, b:int?) = { a }\nlet root() = { f(1) + f(2) }",
    ] {
        let errors = errors(source);
        assert_eq!(errors.len(), 1, "{source}\n{errors:?}");
        assert!(errors[0].contains("admits zero only"), "{errors:?}");
    }

    // A value written for it is still checked against the type it names.
    for source in [
        "let <Row gap:float64? /> = { \"row\" }\nlet a = <Row gap=\"wide\" />",
        "let f(a:int, b:int?) = { a }\nlet root() = { f(2, \"x\") }",
    ] {
        let errors = errors(source);
        assert_eq!(errors.len(), 2, "{source}\n{errors:?}");
    }
}

#[test]
fn every_declaration_site_takes_the_mark() {
    assert_clean(
        "abstract external component <Element />\n\
         external component <Label text:string />\n\
         type Person = { name:string }\n\
         component <Card title:string subtitle?:string /> = { state { note?:string } <Label text={title} /> }\n\
         let <Row gap:float64 = 0.0 content child?:Element /> = { child }\n\
         let f(a:int, b?:int) = { a }\n\
         type Render = <function Item:Person Index?:int />: string",
    );
}

// ---------------------------------------------------------------------------
// An optional property has no default
// ---------------------------------------------------------------------------

#[test]
fn a_default_on_an_optional_property_is_rejected() {
    let errors = errors("type Book = { subtitle?:string = \"none\" }");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("write `subtitle:string = \"none\"` or `subtitle?:string`"),
        "{errors:?}"
    );
}

#[test]
fn the_default_is_quoted_as_written_in_every_property_slot() {
    for (source, expected) in [
        (
            "type Maybe = string\ntype Book = { subtitle?:Maybe = \"none\" }",
            "write `subtitle:Maybe = \"none\"` or `subtitle?:Maybe`",
        ),
        (
            "let <Row gap?:float64 = { 0.5 * 2.0 } /> = { gap }",
            "write `gap:float64 = { 0.5 * 2.0 }` or `gap?:float64`",
        ),
        (
            "component <Card title?:string = \"Untitled\" /> = { state { note?:string = \"x\" } <Label text={title} /> }",
            "write `title:string = \"Untitled\"` or `title?:string`",
        ),
        (
            "type Book = { subtitle?:string = { \"a very long default that will not be quoted in full\" } }",
            "write `subtitle:string = ...` or `subtitle?:string`",
        ),
    ] {
        let errors = errors(source);
        assert!(
            errors.iter().any(|error| error.contains(expected)),
            "expected {expected:?} in {errors:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Reading an optional property yields an occurrence that admits zero
// ---------------------------------------------------------------------------

#[test]
fn an_optional_field_reads_as_optional() {
    let source = "type Book = { subtitle?:string tags?:string+ }\nlet f(b:Book) = { b.subtitle }\nlet g(b:Book) = { b.tags }";
    assert_clean(source);
    assert_eq!(body_type(source, "f"), Type::optional(Type::string()));
    assert_eq!(body_type(source, "g"), Type::zero_or_more(Type::string()));
}

#[test]
fn an_optional_parameter_reads_as_optional_inside_its_function() {
    let errors = errors("let greet(name?:string): string = { \"Hi \" + name }");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("string?"), "{errors:?}");
}

// ---------------------------------------------------------------------------
// Construction treats an optional property that is not written as empty
// ---------------------------------------------------------------------------

#[test]
fn an_omitted_optional_property_is_empty() {
    let source = "type Book = { title:string subtitle?:string }\nlet b = <Book title=\"A\" />\nlet s = { b.subtitle }";
    assert_clean(source);
    assert_eq!(binding_type(source, "s"), Type::optional(Type::string()));
}

#[test]
fn a_defaulted_property_that_is_not_written_takes_its_default() {
    assert_clean(
        "type Book = { title:string = \"Untitled\" }\nlet b = <Book />\nlet t:string = { b.title }",
    );
}

#[test]
fn a_required_property_that_is_not_written_is_still_reported() {
    let errors = errors("type Book = { title:string }\nlet b = <Book />");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("title"), "{errors:?}");
}

#[test]
fn an_optional_property_accepts_what_admits_zero() {
    assert_clean(
        "type Book = { subtitle?:string tags?:string+ }\nlet o:string? = {}\nlet ts:string* = {}\nlet b = <Book subtitle={o} tags={ts} />\nlet c = <Book subtitle={} tags={} />",
    );
}

#[test]
fn a_defaulted_property_rejects_the_empty_value() {
    let errors = errors("type Book = { title:string = \"Untitled\" }\nlet b = <Book title={} />");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("{}") && errors[0].contains("string"),
        "{errors:?}"
    );
}

#[test]
fn a_one_or_more_property_rejects_a_value_that_may_be_empty() {
    let errors = errors(
        "type Box = { items:string+ }\nlet c = true\nlet maybe:string* = {}\nlet b = <Box items={maybe} />\nlet d = <Box items={if c { \"a\" }} />",
    );
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(
        errors[0].contains("string*") && errors[0].contains("string+"),
        "{errors:?}"
    );
    assert!(
        errors[1].contains("string?") && errors[1].contains("string+"),
        "{errors:?}"
    );
}

#[test]
fn an_empty_body_is_not_written_and_a_written_body_may_be_empty() {
    assert_clean(
        "type A = { n:int = 1 }\ntype Box = { content items:A+ = { <A/> } }\nlet b = <Box></Box>",
    );
    let errors = errors(
        "type A = { n:int = 1 }\ntype Box = { content items:A+ = { <A/> } }\nlet c = true\nlet b = <Box>{if c { <A/> }}</Box>",
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("A?") && errors[0].contains("A+"),
        "{errors:?}"
    );
}

// ---------------------------------------------------------------------------
// Optional state initializes empty
// ---------------------------------------------------------------------------

#[test]
fn optional_state_starts_empty() {
    assert_clean(
        "external component <Label text:string />\ncomponent <Search /> = { state { query?:string } <Label text={query ?? \"\"} /> }",
    );
}

#[test]
fn the_empty_value_at_a_defaulted_one_or_more_field_is_rejected() {
    // It is not silently read as "not written", which would bind the default.
    let errors = errors("type Box = { tags:string+ = {\"a\"} }\nlet b = <Box tags={} />");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("{}") && errors[0].contains("string+"),
        "{errors:?}"
    );
}
