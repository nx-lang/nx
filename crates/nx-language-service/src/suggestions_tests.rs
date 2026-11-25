use super::*;
use std::path::PathBuf;

const URI: &str = "nx://tenant/main.nx";

fn snapshot_of(documents: &[(&str, &str)]) -> WorkspaceSnapshot {
    WorkspaceSnapshot::from_documents(
        Option::<PathBuf>::None,
        documents
            .iter()
            .map(|(uri, source)| DocumentInput::new(*uri, *source).with_version(1))
            .collect(),
    )
    .expect("snapshot")
}

fn suggestions_in(documents: &[(&str, &str)], uri: &str) -> SourceTree {
    snapshot_of(documents)
        .source_tree_with_options(
            &DocumentUri::from(uri),
            &SourceTreeOptions {
                suggestions: true,
                ..SourceTreeOptions::default()
            },
        )
        .expect("source tree")
}

fn tree(source: &str) -> SourceTree {
    suggestions_in(&[(URI, source)], URI)
}

/// The suggestion key of `property` on the declaration named `declaration`.
fn key_of(tree: &SourceTree, declaration: &str, property: &str) -> Option<String> {
    tree.declarations
        .iter()
        .find(|entry| entry.name == declaration)
        .and_then(|entry| entry.properties.iter().find(|p| p.name == property))
        .and_then(|property| property.suggestion_key.clone())
}

/// The names suggested for `key`, in order.
fn names(tree: &SourceTree, key: &str) -> Vec<String> {
    tree.suggestions
        .iter()
        .filter(|suggestion| suggestion.key == key)
        .map(|suggestion| suggestion.name.clone())
        .collect()
}

fn suggestion<'t>(tree: &'t SourceTree, key: &str, name: &str) -> &'t SourceSuggestion {
    tree.suggestions
        .iter()
        .find(|suggestion| suggestion.key == key && suggestion.name == name)
        .unwrap_or_else(|| panic!("{name} under {key}: {:#?}", names(tree, key)))
}

const CARD: &str = r#"import "@nx/design"

component <Card padding: Spacing = 0 radius: Radius = 0 fill: Color = #ffffff /> = {
  <div />
}

let root() = <Card padding=8 />
"#;

const SPACING: &str = "@nx/design/types.nx#Spacing";
const COLOR: &str = "@nx/color/color.nx#Color";

#[test]
fn spacing_and_radius_have_different_keys() {
    let tree = tree(CARD);
    assert_eq!(key_of(&tree, "Card", "padding").as_deref(), Some(SPACING));
    assert_eq!(
        key_of(&tree, "Card", "radius").as_deref(),
        Some("@nx/design/types.nx#Radius")
    );
    let spacing = names(&tree, SPACING);
    assert!(spacing.contains(&"env.space.md".to_string()), "{spacing:?}");
    assert!(
        !spacing.iter().any(|name| name.starts_with("env.radius")),
        "{spacing:?}"
    );
    let entry = suggestion(&tree, SPACING, "env.space.md");
    assert_eq!(entry.ty, "Spacing");
    assert_eq!(entry.group.kind, SourceSuggestionGroupKind::Environment);
    assert_eq!(entry.group.label, "Environment");
}

#[test]
fn an_alias_of_a_record_type_shares_its_key() {
    let tree = tree(
        r#"import "@nx/design"
import "@nx/color" as c

component <Swatch fill: Color stroke: c.Color /> = { <div /> }
let root() = <Swatch fill=#ffffff stroke=#000000 />
"#,
    );
    assert_eq!(key_of(&tree, "Swatch", "fill").as_deref(), Some(COLOR));
    assert_eq!(key_of(&tree, "Swatch", "stroke").as_deref(), Some(COLOR));
}

#[test]
fn a_bare_or_inline_constrained_primitive_has_no_key() {
    let tree = tree(
        r#"component <Bar width: float64 where finite && > 0.0 label: string count: int /> = { <div /> }
let root() = <Bar width=1.0 label="a" count=1 />
"#,
    );
    for property in ["width", "label", "count"] {
        assert_eq!(key_of(&tree, "Bar", property), None, "{property}");
    }
    assert!(tree.suggestions.is_empty());
}

