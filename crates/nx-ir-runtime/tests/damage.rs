//! Damage: every corpus image cut at every four-byte boundary, and every cell of every image
//! of a program whose images are all within [`SWEPT_IMAGE_LIMIT`] overwritten, must be refused
//! with a diagnostic or read as a valid image. A damaged image that prepares is then linked and
//! run as the corpus runs it, entrypoints and lifecycles both, since a stranger's image is
//! evaluated, not just opened. A panic anywhere fails the test.

mod common;

use common::{load_corpus, prepare_all, CorpusProgram};
use nx_ir_runtime::{
    ComponentInit, LinkOptions, NxIrRuntimeError, PreparedModule, Program, RuntimeOptions,
};
use std::collections::BTreeMap;

/// The size, in bytes, above which an image takes its program out of the sweep that overwrites
/// every cell. Overwriting a cell links and runs the whole program again, so the sweep grows with
/// the square of a program's size; the corpus's large programs (`specs/ir-conformance/README.md`)
/// have an image above this and are cut at every boundary only.
const SWEPT_IMAGE_LIMIT: usize = 16 * 1024;

/// The programs the sweep leaves out. A program that grows past the limit fails the test until
/// it is named here, so that none leaves the sweep unnoticed.
const LEFT_OUT_OF_THE_SWEEP: [&str; 2] = ["large-catalog", "question-flow"];

#[test]
fn every_truncated_corpus_image_is_refused() {
    let mut refused = 0;
    for program in load_corpus() {
        for (identity, image) in program.images.iter().chain(&program.stripped_images) {
            for end in (0..image.len()).step_by(4) {
                assert!(
                    PreparedModule::prepare(&image[..end]).is_err(),
                    "{}/{identity} cut at {end} of {} bytes prepared",
                    program.name,
                    image.len()
                );
                refused += 1;
            }
        }
    }
    assert!(refused > 0);
}

/// Runs everything the corpus runs against `program`: each function entrypoint, with its
/// arguments when it is a case, and each lifecycle with its batches and a round trip of its
/// instance through the serialized form. Returns how many operations succeeded and how many
/// failed with a diagnostic.
///
/// <para>A damaged cell can turn a loop's bound or a call's target into anything, so every run
/// has a budget: ample for the corpus, whose evaluations cost hundreds of operations, and small
/// enough that a damaged image which loops ends quickly, in a diagnostic. An input limit is set
/// beside it, ample for what the corpus passes, so the measuring pass runs against every damaged
/// image too.</para>
fn drive(linked: &Program, program: &CorpusProgram, identity: &str) -> (usize, usize) {
    let options = RuntimeOptions {
        max_operations: Some(10_000),
        max_input_size: Some(10_000),
        ..RuntimeOptions::default()
    };
    let (mut succeeded, mut failed) = (0, 0);
    let mut count = |error: Option<NxIrRuntimeError>| match error {
        None => succeeded += 1,
        Some(error) => {
            assert!(!error.diagnostics.is_empty());
            failed += 1;
        }
    };
    for entrypoint in &program.entrypoints {
        if entrypoint.module == identity {
            count(
                linked
                    .evaluate_function(&entrypoint.function, &entrypoint.arguments, &options)
                    .err(),
            );
        }
    }
    for lifecycle in &program.lifecycles {
        if lifecycle.module != identity {
            continue;
        }
        count(
            linked
                .evaluate_component(
                    &lifecycle.component,
                    &lifecycle.props,
                    &BTreeMap::new(),
                    &options,
                )
                .err(),
        );
        let initialized = linked.initialize_component(
            &lifecycle.component,
            &lifecycle.props,
            &ComponentInit::default(),
            &options,
        );
        let mut instance = match initialized {
            Ok(initialized) => initialized.instance,
            Err(error) => {
                count(Some(error));
                continue;
            }
        };
        for batch in &lifecycle.batches {
            let stored = serde_json::to_value(&instance).expect("an instance serializes");
            count(linked.restore_component_instance(stored).err());
            match linked.dispatch_component_actions(&instance, batch, &options) {
                Ok(dispatched) => {
                    count(None);
                    instance = dispatched.instance;
                }
                Err(error) => count(Some(error)),
            }
        }
    }
    (succeeded, failed)
}

#[test]
fn every_cell_of_every_corpus_image_can_be_overwritten_without_a_panic() {
    let (mut opened, mut refused, mut unlinked, mut succeeded, mut failed) = (0, 0, 0, 0, 0);
    let (mut swept, mut left_out) = (0, Vec::new());
    for program in load_corpus() {
        let images = || program.images.iter().chain(&program.stripped_images);
        let largest = images().map(|(_, image)| image.len()).max().unwrap_or(0);
        if largest > SWEPT_IMAGE_LIMIT {
            left_out.push((program.name, largest));
            continue;
        }
        swept += 1;
        let intact = prepare_all(&program.stripped_images);
        for (identity, image) in images() {
            for offset in (0..image.len()).step_by(4) {
                for value in [0u32, 1, 0xffff_ffff, 0x7fff_fff0] {
                    let mut damaged = image.clone();
                    damaged[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
                    let Ok(module) = PreparedModule::prepare(damaged) else {
                        refused += 1;
                        continue;
                    };
                    opened += 1;
                    let Ok(linked) = Program::link(
                        &module,
                        |identity| intact.get(identity).cloned(),
                        &LinkOptions::default(),
                    ) else {
                        unlinked += 1;
                        continue;
                    };
                    let (ok, err) = drive(&linked, &program, identity);
                    succeeded += ok;
                    failed += err;
                }
            }
        }
    }
    assert!(
        opened > 0 && refused > 0 && succeeded > 0 && failed > 0,
        "{opened} opened, {refused} refused, {succeeded} succeeded, {failed} failed"
    );
    println!(
        "{swept} programs swept: {opened} damaged images opened ({unlinked} did not link), {refused} refused; {succeeded} operations succeeded, {failed} failed with a diagnostic"
    );
    assert_eq!(
        left_out
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        LEFT_OUT_OF_THE_SWEEP,
        "the programs with an image above {SWEPT_IMAGE_LIMIT} bytes, which the sweep leaves out, are not the ones named: {left_out:?}"
    );
    println!("left out, for an image above {SWEPT_IMAGE_LIMIT} bytes: {left_out:?}");
}
