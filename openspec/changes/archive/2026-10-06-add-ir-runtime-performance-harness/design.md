## Context

`@nx-lang/ir-runtime` reads NX IR images without copying them, links an entry module against
prepared modules (`linkNxIrProgram` with a `resolve` callback), and evaluates functions and
components (`evaluateFunction`, `callFunction`, `initializeComponent`, `evaluateComponent`,
`dispatchComponentActions`). A call can be given a budget (`maxOperations`), an input limit
(`maxInputSize`) and a `usage` object that receives the operations it used and its input size.

What exists around it:

- **The conformance corpus** (`specs/ir-conformance/`): 21 small programs with committed images,
  results, operation counts and, for some, lifecycles. The emitter's tests pin the images byte for
  byte, and both runtimes evaluate every program. It already holds a snippet compiled against an
  implicitly imported catalog (`snippet`, whose catalog is 449 bytes).
- **The cost tests** (`crates/nx-codegen/tests/cost/`): a Rust harness that drives both runtimes
  over generated host values, with a time report that CI runs as a job that does not block.
- **The committed build**: `runtime/typescript/dist` is in the repository, and CI fails when it is
  not what the source builds to. So every revision carries the runtime it would ship, and
  `test/cost-runner.mjs` already runs it "as a host would", importing nothing else.

The hosts this is for:

- **ReachMe** runs the runtime in a Cloudflare Durable Object, caches prepared library modules for
  the life of an isolate, calls a zero-argument function that returns a flow's definition, and
  will dispatch an answer and re-render on every message. Its question-flow library has 28
  question kinds.
- **The DrawnUI fiddle** evaluates `root` of a snippet against a catalog of 858 lines (45 external
  components, 37 types, about 380 properties, a 75 KB image).

## Goals / Non-Goals

**Goals:**

- Numbers for the calls hosts make, cold and warm, with and without limits, on programs of the
  size hosts run.
- A way to answer "did this change make it slower?" that works on a laptop and on a CI runner, and
  that would have named the 60% regression of `define-host-values`.
- The runtime tested, not only timed, against large programs, in both runtimes.
- Workload steps a host can run in its own engine.

**Non-Goals:**

- Optimizing the runtime. This change measures; fixes follow from what it shows.
- Blocking a merge on time.
- Timing the Rust runtime. Its hosts are not on a per-interaction path yet. The workloads are
  corpus programs, which the Rust runtime already loads, so a Rust timing run later needs a driver
  and no new workload.
- Timing the compiler, the wasm SDK or `@nx-lang/agent`'s own code.
- Running workerd in this repository's CI. A host that runs there does that.
- Publishing the harness to npm. A host takes it from a checkout of the release tag.

## Decisions

### D1. The workloads are conformance corpus programs

The first draft kept the workloads as NX source under the runtime's fixtures and compiled them
when the harness ran, because committed images go stale. The corpus already solves that: its
generator rewrites the images, and the emitter's tests fail when a committed image is not what the
compiler emits. Putting the workloads there gives, with no new mechanism:

- images that are always current, read from disk with no compiler, so the harness and its CI job
  need Node and nothing else;
- results and operation counts recorded and checked in both runtimes, so a large program is a
  test that blocks and not only a benchmark that reports;
- lifecycles with their batches, which is the scripted run of the flow;
- the same files for a Rust timing run, and for a host that runs the steps in its own engine.

What it costs:

- **The corpus grows from 444 KB to 1.2 MB**, about half of the growth being the explained text
  of the catalog's two images, and a change to the emitter rewrites two large images and their
  explanations along with the small ones.
- **One check does not scale.** The Rust runtime's damage test overwrites every cell of every
  image and, when the image still prepares, runs the whole program. For a 75 KB image with a
  30-batch lifecycle that is tens of thousands of full runs. So the sweep that overwrites cells
  leaves out a program with an image above a stated size (16,384 bytes; the corpus README says
  so), and such a program's images are only cut at every four-byte boundary, which is linear and
  cheap. The limit is on the program and not the image because the cost of one overwritten cell
  is a link and a run of the whole program: sweeping the catalog snippet's 13 KB image alone, which
  links against the 76 KB catalog each time, added 11 seconds. The TypeScript test already
  overwrites the cells of one small image only.
- **The corpus's other engines must run them.** Expected results come from the interpreter, and
  the generated JavaScript is held to them where executable codegen accepts the program. The flow
  uses action handlers, which executable codegen refuses, so it is skipped there as `handlers` is.

