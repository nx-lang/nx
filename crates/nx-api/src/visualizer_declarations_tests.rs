//! The visualizer library `@nx/visualizer` and the analysis of `visualizer` declarations written
//! with it.

use crate::{
    build_workspace_program_artifact, eval_program_artifact, standard_library, validate_workspace,
    EvalResult, LibraryRegistry, NxDiagnostic, NxLibraryModule, NxLibrarySource, NxSeverity,
    NxWorkspace, NxWorkspaceModule, ProgramArtifact, ProgramBuildContext,
};
use nx_hir::{InterfaceItemKind, Visibility};
use nx_value::NxValue;

const VISUALIZER_ROOT: &str = "@nx/visualizer";

const VISUALIZER_TYPES: [&str; 6] = [
    "ElementVisualizer",
    "IdPosition",
    "PropertyVisualizer",
    "ValueEditor",
    "ValueFormat",
    "ValueVisualizer",
];

fn workspace(files: &[(&str, &str)]) -> NxWorkspace {
    NxWorkspace::new(
        files
            .iter()
            .map(|(identity, source)| {
                NxWorkspaceModule::from_source(*identity, *source).expect("workspace module")
            })
            .collect(),
    )
    .expect("workspace")
}

/// Every diagnostic of the given modules, of any severity.
fn diagnostics(files: &[(&str, &str)]) -> Vec<NxDiagnostic> {
    validate_workspace(&workspace(files), &ProgramBuildContext::empty())
}

/// Every diagnostic of one module that begins with `import "@nx/visualizer"`.
fn visualizer_diagnostics(body: &str) -> Vec<NxDiagnostic> {
    diagnostics(&[("main.nx", &format!("import \"@nx/visualizer\"\n{body}"))])
}

fn assert_clean(body: &str) {
    let diagnostics = visualizer_diagnostics(body);
    assert!(
        diagnostics.is_empty(),
        "expected no diagnostics for {body:?}: {diagnostics:#?}"
    );
}

fn assert_error(body: &str, code: Option<&str>, fragments: &[&str]) {
    let diagnostics = visualizer_diagnostics(body);
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.severity == NxSeverity::Error
                && code.is_none_or(|code| diagnostic.code.as_deref() == Some(code))
                && fragments
                    .iter()
                    .all(|fragment| diagnostic.message.contains(fragment))
        }),
        "expected an error {code:?} containing {fragments:?} for {body:?}: {diagnostics:#?}"
    );
}

// --- The library ------------------------------------------------------------------------------

#[test]
fn the_visualizer_library_exports_exactly_its_six_types() {
    let library = standard_library(VISUALIZER_ROOT).expect("the visualizer library");
    assert!(library.diagnostics.is_empty(), "{:#?}", library.diagnostics);
    let mut exported = library
        .interface_items
        .iter()
        .filter(|item| item.visibility == Visibility::Export)
        .map(|item| item.item_name.as_str())
        .filter(|name| !name.contains('.'))
        .collect::<Vec<_>>();
    exported.sort();
    assert_eq!(exported, VISUALIZER_TYPES);
    assert!(
        library.interface_items.iter().all(|item| !matches!(
            item.item,
            InterfaceItemKind::Function { .. } | InterfaceItemKind::Component { .. }
        )),
        "the library holds types only"
    );
}

#[test]
fn every_visualizer_type_field_and_case_is_documented() {
    let library = standard_library(VISUALIZER_ROOT).expect("the visualizer library");
    let mut undocumented = Vec::new();
    for item in &library.interface_items {
        if item.item_name.contains('.') {
            continue;
        }
        if item.doc.is_none() {
            undocumented.push(item.item_name.clone());
        }
        match &item.item {
            InterfaceItemKind::Record { properties, .. } => {
                for field in properties {
                    if field.doc.is_none() {
                        undocumented.push(format!("{}.{}", item.item_name, field.name));
                    }
                }
            }
            InterfaceItemKind::Union { cases, .. } => {
                for case in cases {
                    if case.doc.is_none() {
                        undocumented.push(format!("{}.{}", item.item_name, case.name));
                    }
                }
            }
            other => panic!("unexpected declaration in the visualizer library: {other:?}"),
        }
    }
    assert!(undocumented.is_empty(), "undocumented: {undocumented:?}");
}

