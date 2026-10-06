//! The differential test of evaluation cost: generated host values, run through the probe library
//! in the Rust runtime and in the TypeScript runtime, must give the same result, the same
//! operation count and input size, and, under budgets below the count, the same failure.
//!
//! <para>The cases are generated here from a seed and given to both runtimes: the Rust runtime in
//! this process, and the TypeScript runtime through one `node` process that runs
//! `runtime/typescript/test/cost-runner.mjs` over the committed `dist`. A difference fails the
//! test naming the seed, the probe and the case, after the case has been shrunk by size. The seed
//! and the number of cases are fixed, so a run is the same every time; `NX_COST_SEED` and
//! `NX_COST_CASES` override them for a longer search by hand:</para>
//!
//! ```text
//! NX_COST_SEED=7 NX_COST_CASES=40000 cargo test --release -p nx-codegen --test cost_differential
//! ```
//!
//! <para>The time report, which compares how a runtime's time grows with how its count grows, is
//! the ignored test of this file: timing is noisy, so it runs as a command of its own, in an
//! optimized build, and does not block a merge.</para>
//!
//! ```text
//! cargo test --release -p nx-codegen --test cost_differential -- --ignored --nocapture time_report
//! ```

mod cost;

use cost::{
    answer, case_count, covered_by, generate, known_to_the_time_report, link, measuring,
    probe_images, seed, stale_finding, written_case, Call, Case, Check, Finding, Gaps, Grow,
    Runtime, Second, Sizes, Spec, TypeName, BOUNDARY_SIZES, KNOWN_FINDINGS, PROBES, PROBE_IDENTITY,
    RESERVED, SCALE,
};
use nx_ir_runtime::Program;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

// ------------------------------------------------------------------------------------------------
// The TypeScript runtime, through the Node runner
// ------------------------------------------------------------------------------------------------

/// The probe library written where the Node runner reads it, and the runner's answers for cases.
///
/// <para>The files go to a temporary directory that is removed afterwards. A runner that is given
/// a place under `NX_COST_DIR`, when that variable names a directory, writes them there
/// instead, where they are kept: the images, and the cases and the answers of the last batch the
/// runner was given, for running the runner by hand. Each test that keeps its files has a place
/// of its own, since the tests of this file run at the same time: the differential test the
/// directory itself, and the time report `time` under it.</para>
struct Runner {
    dir: PathBuf,
    _temporary: Option<tempfile::TempDir>,
    images: Vec<Value>,
}

impl Runner {
    fn new(images: &[(String, Vec<u8>)], kept: Option<&str>) -> Runner {
        let kept = kept
            .and_then(|place| Some(PathBuf::from(std::env::var_os("NX_COST_DIR")?).join(place)));
        let (dir, temporary) = match kept {
            Some(dir) => {
                std::fs::create_dir_all(&dir).expect("the directory NX_COST_DIR names");
                (dir, None)
            }
            None => {
                let temporary = tempfile::TempDir::new().expect("a temporary directory");
                (temporary.path().to_path_buf(), Some(temporary))
            }
        };
        let images = images
            .iter()
            .enumerate()
            .map(|(index, (identity, bytes))| {
                let path = dir.join(format!("{index}.nxir"));
                std::fs::write(&path, bytes).expect("an image file");
                json!({ "identity": identity, "path": path })
            })
            .collect();
        Runner {
            dir,
            _temporary: temporary,
            images,
        }
    }

    fn script() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtime/typescript/test/cost-runner.mjs")
    }

    /// Runs `cases` through the TypeScript runtime in one `node` process and returns its answers,
    /// one for each case in order.
    fn run(&self, mode: &str, cases: &[Value]) -> Vec<Value> {
        let input = self.dir.join("cases.json");
        let output = self.dir.join("answers.json");
        let request = json!({
            "entry": PROBE_IDENTITY,
            "images": self.images,
            "mode": mode,
            "cases": cases,
        });
        std::fs::write(&input, serde_json::to_vec(&request).expect("cases")).expect("a cases file");
        let run = std::process::Command::new("node")
            .arg(Self::script())
            .arg(&input)
            .arg(&output)
            .output()
            .expect("node is required to run the TypeScript runtime");
        assert!(
            run.status.success(),
            "the Node runner failed: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        let answers: Vec<Value> =
            serde_json::from_slice(&std::fs::read(&output).expect("the runner's answers"))
                .expect("the runner's answers");
        assert_eq!(answers.len(), cases.len(), "one answer for each case");
        answers
    }
}

