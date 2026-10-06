//! What the budget refuses is not allocated: a string a `concat` would build, and the names of a
//! record written for the host. And input the input limit refuses is not copied: it is measured
//! before anything is converted.
//!
//! <para>A charge made after the string was built would fail at the same node with the same
//! count, so no diagnostic tells the two orders apart; what differs is what was allocated. This
//! binary counts the bytes its one test allocates, which is why the test is alone in it. The list
//! half of the same rule is the unit test `a_refused_placement_leaves_the_sequence_untouched`
//! beside the evaluator.</para>

mod common;

use common::{corpus_root, link, prepare_all};
use nx_ir_runtime::RuntimeOptions;
use nx_value::NxValue;
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

/// The bytes `run` allocates.
fn allocated_by<T>(run: impl FnOnce() -> T) -> (T, usize) {
    let before = ALLOCATED.load(Ordering::Relaxed);
    let result = run();
    (result, ALLOCATED.load(Ordering::Relaxed) - before)
}

const LENGTH: usize = 1 << 20;

#[test]
fn what_the_budget_does_not_cover_is_not_allocated() {
    let image = corpus_root().join("evaluation-cost/expected/main.nx.nxir");
    let modules = prepare_all(&[("main.nx".to_string(), std::fs::read(image).unwrap())]);
    let program = link(&modules, "main.nx");
    a_refused_concat_allocates_no_string(&program);
    names_written_for_the_host_are_paid_for_before_they_are_copied(&program);
    empty_values_written_for_the_host_are_paid_for_before_they_are_copied(&program);
    input_over_the_limit_is_refused_before_it_is_copied(&program);
}

fn a_refused_concat_allocates_no_string(program: &nx_ir_runtime::Program) {
    // `greet` is `"Hello, " + name`: its result is as long as the name it is given.
    let name = [NxValue::String("x".repeat(LENGTH))];

    // With no budget the call builds its megabyte and more: the measure sees a result.
    let (result, unlimited) =
        allocated_by(|| program.evaluate_function("greet", &name, &RuntimeOptions::default()));
    match result {
        Ok(NxValue::String(text)) => assert_eq!(text.len(), LENGTH + "Hello, ".len()),
        other => panic!("{other:?}"),
    }
    assert!(unlimited >= 2 * LENGTH, "{unlimited} bytes");

    let budget = |operations: u64| RuntimeOptions {
        max_operations: Some(operations),
        ..RuntimeOptions::default()
    };
    let exhausted = |result: nx_ir_runtime::Result<NxValue>| {
        let error = result.expect_err("the budget does not cover the call");
        assert_eq!(
            error.diagnostics[0].limit.map(|limit| limit.name),
            Some("maxOperations")
        );
    };

    // A budget of ten covers the nodes of `greet`, since a short name succeeds under it. So what
    // refuses the long name is the charge for the length of the string and nothing before it.
    let short = [NxValue::String("x".into())];
    assert!(program
        .evaluate_function("greet", &short, &budget(10))
        .is_ok());

    // The baseline is the same call refused at its first node, under a budget of nothing: it
    // reads its argument and builds a diagnostic. The call refused at the `concat` allocates no
    // more than that and a little, where a string built and then refused would add a megabyte.
    let (result, baseline) = allocated_by(|| program.evaluate_function("greet", &name, &budget(0)));
    exhausted(result);
    let (result, limited) = allocated_by(|| program.evaluate_function("greet", &name, &budget(10)));
    exhausted(result);
    assert!(
        limited <= baseline + (64 << 10),
        "a refused concat of a {LENGTH}-byte string allocated {limited} bytes, against {baseline} for the call refused before it began"
    );
}