#[test]
fn an_element_visualizer_with_property_visualizers() {
    assert_clean(
        "type Choice = { value:string label:string description?:string }\n\
         visualizer Choice = <ElementVisualizer title=label id=value idPosition=end icon=\"circle-dot\"><PropertyVisualizer property=description format=id /></ElementVisualizer>",
    );
}

#[test]
fn a_property_visualizer_names_a_property_of_its_target() {
    assert_error(
        "type Choice = { label:string }\n\
         visualizer Choice = <ElementVisualizer><PropertyVisualizer property=value /></ElementVisualizer>",
        None,
        &["value"],
    );
}

#[test]
fn a_color_alias() {
    assert_clean(
        "type Color = string\n\
         visualizer Color = <ValueVisualizer format=color />",
    );
}

#[test]
fn a_property_with_editor_hints() {
    assert_clean(
        "type Card = { title:string padding:int = 8 body?:string }\n\
         visualizer Card = <ElementVisualizer><PropertyVisualizer property=padding editor=slider min=0 max=48 stops={ 0 4 8 16 24 48 } unit=\"px\" /><PropertyVisualizer property=body multiline=true placeholder=\"What it says\" /></ElementVisualizer>",
    );
}

#[test]
fn a_spacing_alias_with_no_format() {
    assert_clean(
        "type Spacing = int\n\
         visualizer Spacing = <ValueVisualizer editor=slider min=0 max=64 stops={ 0 4 8 16 24 32 48 64 } unit=\"px\" />",
    );
}

#[test]
fn hints_take_signed_and_real_numbers() {
    assert_clean(
        "type Offset = float64\n\
         visualizer Offset = <ValueVisualizer min=-10 max=10.5 step=0.5 stops={2} />",
    );
}

#[test]
fn a_segmented_case() {
    assert_clean(
        "type Size = small | medium | large\n\
         type Drink = { size:Size = medium }\n\
         visualizer Drink = <ElementVisualizer><PropertyVisualizer property=size editor=segmented /></ElementVisualizer>",
    );
}

fn warnings_of(body: &str, code: &str) -> Vec<NxDiagnostic> {
    visualizer_diagnostics(body)
        .into_iter()
        .filter(|diagnostic| diagnostic.code.as_deref() == Some(code))
        .collect()
}

#[test]
fn a_slider_on_a_string_is_a_warning() {
    let body = "type Card = { title:string }\n\
                visualizer Card = <ElementVisualizer><PropertyVisualizer property=title editor=slider placeholder=\"Title\" /></ElementVisualizer>";
    let warnings = warnings_of(body, "editor-hint-mismatch");
    assert_eq!(warnings.len(), 1, "{warnings:#?}");
    assert_eq!(warnings[0].severity, NxSeverity::Warning);
    assert!(warnings[0].message.contains("'editor'"), "{warnings:#?}");
    assert!(
        visualizer_diagnostics(body)
            .iter()
            .all(|diagnostic| diagnostic.severity != NxSeverity::Error),
        "a hint is never an error"
    );
}

#[test]
fn each_hint_is_checked_against_its_type() {
    for (body, count) in [
        // Number hints on a string alias.
        ("type Name = string\nvisualizer Name = <ValueVisualizer min=0 unit=\"px\" />", 2),
        // A dropdown on a number.
        ("type Card = { size:int }\nvisualizer Card = <ElementVisualizer><PropertyVisualizer property=size editor=dropdown /></ElementVisualizer>", 1),
        // Multi-line on a number.
        ("type Card = { size:int }\nvisualizer Card = <ElementVisualizer><PropertyVisualizer property=size multiline=true placeholder=\"Size\" /></ElementVisualizer>", 2),
        // A segmented control on a union with a payload case.
        ("type Extra = | none | syrup { flavor:string }\ntype Cup = { extra:Extra = none }\nvisualizer Cup = <ElementVisualizer><PropertyVisualizer property=extra editor=segmented /></ElementVisualizer>", 1),
        // Every hint that fits.
        ("type Card = { size?:float64 label:string }\nvisualizer Card = <ElementVisualizer><PropertyVisualizer property=size editor=field min=0 max=1 step=0.1 unit=\"em\" /><PropertyVisualizer property=label multiline=true /></ElementVisualizer>", 0),
    ] {
        let warnings = warnings_of(body, "editor-hint-mismatch");
        assert_eq!(warnings.len(), count, "{body}: {warnings:#?}");
    }
}

