use super::*;
use crate::source_edit::{is_numeric_literal, is_whole_value, spell_value};
use std::path::{Path, PathBuf};

const URI: &str = "nx://tenant/edit.nx";

const DRINKS: &str = r#"type Size = small | medium | large
type Drink = {
  code:string
  name:string
  size:Size = medium
  iced:boolean = false
  padding?:float64
  title?:string
}
type Label = { content text:string }
let primes:int+ = { 2 3 5 7 }
let total = 4
let root(): Drink+ = {
  <Drink code="LAT" name="Latte" />
  <Drink code="CBR" name="Cold brew" size=large iced={true} padding=8 />
  <Drink
    code="X"
    title="Hi"
    name={"a" + "b"}
  />
}
let label = <Label>Hello</Label>
let count = <Drink code="C" name="n" padding={total + 1} />
"#;

fn snapshot_for(source: &str) -> WorkspaceSnapshot {
    WorkspaceSnapshot::from_documents(
        Option::<PathBuf>::None,
        vec![DocumentInput::new(URI, source).with_version(1)],
    )
    .expect("snapshot")
}

fn edit(source: &str, key: &str, edit: SourceEdit) -> SourceEditAnswer {
    snapshot_for(source)
        .source_edit(&DocumentUri::from(URI), key, &edit)
        .expect("source edit")
}

/// Applies an answer's edits to `source`: every range is in `source` as sent, and none overlap.
fn apply(source: &str, answer: &SourceEditAnswer) -> String {
    let mut text = source.to_string();
    for edit in answer.edits.iter().rev() {
        let start = edit.range.start_byte as usize;
        let end = edit.range.end_byte as usize;
        text.replace_range(start..end, &edit.new_text);
    }
    text
}

/// The document after `edit`, which must be made.
fn edited(source: &str, key: &str, change: SourceEdit) -> String {
    let answer = edit(source, key, change);
    assert_eq!(answer.refused, None, "{key} was refused");
    apply(source, &answer)
}

fn string(value: &str) -> SourceEdit {
    SourceEdit::SetValue {
        value: SourceEditValue::String {
            value: value.to_string(),
        },
    }
}

fn number(text: &str) -> SourceEdit {
    SourceEdit::SetValue {
        value: SourceEditValue::Number {
            text: text.to_string(),
        },
    }
}

fn case(name: &str) -> SourceEdit {
    SourceEdit::SetValue {
        value: SourceEditValue::Case {
            name: name.to_string(),
        },
    }
}

fn boolean(value: bool) -> SourceEdit {
    SourceEdit::SetValue {
        value: SourceEditValue::Boolean { value },
    }
}

fn text(text: &str) -> SourceEdit {
    SourceEdit::SetText {
        text: text.to_string(),
    }
}

