//! Standard libraries: NX source the compiler carries and a module imports as `@nx/<name>`, and
//! the agent library, the first of them.

use crate::artifacts::{build_standard_library, StandardLibrary};
use crate::{
    build_program_artifact_from_source, build_workspace_program_artifact, eval_program_artifact,
    standard_libraries, standard_library, standard_library_entry, standard_library_for_module,
    validate_workspace, EvalResult, LibraryRegistry, NxDiagnostic, NxLibraryModule,
    NxLibrarySource, NxSeverity, NxWorkspace, NxWorkspaceModule, ProgramArtifact,
    ProgramBuildContext, StandardLibraryModule, StandardLibraryStability,
};
use nx_hir::{InterfaceItemKind, Visibility};
use nx_value::NxValue;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

const AGENT_ROOT: &str = "@nx/agent";
const AGENT_MODULE: &str = "@nx/agent/agent.nx";
const HOST_ROOT: &str = "libraries/chat-link";
const SUPPORT_AGENT: &str = include_str!("../../../specs/ir-conformance/agent-library/main.nx");

const AGENT_TYPES: [&str; 13] = [
    "Agent",
    "AgentLimits",
    "Connection",
    "Document",
    "FunctionTool",
    "HttpArguments",
    "HttpConnection",
    "HttpMethod",
    "HttpParam",
    "HttpTool",
    "Tool",
    "ToolContext",
    "WebSearchTool",
];

/// ReachMe's shape: a host library that imports the agent library and extends its abstract types.
const HOST_SOURCE: &str = r#"import "@nx/agent"

export type RecordSearchTool extends Tool = {
  recordKind: string
  maxResults: int = 5
}

export type ChatToolContext extends ToolContext = {
  conversationId: string
  contactEmail?: string
}

export type AssistantConfig = { enabled?:boolean agent?:Agent }

export abstract external component <FlowStep id:string />
export external component <AgentStep extends FlowStep
  agent?: Agent
  content prompt: string
/>
"#;

fn host_library() -> NxLibrarySource {
    NxLibrarySource::new(
        HOST_ROOT,
        vec![NxLibraryModule::new("ChatLink.nx", HOST_SOURCE)],
    )
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

fn errors(files: &[(&str, &str)], context: &ProgramBuildContext) -> Vec<NxDiagnostic> {
    validate_workspace(&workspace(files), context)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == NxSeverity::Error)
        .collect()
}

/// The errors of one module that begins with `import "@nx/agent"`.
fn agent_errors(body: &str) -> Vec<NxDiagnostic> {
    errors(
        &[("main.nx", &format!("import \"@nx/agent\"\n{body}"))],
        &ProgramBuildContext::empty(),
    )
}

fn assert_one_error(diagnostics: &[NxDiagnostic], fragments: &[&str]) {
    assert!(
        diagnostics.iter().any(|diagnostic| fragments
            .iter()
            .all(|fragment| diagnostic.message.contains(fragment))),
        "expected an error containing {fragments:?}: {diagnostics:#?}"
    );
}

fn has_code(diagnostics: &[NxDiagnostic], code: &str) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code.as_deref() == Some(code))
}

fn library_roots(artifact: &ProgramArtifact) -> Vec<String> {
    artifact
        .libraries
        .iter()
        .map(|library| library.root_path.to_string_lossy().to_string())
        .collect()
}

fn root_record(artifact: &ProgramArtifact) -> (String, Vec<(String, NxValue)>) {
    match eval_program_artifact(artifact) {
        EvalResult::Ok(NxValue::Record {
            type_name: Some(name),
            properties,
        }) => (name, properties.into_iter().collect()),
        EvalResult::Ok(value) => panic!("expected a record, got {value:?}"),
        EvalResult::Err(diagnostics) => panic!("evaluation failed: {diagnostics:?}"),
    }
}

// --- The mechanism ---------------------------------------------------------------------------