#[test]
fn a_user_type_with_the_same_name_has_its_own_key() {
    let tree = tree(
        r#"type Spacing = int
component <Gap size: Spacing /> = { <div /> }
let root() = <Gap size=1 />
"#,
    );
    let key = key_of(&tree, "Gap", "size").expect("key");
    assert_ne!(key, SPACING);
    assert!(key.ends_with("#Spacing"), "{key}");
}

#[test]
fn a_user_alias_of_a_token_type_matches_it_but_not_the_reverse() {
    let tree = tree(
        r#"import "@nx/design"

type ButtonGap = Spacing
let gap: ButtonGap = 6
let tight: Spacing = 2

component <Row gap: Spacing = 0 buttonGap: ButtonGap = 0 /> = { <div /> }
let root() = <Row gap=1 />
"#,
    );
    let spacing = names(&tree, SPACING);
    assert!(spacing.contains(&"gap".to_string()), "{spacing:?}");
    assert!(spacing.contains(&"tight".to_string()), "{spacing:?}");
    let button_gap = key_of(&tree, "Row", "buttonGap").expect("key");
    let button_gaps = names(&tree, &button_gap);
    assert!(button_gaps.contains(&"gap".to_string()), "{button_gaps:?}");
    assert!(
        !button_gaps.contains(&"tight".to_string()),
        "{button_gaps:?}"
    );
    assert!(
        !button_gaps.iter().any(|name| name.starts_with("env.")),
        "{button_gaps:?}"
    );
}

#[test]
fn an_untyped_value_matches_by_its_inferred_type() {
    let tree = tree(
        r#"import "@nx/design"

let brandRed = #e11d48
let gap = 12

component <Box fill: Color = #ffffff padding: Spacing = 0 /> = { <div /> }
let root() = <Box />
"#,
    );
    let red = suggestion(&tree, COLOR, "brandRed");
    assert_eq!(red.group.kind, SourceSuggestionGroupKind::Document);
    assert_eq!(red.group.label, "This document");
    assert_eq!(
        red.preview
            .as_ref()
            .map(|preview| (preview.kind, preview.value.as_str())),
        Some((SourcePreviewKind::Color, "#e11d48ff"))
    );
    assert!(!names(&tree, SPACING).contains(&"gap".to_string()));
}

const COLORS: &str = r#"import "@nx/design"

component <Label color: Color = #000000 text: TextColors? /> = { <div /> }
let root() = <Label />
"#;

#[test]
fn tokens_and_their_fields_are_suggested() {
    let tree = tree(COLORS);
    let colors = names(&tree, COLOR);
    for expected in [
        "env.color.accent",
        "env.color.text.muted",
        "standardPalette.blue600",
    ] {
        assert!(
            colors.contains(&expected.to_string()),
            "{expected}: {colors:?}"
        );
    }
    assert!(!colors.contains(&"env.color".to_string()));
    assert!(!colors.contains(&"env.color.text".to_string()));
}

#[test]
fn an_intermediate_record_is_suggested_for_its_type() {
    let tree = tree(COLORS);
    let key = key_of(&tree, "Label", "text").expect("key");
    assert!(key.ends_with("#TextColors"), "{key}");
    assert_eq!(names(&tree, &key), vec!["env.color.text".to_string()]);
}

#[test]
fn an_import_alias_spells_the_name() {
    let tree = tree(
        r#"import "@nx/design" as d

component <Dot fill: d.Color = #000000 /> = { <div /> }
let root() = <Dot />
"#,
    );
    let colors = names(&tree, COLOR);
    assert!(
        colors.contains(&"d.standardPalette.blue600".to_string()),
        "{colors:?}"
    );
    assert!(
        colors.contains(&"env.color.accent".to_string()),
        "{colors:?}"
    );
    assert!(
        !colors.contains(&"standardPalette.blue600".to_string()),
        "{colors:?}"
    );
}

#[test]
fn another_documents_value_needs_an_import() {
    let brand = ("nx://tenant/brand.nx", "export let brandRed = #e11d48\n");
    let unimported = suggestions_in(
        &[
            brand,
            (
                URI,
                "import \"@nx/design\"\n\ncomponent <Dot fill: Color = #000000 /> = { <div /> }\nlet root() = <Dot />\n",
            ),
        ],
        URI,
    );
    assert!(!names(&unimported, COLOR).contains(&"brandRed".to_string()));
    let imported = suggestions_in(
        &[
            brand,
            (
                URI,
                "import \"@nx/design\"\nimport \"./brand.nx\"\n\ncomponent <Dot fill: Color = #000000 /> = { <div /> }\nlet root() = <Dot />\n",
            ),
        ],
        URI,
    );
    let red = suggestion(&imported, COLOR, "brandRed");
    assert_eq!(red.group.kind, SourceSuggestionGroupKind::Import);
    assert_eq!(red.group.label, "./brand.nx");
    let colors = names(&imported, COLOR);
    let at = |name: &str| colors.iter().position(|entry| entry == name).expect(name);
    assert!(
        at("standardPalette.blue600") < at("brandRed"),
        "imports in import order"
    );
}

