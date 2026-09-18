//! Text body content at a `string` content property.
//!
//! One case per evaluation scenario of the `implicit-primitive-conversions` requirement *Text body
//! content binds to a string content property as one string*. The join is decided by type analysis
//! and written into the module as a `Concat` chain, so every test evaluates the analyzed module.

use nx_interpreter::{Interpreter, Value};
use smol_str::SmolStr;

const LABEL: &str = "type Label = { content text:string }\n";

/// Evaluates `function` of the analyzed `source` and returns the `text` its `Label` was bound.
fn label_text(source: &str, function: &str, args: Vec<Value>) -> String {
    let checked = nx_types::check_str(source, "test.nx");
    assert!(
        checked.errors().is_empty(),
        "expected the source to type check, got {:?}",
        checked
            .errors()
            .iter()
            .map(|diagnostic| diagnostic.message().to_string())
            .collect::<Vec<_>>()
    );
    let module = checked.lowered_module.expect("lowered module");
    let result = Interpreter::new()
        .execute_function(&module, function, args)
        .unwrap_or_else(|error| panic!("evaluation failed: {error}"));

    match result {
        Value::Record { type_name, fields } => {
            assert_eq!(type_name.as_str(), "Label");
            match fields.get("text") {
                Some(Value::String(text)) => text.to_string(),
                other => panic!("expected `text` to be one string, got {:?}", other),
            }
        }
        other => panic!("expected a Label record, got {:?}", other),
    }
}

fn string(value: &str) -> Value {
    Value::String(SmolStr::new(value))
}

#[test]
fn text_and_a_braced_int_bind_as_one_string() {
    let source = format!("{LABEL}let f(count:int) = <Label>Total: {{count}}</Label>");
    assert_eq!(label_text(&source, "f", vec![Value::Int(3)]), "Total: 3");
}

#[test]
fn interior_whitespace_between_braced_values_is_kept() {
    let source =
        format!("{LABEL}let f(first:string, last:string) = <Label>{{first}} {{last}}</Label>");
    assert_eq!(
        label_text(&source, "f", vec![string("Ada"), string("Lovelace")]),
        "Ada Lovelace"
    );
}

#[test]
fn surrounding_whitespace_from_layout_is_removed() {
    let source =
        format!("{LABEL}let f(count:int) =\n  <Label>\n    Total: {{count}}\n  </Label>\n");
    assert_eq!(label_text(&source, "f", vec![Value::Int(3)]), "Total: 3");
}

#[test]
fn text_after_a_braced_value_keeps_the_space_before_it() {
    let source = format!("{LABEL}let f(count:int) = <Label>{{count}} items left</Label>");
    assert_eq!(
        label_text(&source, "f", vec![Value::Int(3)]),
        "3 items left"
    );
}

#[test]
fn every_stringifiable_primitive_is_rendered_in_its_canonical_form() {
    let source = format!(
        "{LABEL}let f(ratio:float64, width:float32, on:boolean) = \
         <Label>{{ratio}} / {{width}} / {{on}}</Label>"
    );
    assert_eq!(
        label_text(
            &source,
            "f",
            vec![Value::Float(1.0), Value::Float32(0.1), Value::Boolean(true)]
        ),
        "1 / 0.1 / true"
    );
}

#[test]
fn a_body_of_one_text_run_binds_as_that_text() {
    let source = format!("{LABEL}let f() = <Label>Just text</Label>");
    assert_eq!(label_text(&source, "f", vec![]), "Just text");
}

#[test]
fn a_single_braced_string_binds_as_it_always_has() {
    let source = format!("{LABEL}let f(name:string) = <Label>{{name}}</Label>");
    assert_eq!(label_text(&source, "f", vec![string("Ada")]), "Ada");
}

#[test]
fn a_single_braced_number_binds_as_its_text() {
    let source = format!("{LABEL}let f(count:int) = <Label>{{count}}</Label>");
    assert_eq!(label_text(&source, "f", vec![Value::Int(3)]), "3");

    let literal = format!("{LABEL}let f() = <Label>{{1.0}}</Label>");
    assert_eq!(label_text(&literal, "f", vec![]), "1");
}

