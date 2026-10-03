//! The Rust IR runtime against NX source: each test compiles source, emits IR and runs it on
//! `nx-ir-runtime`. The scenario tests state the expected value; the differential tests compare
//! with the interpreter's evaluation of the same source.

use crate::emit_nx_ir;
use crate::ir::NxIrEmitOptions;
use crate::ir_runtime_sources::SOURCES;
use nx_api::{
    build_program_artifact_from_source, build_workspace_program_artifact,
    dispatch_component_actions_program_artifact, eval_program_artifact_function,
    evaluate_component_program_artifact, initialize_component_program_artifact,
    ComponentDispatchEvalResult, ComponentEvaluateEvalResult, ComponentInitEvalResult, EvalResult,
    LibraryRegistry, NxLibraryModule, NxLibrarySource, NxWorkspace, NxWorkspaceModule,
    ProgramArtifact, ProgramBuildContext,
};
use nx_ir_runtime::{
    apply, diff, merge, ComponentInit, LinkOptions, NxIrRuntimeError, PreparedModule, Program,
    RuntimeOptions,
};
use nx_value::NxValue;
use std::collections::BTreeMap;

fn artifact(source: &str) -> ProgramArtifact {
    build_program_artifact_from_source(source, "main.nx", &ProgramBuildContext::empty())
        .expect("program artifact")
}

/// Emits every module of the program and links the entry against the rest.
fn link(artifact: &ProgramArtifact, debug: bool) -> Program {
    let images = emit_nx_ir(
        artifact,
        &NxIrEmitOptions {
            modules: Some(Vec::new()),
            debug,
        },
    )
    .expect("nx ir");
    let modules: BTreeMap<String, PreparedModule> = images
        .into_iter()
        .map(|image| {
            let module = PreparedModule::prepare(image.bytes)
                .unwrap_or_else(|error| panic!("{}: {error}", image.identity));
            (image.identity, module)
        })
        .collect();
    let entry = modules
        .values()
        .find(|module| module.identity() == "main.nx")
        .expect("the entry");
    Program::link(
        entry,
        |identity| modules.get(identity).cloned(),
        &LinkOptions::default(),
    )
    .expect("links")
}

fn program(source: &str) -> Program {
    link(&artifact(source), true)
}

fn options() -> RuntimeOptions {
    RuntimeOptions::default()
}

fn json(text: &str) -> NxValue {
    NxValue::from_json_str(text).unwrap_or_else(|error| panic!("{text}: {error}"))
}

fn fields(text: &str) -> BTreeMap<String, NxValue> {
    match json(text) {
        NxValue::Record { properties, .. } => properties,
        other => panic!("expected an object, got {other:?}"),
    }
}

/// Canonical equality: numbers by value, records by their fields.
fn same(left: &NxValue, right: &NxValue) -> bool {
    let number = |value: &NxValue| match value {
        NxValue::Int32(value) => Some(f64::from(*value)),
        NxValue::Int(value) => Some(*value as f64),
        NxValue::Float32(value) => Some(f64::from(*value)),
        NxValue::Float(value) => Some(*value),
        _ => None,
    };
    if let (NxValue::Int(left), NxValue::Int(right)) = (left, right) {
        return left == right;
    }
    if let (Some(left), Some(right)) = (number(left), number(right)) {
        return left == right || (left.is_nan() && right.is_nan());
    }
    match (left, right) {
        (NxValue::Array(left), NxValue::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| same(left, right))
        }
        (
            NxValue::Record {
                type_name: left_type,
                properties: left,
            },
            NxValue::Record {
                type_name: right_type,
                properties: right,
            },
        ) => {
            left_type == right_type
                && left.len() == right.len()
                && left
                    .iter()
                    .all(|(key, value)| right.get(key).is_some_and(|other| same(value, other)))
        }
        _ => left == right,
    }
}

#[track_caller]
fn assert_same(actual: &NxValue, expected: &str) {
    let expected = json(expected);
    assert!(
        same(actual, &expected),
        "expected {}, got {}",
        expected.to_json_string().unwrap(),
        actual.to_json_string().unwrap()
    );
}

#[track_caller]
fn failure<T>(result: Result<T, NxIrRuntimeError>) -> NxIrRuntimeError {
    match result {
        Ok(_) => panic!("expected a failure"),
        Err(error) => error,
    }
}

#[track_caller]
fn assert_code<T>(result: Result<T, NxIrRuntimeError>, code: &str, names: &[&str]) {
    let error = failure(result);
    assert_eq!(error.code(), code, "{error}");
    for name in names {
        assert!(
            error.to_string().contains(name),
            "'{error}' does not name '{name}'"
        );
    }
}

fn at<'a>(value: &'a NxValue, path: &[&str]) -> &'a NxValue {
    path.iter().fold(value, |value, step| match value {
        NxValue::Record { properties, .. } => properties
            .get(*step)
            .unwrap_or_else(|| panic!("no '{step}' in {value:?}")),
        NxValue::Array(items) => &items[step.parse::<usize>().expect("an index")],
        other => panic!("cannot read '{step}' of {other:?}"),
    })
}

// ------------------------------------------------------------------------------------------------
// Functions
// ------------------------------------------------------------------------------------------------

#[test]
fn a_root_function_evaluates() {
    let program = program("let root() = { 1 + 2 }");
    assert_eq!(
        program.evaluate_function("root", &[], &options()),
        Ok(NxValue::Int(3))
    );
}

#[test]
fn a_declared_result_is_normalized_and_an_empty_optional_result_is_null() {
    let program = program(
        "let many(): int+ = { 5 }
         let none(): int? = { if false { 1 } }
         let caller(): int* = { none() }",
    );
    assert_same(
        &program.evaluate_function("many", &[], &options()).unwrap(),
        "[5]",
    );
    assert_eq!(
        program.evaluate_function("none", &[], &options()),
        Ok(NxValue::Null)
    );
    assert_same(
        &program
            .evaluate_function("caller", &[], &options())
            .unwrap(),
        "[]",
    );
}

#[test]
fn a_cross_module_call_resolves_through_the_link() {
    let modules = [
        (
            "main.nx",
            "import { answer } from \"./model.nx\"\nlet root(): int = { answer() + 1 }",
        ),
        ("model.nx", "export let answer(): int = { 41 }"),
    ]
    .into_iter()
    .map(|(identity, source)| NxWorkspaceModule::from_source(identity, source).expect("module"))
    .collect();
    let workspace = NxWorkspace::new(modules).expect("workspace");
    let artifact =
        build_workspace_program_artifact(&workspace, "main.nx", &ProgramBuildContext::empty())
            .unwrap_or_else(|diagnostics| panic!("{diagnostics:?}"));
    let program = link(&artifact, false);
    assert_eq!(
        program.modules().collect::<Vec<_>>(),
        ["main.nx", "model.nx"]
    );
    assert_eq!(
        program.evaluate_function("root", &[], &options()),
        Ok(NxValue::Int(42))
    );
}

