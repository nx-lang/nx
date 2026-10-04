## Why

`add-ir-runtime-evaluation-budget` gave both IR runtimes an operation budget and a cost model whose
rule is that work which is not charged does not grow with the size of a value. Its review found
that rule broken in four successive rounds, and after the first, every break needed the same
ingredient: a large or unusual value the host passes at the type `object`, where accepting it costs
one operation whatever it holds. A megabyte field name, an object 80,000 fields wide and a list of
20,000 `null`s each bought seconds of time or hundreds of megabytes for a few thousand operations.
Each was closed by hand, and the documents now tell a host to keep a limit of its own, because
review is not a proof.

Two things would establish the rule better than review has. A host that runs code it did not write
needs a way to bound what it hands that code, so that a step nobody has found yet is limited by the
budget times a small number and not by whatever arrived. And the search for such steps should be
mechanical: generated host values, run through both runtimes, with the count compared between them
and the work compared with the count. Hosts are about to depend on the budget (`add-agent-host-package`,
ReachMe's function tools), so both belong before that.

## What Changes

The change is staged. Each stage is complete and useful when it lands; a later stage does not
change an earlier one.

**Stage 1: a limit on host input, and corpus entrypoints with arguments**

- Define the **input size** of a call in the IR contract: the size of every value the host
  supplies to it, measured as the cost model measures a value written for the host (one per value,
  and one per 64 UTF-16 code units of a string, a type name and a field name).
- Add an input limit to the runtime options of both runtimes: `maxInputSize` in
  `@nx-lang/ir-runtime` and `RuntimeOptions::max_input_size` in `nx-ir-runtime`. It is unset by
  default, which means unlimited, so no host changes behavior. It covers what the host passes to
  one call of a public evaluation API: arguments, props, content, state passed in, the entries of a
  dispatched batch and a state patch. A component instance is not input.
- A call whose input is larger than the limit fails with `nx-ir-resource-limit`, whose `limit`
  names `maxInputSize`, before anything is evaluated; measuring stops as soon as the size passes
  the limit. The limit is separate from the operation budget and charges nothing to it, so
  no operation count changes.
- The conformance corpus gains entrypoints that take arguments, so that host values, which no
  corpus entrypoint supplies today, are pinned across runtimes: results, operation counts,
  recorded failures and input sizes.
- A call reports what it used. A runtime option names a sink the runtime fills when a call ends,
  on success and on failure: the operations used, when a budget is set, and the input size, when
  an input limit is set. A host can log what a call cost and choose a budget from measurement,
  the corpus is regenerated without bisecting, and the tests of stages 2 and 3 read a count in
  one evaluation. Nothing is counted for a host that sets no budget.
- Each runtime exports the input measure, so a host can hold one part of what it passes to a
  number of its own before it calls.
- `add-agent-host-package` limits a tool call's input in two parts. It holds the model's
  arguments to a cap of their own, 1,000 by default, measured with the exported measure before
  the call, and reports input over it as `invalid-input`, which the model can correct. The
  host's context has a separate allowance, and passing it is `resource-limit`.

**Stage 2: a seeded differential test of evaluation cost**

- A deterministic generator of host values biased toward the shapes review found (wide objects,
  long names, long and non-ASCII strings, nested and empty lists, `null`s, values just under and
  over 64 code units, and the names a runtime gives a meaning to), run through a fixed library of
  small NX functions and components that covers at least the operations the spec lists.
- For every generated case both runtimes SHALL give the same result, the same operation count
  and input size, read from the usage report, and, at budgets below the count, the same failing
  declaration and span. The cases are generated once
  and given to both runtimes, and a failure names the seed and the case.

**Stage 3: proportionality checks**

- Each case is run at two scales, the second with its value eight times the size and its step
  repeated eight times as often. In the Rust runtime the bytes a call allocates SHALL stay within
  a fixed multiple of its operation count plus its input size, and the bytes for each unit of
  that sum SHALL at most double between the scales; this is repeatable and blocks a merge.
- In both runtimes, a case whose time grows more than twice as fast as its operation count plus
  input size, past a floor, is reported. Timing is noisy, so this check runs as a job that does
  not block.

**Scope rule.** A step these tests uncover is recorded as a finding with the case that shows it,
listed in the test as known so the suite stays green, and fixed in a change of its own. This change
adds the limit and the tests; it does not change the cost model.

**What the limit is not.** It bounds a step that is linear in a host value at the budget times the
limit. A step driven by a value the program builds is still bounded by the budget squared, so the
documents keep their advice that a host running code it did not write also limits time and memory.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `nx-ir-format`: Defines the input size of a call; the conformance corpus gains entrypoints with
  arguments; and evaluation cost is validated against generated host values, across runtimes and
  against the work a runtime does.
- `typescript-ir-runtime`: Every evaluation API accepts an input limit and refuses a call whose
  host input exceeds it; reports what a call used to a sink in its options; and exports the
  input measure.
- `rust-ir-runtime`: Every evaluation API accepts the same input limit with the same measure,
  reports what a call used, and exports the measure; and its allocations are held to a multiple
  of the operation count.

## Impact

- `runtime/typescript/src/index.ts`: `NxRuntimeOptions.maxInputSize`, a bounded walk over host
  input at the entry of each exported evaluation function, and the `maxInputSize` limit name;
  `NxRuntimeOptions.usage` and the type `NxRuntimeUsage`; the export `measureInputSize`.
  Tests in `runtime/typescript/test/`.
- `crates/nx-ir-runtime`: `RuntimeOptions::max_input_size`, a counting pass over the host's
  values at the top of each public method, and the limit name; `RuntimeOptions::usage` and the
  type `Usage`, for which `RuntimeOptions` stops being `Copy`, `PartialEq` and `Eq`; the exports
  `input_size` and `record_input_size`. Tests in `crates/nx-codegen/src/ir_runtime_tests.rs`.
- `specs/ir-conformance`: `program.json` entrypoints gain optional `arguments` with a case name,
  and a program may ask for input sizes to be recorded; a new corpus program of host-value cases;
  `manifest.json` and the README change. `crates/nx-codegen/src/ir_corpus_tests.rs`,
  `crates/nx-ir-runtime/tests/{corpus,damage}.rs` and `runtime/typescript/test/corpus.test.mjs`
  read them. Nothing under an existing program's `expected/` changes.
- New test code: `crates/nx-codegen/tests/`, that crate's first integration tests, with a probe
  library in NX, a seeded generator, a differential harness and an allocation test binary; and a
  Node runner in `runtime/typescript/test/`. No new runtime dependency and no new crate dependency.
- CI: the differential test and the Rust allocation check run in the existing test jobs; a step
  that fails when the build changes the committed `runtime/typescript/dist`; and one new job that
  does not block runs the time report.
- Docs: `docs/nx-ir-format.md`, both runtime READMEs, `specs/ir-conformance/README.md`.
- `openspec/changes/add-agent-host-package`: its design, spec and tasks gain the two input
  limits and depend on stage 1 of this change; the default of the context allowance is set by a
  task here. If that change is archived by then, the package gains them, with a `MODIFIED` delta
  to its main spec added to this change.
- Not touched: the cost model's charging rules, any recorded operation count, the compiler, the
  emitter, `nx-api`, the CLI and the .NET, Node and wasm bindings.
- Depends on `add-ir-runtime-evaluation-budget`, which must land first: this change builds on its
  cost model, its `limit` on diagnostics, and its `operations.json`. Independent of
  `add-ir-runtime-performance-harness`, whose non-blocking job the time-scaling check sits beside,
  and of `retire-hir-interpreter`.