*Alternative considered:* a `bench/workloads/` directory of source, compiled by `cargo run -p
nx-cli` as `emitted-ir.test.mjs` compiles its sources. It keeps the corpus small, but the CI job
then builds the compiler (minutes), nothing checks the two runtimes agree on a large program, and
the counts would need an expected file of their own.

### D2. Both programs are written for this repository, in the shape of real ones

- **`large-catalog`** is generated by a committed script, deterministic and with no inputs, that
  writes a catalog with the counts measured on the fiddle's DrawnUI catalog: 45 external
  components of which 11 are abstract bases, 37 types (enumerations and records), about 380
  properties, inheritance three levels deep. Beside it, a snippet of about 60 elements that
  nests layouts, sets properties of every kind and uses a `for` and an `if`. The script is kept
  so the catalog can be regenerated when the language changes, and its output is committed.
- **`question-flow`** is written by hand: a library of about 30 question kinds, a flow component
  whose state holds the answers and the current step, 30 steps of which several are shown only
  when an earlier answer has a given value, and a lifecycle of 30 batches, one answer each. It
  also has a zero-argument function that returns the whole flow definition, which is the call
  ReachMe makes today.

Neither copies a host's source. Nothing DrawnUI-specific entered this repository for the fiddle,
by choice (`specs/future.md`, "The DrawnUI Fiddle"), the fiddle's catalog is generated from a
package this repository does not depend on, and ReachMe's library is product code.

*Alternative considered:* copy the fiddle's `drawnui.nx` and three of its presets. It is the real
thing, and it is what the `specs/future.md` entry suggested. It ties a corpus program to another
project's generated output and its licence, and it cannot be regenerated here.

### D3. A third workload for calls that take input, driven from existing programs

The corpus pins small arguments. The cost of reading and checking input shows on large ones, which
the driver builds and passes to functions the corpus already has:

- 10,000 records of three fields, in the three shapes `specs/future.md` names: plain data; the
  same data with one member set to `undefined` at the bottom, which takes the full walk; and data
  nested past 32 levels, which the fast check does not settle. They are passed to `held` of
  `host-values`, which takes a value at `object` and returns it, and to a function of
  `question-flow` that takes a list of typed answer records and returns whether there are any, so
  that reading and checking are timed without the cost of writing the value back;
- a tool function of `agent-tool-context`, called by name through `callFunction` with arguments
  and a context record, under `maxOperations` and `maxInputSize`, as `@nx-lang/agent` calls it.

These are not recorded in the corpus: the inputs are built by the driver, and the corpus's own
cases already hold the two runtimes to one result for those functions.

### D4. Phases

| Phase | What is timed | Workloads |
| --- | --- | --- |
| `load` | Importing the runtime module. Cold only. | all |
| `prepare` | `prepareNxIrModule` for every image of the program. | catalog, flow |
| `link` | `linkNxIrProgram` of the entry against the prepared modules. | catalog, flow |
| `evaluate` | `evaluateFunction` of a zero-argument function: `root`, and the flow's definition. | catalog, flow |
| `initialize` | `initializeComponent` with props. | flow |
| `resume` | `initializeComponent` with `options.state` set to a stored state, which is what a host that keeps only the state does before every dispatch. | flow |
| `evaluate with state` | `evaluateComponent` with props and a state. | flow |
| `dispatch` | `dispatchComponentActions` with one answer; a sample is the whole 30-answer script, reported per answer. | flow |
| `input` | `evaluateFunction` given the 10,000 records: the three shapes at `object`, and the two that are not nested as typed records. | calls |
| `tool call` | `callFunction` by name with arguments and a context. | calls |

Every phase that evaluates is measured twice: with no limits, and with `maxOperations`,
`maxInputSize` and `usage` set to values the call does not reach. The second run is where the
operations and the input size in the report come from, and the difference between the two is what
limits cost a host that sets them.

### D5. What a number means

- **Cold** is the first execution of a phase in a fresh isolate, after `load`: a new
  `worker_threads` worker for each sample, 15 samples, reported as the median and the largest. It
  includes the engine compiling the runtime's code for that phase. It approximates the first
  request an isolate serves; it is not workerd's cold start. For `dispatch` it is the first answer
  alone, not the script.
- **Warm** is the phase after it has run until its time stops falling, and for at least 100 ms:
  the median and the 95th percentile of at least 200 samples. A phase shorter than 0.1 ms is
  sampled in batches.
