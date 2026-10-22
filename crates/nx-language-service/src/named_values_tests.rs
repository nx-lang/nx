use super::*;
use std::path::PathBuf;

const URI: &str = "nx://tenant/menu.nx";
const THEME_URI: &str = "nx://tenant/theme.nx";

const THEME: &str = r##"export type Color = string
export type Theme = { accent:Color muted:Color = "#888888" radius:int = 4 }
/// The brand's warm accent.
export let brand:Color = "#5a3a22"
export let theme = <Theme accent="#c8a27a" />
"##;

const MENU: &str = r##"import "./theme.nx"

type Drink = { name:string color?:Color size:int = 1 content note?:string }
/// The menu's highlight.
let accent:Color = "#c8a27a"
let count = 3
let latte = <Drink name="Latte" />
"##;

fn snapshot() -> WorkspaceSnapshot {
    WorkspaceSnapshot::from_documents(
        Option::<PathBuf>::None,
        vec![
            DocumentInput::new(THEME_URI, THEME).with_version(1),
            DocumentInput::new(URI, MENU).with_version(1),
        ],
    )
    .expect("snapshot")
}

fn paths(values: &[NamedValue]) -> Vec<&str> {
    values.iter().map(|value| value.path.as_str()).collect()
}

#[test]
fn a_color_token_and_a_themes_members() {
    let values = snapshot()
        .named_values(&DocumentUri::from(URI), "latte.value", "color")
        .expect("named values");
    // The document's own first, then the import's, each in declaration order.
    assert_eq!(
        paths(&values),
        ["accent", "brand", "theme.accent", "theme.muted"],
        "{values:#?}"
    );
    let accent = &values[0];
    assert_eq!(accent.ty, "Color");
    assert_eq!(accent.value.as_deref(), Some("\"#c8a27a\""));
    assert_eq!(accent.doc.as_deref(), Some("The menu's highlight."));
    assert_eq!(values[1].doc.as_deref(), Some("The brand's warm accent."));
    assert_eq!(values[2].value.as_deref(), Some("\"#c8a27a\""));
    // An unwritten member reads its default.
    assert_eq!(values[3].value.as_deref(), Some("\"#888888\""));
}

#[test]
fn a_number_property_takes_numbers() {
    let values = snapshot()
        .named_values(&DocumentUri::from(URI), "latte.value", "size")
        .expect("named values");
    assert_eq!(paths(&values), ["count", "theme.radius"], "{values:#?}");
    assert_eq!(values[0].value.as_deref(), Some("3"));
}

#[test]
fn not_an_element_or_not_a_property() {
    let snapshot = snapshot();
    let uri = DocumentUri::from(URI);
    for (key, property) in [
        ("latte.value.name.value", "color"),
        ("latte.value", "nope"),
        ("latte.value", "note"),
        ("missing", "color"),
    ] {
        assert!(
            snapshot
                .named_values(&uri, key, property)
                .expect("answer")
                .is_empty(),
            "{key} {property}"
        );
    }
}

#[test]
fn named_values_round_trip_through_json() {
    let values = snapshot()
        .named_values(&DocumentUri::from(URI), "latte.value", "color")
        .expect("named values");
    let json = serde_json::to_string(&values).expect("json");
    assert!(json.contains(r##"{"path":"accent","type":"Color","doc":"The menu's highlight.","value":"\"#c8a27a\""}"##), "{json}");
    let back: Vec<NamedValue> = serde_json::from_str(&json).expect("back");
    assert_eq!(back, values);
}
