//! Standard libraries: NX source the compiler carries and a module imports as `@nx/<name>`, and
//! the agent library, the first of them.

use nx_api::{
    build_program_artifact_from_source, build_workspace_program_artifact, eval_program_artifact,
    standard_library, standard_library_entry, validate_workspace, EvalResult, LibraryRegistry,
    NxDiagnostic, NxLibraryModule, NxLibrarySource, NxSeverity, NxWorkspace, NxWorkspaceModule,
    ProgramArtifact, ProgramBuildContext, StandardLibraryStability,
};
use nx_value::NxValue;
use std::sync::Arc;

const AGENT_ROOT: &str = "@nx/agent";
const HOST_ROOT: &str = "libraries/chat-link";

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
fn math_standard_library_signatures_identity_and_imports() {
    let math = standard_library_entry("@nx/math").unwrap();
    assert_eq!(math.stability, StandardLibraryStability::Unstable);
    assert!(
        math.artifact().diagnostics.is_empty(),
        "{:?}",
        math.artifact().diagnostics
    );
    assert!(Arc::ptr_eq(
        math.artifact(),
        standard_library("@nx/math").unwrap()
    ));
    let source = "import { sin, pi } from \"@nx/math\"\nlet root() = { sin(pi / 2.0) }";
    let artifact =
        build_program_artifact_from_source(source, "main.nx", &ProgramBuildContext::empty())
            .unwrap();
    match eval_program_artifact(&artifact) {
        EvalResult::Ok(value) => assert_eq!(value, NxValue::Float(1.0)),
        EvalResult::Err(e) => panic!("{e:?}"),
    }
    for op in nx_value::math::MathOp::ALL {
        let schema = nx_api::program_artifact_function_schema(
            &artifact,
            &nx_api::DeclarationRef::in_module("@nx/math/math.nx", op.name()),
            &nx_api::FunctionSchemaOptions::default(),
        )
        .unwrap();
        let value = serde_json::to_value(schema.input_schema.unwrap()).unwrap();
        for param in op.params() {
            assert_eq!(value["properties"][param]["type"], "number");
        }
        assert_eq!(
            serde_json::to_value(schema.output_schema.unwrap()).unwrap()["type"],
            "number"
        );
        assert!(op.descriptor("@nx/math/math.nx").contains(op.name()));
    }
    let pi = math.artifact().exports.get("pi").unwrap();
    assert_eq!(pi.module_file, "@nx/math/math.nx");
    assert!(library_roots(&artifact).contains(&"@nx/math".to_string()));
    let plain = build_program_artifact_from_source(
        "let root() = { 1 }",
        "main.nx",
        &ProgramBuildContext::empty(),
    )
    .unwrap();
    assert!(!library_roots(&plain).contains(&"@nx/math".to_string()));
    assert!(!errors(
        &[("main.nx", "let root() = { sin(1.0) }")],
        &ProgramBuildContext::empty()
    )
    .is_empty());
    // A host cannot supply any reserved root, including the trusted math identity.
    assert!(LibraryRegistry::new()
        .load_library_from_sources(&NxLibrarySource::new(
            "@nx/math",
            vec![NxLibraryModule::new(
                "math.nx",
                "export let sin(x:float64):float64 = { 0.0 }"
            )]
        ))
        .is_err());
}