#[test]
fn the_table_holds_the_agent_library() {
    let roots = standard_libraries()
        .iter()
        .map(|library| library.root)
        .collect::<Vec<_>>();
    assert_eq!(roots, [AGENT_ROOT]);

    let agent = standard_library_entry(AGENT_ROOT).expect("the agent library");
    assert_eq!(agent.stability, StandardLibraryStability::Unstable);
    assert_eq!(agent.typescript_package, "@nx-lang/agent");
    assert_eq!(agent.csharp_namespace, "NxLang.Agent");
    assert_eq!(
        standard_library_for_module(AGENT_MODULE).map(|library| library.root),
        Some(AGENT_ROOT)
    );
    assert!(standard_library_for_module(nx_hir::PRELUDE_MODULE_IDENTITY).is_none());
    assert!(standard_library_entry("@nx/nope").is_none());
}

#[test]
fn a_standard_library_is_analyzed_once_and_is_free_of_diagnostics() {
    let first = standard_library(AGENT_ROOT).expect("the agent library");
    let second = standard_library(AGENT_ROOT).expect("the agent library");
    assert!(Arc::ptr_eq(first, second));
    assert!(first.diagnostics.is_empty(), "{:#?}", first.diagnostics);
    assert_eq!(first.root_path, PathBuf::from(AGENT_ROOT));
    assert_eq!(
        first
            .modules
            .iter()
            .map(|module| module.file_name.as_str())
            .collect::<Vec<_>>(),
        [AGENT_MODULE]
    );

    // Programs built through separate registries bind to that one snapshot.
    for _ in 0..3 {
        let artifact = build(
            &[("main.nx", "import \"@nx/agent\"\nlet root(): Tool* = { }")],
            &LibraryRegistry::new().build_context(),
        );
        let linked = artifact
            .libraries
            .iter()
            .find(|library| library.root_path.as_path() == std::path::Path::new(AGENT_ROOT))
            .expect("the program links the agent library");
        assert!(Arc::ptr_eq(linked, first));
    }
}

#[test]
fn a_single_source_imports_a_standard_library_with_an_empty_context() {
    let artifact = build_program_artifact_from_source(
        "import \"@nx/agent\"\nlet root() = { <Agent name=\"support\">Be brief.</Agent> }",
        "main.nx",
        &ProgramBuildContext::empty(),
    )
    .expect("program artifact");
    assert!(
        artifact.diagnostics.is_empty(),
        "{:#?}",
        artifact.diagnostics
    );

    let (type_name, properties) = root_record(&artifact);
    assert_eq!(type_name, "Agent");
    assert!(properties.contains(&(
        "instructions".to_string(),
        NxValue::String("Be brief.".to_string())
    )));
    assert!(library_roots(&artifact).contains(&AGENT_ROOT.to_string()));
}

#[test]
fn a_standard_library_is_not_in_scope_without_an_import() {
    let diagnostics = errors(
        &[(
            "main.nx",
            "let root(): Agent = { <Agent name=\"support\">Be brief.</Agent> }",
        )],
        &ProgramBuildContext::empty(),
    );
    assert_one_error(&diagnostics, &["Agent"]);

    // A module's own declaration of a library's name is untouched, and the program links nothing
    // it did not link before.
    let artifact = build(
        &[(
            "main.nx",
            "type Tool = { label:string }\nlet root() = { <Tool label=\"x\" /> }",
        )],
        &ProgramBuildContext::empty(),
    );
    assert_eq!(library_roots(&artifact), [nx_hir::PRELUDE_MODULE_IDENTITY]);
}