// ------------------------------------------------------------------------------------------------
// Comparing two answers
// ------------------------------------------------------------------------------------------------

/// Canonical equality, as the corpus tests compare results: numbers by value, so `2.0` is `2`,
/// and records by their fields. With `nulls_as_empty`, a `null` is the empty list it is read as:
/// the documented difference for a `null` inside a value at `object`.
fn canonical_eq(left: &Value, right: &Value, nulls_as_empty: bool) -> bool {
    let empty = |value: &Value| match value {
        Value::Null => true,
        Value::Array(items) => items.is_empty(),
        _ => false,
    };
    if nulls_as_empty && empty(left) && empty(right) {
        return true;
    }
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left.as_f64() == right.as_f64(),
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| canonical_eq(left, right, nulls_as_empty))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left.iter().all(|(key, value)| {
                    right
                        .get(key)
                        .is_some_and(|other| canonical_eq(value, other, nulls_as_empty))
                })
        }
        _ => left == right,
    }
}

/// An answer in a few lines: a result is shown by its length when it is long.
fn brief(answer: &Value) -> String {
    let mut answer = answer.clone();
    if let Some(result) = answer.pointer_mut("/base/result") {
        let text = result.to_string();
        if text.len() > 300 {
            *result = json!(format!("<{} bytes of JSON>", text.len()));
        }
    }
    if let Some(Value::Array(failures)) = answer.pointer_mut("/base/failures") {
        if failures.len() > 6 {
            let count = failures.len();
            failures.truncate(6);
            failures.push(json!(format!("… {count} in all")));
        }
    }
    answer.to_string()
}

/// How the two runtimes' answers for a case differ, if they do.
fn difference(case: &Case, rust: &Value, typescript: &Value) -> Option<String> {
    // The code of a refusal with the argument its diagnostic names, when it names one.
    let refused = |answer: &Value| {
        let base = &answer["base"];
        base.get("refused").map(|code| match &base["argument"] {
            Value::Null => code.to_string(),
            argument => format!("{code} naming the argument {argument}"),
        })
    };
    match (refused(rust), refused(typescript)) {
        // A case both refuse with no budget has no count: the codes and the arguments the
        // diagnostics name are what is compared.
        (Some(rust), Some(typescript)) => {
            return (rust != typescript)
                .then(|| format!("refused with {rust} in Rust and {typescript} in TypeScript"));
        }
        (None, None) => {}
        (rust, typescript) => {
            return Some(format!(
                "one runtime refuses the case and the other does not: Rust {}, TypeScript {}",
                rust.map_or("accepts it".to_string(), |code| format!(
                    "refuses with {code}"
                )),
                typescript.map_or("accepts it".to_string(), |code| format!(
                    "refuses with {code}"
                )),
            ));
        }
    }
    for (what, pointer) in [
        ("the operation count at the base scale", "/base/operations"),
        ("the input size at the base scale", "/base/inputSize"),
        (
            "the count and the input size at the larger scale",
            "/scaled",
        ),
        (
            "the failures under budgets below the count",
            "/base/failures",
        ),
    ] {
        let (rust, typescript) = (rust.pointer(pointer), typescript.pointer(pointer));
        if rust.is_none() || rust != typescript {
            // The first failure that differs says more than the whole lists.
            if let (Some(Value::Array(rust)), Some(Value::Array(typescript))) = (rust, typescript) {
                if let Some((rust, typescript)) = rust
                    .iter()
                    .zip(typescript)
                    .find(|(left, right)| left != right)
                {
                    return Some(format!(
                        "{what} differ: Rust {rust}, TypeScript {typescript}"
                    ));
                }
            }
            return Some(format!(
                "{what} differ: Rust {}, TypeScript {}",
                rust.map_or("nothing".to_string(), Value::to_string),
                typescript.map_or("nothing".to_string(), Value::to_string)
            ));
        }
    }
    for (name, answer) in [("Rust", rust), ("TypeScript", typescript)] {
        if answer["base"]["resultUnderCount"] != json!(true) {
            return Some(format!(
                "in {name} the result under a budget equal to the count is not the result under none"
            ));
        }
    }
    let (rust_result, typescript_result) = (&rust["base"]["result"], &typescript["base"]["result"]);
    if !canonical_eq(rust_result, typescript_result, case.results_may_differ()) {
        return Some("the results differ".to_string());
    }
    None
}

