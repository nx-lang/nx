## Why

Hosts are about to run NX functions they did not write. ReachMe will expose a tenant-authored pure
function to a model as a tool and evaluate it with `@nx-lang/ir-runtime` inside a Cloudflare
Durable Object, where a long synchronous evaluation cannot be interrupted and ends with the
platform killing the invocation. The runtime bounds call depth (100) and the length of one range
(1,000,000), and neither bounds work: three nested `for` loops over ranges of a thousand run a
billion bodies inside both limits, and a function that doubles a list or a string on each of its
100 permitted calls asks for 2^100 items. The interpreter's operation budget (`max_operations`,
1,000,000) is the only limit NX ever had on total work, and `retire-hir-interpreter` drops it,
leaving "does the budget need a successor" as an open question. This change is the successor, in
the IR runtimes, where hosts actually evaluate.

The TypeScript runtime also still fails in two ways that are not diagnostics. It has no bound on
expression nesting, so deep recursion ends in the engine's `RangeError`, and it splices a list into
another with `push(...items)`, which V8 refuses above roughly 125,000 items with the same error,
well inside the default range limit.

## What Changes

- Define the **operation**, the unit of evaluation cost, as part of the IR contract rather than of
  one runtime: one per node evaluated, one per item placed in a sequence a node builds, one per
  64 UTF-16 code units of a string a `concat` produces, one per value checked against a declared
  type, one per pair of values an equality compares, and one per value written for the host,
  strings by their length. Every walk over a value pays for what it visits, so a value held in
  many places costs what it is as a tree wherever it is walked as one. The count depends only on
  the images and the input, so every runtime that implements the budget stops at the same place.
- Add an operation budget to the runtime options of both IR runtimes: `maxOperations` in
  `@nx-lang/ir-runtime` and `RuntimeOptions::max_operations` in `nx-ir-runtime`. It is unset by
  default, which means unlimited, so no existing host changes behavior. One budget covers one call
  of a public evaluation API, whatever that call evaluates: `evaluateFunction`, `callFunction`,
  `constructComponentDescriptor`, `initializeComponent`, `evaluateComponent`,
  `dispatchComponentActions` (the whole batch and the render after it), `normalizeComponentState`
  and `applyComponentStatePatch`, and their Rust counterparts.
- Exceeding the budget fails with the existing code `nx-ir-resource-limit`, before the operation
  that would exceed it runs. Every `nx-ir-resource-limit` diagnostic, in both runtimes, gains a
  structured `limit` holding the limit's name and value, so a host tells an exhausted budget from
  runaway recursion without reading the message.
- Bring the TypeScript runtime to parity with the Rust runtime's nesting bound: expressions may
  nest 1,000 deep across every call of one evaluation, and an engine `RangeError` raised during
  evaluation (stack, string length, array length) is reported as `nx-ir-resource-limit` instead of
  escaping. Splicing no longer depends on the engine's argument limit.
- Record the operation count of every entrypoint and lifecycle step of the conformance corpus, and
  have both runtimes check that each succeeds under exactly that budget and fails under one less.
- Document the budget, the cost model and what it does not bound in both runtime READMEs and in
  `docs/nx-ir-format.md`.

One thing here changes what a JavaScript host sees. The TypeScript runtime no longer holds an
integer outside JavaScript's safe range: it used to carry one as its digits, without being able to
compute with it or accept it at a typed parameter, and now refuses one where it reads it from an
image, with `nx-ir-number`. The record canonical JSON spells such an integer with, passed at
`object`, is a record like any other. `int` is specified as exact over ±(2^53−1) on every backend,
so a program that reaches such an integer is already outside what `int` promises, and refusing it
closes the ways a host could spell something large as one number (review RF24 to RF30). A Rust caller that builds `RuntimeOptions` or a `Diagnostic` with an exhaustive struct
literal gains a field to set.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `nx-ir-format`: Defines the operation as the unit of evaluation cost for an image, and the
  conformance corpus records the operation count of every entrypoint and lifecycle step; the
  canonical encoding rules let a runtime that cannot hold an integer outside the safe range
  refuse it, and say no runtime takes a host's value for one by its shape.
- `typescript-ir-runtime`: Every evaluation API accepts an operation budget and fails with a
  structured resource-limit diagnostic when it is exceeded; expression nesting is bounded; an
  engine range error during evaluation is a diagnostic; an integer outside the safe range is
  refused where it is read.
- `rust-ir-runtime`: Every evaluation API accepts the same operation budget with the same counts
  and the same failing node, and resource-limit diagnostics carry the limit's name and value; the
  two requirements that describe how it differs from the TypeScript runtime for an integer
  outside the safe range are brought up to date.

## Impact

- `runtime/typescript/src/index.ts`: `NxRuntimeOptions.maxOperations`, `NxIrDiagnostic.limit`, one
  per-call evaluation state threaded where the options are threaded today, the nesting bound, the
  range-error conversion at the public entry points, loop-based splicing in `evalItemInto` and
  in dispatch's effects, and charges in `normalizeValue`, `valuesEqual` and `canonicalizeRendered`.
  Two action handlers now compare by node and captured values, as in the Rust runtime. Tests in `runtime/typescript/test/runtime.test.ts` and `corpus.test.mjs`.
- `crates/nx-ir-runtime`: `RuntimeOptions::max_operations`, `Diagnostic::limit`, a counter in
  `Machine` beside the nesting counter, charges in `eval.rs`, `normalize.rs` and `value.rs`
  (equality and the conversion for the host), and a state depth check that follows sharing. Tests in
  `crates/nx-ir-runtime/tests/corpus.rs` and `crates/nx-codegen/src/ir_runtime_tests.rs`.
- `specs/ir-conformance`: a new `expected/operations.json` per program, written by the existing
  regeneration command in `crates/nx-codegen/src/ir_corpus_tests.rs`. Images and `results.json`
  are unchanged; the schema version, the runtime ABI and the required features are unchanged.
- Docs: `runtime/typescript/README.md`, `crates/nx-ir-runtime/README.md`, `docs/nx-ir-format.md`,
  `specs/ir-conformance/README.md`.
- Not touched: the compiler, the emitter's output, `nx-api`, the CLI and the .NET, Node and wasm
  bindings. Native source evaluation keeps the interpreter's own `max_operations` until
  `retire-hir-interpreter` replaces the interpreter; passing the new budget through `nx-api` and
  the bindings is left to that change or a follow-up (see design.md).
- Other changes in this set: `add-agent-host-package` runs `FunctionTool` functions and `HttpTool`
  argument functions under this budget and reads `limit.name` to report the failure, and ReachMe's
  `add-agent-function-tools` chooses the number. This change depends on none of the other eight.
- Active NX changes: independent of `add-ir-runtime-performance-harness`, which supplies the
  overhead measurement if it lands first; answers the open question in `retire-hir-interpreter`
  and can land before or after it.