#[test]
fn either_integer_width_is_accepted_and_arguments_are_checked() {
    let program =
        program("let double(n:int): int = { n * 2 }\nlet half(x:float64): float64 = { x / 2.0 }");
    assert_eq!(
        program.evaluate_function("double", &[NxValue::Int32(3)], &options()),
        Ok(NxValue::Int(6))
    );
    assert_eq!(
        program.evaluate_function("double", &[NxValue::Int(3)], &options()),
        Ok(NxValue::Int(6))
    );
    assert_eq!(
        program.evaluate_function("half", &[NxValue::Int32(3)], &options()),
        Ok(NxValue::Float(1.5))
    );
    assert_eq!(
        program.evaluate_function("half", &[NxValue::Float32(3.0)], &options()),
        Ok(NxValue::Float(1.5))
    );

    assert_code(
        program.evaluate_function("double", &[], &options()),
        "nx-ir-arguments",
        &["n"],
    );
    assert_code(
        program.evaluate_function("double", &[NxValue::Int(1), NxValue::Int(2)], &options()),
        "nx-ir-arguments",
        &["double"],
    );
    assert_code(
        program.evaluate_function("double", &[NxValue::String("x".into())], &options()),
        "nx-ir-boundary-type",
        &["n"],
    );
    assert_code(
        program.evaluate_function("missing", &[], &options()),
        "nx-ir-missing-entrypoint",
        &["missing"],
    );
}