// ------------------------------------------------------------------------------------------------
// The differential run
// ------------------------------------------------------------------------------------------------

/// How many cases are given to the Node runner at a time.
const CHUNK: usize = 200;

struct Harness {
    program: Program,
    runner: Runner,
}

impl Harness {
    /// A harness whose runner keeps its files at `kept` under `NX_COST_DIR` when that is set,
    /// and in a temporary directory otherwise.
    fn new(kept: Option<&str>) -> Harness {
        let images = probe_images();
        Harness {
            program: link(&images),
            runner: Runner::new(&images, kept),
        }
    }

    /// Both runtimes' answers for `cases`, and how each pair differs.
    fn compare(&self, cases: &[Case]) -> Vec<(Value, Value, Option<String>)> {
        let rust: Vec<Value> = cases
            .iter()
            .map(|case| answer(&self.program, case))
            .collect();
        let typescript = self
            .runner
            .run("answers", &cases.iter().map(Case::json).collect::<Vec<_>>());
        cases
            .iter()
            .zip(rust.into_iter().zip(typescript))
            .map(|(case, (rust, typescript))| {
                let difference = difference(case, &rust, &typescript);
                (rust, typescript, difference)
            })
            .collect()
    }

    /// The smallest form of a differing case that still differs: the case is run again with
    /// its sizes divided by two, four and so on, and the last that differs is the one reported.
    /// This is cruder than a property-testing library's shrinking and needs no dependency in two
    /// languages; the report prints the whole case, which is what reproduces it by hand.
    fn shrink(&self, case: &Case) -> (Case, Value, Value, String) {
        let shrunk: Vec<Case> = (0..=12).map(|power| case.shrunk(1 << power)).collect();
        let compared = self.compare(&shrunk);
        shrunk
            .into_iter()
            .zip(compared)
            .filter_map(|(case, (rust, typescript, difference))| {
                difference.map(|difference| (case, rust, typescript, difference))
            })
            .next_back()
            .expect("the case differs at the sizes it was generated with")
    }

    /// The report of one difference: the seed, the probe, the case and both answers.
    fn report(&self, seed: u64, case: &Case) -> String {
        let (case, rust, typescript, difference) = self.shrink(case);
        let json = case.json().to_string();
        let shown = if json.len() <= 4000 {
            json
        } else {
            let path =
                std::env::temp_dir().join(format!("nx-cost-case-{seed:#x}-{}.json", case.id));
            let _ = std::fs::write(&path, &json);
            format!(
                "<{} bytes of JSON, written to {}>",
                json.len(),
                path.display()
            )
        };
        format!(
            "seed {seed:#x}, {}\n  {difference}\n  case: {shown}\n  Rust:       {}\n  TypeScript: {}\n  rerun with NX_COST_SEED={seed:#x} NX_COST_CASES={}",
            case.describe(),
            brief(&rust),
            brief(&typescript),
            case.id + 1,
        )
    }