#[test]
fn the_import_resolves_the_same_from_any_depth() {
    let artifact = build(
        &[
            (
                "main.nx",
                "import \"@nx/agent\"\nimport \"./app/flows/support.nx\"\nlet root(): Agent = { support }",
            ),
            (
                "app/flows/support.nx",
                "import \"@nx/agent\"\nexport let support: Agent = <Agent name=\"support\">Be brief.</Agent>",
            ),
        ],
        &ProgramBuildContext::empty(),
    );
    assert_eq!(
        library_roots(&artifact),
        [nx_hir::PRELUDE_MODULE_IDENTITY, AGENT_ROOT]
    );
}

#[test]
fn every_import_form_names_the_library() {
    let empty = ProgramBuildContext::empty();
    build(
        &[(
            "main.nx",
            "import \"@nx/agent\" as Ai\nlet root() = { <Ai.Agent name=\"support\">Be brief.</Ai.Agent> }",
        )],
        &empty,
    );
    // Behind an alias the unqualified name stays unresolved.
    assert_one_error(
        &errors(
            &[(
                "main.nx",
                "import \"@nx/agent\" as Ai\nlet root(): Agent = { <Ai.Agent name=\"support\">Be brief.</Ai.Agent> }",
            )],
            &empty,
        ),
        &["Agent"],
    );

    build(
        &[(
            "main.nx",
            "import { Agent, Tool } from \"@nx/agent\"\nlet tools: Tool* = { }\nlet root(): Agent = { <Agent name=\"support\">Be brief.</Agent> }",
        )],
        &empty,
    );
    assert_one_error(
        &errors(
            &[(
                "main.nx",
                "import { Agent, Tool } from \"@nx/agent\"\nlet root(): Document = { <Document title=\"t\">Text.</Document> }",
            )],
            &empty,
        ),
        &["Document"],
    );
}

#[test]
fn a_context_limited_to_one_host_root_still_sees_standard_libraries() {
    let registry = LibraryRegistry::new();
    registry
        .load_library_from_sources(&host_library())
        .unwrap_or_else(|diagnostics| panic!("host library: {diagnostics:?}"));
    let context = registry
        .build_context_with_visible_roots([HOST_ROOT])
        .expect("visible roots");

    let artifact = build(
        &[(
            "main.nx",
            "import \"@nx/agent\"\nlet root() = { <Agent name=\"support\">Be brief.</Agent> }",
        )],
        &context,
    );
    assert_eq!(root_record(&artifact).0, "Agent");
}

#[test]
fn a_directory_library_and_an_in_memory_library_import_a_standard_library_alike() {
    let temp = TempDir::new().expect("temp dir");
    let library_dir = temp.path().join("chat-link");
    fs::create_dir_all(&library_dir).expect("library dir");
    fs::write(library_dir.join("ChatLink.nx"), HOST_SOURCE).expect("library module");

    let from_directory = LibraryRegistry::new();
    let directory_library = from_directory
        .load_library_from_directory(&library_dir)
        .unwrap_or_else(|diagnostics| panic!("directory load: {diagnostics:?}"));
    assert!(directory_library
        .dependency_roots
        .contains(&PathBuf::from(AGENT_ROOT)));

    let from_memory = LibraryRegistry::new();
    let memory_library = from_memory
        .load_library_from_sources(&host_library())
        .unwrap_or_else(|diagnostics| panic!("in-memory load: {diagnostics:?}"));
    // Nothing loaded the agent library: the registry records it as a dependency all the same.
    assert_eq!(
        memory_library.dependency_roots,
        vec![PathBuf::from(AGENT_ROOT)]
    );

    // A program built against each reports the same thing: nothing.
    let body = "let root() = { <AssistantConfig enabled=true /> }";
    let directory_root = fs::canonicalize(&library_dir).expect("canonical library dir");
    let directory = validate_workspace(
        &workspace(&[("main.nx", body)]),
        &from_directory
            .build_context()
            .with_implicit_imports([directory_root
                .to_string_lossy()
                .trim_start_matches('/')
                .to_string()]),
    );
    assert!(directory.is_empty(), "{directory:#?}");
    let memory = validate_workspace(
        &workspace(&[("main.nx", &format!("import \"./{HOST_ROOT}\"\n{body}"))]),
        &from_memory.build_context(),
    );
    assert!(memory.is_empty(), "{memory:#?}");
}