/// A record written for the host is given new strings for its field names and its type name.
/// For a declared record they are a few bytes. A host may pass an object at `object` whose one
/// key is a megabyte, and a program may hold that object 400 times in a list for 400 operations:
/// written, it would be 400 megabytes of keys. The names are paid for by length before they are
/// copied, so a budget that does not cover them allocates none of them.
fn names_written_for_the_host_are_paid_for_before_they_are_copied(
    program: &nx_ir_runtime::Program,
) {
    let object = NxValue::Record {
        type_name: None,
        properties: std::collections::BTreeMap::from([("k".repeat(LENGTH), NxValue::Int(1))]),
    };
    let args = [object, NxValue::Int(400)];
    let run = |operations: u64| {
        let options = RuntimeOptions {
            max_operations: Some(operations),
            ..RuntimeOptions::default()
        };
        allocated_by(|| program.evaluate_function("repeated", &args, &options))
    };

    // Refused before it began: the call reads its arguments, which is one copy of the key.
    let (result, baseline) = run(0);
    assert!(result.is_err());
    // 100,000 operations cover the loop and the first six of the 400 objects, each 16,386. So the
    // call allocates the list it built and six copies of the key, not 400.
    let (result, limited) = run(100_000);
    let error = result.expect_err("the budget does not cover 400 megabyte keys");
    assert_eq!(
        error.diagnostics[0].limit.map(|limit| limit.name),
        Some("maxOperations")
    );
    assert!(
        limited <= baseline + 8 * LENGTH,
        "a refused result with a {LENGTH}-byte key held 400 times allocated {limited} bytes, against {baseline} for the call refused before it began"
    );
}

/// An empty value written for the host takes a place in the list that holds it. A host may pass
/// a list of 20,000 of them at `object`, and a program may hold that list 500 times for 500
/// operations: written, it would be ten million entries. Each value written costs one, the empty
/// ones too, before it is written, so a budget that does not cover them allocates only as many
/// copies as it does cover.
fn empty_values_written_for_the_host_are_paid_for_before_they_are_copied(
    program: &nx_ir_runtime::Program,
) {
    const ITEMS: usize = 20_000;
    let object = NxValue::Record {
        type_name: None,
        properties: std::collections::BTreeMap::from([(
            "a".to_string(),
            NxValue::Array(vec![NxValue::Null; ITEMS]),
        )]),
    };
    let run = |times: i64, operations: Option<u64>| {
        let args = [object.clone(), NxValue::Int(times)];
        let options = RuntimeOptions {
            max_operations: operations,
            ..RuntimeOptions::default()
        };
        allocated_by(|| program.evaluate_function("repeated", &args, &options))
    };

    // What one copy written allocates, measured with no budget.
    let (result, once) = run(1, None);
    assert!(result.is_ok());
    // 100,000 operations cover four of the 500 copies, each 20,002, and not a fifth.
    let (result, limited) = run(500, Some(100_000));
    let error = result.expect_err("the budget does not cover 500 copies");
    assert_eq!(
        error.diagnostics[0].limit.map(|limit| limit.name),
        Some("maxOperations")
    );
    assert!(
        limited <= 6 * once,
        "a refused result of {ITEMS} empty values held 500 times allocated {limited} bytes, against {once} for one copy"
    );
}

/// A host value is converted on the way in, which copies it: every string, every name and every
/// list. The input limit is applied by a pass of its own that runs before the conversion and
/// builds nothing, so input it refuses costs a diagnostic and the walk's own stack, whatever its
/// size: a megabyte key, a string of 64 megabytes and a list of a million items are each refused
/// for what refusing two numbers allocates and a little.
fn input_over_the_limit_is_refused_before_it_is_copied(program: &nx_ir_runtime::Program) {
    let run = |value: NxValue, limit: u64| {
        let args = [value, NxValue::Int(1)];
        let options = RuntimeOptions {
            max_input_size: Some(limit),
            ..RuntimeOptions::default()
        };
        let (result, allocated) =
            allocated_by(|| program.evaluate_function("repeated", &args, &options));
        let error = result.expect_err("the limit does not cover the input");
        assert_eq!(
            error.diagnostics[0].limit.map(|limit| limit.name),
            Some("maxInputSize")
        );
        allocated
    };

    // Two numbers under a limit of one: the diagnostic and nothing else.
    let baseline = run(NxValue::Int(1), 1);
    let key = NxValue::Record {
        type_name: None,
        properties: std::collections::BTreeMap::from([("k".repeat(LENGTH), NxValue::Int(1))]),
    };
    let large = [
        ("a megabyte key", key),
        (
            "a 64-megabyte string",
            NxValue::String("x".repeat(64 * LENGTH)),
        ),
        (
            "a list of a million items",
            NxValue::Array(vec![NxValue::Int(1); 1_000_000]),
        ),
    ];
    for (what, value) in large {
        let refused = run(value, 100);
        assert!(
            refused <= baseline + 1024,
            "refusing {what} allocated {refused} bytes, against {baseline} for refusing two numbers"
        );
    }
}
