//! The viewer library `@nx/viewer` and the analysis of `viewer` declarations written with it.

use crate::{
    build_workspace_program_artifact, eval_program_artifact, standard_library, validate_workspace,
    EvalResult, LibraryRegistry, NxDiagnostic, NxLibraryModule, NxLibrarySource, NxSeverity,
    NxWorkspace, NxWorkspaceModule, ProgramArtifact, ProgramBuildContext,
};
use nx_hir::{InterfaceItemKind, Visibility};
use nx_value::NxValue;

const VIEWER_ROOT: &str = "@nx/viewer";

const VIEWER_TYPES: [&str; 8] = [
    "CaseViewer",
    "ElementLayout",
    "ElementViewer",
    "Placement",
    "PropertyViewer",
    "UnionViewer",
    "ValueFormat",
    "ValueViewer",
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

/// Every diagnostic of one module that begins with `import "@nx/viewer"`.
fn viewer_diagnostics(body: &str) -> Vec<NxDiagnostic> {
    diagnostics(&[("main.nx", &format!("import \"@nx/viewer\"\n{body}"))])
}

fn assert_clean(body: &str) {
    let diagnostics = viewer_diagnostics(body);
    assert!(
        diagnostics.is_empty(),
        "expected no diagnostics for {body:?}: {diagnostics:#?}"
    );
}

fn assert_error(body: &str, code: Option<&str>, fragments: &[&str]) {
    let diagnostics = viewer_diagnostics(body);
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
fn the_viewer_library_exports_exactly_its_eight_types() {
    let library = standard_library(VIEWER_ROOT).expect("the viewer library");
    assert!(library.diagnostics.is_empty(), "{:#?}", library.diagnostics);
    let mut exported = library
        .interface_items
        .iter()
        .filter(|item| item.visibility == Visibility::Export)
        .map(|item| item.item_name.as_str())
        .filter(|name| !name.contains('.'))
        .collect::<Vec<_>>();
    exported.sort();
    assert_eq!(exported, VIEWER_TYPES);
    assert!(
        library.interface_items.iter().all(|item| !matches!(
            item.item,
            InterfaceItemKind::Function { .. } | InterfaceItemKind::Component { .. }
        )),
        "the library holds types only"
    );
}

#[test]
fn every_viewer_type_field_and_case_is_documented() {
    let library = standard_library(VIEWER_ROOT).expect("the viewer library");
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
            other => panic!("unexpected declaration in the viewer library: {other:?}"),
        }
    }
    assert!(undocumented.is_empty(), "undocumented: {undocumented:?}");
}

#[test]
fn an_element_viewer_with_property_viewers() {
    assert_clean(
        "type Choice = { value:string label:string description?:string }\n\
         viewer Choice = <ElementViewer layout=row title=label subtitle=description badge=value><PropertyViewer of=value place=details /></ElementViewer>",
    );
}

#[test]
fn a_property_viewer_names_a_property_of_its_target() {
    assert_error(
        "type Choice = { label:string }\n\
         viewer Choice = <ElementViewer><PropertyViewer of=value /></ElementViewer>",
        None,
        &["value"],
    );
}

#[test]
fn case_labels() {
    assert_clean(
        "type ChoiceLayout = list | grid | chips\n\
         viewer ChoiceLayout = <UnionViewer><CaseViewer of=list label=\"a list\" /><CaseViewer of=chips label=\"chips\" example=\"◉ A  ○ B\" /></UnionViewer>",
    );
}

#[test]
fn a_case_that_is_not_in_the_union() {
    assert_error(
        "type ChoiceLayout = list | grid | chips\n\
         viewer ChoiceLayout = <UnionViewer><CaseViewer of=table /></UnionViewer>",
        None,
        &["table"],
    );
}

#[test]
fn an_image_url_alias() {
    assert_clean(
        "type ImageUrl = string\n\
         viewer ImageUrl = <ValueViewer format=image />",
    );
}

// --- Viewer declarations ----------------------------------------------------------------------