- **Each call is warmed and timed in an isolate of its own.** Timing every step in one isolate
  was tried first and was wrong: a step ran on what the steps before it had taught the engine, so
  the same build timed 13% slower on one step when two more programs ran ahead of it, and
  `initialize` read 17 µs where alone it is 13. A host's isolate runs a mix of calls, so its
  times are not below these; what the isolation buys is that a number does not depend on which
  other steps exist.
- **The two variants of a call are sampled in turn**, one sample of each, in that isolate, so
  that drift falls on both and their difference is what limits cost.
- **`bench` times each call in three isolates and reports the middle one.** An isolate now and
  then settles on slower code for a call: the change's review saw `dispatch` at 78 µs in one
  isolate where 24 others gave 48 to 58. The comparison takes one isolate a side in each round,
  and its rounds do the same work.
- **Operations and input size** come from `usage` and are exact. The report also gives
  nanoseconds per operation, which is the number to compare across phases and with the cost
  tests' time report.

### D6. The core has no clock and is handed the runtime

The first draft gave the core a clock function. A deployed Cloudflare Worker has no clock that
advances while code runs: `performance.now()` and `Date.now()` move only after I/O (local workerd
does advance them). A core that reads a clock around each step would report zero there.

So the core is a plain ES module that, given a runtime module and the corpus programs (each one's
`program.json` and images), returns named steps: `{ workload, phase, subject, variant, ready(),
run() }`, where `ready` does once whatever the call needs (preparing, linking, initializing,
building an input) and `run` makes the call once and returns what it used. A step also says what
one run is divided by to give one call (the 30 answers of a dispatch script), the count the corpus
records for it, and the programs and runtime functions it uses, which is what the comparison reads
to say a step cannot be compared. It builds the large inputs itself, so that a host runs the same
ones. It imports nothing, so:

- the Node driver times `run` with `performance.now()`;
- a host imports the core into its engine, passes its own bundled `@nx-lang/ir-runtime`, and times
  the steps however that engine allows, from outside if it must;
- the comparison passes two different builds of the runtime to the same core.

### D7. Compare against the base revision, not a committed baseline

The first draft committed a baseline of warm medians and failed a phase above twice its baseline.
Two problems:

- **A baseline is one machine's numbers.** Whoever regenerates it moves every number by the
  difference between their machine and the last one, and a CI runner is neither. The allowed
  multiple has to absorb that, which is why it was 2×.
- **2× misses what has actually happened.** The regression this repository has seen was 60%. The
  one it accepted without explaining was 6 to 12%.

Instead, `bench:compare` measures two revisions on the same machine in the same run:

- **Base** is, by default, the merge base with `origin/main`; `--base <ref>` names another. Its
  runtime is `runtime/typescript/dist/src` at that revision and its images are the corpus's
  `expected/` files at that revision, both read with `git archive` into a temporary directory. No
  build is needed, because both are committed and CI keeps them current.
- **Head** is the working tree's `dist` and images.
- There is a set number of rounds (default 7). In a round every call is timed once for each
  side, one straight after the other, each in a fresh isolate, with 40 warm samples; which side
  goes first alternates. A round gives a warm median for each phase and side. Running a whole
  side and then the other, each in its own process, was tried first: the two timings of a call
  were then seconds apart, a burst of load on the machine fell on one of them, and ratios between
  identical builds ranged from 0.4 to 3. Back to back a
  round's ratio is beyond 10% one time in twelve, and the median of the rounds stays within it.
- A phase is **named as slower** when the median of its rounds' ratios is above 1 plus a
  fraction (7%) and at least three quarters of the rounds, 6 of 7, are above 1 plus half of it;
  and as faster by the same measure the other way. The report gives both medians, the ratio and
  its spread across rounds.
- **Cold times are compared too**, at no cost in time: each round's fresh isolate takes its first
  call of the variant with no limits as a cold sample before it warms, and the time the runtime
  module took to load there. A call's cold time is one sample a round and is named beyond 20%;
  the load time is the median over a round's isolates and is named beyond 7%, as a warm time is.
- A phase is **not comparable**, and is reported as that and not as slower, when the program's
  `.nx` sources or its `program.json` differ between the revisions, when the base has no such
  program, when the base runtime lacks a function the phase calls, or when the phase fails on
  either side. The command exits with 1 when it names a phase as slower and with 2 when it could
  not compare at all, so that the two are told apart. The steps and the inputs
  the driver builds are always the head's.