#[test]
fn an_unknown_standard_library_is_diagnosed_in_every_import_form() {
    let empty = ProgramBuildContext::empty();
    for import in [
        "import \"@nx/automation\"",
        "import \"@nx/agent/agent.nx\"",
        "import \"@nx/automation\" as Auto",
        "import { Agent } from \"@nx/automation\"",
    ] {
        let diagnostics = errors(
            &[("app/main.nx", &format!("{import}\nlet root() = 1"))],
            &empty,
        );
        let unknown = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code.as_deref() == Some("unknown-standard-library"))
            .unwrap_or_else(|| panic!("{import}: {diagnostics:#?}"));
        let path = import.split('"').nth(1).expect("the import's path");
        assert!(unknown.message.contains(path), "{unknown:?}");
        assert!(unknown.message.contains(AGENT_ROOT), "{unknown:?}");
        assert_eq!(unknown.labels[0].file, "app/main.nx");
        assert_eq!(unknown.labels[0].span.start_line, 1);
        assert_eq!(diagnostics.len(), 1, "{import}: {diagnostics:#?}");
    }

    // The same from a module of a library a host loads.
    let diagnostics = LibraryRegistry::new()
        .load_library_from_sources(&NxLibrarySource::new(
            "libraries/bad",
            vec![NxLibraryModule::new(
                "Bad.nx",
                "import \"@nx/nope\"\nexport type Bad = { id:string }",
            )],
        ))
        .expect_err("the import names no standard library");
    assert!(
        has_code(&diagnostics, "unknown-standard-library"),
        "{diagnostics:#?}"
    );
}

#[test]
fn a_relative_path_into_the_reserved_root_is_refused_not_resolved() {
    let empty = ProgramBuildContext::empty();
    for (identity, import) in [
        ("main.nx", "./@nx/agent"),
        ("app/main.nx", "../@nx/agent"),
        ("app/main.nx", "../@nx/prelude.nx"),
    ] {
        let diagnostics = errors(
            &[(
                identity,
                &format!("import \"{import}\"\nlet root(): Agent = {{ <Agent name=\"support\">Hi.</Agent> }}"),
            )],
            &empty,
        );
        // The import binds nothing; like any import that failed, it is the one report, not each
        // use of a name it would have provided.
        assert_eq!(diagnostics.len(), 1, "{import}: {diagnostics:#?}");
        let reserved = &diagnostics[0];
        assert_eq!(reserved.code.as_deref(), Some("import-path-reserved"));
        assert!(reserved.message.contains(import), "{reserved:?}");
        assert_eq!(reserved.labels[0].file, identity);
    }

    // It is not the standard library by another name: the program links nothing under `@nx/`
    // but the prelude.
    let artifact = build_program_artifact_from_source(
        "import \"./@nx/agent\"\nlet root() = { 1 }",
        "main.nx",
        &empty,
    )
    .expect("program artifact");
    assert_eq!(library_roots(&artifact), [nx_hir::PRELUDE_MODULE_IDENTITY]);

    // The same from a module of a library a host loads from memory.
    let diagnostics = LibraryRegistry::new()
        .load_library_from_sources(&NxLibrarySource::new(
            "libraries/sneaky",
            vec![NxLibraryModule::new(
                "Sneaky.nx",
                "import \"../../@nx/agent\"\nexport type Sneaky = { tool:Tool }",
            )],
        ))
        .expect_err("a relative path into the reserved root is refused");
    assert!(
        has_code(&diagnostics, "import-path-reserved"),
        "{diagnostics:#?}"
    );
}