#[test]
fn an_empty_range_is_a_warning() {
    let warnings = warnings_of(
        "type Spacing = int\nvisualizer Spacing = <ValueVisualizer min=10 max=0 />",
        "editor-hint-range",
    );
    assert_eq!(warnings.len(), 1, "{warnings:#?}");
}

#[test]
fn a_hint_is_not_a_constraint() {
    assert_clean(
        "type Card = { padding:int = 8 }\n\
         visualizer Card = <ElementVisualizer><PropertyVisualizer property=padding max=48 /></ElementVisualizer>\n\
         let card = <Card padding=64 />",
    );
}

// --- Visualizer declarations
// ----------------------------------------------------------------------

#[test]
fn a_visualizer_beside_its_type() {
    assert_clean(
        "type Choice = { value:string label:string }\n\
         visualizer Choice = <ElementVisualizer title=label id=value />",
    );
}

#[test]
fn visualizer_stays_an_ordinary_name_elsewhere() {
    let diagnostics = diagnostics(&[(
        "main.nx",
        "type Settings = { visualizer:string }\n\
         let visualizer = \"Ada\"\n\
         component <Panel visualizer:string /> = { <Label text={visualizer} /> }",
    )]);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != NxSeverity::Error),
        "{diagnostics:#?}"
    );
}

#[test]
fn a_visualizer_declaration_takes_no_visibility() {
    assert_error(
        "type Choice = { label:string }\n\
         export visualizer Choice = <ElementVisualizer title=label />",
        None,
        &["takes no visibility modifier"],
    );
}

#[test]
fn each_kind_takes_its_record() {
    assert_clean(
        "type Choice = { label:string }\n\
         type ImageUrl = string\n\
         component <Card title:string /> = { <Panel /> }\n\
         abstract component <Question label:string />\n\
         external component <SearchBox placeholder?:string />\n\
         action Saved = { id:string }\n\
         visualizer Choice = <ElementVisualizer title=label />\n\
         visualizer ImageUrl = <ValueVisualizer format=id />\n\
         visualizer Card = <ElementVisualizer title=title />\n\
         visualizer Question = <ElementVisualizer title=label />\n\
         visualizer SearchBox = <ElementVisualizer title=placeholder />\n\
         visualizer Saved = <ElementVisualizer title=id />",
    );
}

#[test]
fn the_wrong_record_for_the_kind() {
    assert_error(
        "type ImageUrl = string\n\
         visualizer ImageUrl = <ElementVisualizer />",
        Some("visualizer-record-mismatch"),
        &["ValueVisualizer", "ElementVisualizer"],
    );
    assert_error(
        "type Choice = { label:string }\n\
         visualizer Choice = \"Choice\"",
        Some("visualizer-record-mismatch"),
        &["ElementVisualizer"],
    );
}

#[test]
fn a_visualizer_without_the_library_imported_says_to_import_it() {
    // No `import "@nx/visualizer"`, so `ElementVisualizer` is an unknown element of the same name.
    let diagnostics = diagnostics(&[(
        "main.nx",
        "type Choice = { label:string }\nvisualizer Choice = <ElementVisualizer title=label />",
    )]);
    let mismatch = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code.as_deref() == Some("visualizer-record-mismatch"))
        .unwrap_or_else(|| panic!("no visualizer-record-mismatch in {diagnostics:#?}"));
    assert_eq!(
        mismatch.message,
        "The visualizer of record 'Choice' must be an 'ElementVisualizer' from \"@nx/visualizer\", and this \
         'ElementVisualizer' is not that one; add `import \"@nx/visualizer\"`"
    );
}

#[test]
fn the_wrong_visualizer_record_is_one_error() {
    // An alias's visualizer written as an element visualizer, property visualizers and all: one
    // mismatch, not an error for each property the element visualizer could name.
    for value in [
        "<ElementVisualizer title=small />",
        "<ElementVisualizer><PropertyVisualizer property=small /></ElementVisualizer>",
    ] {
        let source =
            format!("import \"@nx/visualizer\"\ntype Size = string\nvisualizer Size = {value}");
        let diagnostics = diagnostics(&[("main.nx", source.as_str())]);
        let errors = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == crate::NxSeverity::Error)
            .map(|diagnostic| diagnostic.code.as_deref().unwrap_or(""))
            .collect::<Vec<_>>();
        assert_eq!(
            errors,
            ["visualizer-record-mismatch"],
            "{value}: {diagnostics:#?}"
        );
    }
}

