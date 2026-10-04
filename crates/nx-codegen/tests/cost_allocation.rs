//! The allocation check of evaluation cost: what the Rust runtime asks its allocator for during
//! a call must be in proportion to the call's operation count plus its input size.
//!
//! <para>Every generated case of the differential test is run here at both of its scales, the
//! second with its host value eight times the size and its step repeated eight times as often,
//! and two things are asserted of the bytes the call requests, where the units of a call are its
//! operations plus its input size, read from the usage report:</para>
//!
//! ```text
//! bytes ≤ K × units + C
//! bytes for each unit at the larger scale ≤ 2 × bytes for each unit at the base scale
//! ```
//!
//! <para>The first is an absolute ceiling, calibrated below. The second is what finds a copy that
//! is not charged, and it needs no calibration: a step that copies a value on each repeat for a
//! fixed charge allocates sixty-four times as much at the larger scale for about eight times the
//! units, so its bytes for each unit grow eightfold. Neither is meant to catch a step that copies
//! the input a fixed number of times in a call: its bytes grow with the input size, which is
//! among the units, and the input limit bounds it.</para>
//!
//! <para>Bytes requested are the same on every run of one build, which is what lets this check
//! block a merge where a timing could not. They are not promised to be the same in an optimized
//! and an unoptimized build, so the bounds are asserted and no exact number is. This binary
//! counts every byte its process allocates, which is why its one test is alone in it.</para>

mod cost;

use cost::{
    case_count, covered_by, generate, link, measuring, probe_images, seed, stale_finding, Call,
    Case, Check, Sizes, KNOWN_FINDINGS, SCALE,
};
use nx_ir_runtime::Program;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