#[test]
fn a_viewer_beside_its_type() {
    assert_clean(
        "type Choice = { value:string label:string }\n\
         viewer Choice = <ElementViewer title=label badge=value />",
    );
}

#[test]
fn viewer_stays_an_ordinary_name_elsewhere() {
    let diagnostics = diagnostics(&[(
        "main.nx",
        "type Settings = { viewer:string }\n\
         let viewer = \"Ada\"\n\
         component <Panel viewer:string /> = { <Label text={viewer} /> }",
    )]);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != NxSeverity::Error),
        "{diagnostics:#?}"
    );
}

#[test]
fn a_viewer_declaration_takes_no_visibility() {
    assert_error(
        "type Choice = { label:string }\n\
         export viewer Choice = <ElementViewer title=label />",
        None,
        &["takes no visibility modifier"],
    );
}

#[test]
fn each_kind_takes_its_record() {
    assert_clean(
        "type Choice = { label:string }\n\
         type ChoiceLayout = list | grid | chips\n\
         type ImageUrl = string\n\
         component <Card title:string /> = { <Panel /> }\n\
         abstract component <Question label:string />\n\
         external component <SearchBox placeholder?:string />\n\
         action Saved = { id:string }\n\
         viewer Choice = <ElementViewer title=label />\n\
         viewer ChoiceLayout = <UnionViewer />\n\
         viewer ImageUrl = <ValueViewer format=image />\n\
         viewer Card = <ElementViewer title=title />\n\
         viewer Question = <ElementViewer title=label />\n\
         viewer SearchBox = <ElementViewer subtitle=placeholder />\n\
         viewer Saved = <ElementViewer title=id />",
    );
}

#[test]
fn the_wrong_record_for_the_kind() {
    assert_error(
        "type ChoiceLayout = list | grid\n\
         viewer ChoiceLayout = <ElementViewer />",
        Some("viewer-record-mismatch"),
        &["UnionViewer", "ElementViewer"],
    );
    assert_error(
        "type Choice = { label:string }\n\
         viewer Choice = \"Choice\"",
        Some("viewer-record-mismatch"),
        &["ElementViewer"],
    );
}

#[test]
fn a_target_that_cannot_have_a_viewer() {
    assert_error(
        "let answer = 42\n\
         viewer answer = <ValueViewer format=text />",
        Some("viewer-target-not-allowed"),
        &["'answer' cannot have a viewer"],
    );
    assert_error(
        "type Range = { T:type start:T end:T }\n\
         viewer Range = <ElementViewer />",
        Some("viewer-target-not-allowed"),
        &["'Range' cannot have a viewer"],
    );
    assert_error(
        "let greet(name:string) = { name }\n\
         viewer greet = <ElementViewer />",
        Some("viewer-target-not-allowed"),
        &["'greet' cannot have a viewer"],
    );
    assert_error(
        "type Shape = | circle { radius:int } | square\n\
         viewer Shape = <UnionViewer />",
        Some("viewer-target-not-allowed"),
        &["'Shape' cannot have a viewer"],
    );
    assert_error(
        "type Choice = { label:string }\n\
         type Choices = Choice+\n\
         viewer Choices = <ValueViewer format=text />",
        Some("viewer-target-not-allowed"),
        &["'Choices' cannot have a viewer"],
    );
}

#[test]
fn a_derived_declaration_is_not_a_target() {
    for target in [
        "User.Update",
        "User.Property",
        "Card.Props",
        "Card.Props.Update",
        "Card.Props.Property",
        "Card.State",
    ] {
        assert_error(
            &format!(
                "type User = {{ name:string }}\n\
                 component <Card title:string /> = {{ state {{ open:boolean = false }} <Panel /> }}\n\
                 viewer {target} = <ElementViewer />"
            ),
            Some("viewer-target-not-allowed"),
            &[&format!("'{target}' cannot have a viewer")],
        );
    }
}

#[test]
fn an_unknown_target_is_an_unknown_name() {
    assert_error(
        "viewer Nothing = <ElementViewer />",
        Some("unknown-viewer-target"),
        &["Nothing"],
    );
}