#[test]
fn a_standard_library_can_be_an_implicit_import() {
    let context = ProgramBuildContext::empty().with_implicit_imports([AGENT_ROOT]);
    let artifact = build(
        &[
            (
                "app/main.nx",
                "import \"./tools.nx\"\nlet root() = { <Agent name=\"support\" tools={tools}>Be brief.</Agent> }",
            ),
            (
                "app/tools.nx",
                "export let tools: Tool+ = { <WebSearchTool /> }",
            ),
        ],
        &context,
    );
    assert!(library_roots(&artifact).contains(&AGENT_ROOT.to_string()));

    // A module that writes the very import it would be given is not importing it twice.
    let artifact = build(
        &[(
            "main.nx",
            "import \"@nx/agent\"\nlet root() = { <Agent name=\"support\">Be brief.</Agent> }",
        )],
        &context,
    );
    assert_eq!(root_record(&artifact).0, "Agent");
}

#[test]
fn an_implicit_import_of_an_unknown_standard_library_is_refused() {
    let context = ProgramBuildContext::empty().with_implicit_imports(["@nx/nope"]);
    let diagnostics = errors(&[("main.nx", "let root() = 1")], &context);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        diagnostics[0].code.as_deref(),
        Some("unknown-standard-library")
    );
    assert!(diagnostics[0].message.contains("@nx/nope"));
    assert!(diagnostics[0].message.contains(AGENT_ROOT));
    assert!(diagnostics[0].labels.is_empty());
}

#[test]
fn the_reserved_root_refuses_workspace_modules_and_host_libraries() {
    for identity in [
        "@nx/agent/agent.nx",
        "@nx/other.nx",
        nx_hir::PRELUDE_MODULE_IDENTITY,
    ] {
        let diagnostics = validate_workspace(
            &workspace(&[(identity, "export type Tool = { label:string }")]),
            &ProgramBuildContext::empty(),
        );
        assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
        assert!(
            diagnostics[0].message.contains(identity)
                && diagnostics[0].message.contains("reserved"),
            "{diagnostics:#?}"
        );
    }

    let registry = LibraryRegistry::new();
    for root in ["@nx/agent", "@nx/mine"] {
        let diagnostics = registry
            .load_library_from_sources(&NxLibrarySource::new(
                root,
                vec![NxLibraryModule::new(
                    "Mine.nx",
                    "export type Mine = { id:string }",
                )],
            ))
            .expect_err("the root is reserved");
        assert!(
            has_code(&diagnostics, "library-root-reserved"),
            "{diagnostics:#?}"
        );
        assert!(diagnostics[0].message.contains(root), "{diagnostics:#?}");
    }
    assert!(
        registry.build_context().visible_libraries().is_empty(),
        "nothing is retained for a refused root"
    );

    let diagnostics = registry
        .load_library_from_directory("@nx/mine")
        .expect_err("a directory under the reserved root is refused");
    assert!(
        has_code(&diagnostics, "library-root-reserved"),
        "{diagnostics:#?}"
    );
}

#[test]
fn a_program_reaches_a_standard_library_through_a_host_library() {
    let registry = LibraryRegistry::new();
    registry
        .load_library_from_sources(&host_library())
        .unwrap_or_else(|diagnostics| panic!("host library: {diagnostics:?}"));

    let artifact = build(
        &[(
            "main.nx",
            &format!(
                "import \"./{HOST_ROOT}\"\nlet root() = {{ <AssistantConfig enabled=true /> }}"
            ),
        )],
        &registry.build_context(),
    );
    assert_eq!(
        library_roots(&artifact),
        [nx_hir::PRELUDE_MODULE_IDENTITY, AGENT_ROOT, HOST_ROOT]
    );
    assert_eq!(root_record(&artifact).0, "AssistantConfig");
}

