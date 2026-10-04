//! The conformance corpus, from the Rust runtime's side: every image in `specs/ir-conformance`
//! is prepared, linked where its module table requires, and evaluated, and each named
//! entrypoint's canonical value must equal the recorded one. Each lifecycle is initialized and
//! its batches dispatched in order, and every rendered output, tokens included, and every effect
//! list must equal what was recorded. An entrypoint that names arguments, a *case*, is evaluated
//! with them. Every evaluation must cost exactly the operations recorded for it, stop where the
//! recorded failures say a smaller budget stops it, and have exactly the input size recorded for
//! it, where one is; and the usage report must give the recorded numbers.

mod common;

use common::{canonical_eq, json, link, load_corpus, prepare_all, value};
use nx_ir_runtime::{ComponentInit, NxIrRuntimeError, RuntimeOptions, Usage};
use nx_value::NxValue;
use std::fmt::Debug;
use std::sync::Arc;

fn budget(operations: u64) -> RuntimeOptions {
    RuntimeOptions {
        max_operations: Some(operations),
        ..RuntimeOptions::default()
    }
}

/// Checks that an evaluation costs exactly the `recorded` count: under that budget it succeeds,
/// and under one less it fails on the budget. Returns what is wrong, if anything.
fn check_count<T: Debug>(
    recorded: &serde_json::Value,
    run: impl Fn(&RuntimeOptions) -> Result<T, NxIrRuntimeError>,
) -> Option<String> {
    let Some(operations) = recorded.as_u64() else {
        return Some(format!("no operation count is recorded (found {recorded})"));
    };
    if let Err(error) = run(&budget(operations)) {
        return Some(format!(
            "fails under its recorded count of {operations} operations: {error}"
        ));
    }
    if operations == 0 {
        return None;
    }
    match run(&budget(operations - 1)) {
        Err(error) if error.diagnostics[0].limit.map(|limit| limit.name) == Some("maxOperations") => {
            None
        }
        Err(error) => Some(format!(
            "under {} operations, one less than recorded, fails otherwise than on the budget: {error}",
            operations - 1
        )),
        Ok(_) => Some(format!(
            "succeeds under {} operations, one less than recorded",
            operations - 1
        )),
    }
}

/// The options an evaluation is compared with its recorded result under: an input limit equal to
/// its recorded input size where one is recorded, so that the size is shown to admit the call and
/// yield the result, and no limit otherwise.
fn under_recorded_size(recorded: &serde_json::Value) -> RuntimeOptions {
    RuntimeOptions {
        max_input_size: recorded.as_u64(),
        ..RuntimeOptions::default()
    }
}

/// Checks that an evaluation's input has exactly the `recorded` size, where one is recorded: the
/// call is refused for its input under a limit one less. That it proceeds under the size itself
/// is checked where its result is compared.
fn check_input_size<T: Debug>(
    recorded: &serde_json::Value,
    run: impl Fn(&RuntimeOptions) -> Result<T, NxIrRuntimeError>,
) -> Option<String> {
    let size = recorded.as_u64()?;
    if size == 0 {
        return None;
    }
    let limited = RuntimeOptions {
        max_input_size: Some(size - 1),
        ..RuntimeOptions::default()
    };
    match run(&limited) {
        Err(error) if error.diagnostics[0].limit.map(|limit| limit.name) == Some("maxInputSize") => {
            None
        }
        Err(error) => Some(format!(
            "under an input limit of {}, one less than its recorded input size, fails otherwise than on the limit: {error}",
            size - 1
        )),
        Ok(_) => Some(format!(
            "proceeds under an input limit of {}, one less than its recorded input size",
            size - 1
        )),
    }
}

