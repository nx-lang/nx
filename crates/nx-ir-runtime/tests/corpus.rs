//! The conformance corpus, from the Rust runtime's side: every image in `specs/ir-conformance`
//! is prepared, linked where its module table requires, and evaluated, and each named
//! entrypoint's canonical value must equal the recorded one. Each lifecycle is initialized and
//! its batches dispatched in order, and every rendered output, tokens included, and every effect
//! list must equal what was recorded.

mod common;

use common::{canonical_eq, json, link, load_corpus, prepare_all, value};
use nx_ir_runtime::{ComponentInit, RuntimeOptions};
use nx_value::NxValue;

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
                match link(&modules, &entrypoint.module).evaluate_function(
                    &entrypoint.function,
                    &[],
                    &options,
                ) {
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
                let expected =
                    &program.results[format!("{}::{}", lifecycle.module, lifecycle.component)];
                let linked = link(&modules, &lifecycle.module);
                checked += 1;
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
            }
        }
    }
    assert!(checked > 0, "the corpus is empty");
    assert!(
        failures.is_empty(),
        "{} of {checked} failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