#[test]
fn a_standard_library_import_follows_the_duplicate_and_ambiguity_rules() {
    let duplicate = agent_errors("import { Agent } from \"@nx/agent\"\nlet root() = 1");
    assert_one_error(&duplicate, &[AGENT_ROOT, "imported more than once"]);

    // A module's own `Tool` beside the wildcard import is its own: nothing is reported until a use
    // disagrees with it.
    let local = "type Tool = { label:string }\n";
    assert!(agent_errors(&format!("{local}let root() = {{ <Tool label=\"x\" /> }}")).is_empty());
    assert!(!agent_errors(&format!(
        "{local}let root() = {{ <Agent name=\"a\" tools={{ <Tool label=\"x\" /> }}>Hi.</Agent> }}"
    ))
    .is_empty());
}

#[test]
fn a_label_inside_a_standard_library_renders_the_library_source() {
    use nx_diagnostics::{Diagnostic, Label, TextSize, TextSpan};

    let source = standard_library_entry(AGENT_ROOT).expect("agent").modules[0].source;
    let offset = source
        .find("name: string\n  /// What the agent is for")
        .expect("Agent.name");
    let span = TextSpan::new(
        TextSize::from(u32::try_from(offset).expect("offset fits")),
        TextSize::from(u32::try_from(offset + "name".len()).expect("offset fits")),
    );
    let diagnostic = Diagnostic::error("test")
        .with_message("points at the library's own declaration")
        .with_label(Label::secondary(AGENT_MODULE, span))
        .build();

    let artifact = build(
        &[("main.nx", "import \"@nx/agent\"\nlet root(): Tool* = { }")],
        &ProgramBuildContext::empty(),
    );
    let entries = artifact.source_entries();
    let rendered = crate::diagnostics_to_api_with_source_entries(
        &[diagnostic],
        "",
        entries.iter().map(|entry| (entry.identity, entry.source)),
    );
    let label = &rendered[0].labels[0];
    assert_eq!(label.file, AGENT_MODULE);
    let line = source
        .lines()
        .nth(usize::try_from(label.span.start_line - 1).expect("line fits"))
        .expect("the label's line");
    assert_eq!(line.trim(), "name: string");
}

// --- Versions ---------------------------------------------------------------------------------

static FIXTURE_MODULES: [StandardLibraryModule; 2] = [
    StandardLibraryModule {
        identity: "a.nx",
        source: "export type A = { id:string }\n",
    },
    StandardLibraryModule {
        identity: "b.nx",
        source: "export type B = { a:A }\n",
    },
];

static FIXTURE_MODULES_EDITED: [StandardLibraryModule; 2] = [
    StandardLibraryModule {
        identity: "a.nx",
        source: "// A comment.\nexport type A = { id:string }\n",
    },
    StandardLibraryModule {
        identity: "b.nx",
        source: "export type B = { a:A }\n",
    },
];

fn fixture_library(modules: &'static [StandardLibraryModule]) -> StandardLibrary {
    StandardLibrary::new(
        "@nx/fixture",
        StandardLibraryStability::Unstable,
        modules,
        "@nx-lang/fixture",
        "NxLang.Fixture",
    )
}

#[test]
fn the_version_is_a_pinned_hash_of_identities_and_sources() {
    // FNV-1a 64 over `a.nx`, 0, source, 0, `b.nx`, 0, source, 0. Pinned so that every platform and
    // every binding built from these bytes is held to the same digits.
    assert_eq!(
        fixture_library(&FIXTURE_MODULES).version(),
        "8ceb11855a7d5425"
    );
    assert_ne!(
        fixture_library(&FIXTURE_MODULES).version(),
        fixture_library(&FIXTURE_MODULES_EDITED).version(),
        "a comment edit changes the version"
    );
}

