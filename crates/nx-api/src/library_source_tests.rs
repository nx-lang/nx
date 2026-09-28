//! Libraries a host loads from memory: logical roots, library-to-library imports, load order,
//! immutable roots, and implicit imports that name a library.

use crate::{
    build_workspace_program_artifact, eval_program_artifact, validate_workspace, EvalResult,
    LibraryRegistry, NxDiagnostic, NxLibraryModule, NxLibrarySource, NxWorkspace,
    NxWorkspaceModule, ProgramArtifact, ProgramBuildContext,
};
use nx_value::NxValue;
use std::path::{Path, PathBuf};
use std::sync::Arc;

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

const TENANT_SOURCE: &str = "let root() = <ChatLinkConfig title=\"Hi\" questionFlow={<QuestionFlow firstStep={<Step id=\"a\" />} />} />";

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
fn a_missing_dependency_fails_the_load_and_retains_nothing() {
    let registry = LibraryRegistry::new();
    let diagnostics = registry
        .load_library_from_sources(&chat_link())
        .expect_err("chat-link needs question-flow");

    let missing = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code.as_deref() == Some("library-dependency-missing"))
        .unwrap_or_else(|| panic!("{diagnostics:?}"));
    assert!(missing.message.contains("../question-flow"), "{missing:?}");
    assert!(missing.message.contains(QUESTION_FLOW_ROOT), "{missing:?}");
    assert_eq!(
        missing.labels[0].file,
        "libraries/chat-link/ChatLinkConfig.nx"
    );
    assert_eq!(missing.labels[0].span.start_line, 1);

    // chat-link was not retained: loading it once its dependency is present is a first load, not a
    // reload of a partial snapshot.
    registry
        .load_library_from_sources(&question_flow())
        .expect("question-flow");
    registry
        .load_library_from_sources(&chat_link())
        .unwrap_or_else(|diagnostics| panic!("chat-link: {diagnostics:?}"));
}

#[test]
fn a_cycle_among_libraries_is_refused() {
    let a = NxLibrarySource::new(
        "libraries/a",
        vec![NxLibraryModule::new(
            "A.nx",
            "import \"../b\"\nexport type A = { id:string }",
        )],
    );
    let b = NxLibrarySource::new(
        "libraries/b",
        vec![NxLibraryModule::new(
            "B.nx",
            "import \"../a\"\nexport type B = { id:string }",
        )],
    );
    let registry = LibraryRegistry::new();
    let diagnostics = registry
        .load_libraries_from_sources(&[a.clone(), b])
        .expect_err("a cycle is refused");
    assert!(
        has_code(&diagnostics, "library-dependency-cycle"),
        "{diagnostics:?}"
    );
    assert!(
        diagnostics[0]
            .message
            .contains("libraries/a -> libraries/b -> libraries/a"),
        "{diagnostics:?}"
    );

    // Loaded one at a time, each waits on the other, which is a missing dependency.
    let diagnostics = registry
        .load_library_from_sources(&a)
        .expect_err("a needs b");
    assert!(
        has_code(&diagnostics, "library-dependency-missing"),
        "{diagnostics:?}"
    );

    let own = NxLibrarySource::new(
        "libraries/own",
        vec![NxLibraryModule::new(
            "Own.nx",
            "import \".\"\nexport type Own = { id:string }",
        )],
    );
    let diagnostics = registry
        .load_library_from_sources(&own)
        .expect_err("a library importing its own root is a cycle");
    assert!(
        has_code(&diagnostics, "library-dependency-cycle"),
        "{diagnostics:?}"
    );
}

#[test]
fn an_import_that_climbs_above_the_logical_root_is_refused() {
    let escaping = NxLibrarySource::new(
        "libraries/escaping",
        vec![NxLibraryModule::new(
            "Escaping.nx",
            "import \"../../../outside\"\nexport type Escaping = { id:string }",
        )],
    );
    let diagnostics = LibraryRegistry::new()
        .load_library_from_sources(&escaping)
        .expect_err("the import escapes the root");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("escapes")),
        "{diagnostics:?}"
    );
}

#[test]
fn a_library_with_errors_is_not_retained() {
    let broken = NxLibrarySource::new(
        "libraries/broken",
        vec![NxLibraryModule::new(
            "Broken.nx",
            "export let broken(): int = { \"oops\" }",
        )],
    );
    let fixed = NxLibrarySource::new(
        "libraries/broken",
        vec![NxLibraryModule::new(
            "Broken.nx",
            "export let broken(): int = { 1 }",
        )],
    );
    let registry = LibraryRegistry::new();
    let diagnostics = registry
        .load_library_from_sources(&broken)
        .expect_err("a type error fails the load");
    assert_eq!(diagnostics[0].labels[0].file, "libraries/broken/Broken.nx");

    registry
        .load_library_from_sources(&fixed)
        .unwrap_or_else(|diagnostics| panic!("the fixed library loads: {diagnostics:?}"));
}