    /// One differential run: `count` cases from `seed`, with the generated cases a finding of
    /// `findings` covers skipped and each finding's own case required to reproduce. Returns how
    /// many cases were compared and how many of them both runtimes refused, or what is wrong.
    fn run(&self, seed: u64, count: usize, findings: &[Finding]) -> Result<(usize, usize), String> {
        for finding in findings
            .iter()
            .filter(|finding| finding.check == Check::Differential)
        {
            let case = (finding.case)();
            if self.compare(std::slice::from_ref(&case))[0].2.is_none() {
                return Err(stale_finding(finding));
            }
        }
        let cases: Vec<Case> = generate(seed, count, Sizes::STANDARD)
            .into_iter()
            .filter(|case| covered_by(findings, Check::Differential, case).is_none())
            .collect();
        // The cases are compared a few hundred at a time, so that a long search holds only a part
        // of its values at once: a case is its values at both scales, written out.
        let mut refused = 0;
        let mut differing: Vec<&Case> = Vec::new();
        for cases in cases.chunks(CHUNK) {
            let compared = self.compare(cases);
            refused += compared
                .iter()
                .filter(|(rust, _, _)| rust["base"].get("refused").is_some())
                .count();
            differing.extend(
                cases
                    .iter()
                    .zip(&compared)
                    .filter(|(_, (_, _, difference))| difference.is_some())
                    .map(|(case, _)| case),
            );
        }
        if differing.is_empty() {
            return Ok((cases.len(), refused));
        }
        // Every probe that differs is reported once, by its first case.
        let mut probes = BTreeSet::new();
        let reports: Vec<String> = differing
            .iter()
            .filter(|case| probes.insert(case.probe.name))
            .take(8)
            .map(|case| self.report(seed, case))
            .collect();
        Err(format!(
            "{} of {} generated cases differ between the runtimes, in the probes {probes:?}:\n\n{}",
            differing.len(),
            cases.len(),
            reports.join("\n\n")
        ))
    }
}

#[test]
fn the_runtimes_agree_on_every_generated_case() {
    let (seed, count) = (seed(), case_count());
    let started = Instant::now();
    match Harness::new(Some("")).run(seed, count, KNOWN_FINDINGS) {
        Ok((compared, refused)) => println!(
            "seed {seed:#x}: {compared} cases agree in both runtimes ({refused} of them refused by both), in {:.1} s",
            started.elapsed().as_secs_f64()
        ),
        Err(problem) => panic!("{problem}"),
    }
}

/// The rule that keeps the list of known findings true: a finding whose own case no longer shows
/// the step fails the run and says to remove the entry, whatever the seed. The finding here was
/// never one: the two runtimes agree on its case.
#[test]
fn a_known_finding_that_does_not_reproduce_fails_the_run() {
    fn agreed() -> Case {
        written_case(
            "pass",
            Spec::List {
                length: 3,
                item: Box::new(Spec::Index),
                gaps: Gaps::None,
            },
            None,
        )
    }
    let findings = [Finding {
        description: "a finding for the test of the rule, which was never one",
        tracked: "nowhere",
        check: Check::Differential,
        runtime: None,
        case: agreed,
        covers: |case| case.probe.name == "pass",
    }];
    let harness = Harness::new(None);
    for seed in [1, 2, 3] {
        let problem = harness
            .run(seed, 40, &findings)
            .expect_err("a finding that does not reproduce fails the run");
        assert!(
            problem.contains("no longer reproduces") && problem.contains("Remove the entry"),
            "{problem}"
        );
    }
    // And a finding's predicate is what skips generated cases: with none listed, every case of
    // the run is compared.
    let all = harness.run(1, 40, &[]).expect("the runtimes agree");
    assert_eq!(all.0, 40);
}

// ------------------------------------------------------------------------------------------------
// The generator and the probes
// ------------------------------------------------------------------------------------------------

