//! The conformance corpus, from the Rust runtime's side: every image in `specs/ir-conformance`
//! is prepared, linked where its module table requires, and evaluated, and each named
//! entrypoint's canonical value must equal the recorded one. Each lifecycle is initialized and
//! its batches dispatched in order, and every rendered output, tokens included, and every effect
//! list must equal what was recorded. Every evaluation must cost exactly the operations recorded
//! for it, and stop where the recorded failures say a smaller budget stops it.

mod common;

use common::{canonical_eq, json, link, load_corpus, prepare_all, value};
use nx_ir_runtime::{ComponentInit, NxIrRuntimeError, RuntimeOptions};
use nx_value::NxValue;
use std::fmt::Debug;

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

#[test]
fn every_corpus_entrypoint_and_lifecycle_matches_its_recorded_result() {
    let options = RuntimeOptions::default();
    let mut failures = Vec::new();
    let mut checked = 0;
    for program in load_corpus() {
        for (variant, images) in [
            ("with debug", &program.images),
            ("stripped", &program.stripped_images),
        ] {
            let modules = prepare_all(images);
            for entrypoint in &program.entrypoints {
                let label = format!(
                    "{} ({variant}) {}::{}",
                    program.name, entrypoint.module, entrypoint.function
                );
                let expected = value(
                    &program.results[format!("{}::{}", entrypoint.module, entrypoint.function)],
                );
                checked += 1;
                let linked = link(&modules, &entrypoint.module);
                let key = format!("{}::{}", entrypoint.module, entrypoint.function);
                if let Some(problem) = check_count(&program.operations["counts"][&key], |options| {
                    linked.evaluate_function(&entrypoint.function, &[], options)
                }) {
                    failures.push(format!("{label}: {problem}"));
                }
                match linked.evaluate_function(&entrypoint.function, &[], &options) {
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
                let linked = link(&modules, &lifecycle.module);
                if let Some(problem) = check_count(&counts["initial"], |options| {
                    linked.initialize_component(
                        &lifecycle.component,
                        &lifecycle.props,
                        &ComponentInit::default(),
                        options,
                    )
                }) {
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
                let initialized = match linked.initialize_component(
                    &lifecycle.component,
                    &lifecycle.props,
                    &ComponentInit::default(),
                    &options,
                ) {
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
                    if let Some(problem) = check_count(&counts["batches"][index], |options| {
                        linked.dispatch_component_actions(&instance, batch, options)
                    }) {
                        count_problems.push(format!("{label}: batch {index} {problem}"));
                    }
                    match linked.dispatch_component_actions(&instance, batch, &options) {
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
        let Some(recorded) = program.operations.get("failures").and_then(|f| f.as_object()) else {
            continue;
        };
        let modules = prepare_all(&program.images);
        for (key, records) in recorded {
            let (module, function) = key.split_once("::").expect("a key is identity::function");
            let linked = link(&modules, module);
            for record in records.as_array().expect("a list of failures") {
                checked += 1;
                let operations = record["budget"].as_u64().expect("a budget");
                let label = format!("{} {key} under {operations} operations", program.name);
                let error = match linked.evaluate_function(function, &[], &budget(operations)) {
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