#[test]
fn a_root_reloads_identically_and_refuses_different_content() {
    let registry = LibraryRegistry::new();
    let first = registry
        .load_library_from_sources(&question_flow().with_version("1"))
        .expect("first load");
    let again = registry
        .load_library_from_sources(&question_flow().with_version("1"))
        .expect("an identical reload");
    assert!(Arc::ptr_eq(&first, &again));

    // The modules may be listed in another order: they are the same modules.
    let mut reordered = question_flow().with_version("1");
    reordered.modules.reverse();
    let reordered = registry
        .load_library_from_sources(&reordered)
        .expect("a reordered reload");
    assert!(Arc::ptr_eq(&first, &reordered));

    let mut edited = question_flow().with_version("1");
    edited.modules[0].source = Arc::from("export type Step = { id:string label:string }");
    let diagnostics = registry
        .load_library_from_sources(&edited)
        .expect_err("different sources are refused");
    assert!(
        has_code(&diagnostics, "library-root-conflict"),
        "{diagnostics:?}"
    );
    assert!(
        diagnostics[0].message.contains(QUESTION_FLOW_ROOT),
        "{diagnostics:?}"
    );

    let diagnostics = registry
        .load_library_from_sources(&question_flow().with_version("2"))
        .expect_err("a different version is refused");
    assert!(
        has_code(&diagnostics, "library-root-conflict"),
        "{diagnostics:?}"
    );

    let still = registry
        .load_library_from_sources(&question_flow().with_version("1"))
        .expect("the first snapshot is still loaded");
    assert!(Arc::ptr_eq(&first, &still));
}

#[test]
fn a_batch_loads_in_dependency_order_and_answers_in_the_given_order() {
    let registry = LibraryRegistry::new();
    let loaded = registry
        .load_libraries_from_sources(&[chat_link(), question_flow()])
        .unwrap_or_else(|diagnostics| panic!("batch load: {diagnostics:?}"));

    assert_eq!(loaded[0].root_path, PathBuf::from(CHAT_LINK_ROOT));
    assert_eq!(loaded[1].root_path, PathBuf::from(QUESTION_FLOW_ROOT));
}