#[test]
fn the_shipped_agent_library_version_is_pinned() {
    // The version is a hash of the embedded bytes. Update it together with any edit to
    // `std/agent/agent.nx`, and list the edit under `@nx/agent` in the release notes; a change
    // here with no such edit means the embedded bytes moved, a line-ending change included.
    let agent = standard_library_entry(AGENT_ROOT).expect("the agent library");
    assert!(!agent.modules[0].source.contains('\r'));
    assert_eq!(agent.version(), "b06c00c30be50ada");
    assert_eq!(
        standard_library(AGENT_ROOT)
            .expect("agent")
            .version
            .as_deref(),
        Some(agent.version().as_str())
    );
}

#[test]
fn a_program_fingerprint_depends_on_the_standard_library_source() {
    // The program's fingerprint hashes each linked library's, so it is enough that two builds of
    // one library from different source differ there.
    let original = build_standard_library(&fixture_library(&FIXTURE_MODULES));
    let edited = build_standard_library(&fixture_library(&FIXTURE_MODULES_EDITED));
    assert!(
        original.diagnostics.is_empty(),
        "{:#?}",
        original.diagnostics
    );
    assert_ne!(original.fingerprint, edited.fingerprint);
    assert_ne!(original.version, edited.version);
}

// --- The agent library ------------------------------------------------------------------------

#[test]
fn the_agent_library_exports_exactly_its_thirteen_types() {
    let library = standard_library(AGENT_ROOT).expect("the agent library");
    let mut exported = library
        .interface_items
        .iter()
        .filter(|item| item.visibility == Visibility::Export)
        .map(|item| item.item_name.as_str())
        .filter(|name| !name.contains('.'))
        .collect::<Vec<_>>();
    exported.sort();
    assert_eq!(exported, AGENT_TYPES);
    assert!(
        library
            .interface_items
            .iter()
            .all(|item| !matches!(item.item, InterfaceItemKind::Function { .. })),
        "the library holds types only"
    );
}

#[test]
fn every_agent_type_and_field_is_documented() {
    let library = standard_library(AGENT_ROOT).expect("the agent library");
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
            // The cases of `HttpMethod` are the method names themselves.
            InterfaceItemKind::Union { .. } => {}
            other => panic!("unexpected declaration in the agent library: {other:?}"),
        }
    }
    assert!(undocumented.is_empty(), "undocumented: {undocumented:?}");
}

#[test]
fn an_agent_requires_its_instructions() {
    assert_one_error(
        &agent_errors("let root() = { <Agent name=\"support\" /> }"),
        &["instructions"],
    );
    assert!(agent_errors(
        "let root() = { <Agent name=\"support\" instructions=\"Be brief.\" model=\"anything-at-all\" /> }"
    )
    .is_empty());
}

#[test]
fn the_abstract_types_cannot_be_constructed() {
    assert_one_error(
        &agent_errors("let root() = { <Tool name=\"x\" /> }"),
        &["Tool", "abstract"],
    );
}

#[test]
fn a_function_tool_takes_only_a_function() {
    let function = "let findPlans(teamSize:int): string* = { \"Team\" }\n";
    assert!(agent_errors(&format!(
        "{function}let root(): Tool = {{ <FunctionTool function={{findPlans}} /> }}"
    ))
    .is_empty());
    assert_one_error(
        &agent_errors(&format!(
            "{function}let root(): Tool = {{ <FunctionTool function=\"findPlans\" /> }}"
        )),
        &["<function ... />: object*", "string"],
    );
    // NX source cannot forge the record a host later calls.
    assert_one_error(
        &agent_errors(&format!(
            "type Function = {{ module:string name:string }}\n{function}let root(): Tool = {{ <FunctionTool function={{ <Function module=\"main.nx\" name=\"findPlans\" /> }} /> }}"
        )),
        &["<function ... />: object*", "Function"],
    );
}

