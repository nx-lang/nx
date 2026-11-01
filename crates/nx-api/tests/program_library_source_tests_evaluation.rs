//! Libraries a host loads from memory: logical roots, library-to-library imports, load order,
//! immutable roots, and implicit imports that name a library.

use nx_api::{
    build_workspace_program_artifact, eval_program_artifact, validate_workspace, EvalResult,
    LibraryRegistry, NxDiagnostic, NxLibraryModule, NxLibrarySource, NxWorkspace,
    NxWorkspaceModule, ProgramArtifact, ProgramBuildContext,
};
use nx_value::NxValue;
use std::path::{Path, PathBuf};

const QUESTION_FLOW_ROOT: &str = "libraries/question-flow";
const CHAT_LINK_ROOT: &str = "libraries/chat-link";

fn question_flow() -> NxLibrarySource {
    NxLibrarySource::new(
        QUESTION_FLOW_ROOT,
        vec![
            NxLibraryModule::new("Step.nx", "export type Step = { id:string }"),
            NxLibraryModule::new(
                "QuestionFlow.nx",
                "export type QuestionFlow = { firstStep:Step }",
            ),
        ],
    )
}

fn chat_link() -> NxLibrarySource {
    NxLibrarySource::new(
        CHAT_LINK_ROOT,
        vec![NxLibraryModule::new(
            "ChatLinkConfig.nx",
            "import \"../question-flow\"\nexport type ChatLinkConfig = { title:string questionFlow:QuestionFlow }",
        )],
    )
}

fn loaded_registry() -> LibraryRegistry {
    let registry = LibraryRegistry::new();
    registry
        .load_libraries_from_sources(&[question_flow(), chat_link()])
        .unwrap_or_else(|diagnostics| panic!("libraries load: {diagnostics:?}"));
    registry
}

fn implicit_context(registry: &LibraryRegistry) -> ProgramBuildContext {
    registry
        .build_context()
        .with_implicit_imports([CHAT_LINK_ROOT, QUESTION_FLOW_ROOT])
}

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

fn build(files: &[(&str, &str)], context: &ProgramBuildContext) -> ProgramArtifact {
    build_workspace_program_artifact(&workspace(files), files[0].0, context)
        .unwrap_or_else(|diagnostics| panic!("workspace artifact: {diagnostics:?}"))
}

fn root_type_name(artifact: &ProgramArtifact) -> String {
    match eval_program_artifact(artifact) {
        EvalResult::Ok(NxValue::Record {
            type_name: Some(name),
            ..
        }) => name,
        EvalResult::Ok(value) => panic!("expected a record, got {value:?}"),
        EvalResult::Err(diagnostics) => panic!("evaluation failed: {diagnostics:?}"),
    }
}

fn has_code(diagnostics: &[NxDiagnostic], code: &str) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code.as_deref() == Some(code))
}

#[test]
fn a_library_loads_from_memory_under_its_logical_root() {
    let registry = LibraryRegistry::new();
    let library = registry
        .load_library_from_sources(&question_flow())
        .unwrap_or_else(|diagnostics| panic!("library load: {diagnostics:?}"));

    assert_eq!(library.root_path, PathBuf::from(QUESTION_FLOW_ROOT));
    let mut modules = library
        .modules
        .iter()
        .map(|module| module.file_name.as_str())
        .collect::<Vec<_>>();
    modules.sort();
    assert_eq!(
        modules,
        [
            "libraries/question-flow/QuestionFlow.nx",
            "libraries/question-flow/Step.nx"
        ]
    );
    assert!(library.dependency_roots.is_empty());
    // Nothing on disk is named by the root, so a load that read the filesystem would have failed.
    assert!(!Path::new(QUESTION_FLOW_ROOT).exists());

    let artifact = build(
        &[(
            "main.nx",
            "import \"./libraries/question-flow\"\nlet root() = <QuestionFlow firstStep={<Step id=\"a\" />} />",
        )],
        &registry.build_context(),
    );
    assert_eq!(root_type_name(&artifact), "QuestionFlow");
}

#[test]
fn one_in_memory_library_imports_another() {
    let registry = LibraryRegistry::new();
    registry
        .load_library_from_sources(&question_flow())
        .expect("question-flow");
    let chat_link = registry
        .load_library_from_sources(&chat_link())
        .unwrap_or_else(|diagnostics| panic!("chat-link: {diagnostics:?}"));

    assert_eq!(
        chat_link.dependency_roots,
        vec![PathBuf::from(QUESTION_FLOW_ROOT)]
    );
    let artifact = build(
        &[(
            "chat-link.nx",
            &format!("import \"./libraries/chat-link\"\nimport \"./libraries/question-flow\"\n{TENANT_SOURCE}"),
        )],
        &registry.build_context(),
    );
    assert_eq!(root_type_name(&artifact), "ChatLinkConfig");
}