#[test]
fn builds_against_a_registry_reuse_its_snapshots() {
    let registry = loaded_registry();
    let context = implicit_context(&registry);
    let first = build(&[("chat-link.nx", TENANT_SOURCE)], &context);
    let second = build(
        &[(
            "chat-link.nx",
            "let root() = <ChatLinkConfig title=\"Other\" questionFlow={<QuestionFlow firstStep={<Step id=\"b\" />} />} />",
        )],
        &context,
    );

    let loaded = registry
        .load_libraries_from_sources(&[question_flow(), chat_link()])
        .expect("identical reload");
    for artifact in [&first, &second] {
        for library in &loaded {
            assert!(
                artifact
                    .libraries
                    .iter()
                    .any(|used| Arc::ptr_eq(used, library)),
                "the build did not use the loaded snapshot of {}",
                library.root_path.display()
            );
        }
    }
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
fn an_implicit_import_naming_a_workspace_module_and_a_library_is_ambiguous() {
    let registry = loaded_registry();
    let workspace = workspace(&[
        ("chat-link.nx", TENANT_SOURCE),
        (
            "libraries/question-flow",
            "export type Shadow = { id:string }",
        ),
    ]);
    let error = build_workspace_program_artifact(
        &workspace,
        "chat-link.nx",
        &registry
            .build_context()
            .with_implicit_imports([QUESTION_FLOW_ROOT]),
    )
    .expect_err("the identity names both");
    assert!(has_code(&error, "implicit-import-ambiguous"), "{error:?}");
}

#[test]
fn an_implicit_import_naming_nothing_loaded_is_unknown() {
    let registry = loaded_registry();
    let error = build_workspace_program_artifact(
        &workspace(&[("chat-link.nx", TENANT_SOURCE)]),
        "chat-link.nx",
        &registry
            .build_context()
            .with_implicit_imports(["libraries/missing"]),
    )
    .expect_err("nothing is loaded at the identity");
    assert!(has_code(&error, "implicit-import-not-found"), "{error:?}");
    assert!(
        error
            .iter()
            .any(|diagnostic| diagnostic.message.contains("'libraries/missing'")),
        "{error:?}"
    );
}

#[test]
fn a_library_version_reaches_the_program_and_its_fingerprint() {
    let unversioned = LibraryRegistry::new();
    unversioned
        .load_libraries_from_sources(&[question_flow(), chat_link()])
        .expect("libraries");
    let versioned = LibraryRegistry::new();
    versioned
        .load_libraries_from_sources(&[question_flow().with_version("7"), chat_link()])
        .expect("libraries");

    let lhs = build(
        &[("chat-link.nx", TENANT_SOURCE)],
        &implicit_context(&unversioned),
    );
    let rhs = build(
        &[("chat-link.nx", TENANT_SOURCE)],
        &implicit_context(&versioned),
    );
    assert_ne!(lhs.fingerprint, rhs.fingerprint);

    let entries = rhs.source_entries();
    let question_flow_entry = entries
        .iter()
        .find(|entry| entry.identity == "libraries/question-flow/QuestionFlow.nx")
        .expect("the library module's source is one of the program's");
    assert_eq!(question_flow_entry.version, Some("7"));
    assert!(question_flow_entry
        .source
        .contains("export type QuestionFlow"));
    let chat_link_entry = entries
        .iter()
        .find(|entry| entry.identity == "libraries/chat-link/ChatLinkConfig.nx")
        .expect("chat-link's source");
    assert_eq!(chat_link_entry.version, None);
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

fn warning_library() -> NxLibrarySource {
    NxLibrarySource::new(
        "libraries/warns",
        vec![NxLibraryModule::new(
            "Warns.nx",
            "export type Warns = { id:string }\nexport let f(n:int) = { n ?? 0 }",
        )],
    )
}

#[test]
fn a_library_warning_is_reported_at_load_and_not_in_workspace_validation() {
    let registry = LibraryRegistry::new();
    let library = registry
        .load_library_from_sources(&warning_library())
        .unwrap_or_else(|diagnostics| panic!("a warning does not fail the load: {diagnostics:?}"));

    let load_diagnostics = library.api_diagnostics();
    let warning = load_diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code.as_deref() == Some("fallback-never-taken"))
        .unwrap_or_else(|| panic!("{load_diagnostics:?}"));
    assert_eq!(warning.labels[0].file, "libraries/warns/Warns.nx");
    // Rendered against the library's own text, so the position is the warning's, not 1:1.
    assert_eq!(warning.labels[0].span.start_line, 2);

    let context = registry
        .build_context()
        .with_implicit_imports(["libraries/warns"]);
    let files = [("tenant.nx", "let root() = <Warns id=\"a\" />")];
    let diagnostics = validate_workspace(&workspace(&files), &context);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    build(&files, &context);
}

#[test]
fn a_failed_build_answers_with_the_diagnostics_validation_does() {
    let registry = LibraryRegistry::new();
    registry
        .load_libraries_from_sources(&[question_flow(), chat_link(), warning_library()])
        .expect("libraries");
    let context = registry.build_context().with_implicit_imports([
        CHAT_LINK_ROOT,
        QUESTION_FLOW_ROOT,
        "libraries/warns",
    ]);
    let files = [(
        "chat-link.nx",
        "let root() = <ChatLinkConfig title={1} questionFlow={<QuestionFlow firstStep={<Step id=\"a\" />} />} />",
    )];

    let validation = validate_workspace(&workspace(&files), &context);
    let build_error =
        build_workspace_program_artifact(&workspace(&files), "chat-link.nx", &context)
            .expect_err("the title has the wrong type");
    assert!(!validation.is_empty());
    assert_eq!(build_error, validation);
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
fn a_directory_library_warning_is_still_reported_by_workspace_validation() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let ui = temp.path().join("ui");
    std::fs::create_dir(&ui).expect("library dir");
    std::fs::write(
        ui.join("Label.nx"),
        "export type Label = { text:string }\nexport let f(n:int) = { n ?? 0 }",
    )
    .expect("library module");
    let registry = LibraryRegistry::new();
    registry
        .load_library_from_directory(&ui)
        .unwrap_or_else(|diagnostics| panic!("directory library: {diagnostics:?}"));

    // A directory load answers with no warnings, so validation stays where its callers see them,
    // as it did before in-memory libraries existed.
    let diagnostics = validate_workspace(
        &workspace(&[(
            "input.nx",
            "import \"./ui\"\nlet root() = <Label text=\"hi\" />",
        )]),
        &registry.build_context(),
    );
    let warning = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code.as_deref() == Some("fallback-never-taken"))
        .unwrap_or_else(|| panic!("{diagnostics:?}"));
    assert!(
        warning.labels[0].file.ends_with("ui/Label.nx"),
        "{warning:?}"
    );
    assert_eq!(warning.labels[0].span.start_line, 2);
}
