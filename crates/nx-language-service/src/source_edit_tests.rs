use super::*;
use crate::source_edit::{
    attribute_element, element_properties, is_numeric_literal, is_whole_value, spell_value,
};
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

/// Over every `.nx` file in the repository: adding an unwritten property of an element, with its
/// default, yields a document with no new parse errors whose keys are the old ones and the new
/// attribute's.
#[test]
fn adding_defaults_over_the_repository() {
    let mut files = Vec::new();
    nx_files(&repository(), &mut files);
    files.sort();
    let mut checked = 0;
    for file in files {
        let source = std::fs::read_to_string(&file).expect("source");
        let snapshot = snapshot_for(&source);
        let uri = DocumentUri::from(URI);
        let tree = snapshot.source_tree(&uri).expect("tree");
        let errors = parse_errors(&source);
        let mut per_file = 0;
        for (at, node) in tree.nodes.iter().enumerate() {
            if per_file == 6 {
                break;
            }
            if node.role != SourceRole::Element {
                continue;
            }
            let Some(properties) = element_properties(&tree, node) else {
                continue;
            };
            // What the element writes, under a property-list condition too.
            let written = tree
                .nodes
                .iter()
                .filter(|child| {
                    child.role == SourceRole::Attribute
                        && attribute_element(&tree, child) == Some(at)
                })
                .filter_map(|child| child.name.clone())
                .collect::<Vec<_>>();
            let Some((property, value)) = properties.into_iter().find_map(|property| {
                if written.contains(&property.name) || property.flags.contains(&SourceFlag::Content)
                {
                    return None;
                }
                let default = property.default.as_deref()?;
                let value = if default == "true" || default == "false" {
                    SourceEditValue::Boolean {
                        value: default == "true",
                    }
                } else if default.starts_with('"')
                    && default.ends_with('"')
                    && !default.contains('\\')
                    && default.len() >= 2
                {
                    SourceEditValue::String {
                        value: default[1..default.len() - 1].to_string(),
                    }
                } else if is_numeric_literal(default) {
                    SourceEditValue::Number {
                        text: default.to_string(),
                    }
                } else if is_case_name(default) {
                    SourceEditValue::Case {
                        name: default.to_string(),
                    }
                } else {
                    return None;
                };
                Some((property.name.clone(), value))
            }) else {
                continue;
            };
            let answer = snapshot
                .source_edit(&uri, &node.key, &add(&property, value))
                .expect("edit");
            assert_eq!(
                answer.refused,
                None,
                "{} {} {property}",
                file.display(),
                node.key
            );
            let after = apply(&source, &answer);
            assert!(
                parse_errors(&after) <= errors,
                "{} {} {property}: {after}",
                file.display(),
                node.key
            );
            let after_tree = snapshot_for(&after).source_tree(&uri).expect("tree");
            let added = format!("{}.{property}", node.key);
            let after_keys = after_tree
                .nodes
                .iter()
                .map(|node| node.key.as_str())
                .filter(|key| *key != added && !key.starts_with(&format!("{added}.")))
                .collect::<Vec<_>>();
            let keys = tree
                .nodes
                .iter()
                .map(|node| node.key.as_str())
                .collect::<Vec<_>>();
            assert_eq!(
                after_keys,
                keys,
                "{} {} {property}: {after}",
                file.display(),
                node.key
            );
            assert!(
                after_tree.nodes.iter().any(|node| node.key == added),
                "{} {added}: {after}",
                file.display()
            );
            checked += 1;
            per_file += 1;
        }
    }
    assert!(checked > 20, "checked {checked}");
}

// --- Adding a property --------------------------------------------------------------------------

fn add(name: &str, value: SourceEditValue) -> SourceEdit {
    SourceEdit::AddProperty {
        name: name.to_string(),
        value,
    }
}

/// Whether a default is written as a bare case name, such as `medium`.
fn is_case_name(text: &str) -> bool {
    text.starts_with(|first: char| first.is_ascii_alphabetic() || first == '_')
        && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !matches!(text, "true" | "false" | "null")
}