- `--base-runtime <dir>` and `--head-runtime <dir>` name a build directly, for comparing two
  builds that are not revisions, which is how the tasks seed a regression.

The rule and its fractions are set from measured noise (tasks 3.3 and 5.1). A build was compared
with itself forty times, twenty on a desktop and twenty on `ubuntu-latest` runners. One round's
warm ratio is more than 10% from 1 about one time in twelve and ranges from 0.32 to 2.55; the
median of seven stayed within 0.90 to 1.10 over 1,200 steps. The first rule, more than 10% in 6
of 7 rounds, named none of them, but it names a real slowdown of 10% about one time in twenty and
one of 15% four times in five, which is how a seeded delay of 19% was missed at a fraction of
15%. The rule above names none of the 1,200 either, and names the same ratios made 10% larger
84% of the time, 12.5% larger 92% and 15% larger 96%. With agreement at 0.3 of the fraction in
place of half it named 2 of the 1,200. For cold times at 20% it named none of 340, and names a
step 30% slower nine times in ten; for load, none of 20.

Seeded in a copy of the build and compared with the unmodified one, on a desktop and on runners:
a delay of 15 to 19% in dispatch is named, alone; one of 4% is not; 9% more work at module load
is named as `load`, alone; and the host-value fast check disabled is named on every call that
takes plain host values (1.4 to 1.5 times at `object`, 1.1 on typed records, 1.2 to 1.7 on the
tool calls) and on nothing else but the two calls that take a state, which are 1.05 to 1.12
times, at the edge of the fraction: both are named on a runner with no limits, and on the desktop
`resume` alone. A comparison of all 30
variants takes two to three minutes on a runner, cold times included.

Taking each revision's own images means a change to the emitter is compared as well: the base
runtime runs what the base compiler emitted. And the operation counts of both revisions are in the
report, so a phase that costs more operations is shown with no noise at all.

*Alternative considered:* keep a committed baseline and normalize it by a calibration loop run on
the current machine. It keeps a history of numbers in the repository, and it still compares two
machines through one scalar, which does not carry over between a laptop and a runner for code as
unlike as a module reader and an evaluator.

### D8. CI

One job in `build.yml`, `continue-on-error: true` as the cost time report is, Node only:

- on a pull request, `bench:compare` against the revision the change is based on, with the table
  written to the job summary and the JSON uploaded as an artifact;
- on a push to `main`, `bench:compare` against the last release tag, and then `bench` alone, both
  in the summary and uploaded. The release is the last one before the commit (`git describe` of
  `HEAD^`), so a commit that is itself a release is compared with the one before it. The
  comparison runs with `--report-only`: a call slower than in the release is named and does not
  fail the step, which would otherwise fail on every push until the next release, and any other
  exit is a failure to run.

The comparison on a pull request passes what is under 7%, so several small slowdowns can add up
unnamed. Comparing `main` with the last release is what names them, with the mechanism that is
already there: no history of timings is kept. A step whose program changed since the release is
not comparable, so the two large programs are compared from the release after this one. The
release steps (`docs/deployment.md`) say to read that comparison before a tag.

It builds nothing: the runtime build and the images are committed. It needs the base revision's
objects and the tags, so the checkout fetches all history.

## Risks / Trade-offs

- **[A shared runner is noisy]** → Timing the two sides of a call back to back in one job cancels
  most of the difference between runners; a phase is named by the median of its rounds with
  three quarters of them agreeing (D7); the job does not block. Tasks 3.3 and 5.1 measured the
  noise by comparing a build with itself, on runners, before the fraction was fixed.
- **[The workloads drift from real hosts]** → Their shapes are measured from real programs and
  written down in D2. A host that finds a phase the workloads do not predict contributes the
  shape, not its source.
- **[The comparison trusts the committed build]** → CI already fails a revision whose `dist` is
  not what its source builds to. A local comparison of an unbuilt tree would time the old build,
  so `bench` and `bench:compare` build first.
- **[Large corpus programs slow the corpus tests]** → Evaluating them is milliseconds. The one
  quadratic check, the cell sweep, leaves large images out (D1). Task 1.3 measures both runtimes'
  corpus tests before and after.
- **[`retire-hir-interpreter` changes where expected results come from]** → Whichever change
  lands second regenerates these two programs with the rest of the corpus; nothing here depends on
  the interpreter beyond what every corpus program does.

## Migration Plan

None; tests and tooling only.
