//! The visualizer library `@nx/visualizer` and the analysis of `visualizer` declarations written
//! with it.

mod support;
use nx_api::{
    build_workspace_program_artifact, LibraryRegistry, NxLibraryModule, NxLibrarySource,
    NxWorkspace, NxWorkspaceModule, ProgramArtifact,
};
use nx_value::NxValue;
use support::{eval_program_artifact, EvalResult};

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

const UI_ROOT: &str = "libraries/ui";