/// The system allocator, counting every byte asked of it.
struct Counting;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every call is forwarded to `System` unchanged; the counter is the only addition.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        ALLOCATED.fetch_add(size, Ordering::Relaxed);
        unsafe { System.realloc(pointer, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// The bytes a case's call requests at `scale`, with the units the call used: its operations
/// plus its input size. `None` for a call the runtime refuses, which has no count.
///
/// <para>The call is read from its JSON and its instance prepared before the measurement begins,
/// so what is measured is one call of the runtime's API on values the host already holds, and
/// the result it returns.</para>
fn measure(program: &Program, case: &Case, scale: usize) -> Option<(usize, u64)> {
    let call = Call::read(&case.call(scale));
    let instance = call.prepare(program);
    let (options, usage) = measuring();
    let before = ALLOCATED.load(Ordering::Relaxed);
    let result = call.run(program, instance.as_ref(), &options);
    let bytes = ALLOCATED.load(Ordering::Relaxed) - before;
    result.ok()?;
    Some((bytes, usage.operations()? + usage.input_size()?))
}

/// What is wrong with the allocation of a case, if anything.
///
/// <para>A prepared module decodes a node the first time an evaluation reaches it and keeps it,
/// which is a few hundred bytes for a declaration and belongs to the module, not to the call. So
/// the call is made once before it is measured.</para>
fn problem(program: &Program, case: &Case) -> Option<String> {
    measure(program, case, 1)?;
    let (base_bytes, base_units) = measure(program, case, 1)?;
    let (scaled_bytes, scaled_units) = measure(program, case, SCALE)?;
    for (scale, bytes, units) in [
        ("base", base_bytes, base_units),
        ("larger", scaled_bytes, scaled_units),
    ] {
        if bytes as u64 > BYTES_PER_UNIT * units + FIXED_BYTES {
            return Some(format!(
                "at the {scale} scale the call requested {bytes} bytes for {units} units, over the ceiling of {BYTES_PER_UNIT} bytes a unit and {FIXED_BYTES}"
            ));
        }
    }
    let base = base_bytes as f64 / base_units.max(1) as f64;
    let scaled = scaled_bytes as f64 / scaled_units.max(1) as f64;
    (scaled > 2.0 * base).then(|| {
        format!(
            "the bytes for each unit more than double between the scales: {base_bytes} bytes for {base_units} units ({base:.1} a unit) at the base, {scaled_bytes} bytes for {scaled_units} units ({scaled:.1} a unit) at the larger"
        )
    })
}

/// The ceiling on the bytes a call may request for each unit it used, and on what it may request
/// besides: `K` and `C` of the first bound.
///
/// <para>Calibrated on 2026-10-04 from a survey of 3,000 cases of the default seed, 6,000 calls,
/// which measured the same bytes in an unoptimized and in an optimized build. The largest ratio
/// seen was 720 bytes a unit: a value nested 40 levels deep in objects, held as the content of a
/// record or received as a prop, where each level is a record with its fields and its name,
/// converted on the way in, held, and written again. The ceiling is a little over four times
/// that. The smallest calls, of a handful of
/// units, requested a few hundred bytes, which the fixed part covers with room to spare. The
/// largest growth of the bytes for each unit between the scales was 1.86, for a concatenation
/// of one astral character: text under 64 code units costs nothing for its length, so a string
/// that grows from 4 code units to 32 grows its bytes and not its count. That is the bound the
/// cost model leaves uncharged, 63 code units a step, and it is why the second bound is a factor
/// of two and not less. Run the survey again with `NX_COST_ALLOCATION_SURVEY=1` and
/// `--nocapture` after a change of representation that moves these numbers.</para>
const BYTES_PER_UNIT: u64 = 3_000;
const FIXED_BYTES: u64 = 4_096;

#[test]
fn the_rust_runtime_allocates_in_proportion_to_what_it_charges() {
    let program = link(&probe_images());
    let seed = seed();

    if std::env::var_os("NX_COST_ALLOCATION_SURVEY").is_some() {
        survey(&program, seed);
        return;
    }

    for finding in KNOWN_FINDINGS
        .iter()
        .filter(|finding| finding.check == Check::Allocation)
    {
        assert!(
            problem(&program, &(finding.case)()).is_some(),
            "{}",
            stale_finding(finding)
        );
    }
    let cases = generate(seed, case_count(), Sizes::STANDARD);
    let mut checked = 0;
    let mut problems = Vec::new();
    for case in &cases {
        if covered_by(KNOWN_FINDINGS, Check::Allocation, case).is_some() {
            continue;
        }
        checked += 1;
        // The measure is repeatable: one call requests the same bytes each time it is made, once
        // the nodes it reaches have been decoded.
        if checked <= 40 {
            measure(&program, case, 1);
            assert_eq!(
                measure(&program, case, 1),
                measure(&program, case, 1),
                "seed {seed:#x}, {}: two runs of one call requested different bytes",
                case.describe()
            );
        }
        if let Some(problem) = problem(&program, case) {
            problems.push(format!("seed {seed:#x}, {}\n  {problem}", case.describe()));
        }
    }
    assert!(
        problems.is_empty(),
        "{} of {checked} generated cases allocate out of proportion to their count:\n\n{}",
        problems.len(),
        problems
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n\n")
    );
    println!("seed {seed:#x}: {checked} cases allocate in proportion to their count");
}

/// Prints what the suite measures, for setting the ceiling: with `NX_COST_ALLOCATION_SURVEY`.
fn survey(program: &Program, seed: u64) {
    let cases = generate(seed, case_count(), Sizes::STANDARD);
    let mut ratios = Vec::new();
    let mut small = Vec::new();
    let mut growth = Vec::new();
    for case in &cases {
        measure(program, case, 1);
        let measured: Vec<_> = [1, SCALE]
            .into_iter()
            .filter_map(|scale| measure(program, case, scale))
            .collect();
        for (bytes, units) in &measured {
            ratios.push((*bytes as f64 / *units as f64, case.describe()));
            if *units < 20 {
                small.push((*bytes, *units, case.describe()));
            }
        }
        if let [(base_bytes, base_units), (scaled_bytes, scaled_units)] = measured[..] {
            let base = base_bytes as f64 / base_units.max(1) as f64;
            let scaled = scaled_bytes as f64 / scaled_units.max(1) as f64;
            growth.push((
                scaled / base,
                case.describe(),
                base_bytes,
                base_units,
                scaled_bytes,
                scaled_units,
            ));
        }
    }
    ratios.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap());
    small.sort_by_key(|measured| std::cmp::Reverse(measured.0));
    growth.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap());
    println!("largest bytes a unit:");
    for (ratio, case) in ratios.iter().take(5) {
        println!("  {ratio:.1}  {}", &case[..case.len().min(160)]);
    }
    println!("largest bytes, for calls under 20 units:");
    for (bytes, units, case) in small.iter().take(5) {
        println!(
            "  {bytes} bytes for {units} units  {}",
            &case[..case.len().min(160)]
        );
    }
    println!("largest growth of bytes a unit between the scales:");
    for (ratio, case, base_bytes, base_units, scaled_bytes, scaled_units) in growth.iter().take(8) {
        println!(
            "  {ratio:.2}  {base_bytes}/{base_units} -> {scaled_bytes}/{scaled_units}  {}",
            &case[..case.len().min(200)]
        );
    }
}
