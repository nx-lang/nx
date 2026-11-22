use super::*;
use std::path::PathBuf;

const CURSOR: &str = "⟨cursor⟩";
const DESIGN_URI: &str = "nx://tenant/design.nx";
const APP_URI: &str = "nx://tenant/app.nx";

const DESIGN: &str = r#"/// The gap between items, in pixels.
export environment space: int = 4
export type Size = { width: int height: int }
export type Palette = { accent: string ink: string }
/// The page size.
export environment size: Size = <Size width=1 height=2 />
export environment color: Palette = <Palette accent="blue" ink="black" />
/// The language text is shown in.
export environment locale: string = "en-US"
"#;

/// The source with the cursor marker removed, and the marker's position.
fn position_for(source: &str) -> (String, TextPosition) {
    let offset = source.find(CURSOR).expect("fixture has a cursor marker");
    let prefix = &source[..offset];
    let line = prefix.matches('\n').count() as u32;
    let line_start = prefix.rfind('\n').map(|index| index + 1).unwrap_or(0);
    let character = prefix[line_start..].encode_utf16().count() as u32;
    (
        format!("{prefix}{}", &source[offset + CURSOR.len()..]),
        TextPosition::new(line, character),
    )
}

fn snapshot(documents: &[(&str, &str)]) -> WorkspaceSnapshot {
    WorkspaceSnapshot::from_documents(
        Option::<PathBuf>::None,
        documents
            .iter()
            .map(|(uri, source)| DocumentInput::new(*uri, *source).with_version(1))
            .collect(),
    )
    .expect("snapshot")
}

/// The hover at the cursor in `app`, beside `design.nx`.
fn hover_in_app(app: &str) -> Option<String> {
    let (app, position) = position_for(app);
    snapshot(&[(DESIGN_URI, DESIGN), (APP_URI, &app)])
        .hover(&DocumentUri::from(APP_URI), position)
        .expect("hover")
        .map(|hover| hover.contents)
}

/// The completions at the cursor in `app`, beside `design.nx`.
fn completions_in_app(app: &str) -> Vec<CompletionItem> {
    let (app, position) = position_for(app);
    snapshot(&[(DESIGN_URI, DESIGN), (APP_URI, &app)])
        .completions(&DocumentUri::from(APP_URI), position)
        .expect("completions")
        .items
}

fn labels(items: &[CompletionItem]) -> Vec<&str> {
    items.iter().map(|item| item.label.as_str()).collect()
}

/// Spec: "Completion after `env.`".
#[test]
fn completion_after_env_in_attributes_offers_the_visible_values_with_their_types() {
    let items = completions_in_app(
        "import \"./design.nx\"\nexternal component <Text value: string />\nlet page() = { <Text env.⟨cursor⟩ value=\"x\" /> }\n",
    );
    assert_eq!(labels(&items), vec!["color", "locale", "size", "space"]);
    let space = items.iter().find(|item| item.label == "space").unwrap();
    assert_eq!(space.detail.as_deref(), Some("int"));
    assert_eq!(
        space.documentation.as_deref(),
        Some("The gap between items, in pixels.")
    );
}

#[test]
fn completion_after_env_in_an_expression_offers_the_visible_values() {
    let items = completions_in_app("import \"./design.nx\"\nlet gap() = { env.⟨cursor⟩ }\n");
    assert_eq!(labels(&items), vec!["color", "locale", "size", "space"]);
}

#[test]
fn completion_after_a_value_offers_the_fields_of_its_record_type() {
    let items = completions_in_app("import \"./design.nx\"\nlet w() = { env.size.⟨cursor⟩ }\n");
    assert_eq!(labels(&items), vec!["width", "height"]);
    assert_eq!(items[0].detail.as_deref(), Some("int"));
}

#[test]
fn general_completions_leave_environment_values_to_env() {
    let items = completions_in_app("import \"./design.nx\"\nlet gap() = { ⟨cursor⟩ }\n");
    let labels = labels(&items);
    assert!(labels.contains(&"env"), "{labels:?}");
    assert!(
        !labels.iter().any(|label| label.starts_with("env.")),
        "{labels:?}"
    );
}

#[test]
fn hover_on_a_read_shows_the_declaration_and_its_documentation() {
    let hover = hover_in_app("import \"./design.nx\"\nlet gap() = { env.sp⟨cursor⟩ace * 2 }\n")
        .expect("hover");
    assert!(hover.contains("environment space: int = 4"), "{hover}");
    assert!(
        hover.contains("The gap between items, in pixels."),
        "{hover}"
    );
}

#[test]
fn hover_on_a_field_of_a_read_is_the_fields() {
    let hover = hover_in_app("import \"./design.nx\"\nlet w() = { env.size.wid⟨cursor⟩th }\n")
        .expect("hover");
    assert!(hover.contains("width: int"), "{hover}");
    assert!(!hover.contains("environment"), "{hover}");
}

#[test]
fn hover_on_an_override_attribute_shows_the_declaration() {
    let hover = hover_in_app(
        "import \"./design.nx\"\nexternal component <Text value: string />\nlet page() = { <Text env.spa⟨cursor⟩ce=3 value=\"x\" /> }\n",
    )
    .expect("hover");
    assert!(hover.contains("environment space: int = 4"), "{hover}");
}

#[test]
fn hover_on_a_record_default_spells_it_as_written() {
    let hover = hover_in_app("import \"./design.nx\"\nlet c() = { env.col⟨cursor⟩or.accent }\n")
        .expect("hover");
    assert!(
        hover.contains("environment color: Palette = <Palette accent=\"blue\" ink=\"black\" />"),
        "{hover}"
    );
}

/// Spec: "Hover lists what a component reads".
#[test]
fn hover_on_a_component_lists_what_it_reads_through_the_functions_it_calls() {
    let hover = hover_in_app(
        "import \"./design.nx\"\nexternal component <Text value: string />\ncomponent <Bad⟨cursor⟩ge /> = { <Text value={label()} /> }\nlet label() = { env.locale + \"\" + env.space }\n",
    )
    .expect("hover");
    assert!(
        hover.contains("Reads `env.locale`, `env.space`."),
        "{hover}"
    );
}

#[test]
fn hover_on_a_function_that_reads_nothing_lists_nothing() {
    let hover =
        hover_in_app("import \"./design.nx\"\nlet tw⟨cursor⟩ice(x: int): int = { x * 2 }\n")
            .expect("hover");
    assert!(!hover.contains("Reads"), "{hover}");
}

#[test]
fn hover_on_a_top_level_provide_lists_what_it_sets() {
    let hover = hover_in_app("import \"./design.nx\"\npro⟨cursor⟩vide env.size.width = 8\n")
        .expect("hover");
    assert_eq!(hover, "Sets `env.size.width`.");
}

/// Spec: "Hover on a bundle provide lists what it sets".
#[test]
fn hover_on_a_bundle_provide_lists_what_the_bundle_sets() {
    let brand = "import \"./design.nx\"\nexport let theme: env.Update = <env.Update space=12 color=<Palette.Update accent=\"red\" /> />\n";
    let (app, position) = position_for(
        "import \"./design.nx\"\nimport \"./brand.nx\" as brand\nprovide brand.the⟨cursor⟩me\n",
    );
    let hover = snapshot(&[
        (DESIGN_URI, DESIGN),
        ("nx://tenant/brand.nx", brand),
        (APP_URI, &app),
    ])
    .hover(&DocumentUri::from(APP_URI), position)
    .expect("hover")
    .expect("a hover")
    .contents;
    assert!(
        hover.contains("Sets `env.space`, `env.color.accent`."),
        "{hover}"
    );
}
