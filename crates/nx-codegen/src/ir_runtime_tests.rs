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
    apply, diff, merge, ComponentInit, Limit, LinkOptions, NxIrRuntimeError, PreparedModule,
    Program, RuntimeOptions,
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

/// The limit a resource-limit failure names.
#[track_caller]
fn limit_of<T>(result: Result<T, NxIrRuntimeError>) -> Limit {
    let error = failure(result);
    assert_eq!(error.code(), "nx-ir-resource-limit", "{error}");
    error.diagnostics[0]
        .limit
        .unwrap_or_else(|| panic!("'{error}' names no limit"))
}

fn limit(name: &'static str, value: u64) -> Limit {
    Limit {
        name,
        value: Some(value),
    }
}

#[test]
fn unbounded_recursion_ends_in_a_diagnostic() {
    let program = program("let loop(n:int): int = { loop(n + 1) }\nlet root(): int = { loop(0) }");
    assert_code(
        program.evaluate_function("root", &[], &options()),
        "nx-ir-resource-limit",
        &["call depth 100"],
    );
    assert_eq!(
        limit_of(program.evaluate_function("root", &[], &options())),
        limit("maxCallDepth", 100)
    );

    // The host's limit is the host's to raise; the native stack is still never exhausted.
    let unlimited = RuntimeOptions {
        max_call_depth: u32::MAX,
        ..options()
    };
    // Whichever bound is met first names itself: an unoptimized build's frames can use the stack
    // up before a thousand nodes nest.
    let reached = limit_of(program.evaluate_function("root", &[], &unlimited));
    assert!(
        reached == limit("maxExpressionNesting", 1000) || reached == limit("maxStackBytes", 1 << 20),
        "{reached:?}"
    );
}