#[test]
fn the_generator_is_deterministic_and_covers_what_it_draws() {
    let json = |cases: &[Case]| {
        serde_json::to_vec(&cases.iter().map(Case::json).collect::<Vec<_>>()).expect("cases")
    };
    let first = generate(seed(), 60, Sizes::STANDARD);
    let again = generate(seed(), 60, Sizes::STANDARD);
    assert!(
        json(&first) == json(&again),
        "one seed gives one list of cases, byte for byte"
    );
    assert!(
        json(&first) != json(&generate(seed() + 1, 60, Sizes::STANDARD)),
        "another seed gives other cases"
    );

    // A thousand cases cover every probe, shape, boundary size and reserved name.
    let cases = generate(seed(), 1000, Sizes::STANDARD);
    let drawn: BTreeSet<&str> = cases
        .iter()
        .flat_map(|case| case.tags.iter().map(String::as_str))
        .collect();
    let mut wanted: Vec<String> = PROBES
        .iter()
        .map(|probe| format!("probe:{}", probe.name))
        .collect();
    wanted.extend(BOUNDARY_SIZES.iter().map(|size| format!("size:{size}")));
    wanted.extend(RESERVED.iter().map(|form| form.tag().to_string()));
    wanted.extend(
        [
            "size:hundreds",
            "size:thousands",
            "shape:scalar",
            "shape:string",
            "shape:list",
            "shape:object",
            "shape:nested",
            "shape:reserved",
            "shape:null",
            "alphabet:ascii",
            "alphabet:two-byte",
            "alphabet:astral",
            "gaps:null",
            "gaps:empty-list",
            "type-name:short",
            "type-name:long",
            "names:long",
            "content:small",
            "content:large",
            "member:defined",
            "member:extra",
            "second:same",
            "second:first-differs",
            "second:last-differs",
            "second:other-names",
            "second:extra-name",
            "second:unrelated",
        ]
        .map(String::from),
    );
    let missing: Vec<&String> = wanted
        .iter()
        .filter(|tag| !drawn.contains(tag.as_str()))
        .collect();
    assert!(missing.is_empty(), "1,000 cases draw none of: {missing:?}");

    // The larger scale is eight times the size and eight times the repeats; nesting is not scaled.
    let list = written_case(
        "place",
        Spec::Nest {
            levels: 40,
            lists: true,
            leaf: Box::new(Spec::List {
                length: 5,
                item: Box::new(Spec::Index),
                gaps: Gaps::None,
            }),
        },
        None,
    );
    let depth_and_length = |value: &Value| {
        let (mut depth, mut value) = (0, value);
        while let Value::Array(items) = value {
            if items.len() != 1 {
                return (depth, items.len());
            }
            depth += 1;
            value = &items[0];
        }
        (depth, 0)
    };
    let (base, scaled) = (list.call(1), list.call(SCALE));
    assert_eq!(depth_and_length(&base["arguments"][0]), (40, 5));
    assert_eq!(depth_and_length(&scaled["arguments"][0]), (40, 40));
    assert_eq!(
        (&base["arguments"][1], &scaled["arguments"][1]),
        (&json!(16), &json!(128))
    );
}

/// Every probe evaluates in the Rust runtime on a small value written by hand, and the counts of
/// five of them are the ones the cost model gives, worked here.
#[test]
fn every_probe_evaluates_on_a_small_value_at_the_cost_the_model_gives() {
    let program = link(&probe_images());
    let small = |site: cost::Site| match site {
        cost::Site::Untyped | cost::Site::Optional | cost::Site::Spliced | cost::Site::Entry => {
            Spec::Object {
                type_name: TypeName::None,
                fields: 2,
                name_length: 0,
                prefix: 'k',
                value: Box::new(Spec::Index),
                gaps: Gaps::None,
                grow: Grow::Fields,
            }
        }
        cost::Site::Ints | cost::Site::Objects => Spec::List {
            length: 3,
            item: Box::new(Spec::Index),
            gaps: Gaps::None,
        },
        cost::Site::Text => Spec::Text {
            alphabet: cost::Alphabet::Ascii,
            length: 5,
        },
        cost::Site::Pair => Spec::Pair {
            typed: true,
            right: 5,
        },
        cost::Site::Int => Spec::Int(7),
        cost::Site::Float => Spec::Float(1.5),
    };
    for probe in PROBES {
        let case = written_case(
            probe.name,
            small(probe.site),
            probe.pair.then_some(Second::Same),
        );
        let answer = answer(&program, &case);
        // An object that is no action and no invocation is refused as a batch entry, by both
        // runtimes alike; every other probe accepts its small value.
        let refused = answer["base"].get("refused").is_some();
        assert_eq!(
            refused,
            probe.name == "batchEntry",
            "{}: {answer}",
            probe.name
        );
    }

    // What a call costs and how large its input is, as the usage report gives them.
    let used = |call: Value| {
        let call = Call::read(&call);
        let (options, usage) = measuring();
        call.run(&program, None, &options)
            .unwrap_or_else(|error| panic!("{error}"));
        (usage.operations().unwrap(), usage.input_size().unwrap())
    };
    let function = |name: &str, arguments: Value| json!({ "kind": "function", "name": name, "arguments": arguments });
    // `pass`: the argument and the result checked, the `slot`, and the record and its number
    // written. The input is the record and its number.
    assert_eq!(used(function("pass", json!([{ "a": 1 }]))), (5, 2));
    // `hold`, twice: two arguments checked; the `forRange` and the seven of its `Range`; for each
    // iteration the `call`, its callee and its `slot`, the argument checked as one value, the
    // literal, the result checked and the item placed; and the list and its two numbers written.
    assert_eq!(
        used(function("hold", json!([[1, 2, 3], 2]))),
        (2 + 8 + 2 * 7 + 3, 5)
    );
    // `compareEq`, once: three arguments, the loop's eight, the `binary` and its two `slot`
    // nodes, four pairs compared (the lists and their three items), the item placed, and the
    // list and its boolean written.
    assert_eq!(
        used(function("compareEq", json!([[1, 2, 3], [1, 2, 4], 1]))),
        (3 + 8 + 3 + 4 + 1 + 2, 9)
    );
    // `typedInts`, twice: three items and the count checked; the loop's eight; for each
    // iteration the `call`, its callee and its `slot`, three items checked, the literal, the
    // result checked and the item placed; and the list and its two numbers written.
    assert_eq!(
        used(function("typedInts", json!([[1, 2, 3], 2]))),
        (4 + 8 + 2 * 9 + 3, 5)
    );
    // `bindContent`, once: two arguments; the loop's eight; the `call`, its callee and its
    // `slot`, four items bound, the list checked as one value, the literal, the result checked
    // and the item placed; and the list and its number written.
    assert_eq!(
        used(function("bindContent", json!([[1, 2, 3, 4], 1]))),
        (2 + 8 + 11 + 2, 6)
    );
}

