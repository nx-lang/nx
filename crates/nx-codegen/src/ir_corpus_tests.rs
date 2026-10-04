//! The NX IR conformance corpus, from the emitter's side.
//!
//! <para>`specs/ir-conformance` holds NX programs with the images the emitter produces for them,
//! the explained text of each image, the values the interpreter evaluates their entrypoints to,
//! what it renders and emits when a host drives their component lifecycles, and what each
//! evaluation costs in operations. These tests pin the images byte for byte, keep the explained
//! text, the expected results and the operation counts in step, check that the corpus covers every
//! kind the schema defines, and hold the size budget. Set
//! `NX_UPDATE_CORPUS=1` to rewrite the expected files after an intended change, then review the
//! diff of the explained text.</para>

use crate::ir::NxIrEmitOptions;
use crate::{build_codegen_program, build_nx_ir_artifacts};
use nx_api::{
    build_workspace_program_artifact, dispatch_component_actions_program_artifact,
    eval_program_artifact_function, initialize_component_program_artifact,
    ComponentDispatchEvalResult, ComponentInitEvalResult, EvalResult, NxWorkspace,
    NxWorkspaceModule, ProgramArtifact, ProgramBuildContext,
};
use nx_ir::{explain_nx_ir, explain_nx_ir_image};
use nx_ir::{kinds, write_nx_ir_image, NxIrArtifact, NxIrImage};
use nx_ir_runtime::{
    ComponentInit, LinkOptions, NxIrRuntimeError, PreparedModule, Program, RuntimeOptions,
};
use nx_value::NxValue;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const SIZE_BUDGET: f64 = 6.0;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProgramManifest {
    entry: String,
    #[serde(default)]
    implicit_imports: Vec<String>,
    #[serde(default)]
    versions: BTreeMap<String, String>,
    emit: Vec<String>,
    entrypoints: Vec<Entrypoint>,
    #[serde(default)]
    lifecycles: Vec<Lifecycle>,
    /// Whether `operations.json` also records, for each entrypoint, where budgets below its count
    /// stop it.
    #[serde(default)]
    record_failures: bool,
}

#[derive(Debug, Deserialize)]
struct Entrypoint {
    module: String,
    function: String,
}

/// A component to initialize and the batches to dispatch against it, in order, written as a host
/// would send them: handler tokens are literal, since the token scheme is deterministic and the
/// literal is itself the claim a runtime is checked against.
#[derive(Debug, Deserialize)]
struct Lifecycle {
    module: String,
    component: String,
    #[serde(default)]
    props: BTreeMap<String, Value>,
    batches: Vec<Vec<Value>>,
}

pub(crate) struct CorpusProgram {
    pub(crate) name: String,
    dir: PathBuf,
    manifest: ProgramManifest,
    sources: BTreeMap<String, String>,
    artifact: ProgramArtifact,
}

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../specs/ir-conformance")
}

fn updating() -> bool {
    std::env::var_os("NX_UPDATE_CORPUS").is_some()
}

fn collect_sources(dir: &Path, prefix: &str, sources: &mut BTreeMap<String, String>) {
    let mut entries = fs::read_dir(dir)
        .expect("corpus program directory")
        .map(|entry| entry.expect("directory entry").path())
        .collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if path.is_dir() {
            if name != "expected" {
                collect_sources(&path, &format!("{prefix}{name}/"), sources);
            }
        } else if name.ends_with(".nx") {
            let source = fs::read_to_string(&path).expect("corpus source");
            sources.insert(format!("{prefix}{name}"), source);
        }
    }
}