#[test]
fn an_emitted_action_is_a_target() {
    assert_clean(
        "component <Form emits { Submitted { email:string } } /> = { <Panel /> }\n\
         viewer Form.Submitted = <ElementViewer title=email />",
    );
}

#[test]
fn children_take_the_target_too() {
    assert_clean(
        "type Choice = { value:string label:string }\n\
         viewer Choice = <ElementViewer title=label><PropertyViewer of=value label=\"Stored as\" /></ElementViewer>",
    );
}

#[test]
fn a_components_viewer_names_its_props() {
    assert_clean(
        "component <Question label:string help?:string /> = { <Panel /> }\n\
         viewer Question = <ElementViewer title=label subtitle=help />",
    );
    // A component's state is not among its props.
    assert_error(
        "component <Counter step:int /> = { state { count:int = 0 } <Panel /> }\n\
         viewer Counter = <ElementViewer title=count />",
        None,
        &["count"],
    );
}

#[test]
fn a_misspelled_property_is_a_type_error() {
    assert_error(
        "type Choice = { label:string }\n\
         viewer Choice = <ElementViewer title=lable />",
        None,
        &["lable"],
    );
}

#[test]
fn a_wrong_explicit_target() {
    assert_error(
        "type Choice = { label:string }\n\
         type Other = { name:string }\n\
         viewer Choice = <ElementViewer Target=Other title=name />",
        Some("viewer-target-mismatch"),
        &["Target=Other", "Choice"],
    );
    // Binding the target itself is allowed.
    assert_clean(
        "type Choice = { label:string }\n\
         viewer Choice = <ElementViewer Target=Choice title=label />",
    );
}

#[test]
fn outside_a_viewer_declaration_the_rule_is_unchanged() {
    assert_error("let h = <ElementViewer />", None, &["Target"]);
}

#[test]
fn a_computed_label_is_rejected() {
    for value in [
        "<ElementViewer name={prefix} />",
        "<ElementViewer>Pick {prefix}</ElementViewer>",
        "<ElementViewer>{if true { <PropertyViewer of=label /> }}</ElementViewer>",
        "<ElementViewer>{for x in { 1 2 } { <PropertyViewer of=label /> }}</ElementViewer>",
        "{ <ElementViewer /> }",
    ] {
        assert_error(
            &format!(
                "type Choice = {{ label:string }}\n\
                 let prefix = \"Pick\"\n\
                 viewer Choice = {value}"
            ),
            None,
            &["A viewer's values must be written out"],
        );
    }
    // A bare name resolves only against a union's cases, never to a value.
    assert_error(
        "type Choice = { label:string }\n\
         let prefix = \"Pick\"\n\
         viewer Choice = <ElementViewer name=prefix />",
        None,
        &["prefix"],
    );
}

#[test]
fn literals_and_bare_names_are_accepted() {
    assert_clean(
        "component <Question label:string required:boolean = true /> = { <Panel /> }\n\
         viewer Question = <ElementViewer name=\"Question\" title=label><PropertyViewer of=required whenTrue=\"Required\" whenFalse=\"Optional\" /></ElementViewer>",
    );
}

#[test]
fn twice_in_one_module() {
    let diagnostics = viewer_diagnostics(
        "type Choice = { label:string }\n\
         viewer Choice = <ElementViewer title=label />\n\
         viewer Choice = <ElementViewer name=\"Option\" />",
    );
    let duplicates = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code.as_deref() == Some("duplicate-viewer"))
        .collect::<Vec<_>>();
    assert_eq!(duplicates.len(), 1, "{diagnostics:#?}");
    assert!(duplicates[0].message.contains("Choice"));
    assert_eq!(duplicates[0].labels.len(), 2, "{:#?}", duplicates[0]);
}