#[test]
fn a_braced_string_at_either_end_keeps_its_spaces() {
    let source =
        format!("{LABEL}let f(count:int) = <Label>{{\"  pad \"}}{{count}}{{\" end  \"}}</Label>");
    assert_eq!(
        label_text(&source, "f", vec![Value::Int(2)]),
        "  pad 2 end  "
    );
}

#[test]
fn line_breaks_between_pieces_read_as_one_space() {
    let source = format!(
        "{LABEL}let f(count:int) =\n  <Label>\n    Total:\n    {{count}}\n    items\n  </Label>\n"
    );
    assert_eq!(
        label_text(&source, "f", vec![Value::Int(3)]),
        "Total: 3 items"
    );

    let names = format!(
        "{LABEL}let f(first:string, last:string) =\n  <Label>\n    {{first}}\n    {{last}}\n  </Label>\n"
    );
    assert_eq!(
        label_text(&names, "f", vec![string("Ada"), string("Lovelace")]),
        "Ada Lovelace"
    );
}

#[test]
fn line_breaks_inside_a_text_run_read_as_one_space() {
    let source = format!(
        "{LABEL}let f() =\n  <Label>\n    Two  spaces\n      and a wrapped line\n  </Label>\n"
    );
    assert_eq!(
        label_text(&source, "f", vec![]),
        "Two  spaces and a wrapped line"
    );
}

#[test]
fn a_braced_line_break_and_raw_text_are_kept() {
    let source = format!("{LABEL}let f() = <Label>first{{\"\n\"}}second</Label>");
    assert_eq!(label_text(&source, "f", vec![]), "first\nsecond");

    let raw = format!("{LABEL}let f() = <Label:raw>\n  a {{ b }}\n    c\n</Label>");
    assert_eq!(label_text(&raw, "f", vec![]), "\n  a { b }\n    c\n");
}

#[test]
fn a_typed_text_body_keeps_its_line_breaks_and_loses_its_indentation() {
    // A typed body is text for a processor the host supplies, so its blank lines and list markers
    // survive; only the indentation the source is written at comes off.
    let source = format!(
        "{LABEL}let f(count:int) =\n  \
         <Label:markdown>\n    # Title\n\n    Total: @{{count}}\n\n    - one\n    - two\n  </Label>\n"
    );
    assert_eq!(
        label_text(&source, "f", vec![Value::Int(3)]),
        "# Title\n\nTotal: 3\n\n- one\n- two"
    );

    // A deeper line keeps the indentation past the common one, which is what a nested list needs.
    let nested =
        format!("{LABEL}let f() =\n  <Label:markdown>\n    - one\n      - nested\n  </Label>\n");
    assert_eq!(label_text(&nested, "f", vec![]), "- one\n  - nested");

    let text_only = format!("{LABEL}let f() = <Label:markdown>Just text</Label>");
    assert_eq!(label_text(&text_only, "f", vec![]), "Just text");
}

#[test]
fn a_plain_body_still_reads_its_line_breaks_as_layout() {
    // Only a typed body keeps its line breaks: a plain one reads them as layout, as before.
    let source = format!(
        "{LABEL}let f(count:int) =\n  <Label>\n    Total: {{count}}\n\n    items\n  </Label>\n"
    );
    assert_eq!(
        label_text(&source, "f", vec![Value::Int(3)]),
        "Total: 3 items"
    );
}

#[test]
fn an_escape_in_a_typed_text_body_binds_as_the_character_it_escapes() {
    let source =
        format!("{LABEL}let f(n:int) = <Label:markdown>at \\@ sign \\{{x\\}} @{{n}}</Label>");
    assert_eq!(
        label_text(&source, "f", vec![Value::Int(2)]),
        "at @ sign {x} 2"
    );
}