pub(crate) fn load_programs() -> Vec<CorpusProgram> {
    let root = corpus_root();
    let mut dirs = fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("corpus root {}: {error}", root.display()))
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    dirs.sort();
    assert!(
        !dirs.is_empty(),
        "the corpus at {} has no programs",
        root.display()
    );

    dirs.into_iter()
        .map(|dir| {
            let name = dir.file_name().unwrap().to_string_lossy().to_string();
            let manifest: ProgramManifest = serde_json::from_str(
                &fs::read_to_string(dir.join("program.json"))
                    .unwrap_or_else(|error| panic!("{name}/program.json: {error}")),
            )
            .unwrap_or_else(|error| panic!("{name}/program.json: {error}"));
            let mut sources = BTreeMap::new();
            collect_sources(&dir, "", &mut sources);
            let modules = sources
                .iter()
                .map(|(identity, source)| {
                    let module = NxWorkspaceModule::from_source(identity.clone(), source.clone())
                        .expect("workspace module");
                    match manifest.versions.get(identity) {
                        Some(version) => module.with_version(version.clone()),
                        None => module,
                    }
                })
                .collect::<Vec<_>>();
            let workspace = NxWorkspace::new(modules).expect("workspace");
            let build_context =
                ProgramBuildContext::empty().with_implicit_imports(&manifest.implicit_imports);
            let artifact =
                build_workspace_program_artifact(&workspace, &manifest.entry, &build_context)
                    .unwrap_or_else(|diagnostics| {
                        panic!("corpus program '{name}' does not build: {diagnostics:#?}")
                    });
            CorpusProgram {
                name,
                dir,
                manifest,
                sources,
                artifact,
            }
        })
        .collect()
}

pub(crate) fn emit(program: &CorpusProgram, debug: bool) -> Vec<NxIrArtifact> {
    let codegen = build_codegen_program(&program.artifact).unwrap_or_else(|error| {
        panic!("corpus program '{}': {:?}", program.name, error.diagnostics)
    });
    build_nx_ir_artifacts(
        &codegen,
        &NxIrEmitOptions {
            modules: Some(program.manifest.emit.clone()),
            debug,
        },
    )
    .unwrap_or_else(|error| panic!("corpus program '{}': {:?}", program.name, error.diagnostics))
}

/// `expected/<identity>.nxir` or `.stripped.nxir`, with `/` in the identity written `__`.
fn expected_path(program: &CorpusProgram, identity: &str, debug: bool) -> PathBuf {
    let suffix = if debug { ".nxir" } else { ".stripped.nxir" };
    program
        .dir
        .join("expected")
        .join(format!("{}{suffix}", identity.replace('/', "__")))
}

/// The explained text beside an image: `<image>.txt`.
fn explained_path(image: &Path) -> PathBuf {
    let mut name = image.file_name().unwrap().to_os_string();
    name.push(".txt");
    image.with_file_name(name)
}

/// The first lines on which two explained artifacts differ, for a failure message a person can
/// act on without reading the tables.
fn explained_difference(expected: &str, actual: &str) -> String {
    let expected_lines = expected.lines().collect::<Vec<_>>();
    let actual_lines = actual.lines().collect::<Vec<_>>();
    let mut report = String::new();
    let mut shown = 0;
    for index in 0..expected_lines.len().max(actual_lines.len()) {
        let left = expected_lines.get(index).copied().unwrap_or("<end>");
        let right = actual_lines.get(index).copied().unwrap_or("<end>");
        if left != right {
            report.push_str(&format!(
                "  line {}:\n    expected: {left}\n    actual:   {right}\n",
                index + 1
            ));
            shown += 1;
            if shown == 10 {
                report.push_str("  ...\n");
                break;
            }
        }
    }
    if report.is_empty() {
        report.push_str("  (the explained text is identical; the difference is in the tables or the debug section)\n");
    }
    report
}

fn write_expected(path: &Path, content: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).expect("expected directory");
    fs::write(path, content).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
}

/// The corpus's images: every program's artifacts with and without debug data, each with the path
/// its expected image lives at.
fn corpus_images() -> Vec<(String, PathBuf, NxIrArtifact, Vec<u8>)> {
    let mut images = Vec::new();
    for program in load_programs() {
        for debug in [false, true] {
            for artifact in emit(&program, debug) {
                let identity = artifact.modules[0].identity.clone();
                let path = expected_path(&program, &identity, debug);
                let bytes = write_nx_ir_image(&artifact).expect("image");
                let label = format!(
                    "{} ({identity}{})",
                    program.name,
                    if debug { ", with debug" } else { "" }
                );
                images.push((label, path, artifact, bytes));
            }
        }
    }
    images
}