#[test]
fn integers_past_the_float_safe_range_compute_as_wrapping_int64() {
    let program = program(
        "let square(n:int): int = { n * n }
         let next(n:int): int = { n + 1 }
         let half(n:int): float64 = { n / 2 }
         let more(n:int): boolean = { n > 1 }
         let wide(): int = { 9007199254740993 }",
    );
    let call = |name: &str, n: i64| program.evaluate_function(name, &[NxValue::Int(n)], &options());

    // Exact where JavaScript's numbers are not.
    assert_eq!(
        call("next", 9_007_199_254_740_992),
        Ok(NxValue::Int(9_007_199_254_740_993))
    );
    assert_eq!(call("more", 9_007_199_254_740_993), Ok(NxValue::Bool(true)));
    assert_eq!(
        program.evaluate_function("wide", &[], &options()),
        Ok(NxValue::Int(9_007_199_254_740_993))
    );
    // Division is float division, whatever the operands.
    assert_eq!(
        call("half", 9_007_199_254_740_992),
        Ok(NxValue::Float(4_503_599_627_370_496.0))
    );
    // Past the 64 bits, the result wraps.
    assert_eq!(
        call("square", 3_037_000_500),
        Ok(NxValue::Int(3_037_000_500i64.wrapping_mul(3_037_000_500)))
    );
    assert_eq!(call("next", i64::MAX), Ok(NxValue::Int(i64::MIN)));

    // Canonical JSON spells a wide integer as a wrapper, which an integer site reads.
    let wrapped = json(r#"{ "$type": "nx.int", "value": "9007199254740993" }"#);
    assert_eq!(
        program.evaluate_function("next", &[wrapped], &options()),
        Ok(NxValue::Int(9_007_199_254_740_994))
    );
    assert_code(
        program.evaluate_function(
            "next",
            &[json(r#"{ "$type": "nx.int", "value": "many" }"#)],
            &options(),
        ),
        "nx-ir-boundary-type",
        &["n"],
    );
}

#[test]
fn a_function_record_is_called_by_parameter_name() {
    let program = program(
        "type Contact = { name:string }
         let <Row Item:Contact Index:int />: string = { Item.name + \"#\" + Index }
         let template() = { Row }",
    );
    let row = program
        .evaluate_function("template", &[], &options())
        .unwrap();
    assert_same(
        &row,
        r#"{ "$type": "Function", "module": "main.nx", "name": "Row" }"#,
    );

    let args = fields(r#"{ "Item": { "name": "Ada" }, "Index": 2, "Extra": true }"#);
    assert_eq!(
        program.call_function(&row, &args, &options()),
        Ok(NxValue::String("Ada#2".into()))
    );

    let args = fields(r#"{ "Item": { "name": "Ada" } }"#);
    assert_code(
        program.call_function(&row, &args, &options()),
        "nx-ir-arguments",
        &["Index"],
    );

    let other = json(r#"{ "$type": "Function", "module": "main.nx", "name": "Nope" }"#);
    assert_code(
        program.call_function(&other, &BTreeMap::new(), &options()),
        "nx-ir-function-value",
        &["Nope"],
    );
    assert_code(
        program.call_function(&NxValue::Int(1), &BTreeMap::new(), &options()),
        "nx-ir-function-value",
        &[],
    );
}

#[test]
fn a_subtype_declared_in_a_module_nothing_references_is_accepted_from_a_host() {
    let modules = [
        (
            "main.nx",
            "import \"./base.nx\"\nimport \"./x.nx\"\nlet f(s:Base): string = { \"ok\" }",
        ),
        (
            "base.nx",
            "export abstract type Base = { id:int }\nexport type A extends Base = { a:string }",
        ),
        (
            "x.nx",
            "import \"./base.nx\"\nexport type X extends Base = { x:string }",
        ),
        (
            "y.nx",
            "import \"./base.nx\"\nexport type Y extends Base = { y:string }",
        ),
    ];
    let workspace = NxWorkspace::new(
        modules
            .iter()
            .map(|(identity, source)| {
                NxWorkspaceModule::from_source(*identity, *source).expect("module")
            })
            .collect(),
    )
    .expect("workspace");
    let artifact =
        build_workspace_program_artifact(&workspace, "main.nx", &ProgramBuildContext::empty())
            .expect("program artifact");
    let program = link(&artifact, false);
    let f = json(r#"{ "$type": "Function", "module": "main.nx", "name": "f" }"#);
    for argument in [
        r#"{ "s": { "$type": "X", "id": 1, "x": "q" } }"#,
        r#"{ "s": { "$type": "Y", "id": 1, "y": "q" } }"#,
    ] {
        assert_eq!(
            program.call_function(&f, &fields(argument), &options()),
            Ok(NxValue::String("ok".into())),
            "{argument}"
        );
    }
}

// ------------------------------------------------------------------------------------------------
// The agent library and a host library that extends it
// ------------------------------------------------------------------------------------------------

/// Builds `source` as `main.nx` against a host library that imports `@nx/agent`, with both
/// implicitly imported, and links every emitted module.
fn agent_host_program(host_modules: &[(&str, &str)], source: &str) -> Program {
    let registry = LibraryRegistry::new();
    registry
        .load_library_from_sources(&NxLibrarySource::new(
            "libraries/chat-link",
            host_modules
                .iter()
                .map(|(identity, source)| NxLibraryModule::new(*identity, *source))
                .collect(),
        ))
        .unwrap_or_else(|diagnostics| panic!("host library: {diagnostics:?}"));
    let workspace = NxWorkspace::new(vec![
        NxWorkspaceModule::from_source("main.nx", source).expect("module")
    ])
    .expect("workspace");
    let context = registry
        .build_context()
        .with_implicit_imports(["libraries/chat-link", "@nx/agent"]);
    let artifact = build_workspace_program_artifact(&workspace, "main.nx", &context)
        .unwrap_or_else(|diagnostics| panic!("{diagnostics:?}"));
    link(&artifact, false)
}

/// A `Tool+` field holding a subtype a host library declares. The HIR interpreter rejects this
/// ("expected Tool, got RecordSearchTool") when the subtype and its abstract base are declared in
/// different modules, so the case is held on the IR runtime.
#[test]
fn a_tool_list_holds_a_subtype_declared_in_a_host_library() {
    let program = agent_host_program(
        &[(
            "Tools.nx",
            "import \"@nx/agent\"\n\
             export type RecordSearchTool extends Tool = { recordKind:string maxResults:int = 5 }",
        )],
        "let findPlans(teamSize:int): string* = { \"Team\" }
         let root() = {
           <Agent name=\"support\" tools={
             <WebSearchTool />
             <RecordSearchTool recordKind=\"company\" />
             <FunctionTool function={findPlans} />
           }>Be brief.</Agent>
         }",
    );
    assert_eq!(
        program.modules().collect::<Vec<_>>(),
        [
            "main.nx",
            "@nx/agent/agent.nx",
            "libraries/chat-link/Tools.nx"
        ]
    );
    let agent = program.evaluate_function("root", &[], &options()).unwrap();
    assert_same(
        at(&agent, &["tools"]),
        r#"[
            { "$type": "WebSearchTool" },
            { "$type": "RecordSearchTool", "recordKind": "company", "maxResults": 5 },
            { "$type": "FunctionTool",
              "function": { "$type": "Function", "module": "main.nx", "name": "findPlans" } }
        ]"#,
    );
}

/// A library default that names a value of another module of the library: ReachMe's
/// `agent: Agent = {surveyAgent}`. The HIR interpreter resolves the default in the wrong module's
/// scope ("Undefined variable: surveyAgent"), so the case is held on the IR runtime.
#[test]
fn an_omitted_property_takes_a_library_default_that_names_a_library_agent() {
    let program = agent_host_program(
        &[
            (
                "Agents.nx",
                "import \"@nx/agent\"\n\
                 export let surveyAgent: Agent = <Agent:markdown name=\"survey\">Ask one question at a time.</Agent>",
            ),
            (
                "Steps.nx",
                "import \"@nx/agent\"\n\
                 export abstract external component <FlowStep id:string />\n\
                 export external component <AgentStep extends FlowStep agent: Agent = {surveyAgent} content prompt: string />",
            ),
        ],
        "let root() = { <AgentStep id=\"a\">Ask for the order number.</AgentStep> }",
    );
    let step = program.evaluate_function("root", &[], &options()).unwrap();
    assert_same(
        at(&step, &["agent"]),
        r#"{ "$type": "Agent", "name": "survey", "instructions": "Ask one question at a time." }"#,
    );
    assert_eq!(
        at(&step, &["prompt"]),
        &NxValue::String("Ask for the order number.".into())
    );
}

// ------------------------------------------------------------------------------------------------
// Limits and diagnostics
// ------------------------------------------------------------------------------------------------

#[test]
fn unbounded_recursion_ends_in_a_diagnostic() {
    let program = program("let loop(n:int): int = { loop(n + 1) }\nlet root(): int = { loop(0) }");
    assert_code(
        program.evaluate_function("root", &[], &options()),
        "nx-ir-resource-limit",
        &["call depth 100"],
    );

    // The host's limit is the host's to raise; the native stack is still never exhausted.
    let unlimited = RuntimeOptions {
        max_call_depth: u32::MAX,
        ..options()
    };
    assert_code(
        program.evaluate_function("root", &[], &unlimited),
        "nx-ir-resource-limit",
        &["nesting"],
    );
}

#[test]
fn an_oversized_range_is_refused_before_its_body_runs() {
    let program = program("let root() = { for i in 0..2000000 { i } }");
    assert_code(
        program.evaluate_function("root", &[], &options()),
        "nx-ir-resource-limit",
        &["1000000"],
    );
    let raised = RuntimeOptions {
        max_range_length: 2_000_001,
        ..options()
    };
    match program.evaluate_function("root", &[], &raised) {
        Ok(NxValue::Array(items)) => assert_eq!(items.len(), 2_000_000),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_runtime_failure_names_its_declaration_and_its_span_when_the_image_has_one() {
    let source = "let ratio(n:int): int = { n / 0 }";
    let artifact = artifact(source);
    for debug in [true, false] {
        let error = failure(link(&artifact, debug).evaluate_function(
            "ratio",
            &[NxValue::Int(1)],
            &options(),
        ));
        let diagnostic = &error.diagnostics[0];
        assert_eq!(diagnostic.code, "nx-ir-division-by-zero");
        assert_eq!(diagnostic.declaration.as_deref(), Some("main.nx::ratio"));
        match (&diagnostic.source, debug) {
            (Some(span), true) => {
                assert_eq!(span.identity, "main.nx");
                assert_eq!(&source[span.start as usize..span.end as usize], "n / 0");
            }
            (None, false) => {}
            (span, _) => panic!("debug {debug}: {span:?}"),
        }
    }
}

// ------------------------------------------------------------------------------------------------
// The host boundary
// ------------------------------------------------------------------------------------------------

const LIST_SOURCE: &str = "
abstract type Base = { id:int }
type User extends Base = { name:string }
type Team = { title:string }
external component <Text value:string />
component <Listing subtitle?:string items:string+ owner?:Base /> = {
  <Text value={subtitle ?? \"none\"} />
}";

#[test]
fn host_absence_decodes_by_the_declared_occurrence() {
    let program = program(LIST_SOURCE);
    let init = ComponentInit::default();
    let rendered = program
        .initialize_component(
            "Listing",
            &fields(r#"{ "subtitle": null, "items": "one" }"#),
            &init,
            &options(),
        )
        .unwrap()
        .rendered;
    assert_same(&rendered, r#"{ "$type": "Text", "value": "none" }"#);

    assert_code(
        program.initialize_component(
            "Listing",
            &fields(r#"{ "items": null }"#),
            &init,
            &options(),
        ),
        "nx-ir-boundary-type",
        &["items"],
    );
    assert_code(
        program.initialize_component("Listing", &fields(r#"{ "items": [] }"#), &init, &options()),
        "nx-ir-boundary-type",
        &["items"],
    );
}

#[test]
fn a_host_null_at_an_object_site_is_the_empty_list() {
    // `null` is the empty value before any type is known, and `object` takes any value.
    let program = program("let o(x:object): object = { x }");
    assert_same(
        &program
            .evaluate_function("o", &[NxValue::Null], &options())
            .unwrap(),
        "[]",
    );
    assert_same(
        &program
            .evaluate_function("o", &[json(r#"{ "a": null, "b": [null, 1] }"#)], &options())
            .unwrap(),
        r#"{ "a": [], "b": [[], 1] }"#,
    );
}

#[test]
fn malformed_host_input_is_rejected_with_the_boundary_codes() {
    let program = program(LIST_SOURCE);
    let init = ComponentInit::default();
    let initialize =
        |props: &str| program.initialize_component("Listing", &fields(props), &init, &options());

    assert_code(
        initialize(r#"{ "items": ["a"], "extra": 1 }"#),
        "nx-ir-boundary-field",
        &["extra"],
    );
    assert_code(initialize(r#"{}"#), "nx-ir-boundary-field", &["items"]);
    assert_code(
        initialize(r#"{ "items": ["a"], "owner": { "$type": "Team", "title": "x" } }"#),
        "nx-ir-boundary-type",
        &["Team"],
    );
    assert_code(
        initialize(r#"{ "items": ["a"], "owner": { "id": 1 } }"#),
        "nx-ir-boundary-type",
        &["Base"],
    );
    assert!(initialize(
        r#"{ "items": ["a"], "owner": { "$type": "User", "id": 1, "name": "Ada" } }"#
    )
    .is_ok());
}

// ------------------------------------------------------------------------------------------------
// Components
// ------------------------------------------------------------------------------------------------

const COUNTER_SOURCE: &str = "
external component <Button label?:string emits { Tapped { } } />
external component <TextInput value:string />
action SearchSubmitted = { searchString:string }
action DoSearch = { search:string }
component <Counter /> = {
  state { count:int = 0 note?:string }
  <Button onTapped=<Update count={count + 1} /> />
}
component <SearchBox placeholder:string = \"Find\" emits { SearchSubmitted } /> = {
  state { query:string = {placeholder} }
  <TextInput value={query} />
}
component <Page emits { DoSearch } /> = {
  state { count:int = 0 }
  <SearchBox onSearchSubmitted=<DoSearch search={action.searchString} /> />
}";

fn tap(token: &str) -> NxValue {
    json(&format!(
        r#"{{ "$type": "ActionHandlerInvocation", "token": "{token}", "action": {{ "$type": "Button.Tapped" }} }}"#
    ))
}

#[test]
fn initialization_returns_rendered_output_state_and_an_instance() {
    let program = program(COUNTER_SOURCE);
    let initialized = program
        .initialize_component(
            "Counter",
            &BTreeMap::new(),
            &ComponentInit::default(),
            &options(),
        )
        .unwrap();
    assert_same(
        &initialized.rendered,
        r#"{ "$type": "Button", "onTapped": { "$type": "ActionHandler", "action": "Button.Tapped", "token": "h1-1" } }"#,
    );
    assert_same(
        &NxValue::Record {
            type_name: None,
            properties: initialized.state,
        },
        r#"{ "count": 0 }"#,
    );
    assert_eq!(initialized.instance.component(), "Counter");
    assert_eq!(initialized.instance.generation(), 1);
}

#[test]
fn a_handler_invocation_patches_state_and_patches_compound_within_a_batch() {
    let program = program(COUNTER_SOURCE);
    let initialized = program
        .initialize_component(
            "Counter",
            &BTreeMap::new(),
            &ComponentInit::default(),
            &options(),
        )
        .unwrap();
    let dispatched = program
        .dispatch_component_actions(
            &initialized.instance,
            &[tap("h1-1"), tap("h1-1")],
            &options(),
        )
        .unwrap();
    assert_eq!(dispatched.state.get("count"), Some(&NxValue::Int(2)));
    assert!(dispatched.effects.is_empty());
    assert_eq!(
        at(&dispatched.rendered, &["onTapped", "token"]),
        &NxValue::String("h2-1".into())
    );

    // The instance given is untouched: dispatching against it again starts from its state.
    let again = program
        .dispatch_component_actions(&initialized.instance, &[tap("h1-1")], &options())
        .unwrap();
    assert_eq!(again.state.get("count"), Some(&NxValue::Int(1)));
}

#[test]
fn a_stale_token_is_rejected_and_the_instance_survives() {
    let program = program(COUNTER_SOURCE);
    let instance = program
        .initialize_component(
            "Counter",
            &BTreeMap::new(),
            &ComponentInit::default(),
            &options(),
        )
        .unwrap()
        .instance;
    let next = program
        .dispatch_component_actions(&instance, &[tap("h1-1")], &options())
        .unwrap()
        .instance;

    assert_code(
        program.dispatch_component_actions(&next, &[tap("h2-1"), tap("h1-1")], &options()),
        "nx-ir-handler-token",
        &["h1-1"],
    );
    let after = program
        .dispatch_component_actions(&next, &[tap("h2-1")], &options())
        .unwrap();
    assert_eq!(after.state.get("count"), Some(&NxValue::Int(2)));

    assert_code(
        program.dispatch_component_actions(&next, &[json(r#"{ "$type": "Nope" }"#)], &options()),
        "nx-ir-component-action",
        &["Nope"],
    );
    assert_code(
        program.dispatch_component_actions(&next, &[NxValue::Int(1)], &options()),
        "nx-ir-boundary-type",
        &["dispatch entry 0"],
    );
}

#[test]
fn a_parent_bound_handlers_results_are_effects() {
    let program = program(COUNTER_SOURCE);
    let page = program
        .initialize_component(
            "Page",
            &BTreeMap::new(),
            &ComponentInit::default(),
            &options(),
        )
        .unwrap();
    let NxValue::Record {
        type_name,
        properties,
    } = &page.rendered
    else {
        panic!("{:?}", page.rendered);
    };
    assert_eq!(type_name.as_deref(), Some("SearchBox"));

    // Without the parent an ActionHandler record names nothing.
    assert_code(
        program.initialize_component(
            "SearchBox",
            properties,
            &ComponentInit::default(),
            &options(),
        ),
        "nx-ir-boundary-field",
        &["onSearchSubmitted"],
    );

    let init = ComponentInit {
        parent: Some(&page.instance),
        state: None,
    };
    let child = program
        .initialize_component("SearchBox", properties, &init, &options())
        .unwrap();
    let dispatched = program
        .dispatch_component_actions(
            &child.instance,
            &[json(
                r#"{ "$type": "SearchSubmitted", "searchString": "docs" }"#,
            )],
            &options(),
        )
        .unwrap();
    // The handler the child holds is the very handler the parent rendered.
    assert!(child
        .instance
        .bound_handler_is("onSearchSubmitted", &page.instance, "h1-1"));
    assert!(!child
        .instance
        .bound_handler_is("onSearchSubmitted", &page.instance, "h1-2"));
    assert_eq!(dispatched.effects.len(), 1);
    assert_same(
        &dispatched.effects[0],
        r#"{ "$type": "DoSearch", "search": "docs" }"#,
    );
    assert_eq!(dispatched.state, child.state);

    // A malformed payload fails whether or not a handler is bound.
    assert_code(
        program.dispatch_component_actions(
            &child.instance,
            &[json(r#"{ "$type": "SearchSubmitted" }"#)],
            &options(),
        ),
        "nx-ir-boundary-field",
        &["searchString"],
    );
}

#[test]
fn an_invalid_state_or_patch_is_rejected() {
    let program = program(COUNTER_SOURCE);
    let none = BTreeMap::new();
    assert_code(
        program.evaluate_component(
            "Counter",
            &none,
            &fields(r#"{ "count": 1, "extra": 2 }"#),
            &options(),
        ),
        "nx-ir-boundary-field",
        &["extra"],
    );
    assert_code(
        program.normalize_component_state("Counter", &fields(r#"{}"#), &options()),
        "nx-ir-boundary-field",
        &["count"],
    );
    let current = fields(r#"{ "count": 1, "note": "kept" }"#);
    assert_code(
        program.apply_component_state_patch(
            "Counter",
            &current,
            &json(r#"{ "count": "two" }"#),
            &options(),
        ),
        "nx-ir-boundary-type",
        &["count"],
    );
    assert_code(
        program.apply_component_state_patch(
            "Counter",
            &current,
            &json(r#"{ "nope": 1 }"#),
            &options(),
        ),
        "nx-ir-state-field",
        &["nope"],
    );
    assert_code(
        program.apply_component_state_patch(
            "Counter",
            &current,
            &json(r#"{ "$type": "Page.Update" }"#),
            &options(),
        ),
        "nx-ir-state-patch",
        &["Counter.Update"],
    );

    let next = program
        .apply_component_state_patch(
            "Counter",
            &current,
            &json(r#"{ "$type": "Counter.Update", "count": 5, "note": null }"#),
            &options(),
        )
        .unwrap();
    assert_same(
        &NxValue::Record {
            type_name: None,
            properties: next,
        },
        r#"{ "count": 5 }"#,
    );

    // A supplied state replaces the initial one.
    let init = ComponentInit {
        parent: None,
        state: Some(&current),
    };
    let initialized = program
        .initialize_component("Counter", &none, &init, &options())
        .unwrap();
    assert_eq!(initialized.state, current);
}

#[test]
fn pure_evaluation_and_descriptors_carry_no_tokens() {
    let program = program(COUNTER_SOURCE);
    let rendered = program
        .evaluate_component(
            "Counter",
            &BTreeMap::new(),
            &fields(r#"{ "count": 3 }"#),
            &options(),
        )
        .unwrap();
    assert_same(
        &rendered,
        r#"{ "$type": "Button", "onTapped": { "$type": "ActionHandler", "action": "Button.Tapped" } }"#,
    );

    let descriptor = program
        .construct_component_descriptor(
            "SearchBox",
            &fields(r#"{ "placeholder": "Look" }"#),
            &[],
            &options(),
        )
        .unwrap();
    assert_same(
        &descriptor,
        r#"{ "$type": "SearchBox", "placeholder": "Look" }"#,
    );
    assert_code(
        program.construct_component_descriptor(
            "SearchBox",
            &BTreeMap::new(),
            &[NxValue::Int(1)],
            &options(),
        ),
        "nx-ir-boundary-field",
        &["content"],
    );
}

// ------------------------------------------------------------------------------------------------
// Stored instances
// ------------------------------------------------------------------------------------------------

#[test]
fn a_restored_instance_dispatches_as_the_original_does() {
    let artifact = artifact(COUNTER_SOURCE);
    let first = link(&artifact, false);
    let page = first
        .initialize_component(
            "Page",
            &BTreeMap::new(),
            &ComponentInit::default(),
            &options(),
        )
        .unwrap();
    let NxValue::Record { properties, .. } = &page.rendered else {
        panic!()
    };
    let init = ComponentInit {
        parent: Some(&page.instance),
        state: None,
    };
    let child = first
        .initialize_component("SearchBox", properties, &init, &options())
        .unwrap();
    let counter = first
        .initialize_component(
            "Counter",
            &BTreeMap::new(),
            &ComponentInit::default(),
            &options(),
        )
        .unwrap();

    // A later process: the same images, prepared and linked again.
    let second = link(&artifact, false);
    let stored_counter = serde_json::to_string(&counter.instance).unwrap();
    let restored = second
        .restore_component_instance(&mut serde_json::Deserializer::from_str(&stored_counter))
        .unwrap();
    let original = first
        .dispatch_component_actions(&counter.instance, &[tap("h1-1")], &options())
        .unwrap();
    let replayed = second
        .dispatch_component_actions(&restored, &[tap("h1-1")], &options())
        .unwrap();
    assert_eq!(replayed.rendered, original.rendered);
    assert_eq!(replayed.effects, original.effects);
    assert_eq!(replayed.state, original.state);

    // A child's handler from its parent survives storage and still equals the parent's.
    let stored_child = serde_json::to_string(&child.instance).unwrap();
    let stored_page = serde_json::to_string(&page.instance).unwrap();
    let restored_child = second
        .restore_component_instance(&mut serde_json::Deserializer::from_str(&stored_child))
        .unwrap();
    let restored_page = second
        .restore_component_instance(&mut serde_json::Deserializer::from_str(&stored_page))
        .unwrap();
    let submit = [json(
        r#"{ "$type": "SearchSubmitted", "searchString": "docs" }"#,
    )];
    assert_eq!(
        second
            .dispatch_component_actions(&restored_child, &submit, &options())
            .unwrap()
            .effects,
        first
            .dispatch_component_actions(&child.instance, &submit, &options())
            .unwrap()
            .effects
    );
    assert!(restored_page.same_handler("h1-1", &page.instance, "h1-1"));
    assert!(!restored_page.same_handler("h1-1", &counter.instance, "h1-1"));
    let init = ComponentInit {
        parent: Some(&restored_page),
        state: None,
    };
    assert!(second
        .initialize_component("SearchBox", properties, &init, &options())
        .is_ok());
}

#[test]
fn an_instance_of_another_program_revision_is_refused() {
    let first = program(COUNTER_SOURCE);
    let revised = program(&format!("{COUNTER_SOURCE}\n// revised"));
    let instance = first
        .initialize_component(
            "Counter",
            &BTreeMap::new(),
            &ComponentInit::default(),
            &options(),
        )
        .unwrap()
        .instance;

    assert_code(
        revised.dispatch_component_actions(&instance, &[tap("h1-1")], &options()),
        "nx-ir-component",
        &["another program"],
    );
    let init = ComponentInit {
        parent: Some(&instance),
        state: None,
    };
    assert_code(
        revised.initialize_component("Counter", &BTreeMap::new(), &init, &options()),
        "nx-ir-component",
        &["another program"],
    );
    let stored = serde_json::to_string(&instance).unwrap();
    assert_code(
        revised.restore_component_instance(&mut serde_json::Deserializer::from_str(&stored)),
        "nx-ir-component",
        &["another program"],
    );
}

#[test]
fn an_altered_instance_is_refused() {
    let program = program(COUNTER_SOURCE);
    let instance = program
        .initialize_component(
            "Counter",
            &BTreeMap::new(),
            &ComponentInit::default(),
            &options(),
        )
        .unwrap()
        .instance;
    let stored: serde_json::Value = serde_json::to_value(&instance).unwrap();
    let restore = |altered: &serde_json::Value| program.restore_component_instance(altered.clone());
    assert!(restore(&stored).is_ok());

    // The handler: only a handler node of the declaration it names is one.
    let handler = stored["handlers"][0].as_u64().expect("the handler's value") as usize;
    let node = stored["values"][handler]["Handler"]["node"]
        .as_u64()
        .expect("the handler's node");
    for (field, value) in [
        ("node", serde_json::json!(node - 1)),
        ("node", serde_json::json!(4_000_000_000u32)),
        ("module", serde_json::json!("other.nx")),
        ("declaration", serde_json::json!("Page")),
        ("declaration", serde_json::json!("Nope")),
    ] {
        let mut altered = stored.clone();
        altered["values"][handler]["Handler"][field] = value.clone();
        assert_code(restore(&altered), "nx-ir-component", &[]);
    }

    // The values: an entry names only entries before it, and a token names a handler.
    let count = stored["state"][0][1].as_u64().expect("the count's value") as usize;
    let mut altered = stored.clone();
    altered["values"][handler]["Handler"]["captured"][0] = serde_json::json!(handler);
    assert_code(restore(&altered), "nx-ir-component", &["before"]);
    let mut altered = stored.clone();
    altered["handlers"][0] = serde_json::json!(count);
    assert_code(restore(&altered), "nx-ir-component", &["not a handler"]);
    let mut altered = stored.clone();
    altered["values"][count] =
        serde_json::json!({ "Function": { "module": "main.nx", "name": "nope" } });
    assert_code(restore(&altered), "nx-ir-component", &["nope"]);

    // The state, the props and the handler properties: what the component declares.
    let mut altered = stored.clone();
    altered["values"][count] = serde_json::json!({ "Str": "oops" });
    assert_code(
        restore(&altered),
        "nx-ir-component",
        &["Counter state.count"],
    );
    let mut altered = stored.clone();
    altered["state"] = serde_json::json!([]);
    assert_code(restore(&altered), "nx-ir-component", &["count"]);
    let mut altered = stored.clone();
    altered["state"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!(["bogus", count]));
    assert_code(restore(&altered), "nx-ir-component", &["bogus"]);
    let mut altered = stored.clone();
    altered["props"] = serde_json::json!([["bogus", count]]);
    assert_code(restore(&altered), "nx-ir-component", &["bogus"]);
    for property in ["onTapped", "bogus"] {
        let mut altered = stored.clone();
        altered["handler_props"] = serde_json::json!([[property, handler]]);
        assert_code(restore(&altered), "nx-ir-component", &[property]);
    }

    let mut altered = stored.clone();
    altered["component"] = serde_json::json!("Nope");
    assert_code(restore(&altered), "nx-ir-component", &["Nope"]);
    assert_code(
        restore(&serde_json::json!({ "handlers": 3 })),
        "nx-ir-component",
        &[],
    );
    assert_code(
        restore(&serde_json::json!("nonsense")),
        "nx-ir-component",
        &[],
    );
}

#[test]
fn an_instance_is_tied_to_the_images_and_not_only_to_the_source() {
    // The same source emitted twice, with and without its debug section: the fingerprints are
    // equal and the images are not, and a handler names its node by index into the image.
    let artifact = artifact(COUNTER_SOURCE);
    let (with_debug, stripped) = (link(&artifact, true), link(&artifact, false));
    assert_eq!(
        with_debug.entry().fingerprint(),
        stripped.entry().fingerprint()
    );
    let instance = with_debug
        .initialize_component(
            "Counter",
            &BTreeMap::new(),
            &ComponentInit::default(),
            &options(),
        )
        .unwrap()
        .instance;
    assert_code(
        stripped.dispatch_component_actions(&instance, &[tap("h1-1")], &options()),
        "nx-ir-component",
        &["another program"],
    );
    assert_code(
        stripped.restore_component_instance(serde_json::to_value(&instance).unwrap()),
        "nx-ir-component",
        &["another program"],
    );
}

const NESTING_SOURCE: &str = "
external component <Button emits { Tapped { } } />
type Link = { child?:Link }
component <Deep /> = {
  state { tree?:Link }
  <Button onTapped=<Update tree=<Link child={tree} /> /> />
}
component <Holder data:object /> = {
  state { n:int = 0 }
  <Button onTapped=<Update n={n + 1} /> />
}
component <Rows items:string+ /> = {
  state { n:int = 0 }
  for item in items { <Button onTapped=<Update n={n + 1} /> /> }
}";

/// Runs `test` on a thread with the stack a Rust test thread has by default.
fn on_a_small_stack(test: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(test)
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn a_batch_that_nests_state_past_the_limit_fails_at_the_entry_that_crosses_it() {
    on_a_small_stack(|| {
        let program = program(NESTING_SOURCE);
        let instance = program
            .initialize_component(
                "Deep",
                &BTreeMap::new(),
                &ComponentInit::default(),
                &options(),
            )
            .unwrap()
            .instance;
        let batch = |length: usize| vec![tap("h1-1"); length];
        assert!(program
            .dispatch_component_actions(&instance, &batch(50), &options())
            .is_ok());
        for length in [300, 5_000, 200_000] {
            assert_code(
                program.dispatch_component_actions(&instance, &batch(length), &options()),
                "nx-ir-resource-limit",
                &[],
            );
        }
    });
}

#[test]
fn a_stored_instance_writes_a_shared_value_once_and_nests_no_deeper_than_its_table() {
    on_a_small_stack(|| {
        let program = program(NESTING_SOURCE);

        // Every row's handler captures the whole list; the stored form holds one copy of it.
        let size = |rows: usize| {
            let items: Vec<String> = (0..rows).map(|row| format!("row {row}")).collect();
            let props = fields(&serde_json::json!({ "items": items }).to_string());
            let rows = program
                .initialize_component("Rows", &props, &ComponentInit::default(), &options())
                .unwrap();
            let stored = serde_json::to_string(&rows.instance).unwrap();
            assert!(program
                .restore_component_instance(&mut serde_json::Deserializer::from_str(&stored))
                .is_ok());
            stored.len()
        };
        let (small, large) = (size(10), size(1_000));
        assert!(
            large < small * 200,
            "{small} bytes for 10 rows, {large} for 1,000"
        );

        // A value nested as deeply as the boundary admits round-trips through a deserializer
        // that refuses deep input, and dispatches as the original does.
        let mut data = NxValue::Bool(true);
        for _ in 0..125 {
            data = NxValue::Record {
                type_name: None,
                properties: BTreeMap::from([("inner".to_string(), NxValue::Array(vec![data]))]),
            };
        }
        let props = BTreeMap::from([("data".to_string(), data)]);
        let holder = program
            .initialize_component("Holder", &props, &ComponentInit::default(), &options())
            .unwrap();
        let stored = serde_json::to_string(&holder.instance).unwrap();
        let restored = program
            .restore_component_instance(&mut serde_json::Deserializer::from_str(&stored))
            .unwrap();
        let dispatch = |instance| {
            program
                .dispatch_component_actions(instance, &[tap("h1-1")], &options())
                .unwrap()
        };
        let (original, again) = (dispatch(&holder.instance), dispatch(&restored));
        assert_eq!(original.rendered, again.rendered);
        assert_eq!(original.state, again.state);

        // A table that nests deeper than a stored instance may is refused, not rebuilt.
        let mut altered: serde_json::Value = serde_json::from_str(&stored).unwrap();
        let values = altered["values"].as_array_mut().unwrap();
        let first = values.len();
        values.push(serde_json::json!({ "Bool": true }));
        for index in first..first + 2_000 {
            values.push(serde_json::json!({ "Seq": [index] }));
        }
        assert_code(
            program.restore_component_instance(altered),
            "nx-ir-component",
            &["1024"],
        );

        // So is a table that names one entry twice at every level: small, and enormous written
        // out, which is how dispatch would return it.
        let mut altered: serde_json::Value = serde_json::from_str(&stored).unwrap();
        let data = altered["props"][0][1].clone();
        let values = altered["values"].as_array_mut().unwrap();
        let first = values.len();
        values.push(serde_json::json!({ "Bool": true }));
        for index in first..first + 60 {
            values.push(serde_json::json!({ "Seq": [index, index] }));
        }
        assert_ne!(data, serde_json::json!(first + 60));
        altered["props"][0][1] = serde_json::json!(first + 60);
        assert_code(
            program.restore_component_instance(altered),
            "nx-ir-component",
            &["16777216"],
        );
    });
}

// ------------------------------------------------------------------------------------------------
// Update helpers
// ------------------------------------------------------------------------------------------------

#[test]
fn the_update_helpers_follow_the_intrinsics() {
    let user = json(r#"{ "$type": "User", "name": "Ada", "email": "x@y" }"#);
    let applied = apply(&user, &json(r#"{ "$type": "User.Update", "email": null }"#)).unwrap();
    assert_same(&applied, r#"{ "$type": "User", "name": "Ada" }"#);
    assert_code(
        apply(&user, &json(r#"{ "$type": "Team.Update", "name": "x" }"#)),
        "nx-ir-intrinsic",
        &["Team.Update", "User"],
    );

    let merged = merge(
        &json(r#"{ "$type": "User.Update", "name": "Kai", "email": "a@b" }"#),
        &json(r#"{ "$type": "User.Update", "email": null }"#),
    )
    .unwrap();
    assert_eq!(
        merged,
        json(r#"{ "$type": "User.Update", "name": "Kai", "email": null }"#)
    );

    let changed = diff(&user, &json(r#"{ "$type": "User", "name": "Kai" }"#)).unwrap();
    assert_eq!(
        changed,
        json(r#"{ "$type": "User.Update", "name": "Kai", "email": null }"#)
    );

    let program = program(
        "type User = { name:string email?:string age?:int }
         let root(): User.Update = { <User.Update name=\"x\" /> }",
    );
    assert_eq!(
        program.changed(&json(
            r#"{ "$type": "User.Update", "age": 3, "email": null, "name": "x" }"#
        )),
        Ok(vec![
            "name".to_string(),
            "email".to_string(),
            "age".to_string()
        ])
    );
    assert_code(
        program.changed(&json(r#"{ "$type": "Team.Update" }"#)),
        "nx-ir-intrinsic",
        &["Team.Update"],
    );
    assert_code(
        apply(&NxValue::Int(1), &user),
        "nx-ir-intrinsic",
        &["apply"],
    );
}

// ------------------------------------------------------------------------------------------------
// Differential: the interpreter evaluates the same source
// ------------------------------------------------------------------------------------------------

/// Compares the two engines on every entrypoint of a program's entry module: each function
/// called with no arguments, and each component initialized with no props and then evaluated
/// from the state initialization produced. Where both fail the program is one neither runs, and
/// that agreement is all there is to check.
/// The token and the action name of every handler in rendered output.
fn rendered_handlers(value: &NxValue, handlers: &mut Vec<(String, String)>) {
    match value {
        NxValue::Array(items) => items
            .iter()
            .for_each(|item| rendered_handlers(item, handlers)),
        NxValue::Record {
            type_name,
            properties,
        } => match (properties.get("token"), properties.get("action")) {
            (Some(NxValue::String(token)), Some(NxValue::String(action)))
                if type_name.as_deref() == Some("ActionHandler") =>
            {
                handlers.push((token.clone(), action.clone()))
            }
            _ => properties
                .values()
                .for_each(|item| rendered_handlers(item, handlers)),
        },
        _ => {}
    }
}

fn differences(source: &str, artifact: &ProgramArtifact, program: &Program) -> Vec<String> {
    let mut differences = Vec::new();
    let show = |value: &NxValue| value.to_json_string().unwrap_or_default();
    let entry = program.entry();
    let no_props = NxValue::Record {
        type_name: None,
        properties: BTreeMap::new(),
    };
    for name in entry.function_entrypoints() {
        let native = eval_program_artifact_function(artifact, "main.nx", name);
        match (program.evaluate_function(name, &[], &options()), native) {
            (Ok(actual), EvalResult::Ok(expected)) if !same(&actual, &expected) => differences.push(format!(
                "{name}(): the interpreter gives {}, the IR runtime {}\n{source}",
                show(&expected),
                show(&actual)
            )),
            (Ok(actual), EvalResult::Err(diagnostics)) => differences.push(format!(
                "{name}(): the interpreter fails ({diagnostics:?}), the IR runtime gives {}\n{source}",
                show(&actual)
            )),
            (Err(error), EvalResult::Ok(expected)) => differences.push(format!(
                "{name}(): the interpreter gives {}, the IR runtime fails: {error}\n{source}",
                show(&expected)
            )),
            _ => {}
        }
    }
    for name in entry.component_entrypoints() {
        let native = initialize_component_program_artifact(artifact, name, &no_props);
        let initialized = program.initialize_component(
            name,
            &BTreeMap::new(),
            &ComponentInit::default(),
            &options(),
        );
        match (&initialized, native) {
            (Ok(actual), ComponentInitEvalResult::Ok(expected)) if !same(&actual.rendered, &expected.rendered) => {
                differences.push(format!(
                    "<{name}/> initialized: the interpreter renders {}, the IR runtime {}\n{source}",
                    show(&expected.rendered),
                    show(&actual.rendered)
                ))
            }
            (Ok(_), ComponentInitEvalResult::Err(diagnostics)) => differences.push(format!(
                "<{name}/> initialized: the interpreter fails ({diagnostics:?}), the IR runtime does not\n{source}"
            )),
            // The interpreter renders an external component as its own descriptor; the IR
            // runtimes initialize only a component with a body.
            (Err(error), ComponentInitEvalResult::Ok(_)) if error.to_string().contains("has no body") => {}
            (Err(error), ComponentInitEvalResult::Ok(_)) => differences.push(format!(
                "<{name}/> initialized: the IR runtime fails and the interpreter does not: {error}\n{source}"
            )),
            _ => {}
        }
        let Ok(initialized) = initialized else {
            continue;
        };
        // Each handler the body rendered, run once with the bare action it accepts: the engines
        // agree on what it returns and on what the body renders against the state it leaves.
        if let ComponentInitEvalResult::Ok(native) =
            initialize_component_program_artifact(artifact, name, &no_props)
        {
            let mut handlers = Vec::new();
            rendered_handlers(&initialized.rendered, &mut handlers);
            for (token, action) in handlers {
                let batch = [json(&format!(
                    r#"{{ "$type": "ActionHandlerInvocation", "token": "{token}", "action": {{ "$type": "{action}" }} }}"#
                ))];
                let native = dispatch_component_actions_program_artifact(
                    artifact,
                    &native.state_snapshot,
                    &batch,
                );
                let dispatched =
                    program.dispatch_component_actions(&initialized.instance, &batch, &options());
                match (dispatched, native) {
                    (Ok(actual), ComponentDispatchEvalResult::Ok(expected))
                        if !same(&actual.rendered, &expected.rendered)
                            || !same(
                                &NxValue::Array(actual.effects.clone()),
                                &NxValue::Array(expected.effects.clone()),
                            ) =>
                    {
                        differences.push(format!(
                            "<{name}/> dispatched {action}: the interpreter renders {} with effects {}, the IR runtime {} with effects {}\n{source}",
                            show(&expected.rendered),
                            show(&NxValue::Array(expected.effects)),
                            show(&actual.rendered),
                            show(&NxValue::Array(actual.effects))
                        ))
                    }
                    (Ok(_), ComponentDispatchEvalResult::Err(diagnostics)) => differences.push(format!(
                        "<{name}/> dispatched {action}: the interpreter fails ({diagnostics:?}), the IR runtime does not\n{source}"
                    )),
                    (Err(error), ComponentDispatchEvalResult::Ok(_)) => differences.push(format!(
                        "<{name}/> dispatched {action}: the IR runtime fails and the interpreter does not: {error}\n{source}"
                    )),
                    _ => {}
                }
            }
        }
        let state = NxValue::Record {
            type_name: None,
            properties: initialized.state.clone(),
        };
        let native = evaluate_component_program_artifact(artifact, name, &no_props, &state);
        let evaluated =
            program.evaluate_component(name, &BTreeMap::new(), &initialized.state, &options());
        match (evaluated, native) {
            (Ok(actual), ComponentEvaluateEvalResult::Ok(expected)) if !same(&actual, &expected.rendered) => {
                differences.push(format!(
                    "<{name}/> evaluated: the interpreter renders {}, the IR runtime {}\n{source}",
                    show(&expected.rendered),
                    show(&actual)
                ))
            }
            (Ok(_), ComponentEvaluateEvalResult::Err(diagnostics)) => differences.push(format!(
                "<{name}/> evaluated: the interpreter fails ({diagnostics:?}), the IR runtime does not\n{source}"
            )),
            (Err(error), ComponentEvaluateEvalResult::Ok(_)) => differences.push(format!(
                "<{name}/> evaluated: the IR runtime fails and the interpreter does not: {error}\n{source}"
            )),
            _ => {}
        }
    }
    differences
}

/// Every arithmetic and comparison operator at both numeric carriers, so a wrong operator in
/// either engine shows as a difference.
const OPERATOR_SOURCE: &str = "
let sum(): int = { 7 + 5 }
let difference(): int = { 7 - 5 }
let product(): int = { 7 * 5 }
let quotient(): int = { 7 / 5 }
let remainder(): int = { -7 % 5 }
let negated(): int = { -(7 - 9) }
let floatSum(): float64 = { 7.5 + 5.25 }
let floatDifference(): float64 = { 7.5 - 5.25 }
let floatProduct(): float64 = { 7.5 * 5.25 }
let floatQuotient(): float64 = { 7.5 / 2.5 }
let floatRemainder(): float64 = { 7.5 % 2.0 }
let mixed(): float64 = { 7 - 2.5 }
let less(): boolean = { 1 < 2 }
let lessOrEqual(): boolean = { 2 <= 2 }
let greater(): boolean = { 3 > 4 }
let greaterOrEqual(): boolean = { 4 >= 5 }
let equal(): boolean = { 1 == 1 }
let unequal(): boolean = { 1 != 1 }
let both(): boolean = { true && false }
let either(): boolean = { true || false }
let inverted(): boolean = { !true }
let joined(): string = { \"a\" + 1 + \"b\" + 2.5 + true }
";

/// Components whose rendered output shows their state, so a dispatch that patches the state
/// wrongly, or returns the wrong effects, shows as a difference.
const DISPATCH_SOURCE: &str = "
external component <Label text:int />
external component <Button label:string emits { Tapped { } } />
external component <Card items:object+ header:object count:int />
action Reset = { }
component <Tally step:int = 2 emits { Reset } /> = {
  state { count:int = 0 }
  <Card
    items={<Label text={count} />}
    header=<Button label=\"Add\" onTapped=<Update count={count + step} /> />
    count={count} />
}
component <Resetting emits { Reset } /> = {
  state { count:int = 5 }
  <Card
    items={<Label text={count} />}
    header=<Button label=\"Reset\" onTapped={<Update count=0 /> <Reset />} />
    count={count} />
}
";

#[test]
fn emitted_ir_agrees_with_the_interpreter() {
    let mut differences_found = Vec::new();
    let mut compared = 0;
    // The sources written here must compile; one of the shared list may be a program the
    // compiler refuses, which the TypeScript test checks for and this one skips.
    let own = [
        COUNTER_SOURCE,
        LIST_SOURCE,
        OPERATOR_SOURCE,
        DISPATCH_SOURCE,
    ];
    for source in SOURCES.iter().copied().chain(own) {
        let emitted =
            build_program_artifact_from_source(source, "main.nx", &ProgramBuildContext::empty())
                .ok()
                .and_then(|artifact| {
                    Some((
                        emit_nx_ir(&artifact, &NxIrEmitOptions::default()).ok()?,
                        artifact,
                    ))
                });
        let Some((images, artifact)) = emitted else {
            assert!(
                !own.contains(&source),
                "the source does not compile:\n{source}"
            );
            continue;
        };
        let program = Program::prepare(images.into_iter().next().expect("the entry image").bytes)
            .unwrap_or_else(|error| panic!("{error}\n{source}"));
        compared += 1;
        differences_found.extend(differences(source, &artifact, &program));
    }
    assert!(compared > 20, "only {compared} sources compiled");
    assert!(
        differences_found.is_empty(),
        "{}",
        differences_found.join("\n\n")
    );
}