#[test]
fn a_loaded_library_is_imported_implicitly() {
    let registry = loaded_registry();
    let artifact = build(
        &[("chat-link.nx", TENANT_SOURCE)],
        &implicit_context(&registry),
    );
    assert_eq!(root_type_name(&artifact), "ChatLinkConfig");

    let diagnostics = validate_workspace(
        &workspace(&[("chat-link.nx", TENANT_SOURCE)]),
        &implicit_context(&registry),
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn a_written_import_of_an_implicit_library_is_not_a_duplicate() {
    let registry = loaded_registry();
    let artifact = build(
        &[(
            "chat-link.nx",
            &format!("import {{ ChatLinkConfig }} from \"./libraries/chat-link\"\n{TENANT_SOURCE}"),
        )],
        &implicit_context(&registry),
    );
    assert_eq!(root_type_name(&artifact), "ChatLinkConfig");
}

#[test]
fn a_workspace_module_inside_a_library_root_is_refused() {
    let registry = loaded_registry();
    // One module takes a library module's exact identity; the other only lies under its root. Both
    // would let a tenant's text stand in for the shared library's inside its own program.
    for (identity, source) in [
        (
            "libraries/question-flow/Step.nx",
            "export type Step = { y:int }",
        ),
        (
            "libraries/question-flow/Evil.nx",
            "export type Evil = { x:int }",
        ),
    ] {
        let files = [("chat-link.nx", TENANT_SOURCE), (identity, source)];
        let error = build_workspace_program_artifact(
            &workspace(&files),
            "chat-link.nx",
            &implicit_context(&registry),
        )
        .expect_err("the workspace claims part of the library's namespace");
        assert!(
            has_code(&error, "workspace-module-in-library-root"),
            "{error:?}"
        );
        assert!(
            error
                .iter()
                .any(|diagnostic| diagnostic.message.contains(identity)
                    && diagnostic.message.contains(QUESTION_FLOW_ROOT)),
            "{error:?}"
        );

        let diagnostics = validate_workspace(&workspace(&files), &implicit_context(&registry));
        assert!(
            has_code(&diagnostics, "workspace-module-in-library-root"),
            "{diagnostics:?}"
        );
    }

    // A sibling of the root that merely shares its prefix is not inside it.
    let artifact = build(
        &[
            ("chat-link.nx", TENANT_SOURCE),
            (
                "libraries/question-flow-extras.nx",
                "export type Extra = { x:int }",
            ),
        ],
        &implicit_context(&registry),
    );
    assert_eq!(root_type_name(&artifact), "ChatLinkConfig");
}

#[test]
fn a_directory_library_whose_path_ends_in_a_workspace_identity_does_not_make_it_ambiguous() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let drawnui = temp.path().join("drawnui");
    std::fs::create_dir(&drawnui).expect("library dir");
    std::fs::write(
        drawnui.join("Label.nx"),
        "export type Label = { text:string }",
    )
    .expect("library module");
    let registry = LibraryRegistry::new();
    registry
        .load_library_from_directory(&drawnui)
        .unwrap_or_else(|diagnostics| panic!("directory library: {diagnostics:?}"));

    // The implicitly imported workspace module wins, as it did before libraries could be named
    // implicitly and as a written import of the identity still does.
    let artifact = build(
        &[
            ("input.nx", "let root() = <Control x={1} />"),
            ("drawnui", "export type Control = { x:int }"),
        ],
        &registry.build_context().with_implicit_imports(["drawnui"]),
    );
    assert_eq!(root_type_name(&artifact), "Control");
}

#[test]
fn a_union_case_field_default_declared_in_a_library_applies_to_a_tenant_construction() {
    let registry = LibraryRegistry::new();
    registry
        .load_libraries_from_sources(&[NxLibrarySource::new(
            "libraries/scale",
            vec![NxLibraryModule::new(
                "Scale.nx",
                "export type ScaleConfig =\n  | numeric { min:int = 0 max:int = 10 step:int = 1 label?:string }\n  | labeled { options:string+ }\n",
            )],
        )])
        .unwrap_or_else(|diagnostics| panic!("library loads: {diagnostics:?}"));
    let context = registry
        .build_context()
        .with_implicit_imports(["libraries/scale"]);
    let files = [("tenant/a.nx", "let root() = <ScaleConfig.numeric max=5 />")];

    let diagnostics = validate_workspace(&workspace(&files), &context);
    assert!(
        diagnostics.is_empty(),
        "an omitted field with a default is not missing: {diagnostics:?}"
    );

    let omitted_required = validate_workspace(
        &workspace(&[("tenant/a.nx", "let root() = <ScaleConfig.labeled />")]),
        &context,
    );
    assert!(
        has_code(&omitted_required, "missing-union-case-field"),
        "a field with neither a default nor `?` is still required: {omitted_required:?}"
    );

    match eval_program_artifact(&build(&files, &context)) {
        EvalResult::Ok(NxValue::Record { properties, .. }) => {
            assert_eq!(properties.get("min"), Some(&NxValue::Int(0)));
            assert_eq!(properties.get("max"), Some(&NxValue::Int(5)));
            assert_eq!(properties.get("step"), Some(&NxValue::Int(1)));
            assert!(!properties.contains_key("label"));
        }
        EvalResult::Ok(value) => panic!("expected a record, got {value:?}"),
        EvalResult::Err(diagnostics) => panic!("evaluation failed: {diagnostics:?}"),
    }
}

const TENANT_SOURCE: &str = "let root() = <ChatLinkConfig title=\"Hi\" questionFlow={<QuestionFlow firstStep={<Step id=\"a\" />} />} />";