#[test]
fn expressions_nested_past_the_bound_name_the_nesting_limit() {
    // A thousand and one nested negations, each a node inside the last. In an optimized build the
    // count is what stops them; an unoptimized build's frames are large enough that the stack
    // bound is met first, so the bound's own value is pinned in every build by the unit test
    // `expressions_nest_exactly_a_thousand_deep` in `nx-ir-runtime`, which starts the count near
    // it. The compiler recurses over them too, so all of it runs on a big stack.
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            let source = format!(
                "let root(n:int): int = {{ {}n{} }}",
                "-(".repeat(1001),
                ")".repeat(1001)
            );
            let program = program(&source);
            let reached = limit_of(program.evaluate_function("root", &[NxValue::Int(1)], &options()));
            assert!(
                reached == limit("maxExpressionNesting", 1000)
                    || (cfg!(debug_assertions) && reached == limit("maxStackBytes", 1 << 20)),
                "{reached:?}"
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn a_value_nested_too_deeply_names_the_value_nesting_limit() {
    let program = program("let id(x:object): object = { x }");
    let mut value = NxValue::Int(1);
    for _ in 0..300 {
        value = NxValue::Array(vec![value]);
    }
    assert_eq!(
        limit_of(program.evaluate_function("id", &[value], &options())),
        limit("maxValueNesting", 256)
    );
}

#[test]
fn a_diagnostic_that_is_not_a_resource_limit_names_no_limit() {
    let program = program("let ratio(n:int): int = { n / 0 }");
    let error = failure(program.evaluate_function("ratio", &[NxValue::Int(1)], &options()));
    assert_eq!(error.code(), "nx-ir-division-by-zero");
    assert_eq!(error.diagnostics[0].limit, None);
}

#[test]
fn an_oversized_range_is_refused_before_its_body_runs() {
    let program = program("let root() = { for i in 0..2000000 { i } }");
    assert_code(
        program.evaluate_function("root", &[], &options()),
        "nx-ir-resource-limit",
        &["1000000"],
    );
    assert_eq!(
        limit_of(program.evaluate_function("root", &[], &options())),
        limit("maxRangeLength", 1_000_000)
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
// The operation budget
// ------------------------------------------------------------------------------------------------

fn budget(operations: u64) -> RuntimeOptions {
    RuntimeOptions {
        max_operations: Some(operations),
        ..options()
    }
}

/// What an evaluation costs: the least budget it succeeds under, found by doubling the budget and
/// then bisecting.
fn cost<T>(run: impl Fn(&RuntimeOptions) -> Result<T, NxIrRuntimeError>) -> u64 {
    let succeeds = |operations: u64| run(&budget(operations)).is_ok();
    let mut high = 1;
    while !succeeds(high) {
        high *= 2;
    }
    let mut low = 0;
    while low < high {
        let middle = low + (high - low) / 2;
        if succeeds(middle) {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    high
}

/// Asserts that an evaluation costs exactly `operations`: it succeeds under that budget and fails
/// on the budget under one less.
#[track_caller]
fn assert_costs<T: std::fmt::Debug>(
    operations: u64,
    run: impl Fn(&RuntimeOptions) -> Result<T, NxIrRuntimeError>,
) {
    if let Err(error) = run(&budget(operations)) {
        panic!("fails under a budget of {operations}: {error}");
    }
    assert_eq!(
        limit_of(run(&budget(operations - 1))),
        limit("maxOperations", operations - 1)
    );
}

const COST_SOURCE: &str = "
let add(a:int, b:int) = { a + b }
let squares() = { for i in 0..4 { i * i } }
let pick(flag:boolean) = { if flag { 1 } else { for i in 0..1000 { i } } }
let one() = { 1 }
let callOne() = { one() }
let twice(xs:int+) = { xs xs }
let same(xs:int+) = { xs }
let join(a:string, b:string) = { a + b }
let sameText(a:string, b:string) = { a == b }
type Shape = | dot | ring { radius:int } | box { side:int }
let corners(shape:Shape) = { if shape is { dot, ring => 0  box => 4 } }
let triple(n:int) = { n * 3 }
let through(f: <function n:int />: int) = <f n={2} />
let nested() = { for a in 0..1000 { for b in 0..1000 { for c in 0..1000 { a + b + c } } } }
let wide() = { for i in 0..1000000 { i } }
let growList(n:int, xs:int+): int+ = { if n == 0 { xs } else { growList(n - 1, { xs xs }) } }
let growText(n:int, s:string): string = { if n == 0 { s } else { growText(n - 1, s + s) } }
";

fn ints(count: usize) -> NxValue {
    NxValue::Array((0..count as i64).map(NxValue::Int).collect())
}

#[test]
fn an_evaluation_costs_its_nodes_its_items_and_its_text() {
    let program = program(COST_SOURCE);
    let run = |name: &'static str, args: Vec<NxValue>| {
        let program = &program;
        move |options: &RuntimeOptions| program.evaluate_function(name, &args, options)
    };
    // The worked counts of `docs/nx-ir-format.md`: what is evaluated, what is checked against a
    // type on the way in, what is placed, and what is written for the host on the way out.
    //
    // Two arguments checked, three nodes, one number written.
    assert_costs(6, run("add", vec![NxValue::Int(1), NxValue::Int(2)]));
    // The 21 of the loop itself, the three fields of its `Range` checked, and the list and its
    // four numbers written.
    assert_costs(29, run("squares", vec![]));
    // One argument checked, five for the branch taken, and its one-item list and number written.
    assert_costs(8, run("pick", vec![NxValue::Bool(true)]));
    assert_costs(4, run("callOne", vec![]));
    // 500 items checked, 1,003 to build the list, the list and its 1,000 items written.
    assert_costs(2504, run("twice", vec![ints(500)]));
    // A host's list is checked item by item and written item by item, around the one node.
    assert_costs(20_002, run("same", vec![ints(10_000)]));
    // A `concat` of 2,000 code units: two arguments checked, its node and two slots, 31 for the
    // length it builds, and 32 to write the string, one for the value and 31 for its length. A
    // short one pays for no length. `é` is one UTF-16 code unit and two UTF-8 bytes.
    let text = |unit: &str, count: usize| NxValue::String(unit.repeat(count));
    assert_costs(68, run("join", vec![text("a", 1000), text("b", 1000)]));
    assert_costs(68, run("join", vec![text("é", 1000), text("é", 1000)]));
    assert_costs(6, run("join", vec![text("a", 31), text("b", 32)]));
    assert_costs(8, run("join", vec![text("a", 32), text("b", 32)]));
    // An equality of two strings reads them: two arguments checked, its node and two slots, one
    // for the pair and 15 for the 1,000 code units of the shorter, and the boolean written.
    assert_costs(22, run("sameText", vec![text("a", 1000), text("a", 1000)]));
    assert_costs(22, run("sameText", vec![text("a", 1000), text("b", 5000)]));
    assert_costs(7, run("sameText", vec![text("a", 1000), text("a", 63)]));
}

#[test]
fn a_pattern_and_a_callee_cost_one_however_they_are_resolved() {
    let program = program(COST_SOURCE);
    let shape = |text: &str| json(text);
    // `ifIs`, its slot, the literal of the arm taken and the number written are four. The
    // argument is checked: one for a constant case, two for a case and its field. Each pattern
    // tested is one. `dot` is a constant case the runtime evaluates and then compares with the
    // scrutinee, which is one more; `ring` and `box` are cases with fields, which it reads in
    // place and matches by type, comparing nothing.
    for (value, checked, tested) in [
        (r#""dot""#, 1, 2),
        (r#"{ "$type": "Shape.ring", "radius": 1 }"#, 2, 3),
        (r#"{ "$type": "Shape.box", "side": 1 }"#, 2, 4),
    ] {
        let args = vec![shape(value)];
        assert_costs(4 + checked + tested, |options: &RuntimeOptions| {
            program.evaluate_function("corners", &args, options)
        });
    }
    // A function value called by name: the value checked, the named call, its slot callee and
    // the literal argument, then the argument checked and the body of `triple` (the binary node,
    // its slot and its literal), and the number written.
    let triple = json(r#"{ "$type": "Function", "module": "main.nx", "name": "triple" }"#);
    assert_costs(9, |options: &RuntimeOptions| {
        program.evaluate_function("through", std::slice::from_ref(&triple), options)
    });
}

#[test]
fn nested_loops_end_at_the_budget() {
    let program = program(COST_SOURCE);
    let error = failure(program.evaluate_function("nested", &[], &budget(100_000)));
    let diagnostic = &error.diagnostics[0];
    assert_eq!(diagnostic.limit, Some(limit("maxOperations", 100_000)));
    assert_eq!(diagnostic.declaration.as_deref(), Some("main.nx::nested"));
    assert!(diagnostic.source.is_some(), "{diagnostic:?}");
}

#[test]
fn an_absent_budget_is_unlimited() {
    let program = program(COST_SOURCE);
    match program.evaluate_function("wide", &[], &options()) {
        Ok(NxValue::Array(items)) => assert_eq!(items.len(), 1_000_000),
        other => panic!("{other:?}"),
    }
}

/// The source text a diagnostic's span covers.
fn spanned<'a>(error: &NxIrRuntimeError, source: &'a str) -> &'a str {
    let span = error.diagnostics[0]
        .source
        .as_ref()
        .unwrap_or_else(|| panic!("'{error}' carries no span"));
    &source[span.start as usize..span.end as usize]
}

#[test]
fn a_value_that_doubles_on_each_call_is_stopped_at_the_doubling_the_budget_cannot_pay_for() {
    // Sixty doublings would be 2^60 items or code units: without the budget this aborts the
    // process on an allocation failure, inside the default call depth.
    let program = program(COST_SOURCE);
    let list = || NxValue::Array(vec![NxValue::Int(1)]);
    let text = || NxValue::String("x".into());
    let grow = |name: &'static str, doublings: i64, seed: NxValue, operations: u64| {
        program.evaluate_function(name, &[NxValue::Int(doublings), seed], &budget(operations))
    };

    // A list is paid for where it is checked as well as where it is built: `growList` checks its
    // argument on the way in and its declared result on the way out of every call, so a list of
    // 2^k items costs about k times its length to return through k calls. Twelve doublings fit in
    // 100,000 and thirteen do not. Which charge the budget runs out at depends on the doublings
    // asked for, a check of the list or the placing of its items, and either way it is in
    // `growList`. That an item is charged before it is placed is pinned in `nx-ir-runtime`, by
    // the unit test `a_refused_placement_leaves_the_sequence_untouched`.
    match grow("growList", 12, list(), 100_000) {
        Ok(NxValue::Array(items)) => assert_eq!(items.len(), 1 << 12),
        other => panic!("{other:?}"),
    }
    for doublings in [13, 60] {
        let error = failure(grow("growList", doublings, list(), 100_000));
        assert_eq!(
            error.diagnostics[0].limit,
            Some(limit("maxOperations", 100_000))
        );
        assert_eq!(
            error.diagnostics[0].declaration.as_deref(),
            Some("main.nx::growList")
        );
    }

    // A string of 2^j code units costs 2^j / 64 to build, so the doublings through the
    // twenty-first cost 2^16 - 1, and writing the result costs 2^15 more: inside 100,000. The
    // twenty-second would cost 2^16 to build, and is refused at the `concat` that would build it;
    // `tests/allocation.rs` in `nx-ir-runtime` shows that nothing was allocated for it.
    match grow("growText", 21, text(), 100_000) {
        Ok(NxValue::String(built)) => assert_eq!(built.len(), 1 << 21),
        other => panic!("{other:?}"),
    }
    for doublings in [22, 60] {
        let error = failure(grow("growText", doublings, text(), 100_000));
        assert_eq!(
            error.diagnostics[0].limit,
            Some(limit("maxOperations", 100_000))
        );
        assert_eq!(spanned(&error, COST_SOURCE), "s + s");
    }

    // And the count is exact: a doubling whose items the budget covers to the last one runs, and
    // one operation fewer stops it.
    for (name, seed) in [("growList", list()), ("growText", text())] {
        let args = [NxValue::Int(10), seed];
        let run = |options: &RuntimeOptions| program.evaluate_function(name, &args, options);
        assert_costs(cost(run), run);
    }
}

const SHARED_SOURCE: &str = "
external component <Button emits { Tapped { } } />
type Tree = { kids?:Tree+ }
let growTree(n:int, t:Tree): Tree = { if n == 0 { t } else { growTree(n - 1, <Tree kids={ t t } />) } }
let held(n:int): int = { if growTree(n, <Tree />) == <Tree /> { 1 } else { 0 } }
let build(n:int): Tree = { growTree(n, <Tree />) }
let growPair(n:int, t:object): object = { if n == 0 { t } else { growPair(n - 1, <pair a={t} b={t} />) } }
let samePair(n:int): boolean = { growPair(n, <leaf />) == growPair(n, <leaf />) }
let buildPair(n:int): object = { growPair(n, <leaf />) }
let keepPair(n:int): int = { if growPair(n, <leaf />) is { {} => 0  else => 1 } }
let growText(n:int, s:string): string = { if n == 0 { s } else { growText(n - 1, s + s) } }
let repeat(s:string, count:int) = { for i in 0..count { s } }
let fan(n:int, count:int) = { repeat(growText(n, \"x\"), count) }
component <Holder /> = {
  state { view?:object }
  <Button onTapped=<Update view={growPair(40, <leaf />)} /> />
}";

/// A value that holds another value twice is one allocation reached by two paths: forty levels of
/// that are 2^40 values to anything that walks them as a tree, built for a few hundred operations.
/// Every walk the runtime makes is either paid for by the budget or follows the sharing, so each
/// of these ends in a diagnostic at once; without the charges none of them ends at all.
#[test]
fn a_value_shared_many_times_over_is_paid_for_wherever_it_is_walked() {
    let program = program(SHARED_SOURCE);
    let limited = budget(100_000);
    let exhausted = limit("maxOperations", 100_000);
    let levels = [NxValue::Int(40)];

    // Checked against a type: each call of `growTree` checks its argument, value by value.
    assert_eq!(
        limit_of(program.evaluate_function("held", &levels, &limited)),
        exhausted
    );
    assert_eq!(
        limit_of(program.evaluate_function("build", &levels, &limited)),
        exhausted
    );
    // Compared: `object` is checked as one opaque value and an element has no declared fields,
    // so nothing has walked these two values until `==` does, pair by pair.
    assert_eq!(
        limit_of(program.evaluate_function("samePair", &levels, &limited)),
        exhausted
    );
    // Written for the host: nothing walks this one until it is returned.
    let error = failure(program.evaluate_function("buildPair", &levels, &limited));
    assert_eq!(error.diagnostics[0].limit, Some(exhausted));
    assert_eq!(
        error.diagnostics[0].declaration.as_deref(),
        Some("main.nx::buildPair")
    );
    assert_eq!(error.diagnostics[0].source, None);
    // Held and never walked: it costs what building it costs, and nothing more.
    assert!(
        cost(|options| program.evaluate_function("keepPair", &levels, options)) < 1000,
        "a value nobody walks is not paid for by its size"
    );

    // Stored in state: the runtime's own depth check follows the sharing, so the batch gets as
    // far as writing the state for the host, which is where the budget refuses it.
    let instance = program
        .initialize_component("Holder", &BTreeMap::new(), &ComponentInit::default(), &options())
        .unwrap()
        .instance;
    assert_eq!(
        limit_of(program.dispatch_component_actions(&instance, &[tap("h1-1")], &limited)),
        exhausted
    );
}

#[test]
fn a_string_held_many_times_over_is_paid_for_each_time_it_is_written() {
    // One string of 2^14 code units costs 255 operations to build, and a list of 100 references to
    // it 200 more. Written for the host it is 100 strings: each costs one and 256 for its length.
    let program = program(SHARED_SOURCE);
    let args = [NxValue::Int(14), NxValue::Int(100)];
    let run = |options: &RuntimeOptions| program.evaluate_function("fan", &args, options);
    let operations = cost(run);
    assert!(operations > 100 * 257, "{operations}");
    assert_eq!(limit_of(run(&budget(1000))), limit("maxOperations", 1000));
}

const WALK_SOURCE: &str = "
type F = { v:float64 }
type Opt = { a:int = 1 b?:int c?:string }
let selfEq(f:F): boolean = { f == f }
let same(o:object): boolean = { o == o }
let selfEqElement(v:float64): boolean = { same(<box v={v} />) }
let ignore(content c:object): int = { 1 }
let many(xs:object, count:int) = { for i in 0..count { ignore(xs) } }
let repeated(o:object, n:int) = { for i in 0..n { o } }
let passObject(o:object): object = { o }
let patchOnly() = { <Opt.Update b={5} /> }
let cmpMany(a:object, b:object, count:int) = { for i in 0..count { a == b } }
let bindObject(xs:object): int = { ignore(xs) }
external component <Button emits { Tapped { } } />
action Ping = { n:int }
let wideOther(): object = { 1152921504606846977 }
let wideSame(): object = { 1152921504606846976 }
let patWide(): int = { if wideOther() is { 1152921504606846976 => 1  else => 0 } }
let patWideSame(): int = { if wideSame() is { 1152921504606846976 => 1  else => 0 } }
let eqWide(o:object): boolean = { o == wideSame() }
let patHost(o:object): int = { if o is { 1152921504606846976 => 1  else => 0 } }
component <WideChild n:object /> = { <row same={n == 1152921504606846976} /> }
component <WideParent /> = { <WideChild n={1152921504606846976} /> }
component <WideHolder /> = {
  state { held:object = {1152921504606846976} }
  <row same={held == 1152921504606846976} />
}
let button(a:int, b:int) = <Button onTapped=<Ping n={a + b} /> />
let sameButton(a:int, b:int, c:int, d:int): boolean = { button(a, b) == button(c, d) }
component <Keeper /> = {
  state { data?:object n:int = {0} }
  <panel>
    <Button onTapped=<Update data=<bag>{for i in 0..50000 { <leaf /> }}</bag> /> />
    <Button onTapped=<Update data={data} n={n + 1} /> />
    <Button onTapped=<Update data=<box>{data}</box> n={n + 1} /> />
    <Button onTapped=<Update data=<leaf /> /> />
  </panel>
}
";

#[test]
fn two_handlers_are_compared_up_to_the_first_captured_value_that_differs() {
    // Each button holds a handler made by one node, which captured the two parameters of
    // `button`. Twenty-three is what surrounds the comparison in `sameButton`; the comparison is
    // the pair of buttons, the pair of handlers, and each pair of captured values compared. The
    // slots of a frame are in one order in every runtime, so the walk stops at the first that
    // differs: a walk to the end would cost one more where both differ.
    let program = program(WALK_SOURCE);
    let run = |args: [i64; 4]| {
        let args = args.map(NxValue::Int);
        let program = &program;
        move |options: &RuntimeOptions| program.evaluate_function("sameButton", &args, options)
    };
    assert_costs(23 + 4, run([1, 2, 1, 2]));
    assert_costs(23 + 3, run([9, 2, 1, 2]));
    assert_costs(23 + 4, run([1, 9, 1, 2]));
    assert_costs(23 + 3, run([9, 9, 1, 2]));
    for (args, equal) in [([1, 2, 1, 2], true), ([9, 9, 1, 2], false), ([1, 9, 1, 2], false)] {
        for options in [options(), budget(100_000)] {
            assert_eq!(run(args)(&options), Ok(NxValue::Bool(equal)));
        }
    }
}

#[test]
fn an_equality_gives_one_result_with_and_without_a_budget() {
    // A value that holds a NaN is not equal to itself, however it is held: the comparison looks
    // at the values under any budget and under none, and never at whether they are one allocation.
    let program = program(WALK_SOURCE);
    let record = NxValue::Record {
        type_name: None,
        properties: BTreeMap::from([("v".to_string(), NxValue::Float(f64::NAN))]),
    };
    for options in [options(), budget(100_000)] {
        assert_eq!(
            program.evaluate_function("selfEq", std::slice::from_ref(&record), &options),
            Ok(NxValue::Bool(false))
        );
        assert_eq!(
            program.evaluate_function("selfEqElement", &[NxValue::Float(f64::NAN)], &options),
            Ok(NxValue::Bool(false))
        );
        assert_eq!(
            program.evaluate_function("selfEqElement", &[NxValue::Float(1.5)], &options),
            Ok(NxValue::Bool(true))
        );
    }
}

#[test]
fn a_list_bound_to_a_content_parameter_is_paid_for_at_every_call() {
    // The list is built anew for each call, so each call places its items: a host's list at
    // `object`, which is checked as one value, is not copied for the price of a call.
    let program = program(WALK_SOURCE);
    let run = |length: usize, calls: i64| {
        let args = [ints(length), NxValue::Int(calls)];
        let program = &program;
        move |options: &RuntimeOptions| program.evaluate_function("many", &args, options)
    };
    // Two arguments checked; the loop and its `Range`, eight; and for each of ten calls the
    // `call`, its callee and its argument, 100 items bound, the list and the result checked, the
    // body, and the item the loop places; then the list and its ten numbers written.
    assert_costs(2 + 8 + 10 * (3 + 100 + 2 + 1 + 1) + 11, run(100, 10));
    let error = failure(run(20_000, 16_000)(&budget(100_000)));
    let diagnostic = &error.diagnostics[0];
    assert_eq!(diagnostic.limit, Some(limit("maxOperations", 100_000)));
    assert_eq!(diagnostic.declaration.as_deref(), Some("main.nx::ignore"));
    assert_eq!(diagnostic.source, None);
}

#[test]
fn names_a_host_supplied_are_paid_for_by_length_where_they_are_written_and_compared() {
    // A declared name is short and costs nothing. A host may send an object at `object` whose
    // key, or whose `$type`, is as long as it likes, and the Rust runtime copies both each time
    // the object is written: a megabyte key written 20 times is 20 megabytes.
    let program = program(WALK_SOURCE);
    let long = "k".repeat(1 << 20);
    let keyed = NxValue::Record {
        type_name: None,
        properties: BTreeMap::from([(long.clone(), NxValue::Int(1))]),
    };
    let typed = NxValue::Record {
        type_name: Some(long),
        properties: BTreeMap::from([("k".to_string(), NxValue::Int(1))]),
    };
    // Compared with itself, an object costs the pair, the names read and the pair of values: the
    // key twice, once for each record that holds it, or the type name once.
    for (object, names) in [(keyed, 2 << 14), (typed, 1 << 14)] {
        let args = [object.clone(), NxValue::Int(20)];
        let written = cost(|options| program.evaluate_function("repeated", &args, options));
        // Twenty objects, each one for the record, 16,384 for its name and one for its value.
        assert!(written > 20 * (1 << 14), "{written}");
        assert_eq!(
            limit_of(program.evaluate_function("repeated", &args, &budget(100_000))),
            limit("maxOperations", 100_000)
        );
        let compared = cost(|options| program.evaluate_function("same", std::slice::from_ref(&object), options));
        assert!((names..names + 100).contains(&compared), "{compared}");
    }
}

#[test]
fn text_in_a_form_the_runtime_gives_a_meaning_to_is_paid_for_by_its_length() {
    // A host can spell the forms a runtime gives a meaning to: the record a wide integer takes
    // in JSON, and records with the type name or the tag of a handler or a function. What it puts
    // in them is text like any other, written and compared at its length.
    let program = program(WALK_SOURCE);
    let long = "k".repeat(1 << 20);
    let spelled = [
        ("nx.int", "value"),
        ("ActionHandler", "action"),
        ("Function", "module"),
    ]
    .map(|(type_name, field)| NxValue::Record {
        type_name: Some(type_name.to_string()),
        properties: BTreeMap::from([(field.to_string(), NxValue::String(long.clone()))]),
    })
    .into_iter()
    .chain([NxValue::Record {
        type_name: Some("nx.int".to_string()),
        properties: BTreeMap::from([
            ("value".to_string(), NxValue::String("1".to_string())),
            ("extra".to_string(), NxValue::String(long.clone())),
        ]),
    }])
    .chain(["actionHandler", "functionReference"].map(|tag| NxValue::Record {
        type_name: None,
        properties: BTreeMap::from([
            ("$nxKind".to_string(), NxValue::String(tag.to_string())),
            ("text".to_string(), NxValue::String(long.clone())),
        ]),
    }));
    for object in spelled {
        let args = [object.clone(), NxValue::Int(20)];
        let written = cost(|options| program.evaluate_function("repeated", &args, options));
        assert!(written > 20 * (1 << 14), "{written}");
        assert_eq!(
            limit_of(program.evaluate_function("repeated", &args, &budget(100_000))),
            limit("maxOperations", 100_000)
        );
        let compared = cost(|options| program.evaluate_function("same", std::slice::from_ref(&object), options));
        assert!(compared >= 1 << 14, "{compared}");
    }
}

#[test]
fn a_wide_integer_pattern_matches_by_its_digits() {
    // An integer outside JavaScript's safe range is a number like any other here: a pattern that
    // is one matches by equality, and the comparison is paid for. The TypeScript runtime cannot
    // hold such an integer and refuses it where it reads it.
    let program = program(WALK_SOURCE);
    for (function, matched) in [("patWide", 0), ("patWideSame", 1)] {
        let run = |options: &RuntimeOptions| program.evaluate_function(function, &[], options);
        assert_eq!(run(&options()), Ok(NxValue::Int(matched)));
        assert_costs(10, run);
    }
}

#[test]
fn the_json_form_of_a_wide_integer_is_a_record_at_object() {
    // A host that means an integer outside JavaScript's safe range passes `NxValue::Int`. The
    // record canonical JSON spells one with is, at `object`, the record it is: it costs what a
    // record with one string field costs, it is not equal to the program's integer of those
    // digits, and it does not match that integer as a pattern. The TypeScript runtime reads the
    // record the same way, and refuses the program's own integer, which it cannot hold.
    let program = program(WALK_SOURCE);
    let spelled = json(r#"{ "$type": "nx.int", "value": "1152921504606846976" }"#);
    let wide = NxValue::Record {
        type_name: Some("nx.int".to_string()),
        properties: (0..8000)
            .map(|key| (format!("k{key}"), NxValue::Int(key)))
            .chain([("value".to_string(), NxValue::String("1152921504606846976".to_string()))])
            .collect(),
    };
    let integer = NxValue::Int(1152921504606846976);
    let run = |function: &'static str, argument: &NxValue| {
        let args = [argument.clone()];
        let program = &program;
        move |options: &RuntimeOptions| program.evaluate_function(function, &args, options)
    };
    // The program's own: the node, the result checked, and one value written.
    assert_costs(3, |options| program.evaluate_function("wideSame", &[], options));
    // Written as a record and its string, where a number is four.
    assert_costs(5, run("passObject", &spelled));
    // One pair: a record is no integer, however wide it is.
    assert_costs(10, run("eqWide", &spelled));
    assert_costs(10, run("eqWide", &wide));
    assert_costs(8, run("patHost", &spelled));
    assert_eq!(run("passObject", &spelled)(&options()), Ok(spelled.clone()));
    assert_eq!(run("eqWide", &spelled)(&options()), Ok(NxValue::Bool(false)));
    assert_eq!(run("patHost", &spelled)(&options()), Ok(NxValue::Int(0)));
    assert_eq!(run("same", &spelled)(&budget(100_000)), Ok(NxValue::Bool(true)));
    assert_eq!(run("eqWide", &integer)(&options()), Ok(NxValue::Bool(true)));
    assert_eq!(run("patHost", &integer)(&options()), Ok(NxValue::Int(1)));

    // The integer stays the integer where a host hands it on: as a prop of a child, and in state
    // passed to the next call. The wrapper in either place, at `object`, is the record.
    let rendered = |component: &str, props: &[(&str, &NxValue)], state: Option<&NxValue>| {
        let props: BTreeMap<String, NxValue> = props
            .iter()
            .map(|(name, value)| (name.to_string(), (*value).clone()))
            .collect();
        let state = state.map(|held| BTreeMap::from([("held".to_string(), held.clone())]));
        let init = ComponentInit {
            state: state.as_ref(),
            ..ComponentInit::default()
        };
        program
            .initialize_component(component, &props, &init, &options())
            .expect("the component initializes")
            .rendered
    };
    let same = json(r#"{ "$type": "row", "same": true }"#);
    let differs = json(r#"{ "$type": "row", "same": false }"#);
    assert_eq!(rendered("WideChild", &[("n", &integer)], None), same);
    assert_eq!(rendered("WideChild", &[("n", &spelled)], None), differs);
    assert_eq!(rendered("WideHolder", &[], None), same);
    assert_eq!(rendered("WideHolder", &[], Some(&integer)), same);
    assert_eq!(rendered("WideHolder", &[], Some(&spelled)), differs);
}

#[test]
fn two_records_are_lined_up_by_name_and_every_name_is_paid_for() {
    // Lining two records up reads the names of both, so a comparison that fails on the names is
    // paid for by the names: a wide object a host passed at `object` is not compared with another
    // for the price of one operation, whichever of the two is the wide one.
    let program = program(WALK_SOURCE);
    let run = |left: &str, right: &str, count: i64| {
        let args = [json(left), json(right), NxValue::Int(count)];
        let program = &program;
        move |options: &RuntimeOptions| program.evaluate_function("cmpMany", &args, options)
    };
    // Three arguments checked, eight for the loop and its `Range`, the `binary` and its two
    // slots, the item placed, and the list and its boolean written: 17, and then the comparison.
    // The pair of records, the pair of values under `a`, and one for each of `b` and `c`, which
    // only one holds.
    assert_costs(17 + 4, run(r#"{"a":1,"b":2}"#, r#"{"a":1,"c":2}"#, 1));
    // The pair of records and the pair under `a`; `b` is held by the second alone, and the first.
    assert_costs(17 + 3, run(r#"{"a":1}"#, r#"{"a":1,"b":2}"#, 1));
    assert_costs(17 + 3, run(r#"{"a":1,"b":2}"#, r#"{"a":1}"#, 1));
    // Records of different types: the pair, and nothing about their fields.
    assert_costs(
        17 + 1,
        run(r#"{"$type":"A","a":1,"b":2}"#, r#"{"$type":"B","a":1}"#, 1),
    );

    let wide = format!(
        "{{{}}}",
        (0..8000)
            .map(|key| format!(r#""k{key}":{key}"#))
            .collect::<Vec<_>>()
            .join(",")
    );
    for (left, right) in [(wide.as_str(), r#"{"x":1}"#), (r#"{"x":1}"#, wide.as_str())] {
        assert!(cost(run(left, right, 1)) > 8000);
        assert_eq!(
            limit_of(run(left, right, 16_000)(&budget(100_000))),
            limit("maxOperations", 100_000)
        );
    }
}

#[test]
fn two_lists_are_compared_in_order_up_to_the_first_pair_that_differs() {
    // A list's items are in one order in every runtime, so a comparison that stops at the first
    // difference costs the same everywhere. Seventeen is what surrounds each comparison in
    // `cmpMany`; the comparison is the pair of lists and each pair of items compared.
    let program = program(WALK_SOURCE);
    let run = |left: &str, right: &str, count: i64| {
        let args = [json(left), json(right), NxValue::Int(count)];
        let program = &program;
        move |options: &RuntimeOptions| program.evaluate_function("cmpMany", &args, options)
    };
    assert_costs(17 + 4, run("[1,2,3]", "[1,2,3]", 1));
    assert_costs(17 + 4, run("[1,2,3]", "[1,2,9]", 1));
    assert_costs(17 + 2, run("[0,1,2]", "[9,1,2]", 1));
    // Lists of different lengths compare no items.
    assert_costs(17 + 1, run("[1,2]", "[1,2,3]", 1));
    // The first items are lists that differ in their second item: the outer pair, the inner
    // pair, and two pairs of numbers. The second items are not reached.
    assert_costs(17 + 4, run("[[1,2],[3,4]]", "[[1,9],[3,4]]", 1));
    // Two records are compared field by field to the end, whichever differs, so the first items
    // cost the pair and both pairs of fields; the list then stops there.
    assert_costs(
        17 + 4,
        run(r#"[{"a":1,"b":2},{"a":5}]"#, r#"[{"a":9,"b":9},{"a":5}]"#, 1),
    );

    // Two lists of 100,000 that differ in their first item cost two operations to compare, under
    // a budget as under none, so 200 comparisons fit a budget of 100,000 many times over.
    let long = |first: i64| {
        NxValue::Array(
            std::iter::once(first)
                .chain(1..100_000)
                .map(NxValue::Int)
                .collect(),
        )
    };
    let args = [long(0), long(-1), NxValue::Int(200)];
    let operations = cost(|options| program.evaluate_function("cmpMany", &args, options));
    assert!(operations < 2000, "{operations}");
}

#[test]
fn a_value_made_of_empty_values_costs_its_length_to_write_and_to_bind() {
    // An empty value takes a place in a list like any other, so a list of them is as long to copy
    // as a list of numbers. A program cannot build one, since sequences flatten, but a host can
    // pass one at `object`: 20,000 of them in a list, as `null` or as `[]`, or under 8,000 keys.
    let program = program(WALK_SOURCE);
    let nulls = format!(r#"{{"a":[{}]}}"#, vec!["null"; 20_000].join(","));
    let empties = format!(r#"{{"a":[{}]}}"#, vec!["[]"; 20_000].join(","));
    let keys = format!(
        "{{{}}}",
        (0..8000)
            .map(|key| format!(r#""k{key}":null"#))
            .collect::<Vec<_>>()
            .join(",")
    );
    for (object, values) in [(nulls, 20_000), (empties, 20_000), (keys, 8000)] {
        let args = [json(&object), NxValue::Int(500)];
        let run = |options: &RuntimeOptions| program.evaluate_function("repeated", &args, options);
        // Written 500 times, it costs its values 500 times.
        assert!(cost(|options| {
            program.evaluate_function("repeated", &[args[0].clone(), NxValue::Int(2)], options)
        }) > 2 * values);
        assert_eq!(limit_of(run(&budget(100_000))), limit("maxOperations", 100_000));
    }
    // Bound to a content parameter 10,000 times, a list of 20,000 empty values costs its length
    // each time, as a list of numbers does.
    let args = [
        json(&format!("[{}]", vec!["null"; 20_000].join(","))),
        NxValue::Int(10_000),
    ];
    let error = failure(program.evaluate_function("many", &args, &budget(100_000)));
    assert_eq!(error.diagnostics[0].limit, Some(limit("maxOperations", 100_000)));
    assert_eq!(error.diagnostics[0].declaration.as_deref(), Some("main.nx::ignore"));
}

#[test]
fn every_item_bound_to_a_content_parameter_costs_one_whatever_it_holds() {
    // The argument checked, the `call`, its callee and its slot, the list checked, the body, both
    // results checked, and the number written: nine, and one for each item of the list, empty or
    // not. A list among the items costs one for each item it contributes, and one when it
    // contributes none.
    let program = program(WALK_SOURCE);
    for (list, items) in [
        ("[null]", 1),
        ("[1,null]", 2),
        ("[null,[null],1]", 3),
        ("[[null,null],1]", 3),
        ("[[],[],[1,2,3]]", 5),
    ] {
        let args = [json(list)];
        assert_costs(9 + items, |options: &RuntimeOptions| {
            program.evaluate_function("bindObject", &args, options)
        });
    }
}

#[test]
fn a_patch_that_supplies_a_large_value_again_does_not_walk_it_again() {
    // The depth check after a patch is the runtime's own walk, which nothing pays for. It
    // remembers what it found for the batch, so 4,000 patches that each hand the same 50,000
    // elements back, or wrap them once more, do not walk them 4,000 times.
    let program = program(WALK_SOURCE);
    let instance = program
        .initialize_component("Keeper", &BTreeMap::new(), &ComponentInit::default(), &options())
        .unwrap()
        .instance;
    let batch = |token: &str, count: usize| -> Vec<NxValue> {
        std::iter::once(tap("h1-1"))
            .chain(std::iter::repeat_n(tap(token), count))
            .collect()
    };
    let started = std::time::Instant::now();
    let kept = program
        .dispatch_component_actions(&instance, &batch("h1-2", 4000), &options())
        .unwrap();
    assert_eq!(kept.state["n"], NxValue::Int(4000));
    // Wrapped once more by each patch, the value nests one level deeper each time. Each batch
    // ends by storing a shallow value, so the state it leaves is never too deep to write and
    // only the check after a patch can refuse it: the element, its content list and a leaf are
    // three levels, so 253 wraps reach the 256th and one more is refused, found by adding the
    // levels remembered for the old value to the depth it now sits at.
    let wrapped = |wraps: usize| {
        let mut entries = batch("h1-3", wraps);
        entries.push(tap("h1-4"));
        program.dispatch_component_actions(&instance, &entries, &options())
    };
    let first_refused = (1..400).find(|wraps| wrapped(*wraps).is_err());
    assert_eq!(first_refused, Some(254));
    assert_eq!(limit_of(wrapped(254)), limit("maxValueNesting", 256));
    // Walking 50,000 elements 4,000 times would take a second in a release build and many in
    // this one; the three batches take a small fraction of that.
    assert!(
        started.elapsed() < std::time::Duration::from_secs(20),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn a_null_in_an_opaque_host_value_is_the_empty_value_and_one_value_written() {
    // The argument and the result are each checked as one value and the slot is one node. Every
    // value written is one, the empty ones too: the record, the empty value under `a`, the list
    // under `b` and its two empty items, and the empty list under `c`.
    let program = program(WALK_SOURCE);
    let object = json(r#"{ "a": null, "b": [null, null], "c": [] }"#);
    assert_costs(3 + 6, |options: &RuntimeOptions| {
        program.evaluate_function("passObject", std::slice::from_ref(&object), options)
    });
}

#[test]
fn a_field_of_an_update_record_is_checked_for_the_update_record() {
    // The record's node, its literal, the field checked, and the record and its field written.
    let program = program(WALK_SOURCE);
    let run = |options: &RuntimeOptions| program.evaluate_function("patchOnly", &[], options);
    assert_costs(5, run);
    // The third charge is the check of `b`, which belongs to the update record and to no node.
    let error = failure(run(&budget(2)));
    assert_eq!(
        error.diagnostics[0].declaration.as_deref(),
        Some("main.nx::Opt.Update")
    );
    assert_eq!(error.diagnostics[0].source, None);
}

const BUSY_SOURCE: &str = "
external component <Button emits { Tapped { } } />
component <Busy /> = {
  state { items?:int+ }
  <Button onTapped=<Update items={for i in 0..300 { i }} /> />
}
type Bag = { items?:int+ }
component <Eager /> = {
  state { bag:Bag = <Bag items={for i in 0..100000 { i }} /> }
  <Button />
}
type Heavy = { bag:Bag = <Bag items={for i in 0..100000 { i }} /> }
component <Costly heavy:Heavy = <Heavy /> /> = {
  state { held?:Heavy }
  <Button />
}";

#[test]
fn a_batch_shares_one_budget_and_leaves_the_instance_usable() {
    let program = program(BUSY_SOURCE);
    let instance = program
        .initialize_component("Busy", &BTreeMap::new(), &ComponentInit::default(), &options())
        .unwrap()
        .instance;
    let one = cost(|options| program.dispatch_component_actions(&instance, &[tap("h1-1")], options));
    // Each handler costs well over a third of the budget, so three of them exceed it together.
    let limited = budget(one * 2);
    assert_eq!(
        limit_of(program.dispatch_component_actions(
            &instance,
            &[tap("h1-1"), tap("h1-1"), tap("h1-1")],
            &limited
        )),
        limit("maxOperations", one * 2)
    );
    assert!(program
        .dispatch_component_actions(&instance, &[tap("h1-1")], &limited)
        .is_ok());
}

#[test]
fn a_state_default_is_under_the_budget() {
    let program = program(BUSY_SOURCE);
    assert_eq!(
        limit_of(program.initialize_component(
            "Eager",
            &BTreeMap::new(),
            &ComponentInit::default(),
            &budget(1000)
        )),
        limit("maxOperations", 1000)
    );
}

#[test]
fn every_component_api_is_under_the_budget() {
    // `Heavy` has a field whose default is a loop of 100,000. It runs when a `Heavy` is
    // constructed, whether as the default of the `heavy` prop or from a host's `{}`, and each API
    // that constructs one is stopped by a budget of 1,000.
    let program = program(BUSY_SOURCE);
    let none = BTreeMap::new();
    let held = fields(r#"{ "held": {} }"#);
    let limited = budget(1000);
    let exhausted = limit("maxOperations", 1000);

    assert_eq!(
        limit_of(program.construct_component_descriptor("Costly", &none, &[], &limited)),
        exhausted
    );
    assert_eq!(
        limit_of(program.evaluate_component("Costly", &none, &none, &limited)),
        exhausted
    );
    assert_eq!(
        limit_of(program.normalize_component_state("Costly", &held, &limited)),
        exhausted
    );
    assert_eq!(
        limit_of(program.apply_component_state_patch(
            "Costly",
            &none,
            &json(r#"{ "held": {} }"#),
            &limited
        )),
        exhausted
    );

    // With no budget each of them evaluates the loop and succeeds.
    assert!(program
        .construct_component_descriptor("Costly", &none, &[], &options())
        .is_ok());
    assert!(program
        .evaluate_component("Costly", &none, &none, &options())
        .is_ok());
    assert!(program
        .normalize_component_state("Costly", &held, &options())
        .is_ok());
    assert!(program
        .apply_component_state_patch("Costly", &none, &json(r#"{ "held": {} }"#), &options())
        .is_ok());
}

#[test]
fn each_call_starts_with_the_whole_budget() {
    let program = program(COST_SOURCE);
    let operations = cost(|options| program.evaluate_function("squares", &[], options));
    let shared = budget(operations * 3 / 2);
    assert!(program.evaluate_function("squares", &[], &shared).is_ok());
    assert!(program.evaluate_function("squares", &[], &shared).is_ok());
}

#[test]
fn call_function_is_under_the_budget() {
    let program = program(COST_SOURCE);
    let squares = json(r#"{ "$type": "Function", "module": "main.nx", "name": "squares" }"#);
    assert_costs(29, |options: &RuntimeOptions| {
        program.call_function(&squares, &BTreeMap::new(), options)
    });
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