#[test]
fn a_target_that_cannot_have_a_visualizer() {
    assert_error(
        "let answer = 42\n\
         visualizer answer = <ValueVisualizer format=id />",
        Some("visualizer-target-not-allowed"),
        &["'answer' cannot have a visualizer"],
    );
    assert_error(
        "type Range = { T:type start:T end:T }\n\
         visualizer Range = <ElementVisualizer />",
        Some("visualizer-target-not-allowed"),
        &["'Range' cannot have a visualizer"],
    );
    assert_error(
        "let greet(name:string) = { name }\n\
         visualizer greet = <ElementVisualizer />",
        Some("visualizer-target-not-allowed"),
        &["'greet' cannot have a visualizer"],
    );
    assert_error(
        "type Size = small | large\n\
         visualizer Size = <ElementVisualizer />",
        Some("visualizer-target-not-allowed"),
        &["'Size' cannot have a visualizer", "union"],
    );
    assert_error(
        "type Choice = { label:string }\n\
         type Choices = Choice+\n\
         visualizer Choices = <ValueVisualizer format=id />",
        Some("visualizer-target-not-allowed"),
        &["'Choices' cannot have a visualizer"],
    );
}

#[test]
fn a_derived_declaration_is_not_a_target() {
    for (target, reason) in [
        ("User.Update", "it is a derived update record"),
        ("User.Property", "it is a derived property union"),
        ("Card.Props", "it is a derived props record"),
        ("Card.Props.Update", "it is a derived update record"),
        ("Card.Props.Property", "it is a derived property union"),
        ("Card.State", "it is a derived state record"),
    ] {
        assert_error(
            &format!(
                "type User = {{ name:string }}\n\
                 component <Card title:string /> = {{ state {{ open:boolean = false }} <Panel /> }}\n\
                 visualizer {target} = <ElementVisualizer />"
            ),
            Some("visualizer-target-not-allowed"),
            &[&format!("'{target}' cannot have a visualizer: {reason}")],
        );
    }
}

#[test]
fn an_unknown_target_is_an_unknown_name() {
    assert_error(
        "visualizer Nothing = <ElementVisualizer />",
        Some("unknown-visualizer-target"),
        &["Nothing"],
    );
}

#[test]
fn an_emitted_action_is_a_target() {
    assert_clean(
        "component <Form emits { Submitted { email:string } } /> = { <Panel /> }\n\
         visualizer Form.Submitted = <ElementVisualizer title=email />",
    );
}

#[test]
fn children_take_the_target_too() {
    assert_clean(
        "type Choice = { value:string label:string }\n\
         visualizer Choice = <ElementVisualizer title=label><PropertyVisualizer property=value format=id /></ElementVisualizer>",
    );
}

#[test]
fn a_components_visualizer_names_its_props() {
    assert_clean(
        "component <Question label:string help?:string /> = { <Panel /> }\n\
         visualizer Question = <ElementVisualizer title=label id=help />",
    );
    // A component's state is not among its props.
    assert_error(
        "component <Counter step:int /> = { state { count:int = 0 } <Panel /> }\n\
         visualizer Counter = <ElementVisualizer title=count />",
        None,
        &["count"],
    );
}

#[test]
fn a_misspelled_property_is_a_type_error() {
    assert_error(
        "type Choice = { label:string }\n\
         visualizer Choice = <ElementVisualizer title=lable />",
        None,
        &["lable"],
    );
}

#[test]
fn a_wrong_explicit_target() {
    assert_error(
        "type Choice = { label:string }\n\
         type Other = { name:string }\n\
         visualizer Choice = <ElementVisualizer Target=Other title=name />",
        Some("visualizer-target-mismatch"),
        &["Target=Other", "Choice"],
    );
    // Binding the target itself is allowed.
    assert_clean(
        "type Choice = { label:string }\n\
         visualizer Choice = <ElementVisualizer Target=Choice title=label />",
    );
}

#[test]
fn outside_a_visualizer_declaration_the_rule_is_unchanged() {
    assert_error("let h = <ElementVisualizer />", None, &["Target"]);
}