#[test]
fn a_viewer_of_an_imported_declaration() {
    let diagnostics = diagnostics(&[
        (
            "main.nx",
            "import \"@nx/viewer\"\nimport { Choice, Card } from \"./lib.nx\"\n\
             viewer Choice = <ElementViewer title=label />\n\
             viewer Card = <ElementViewer title=title />",
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

/// A library with components, one of them abstract, and its own viewer declarations.
fn ui_registry() -> LibraryRegistry {
    let registry = LibraryRegistry::new();
    registry
        .load_library_from_sources(&NxLibrarySource::new(
            UI_ROOT,
            vec![NxLibraryModule::new(
                "ui.nx",
                "import \"@nx/viewer\"
export abstract component <Question label:string help?:string />
export component <Card extends Question title:string /> = { <Panel /> }
viewer Question = <ElementViewer title=label subtitle=help />
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
fn a_viewer_of_an_imported_component_and_a_library_with_viewers() {
    // The library's own viewer declarations analyze cleanly, and a document may declare a viewer
    // for a declaration it imports from the library.
    let artifact = library_artifact(&format!(
        "import \"@nx/viewer\"
import {{ Card }} from \"./{UI_ROOT}\"
viewer Card = <ElementViewer name=\"Card\" title=title badge=label />
let root(): Card.Props = {{ <Card.Props label=\"Q\" title=\"Hi\" /> }}"
    ));
    assert!(
        artifact.diagnostics.is_empty(),
        "{:#?}",
        artifact.diagnostics
    );
}

// --- Viewer declarations across a program ----------------------------------------------------

const CHOICE_LIBRARY: (&str, &str) = ("lib.nx", "export type Choice = { label:string }");

fn rewording(name: &str) -> String {
    format!(
        "import \"@nx/viewer\"\nimport {{ Choice }} from \"./lib.nx\"\nviewer Choice = <ElementViewer name=\"{name}\" />\nexport let marker = 1"
    )
}

fn duplicates_in(diagnostics: &[NxDiagnostic], file: &str) -> Vec<NxDiagnostic> {
    diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.code.as_deref() == Some("duplicate-viewer")
                && diagnostic
                    .labels
                    .first()
                    .is_some_and(|label| label.file == file)
        })
        .cloned()
        .collect()
}

#[test]
fn two_program_modules_declare_a_viewer_for_the_same_target() {
    let a = rewording("A");
    let b = rewording("B");
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
    // Each of the two modules loads only its own viewer declaration.
    assert!(duplicates_in(&diagnostics, "a.nx").is_empty());
    assert!(duplicates_in(&diagnostics, "b.nx").is_empty());
}

#[test]
fn a_document_holding_one_of_two_duplicates_is_told_at_its_own() {
    let a = rewording("A");
    let main = format!("import \"./a.nx\"\n{}", rewording("Main"));
    let diagnostics = diagnostics(&[("main.nx", &main), ("a.nx", &a), CHOICE_LIBRARY]);
    let main_duplicates = duplicates_in(&diagnostics, "main.nx");
    assert_eq!(main_duplicates.len(), 1, "{diagnostics:#?}");
    let viewer_line = main.find("viewer Choice").expect("the viewer declaration");
    assert_eq!(
        main_duplicates[0].labels[0].span.start_byte as usize, viewer_line,
        "{:#?}",
        main_duplicates[0]
    );
}

#[test]
fn the_declaring_modules_viewer_never_conflicts() {
    let a = rewording("A");
    let diagnostics = diagnostics(&[
        ("main.nx", "import \"./a.nx\"\nlet root() = { 1 }"),
        ("a.nx", &a),
        (
            "lib.nx",
            "import \"@nx/viewer\"\nexport type Choice = { label:string }\nviewer Choice = <ElementViewer title=label />",
        ),
    ]);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code.as_deref() != Some("duplicate-viewer")),
        "{diagnostics:#?}"
    );
}

#[test]
fn an_unloaded_modules_viewer_does_not_count() {
    let a = rewording("A");
    let c = rewording("C");
    let diagnostics = diagnostics(&[
        ("main.nx", "import \"./a.nx\"\nlet root() = { 1 }"),
        ("a.nx", &a),
        ("c.nx", &c),
        CHOICE_LIBRARY,
    ]);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code.as_deref() != Some("duplicate-viewer")),
        "{diagnostics:#?}"
    );
}
