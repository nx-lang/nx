## Why

When a host calls an NX function and an argument does not fit its parameter, both IR runtimes
answer with a code and a sentence. The sentence names the argument; nothing a program can read
does. `@nx-lang/agent` therefore cannot tell an argument the model sent from the context record
the host filled in, and reports both as `invalid-input`, the one failure the model is told about
in full and asked to correct. A host's own mistake is put to the model as the model's.
`add-agent-host-package` recorded this as an accepted risk because the fix belongs in the
runtimes, and parsing a message was never an option.

## What Changes

- Both IR runtimes add one optional member, `argument`, to a diagnostic: the name of the parameter
  of the function the host called whose argument the failure is in, or which is missing. It is
  set for a failure in a value the host passed to the call it made (`callFunction` and
  `evaluateFunction`), and for nothing else: not a failure a default raises, though a record's
  defaults are filled in while its argument is checked; not a resource limit; not a failure
  inside the function or in its result.
- The two runtimes name the same argument for the same call, and the conformance corpus holds
  them to it. For that the corpus gains a case that fails: its diagnostic's code and argument are
  recorded from the Rust runtime in place of a result, and checked in the TypeScript runtime.
- `@nx-lang/agent` classifies a failed call by that member and no longer by the code alone:
  - a boundary failure whose `argument` is a context parameter is `invalid-context`;
  - a boundary failure whose `argument` is any other parameter is `invalid-input`, as today;
  - **BREAKING** (the package is unstable): a boundary-family failure that names no argument is
    `evaluation-failed`, where it is `invalid-input` today. It was not found in what the model
    sent, so the model cannot correct it. A record field default of the wrong type is the case
    that exists today.
- Both runtime READMEs, `docs/nx-ir-format.md`, the agent package's README and the host page say
  what `argument` is and what the three codes now mean.

The message text does not change, and no code changes. No image, schema or feature of NX IR
changes, and the compiler does not change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `typescript-ir-runtime`: a diagnostic for a failure in an argument of the called function names
  the argument.
- `rust-ir-runtime`: the same, with the same names.
- `nx-ir-format`: the requirement *The conformance corpus evaluates entrypoints with arguments*,
  which today admits only cases every runtime accepts, admits a case every runtime fails alike.
- `agent-host-package`: how a failed evaluation is classified, in the requirement *A function
  tool runs its function under the evaluation budget*. `treat-host-values-as-json` modifies the
  same requirement; whichever of the two is applied second brings its delta up to the main spec
  first.

## Impact

- `runtime/typescript/src/index.ts` (`NxIrDiagnostic`, the binding of a called function's
  arguments) and its committed `dist`.
- `crates/nx-ir-runtime` (`error.rs`, `eval.rs`, `normalize.rs`, and `component.rs`, where
  `call_function` and `evaluate_function` enter a function), and its `README.md`.
- `specs/ir-conformance`: a new program, `argument-diagnostics`, with `manifest.json` and the
  README, which describes a case that fails. The generator
  (`crates/nx-codegen/src/ir_corpus_tests.rs`) records one, and the corpus tests of both runtimes
  (`crates/nx-ir-runtime/tests/corpus.rs`, `runtime/typescript/test/corpus.test.mjs`) check it.
- `crates/nx-codegen/tests/cost_differential.rs`, `crates/nx-codegen/tests/cost/mod.rs` and
  `runtime/typescript/test/cost-runner.mjs`, the differential test that runs both runtimes.
- `docs/nx-ir-format.md`, which describes a diagnostic's members for both runtimes.
- `packages/agent/src/classify.ts`, `execute.ts`, their tests, and `README.md`.
- `sites/website/.../reference/libraries/agent-hosts.md`, `src/vscode/CHANGELOG.md`.
- Hosts: a host that matched `invalid-input` to mean "any boundary failure" sees `invalid-context`
  or `evaluation-failed` for the two cases that were never the model's. ReachMe is the one host.