#[test]
fn a_computed_label_is_rejected() {
    for value in [
        "<ElementVisualizer icon={prefix} />",
        "<ElementVisualizer>Pick {prefix}</ElementVisualizer>",
        "<ElementVisualizer>{if true { <PropertyVisualizer property=label /> }}</ElementVisualizer>",
        "<ElementVisualizer>{for x in { 1 2 } { <PropertyVisualizer property=label /> }}</ElementVisualizer>",
        "{ <ElementVisualizer /> }",
    ] {
        assert_error(
            &format!(
                "type Choice = {{ label:string }}\n\
                 let prefix = \"Pick\"\n\
                 visualizer Choice = {value}"
            ),
            None,
            &["A visualizer's values must be written out"],
        );
    }
    // A bare name resolves only against a union's cases, never to a value.
    assert_error(
        "type Choice = { label:string }\n\
         let prefix = \"Pick\"\n\
         visualizer Choice = <ElementVisualizer idPosition=prefix />",
        None,
        &["prefix"],
    );
}

#[test]
fn literals_and_bare_names_are_accepted() {
    assert_clean(
        "component <Question label:string required:boolean = true /> = { <Panel /> }\n\
         visualizer Question = <ElementVisualizer icon=\"list-checks\" title=label idPosition=end><PropertyVisualizer property=label format=id /></ElementVisualizer>",
    );
}

#[test]
fn twice_in_one_module() {
    let diagnostics = visualizer_diagnostics(
        "type Choice = { label:string }\n\
         visualizer Choice = <ElementVisualizer title=label />\n\
         visualizer Choice = <ElementVisualizer icon=\"option\" />",
    );
    let duplicates = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code.as_deref() == Some("duplicate-visualizer"))
        .collect::<Vec<_>>();
    assert_eq!(duplicates.len(), 1, "{diagnostics:#?}");
    assert!(duplicates[0].message.contains("Choice"));
    assert_eq!(duplicates[0].labels.len(), 2, "{:#?}", duplicates[0]);
}