#[test]
fn a_peer_label_is_the_path_from_the_document() {
    assert_eq!(
        super::source_tree::relative_path_for_tests("lib/card.nx", "lib/brand.nx"),
        "./brand.nx"
    );
    assert_eq!(
        super::source_tree::relative_path_for_tests("lib/ui/card.nx", "lib/brand.nx"),
        "../brand.nx"
    );
}

#[test]
fn a_selective_import_brings_only_its_names() {
    let tree = tree(
        r#"import { standardPalette as d.standardPalette, Color } from "@nx/design"

component <Dot fill: Color = #000000 /> = { <div /> }
let root() = <Dot />
"#,
    );
    let imported = tree
        .suggestions
        .iter()
        .filter(|entry| entry.group.kind == SourceSuggestionGroupKind::Import)
        .map(|entry| entry.name.as_str())
        .collect::<Vec<_>>();
    assert!(
        imported.contains(&"d.standardPalette.blue600"),
        "{imported:?}"
    );
    assert!(
        imported
            .iter()
            .all(|name| name.starts_with("d.standardPalette.")),
        "{imported:?}"
    );
}

#[test]
fn an_ambiguous_name_is_left_out() {
    let tree = suggestions_in(
        &[
            (
                "nx://tenant/one/a.nx",
                "import \"@nx/color\" as c\nexport let accent: c.Color = #ff0000\n",
            ),
            (
                "nx://tenant/two/b.nx",
                "import \"@nx/color\" as c\nexport let accent: c.Color = #00ff00\n",
            ),
            (
                URI,
                "import \"@nx/color\" as c\nimport \"./one/a.nx\"\nimport \"./two/b.nx\"\n\ncomponent <Dot fill: c.Color = #000000 /> = { <div /> }\nlet root() = <Dot />\n",
            ),
        ],
        URI,
    );
    assert!(
        !names(&tree, COLOR).contains(&"accent".to_string()),
        "{:?}",
        names(&tree, COLOR)
    );
    // With one of the two imports, the name is not ambiguous.
    let single = suggestions_in(
        &[
            (
                "nx://tenant/one/a.nx",
                "import \"@nx/color\" as c\nexport let accent: c.Color = #ff0000\n",
            ),
            (
                URI,
                "import \"@nx/color\" as c\nimport \"./one/a.nx\"\n\ncomponent <Dot fill: c.Color = #000000 /> = { <div /> }\nlet root() = <Dot />\n",
            ),
        ],
        URI,
    );
    assert!(names(&single, COLOR).contains(&"accent".to_string()));
}

#[test]
fn a_parameter_shadows_a_top_level_value_inside_its_declaration() {
    let tree = tree(
        r#"import "@nx/color" as c

let tint = #e11d48
component <Dot fill: c.Color = #000000 /> = { <div /> }
component <Badge tint: c.Color = #000000 /> = { <Dot fill={tint} /> }
let root() = <Badge />
"#,
    );
    let badge = tree
        .declarations
        .iter()
        .position(|entry| entry.name == "Badge")
        .expect("Badge") as u32;
    let entries = tree
        .suggestions
        .iter()
        .filter(|entry| entry.name == "tint" && entry.key == COLOR)
        .collect::<Vec<_>>();
    let scoped = entries
        .iter()
        .find(|entry| entry.scope.is_some())
        .expect("scoped");
    assert_eq!(scoped.scope, Some(badge));
    assert_eq!(scoped.group.kind, SourceSuggestionGroupKind::Declaration);
    assert_eq!(scoped.group.label, "Badge");
    let unscoped = entries
        .iter()
        .find(|entry| entry.scope.is_none())
        .expect("unscoped");
    assert!(unscoped.self_.contains(&badge), "{:?}", unscoped.self_);
}