// ------------------------------------------------------------------------------------------------
// The time report
// ------------------------------------------------------------------------------------------------

/// The time under which a case is not reported, for each runtime, in milliseconds at the larger
/// scale. A timing this short is within the noise of the machine, and a step that is not charged
/// shows as time only once a case takes longer than this.
///
/// <para>Measured on 2026-10-03 on one x86-64 laptop, in a release build and Node 24, over the
/// 400 cases of the default seed with the repeats multiplied by eight. On the unmodified
/// runtimes, the longest case whose time outgrew its units twofold and that is no known finding
/// took up to 0.67 ms at the larger scale in the Rust runtime and up to 1.2 ms in the
/// TypeScript runtime, over a dozen runs of the report on two sets of cases: small cases, whose
/// base timing is tens of microseconds. The Rust floor is three times that and the TypeScript
/// floor two and a half times. The TypeScript runtime's is the noisier measure because the
/// engine compiles a function as it runs, so a short base timing is slow for its size and a
/// case has to run for a few milliseconds before its two scales compare.</para>
const RUST_FLOOR_MS: f64 = 2.0;
const TYPESCRIPT_FLOOR_MS: f64 = 3.0;

/// How many times the growth of its units a case's time may grow before it is reported.
const TIME_GROWTH_ALLOWED: f64 = 2.0;

/// The multiplier on the repeats of both scales of the report unless `NX_COST_TIME_MULTIPLIER`
/// names another: a case repeats its step 128 times at its base scale and 1,024 times at its
/// larger one, where the blocking tests repeat it 16 and 128 times. The values keep their sizes.
///
/// <para>Set so that each kind of step the report is for, re-introduced in a runtime, is
/// reported there. The honest work of a call, converting its input and writing its result, is
/// about a hundred nanoseconds for each value in either runtime, and a walk or a copy that is not
/// charged is one or two, so such a step shows beside the honest work only when it is repeated
/// several hundred times. With the repeats as the blocking tests have them, the walk of the
/// whole state on every patch grew its case's time 1.8 times as fast as its units, under the
/// twofold the report looks for; with them multiplied by eight every step tried is reported,
/// each past its runtime's floor (the measurements are in task 9.3 of
/// `add-ir-runtime-input-limit-and-cost-tests`). Multiplying the sizes of the values as well
/// made the report name honest cases instead: a result or a string eight times larger again
/// leaves the processor's caches, and its time for each unit doubles for that alone.</para>
const TIME_MULTIPLIER: u64 = 8;

/// The cases the report times unless `NX_COST_CASES` names another number.
const TIME_CASES: usize = 400;