#[test]
fn a_visualizer_of_an_imported_declaration() {
    let diagnostics = diagnostics(&[
        (
            "main.nx",
            "import \"@nx/visualizer\"\nimport { Choice, Card } from \"./lib.nx\"\n\
             visualizer Choice = <ElementVisualizer title=label />\n\
             visualizer Card = <ElementVisualizer title=title />",
        ),
        (
            "lib.nx",
            "export type Choice = { label:string }\n\
             export component <Card title:string /> = { <Panel /> }",
        ),
    ]);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

// --- Libraries --------------------------------------------------------------------------------

const UI_ROOT: &str = "libraries/ui";

/// A library with components, one of them abstract, and its own visualizer declarations.
fn ui_registry() -> LibraryRegistry {
    let registry = LibraryRegistry::new();
    registry
        .load_library_from_sources(&NxLibrarySource::new(
            UI_ROOT,
            vec![NxLibraryModule::new(
                "ui.nx",
                "import \"@nx/visualizer\"
export abstract component <Question label:string help?:string />
export component <Card extends Question title:string /> = { <Panel /> }
visualizer Question = <ElementVisualizer title=label icon=\"circle-help\" />
",
            )],
        ))
        .unwrap_or_else(|diagnostics| panic!("ui library: {diagnostics:?}"));
    registry
}

fn library_artifact(source: &str) -> ProgramArtifact {
    build_workspace_program_artifact(
        &workspace(&[("main.nx", source)]),
        "main.nx",
        &ui_registry().build_context(),
    )
    .unwrap_or_else(|diagnostics| panic!("workspace artifact: {diagnostics:?}"))
}

#[test]
fn a_props_record_of_an_imported_component_is_available_through_the_import() {
    let artifact = library_artifact(&format!(
        "import {{ Card }} from \"./{UI_ROOT}\"
let root(): Card.Props = {{ <Card.Props label=\"Q\" title=\"Hi\" /> }}
let key(): Card.Props.Property = {{ Card.Props.Property.label }}"
    ));
    assert!(
        artifact.diagnostics.is_empty(),
        "{:#?}",
        artifact.diagnostics
    );
    match eval_program_artifact(&artifact) {
        EvalResult::Ok(NxValue::Record {
            type_name,
            properties,
        }) => {
            assert_eq!(type_name.as_deref(), Some("Card.Props"));
            assert_eq!(
                properties.get("label"),
                Some(&NxValue::String("Q".to_string()))
            );
        }
        EvalResult::Ok(value) => panic!("expected a Card.Props record, got {value:?}"),
        EvalResult::Err(diagnostics) => panic!("evaluation failed: {diagnostics:?}"),
    }
}

#[test]
fn a_visualizer_of_an_imported_component_and_a_library_with_visualizers() {
    // The library's own visualizer declarations analyze cleanly, and a document may declare a
    // visualizer for a declaration it imports from the library.
    let artifact = library_artifact(&format!(
        "import \"@nx/visualizer\"
import {{ Card }} from \"./{UI_ROOT}\"
visualizer Card = <ElementVisualizer icon=\"card\" title=title id=label />
let root(): Card.Props = {{ <Card.Props label=\"Q\" title=\"Hi\" /> }}"
    ));
    assert!(
        artifact.diagnostics.is_empty(),
        "{:#?}",
        artifact.diagnostics
    );
}

// --- Visualizer declarations across a program ----------------------------------------------------

const CHOICE_LIBRARY: (&str, &str) = ("lib.nx", "export type Choice = { label:string }");

fn icon_override(icon: &str) -> String {
    format!(
        "import \"@nx/visualizer\"\nimport {{ Choice }} from \"./lib.nx\"\nvisualizer Choice = <ElementVisualizer icon=\"{icon}\" />\nexport let marker = 1"
    )
}

fn duplicates_in(diagnostics: &[NxDiagnostic], file: &str) -> Vec<NxDiagnostic> {
    diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.code.as_deref() == Some("duplicate-visualizer")
                && diagnostic
                    .labels
                    .first()
                    .is_some_and(|label| label.file == file)
        })
        .cloned()
        .collect()
}

#[test]
fn two_program_modules_declare_a_visualizer_for_the_same_target() {
    let a = icon_override("A");
    let b = icon_override("B");
    let diagnostics = diagnostics(&[
        (
            "main.nx",
            "import \"./a.nx\"\nimport \"./b.nx\"\nlet root() = { 1 }",
        ),
        ("a.nx", &a),
        ("b.nx", &b),
        CHOICE_LIBRARY,
    ]);
    let main = duplicates_in(&diagnostics, "main.nx");
    assert_eq!(main.len(), 1, "{diagnostics:#?}");
    assert!(
        main[0].message.contains("'a.nx' and 'b.nx'"),
        "{:#?}",
        main[0]
    );
    // Each of the two modules loads only its own visualizer declaration.
    assert!(duplicates_in(&diagnostics, "a.nx").is_empty());
    assert!(duplicates_in(&diagnostics, "b.nx").is_empty());
}

#[test]
fn a_document_holding_one_of_two_duplicates_is_told_at_its_own() {
    let a = icon_override("A");
    let main = format!("import \"./a.nx\"\n{}", icon_override("Main"));
    let diagnostics = diagnostics(&[("main.nx", &main), ("a.nx", &a), CHOICE_LIBRARY]);
    let main_duplicates = duplicates_in(&diagnostics, "main.nx");
    assert_eq!(main_duplicates.len(), 1, "{diagnostics:#?}");
    let visualizer_line = main
        .find("visualizer Choice")
        .expect("the visualizer declaration");
    assert_eq!(
        main_duplicates[0].labels[0].span.start_byte as usize, visualizer_line,
        "{:#?}",
        main_duplicates[0]
    );
}

#[test]
fn the_declaring_modules_visualizer_never_conflicts() {
    let a = icon_override("A");
    let diagnostics = diagnostics(&[
        ("main.nx", "import \"./a.nx\"\nlet root() = { 1 }"),
        ("a.nx", &a),
        (
            "lib.nx",
            "import \"@nx/visualizer\"\nexport type Choice = { label:string }\nvisualizer Choice = <ElementVisualizer title=label />",
        ),
    ]);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code.as_deref() != Some("duplicate-visualizer")),
        "{diagnostics:#?}"
    );
}

#[test]
fn an_unloaded_modules_visualizer_does_not_count() {
    let a = icon_override("A");
    let c = icon_override("C");
    let diagnostics = diagnostics(&[
        ("main.nx", "import \"./a.nx\"\nlet root() = { 1 }"),
        ("a.nx", &a),
        ("c.nx", &c),
        CHOICE_LIBRARY,
    ]);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code.as_deref() != Some("duplicate-visualizer")),
        "{diagnostics:#?}"
    );
}
