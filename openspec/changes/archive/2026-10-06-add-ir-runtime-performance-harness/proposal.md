## Why

`@nx-lang/ir-runtime` has no measurement of what its calls cost on a program of the size hosts
run. Hosts are putting it on a per-interaction path: ReachMe evaluates NX IR inside a Cloudflare
Durable Object, is adopting `@nx-lang/agent` to call tool functions on every step of an agent's
loop, and plans to re-evaluate a question-flow component on every answer. Three things in the
repository already wait on numbers nobody can reproduce:

- **A regression was caught by hand, once.** The first implementation of reading host values
  (`define-host-values`) made a call that takes 10,000 records 60% slower. It was found because
  the author wrote a benchmark for that change, and the fix was judged against one number from one
  machine. `specs/future.md` ("Reading a host value costs about 5% of a call that only takes
  input") says so: "Nothing watches the cost, and no test fails if it doubles."
- **The runtime is no longer run against a large program here.** The playground's DrawnUI
  examples were this repository's only run of the runtime against a real catalog, and they left
  with DrawnUI (`specs/future.md`, "`@nx-lang/ir-runtime` against a large catalog has no test
  here"). The largest module in the conformance corpus is 87 lines.
- **Limits were chosen without measurement.** The agent package's default budget of 100,000
  operations was reasoned, not counted (`specs/future.md`, "TODO: set the agent package's default
  operation budget from measured tools"), and nothing says what setting `maxOperations` and
  `maxInputSize` costs a call.

What is measured today answers a different question. The cost tests' time report
(`crates/nx-codegen/tests/cost/`) runs generated host values through small probes at two scales
and names a case whose time grows faster than its operation count. It finds a step that is not
charged. It does not say how long a host waits for a link, a render or a dispatch, or whether a
change made any of them slower.

## What Changes

- **Two programs of realistic size join the conformance corpus.** `large-catalog` is a catalog of
  external components with the measured shape of a real one (45 components, 11 of them abstract,
  37 types, about 380 properties) and a snippet that uses it. `question-flow` is a library of
  about 30 question kinds and a 30-step flow with conditional content, with a lifecycle that
  dispatches one answer per step. Both runtimes run them as they run every corpus program, so
  their images, results and operation counts are pinned and a difference between the runtimes on
  a large program fails a test. This closes the test gap, not only the measurement gap.
- **A benchmark harness in `runtime/typescript/bench/`** times each call a host makes on those
  programs: loading the runtime, preparing a module, linking, evaluating a function, initializing
  a component, resuming one from stored state, evaluating one with state, and dispatching. A third
  workload measures calls that take input: calls given 10,000 records in five shapes (at
  `object` as plain data, with one `undefined` member and nested past 32 levels; and as records
  of a declared type, plain and with one `undefined` member) and a tool call by name with a
  budget and an input limit.
- **The report gives time and count.** For each phase: the cold time in a fresh isolate, the warm
  median and 95th percentile with no limits and with limits set, the operations the call used and
  its input size, as a table and as JSON. The count is exact and the same on every machine, so a
  change in it is a fact where a change in time is an estimate.
- **A comparison against the base revision replaces a committed baseline.** `bench:compare` runs
  the revision a change is based on and the change itself, alternately, on the same machine: each
  revision's committed runtime build against that revision's committed images. It names each
  phase that is slower by more than a set fraction, with both medians, and compares the same way
  each phase's first execution in a fresh isolate and the time the runtime module takes to load.
  No file of timings from one machine is committed or compared against another machine.
- **The workload steps run in any JavaScript engine.** The core that builds and runs the steps
  imports no Node API, reads no clock and takes the runtime as an argument, so a host runs the
  same steps in its own engine (workerd, say) against its own bundled runtime and times them as
  that engine allows. A Node driver does the timing here.
- **CI runs the comparison as a job that does not block a merge**, and writes the report to the
  job summary: on a pull request against the revision the change is based on, and on a push to
  `main` against the last release, which names slowdowns too small to be named on any one pull
  request once they have added up.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `nx-ir-format`: the conformance corpus holds programs of the size hosts run, and says which of
  its checks a large image is left out of.
- `typescript-ir-runtime`: the runtime package carries a phase-level performance harness, workload
  steps that run outside Node, and a comparison against the base revision.

## Impact

- `specs/ir-conformance/`: two new programs with their expected files, `manifest.json`, the
  README. The corpus is 444 KB today and grows to 1.2 MB.
- `crates/nx-ir-runtime/tests/damage.rs`: the sweep that overwrites every cell of an image leaves
  out a program with an image above a stated size, whose images are still cut at every boundary.
- `runtime/typescript`: new `bench/` directory (core, Node driver, compare), `bench` and
  `bench:compare` scripts. Not in the published package.
- `.github/workflows/build.yml`: one job, Node only, `continue-on-error`.
- `runtime/typescript/README.md`, `specs/ir-conformance/README.md`, `docs/nx-ir-format.md`,
  `docs/deployment.md` (a release step), `specs/future.md` (one entry removed, three updated with
  what was measured, and a section for what the harness found and does not see).
- No runtime API or behavior change, and no change to an existing corpus program's expected files.