#[test]
fn an_http_tool_checks_its_arguments_function_and_method() {
    let prelude = "let shop = <HttpConnection name=\"shop\" baseUrl=\"https://api.example.com\" />\n\
        let findPlans(teamSize:int): string* = { \"Team\" }\n\
        let lookupOrder(orderId:string): HttpArguments = <HttpArguments pathParams={ <HttpParam name=\"orderId\" value={orderId} /> } />\n";
    let tool = |method: &str, arguments: &str| {
        agent_errors(&format!(
            "{prelude}let root(): Tool = {{ <HttpTool connection={{shop}} method={{{method}}} path=\"/orders/{{orderId}}\" arguments={{{arguments}}} /> }}"
        ))
    };
    assert!(tool("HttpMethod.get", "lookupOrder").is_empty());
    assert_one_error(
        &tool("HttpMethod.get", "findPlans"),
        &["string*", "HttpArguments"],
    );
    assert_one_error(&tool("HttpMethod.head", "lookupOrder"), &["head"]);

    assert_one_error(
        &agent_errors(
            "let root() = { <HttpConnection name=\"shop\" baseUrl=\"https://api.example.com\" auth=\"token\" /> }",
        ),
        &["auth"],
    );
    // `HttpArguments` is not generic: it takes no type argument and holds any body.
    assert!(agent_errors(
        "type NewTicket = { subject:string }\nlet root(): HttpArguments = { <HttpArguments body={ <NewTicket subject=\"s\" /> } /> }"
    )
    .is_empty());
}

#[test]
fn a_host_library_extends_the_agent_types_and_a_program_uses_them() {
    let registry = LibraryRegistry::new();
    registry
        .load_library_from_sources(&host_library())
        .unwrap_or_else(|diagnostics| panic!("host library: {diagnostics:?}"));
    let context = registry
        .build_context()
        .with_implicit_imports([HOST_ROOT, AGENT_ROOT]);

    let diagnostics = validate_workspace(
        &workspace(&[(
            "main.nx",
            r#"let lookup(orderId:string, context:ChatToolContext): string = { context.conversationId }
let tools: Tool+ = {
  <RecordSearchTool recordKind="company" />
  <FunctionTool function={lookup} />
}
let agent = <Agent name="support" tools={tools}>Be brief.</Agent>
let root() = { <AgentStep id="a" agent={agent}>Ask for the order number.</AgentStep> }
"#,
        )]),
        &context,
    );
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn the_worked_example_type_checks_with_the_library_as_an_implicit_import() {
    let diagnostics = validate_workspace(
        &workspace(&[("main.nx", SUPPORT_AGENT)]),
        &ProgramBuildContext::empty().with_implicit_imports([AGENT_ROOT]),
    );
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

// --- The reference page -----------------------------------------------------------------------

const REFERENCE_PAGE: &str =
    include_str!("../../../sites/website/src/content/docs/reference/libraries/agent.md");

/// The body of the first `nx` code block after `heading` on the reference page, with LF line
/// endings.
///
/// <para>The `.nx` sources are pinned to LF, but a Markdown page checks out with the platform's
/// line endings, so the page is normalized before it is compared with them.</para>
fn reference_block(heading: &str) -> String {
    let page = REFERENCE_PAGE.replace("\r\n", "\n");
    let section = &page[page
        .find(heading)
        .unwrap_or_else(|| panic!("the page has a '{heading}' section"))..];
    let start = section.find("```nx\n").expect("an nx block") + "```nx\n".len();
    let end = section[start..].find("```").expect("the block closes") + start;
    section[start..end].to_string()
}

/// The page quotes the library and its worked example; they are the published source of an
/// unstable library, so a change to either has to reach the page too.
#[test]
fn the_reference_page_quotes_the_library_and_the_worked_example_exactly() {
    let library = standard_library_entry(AGENT_ROOT).expect("agent").modules[0].source;
    assert_eq!(
        reference_block("## The library source"),
        library,
        "update the library source on the reference page"
    );
    assert_eq!(
        reference_block("## A worked example").strip_prefix("import \"@nx/agent\"\n\n"),
        Some(SUPPORT_AGENT),
        "update the worked example on the reference page"
    );
}
