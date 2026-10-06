## Context

See proposal.md for motivation. The facts that shape the design:

- `add-ir-runtime-evaluation-budget` charges an operation for every node evaluated, item placed,
  value checked against a type, pair compared and value written for the host, and text by its
  length. A value at the type `object` is checked as one value whatever it holds, so a host value
  there enters an evaluation for one operation.
- Its review (`review.md`, RF8 to RF20) found steps that did work in proportion to a value's size
  for a fixed count. Those a program could drive with a value it built itself (a list bound to a
  content parameter, state walked after a patch) are bounded by the budget, since building a value
  costs its size. Those that needed a shape only a host can supply were not bounded at all: a
  field name of a megabyte, an untyped object 80,000 fields wide, a list of 20,000 `null`s. A
  program cannot make those: names are declared, a record's width is its declaration's, and
  sequences flatten empty values away.
- The Rust runtime converts every host value on the way in (`from_host`, `fields_from_host`), one
  recursive walk per value, bounded in nesting at 256. It does not convert them all at the entry:
  `dispatch_component_actions` converts each entry of a batch as it reaches it, after the earlier
  entries' handlers have run, and `initialize_component` and `evaluate_component` convert a state
  after the props are normalized and their defaults evaluated. The TypeScript runtime uses host
  values as they arrive and walks nothing until a typed site checks them. A JavaScript host value
  can hold the same object twice, or itself.