#[test]
fn applying_the_answer_changes_only_the_target() {
    let after = edited(DRINKS, "root.body[0].name.value", string("Flat white"));
    assert_eq!(
        after,
        DRINKS.replace(r#"name="Latte""#, r#"name="Flat white""#)
    );
}

#[test]
fn the_edit_carries_protocol_ranges() {
    let answer = edit(DRINKS, "root.body[0].name.value", string("Flat white"));
    let range = answer.edits[0].range;
    let line = DRINKS.lines().nth(range.start.line as usize).unwrap();
    assert_eq!(
        &line[range.start.character as usize..range.end.character as usize],
        "\"Latte\""
    );
}

#[test]
fn no_change_answers_no_edits() {
    let answer = edit(DRINKS, "root.body[1].size.value", case("large"));
    assert_eq!(
        answer,
        SourceEditAnswer {
            edits: Vec::new(),
            refused: None
        }
    );
}

#[test]
fn a_string_with_a_quote_and_a_backslash() {
    let after = edited(
        DRINKS,
        "root.body[2].title.value",
        string(r#"Say "hi" \o/"#),
    );
    assert!(after.contains(r#"title="Say \"hi\" \\o/""#), "{after}");
}

#[test]
fn a_braced_boolean_keeps_its_braces() {
    let after = edited(DRINKS, "root.body[1].iced.value", boolean(false));
    assert!(after.contains("iced={false}"), "{after}");
}

#[test]
fn a_case() {
    let after = edited(DRINKS, "root.body[1].size.value", case("small"));
    assert!(after.contains("size=small iced"), "{after}");
}

#[test]
fn a_qualified_case_keeps_its_qualifier() {
    let source = "type Size = small | large\nlet s: Size = { Size.small }\n";
    let after = edited(source, "s.value", case("large"));
    assert_eq!(
        after,
        "type Size = small | large\nlet s: Size = { Size.large }\n"
    );
}

#[test]
fn a_number_kept_as_typed() {
    let after = edited(DRINKS, "root.body[1].padding.value", number("12.5"));
    assert!(after.contains("padding=12.5 />"), "{after}");
    let after = edited(DRINKS, "root.body[1].padding.value", number("-3"));
    assert!(after.contains("padding=-3 />"), "{after}");
}

#[test]
fn an_item_of_a_sequence_and_a_let_value() {
    let after = edited(DRINKS, "primes.value[1]", number("4"));
    assert!(after.contains("let primes:int+ = { 2 4 5 7 }"), "{after}");
    let after = edited(DRINKS, "total.value", number("5"));
    assert!(after.contains("let total = 5"), "{after}");
}

#[test]
fn text_is_written_as_it_is() {
    let after = edited(DRINKS, "label.value.text[0]", text("Fish & chips > all"));
    assert!(
        after.contains("<Label>Fish & chips > all</Label>"),
        "{after}"
    );
}

#[test]
fn text_that_would_not_read_back_is_refused() {
    for typed in [
        "a < b",
        "use {name}",
        "if only",
        "for you",
        "else what",
        "  padded",
    ] {
        let answer = edit(DRINKS, "label.value.text[0]", text(typed));
        assert_eq!(
            answer.refused,
            Some(SourceEditRefusal::InvalidValue),
            "{typed}"
        );
    }
}

#[test]
fn removing_on_one_line() {
    let after = edited(DRINKS, "root.body[1].size", SourceEdit::Remove);
    assert!(
        after.contains(r#"<Drink code="CBR" name="Cold brew" iced={true} padding=8 />"#),
        "{after}"
    );
    let after = edited(DRINKS, "root.body[1].padding", SourceEdit::Remove);
    assert!(after.contains("iced={true} />"), "{after}");
}

#[test]
fn removing_on_its_own_line() {
    let after = edited(DRINKS, "root.body[2].title", SourceEdit::Remove);
    assert_eq!(after, DRINKS.replace("    title=\"Hi\"\n", ""));
}

#[test]
fn removing_the_first_on_a_shared_line() {
    let source = "type T = { a?:int b?:int }\nlet v = <T\n  a=1 b=2\n/>\n";
    let after = edited(source, "v.value.a", SourceEdit::Remove);
    assert_eq!(after, "type T = { a?:int b?:int }\nlet v = <T\n  b=2\n/>\n");
}

#[test]
fn an_operand_is_not_editable() {
    let answer = edit(DRINKS, "count.value.padding.value.right", number("2"));
    assert_eq!(answer.refused, Some(SourceEditRefusal::NotEditable));
    assert!(answer.edits.is_empty());
}

#[test]
fn a_default_is_not_editable() {
    let answer = edit(DRINKS, "Drink.size.default", case("large"));
    assert_eq!(answer.refused, Some(SourceEditRefusal::NotEditable));
}

#[test]
fn an_unknown_key() {
    let answer = edit(DRINKS, "nowhere.value", number("1"));
    assert_eq!(answer.refused, Some(SourceEditRefusal::UnknownNode));
}

#[test]
fn values_that_cannot_be_spelled() {
    for (key, change) in [
        ("root.body[1].padding.value", number("12px")),
        ("root.body[1].padding.value", number("1e5")),
        ("root.body[1].padding.value", number("")),
        ("root.body[1].size.value", case("very large")),
        ("root.body[1].size.value", case("true")),
        ("label.value.text[0]", text("")),
        ("label.value.text[0]", text("two\nlines")),
    ] {
        let answer = edit(DRINKS, key, change.clone());
        assert_eq!(
            answer.refused,
            Some(SourceEditRefusal::InvalidValue),
            "{change:?}"
        );
    }
}

#[test]
fn numeric_literals() {
    for good in [
        "0", "12", "-3", "1.5", "-0.25", "1.5e3", "2.0E-4", "0xFF", "-0x1a",
    ] {
        assert!(
            spell_value(&SourceEditValue::Number { text: good.into() }).is_ok(),
            "{good}"
        );
    }
    for bad in [
        "", "-", "1.", ".5", "1e3", "0x", "1_000", "+1", "12px", "--1",
    ] {
        assert!(
            spell_value(&SourceEditValue::Number { text: bad.into() }).is_err(),
            "{bad}"
        );
    }
}

#[test]
fn the_wrong_edit_for_the_node() {
    // A value given to remove, an attribute given a value, a content property removed.
    for (key, change) in [
        ("root.body[0].name.value", SourceEdit::Remove),
        ("root.body[0].name", string("x")),
        ("label.value.text", SourceEdit::Remove),
        ("root.body[0].name.value", text("x")),
    ] {
        let answer = edit(DRINKS, key, change.clone());
        assert_eq!(
            answer.refused,
            Some(SourceEditRefusal::NotEditable),
            "{key} {change:?}"
        );
    }
}

#[test]
fn text_over_several_lines_and_typed_text_are_not_editable() {
    let source = "type Label = { content text:string }\nlet a = <Label>\n  one\n  two\n</Label>\n";
    let answer = edit(source, "a.value.text[0]", text("x"));
    assert_eq!(answer.refused, Some(SourceEditRefusal::NotEditable));
}

#[test]
fn the_answer_round_trips_through_json() {
    let answer = edit(DRINKS, "root.body[1].size", SourceEdit::Remove);
    let json = serde_json::to_string(&answer).unwrap();
    assert!(json.contains("\"newText\""), "{json}");
    let back: SourceEditAnswer = serde_json::from_str(&json).unwrap();
    assert_eq!(back, answer);
    let refused = serde_json::to_string(&edit(DRINKS, "x", SourceEdit::Remove)).unwrap();
    assert_eq!(refused, r#"{"edits":[],"refused":"unknownNode"}"#);
    let request: SourceEdit =
        serde_json::from_str(r#"{"kind":"setValue","value":{"kind":"case","name":"large"}}"#)
            .unwrap();
    assert_eq!(request, case("large"));
}

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn nx_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if name.starts_with('.') || name == "node_modules" || name == "target" || name == "dist"
            {
                continue;
            }
            nx_files(&path, out);
        } else if name.ends_with(".nx") {
            out.push(path);
        }
    }
}

#[test]
fn a_negative_number_only_where_the_grammar_takes_one() {
    // An unbraced attribute value or a `let` value may be signed.
    let after = edited(DRINKS, "total.value", number("-4"));
    assert!(after.contains("let total = -4"), "{after}");
    // A sequence item may not, nor a braced value, where `-` reads as an operator.
    let answer = edit(DRINKS, "primes.value[1]", number("-4"));
    assert_eq!(answer.refused, Some(SourceEditRefusal::InvalidValue));
    let source = "type T = { p:int }\nlet v = <T p={8} />\n";
    let answer = edit(source, "v.value.p.value", number("-3"));
    assert_eq!(answer.refused, Some(SourceEditRefusal::InvalidValue));
    let after = edited(source, "v.value.p.value", number("3"));
    assert!(after.contains("p={3}"), "{after}");
}

#[test]
fn a_bare_case_only_where_it_resolves() {
    // A braced literal set to a case would read as an undefined name.
    let source = "type Size = small | large\nlet s:string = {\"a\"}\n";
    let answer = edit(source, "s.value", case("large"));
    assert_eq!(answer.refused, Some(SourceEditRefusal::InvalidValue));
    let source = "type Size = small | large\nlet s:string+ = { \"a\" \"b\" }\n";
    let answer = edit(source, "s.value[0]", case("large"));
    assert_eq!(answer.refused, Some(SourceEditRefusal::InvalidValue));
}

#[test]
fn a_run_keeps_the_space_before_what_follows() {
    let source = "type L = { content text:string }\nlet l = <L>Hello {1} world</L>\n";
    let after = edited(source, "l.value.text[0]", text("Hi"));
    assert!(after.contains("<L>Hi {1} world</L>"), "{after}");
}

#[test]
fn removing_takes_a_trailing_comment_with_it() {
    let source = "type T = { a?:int b?:int }\nlet v = <T\n  a=1 // note\n  b=2\n/>\n";
    let after = edited(source, "v.value.a", SourceEdit::Remove);
    assert_eq!(after, "type T = { a?:int b?:int }\nlet v = <T\n  b=2\n/>\n");
    let source = "type T = { a?:int b?:int }\nlet v = <T a=1 // note\n  b=2 />\n";
    let after = edited(source, "v.value.a", SourceEdit::Remove);
    assert_eq!(after, "type T = { a?:int b?:int }\nlet v = <T\n  b=2 />\n");
}

#[test]
fn removing_keeps_a_comment_line_above() {
    let source = "type T = { a?:int b?:int }\nlet v = <T\n  // about a\n  a=1\n  b=2\n/>\n";
    let after = edited(source, "v.value.a", SourceEdit::Remove);
    assert_eq!(
        after,
        "type T = { a?:int b?:int }\nlet v = <T\n  // about a\n  b=2\n/>\n"
    );
}

/// The number of parse errors in `source`.
fn parse_errors(source: &str) -> usize {
    nx_syntax::parse_str(source, "edited.nx").errors.len()
}

/// Over every `.nx` file in the repository: each editable literal set to itself answers no edits,
/// and set to another value of its kind yields a document with the same keys and no new parse errors.
#[test]
fn round_trips_over_the_repository() {
    let mut files = Vec::new();
    nx_files(&repository(), &mut files);
    files.sort();
    assert!(files.len() > 20, "found {} files", files.len());
    let mut checked = 0;
    for file in files {
        let source = std::fs::read_to_string(&file).expect("source");
        let snapshot = snapshot_for(&source);
        let uri = DocumentUri::from(URI);
        let tree = snapshot.source_tree(&uri).expect("tree");
        let keys: Vec<&str> = tree.nodes.iter().map(|node| node.key.as_str()).collect();
        let errors = parse_errors(&source);
        // A few values per file keep the test quick; every file still takes part.
        let mut per_file = 0;
        for (at, node) in tree.nodes.iter().enumerate() {
            if per_file == 8 {
                break;
            }
            if !matches!(node.role, SourceRole::Literal | SourceRole::Case)
                || !is_whole_value(&tree, at)
            {
                continue;
            }
            let written = node
                .value
                .clone()
                .or_else(|| node.name.clone())
                .unwrap_or_default();
            let (same, other) = match node.role {
                SourceRole::Case => {
                    // Another case of the same union, when it has one with no properties.
                    let cases = node
                        .declaration
                        .and_then(|entry| tree.declarations.get(entry as usize))
                        .map(|entry| entry.cases.clone())
                        .unwrap_or_default();
                    let Some(other) = cases.iter().find(|candidate| {
                        candidate.name != written && candidate.properties.is_empty()
                    }) else {
                        continue;
                    };
                    (case(&written), case(&other.name))
                }
                _ if written == "true" || written == "false" => {
                    (boolean(written == "true"), boolean(written != "true"))
                }
                _ if written.starts_with('"') => {
                    // Only strings with no escapes are compared to themselves.
                    if written.contains('\\') {
                        continue;
                    }
                    (string(&written[1..written.len() - 1]), string("other"))
                }
                _ if is_numeric_literal(&written) => (number(&written), number("42")),
                _ => continue,
            };
            let answer = snapshot.source_edit(&uri, &node.key, &same).expect("edit");
            assert_eq!(
                answer,
                SourceEditAnswer {
                    edits: Vec::new(),
                    refused: None
                },
                "{} {}",
                file.display(),
                node.key
            );
            // A negative number is written where the grammar takes one, and refused elsewhere.
            let mut others = vec![other];
            if matches!(node.role, SourceRole::Literal) && is_numeric_literal(&written) {
                others.push(number("-42"));
            }
            for other in others {
                let answer = snapshot.source_edit(&uri, &node.key, &other).expect("edit");
                if answer.refused == Some(SourceEditRefusal::InvalidValue) && other == number("-42")
                {
                    continue;
                }
                assert_eq!(answer.refused, None, "{} {}", file.display(), node.key);
                let after = apply(&source, &answer);
                assert!(
                    parse_errors(&after) <= errors,
                    "{} {}: {after}",
                    file.display(),
                    node.key
                );
                let after_tree = snapshot_for(&after).source_tree(&uri).expect("tree");
                let after_keys: Vec<&str> = after_tree
                    .nodes
                    .iter()
                    .map(|node| node.key.as_str())
                    .collect();
                assert_eq!(after_keys, keys, "{} {}", file.display(), node.key);
            }
            checked += 1;
            per_file += 1;
        }
    }
    assert!(checked > 100, "checked {checked}");
}