#[test]
fn a_value_is_not_suggested_inside_itself() {
    let tree = tree(
        r#"import "@nx/design"

let brand: SemanticColors = <SemanticColors
  background=#ffffff surface=#ffffff surfaceMuted=#eeeeee
  text=<TextColors normal=#111111 muted=#666666 disabled=#999999 />
  border=#dddddd focus=#3366ff accent=#e11d48 onAccent=#ffffff
  danger=#cc0000 onDanger=#ffffff warning=#ffaa00 onWarning=#000000
  success=#00aa00 onSuccess=#ffffff
/>
component <Dot fill: Color = #000000 /> = { <div /> }
let root() = <Dot />
"#,
    );
    let brand = tree
        .declarations
        .iter()
        .position(|entry| entry.name == "brand")
        .expect("brand") as u32;
    let accent = suggestion(&tree, COLOR, "brand.accent");
    assert!(accent.self_.contains(&brand), "{:?}", accent.self_);
    assert_eq!(
        accent
            .preview
            .as_ref()
            .map(|preview| preview.value.as_str()),
        Some("#e11d48ff")
    );
    // A token that reads nothing of this document is offered everywhere.
    assert!(suggestion(&tree, COLOR, "env.color.accent")
        .self_
        .is_empty());
}

/// `@nx/design`'s own modules as documents of one snapshot.
fn design_documents() -> Vec<(String, String)> {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../nx-program/src/std/design");
    ["tokens.nx", "types.nx", "palette.nx"]
        .into_iter()
        .map(|file| {
            (
                format!("nx://design/{file}"),
                std::fs::read_to_string(root.join(file)).expect("design source"),
            )
        })
        .collect()
}

#[test]
fn a_tokens_default_does_not_read_itself() {
    let documents = design_documents();
    let documents = documents
        .iter()
        .map(|(uri, source)| (uri.as_str(), source.as_str()))
        .collect::<Vec<_>>();
    let tree = suggestions_in(&documents, "nx://design/tokens.nx");
    let index = |name: &str| {
        tree.declarations
            .iter()
            .position(|entry| entry.name == name && entry.module.ends_with("tokens.nx"))
            .unwrap_or_else(|| panic!("{name}")) as u32
    };
    let (color, font) = (index("env.color"), index("env.font"));
    let colors = tree
        .suggestions
        .iter()
        .filter(|entry| entry.name.starts_with("env.color."))
        .collect::<Vec<_>>();
    assert!(!colors.is_empty());
    assert!(colors.iter().all(|entry| entry.self_.contains(&color)));
    let text = tree
        .suggestions
        .iter()
        .filter(|entry| entry.name.starts_with("env.text."))
        .collect::<Vec<_>>();
    assert!(!text.is_empty());
    assert!(text.iter().all(|entry| entry.self_.contains(&font)));
}

#[test]
fn scales_read_small_to_large_and_the_environment_comes_first() {
    let tree = tree(
        r#"import "@nx/design"

let brandRed = #e11d48
component <Box padding: Spacing = 0 fill: Color = #ffffff /> = { <div /> }
let root() = <Box />
"#,
    );
    let spaces = names(&tree, SPACING)
        .into_iter()
        .filter(|name| name.starts_with("env.space."))
        .collect::<Vec<_>>();
    assert_eq!(
        spaces,
        ["none", "xxs", "xs", "sm", "md", "lg", "xl", "xxl", "xxxl"]
            .map(|step| format!("env.space.{step}"))
    );
    let colors = names(&tree, COLOR);
    let at = |name: &str| colors.iter().position(|entry| entry == name).expect(name);
    let last_environment = colors
        .iter()
        .rposition(|name| name.starts_with("env."))
        .expect("environment");
    assert!(last_environment < at("brandRed"));
    assert!(at("brandRed") < at("standardPalette.blue600"));
}