/// One scale of a case as a runtime timed it: the least of five timings, and the units used.
#[derive(Clone, Copy)]
struct Timed {
    milliseconds: f64,
    units: f64,
}

fn timed_from(answer: &Value) -> Option<Timed> {
    Some(Timed {
        milliseconds: answer.get("milliseconds")?.as_f64()?,
        units: answer.get("operations")?.as_f64()? + answer.get("inputSize")?.as_f64()?,
    })
}

/// The least of five timings of a call in the Rust runtime under limits it cannot reach, with
/// what the call used; `None` for a call the runtime refuses.
fn rust_timed(program: &Program, call: &Value) -> Option<Timed> {
    let call = Call::read(call);
    let (options, usage) = measuring();
    let mut least = f64::INFINITY;
    for _ in 0..5 {
        let instance = call.prepare(program);
        let started = Instant::now();
        let result = call.run(program, instance.as_ref(), &options);
        let elapsed = started.elapsed().as_secs_f64() * 1e3;
        result.ok()?;
        least = least.min(elapsed);
    }
    Some(Timed {
        milliseconds: least,
        units: (usage.operations()? + usage.input_size()?) as f64,
    })
}

/// How far a case's time outgrew its units between the scales: the growth of its time over the
/// growth of its operation count plus input size. One is time in step with the count.
fn outgrowth(base: Timed, scaled: Timed) -> f64 {
    let units = scaled.units / base.units.max(1.0);
    let time = scaled.milliseconds / base.milliseconds.max(1e-6);
    time / units.max(1e-9)
}