/// Checks that the usage report of an evaluation gives its `recorded` operation count and, where
/// one is recorded, its input size, under a budget and a limit it cannot reach.
fn check_usage<T: Debug>(
    count: &serde_json::Value,
    input_size: &serde_json::Value,
    run: impl Fn(&RuntimeOptions) -> Result<T, NxIrRuntimeError>,
) -> Option<String> {
    let usage = Arc::new(Usage::new());
    let options = RuntimeOptions {
        max_operations: Some(1 << 40),
        max_input_size: Some(1 << 40),
        usage: Some(Arc::clone(&usage)),
        ..RuntimeOptions::default()
    };
    if let Err(error) = run(&options) {
        return Some(format!("fails with a usage report asked for: {error}"));
    }
    if usage.operations() != count.as_u64() {
        return Some(format!(
            "the usage report gives {:?} operations, and {count} are recorded",
            usage.operations()
        ));
    }
    if !input_size.is_null() && usage.input_size() != input_size.as_u64() {
        return Some(format!(
            "the usage report gives an input size of {:?}, and {input_size} is recorded",
            usage.input_size()
        ));
    }
    None
}

#[test]
fn every_corpus_entrypoint_and_lifecycle_matches_its_recorded_result() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for program in load_corpus() {
        for (variant, images) in [
            ("with debug", &program.images),
            ("stripped", &program.stripped_images),
        ] {
            let modules = prepare_all(images);
            for entrypoint in &program.entrypoints {
                let key = entrypoint.key();
                let label = format!("{} ({variant}) {key}", program.name);
                let expected = value(&program.results[&key]);
                checked += 1;
                let linked = link(&modules, &entrypoint.module);
                let run = |options: &RuntimeOptions| {
                    linked.evaluate_function(&entrypoint.function, &entrypoint.arguments, options)
                };
                let count = &program.operations["counts"][&key];
                let input_size = &program.operations["inputSizes"][&key];
                for problem in [
                    check_count(count, run),
                    check_input_size(input_size, run),
                    check_usage(count, input_size, run),
                ]
                .into_iter()
                .flatten()
                {
                    failures.push(format!("{label}: {problem}"));
                }
                match run(&under_recorded_size(input_size)) {
                    Ok(actual) if canonical_eq(&actual, &expected) => {}
                    Ok(actual) => failures.push(format!(
                        "{label}: expected {}, got {}",
                        json(&expected),
                        json(&actual)
                    )),
                    Err(error) => failures.push(format!("{label}: {error}")),
                }
            }
            for lifecycle in &program.lifecycles {
                let label = format!(
                    "{} ({variant}) {}::{} lifecycle",
                    program.name, lifecycle.module, lifecycle.component
                );
                let key = format!("{}::{}", lifecycle.module, lifecycle.component);
                let expected = &program.results[&key];
                let counts = &program.operations["counts"][&key];
                let input_sizes = &program.operations["inputSizes"][&key];
                let linked = link(&modules, &lifecycle.module);
                let initialize = |options: &RuntimeOptions| {
                    linked.initialize_component(
                        &lifecycle.component,
                        &lifecycle.props,
                        &ComponentInit::default(),
                        options,
                    )
                };
                for problem in [
                    check_count(&counts["initial"], initialize),
                    check_input_size(&input_sizes["initial"], initialize),
                    check_usage(&counts["initial"], &input_sizes["initial"], initialize),
                ]
                .into_iter()
                .flatten()
                {
                    failures.push(format!("{label}: initialization {problem}"));
                }
                checked += 1;
                let mut count_problems = Vec::new();
                let mut expect = |what: String, actual: &NxValue, wanted: &serde_json::Value| {
                    let wanted = value(wanted);
                    if !canonical_eq(actual, &wanted) {
                        failures.push(format!(
                            "{label}: {what}: expected {}, got {}",
                            json(&wanted),
                            json(actual)
                        ));
                    }
                };
                let initialized = match initialize(&under_recorded_size(&input_sizes["initial"])) {
                    Ok(initialized) => initialized,
                    Err(error) => {
                        failures.push(format!("{label}: {error}"));
                        continue;
                    }
                };
                expect(
                    "initial rendered output".to_string(),
                    &initialized.rendered,
                    &expected["initial"],
                );
                let mut instance = initialized.instance;
                for (index, batch) in lifecycle.batches.iter().enumerate() {
                    let dispatch = |options: &RuntimeOptions| {
                        linked.dispatch_component_actions(&instance, batch, options)
                    };
                    let count = &counts["batches"][index];
                    let input_size = &input_sizes["batches"][index];
                    for problem in [
                        check_count(count, dispatch),
                        check_input_size(input_size, dispatch),
                        check_usage(count, input_size, dispatch),
                    ]
                    .into_iter()
                    .flatten()
                    {
                        count_problems.push(format!("{label}: batch {index} {problem}"));
                    }
                    match dispatch(&under_recorded_size(input_size)) {
                        Ok(dispatched) => {
                            let recorded = &expected["batches"][index];
                            expect(
                                format!("batch {index} rendered output"),
                                &dispatched.rendered,
                                &recorded["rendered"],
                            );
                            expect(
                                format!("batch {index} effects"),
                                &NxValue::Array(dispatched.effects),
                                &recorded["effects"],
                            );
                            instance = dispatched.instance;
                        }
                        Err(error) => {
                            failures.push(format!("{label}: batch {index}: {error}"));
                            break;
                        }
                    }
                }
                failures.extend(count_problems);
            }
        }
    }
    assert!(checked > 0, "the corpus is empty");
    assert!(
        failures.is_empty(),
        "{} problems in {checked} evaluations:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// A budget below an evaluation's count stops it at the node the corpus records: the same
/// declaration and, from the image with its debug section, the same span. Equal counts alone do
/// not show that two runtimes charge in the same order; these do.
#[test]
fn every_recorded_failure_stops_at_its_recorded_node() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for program in load_corpus() {
        let Some(recorded) = program
            .operations
            .get("failures")
            .and_then(|f| f.as_object())
        else {
            continue;
        };
        let modules = prepare_all(&program.images);
        for (key, records) in recorded {
            // A key is `identity::function`, or `identity::function#case` for a case, whose
            // arguments are the entrypoint's: the entrypoint is found by its key, not by parsing.
            let entrypoint = program
                .entrypoints
                .iter()
                .find(|entrypoint| entrypoint.key() == *key)
                .unwrap_or_else(|| {
                    panic!(
                        "{}: failures are recorded for {key}, which is no entrypoint",
                        program.name
                    )
                });
            let linked = link(&modules, &entrypoint.module);
            for record in records.as_array().expect("a list of failures") {
                checked += 1;
                let operations = record["budget"].as_u64().expect("a budget");
                let label = format!("{} {key} under {operations} operations", program.name);
                let error = match linked.evaluate_function(
                    &entrypoint.function,
                    &entrypoint.arguments,
                    &budget(operations),
                ) {
                    Ok(_) => {
                        failures.push(format!("{label}: succeeds"));
                        continue;
                    }
                    Err(error) => error,
                };
                let diagnostic = &error.diagnostics[0];
                let actual = serde_json::json!({
                    "budget": operations,
                    "declaration": diagnostic.declaration,
                    "limit": diagnostic.limit.map(|limit| limit.name),
                    "source": diagnostic.source.as_ref().map(|source| serde_json::json!({
                        "identity": source.identity,
                        "start": source.start,
                        "end": source.end,
                    })),
                });
                let mut expected = record.clone();
                expected["limit"] = "maxOperations".into();
                if expected.get("source").is_none() {
                    expected["source"] = serde_json::Value::Null;
                }
                if actual != expected {
                    failures.push(format!("{label}: expected {expected}, got {actual}"));
                }
            }
        }
    }
    assert!(checked > 0, "the corpus records no failures");
    assert!(
        failures.is_empty(),
        "{} of {checked} failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