- The two runtimes are given different forms of one input. An integer outside the safe range is
  `NxValue::Int` to a Rust host; canonical JSON spells it as the record
  `{ "$type": "nx.int", "value": "<digits>" }`, which at `object` is a record to both runtimes:
  the TypeScript runtime does not hold such an integer at all, and the Rust runtime knows one by
  its own type and not by that shape (the budget change's RF28 and RF31). A JavaScript host can leave a map out, pass `undefined`, or pass values that
  are no canonical value.
- The conformance corpus evaluates function entrypoints with no arguments. Lifecycles carry props
  and batches, but no corpus case supplies a host value to a function, so every host-value case
  the review found is pinned only by each runtime's own tests.
- `nx-api`'s source evaluation, which records the corpus's `results.json`, takes no arguments.
  `operations.json` is already recorded from the Rust runtime. Regenerating the corpus rewrites
  `manifest.json`.
- `crates/nx-codegen`'s unit tests compile NX source, link it in `nx-ir-runtime`, and already run
  `node` for the generated-JavaScript check. The crate has no `tests/` directory.
  `runtime/typescript/dist` is committed. CI runs `cargo test --workspace` before `pnpm -r build`.
  `crates/nx-ir-runtime/tests/allocation.rs` has a counting global allocator, alone in its binary.
- The workspace has no property-testing dependency in Rust or in the TypeScript packages, and
  neither runtime package has a changelog.
- A runtime does not say what a call cost. The budget change left that out, so the corpus finds
  each count by doubling and bisecting the budget, about 25 evaluations for one number.
  `RuntimeOptions` in the Rust runtime is `Copy` and holds numbers only.
- CI runs `cargo test --workspace` and `pnpm -r test` as blocking steps, and has one step that
  does not block (`continue-on-error`).

## Goals / Non-Goals

**Goals:**

- A host bounds, with one number, how much it hands an evaluation, measured the same way in every
  runtime.
- With a limit `C` and a budget `B`, a step that is linear in a host value and is not charged, found
  or not, costs at most about `B × C` units of work in one call, where a unit is a value or 64 code
  units of text. At the numbers suggested for the agent package, 100,000 and 1,000, that is about
  10^8: seconds at the worst (2 to 7 in the TypeScript runtime, by the budget review's timings),
  and a ceiling, where the review's cases had none. It is the cost of a step nobody has found,
  not of an ordinary call.
- Host-value cases are pinned across runtimes by the corpus, not only by each runtime's tests.
- An uncharged step shows up in a test run: as a disagreement between runtimes, as allocation out
  of proportion to the count, or as time that grows faster than the count.
- A host can read what a call cost, to log it and to choose a budget from measurement, and the
  tests read a count in one evaluation.
- No operation count changes, and a host that sets no limit pays nothing.

**Non-Goals:**

- Making cost proportional to the count for values a program builds itself. Those are bounded by
  the square of the budget whatever the limit is; closing a step of that kind is a change to the
  cost model, in a change of its own.
- Bounding a step that is worse than linear in a host value. The limit makes such a step's cost a
  function of the limit, not of what arrived, and that is all.
- Bounding what an instance holds. Its state was written under the budget of the call that wrote
  it, so it is bounded by that budget when every earlier call had one, and by nothing when a host
  ran one without. A host that limits input and not operations has not bounded state.
- Charging host input to the operation budget. Decision 3 says why.
- A limit that applies only at `object` sites. Decision 1.
- Generating NX programs. The programs are a fixed library and the host values are generated; every
  break after RF1 came from the input side. A program generator is a possible later change.
- A bound on the nesting of a host value in the TypeScript runtime. The measuring walk does not
  recurse (decision 4), and deep values within the limit reach evaluation as they do today.
- A bound on what a host's own code does when its values are read: a getter or a Proxy is the
  host's code, and a runtime cannot limit it.
- Running the generated-JavaScript check on corpus cases with arguments. Their results are
  recorded from one runtime and checked in the other, which is the comparison this change is for.
- The limit in `nx-api`, the CLI and the .NET, Node and wasm bindings, as for the budget.
- Fixing what the tests find (the proposal's scope rule).

## Decisions

### 1. One limit over everything the host supplies to a call

The limit covers every value the host passes to one call, typed or not, measured once at the
entry. The alternative is to limit only what reaches an `object` site, which is the only place a
host value is not already charged value by value. But at an `object` site a runtime cannot tell a
value the host supplied from one the program built, and the check there is a single operation on
purpose: a program-built value that holds another twice is cheap to hold and must stay so. Telling
them apart would mean threading "this came from the host" through every type check in both
runtimes. Measuring at the entry is one walk in one place, and typed input being counted twice,
once by the limit and once by its type check, costs nothing a host would notice.

A component instance is not input. The runtime produced it under earlier budgets, where what it
holds was charged as it was written. The update helpers (`applyUpdate`, `mergeUpdates`,
`diffRecords`, `changedFields` and their Rust forms) take no options and evaluate no program code,
and are outside the limit as restoring an instance is.

### 2. The unit is the size a value has when written

The input size uses the measure the cost model already defines for a value written for the host:
one per value, a sequence, the empty value and a `null` included, and one per 64 UTF-16 code units
of a string, a type name and a field name. It counts exactly the things the review's breaks were
made of (items, fields, long text), and a host can reason about it: a value costs the same to hand
in as to get back.

Maps the host passes (arguments by name, props, a state, a patch) are measured as one record each,
so their names count like any field names.

The runtimes compute this measure today for values they write, and agree on it, for every value a
program can produce. They are not given the same forms of input, so the spec fixes the cases where
the forms differ (`specs/nx-ir-format`, the list under the first requirement):

- **The JSON form of a wide integer is a record.** The `nx.int` record is measured as any record
  is, in both runtimes: two for a real integer's digits, and 16,386 for a megabyte of text
  spelled as one. No measure has to recognize it. `NxValue::Int` is one value in the Rust
  runtime whatever its size; the TypeScript runtime holds no integer outside the safe range and
  has no other form of one to measure. This follows the budget change, where both runtimes read
  the record at `object` as a record (its RF24, RF28 and RF31).
- **Props or named arguments left out are an empty record**, size one, so that a TypeScript
  host's `undefined` props and a Rust host's empty map measure alike. A state is optional in both
  runtimes, and one the host does not pass is not input.
- **`$type` holding a string is a type name** wherever it appears, in a record or in a map
  measured as one; holding anything else it is a field.
- **An `undefined` element of the positional arguments, or a hole in them, is the empty value**,
  size one. A parameter the list does not reach is not input, so the size does not depend on the
  declaration.
- **A value that is not a canonical value is one value and is not entered.** In JavaScript that
  is anything but `null`, a boolean, a number, a string, an array or a plain object: a function, a
  big integer, a `Date`, a `Map`, and the handler and function objects the runtime keeps for its
  own use, which a walk must not follow into the linked program. The runtime knows its own by
  having made them (a handler by identity, a function value by its class), not by a marking: an
  object from `JSON.parse` that carries `$nxKind` is a plain object and is measured as one. What
  the runtime then does with a value that is not canonical, which is mostly to refuse it, is
  unchanged.

*Alternative: bytes of JSON.* A host knows that number before it calls, but the runtimes do not
see JSON: the Rust runtime is given `NxValue` and the TypeScript runtime objects. A host that
wants a byte limit applies it where it has the bytes.

### 3. A limit of its own, charged to nothing

`maxInputSize` is a separate option, reported as `nx-ir-resource-limit` with its own limit name.
Measuring charges no operations.

*Alternative: charge the input to the operation budget.* One number instead of two, and input size
could never exceed the budget. Rejected: it changes every count that has arguments and every
recorded corpus count again; and it bounds a host value only by the budget, so an uncharged step
is still worth the budget squared. A small separate limit is what makes the product small. A host
that wants both sets both.

The limit is unset by default, like the budget, and for the same reason: a host that runs its own
programs passes itself large values on purpose.

### 4. Measured at the entry, by a walk that stops at the limit

The measure is taken before anything is checked or evaluated, and the walk stops as soon as its
count passes the limit. In both runtimes it is a pass of its own over the host's values, at the
top of each public method, that builds nothing:

- **Rust.** A count-only pass over the `&NxValue` arguments, with an explicit stack and no
  recursion, run before the method looks up its entrypoint or converts anything. It cannot ride on
  `from_host`, because three methods convert some of their input after evaluation has begun (see
  Context), and the limit has to refuse a batch before its first handler runs. A string, a type
  name or a field name is refused from its byte length when that alone passes what is left of the
  limit (a UTF-16 code unit is at most three bytes of UTF-8, so `bytes / 192` is a lower bound on
  its cost), and is scanned only otherwise, so the text scanned in a call is at most 192 bytes
  for each unit of the limit. The conversion that follows is unchanged, including
  its nesting bound.
- **TypeScript.** Each exported evaluation function measures its host values before it does
  anything else, inside the wrapper that already validates `maxOperations` and converts engine
  errors. The walk uses an explicit stack, not recursion, and treats every reference it meets as a
  new value, so an object that holds itself counts up to the limit and is refused. Arrays are read
  by index. An object's names have to be listed before any is read, and the engine lists them all
  at once, so the one object at which the limit is passed costs an enumeration of all its names:
  measured at 410 ms for two million names in Node 24. That is the bound the spec states: the
  limit, plus one such listing at most once in a call. To keep it at once, the walk counts an
  object's members when it lists them. Every member costs at least one, except a `$type` that
  holds a string, so an object with more members than the limit has room for is refused at that
  listing with none of them read, and the members of an object that is entered are reserved
  against the limit until they are read. The names listed in a call are then at most twice the
  limit and those of the one object refused. Without the reservation a wide object that holds
  itself was listed once for every unit of the limit and exhausted the heap before the limit
  refused it (the implementation review's RF29 and RF35).

With no limit there is no pass in either runtime.

`maxInputSize` is validated as `maxOperations` is: a value that is not a non-negative safe integer
is refused with `nx-ir-options`. The failure names no declaration and has no span, since nothing
has been evaluated.

Order at the entry: validate the options, measure the input, then proceed. The input is refused
before the program is looked at, so a wrong entrypoint name, props of the wrong type or an
exhausted budget with oversized input all report the input. The same holds in Rust for the nesting
bound: input over the limit is reported for its size, and input within the limit that nests more
than 256 deep is refused for its nesting in the conversion, as today. The TypeScript runtime has
no nesting bound on host values, so that last case is the one place the two differ, and the spec
says so.

### 5. Corpus entrypoints with arguments

`program.json` entrypoints gain two optional members that go together: `arguments`, a list of
canonical values passed by position, and `case`, a name. An entrypoint with `arguments` and no
`case` is refused by the corpus tests. Results, counts and failures are keyed
`identity::function#case`; an entrypoint with neither member keeps its key.

A program whose manifest sets `recordInputSizes`, as `recordFailures` is set today, has
`inputSizes` written to its `operations.json`: one for each case, and one for each lifecycle's
initialization and each of its batches. They are checked through the limit as counts are checked
through the budget: accepted at `n`, refused at `n - 1`. Only the new program sets it, so no
existing program's `expected/` files change. `manifest.json` and the corpus README do change.

Everything that reads corpus keys has to learn the case form: the regeneration and the count
check in `ir_corpus_tests.rs`, `corpus_results_match_the_interpreter` there (which skips cases
with arguments, since the interpreter's source evaluation takes none), the two loops in the
runtimes' corpus tests that split a failure's key on `::`, and `tests/damage.rs`.

Expected results for a case with arguments are recorded from the Rust IR runtime, as operation
counts are; `retire-hir-interpreter` makes that runtime the source of all results in any case.
Regeneration reads each count and input size from the usage report of decision 12 and no longer
bisects. The check the runtimes make stays as it is, a budget of `n` and one of `n - 1`, since
that is what says the recorded number is the least that works; each runtime also checks that its
usage report gives the recorded numbers.

The cases live in a new corpus program, `host-values`, with one lifecycle, so the programs that
pin the emitter are not touched. They use only inputs that both runtimes accept, spelled alike,
with the same result. The `nx.int` record at `object` is one of them, a record in both, and has a
case. A `null` inside a value at `object` gives different results, so it goes in the generated
cases of decision 6. The
kinds of input the corpus cannot express, arguments by name, content, a state passed in and a
patch, are tested in each runtime against the sizes the format document works out.

### 6. The differential harness is an integration test driven from Rust

The generator, the probe library and the harness live under `crates/nx-codegen/tests/`, the first
integration tests of that crate, because the allocation check of decision 7 has to be a binary of
its own and must see the same generator. `tests/cost/mod.rs` holds the generator, the probes and
the helpers; `tests/cost_differential.rs` and `tests/cost_allocation.rs` include it. `nx-codegen`
is the one place that has the compiler, the Rust runtime and a way to start `node`.

1. Compile the probe library, a single NX source file kept beside the tests, and write its images
   to a temporary directory. The library holds no integer literal outside JavaScript's safe range
   and no arithmetic that can leave it: the TypeScript runtime refuses such an integer where the
   Rust runtime computes with it, so a probe that reached one would differ on every case for a
   reason that has nothing to do with cost.
2. Generate the cases from a seed. A case is a probe, its host values, and a repeat count where
   the probe repeats a step, at two scales: a base, and a scale with the values eight times the
   size and the repeats eight times as many. The base repeat count is 16 and the scaled one 128.
   Write the cases as JSON, which both runtimes read: an integer outside the safe range is the
   `nx.int` record in both, since JSON has no other spelling and each runtime reads that as a
   record at `object`.
3. For each case, read the Rust runtime's operation count and input size at both scales from
   its usage report (decision 12), one evaluation each. At the base scale also find its result
   and its failures: at every budget below the count when the count is at most 200, and
   otherwise at a quarter, a half and one less. Check that the result under a budget equal to
   the count is the result under none.
4. Start one `node` process on a runner script in `runtime/typescript/test/`, which reads the
   images and the cases, does the same with the TypeScript runtime, and writes its answers as JSON.
5. Compare results, counts, input sizes and failures. A difference fails with the seed, the probe,
   the case as JSON, and both answers.

Counts and input sizes are compared at both scales, since each is read in one evaluation.
Results and failures under smaller budgets are compared at the base scale only: they are what
takes many evaluations a case, and the larger scale adds size, not kinds of case.

Results are compared as the corpus tests compare them (`canonical_eq`): numbers by value, so `2.0`
is `2`, and a wide integer in one spelling. The probes and shapes on a list in the test of the
differences the format documents (a `null` inside a value at `object`) are excepted; for those the
count and the failures are still compared.

The generator pairs a shape only with the probes that both runtimes accept it at. The rules are
in one table in the test: a `null` is never a whole argument at `object`; the `nx.int` record, whatever its digits, goes to untyped sites only; a value goes to a typed probe only when it fits the type; and nesting is not scaled, at either scale, past the 256 levels the Rust runtime accepts in a host value, since counts are compared at both.

The count is read from the usage report, under a budget large enough that it cannot be reached.
`cargo test` builds without optimization, so the case count is still chosen to keep the test under
a minute there: 400 cases, at about 26 ms a case, which with the allocation test is about 15
seconds on a laptop and leaves room for a slower runner. Cases are given to the Node runner 200
at a time, so that a longer search by hand holds only a part of its values at once.

The probe library holds 41 probes, more than the operations the spec lists. `compareSelf` and
`compareUnequal` compare a value with itself and a record that holds a number not equal to
itself, without which a runtime whose equality depended on the budget would not show. And three
probes hand a call more than it declares, which the implementation review asked for (RF33): the
content a host passes to a descriptor, arguments by name the function does not declare, and
members an invocation record does not define. One pairing rule came from the first run: a `null`
is not an item of a list a probe splices into a list typed `object+`, where each item is a whole
value at `object`.

The generator is a small seeded pseudo-random generator, with no new dependency. It draws a shape
(scalar, string, list, object, nesting of those), then sizes from a distribution weighted toward
the boundaries the cost model has: 0, 1, 63, 64 and 65 code units, a few hundred, a few thousand.
Strings draw from ASCII, two-byte and astral characters so that code units and bytes differ.
Objects draw names that are short, long, and shared or not shared between two values of one case,
with and without `$type`. One dimension is the names a runtime gives a meaning to: the type names
`nx.int`, `Function`, `ActionHandler`, `ActionHandlerInvocation` and one ending `.Update`, and the
key `$nxKind` with the values the TypeScript runtime uses, each with small and with large
contents, in one JSON spelling that both runtimes read. The large contents go both in the member
the form defines (the `value` of an `nx.int`) and in a member it does not (an `extra` beside it):
a runtime that recognizes a form by one member and ignores the rest hides whatever the rest
holds. These are the shapes a runtime is most likely to treat as its own and not look inside.

Reserved names also make cases that both runtimes refuse with no budget set: a handler record
where no parent is given, an invocation whose token names no handler. Such a case has no count
to find. The runtimes are compared on the code of the diagnostic, and the case is run at no
other budget.

On a failure the harness shrinks by size alone: it runs the failing probe again with the same seed
at smaller size parameters and reports the smallest that still fails. That is cruder than a
property-testing library's shrinking and needs no dependency in two languages.

The seed and the number of cases are fixed in CI, so the run is the same every time; two
environment variables override them for a longer search by hand.

The Node runner imports the committed `runtime/typescript/dist`. It does not compare file times to
decide whether `dist` is current: a checkout writes `dist` before `src`, so that check would fail
on every fresh clone. A stale `dist` is caught where it is made, by a CI step that fails when,
after the build, `git status --porcelain --ignored runtime/typescript/dist` lists anything: a
built file that differs, or one that was never committed. `--ignored` is needed for the second,
since the package's `.gitignore` lists `dist/` and its files are added by force.

With `NX_COST_DIR` naming a directory, the harness keeps the images, the cases and the answers
there instead of in a temporary directory, so that the runner can be run on them by hand. Each
test that keeps its files has a place of its own under it, since the tests run at the same time.

*Alternative: a property-testing library in each language.* Two generators would have to produce
the same cases, or the comparison would be of two different samples. One generator, and JSON
between the processes, is the simpler way to make both runtimes see the same input.

### 7. Allocation is checked in Rust, where it can be counted

`tests/cost_allocation.rs` has a counting global allocator, alone in its binary as
`nx-ir-runtime`'s `tests/allocation.rs` is. It runs every generated case at both scales, reads
the count and the input size from the usage report, and asserts two things, where `units` is the
operation count plus the input size:

    bytes requested ≤ K × units + C
    bytes per unit at the larger scale ≤ 2 × bytes per unit at the base

The first is an absolute ceiling. `K` and `C` are constants in the test, set from a first
measurement of the whole suite with headroom of about four times the largest ratio seen, and
recorded with that measurement in the test's comment: `K` is 3,000 bytes a unit against a largest
ratio of 720, and `C` 4,096 bytes. A prepared module decodes a node the first time an evaluation
reaches it and keeps it, which belongs to the module and not to the call, so each call is made
once before it is measured. The second is what finds an uncharged copy,
and it does not depend on the calibration: a step that copies a value on each repeat for a fixed
charge allocates sixty-four times as much at the larger scale for about eight times the units, so
its bytes per unit grow eightfold.

Neither assertion is meant to catch a step that copies the input a fixed number of times in a
call. Its bytes grow with the input size, which is in `units`; it is bounded by the input limit
and is not a break of the cost model.

Bytes requested are the same on every run of one build, which is what makes the check fit to
block a merge. They are not promised to be the same in an optimized and an unoptimized build, so
the test asserts the bounds and no exact number, and the task that sets `K` measures both builds.

The TypeScript runtime has no such measure: the engine does not expose allocation, and heap
readings move with the collector. Its allocation is covered by the time report below and by the
fact that the two runtimes charge alike.

### 8. Time is compared with the count, and does not block

For each case the harness times both scales, in both runtimes, taking the least of five runs, and
reports a case when

- its time at the larger scale is more than twice the growth of its units (operation count plus
  input size) times its time at the base, and
- its time at the larger scale is past a floor, set for each runtime from the noise measured in
  it on small cases.

The report runs in an optimized build and is not held to a minute. It uses the generator's sizes
as the blocking tests do and multiplies the repeats of both scales by eight, so a case repeats its
step 128 times at its base scale and 1,024 times at its larger one. Task 9.3 re-introduced a hole
of each kind, in a runtime that had it, and recorded its time beside that runtime's floor, and
this is the multiplier at which each is reported. The honest work of a call, converting its input
and writing its result, is about a hundred nanoseconds for each value in either runtime, and a
walk or a copy that is not charged is one or two, so such a step shows beside the honest work
only when it is repeated several hundred times: with the repeats as the blocking tests have them
the walk of the whole state on every patch grew its case's time 1.8 times as fast as its units,
under the twofold the report looks for. Multiplying the sizes of the values as well was tried and
dropped: a string or a result eight times larger again leaves the processor's caches, its time
for each unit doubles for that alone, and the report named nine honest cases.

The floors are 2 ms in the Rust runtime and 3 ms in the TypeScript runtime, each two and a half
to three times the longest small case that outgrew its units on the unmodified runtimes. A case
that looks reportable after its five timings is timed ten more times and the least kept, which is
still the least of several runs and sets aside most of what one noisy timing reports.

Scaling the value and the repeats together is what makes one rule fit the review's breaks. A copy
on every call (RF8, RF10, RF20, and the flat-count rows of RF17) grows its count eightfold with
the repeats and its time sixty-four-fold. A walk repeated within one call (RF9 and RF18: the whole
state walked once for every patch of a batch) does the same, which no run that scaled the value
alone could show, since writing the state makes the count grow with it. A step that is quadratic
in a value for a linear count (RF11) grows its time sixty-four-fold against eight. Honest work
that is not charged at all, as the Rust runtime's one conversion of a host value on the way in,
grows with the input size, which is in the units, so it is not reported.

This runs as its own command and its own CI job, with `continue-on-error`, beside the job
`add-ir-runtime-performance-harness` adds if that has landed. It reports and does not fail a
merge: a timing test that blocks will be switched off the first week it flakes.

### 9. Known findings are listed, and the list is checked

When a check finds a step that is uncharged, the fix belongs to the cost model and to a change of
its own. So the tests carry a list of known findings. An entry has:

- one concrete case, written out in full, that shows the step: a probe, its values and repeats,
  and which check it fails;
- a predicate over generated cases (a probe and a condition on the shape) that says which of them
  the finding covers;
- a line saying what was found and where it is tracked.

The blocking tests skip the generated cases an entry's predicate covers, run the entry's own case
directly, and fail when that case passes its check, with a message saying to remove the entry.
Because the case is concrete, the rule does not depend on the seed, and a longer search under
another seed neither trips it nor hides behind it. That keeps the suite green while a finding is
open, keeps the list honest, and makes the fix's change the one that deletes the entry.

A finding only the time report shows has an entry of the same form that the report marks as
known. It is exempt from the must-reproduce rule: time cannot be required to reproduce in a test
that blocks. Such an entry names the runtime it was seen in, and the report takes a case it
covers as known for that runtime only, so the other runtime outgrowing its units on the same case
is still reported (the implementation review's RF32).

The blocking tests found nothing: the list holds no differential and no allocation finding. Its
three entries are the time report's, and none is a step that is not charged: wide objects compared
or written in the TypeScript runtime, whose time for each field rises about two and a half times
between a few hundred fields and a few thousand, and results of half a million values and more in
each runtime. They are tracked in `investigate-cost-time-report-findings`.

### 10. The agent package's default

`add-agent-host-package` applies a default budget to tool calls and states why. It gains limits
on input the same way, for function-tool calls and arguments-function calls alike, and it keeps
what the model sends apart from what the host supplies:

- **The model's arguments have a cap of their own**, `maxArgumentsSize`, 1,000 by default. The
  package measures the input object with `measureInputSize` (decision 13) before it calls the
  runtime, and input over the cap is a failure with code `invalid-input` and a `limit` naming the
  option. The model sent it and can send less; `resource-limit` would tell it the tool ran too
  long, which it cannot act on. A tool's arguments are tens to hundreds of values, so 1,000 is
  ample.
- **The host's context has a separate allowance**, `maxContextSize`, and the package measures it
  too: the context record as it is passed, `callId` included, with `measureInputSize` before the
  call. Over the allowance is a failure with code `resource-limit` and a `limit` naming
  `maxContextSize`: the host's to fix, and nothing the model did. A `ToolContext` record is the
  host's own and may be large, and under one shared number it would eat the model's allowance and
  the model would be blamed for it. The default is set from a measurement (task 6.1): the context
  of the `agent-tool-context` corpus program, and ReachMe's if its author supplies one, times
  four and rounded up to a thousand, and 1,000 at the least. Measured, the corpus program's
  context is 4, so the default is 1,000; no ReachMe context was supplied.
- **The runtime's limit is a backstop, set by the package.** Both parts are already held to their
  numbers, so the package sets `maxInputSize` to what a call within them can reach: the arguments
  cap; for each context parameter of the function, the context allowance (the same record is
  passed to each) and what the parameter's name costs as a field name of the arguments record,
  one for every 64 code units, which neither measurement holds; and the size of the function
  record. A call that passes both checks cannot
  pass it, so the model's mistake is never reported as the runtime's limit. A `maxInputSize` in
  the host's own runtime options is not used for these calls: a smaller one would refuse input
  the package had accepted and report it wrongly, and a host raises the limits through the two
  options that name what they bound.

Where this is written depends on what has landed: in that change's design, spec and tasks while
it is open, which is where this change puts it now; in the package, its tests and a `MODIFIED`
delta to its main spec added to this change once it is archived.

### 11. Stages

Stage 1 is the limit, the corpus cases and the agent package's default: the part a host needs.
Stage 2 is the differential harness. Stage 3 is the two proportionality checks, which reuse the
harness's cases. Each stage ends with the repository's tests passing and can be merged alone.

### 12. A call reports what it used, to a sink in the options

The budget change left usage reporting out. Three things want it now: a host that logs what a
tool call cost or chooses a budget from what its programs really use (setting the agent package's
default from measured tools is still open); the corpus regeneration; and the tests of stages 2 and
3, where finding a count by bisection is about 25 evaluations and reading it is one. The Node
runner uses the published package, so a hook for tests alone would have to be exported anyway. It
is a public option.

- **TypeScript.** `NxRuntimeOptions.usage` is an object of the host's that the runtime writes to:
  `operations` and `inputSize`. The runtime removes both at the start of a call and sets them
  when the call ends, on a return and on a throw, inside the wrapper every exported evaluation
  function already has. A callback was the alternative; it would run the host's code inside the
  runtime's wrapper, where an exception from it has no good meaning. The same care is owed to
  the sink. It is validated with the other options, and before them, so that a call refused
  for another option still leaves the object cleared of an earlier call's numbers: at the start
  of the call the runtime writes both members to it and removes them again, and a `usage` that
  is not an object, or on which either step fails, is `nx-ir-options` before anything runs. The write is the test:
  removing a member a frozen or non-extensible object does not have succeeds, and only writing
  to one fails. The write at the end
  is guarded: if it throws, as a Proxy's trap or a setter can, the exception is dropped and the
  call's own result or error stands. The counter a failed call leaves is not what is reported:
  the TypeScript runtime subtracts a charge and then tests it, so the report adds a refused
  charge back, and both runtimes report the operations charged before it.
- **Rust.** `RuntimeOptions` gains `usage: Option<Arc<Usage>>`. `Usage` holds two atomics behind
  `operations()` and `input_size()`, each `Option<u64>`, so the options stay `Send`, `Sync` and
  cheap to clone. They keep `Debug` and `Clone` and lose `Copy`, `PartialEq` and `Eq`, which an
  `Arc` of atomics cannot derive; nothing in the workspace compares two options, and the tests
  and callers that copied them clone them. `Machine` borrows the options and no longer copies
  them, so a call clones nothing. The report is filled in one place: each public method runs its
  body through one helper that writes the report from the machine when the body returns, `Ok` or
  `Err`, so no early return through `?` can skip it. The project needs no backward compatibility
  yet. A lifetime on the options, or a second argument on every method, were the alternatives,
  and each touches every caller for good.

Operations are reported only when a budget is set: with none, a walk over a value skips its
charges altogether, which is what keeps the budget free for a host that does not use it, and a
report must not bring that cost back. A host that wants the count and no limit sets a budget it
cannot reach. The input size is reported only when an input limit is set, for the same reason,
and only when the input was within it, since a refused input was not measured to the end.

The number reported for a call that succeeds is its operation count as the corpus defines it, the
least budget that works: charging is the same under every budget, so what was charged under a
large one is what a tight one must hold. Both runtimes' corpus tests check that against every
recorded count.

### 13. The measure is exported

`measureInputSize(value, limit?)` in the TypeScript runtime, and `input_size(&NxValue,
Option<u64>)` in the Rust one, return the size of one value as the limit measures it, stopping at
the limit. It is the walk of decision 4 under a public name. A Rust host holds props, a state, a
patch and arguments by name as a `BTreeMap<String, NxValue>`, so the Rust runtime exports
`record_input_size` for a map as well, measured as the one record a call measures it as; a
JavaScript host passes a plain object either way. A list the host measures as one value is one
more than its entries add to a call that takes them as arguments, content or a batch, which the
doc comments say. The limit is optional, but a JavaScript value that holds itself has no finite
size, and measuring one with no limit does not return, which the doc comment and the README say.
The agent package needs it to hold
the model's arguments to a number of their own (decision 10), and any host that wants to limit one
part of what it passes, or to report a size before it calls, needs the same.

## Risks / Trade-offs

- [A limit set too low refuses legitimate input] → Unset by default. The failure names the limit
  and its value, so a host that outgrows its number sees which number.
- [The limit is not a proportionality guarantee: an uncharged step driven by a program-built value
  is still worth the budget squared, a step worse than linear is bounded only as a function of the
  limit, and state is bounded only when every call that wrote it had a budget] → Stated in the
  proposal, the format document and both READMEs, which keep their advice that a host running
  code it did not write also limits time and memory. Stages 2 and 3 are what look for those steps.
- [Refusing one very wide JavaScript object still lists all its names once] → Stated in the spec
  as the bound. It is once in a call and not multiplied by the budget.
- [Fixed programs miss a step only an unusual program reaches] → The spec lists the operations
  that have probes and requires a reason for one that has none; a new operation in a runtime
  needs a probe. A program generator is left as a later change.
- [The time report is noisy] → It does not block, it takes the least of five runs, it needs time
  to outgrow the units twofold over an eightfold scale, and it ignores times under the floor.
- [The allocation ceiling is calibrated, so a legitimate change in representation can trip it] →
  The headroom is fourfold, the constants are in one place with the measurement that set them, and
  the relative assertion, which needs no calibration, is the one that finds copies.
- [`cargo test` now depends on `node` and on the committed `runtime/typescript/dist` being
  current] → It already depends on `node` for the generated-JavaScript check. CI fails when the
  build changes `dist`, so a stale one does not reach `main`.
- [The differential test is slow without optimization] → Counts are read, not bisected, and the
  case count is set from a measured time per case to keep the blocking run under a minute; the
  longer search is run by hand.
- [A usage sink is one more thing in the options, and in Rust it costs `Copy`, `PartialEq` and
  `Eq`] → It is one
  optional field, absent by default, and nothing is counted for it that a budget does not already
  count.
- [A host shares one usage object between calls and reads a stale number] → The runtime clears it
  at the start of every call given it, and a call is synchronous, so a host that reads it straight
  after the call has that call's numbers. Calls a host starts together and awaits later, as tool
  calls can be, overwrite one another in a shared object; the spec says to give each its own.
- [Shrinking by size is crude] → A failure prints the whole case as JSON, which is the input to
  reproduce it by hand in either runtime.
- [The known-findings list could become a place to park failures] → Each entry must name where it
  is tracked, and an entry whose case stops reproducing fails the run.
- [Measuring input costs a pass the runtimes did not make before] → Only when a limit is set, and
  bounded by the limit.

## Migration Plan

Additive. No image, schema, ABI or feature change, and no operation count changes. Hosts opt in by
setting the option. No existing corpus program's `expected/` files change; `manifest.json` gains
the new program. A runtime that does not read `arguments`, `case` or `inputSizes` skips the new
program. Rollback of any stage is reverting it.

## Open Questions

- Settled in implementation: CI runs 400 generated cases, set from a measured 26 ms a case in an
  unoptimized build (decision 6).
- Settled since this was first written: the agent package returns a tool call's usage on its
  result, through a sink of its own for each call (`add-agent-host-package`, *Results are
  values*), since tool calls started together cannot share one.
- Settled since this was first written: the `nx.int` record at `object` is a record in both
  runtimes (the budget change's RF28), so the two no longer differ in count or answer for it and
  the differential test has nothing to record there.