#[test]
fn previews_follow_names_defaults_and_the_default_color_mode() {
    let tree = tree(
        r#"import "@nx/design"

let gap: Spacing = 4
let wide: Spacing = { gap * 2 }
component <Box padding: Spacing = 0 fill: Color = #ffffff /> = { <div /> }
let root() = <Box />
"#,
    );
    let preview = |name: &str, key: &str| {
        suggestion(&tree, key, name)
            .preview
            .as_ref()
            .map(|preview| (preview.kind, preview.value.clone()))
    };
    assert_eq!(
        preview("env.color.accent", COLOR),
        preview("standardPalette.blue600", COLOR)
    );
    assert_eq!(
        preview("env.color.accent", COLOR).map(|p| p.0),
        Some(SourcePreviewKind::Color)
    );
    assert_eq!(
        preview("env.space.md", SPACING),
        Some((SourcePreviewKind::Number, "12".to_string()))
    );
    assert_eq!(preview("wide", SPACING), None);
    // A color inside sRGB carries the hex that writes it; one outside carries none.
    let hex = |name: &str| {
        suggestion(&tree, COLOR, name)
            .preview
            .as_ref()
            .and_then(|preview| preview.hex.clone())
    };
    assert_eq!(hex("standardPalette.blue600").as_deref(), Some("#155dfc"));
    assert_eq!(hex("standardPalette.green500"), None);
    assert!(suggestion(&tree, COLOR, "standardPalette.green500")
        .preview
        .as_ref()
        .is_some_and(|preview| preview.outside_srgb));
    assert_eq!(
        preview("gap", SPACING),
        Some((SourcePreviewKind::Number, "4".to_string()))
    );
}

#[test]
fn string_boolean_and_case_previews() {
    let tree = tree(
        r#"type Label = string
type Flag = boolean
type Mode = light | dark
let title: Label = "Hello"
let on: Flag = true
let mode: Mode = dark
component <Box title: Label = "" on: Flag = false mode: Mode = light /> = { <div /> }
let root() = <Box />
"#,
    );
    let preview = |name: &str| {
        tree.suggestions
            .iter()
            .find(|entry| entry.name == name && entry.scope.is_none())
            .and_then(|entry| entry.preview.clone())
            .map(|preview| (preview.kind, preview.value))
    };
    assert_eq!(
        preview("title"),
        Some((SourcePreviewKind::String, "Hello".to_string()))
    );
    assert_eq!(
        preview("on"),
        Some((SourcePreviewKind::Boolean, "true".to_string()))
    );
    assert_eq!(
        preview("mode"),
        Some((SourcePreviewKind::Case, "dark".to_string()))
    );
}

#[test]
fn this_documents_lets_carry_keys() {
    let tree = tree("import \"@nx/design\"\n\nlet gap: Spacing = 4\nlet red = #ff0000\n");
    let key = |name: &str| {
        tree.declarations
            .iter()
            .find(|entry| entry.name == name)
            .and_then(|entry| entry.suggestion_key.clone())
    };
    assert_eq!(key("gap").as_deref(), Some(SPACING));
    assert_eq!(key("red").as_deref(), Some(COLOR));
    assert!(names(&tree, SPACING).contains(&"env.space.md".to_string()));
}

#[test]
fn without_the_option_nothing_changes() {
    let snapshot = snapshot_of(&[(URI, CARD)]);
    let plain = snapshot.source_tree(&DocumentUri::from(URI)).expect("tree");
    assert!(plain.suggestions.is_empty());
    assert!(plain
        .declarations
        .iter()
        .all(|entry| entry.suggestion_key.is_none()
            && entry.properties.iter().all(|p| p.suggestion_key.is_none())));
    let json = serde_json::to_string(&plain).expect("json");
    assert!(!json.contains("suggestion"), "{json}");
    let round: SourceTree =
        serde_json::from_str(&serde_json::to_string(&tree(CARD)).expect("json"))
            .expect("round trip");
    assert_eq!(round, tree(CARD));
}

#[test]
fn graphics_properties_use_the_design_aliases() {
    let tree = tree(
        r#"import "@nx/graphics" as g
import "@nx/design"

let root() = <g.Group>
  <g.Rectangle width=10 height=10 rx=2 stroke=<g.Stroke paint=none width=1 /> />
  <g.Text text="Hi" fontSize=12 />
</g.Group>
"#,
    );
    let border = "@nx/design/types.nx#BorderWidth";
    assert_eq!(key_of(&tree, "Stroke", "width").as_deref(), Some(border));
    assert_eq!(
        key_of(&tree, "Rectangle", "rx").as_deref(),
        Some("@nx/design/types.nx#Radius")
    );
    assert_eq!(
        key_of(&tree, "Text", "fontSize").as_deref(),
        Some("@nx/design/types.nx#FontSize")
    );
    let widths = names(&tree, border);
    assert!(
        widths.contains(&"env.borderWidth.thin".to_string()),
        "{widths:?}"
    );
}