/// The time report: every case timed at two scales in both runtimes, and a case reported when
/// its time at the larger scale is more than twice the growth of its units times its time at the
/// base, and is past its runtime's floor. A time is the least of five runs, and of fifteen for a
/// case that looks reportable after the first five.
///
/// <para>Scaling the value and the repeats together is what makes one rule fit the steps the
/// budget change's review found. A copy made on every call for a fixed charge grows its count
/// eightfold with the repeats and its time sixty-four-fold. A walk repeated within one call does
/// the same. A step that is quadratic in a value for a linear count grows its time sixty-four-
/// fold against eight. Honest work that is charged nothing, as the Rust runtime's one conversion
/// of a host value on the way in, grows with the input size, which is among the units, and is
/// not reported.</para>
///
/// <para>Timing is noisy, so this is not one of the blocking tests: it is ignored, runs as a
/// command of its own in an optimized build, and fails only to say that it reported a case that
/// is not a known finding.</para>
#[test]
#[ignore = "the time report: run it explicitly, in a release build"]
fn time_report() {
    let seed = seed();
    let count = match std::env::var("NX_COST_CASES") {
        Ok(_) => case_count(),
        Err(_) => TIME_CASES,
    };
    let multiplier = match std::env::var("NX_COST_TIME_MULTIPLIER") {
        Ok(text) => text
            .parse()
            .unwrap_or_else(|_| panic!("NX_COST_TIME_MULTIPLIER is not a number: {text}")),
        Err(_) => TIME_MULTIPLIER,
    };
    // Every timing of the probe `NX_COST_TIME_PROBE` names is printed, reported or not: what a
    // re-introduced step takes beside its runtime's floor is read from this.
    let shown = std::env::var("NX_COST_TIME_PROBE").ok();
    let harness = Harness::new(Some("time"));
    let cases = generate(seed, count, Sizes::STANDARD);
    println!(
        "time report: seed {seed:#x}, {count} cases, the repeats of both scales multiplied by {multiplier}{}",
        if cfg!(debug_assertions) { " (an unoptimized build: the Rust timings mean little)" } else { "" }
    );

    // Every case is timed once in both runtimes. A case that looks reportable is then timed
    // twice more and the least of all its timings kept at each scale, which is still the least
    // of several runs and sets aside most of what one noisy timing reports.
    let time = |cases: &[&Case]| -> Vec<[Option<(Timed, Timed)>; 2]> {
        let mut timings = Vec::with_capacity(cases.len());
        for cases in cases.chunks(40) {
            let requests: Vec<Value> = cases.iter().map(|case| case.json_at(multiplier)).collect();
            let typescript = harness.runner.run("time", &requests);
            for (request, typescript) in requests.iter().zip(&typescript) {
                let rust = rust_timed(&harness.program, &request["base"])
                    .zip(rust_timed(&harness.program, &request["scaled"]));
                let typescript =
                    timed_from(&typescript["base"]).zip(timed_from(&typescript["scaled"]));
                timings.push([rust, typescript]);
            }
        }
        timings
    };
    let runtimes = [("Rust", RUST_FLOOR_MS), ("TypeScript", TYPESCRIPT_FLOOR_MS)];
    let reportable = |runtime: usize, timing: Option<(Timed, Timed)>| {
        timing.is_some_and(|(base, scaled)| {
            outgrowth(base, scaled) > TIME_GROWTH_ALLOWED
                && scaled.milliseconds > runtimes[runtime].1
        })
    };
    let all: Vec<&Case> = cases.iter().collect();
    let mut timings = time(&all);
    let suspects: Vec<usize> = (0..cases.len())
        .filter(|index| (0..2).any(|runtime| reportable(runtime, timings[*index][runtime])))
        .collect();
    for _ in 0..2 {
        let again = time(
            &suspects
                .iter()
                .map(|index| &cases[*index])
                .collect::<Vec<_>>(),
        );
        for (index, again) in suspects.iter().zip(again) {
            for runtime in 0..2 {
                if let (Some((base, scaled)), Some((base_again, scaled_again))) =
                    (&mut timings[*index][runtime], again[runtime])
                {
                    base.milliseconds = base.milliseconds.min(base_again.milliseconds);
                    scaled.milliseconds = scaled.milliseconds.min(scaled_again.milliseconds);
                }
            }
        }
    }

    let mut reported = Vec::new();
    let mut largest_under_floor = [0f64; 2];
    let mut timed_cases = [0usize; 2];
    for (case, timings) in cases.iter().zip(&timings) {
        for (runtime, (name, floor)) in runtimes.into_iter().enumerate() {
            let Some((base, scaled)) = timings[runtime] else {
                continue;
            };
            timed_cases[runtime] += 1;
            let outgrowth = outgrowth(base, scaled);
            if shown.as_deref() == Some(case.probe.name) {
                println!(
                    "  ({name}, case {} of `{}`: {:.0} units in {:.3} ms, then {:.0} units in {:.3} ms: time grew {outgrowth:.1} times as fast as the units; the floor is {floor} ms)",
                    case.id, case.probe.name, base.units, base.milliseconds, scaled.units, scaled.milliseconds
                );
            }
            if outgrowth <= TIME_GROWTH_ALLOWED {
                continue;
            }
            if scaled.milliseconds <= floor {
                largest_under_floor[runtime] =
                    largest_under_floor[runtime].max(scaled.milliseconds);
                continue;
            }
            let known = known_to_the_time_report(
                KNOWN_FINDINGS,
                case,
                [Runtime::Rust, Runtime::TypeScript][runtime],
            );
            println!(
                "  {}{name}: time grew {outgrowth:.1} times as fast as the units: {:.0} units in {:.3} ms at the base, {:.0} units in {:.3} ms at the larger scale\n    {}",
                match known {
                    Some(finding) => format!("[known: {}, tracked in {}] ", finding.description, finding.tracked),
                    None => String::new(),
                },
                base.units,
                base.milliseconds,
                scaled.units,
                scaled.milliseconds,
                case.describe(),
            );
            if known.is_none() {
                reported.push(format!("{name}: {}", case.describe()));
            }
        }
    }
    for (runtime, name) in ["Rust", "TypeScript"].into_iter().enumerate() {
        println!(
            "{name}: {} cases timed; the longest case under the floor whose time outgrew its units took {:.3} ms",
            timed_cases[runtime], largest_under_floor[runtime]
        );
    }
    assert!(
        reported.is_empty(),
        "the time report names {} case(s) whose time grows faster than their count, which are not known findings. Timing is noisy: run it again, and if a case is still reported, look at the step its probe exercises.\n{}",
        reported.len(),
        reported.join("\n")
    );
    println!(
        "no case that is not a known finding outgrew its units in time past its runtime's floor"
    );
}