#[test]
fn corpus_artifacts_are_pinned() {
    let mut failures = Vec::new();
    for (label, path, artifact, actual) in corpus_images() {
        let explained_actual = explain_nx_ir(&artifact).expect("explain");
        if updating() {
            write_expected(&path, &actual);
            write_expected(&explained_path(&path), explained_actual.as_bytes());
            continue;
        }
        let expected = match fs::read(&path) {
            Ok(expected) => expected,
            Err(_) => {
                failures.push(format!(
                    "{label}: no expected artifact at {} (run with NX_UPDATE_CORPUS=1 to create it)",
                    path.display()
                ));
                continue;
            }
        };
        if expected != actual {
            let explained_expected = explain_nx_ir_image(&expected)
                .unwrap_or_else(|error| format!("<the expected image does not open: {error}>"));
            failures.push(format!(
                "{label} differs from {}:\n{}",
                path.display(),
                explained_difference(&explained_expected, &explained_actual)
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "corpus artifacts changed:\n{}",
        failures.join("\n")
    );
}

/// The committed text beside each image is the explanation of that image, so a reviewer reading
/// the text reads what the bytes say.
#[test]
fn corpus_explained_text_matches_the_committed_images() {
    if updating() {
        return;
    }
    let mut failures = Vec::new();
    for (label, path, _, _) in corpus_images() {
        let Ok(image) = fs::read(&path) else {
            continue;
        };
        let text_path = explained_path(&path);
        let expected = fs::read_to_string(&text_path).unwrap_or_default();
        let actual = explain_nx_ir_image(&image)
            .unwrap_or_else(|error| panic!("{label}: the committed image does not open: {error}"));
        if expected != actual {
            failures.push(format!(
                "{label}: {} is not the explanation of {}:\n{}",
                text_path.display(),
                path.display(),
                explained_difference(&expected, &actual)
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "corpus explained text is stale (run with NX_UPDATE_CORPUS=1):\n{}",
        failures.join("\n")
    );
}

/// Every committed image opens, and reads back as the model the emitter produces for it.
#[test]
fn corpus_images_read_back_as_their_models() {
    for (label, _, artifact, bytes) in corpus_images() {
        let image = NxIrImage::open(&bytes).unwrap_or_else(|error| panic!("{label}: {error}"));
        assert_eq!(image.to_artifact(), artifact, "{label}");
    }
}

/// The interpreter's JSON with every integer outside JavaScript's safe range spelled as the
/// runtime's `nx.int` wrapper, which is the canonical form `docs/nx-ir-format.md` gives such a
/// value: a JSON reader that parses the bare digits loses them.
fn canonical_json(value: Value) -> Value {
    const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
    match value {
        Value::Number(number) => match number.as_i64() {
            Some(int) if !(-MAX_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(&int) => {
                serde_json::json!({ "$type": "nx.int", "value": int.to_string() })
            }
            _ => Value::Number(number),
        },
        Value::Array(items) => Value::Array(items.into_iter().map(canonical_json).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .into_iter()
                .map(|(key, value)| (key, canonical_json(value)))
                .collect(),
        ),
        other => other,
    }
}

fn canonical_nx_value(value: &NxValue) -> Value {
    canonical_json(
        serde_json::from_str(&value.to_json_string().expect("json")).expect("json value"),
    )
}

fn nx_value(value: &Value) -> NxValue {
    serde_json::from_value(value.clone()).expect("an NxValue")
}

/// Runs one lifecycle through `nx-api` and records what a runtime must reproduce: the rendered
/// output of initialization and, per batch, the rendered output and the effects. State is not
/// recorded, since `nx-api` returns it only inside an opaque snapshot; the rendered output
/// reflects it.
fn run_lifecycle(program: &CorpusProgram, lifecycle: &Lifecycle) -> Value {
    assert_eq!(
        lifecycle.module, program.manifest.entry,
        "corpus program '{}': a lifecycle's component is reached through the entry module",
        program.name
    );
    let props = NxValue::Record {
        type_name: None,
        properties: lifecycle
            .props
            .iter()
            .map(|(name, value)| (name.clone(), nx_value(value)))
            .collect(),
    };
    let initialized = match initialize_component_program_artifact(
        &program.artifact,
        &lifecycle.component,
        &props,
    ) {
        ComponentInitEvalResult::Ok(result) => result,
        ComponentInitEvalResult::Err(diagnostics) => panic!(
            "corpus program '{}' lifecycle {} failed to initialize: {diagnostics:#?}",
            program.name, lifecycle.component
        ),
    };
    let mut snapshot = initialized.state_snapshot;
    let mut batches = Vec::with_capacity(lifecycle.batches.len());
    for (index, batch) in lifecycle.batches.iter().enumerate() {
        let entries = batch.iter().map(nx_value).collect::<Vec<_>>();
        let dispatched = match dispatch_component_actions_program_artifact(
            &program.artifact,
            &snapshot,
            &entries,
        ) {
            ComponentDispatchEvalResult::Ok(result) => result,
            ComponentDispatchEvalResult::Err(diagnostics) => panic!(
                "corpus program '{}' lifecycle {} batch {index} failed: {diagnostics:#?}",
                program.name, lifecycle.component
            ),
        };
        batches.push(serde_json::json!({
            "rendered": canonical_nx_value(&dispatched.rendered),
            "effects": dispatched.effects.iter().map(canonical_nx_value).collect::<Vec<_>>(),
        }));
        snapshot = dispatched.state_snapshot;
    }
    serde_json::json!({
        "initial": canonical_nx_value(&initialized.rendered),
        "batches": batches,
    })
}

#[test]
fn corpus_results_match_the_interpreter() {
    let mut failures = Vec::new();
    for program in load_programs() {
        let mut results = BTreeMap::new();
        for entrypoint in &program.manifest.entrypoints {
            let value = match eval_program_artifact_function(
                &program.artifact,
                &entrypoint.module,
                &entrypoint.function,
            ) {
                EvalResult::Ok(value) => value,
                EvalResult::Err(diagnostics) => panic!(
                    "corpus program '{}' entrypoint {}::{} failed: {diagnostics:#?}",
                    program.name, entrypoint.module, entrypoint.function
                ),
            };
            results.insert(
                format!("{}::{}", entrypoint.module, entrypoint.function),
                canonical_nx_value(&value),
            );
        }
        for lifecycle in &program.manifest.lifecycles {
            let key = format!("{}::{}", lifecycle.module, lifecycle.component);
            assert!(
                !results.contains_key(&key),
                "corpus program '{}' records {key} twice",
                program.name
            );
            results.insert(key, run_lifecycle(&program, lifecycle));
        }
        let path = program.dir.join("expected").join("results.json");
        let actual = format!(
            "{}\n",
            serde_json::to_string_pretty(&results).expect("results")
        );
        if updating() {
            write_expected(&path, actual.as_bytes());
            continue;
        }
        let expected = fs::read_to_string(&path).unwrap_or_default();
        if expected != actual {
            failures.push(format!(
                "{}: the interpreter's results differ from {}:\n{}",
                program.name,
                path.display(),
                explained_difference(&expected, &actual)
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "corpus results changed:\n{}",
        failures.join("\n")
    );
}

/// A program's images as the Rust runtime prepares them, by identity.
fn prepared_modules(program: &CorpusProgram, debug: bool) -> BTreeMap<String, PreparedModule> {
    emit(program, debug)
        .into_iter()
        .map(|artifact| {
            let identity = artifact.modules[0].identity.clone();
            let bytes = write_nx_ir_image(&artifact).expect("image");
            let module = PreparedModule::prepare(bytes)
                .unwrap_or_else(|error| panic!("{}/{identity}: {error}", program.name));
            (identity, module)
        })
        .collect()
}

/// Links the program whose entry is `entry`. Only the program's own modules are supplied; an image
/// that names the prelude links the runtime's built-in one.
fn linked_program(modules: &BTreeMap<String, PreparedModule>, entry: &str) -> Program {
    Program::link(
        &modules[entry],
        |identity| modules.get(identity).cloned(),
        &LinkOptions::default(),
    )
    .unwrap_or_else(|error| panic!("{entry}: {error}"))
}

fn budget(operations: u64) -> RuntimeOptions {
    RuntimeOptions {
        max_operations: Some(operations),
        ..RuntimeOptions::default()
    }
}

/// What an evaluation costs: the least budget it succeeds under, found by doubling the budget and
/// then bisecting. An evaluation that fails for any other reason fails the test.
fn count_operations<T>(
    label: &str,
    run: impl Fn(&RuntimeOptions) -> Result<T, NxIrRuntimeError>,
) -> u64 {
    if let Err(error) = run(&RuntimeOptions::default()) {
        panic!("{label} fails in the Rust runtime: {error}");
    }
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

/// Where a budget below an evaluation's count stops it: the declaration and the span of the node
/// whose charge exhausted it.
fn failure_record<T>(
    label: &str,
    operations: u64,
    run: impl Fn(&RuntimeOptions) -> Result<T, NxIrRuntimeError>,
) -> Value {
    let Err(error) = run(&budget(operations)) else {
        panic!("{label} succeeds under a budget of {operations}, below its count");
    };
    let diagnostic = &error.diagnostics[0];
    assert_eq!(
        diagnostic.limit.map(|limit| limit.name),
        Some("maxOperations"),
        "{label}: {error}"
    );
    let mut record = serde_json::json!({
        "budget": operations,
        "declaration": diagnostic.declaration,
    });
    if let Some(source) = &diagnostic.source {
        record["source"] = serde_json::json!({
            "identity": source.identity,
            "start": source.start,
            "end": source.end,
        });
    }
    record
}

/// What every evaluation of a program costs in the Rust runtime: each entrypoint's count, keyed as
/// `results.json` keys it, and for each lifecycle the count of initialization and of each batch.
/// A program that records failures also gets, per entrypoint, where half its count and one less
/// than its count stop it.
fn measure_operations(program: &CorpusProgram) -> Value {
    let modules = prepared_modules(program, true);
    let mut counts = serde_json::Map::new();
    let mut failures = serde_json::Map::new();
    for entrypoint in &program.manifest.entrypoints {
        let key = format!("{}::{}", entrypoint.module, entrypoint.function);
        let label = format!("corpus program '{}' entrypoint {key}", program.name);
        let linked = linked_program(&modules, &entrypoint.module);
        let run = |options: &RuntimeOptions| linked.evaluate_function(&entrypoint.function, &[], options);
        let operations = count_operations(&label, run);
        counts.insert(key.clone(), operations.into());
        if program.manifest.record_failures && operations > 0 {
            let budgets = BTreeSet::from([operations / 2, operations - 1]);
            failures.insert(
                key,
                budgets
                    .into_iter()
                    .map(|budget| failure_record(&label, budget, run))
                    .collect(),
            );
        }
    }
    for lifecycle in &program.manifest.lifecycles {
        let key = format!("{}::{}", lifecycle.module, lifecycle.component);
        let label = format!("corpus program '{}' lifecycle {key}", program.name);
        let linked = linked_program(&modules, &lifecycle.module);
        let props = lifecycle
            .props
            .iter()
            .map(|(name, value)| (name.clone(), nx_value(value)))
            .collect::<BTreeMap<_, _>>();
        let initialize = |options: &RuntimeOptions| {
            linked.initialize_component(&lifecycle.component, &props, &ComponentInit::default(), options)
        };
        let initial = count_operations(&label, initialize);
        let mut instance = initialize(&RuntimeOptions::default())
            .expect("initializes")
            .instance;
        let mut batches = Vec::with_capacity(lifecycle.batches.len());
        for (index, batch) in lifecycle.batches.iter().enumerate() {
            let entries = batch.iter().map(nx_value).collect::<Vec<_>>();
            let dispatch =
                |options: &RuntimeOptions| linked.dispatch_component_actions(&instance, &entries, options);
            batches.push(count_operations(&format!("{label} batch {index}"), dispatch));
            instance = dispatch(&RuntimeOptions::default())
                .expect("dispatches")
                .instance;
        }
        counts.insert(
            key,
            serde_json::json!({ "initial": initial, "batches": batches }),
        );
    }
    let mut operations = serde_json::json!({ "counts": counts });
    if !failures.is_empty() {
        operations["failures"] = Value::Object(failures);
    }
    operations
}

/// The numbers of two `operations.json` files that differ, each named by its program and key, so a
/// failure says which evaluation's count moved.
fn operation_differences(program: &str, expected: &Value, actual: &Value) -> Vec<String> {
    let mut differences = Vec::new();
    let mut walk = |path: String, expected: Option<&Value>, actual: Option<&Value>| {
        if expected != actual {
            differences.push(format!(
                "{program}: {path}: recorded {}, the Rust runtime gives {}",
                expected.map_or("nothing".to_string(), Value::to_string),
                actual.map_or("nothing".to_string(), Value::to_string)
            ));
        }
    };
    for section in ["counts", "failures"] {
        let keys = [expected, actual]
            .iter()
            .filter_map(|file| file.get(section).and_then(Value::as_object))
            .flat_map(|entries| entries.keys().cloned())
            .collect::<BTreeSet<_>>();
        for key in keys {
            walk(
                format!("{section} {key}"),
                expected.get(section).and_then(|entries| entries.get(&key)),
                actual.get(section).and_then(|entries| entries.get(&key)),
            );
        }
    }
    differences
}

/// Every corpus evaluation's operation count, and the recorded failures, are what the Rust runtime
/// computes; regeneration writes them.
#[test]
fn corpus_operation_counts_match_the_rust_runtime() {
    let mut failures = Vec::new();
    for program in load_programs() {
        let actual = measure_operations(&program);
        let path = program.dir.join("expected").join("operations.json");
        if updating() {
            let text = format!(
                "{}\n",
                serde_json::to_string_pretty(&actual).expect("operations")
            );
            write_expected(&path, text.as_bytes());
            continue;
        }
        let expected = fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .unwrap_or(Value::Null);
        failures.extend(operation_differences(&program.name, &expected, &actual));
    }
    assert!(
        failures.is_empty(),
        "corpus operation counts changed (run with NX_UPDATE_CORPUS=1 after an intended change):\n{}",
        failures.join("\n")
    );
}

/// The kinds an artifact uses, by table.
fn coverage_of(artifact: &NxIrArtifact) -> BTreeMap<&'static str, BTreeSet<String>> {
    let mut coverage: BTreeMap<&'static str, BTreeSet<String>> = BTreeMap::new();
    let mut note = |table: &'static str, names: &[(i64, &'static str)], kind: i64| {
        let name = kinds::name(names, kind)
            .map(str::to_string)
            .unwrap_or_else(|| panic!("{table} kind {kind} is not one the schema assigns"));
        coverage.entry(table).or_default().insert(name);
    };
    let first = |entry: &nx_ir::IrItem, position: usize| -> i64 {
        entry
            .as_list()
            .and_then(|items| items.get(position))
            .and_then(|item| item.as_int())
            .unwrap_or(-1)
    };
    for node in &artifact.nodes {
        let kind = first(node, 0);
        note("nodes", kinds::node::NAMES, kind);
        if kind == kinds::node::BINARY {
            note("binaryOperators", kinds::binary::NAMES, first(node, 1));
        }
        if kind == kinds::node::UNARY {
            note("unaryOperators", kinds::unary::NAMES, first(node, 1));
        }
        if kind == kinds::node::INTRINSIC {
            note("intrinsics", kinds::intrinsic::NAMES, first(node, 1));
        }
    }
    for ty in &artifact.types {
        note("types", kinds::ty::NAMES, first(ty, 0));
    }
    for constant in &artifact.constants {
        note("constants", kinds::constant::NAMES, first(constant, 0));
    }
    for declaration in &artifact.declarations {
        note(
            "declarations",
            kinds::declaration::NAMES,
            first(declaration, 0),
        );
    }
    coverage
}

#[test]
fn corpus_covers_every_kind() {
    let tables: [(&str, &[(i64, &str)]); 7] = [
        ("nodes", kinds::node::NAMES),
        ("types", kinds::ty::NAMES),
        ("constants", kinds::constant::NAMES),
        ("declarations", kinds::declaration::NAMES),
        ("binaryOperators", kinds::binary::NAMES),
        ("unaryOperators", kinds::unary::NAMES),
        ("intrinsics", kinds::intrinsic::NAMES),
    ];
    let mut coverage: BTreeMap<&str, BTreeMap<String, BTreeSet<String>>> = tables
        .iter()
        .map(|(table, names)| {
            (
                *table,
                names
                    .iter()
                    .map(|(_, name)| (name.to_string(), BTreeSet::new()))
                    .collect(),
            )
        })
        .collect();
    let programs = load_programs();
    for program in &programs {
        for artifact in emit(program, false) {
            for (table, names) in coverage_of(&artifact) {
                for name in names {
                    coverage
                        .get_mut(table)
                        .expect("known table")
                        .get_mut(&name)
                        .expect("known kind")
                        .insert(program.name.clone());
                }
            }
        }
    }

    let uncovered = coverage
        .iter()
        .flat_map(|(table, names)| {
            names
                .iter()
                .filter(|(_, programs)| programs.is_empty())
                .map(move |(name, _)| format!("{table}.{name}"))
        })
        .collect::<Vec<_>>();
    assert!(
        uncovered.is_empty(),
        "no corpus program covers: {}",
        uncovered.join(", ")
    );

    let manifest = serde_json::json!({
        "programs": programs.iter().map(|program| program.name.clone()).collect::<Vec<_>>(),
        "coverage": coverage,
    });
    let manifest = format!(
        "{}\n",
        serde_json::to_string_pretty(&manifest).expect("manifest")
    );
    let path = corpus_root().join("manifest.json");
    if updating() {
        write_expected(&path, manifest.as_bytes());
        return;
    }
    let expected = fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        expected,
        manifest,
        "the corpus manifest at {} is out of date; run with NX_UPDATE_CORPUS=1",
        path.display()
    );
}

#[test]
fn corpus_artifacts_fit_the_size_budget() {
    let mut over_budget = Vec::new();
    for program in load_programs() {
        for artifact in emit(&program, false) {
            let identity = artifact.modules[0].identity.clone();
            let image = write_nx_ir_image(&artifact).expect("image");
            // A standard library module is not one of the workspace's files; its source is the
            // text the compiler carries, which the program artifact holds.
            let source_len = program
                .sources
                .get(&identity)
                .map(String::as_str)
                .or_else(|| program.artifact.source_text(&identity))
                .unwrap_or_else(|| panic!("{}/{identity}: no source", program.name))
                .len();
            let ratio = image.len() as f64 / source_len as f64;
            println!(
                "{}/{}: {} bytes of IR for {} bytes of source, {ratio:.2}x",
                program.name,
                identity,
                image.len(),
                source_len
            );
            if ratio > SIZE_BUDGET {
                over_budget.push(format!("{}/{}: {ratio:.2}x", program.name, identity));
            }
        }
    }
    assert!(
        over_budget.is_empty(),
        "artifacts over the {SIZE_BUDGET}x budget: {}",
        over_budget.join(", ")
    );
}

/// Whether generated-code diagnostics refuse a construct executable source codegen does not
/// support, rather than report a failure of the emitter.
fn is_unsupported_by_source_codegen(message: &str) -> bool {
    message.contains("not supported")
}

/// Two canonical values, with numbers compared by value: generated JavaScript prints `2.0` as `2`.
///
/// <para>A function value in generated JavaScript is the JavaScript function itself, which knows
/// its name and not its module, so the harness prints it as a `Function` record without `module`
/// and it agrees with the recorded record of that name.</para>
fn canonical_values_agree(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Object(left), Value::Object(right))
            if left.get("$type").and_then(Value::as_str) == Some("Function")
                && !left.contains_key("module") =>
        {
            right.get("$type").and_then(Value::as_str) == Some("Function")
                && left.get("name") == right.get("name")
        }
        (Value::Number(left), Value::Number(right)) => left.as_f64() == right.as_f64(),
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| canonical_values_agree(left, right))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left.iter().all(|(key, value)| {
                    right
                        .get(key)
                        .is_some_and(|other| canonical_values_agree(value, other))
                })
        }
        _ => left == right,
    }
}

/// A generated entrypoint's result against the recorded one. A generated function is typed NX code,
/// not a host boundary: an empty `T?` result is `nxEmpty`, as its TypeScript signature says, where
/// an entry call through a runtime returns the host's `null`.
fn entry_results_agree(generated: &Value, recorded: &Value) -> bool {
    match (generated, recorded) {
        (Value::Array(items), Value::Null) => items.is_empty(),
        _ => canonical_values_agree(generated, recorded),
    }
}

/// The third engine: generated JavaScript run over every corpus entrypoint of an entry module
/// must evaluate to the result the interpreter recorded, which the IR runtime is held to as well.
///
/// <para>Executable source codegen refuses some constructs outright — match expressions, calls of
/// function-typed values, action handlers — so a program it refuses, and an entrypoint that
/// reaches an unsupported construct at run time, are skipped, and only then. Every refusal must be
/// one of those; any other failure to generate or to run fails the test.</para>
#[test]
fn generated_javascript_agrees_with_the_recorded_results() {
    let mut failures = Vec::new();
    let mut compared = 0;
    let mut skipped = Vec::new();
    for program in load_programs() {
        let output =
            match crate::emit_program(&program.artifact, &crate::CodegenOptions::javascript()) {
                Ok(output) => output,
                Err(error) => {
                    let unexpected = error
                        .diagnostics
                        .iter()
                        .filter(|diagnostic| {
                            !is_unsupported_by_source_codegen(diagnostic.message())
                        })
                        .map(|diagnostic| diagnostic.message().to_string())
                        .collect::<Vec<_>>();
                    if !unexpected.is_empty() {
                        failures.push(format!(
                            "{}: JavaScript generation failed: {unexpected:?}",
                            program.name
                        ));
                    }
                    skipped.push(program.name.clone());
                    continue;
                }
            };
        let entrypoints = program
            .manifest
            .entrypoints
            .iter()
            .filter(|entrypoint| entrypoint.module == program.manifest.entry)
            .collect::<Vec<_>>();
        let dir = tempfile::TempDir::new().expect("temp dir");
        fs::write(dir.path().join("package.json"), r#"{ "type": "module" }"#)
            .expect("package file");
        for file in output.files {
            fs::write(dir.path().join(file.relative_path), file.content).expect("generated file");
        }
        let names = entrypoints
            .iter()
            .map(|entrypoint| serde_json::to_string(&entrypoint.function).expect("name"))
            .collect::<Vec<_>>()
            .join(", ");
        let script = format!(
            "const m = await import({:?});\n\
             const out = {{}};\n\
             for (const name of [{names}]) {{\n\
               try {{ out[name] = {{ ok: m[name]() }}; }}\n\
               catch (error) {{ out[name] = {{ error: String(error && error.message || error) }}; }}\n\
             }}\n\
             const fn = (key, value) => typeof value === 'function'\n\
               ? {{ $type: 'Function', name: value.name }} : value;\n\
             console.log(JSON.stringify(out, fn));",
            format!("file://{}", dir.path().join("index.js").display())
        );
        let run = std::process::Command::new("node")
            .args(["--input-type=module", "--eval", &script])
            .output()
            .expect("node is required to run generated JavaScript");
        assert!(
            run.status.success(),
            "{}: node failed: {}",
            program.name,
            String::from_utf8_lossy(&run.stderr)
        );
        let actual: BTreeMap<String, Value> =
            serde_json::from_slice(&run.stdout).expect("generated results");
        let expected: BTreeMap<String, Value> = serde_json::from_str(
            &fs::read_to_string(program.dir.join("expected").join("results.json"))
                .expect("recorded results"),
        )
        .expect("recorded results");
        for entrypoint in entrypoints {
            let key = format!("{}::{}", entrypoint.module, entrypoint.function);
            let outcome = &actual[&entrypoint.function];
            match (outcome.get("ok"), outcome.get("error")) {
                (Some(value), _) => {
                    compared += 1;
                    if !entry_results_agree(value, &expected[&key]) {
                        failures.push(format!(
                            "{}: {key}: generated JavaScript gave {value}, the interpreter {}",
                            program.name, expected[&key]
                        ));
                    }
                }
                (None, Some(Value::String(message)))
                    if is_unsupported_by_source_codegen(message) =>
                {
                    skipped.push(key);
                }
                _ => failures.push(format!(
                    "{}: {key}: generated JavaScript failed: {outcome}",
                    program.name
                )),
            }
        }
    }
    println!("{compared} corpus entrypoints compared; skipped as unsupported: {skipped:?}");
    assert!(
        compared > 0,
        "no corpus entrypoint ran as generated JavaScript"
    );
    assert!(
        failures.is_empty(),
        "generated JavaScript disagrees with the corpus:\n{}",
        failures.join("\n")
    );
}