const ADDING: &str = r##"type Color = string
type Drink = {
  code:string
  name:string
  iced:boolean = false
  price?:float64
  color?:Color
}
type Label = { tone?:string content text:string }
let accent:Color = "#c8a27a"
let theme = <Drink code="T" name="Theme" color="#5a3a22" />
let one = <Drink code="LAT" name="Latte" />
let lines = <Drink
    code="CBR"
    name="Cold brew" // the name
  />
let closing = <Drink
    code="CBR"
    name="Cold brew" />
let bare = <Label>Hi</Label>
"##;

#[test]
fn adding_on_one_line() {
    assert!(edited(
        ADDING,
        "one.value",
        add("iced", SourceEditValue::Boolean { value: true })
    )
    .contains(r#"let one = <Drink code="LAT" name="Latte" iced=true />"#));
}

#[test]
fn adding_a_property_per_line() {
    let after = edited(
        ADDING,
        "lines.value",
        add(
            "price",
            SourceEditValue::Number {
                text: "5.25".to_string(),
            },
        ),
    );
    assert!(
        after.contains("    name=\"Cold brew\" // the name\n    price=5.25\n  />"),
        "{after}"
    );
    let after = edited(
        ADDING,
        "closing.value",
        add(
            "price",
            SourceEditValue::Number {
                text: "-1".to_string(),
            },
        ),
    );
    assert!(
        after.contains("    name=\"Cold brew\"\n    price=-1 />"),
        "{after}"
    );
}

#[test]
fn adding_to_an_element_with_no_attributes() {
    let after = edited(
        ADDING,
        "bare.value",
        add(
            "tone",
            SourceEditValue::String {
                value: "warm".to_string(),
            },
        ),
    );
    assert!(
        after.contains(r#"let bare = <Label tone="warm">Hi</Label>"#),
        "{after}"
    );
}

#[test]
fn adding_what_cannot_be_added_is_refused() {
    for (key, name) in [
        ("one.value", "name"),
        ("one.value", "nope"),
        ("bare.value", "text"),
    ] {
        let answer = edit(
            ADDING,
            key,
            add(name, SourceEditValue::Boolean { value: true }),
        );
        assert_eq!(
            answer.refused,
            Some(SourceEditRefusal::InvalidValue),
            "{key} {name}"
        );
        assert!(answer.edits.is_empty());
    }
    let answer = edit(
        ADDING,
        "one.value.code.value",
        add("iced", SourceEditValue::Boolean { value: true }),
    );
    assert_eq!(answer.refused, Some(SourceEditRefusal::NotEditable));
}

#[test]
fn adding_after_a_value_over_several_lines() {
    let source = "type Drink = { code:string name:string iced:boolean = false }\nlet tag = <Drink code=\"M\" name=\"a\nb\" />\nlet rows = <Drink\n    code=\"M\"\n    name=\"a\nb\"\n  />\n";
    let after = edited(
        source,
        "tag.value",
        add("iced", SourceEditValue::Boolean { value: true }),
    );
    assert!(after.contains("name=\"a\nb\" iced=true />"), "{after}");
    let after = edited(
        source,
        "rows.value",
        add("iced", SourceEditValue::Boolean { value: true }),
    );
    assert!(
        after.contains("    name=\"a\nb\"\n    iced=true\n  />"),
        "{after}"
    );
}

#[test]
fn a_property_written_under_a_condition_is_not_added_again() {
    let source = "type Drink = { code:string iced:boolean = false price?:float64 }\nlet flag = true\nlet d = <Drink code=\"A\" if flag { iced=true } />\n";
    let answer = edit(
        source,
        "d.value",
        add("iced", SourceEditValue::Boolean { value: false }),
    );
    assert_eq!(answer.refused, Some(SourceEditRefusal::InvalidValue));
    // Another property goes after the element's own attributes.
    let after = edited(
        source,
        "d.value",
        add(
            "price",
            SourceEditValue::Number {
                text: "2".to_string(),
            },
        ),
    );
    assert!(
        after.contains(r#"<Drink code="A" price=2 if flag { iced=true } />"#),
        "{after}"
    );
}

#[test]
fn adding_to_a_union_cases_element() {
    let source = "type Extra = none | syrup { flavor:string pumps:int = 1 }\nlet promo = <Extra.syrup flavor=\"mocha\" />\n";
    let after = edited(
        source,
        "promo.value",
        add(
            "pumps",
            SourceEditValue::Number {
                text: "2".to_string(),
            },
        ),
    );
    assert!(
        after.contains(r#"<Extra.syrup flavor="mocha" pumps=2 />"#),
        "{after}"
    );
    let answer = edit(
        source,
        "promo.value",
        add(
            "flavor",
            SourceEditValue::String {
                value: "x".to_string(),
            },
        ),
    );
    assert_eq!(answer.refused, Some(SourceEditRefusal::InvalidValue));
}

// --- References ---------------------------------------------------------------------------------

#[test]
fn a_reference_is_not_editable() {
    let source = ADDING.replace(r##"color="#5a3a22""##, "color={accent}");
    let snapshot = snapshot_for(&source);
    let uri = DocumentUri::from(URI);
    let tree = snapshot.source_tree(&uri).expect("tree");
    let reference = tree
        .nodes
        .iter()
        .find(|node| node.key.starts_with("theme.value.color.value"))
        .expect("the reference's node");
    let answer = edit(&source, &reference.key, string("#5a3a22"));
    assert_eq!(
        answer.refused,
        Some(SourceEditRefusal::NotEditable),
        "{}",
        reference.key
    );
    assert!(answer.edits.is_empty());
}

const FLOW: &str = r#"type Step = { id:string label?:string }
type Choice = { value:string }
type QuestionFlow = { content steps:Step* }
type Select = { choices:Choice+ }
let wide = true
let flow = <QuestionFlow>
  <Step id="A" />
  <Step
    id="B"
    label="Bee"
  />
  // Ask this first
  <Step id="C" /> // see C
</QuestionFlow>
let spaced = <QuestionFlow>
  <Step id="A" />

  <Step id="B" />

  <Step id="C" />
</QuestionFlow>
let branches = <QuestionFlow>
  <Step id="A" />
  if wide {
    <Step id="B" />
  }
  <Step id="C" />
</QuestionFlow>
let inline = <Select choices={ <Choice value="a" /> <Choice value="b" /> } />
let listed = <Select choices={
  <Choice value="a" />
  /// The second
  <Choice value="b" />
  <Choice value="c" />
} />
let mixed = <Select choices={ <Choice value="a" />
  <Choice value="b" /> } />
"#;

fn move_to(to: usize) -> SourceEdit {
    SourceEdit::Move { to }
}

/// The text between `from` and `to` in `source`, the first of each.
fn between<'s>(source: &'s str, from: &str, to: &str) -> &'s str {
    let start = source.find(from).expect("from") + from.len();
    let end = start + source[start..].find(to).expect("to");
    &source[start..end]
}

#[test]
fn a_step_moves_down() {
    let after = edited(FLOW, "flow.value.steps[0]", move_to(1));
    assert_eq!(
        between(&after, "let flow = <QuestionFlow>\n", "</QuestionFlow>"),
        "  <Step\n    id=\"B\"\n    label=\"Bee\"\n  />\n  <Step id=\"A\" />\n  // Ask this first\n  <Step id=\"C\" /> // see C\n"
    );
    // Nothing outside the list changes.
    assert_eq!(after.len(), FLOW.len());
    assert_eq!(
        between(&after, "let spaced", "let inline"),
        between(FLOW, "let spaced", "let inline")
    );
}

#[test]
fn its_comment_goes_with_it() {
    let after = edited(FLOW, "flow.value.steps[2]", move_to(0));
    assert_eq!(
        between(&after, "let flow = <QuestionFlow>\n", "</QuestionFlow>"),
        "  // Ask this first\n  <Step id=\"C\" /> // see C\n  <Step id=\"A\" />\n  <Step\n    id=\"B\"\n    label=\"Bee\"\n  />\n"
    );
}

#[test]
fn a_doc_comment_goes_with_it() {
    let after = edited(FLOW, "listed.value.choices.value[1]", move_to(2));
    assert_eq!(
        between(&after, "let listed = <Select choices={\n", "} />"),
        "  <Choice value=\"a\" />\n  <Choice value=\"c\" />\n  /// The second\n  <Choice value=\"b\" />\n"
    );
}

#[test]
fn moving_on_one_line() {
    let after = edited(FLOW, "inline.value.choices.value[0]", move_to(1));
    assert!(after.contains(
        "let inline = <Select choices={ <Choice value=\"b\" /> <Choice value=\"a\" /> } />"
    ));
    let after = edited(FLOW, "inline.value.choices.value[1]", move_to(0));
    assert!(after.contains(
        "let inline = <Select choices={ <Choice value=\"b\" /> <Choice value=\"a\" /> } />"
    ));
}

#[test]
fn moving_past_a_condition() {
    let after = edited(FLOW, "branches.value.steps[2]", move_to(1));
    assert_eq!(
        between(&after, "let branches = <QuestionFlow>\n", "</QuestionFlow>"),
        "  <Step id=\"A\" />\n  <Step id=\"C\" />\n  if wide {\n    <Step id=\"B\" />\n  }\n"
    );
}

#[test]
fn blank_lines_between_items_stay() {
    let body = |source: &str| {
        between(source, "let spaced = <QuestionFlow>\n", "</QuestionFlow>").to_string()
    };
    let down = edited(FLOW, "spaced.value.steps[0]", move_to(1));
    assert_eq!(
        body(&down),
        "  <Step id=\"B\" />\n\n  <Step id=\"A\" />\n\n  <Step id=\"C\" />\n"
    );
    let to_end = edited(FLOW, "spaced.value.steps[0]", move_to(2));
    assert_eq!(
        body(&to_end),
        "  <Step id=\"B\" />\n\n  <Step id=\"C\" />\n\n  <Step id=\"A\" />\n"
    );
    let up = edited(FLOW, "spaced.value.steps[2]", move_to(0));
    assert_eq!(
        body(&up),
        "  <Step id=\"C\" />\n\n  <Step id=\"A\" />\n\n  <Step id=\"B\" />\n"
    );
}

#[test]
fn moving_to_where_it_is() {
    let answer = edit(FLOW, "flow.value.steps[1]", move_to(1));
    assert_eq!(answer.refused, None);
    assert!(answer.edits.is_empty());
}

#[test]
fn moving_what_cannot_move_is_refused() {
    let refused = |key: &str, to: usize| edit(FLOW, key, move_to(to)).refused;
    // An item an `if` makes.
    assert_eq!(
        refused("branches.value.steps[1].then[0]", 0),
        Some(SourceEditRefusal::NotEditable)
    );
    // Not an element, though an item.
    assert_eq!(
        refused("branches.value.steps[1]", 0),
        Some(SourceEditRefusal::NotEditable)
    );
    // Not an item.
    assert_eq!(
        refused("flow.value", 0),
        Some(SourceEditRefusal::NotEditable)
    );
    assert_eq!(
        refused("flow.value.steps[0].id", 0),
        Some(SourceEditRefusal::NotEditable)
    );
    // Past the end.
    assert_eq!(
        refused("flow.value.steps[0]", 3),
        Some(SourceEditRefusal::InvalidValue)
    );
    // Neither an item to a line nor on one line.
    assert_eq!(
        refused("mixed.value.choices.value[0]", 1),
        Some(SourceEditRefusal::NotEditable)
    );
    // A list that holds text.
    let text = "type T = { content items:string* }\ntype I = {}\nlet t = <T>Hi <I /> <I /></T>\n";
    let answer = edit(text, "t.value.items[1]", move_to(2));
    assert_eq!(answer.refused, Some(SourceEditRefusal::NotEditable));
}

#[test]
fn removing_a_step_and_its_comment() {
    let after = edited(FLOW, "flow.value.steps[2]", SourceEdit::Remove);
    assert_eq!(
        between(&after, "let flow = <QuestionFlow>\n", "</QuestionFlow>"),
        "  <Step id=\"A\" />\n  <Step\n    id=\"B\"\n    label=\"Bee\"\n  />\n"
    );
    let after = edited(FLOW, "flow.value.steps[1]", SourceEdit::Remove);
    assert_eq!(
        between(&after, "let flow = <QuestionFlow>\n", "</QuestionFlow>"),
        "  <Step id=\"A\" />\n  // Ask this first\n  <Step id=\"C\" /> // see C\n"
    );
}

#[test]
fn removing_leaves_no_doubled_blank_line() {
    let body = |source: &str| {
        between(source, "let spaced = <QuestionFlow>\n", "</QuestionFlow>").to_string()
    };
    let first = edited(FLOW, "spaced.value.steps[0]", SourceEdit::Remove);
    assert_eq!(body(&first), "  <Step id=\"B\" />\n\n  <Step id=\"C\" />\n");
    let middle = edited(FLOW, "spaced.value.steps[1]", SourceEdit::Remove);
    assert_eq!(
        body(&middle),
        "  <Step id=\"A\" />\n\n  <Step id=\"C\" />\n"
    );
    let last = edited(FLOW, "spaced.value.steps[2]", SourceEdit::Remove);
    assert_eq!(body(&last), "  <Step id=\"A\" />\n\n  <Step id=\"B\" />\n");
}

#[test]
fn removing_an_item_on_one_line() {
    let after = edited(FLOW, "inline.value.choices.value[0]", SourceEdit::Remove);
    assert!(after.contains("let inline = <Select choices={ <Choice value=\"b\" /> } />"));
    let after = edited(FLOW, "inline.value.choices.value[1]", SourceEdit::Remove);
    assert!(after.contains("let inline = <Select choices={ <Choice value=\"a\" /> } />"));
}

#[test]
fn removing_an_item_with_crlf_line_breaks() {
    let source = FLOW.replace('\n', "\r\n");
    let after = edited(&source, "flow.value.steps[2]", SourceEdit::Remove);
    assert!(after.contains("  />\r\n</QuestionFlow>"));
    let after = edited(&source, "flow.value.steps[2]", move_to(0));
    assert!(after.contains(
        "<QuestionFlow>\r\n  // Ask this first\r\n  <Step id=\"C\" /> // see C\r\n  <Step id=\"A\" />"
    ));
}

#[test]
fn moving_the_only_item() {
    let source = "type S = {}\ntype F = { content steps:S* }\nlet f = <F>\n  <S />\n</F>\n";
    assert!(edit(source, "f.value.steps[0]", move_to(0))
        .edits
        .is_empty());
    assert_eq!(
        edit(source, "f.value.steps[0]", move_to(1)).refused,
        Some(SourceEditRefusal::InvalidValue)
    );
    assert_eq!(
        edited(source, "f.value.steps[0]", SourceEdit::Remove),
        "type S = {}\ntype F = { content steps:S* }\nlet f = <F>\n</F>\n"
    );
}

/// The keys of a tree outside the list keyed `list`, without comments, which are keyed by what
/// follows them.
fn keys_outside(tree: &crate::SourceTree, list: &str) -> Vec<String> {
    tree.nodes
        .iter()
        .filter(|node| !matches!(node.role, SourceRole::Comment | SourceRole::DocComment))
        .map(|node| node.key.clone())
        .filter(|key| {
            key != list
                && !key.starts_with(&format!("{list}["))
                && !key.starts_with(&format!("{list}."))
        })
        .collect()
}

/// The text of each item of the list keyed `list`, in order.
fn item_texts(source: &str, tree: &crate::SourceTree, list: &str) -> Vec<String> {
    let mut items = tree
        .nodes
        .iter()
        .filter_map(|node| {
            let position = node
                .key
                .strip_prefix(list)?
                .strip_prefix('[')?
                .strip_suffix(']')?
                .parse::<usize>()
                .ok()?;
            Some((position, node))
        })
        .collect::<Vec<_>>();
    items.sort_by_key(|(position, _)| *position);
    items
        .into_iter()
        .map(|(_, node)| {
            source[node.range.start_byte as usize..node.range.end_byte as usize].to_string()
        })
        .collect()
}

/// Over every `.nx` file in the repository: every item that moves, moved to each position, yields a
/// document with no new parse errors, its list's items in the new order and every key outside the
/// list unchanged; removing it yields one without it.
#[test]
fn moving_and_removing_items_over_the_repository() {
    let mut files = Vec::new();
    nx_files(&repository(), &mut files);
    files.sort();
    let uri = DocumentUri::from(URI);
    let mut moved = 0;
    let mut removed = 0;
    for file in files {
        let source = std::fs::read_to_string(&file).expect("source");
        let snapshot = snapshot_for(&source);
        let tree = snapshot.source_tree(&uri).expect("tree");
        let errors = parse_errors(&source);
        // A few lists per file keep the test quick.
        let mut lists: Vec<String> = Vec::new();
        for node in &tree.nodes {
            if node.role != SourceRole::Element {
                continue;
            }
            let Some(owner) = node.parent.map(|owner| &tree.nodes[owner as usize]) else {
                continue;
            };
            if lists.contains(&owner.key) {
                continue;
            }
            let answer = snapshot
                .source_edit(&uri, &node.key, &move_to(0))
                .expect("edit");
            if answer.refused.is_some() {
                assert_eq!(
                    answer.refused,
                    Some(SourceEditRefusal::NotEditable),
                    "{} {}",
                    file.display(),
                    node.key
                );
                continue;
            }
            if lists.len() == 2 {
                break;
            }
            lists.push(owner.key.clone());
        }
        for list in lists {
            let items = item_texts(&source, &tree, &list);
            let count = items.len();
            let outside = keys_outside(&tree, &list);
            for from in 0..count.min(4) {
                let key = format!("{list}[{from}]");
                if tree
                    .nodes
                    .iter()
                    .all(|node| node.key != key || node.role != SourceRole::Element)
                {
                    continue;
                }
                // To each end and a step either way.
                let mut targets = vec![
                    0,
                    count - 1,
                    from.saturating_sub(1),
                    (from + 1).min(count - 1),
                ];
                targets.sort_unstable();
                targets.dedup();
                for to in targets {
                    let answer = snapshot
                        .source_edit(&uri, &key, &move_to(to))
                        .expect("edit");
                    assert_eq!(answer.refused, None, "{} {key} to {to}", file.display());
                    let after = apply(&source, &answer);
                    assert!(
                        parse_errors(&after) <= errors,
                        "{} {key} to {to}: {after}",
                        file.display()
                    );
                    let after_tree = snapshot_for(&after).source_tree(&uri).expect("tree");
                    let mut expected = items.clone();
                    let item = expected.remove(from);
                    expected.insert(to, item);
                    assert_eq!(
                        item_texts(&after, &after_tree, &list),
                        expected,
                        "{} {key} to {to}: {after}",
                        file.display()
                    );
                    assert_eq!(
                        keys_outside(&after_tree, &list),
                        outside,
                        "{} {key} to {to}",
                        file.display()
                    );
                    moved += 1;
                }
                let answer = snapshot
                    .source_edit(&uri, &key, &SourceEdit::Remove)
                    .expect("edit");
                assert_eq!(answer.refused, None, "{} {key}", file.display());
                let after = apply(&source, &answer);
                assert!(
                    parse_errors(&after) <= errors,
                    "{} {key}: {after}",
                    file.display()
                );
                let after_tree = snapshot_for(&after).source_tree(&uri).expect("tree");
                assert_eq!(
                    keys_outside(&after_tree, &list),
                    outside,
                    "{} {key} removed",
                    file.display()
                );
                if count > 2 {
                    let mut expected = items.clone();
                    expected.remove(from);
                    assert_eq!(
                        item_texts(&after, &after_tree, &list),
                        expected,
                        "{} {key}: {after}",
                        file.display()
                    );
                }
                removed += 1;
            }
        }
    }
    assert!(
        moved > 100 && removed > 20,
        "moved {moved}, removed {removed}"
    );
}
