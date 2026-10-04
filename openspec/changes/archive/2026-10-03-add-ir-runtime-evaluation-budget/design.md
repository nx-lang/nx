## Context

See proposal.md for motivation. The facts that shape the design:

- `runtime/typescript/src/index.ts` evaluates through one function, `evalNode`. The options a host
  passes (`NxRuntimeOptions`: `maxCallDepth`, `maxRangeLength`) are threaded as a plain object into
  every internal path that can evaluate a node: `invokeFunction`, `normalizeFields` (state and
  field defaults), `invokeHandler`, `patchComponentState`. An `EvalContext` is built per function
  call and holds `options` and `depth`. There is no state that lives for exactly one public call.
- `crates/nx-ir-runtime/src/eval.rs` evaluates through `Machine::eval`, which already counts
  nesting in a `Cell<u32>` and checks the native stack. A `Machine` is built per public call from
  `&RuntimeOptions`, which is `Copy` with public fields and a `Default`.
- The Rust runtime has limits the TypeScript runtime lacks: expression nesting (1,000, fixed), a
  1 MiB stack budget, value nesting at the host boundary (256), and bounds on a serialized
  instance. The TypeScript runtime has call depth and range length only, checks the call depth
  with a default of 100 in `invokeFunction`, and reports every limit as `nx-ir-resource-limit`
  with a message and no structured field (`NxIrDiagnostic` is `severity`, `code`, `message`,
  `source?`, `declaration?`).
- `crates/nx-interpreter/src/context.rs` counts one operation per HIR expression evaluated
  (`check_operation_limit`, called at the top of `eval_expr_arms`) against `max_operations`,
  1,000,000 by default. `retire-hir-interpreter` deletes it; its design says the budget "has no
  equivalent and is dropped" and lists a successor as an open question.
- `evalItemInto` splices with `items.push(...value)`, and `dispatchComponentActions` collects a
  parent-bound handler's effects with `effects.push(...invokeHandler(...))`. V8 raises
  `RangeError: Maximum call stack size exceeded` for a spread of about 125,000 arguments or more
  (checked on Node 24: 100,000 succeeds, 130,000 fails), so content built from a list longer than
  that, or a handler returning that many actions, fails with an error that is not an
  `NxIrRuntimeError`, although `maxRangeLength` admits a million. `spliceContent` uses `flatMap`,
  which has no such limit.
- Both runtimes take one shortcut past their evaluator besides `ifIs` patterns: the Rust runtime
  resolves a call's callee in place when it is a `reference` to a function (`Machine::callee`),
  without passing the node through `Machine::eval`, where the TypeScript runtime evaluates it with
  `evalNode`. A per-node charge placed only in each evaluator would count every direct call once in
  one runtime and not at all in the other.
- The Rust runtime already builds one `Machine` per public call, and `dispatch_component_actions`
  runs every handler of the batch and the render after it on that one `Machine`.
  `restore_component_instance` builds one with the default options but evaluates no node: it
  checks the stored props and state without defaulting anything.
- In a Cloudflare Worker or Durable Object, `Date.now()` does not advance while JavaScript runs;
  it advances at I/O. A synchronous evaluator cannot time itself there.
- NX is pure and has no mutable accumulator, so what an evaluation allocates comes from
  sequence-building nodes (`array`, `for`, `forRange`, content lists), from `concat`, and from
  constructions. But a value is held by reference: a record that names one value in two fields, or
  a list that holds it twice, is a few words that unfold to twice the value for anything that
  walks it as a tree. Three things in each runtime do: checking a value against a declared type,
  which also rebuilds it; structural equality; and converting a result for the host, where the
  canonical form has no sharing. The Rust runtime has a fourth of its own, the check that state
  nests no deeper than 256 after a patch.
- The two runtimes differ for integers outside JavaScript's safe range (`docs/nx-ir-format.md`,
  *Rust runtime*), so "the same result" between them already has that one exception.

## Goals / Non-Goals

**Goals:**

- A host bounds the work and the allocation of one evaluation call with one number.
- The number means the same thing in every runtime: the same images and input stop at the same
  node.
- Exhaustion is an `NxIrRuntimeError` a host can recognize from data, and the TypeScript runtime
  has no remaining path from tenant code to an uncaught engine error.
- The work and the allocation of an evaluation are proportional to its count: no step costs time or
  memory that grows with the size of a value without paying for it.
- A host that sets no budget, or a generous one, pays one counter update per charge and nothing
  else.

**Non-Goals:**

- A wall-clock deadline or cancellation. It cannot be deterministic, inside a Durable Object the
  clock does not move during evaluation, and time does not bound allocation: a list that doubles
  on each call exhausts an isolate's memory in milliseconds, long before a deadline. A host outside
  a Worker can add a deadline beside the budget later, checked every few thousand operations.
- A memory limit in bytes. Real allocations differ between the runtimes, and JavaScript offers no
  hook to count them; the count bounds what is allocated, in values (decision 2).
- A limit on the size of a value the host passes in. Reading the host's input costs nothing; it is
  charged where it is checked against a type, like any other value.
- Reporting the operations a successful call used. The corpus is checked without it (decision 8).
- The budget in `nx-api`, the CLI and the .NET, Node and wasm bindings (decision 9).
- Value-nesting and serialized-instance limits in the TypeScript runtime. The engine-error
  conversion of decision 7 covers a deeply nested host value; a deterministic bound can follow.
- Choosing the number a host uses. `add-agent-host-package` and ReachMe's
  `add-agent-function-tools` do that.

## Decisions

### 1. The cost model belongs to the IR contract

The rules for what costs an operation go in `nx-ir-format` and `docs/nx-ir-format.md`, and each
runtime's spec refers to them. A budget is only portable if a count is a property of the images
and the input; stating the rules once, next to the node table they are written in terms of, is what
makes a third runtime able to implement them.

*Alternative: define the count in the TypeScript runtime's spec and have the Rust runtime "match
it".* Rejected: the rules would be whatever one implementation happens to do, and the first
refactor of `evalNode` would change them.

### 2. An operation is a unit of work on a value: evaluated, placed, built, checked, compared or written

Six charges, all stated in terms of the IR and of values, never of an implementation:

| Charge | Amount | When |
| --- | --- | --- |
| A node is evaluated | 1 | Before its children. Every evaluation counts: loop bodies per iteration, defaults each time they run, a value's initializer each time it is referenced. |
| An item is placed in a sequence a node builds | 1 per item | As the item is placed. `array`, `for`, `forRange`, the content lists of `record`, `unionCase`, `element`, `component`, and the list a call binds to a content parameter. |
| A `concat` produces a string | `floor(length / 64)`, in UTF-16 code units | After the operands, before the string. |
| A value is checked against a declared type | 1 per value that is not a sequence | Before it is checked. A record is one and then its fields; a sequence is its items; `object` is one value whatever it holds. |
| Two values are compared | 1 per pair, 1 per field name only one of two records holds, and `floor(length / 64)` for text read | Before they are compared. The items of two sequences in order up to the first pair that differs; every pair of fields two records line up, with no early exit. Text is two strings, two type names, and the field names of two records of one type. |
| A value is written for the host | 1 per value, a sequence and the empty value included, and `floor(length / 64)` for text written | Before it is written. Results, descriptors, rendered output, effects and state. Text is a string, a type name and a field name. |

Node evaluations bound the number of steps. They do not bound the work of a step: a function that
passes `{ xs xs }` or `s + s` to itself doubles a value per call and reaches 2^100 inside the
default call depth in a few hundred nodes. The item and string charges make every item and every
64 code units an evaluation allocates cost budget.

That is still not enough, which a review of the first implementation showed (RF1 in `review.md`).
A record that holds one value twice costs a constant to build, and forty levels of it are 2^40
values to a walk. Under the first three charges alone, a call costing 253 operations took half a
second, one costing 343 took a minute, and no budget stopped it: the type check at each call
rebuilt the value as a tree, equality compared it as one, and the result was written as one. The
same held for a string, held a hundred times in a list for two operations each and written a
hundred times. So the last three charges make every walk over a value pay for each value it
visits.

A second review pass then looked for steps that are linear in a value and free, without sharing,
and found four (RF8 to RF11): a call copied a list bound to a content parameter; the Rust runtime
copied a record's names each time it wrote one, and scanned a record's fields for each field it
compared; and its state depth check walked the whole state after every patch. A host object at
`object` makes each of these as large as the host sends, and a program repeats them for a few
operations each. They are closed the same way: the copy is charged as the placement it is, names
are text and cost their length where they are compared and written, records are compared in time
proportional to their width, and the depth check walks only the fields a patch supplies. A third
pass found one more of the kind (RF17): lining two records up by name read every name of both
before it compared anything, for one operation, so a wide object compared with a narrow one cost
its width in time and nothing in count. Two records of one type are now lined up at a price, one
for each name only one of them holds, and two of different types compare nothing further; and the
depth check remembers, for the call, what it found under each shared allocation (RF18). A fourth
found that an empty value cost nothing to write or to bind while a list of 20,000 of them was
copied each time (RF20), which came from this decision's own first wording, "a sequence is its
items and nothing more": every value written is now one, a sequence and an empty one included,
and every item bound to a content parameter is one. The rule
the format now states is the general one, that work which is not charged does not grow with the
size of a value, held or unfolded. What is left uncharged in a step is bounded by the declarations
of the program (the fields of a declared record, the slots of a frame), not by its data, so the
time and the memory of an evaluation are proportional to its count. That is a claim about every
step of two runtimes, established by reading them and by four rounds of an independent reviewer's
probes, each of which found a step the last had missed, always through a value a host passed at
`object`. So the documents a host reads state it as the intent of the rules and as what the tests
and the corpus cover, and tell a host that runs code it did not write to keep a limit of its own
on time and memory as well, a CPU limit or a separate isolate. A step added later has to keep the
rule; a limit on what a host may pass at `object`, or a generated test that holds time to a
multiple of the count, would establish it better than review has, and neither is in this change.

A value nobody walks costs only what building it cost. An element that holds one value in two
properties is checked against no type, so a function may build one forty levels deep, test it for
emptiness and return a number for a few hundred operations; it pays when it compares the value or
returns it.

A pattern of an `ifIs` arm costs one for its node whether the runtime evaluates the node or, as
both runtimes do today for a union case with fields (`evalPattern`, `eval_pattern`), reads the case
name without evaluating it. The callee of a call costs one for its node in the same way, whether
the runtime evaluates it (TypeScript) or resolves a function `reference` in place (Rust's
`Machine::callee`). The rules are written so that neither shortcut is observable in the count, and
each runtime charges the node explicitly where it skips its evaluator.

**Type checks.** The charge is one per value checked against a type that is not a sequence type,
made where both runtimes already recurse: after a sequence type has been taken apart into its
items and a one-item sequence at an exactly-one site has been read as its item, and before the
value is matched against the type. It covers host input as well as the program's own values. The
check is one piece of code and the same work for both, and a host that passes ten thousand rows
pays ten thousand to have them checked. Typed data nested in typed data is checked again at each
enclosing construction, which is what the runtimes did before this change without charging for
it; the count now says so, and a later change that stops re-checking a value the runtime itself
built would lower both.

**Equality: an early exit where the order is fixed.** A comparison of two records that stopped at
the first difference would cost an amount that depends on which field differs first, and the two
runtimes do not hold a record's fields in one order everywhere: the result of `apply` and an
untyped host object differ. So two records of one type compare every pair of field values,
differing or not, and the cost is a property of the two values; with no budget nothing is counted
and the comparison stops at the first difference, which cannot change its result. A record is as
wide as its declaration, or pays for its width to be lined up, so the work is small.

The same rule was first applied to sequences, for uniformity, and it should not have been: two
lists of 100,000 that differ in their first item cost 100,001 operations and the time to compare
them all, 121 ms for 200 such comparisons in the TypeScript runtime against 0.06 ms with no
budget. A sequence's items are in one order in every runtime, and so are the slots a handler
captured, so a comparison that stops at the first pair that differs costs the same everywhere.
Two sequences, and two handlers, are compared in order up to and including that pair. The Rust runtime's shortcut for two records that are one allocation is gone:
a record that holds a NaN is not equal to itself, so the shortcut gave an answer the TypeScript
runtime never gave, and under a budget, where a count has to look inside, a different answer
from the same runtime with none. A comparison's result depends on its values alone. Two handlers
compare as the Rust runtime compared them, by node and captured values, which the TypeScript
runtime now does too; it compared its own internal objects field by field.

**Writing for the host.** The canonical form is a tree, so this is where sharing ends and a value's
size as a tree is real. Everything a call returns is charged, state included, since a string held
many times in state is copied as many times. The TypeScript runtime returns state without
rewriting it, so there it walks the state to charge for it, under a budget only; with none nothing
reads the count.

**The Rust depth check.** `check_depth` walked state as a tree after each patch, and the TypeScript
runtime has no such walk, so charging it would make the two counts differ. It is bounded instead.
It walks only the fields the patch supplies, since nothing else can have nested deeper, so a batch
of cheap patches does not walk a large state once each. And it remembers, for the call, how many
levels it found under each allocation that more than one thing holds: a value held twice is walked
once, and so is a value a later patch supplies again or wraps in a new one. A remembered
allocation is kept alive for the call, so its address cannot come to name another.

UTF-16 code units are the string unit because the count is free in JavaScript (`length`) and
linear in Rust, where `concat` is already linear in the operands it copies. UTF-8 bytes would make
the JavaScript runtime scan, and flatten, every string it concatenates. One operation per 64 units
keeps ordinary text building cheap next to nodes: assembling a 2,000-character string costs about
31 operations per concatenation at full length, where one per unit would cost 2,000.

*Alternative: charge a value's size where it is placed in another, so that no value is larger as a
tree than its budget.* It bounds every walk, present and future, without charging any. Rejected: it
needs a new walk at every construction to measure what is placed, where the three charges ride on
walks that already happen; it charges for values nobody walks; and nested output is charged once
per level whether typed or not.

*Alternative: keep sharing through the walks.* A type check that remembered what it had checked,
and an equality that remembered pairs, would be linear in what is held. Rejected: the result for
the host has to be a tree whatever the walks do, so its size has to be paid for somewhere, and the
bookkeeping would have to be identical in both runtimes to keep one count.

*Alternative: count real allocations.* JavaScript has no allocator hook and a Worker no heap
statistics, real bytes differ between the runtimes, and an equality over shared structure runs for
minutes allocating nothing. A byte limit is a possible backstop for the Rust runtime, not a
definition of the count.

*Alternative: nodes only, with separate `maxSequenceLength` and `maxStringLength` limits.*
Rejected: three numbers for the host to choose, a list of lists evades a per-sequence cap, and a
legitimate large render would need the caps raised while the node budget already says how much
work is allowed.

*Alternative: refuse a `forRange` whose count exceeds the remaining budget before its body runs.*
Sound, since every iteration costs at least one, but it adds a rule and saves at most the budget's
worth of work. Not taken.

### 3. One budget per public call, on every evaluation API

`maxOperations` is a field of the options every evaluation API already takes, so it reaches
`evaluateFunction`, `callFunction`, `constructComponentDescriptor`, `initializeComponent`,
`evaluateComponent`, `dispatchComponentActions`, `normalizeComponentState` and
`applyComponentStatePatch` with no new signatures. The motivating host needs only the first two.
The others take it because they take the options, because a component's state defaults and
handlers are program code like a function body, and because an option some APIs silently ignored
would be a trap. For dispatch the unit is the batch: every handler it runs and the render after
it, since a batch is atomic already.

Each runtime gains one piece of state per public call. In TypeScript a small internal object,
created at each public entry from the host's options, holds the resolved limits, the operations
remaining and the nesting depth; it is passed wherever `options` is passed today and held by
`EvalContext` in place of `options`. In Rust the count is a `Cell<u64>` in `Machine`, beside
`nesting`; the runtime already builds one `Machine` per public call and shares it across a
dispatch batch and its render, so that part needs no restructuring. `restore_component_instance`
takes no options and evaluates no node, so it has no budget.

A public API that reaches another internally (dispatch renders, initialization normalizes state)
must pass the state it was given, not build a new one from the options, or the budget would
restart mid-call. Building the state only at exported functions, and giving internal functions no
way to construct it from options, makes that mistake a type error.

*Alternative: a host-owned meter object shared across calls.* It would let a host spread one
budget over several evaluations. No host in this set needs it: the agent host package evaluates
one function per tool call. It also does not fit `RuntimeOptions`, which is `Copy`. A host that
wants a total keeps its own.

### 4. Unset means unlimited

`maxOperations` is optional and absent by default, and `max_operations` is `Option<u64>`,
defaulting to `None`. A default budget would change behavior for every current host: a DrawnUI
render over a virtual list of a million rows costs several million operations, and the existing
`maxRangeLength` default of one million could not be reached under a default of one million
operations. The interpreter's default worked because it counted against small programs run from
the CLI. A host that runs code it did not write must opt in, and both READMEs say so plainly;
`add-agent-host-package` applies its own default for tools, so a tool author never runs unbounded
by omission.

A value that is not a non-negative safe integer is refused with a new diagnostic code,
`nx-ir-options`, before anything runs. `NaN` from a host's arithmetic on a missing setting must not
compare its way into an unlimited evaluation. Zero is valid and refuses any call that evaluates a
node.

### 5. The limit is data on the diagnostic

The code stays `nx-ir-resource-limit`, which hosts already handle. `NxIrDiagnostic` gains
`limit?: { name: string; value?: number }` and the Rust `Diagnostic` gains the equivalent optional
field, set on every resource-limit diagnostic and on no other. Names are the TypeScript option
names in both runtimes, so one host-side mapping serves both: `maxOperations`, `maxCallDepth`,
`maxRangeLength`, `maxExpressionNesting`, and `engine` (no value) for a JavaScript engine limit;
the Rust runtime adds `maxStackBytes` and `maxValueNesting`. The Rust runtime reports nesting and
stack exhaustion through one function today (`nesting_exceeded`); it is split so each names its
own limit.

*Alternative: a new code, `nx-ir-operation-limit`.* Rejected: a host that treats
`nx-ir-resource-limit` as "the program asked for too much" would have to learn a second code for
the same meaning, and the call-depth and range diagnostics would still be told apart only by
their messages.

### 6. The Rust runtime gets the budget in this change

Determinism across runtimes is a claim about two implementations, and it is only tested if both
exist. The corpus's expected files are written by Rust tests (`ir_corpus_tests.rs`), so recording
counts needs a Rust implementation anyway. The cost is small: `Machine::eval` is already the one
place every node passes through. And `retire-hir-interpreter` is about to make this runtime the
native evaluator, at which point it is the only place a successor to `max_operations` can live.

### 7. The TypeScript runtime stops leaking engine errors

Three changes, all needed for "a structured failure, never a hang or a foreign exception":

- **Nesting.** The per-call state counts `evalNode` depth and refuses past 1,000, the Rust
  runtime's `MAX_NESTING`. It is incremented on entry and decremented on normal return; a throw
  abandons the state, so no `finally` is needed on the hot path.
- **Engine range errors.** Each public evaluation API catches a `RangeError` and rethrows it as
  `nx-ir-resource-limit` with `limit.name` `engine`. This is a backstop: where the engine's stack
  is smaller than 1,000 nested nodes need (a browser worker, workerd), the failure is still a
  diagnostic, though not at a deterministic node. Only `RangeError` is converted; any other
  exception is a runtime bug and propagates.
- **Splicing.** `evalItemInto` appends in a loop, which the per-item charge needs anyway, and so
  does the effects collection in `dispatchComponentActions`, uncharged. `spliceContent`'s
  `flatMap` has no argument limit and stays as it is, uncharged.

### 8. The corpus records counts and checks them through the limit

Each corpus program gains `expected/operations.json`: for an entrypoint, the count under the key
`results.json` uses; for a lifecycle, the count of initialization and one per batch. It is a
separate file so that `results.json`, which `retire-hir-interpreter` regenerates and re-specifies,
is untouched.

A runtime checks a count `n` by evaluating under a budget of `n` (must give the recorded result)
and of `n - 1` (must fail on the budget). That pins the count exactly using only the public
option, so no API for reading the count is needed. Regeneration finds each count the same way from
the Rust runtime, by doubling and bisecting the budget; corpus programs cost tens to hundreds of
operations, so this is a handful of evaluations each.

Equal totals do not show that two runtimes charge in the same order, which is what decides the
node a smaller budget stops at. So the file also records failures: for chosen entrypoints, a
budget below the count with the declaration and span of the diagnostic it produces. Both runtimes
check those against the image that carries its debug section.

The corpus gains a small program (working name `evaluation-cost`) with one entrypoint per charging
rule, because the existing programs do not contain a `concat` of 64 code units or an arm with
several patterns chosen for this purpose. Its images are new files; no existing image changes.

### 9. `nx-api` and the bindings are sequenced with `retire-hir-interpreter`

Today `nx-api` re-exports the interpreter's `ResourceLimits`, `max_operations` included, and
native source evaluation enforces it. `retire-hir-interpreter` makes `ResourceLimits` `nx-api`'s
own type "holding the call-depth and range-length limits". This change does not touch `nx-api`.
Whichever of the two lands second does the small join: `ResourceLimits::max_operations` maps to
`RuntimeOptions::max_operations`. If this change lands first, `retire-hir-interpreter` keeps the
field instead of dropping it; if it lands second, a follow-up adds it. The unit changes either way,
from HIR expressions to IR operations, and the default there is for that change to choose.

`add-ir-runtime-performance-harness` is independent. If it has landed, its `bench:check` is the
overhead measurement; if not, the task list measures with a throwaway loop.

### 10. Overhead

The TypeScript charge is `if ((state.remaining -= n) < 0) fail`. With no budget `remaining` starts
at `Infinity`, which stays `Infinity` under subtraction, so a node, an item and a type check have
one code path and no "is the budget on" branch. The Rust charge is a `Cell<u64>` decrement with the
absent budget as `u64::MAX`, and `Machine::eval` tests the budget, the nesting bound and the stack
in one branch, leaving which was met to a cold function. The item charge rides on a loop that
already runs per item, and the string charge on an operation already linear in the string.

Two kinds of charge are skipped outright when no budget is set, where a charge cannot fail and
nothing reads the count. One is any charge that takes work to measure: the Rust runtime counts
UTF-16 code units, which is a scan unless the text is ASCII, and the TypeScript runtime walks
returned state only to charge for it. The other is the charges of a walk over a value, for
equality and for the result written for the host, which would otherwise be a call per value: with
no budget the Rust `Meter` is the one that charges nobody.

**Measured** (tasks 6.1, 6.2, 6.5 and 7.14; warm medians against the commit before this change,
base and new runs interleaved, ten rounds for Rust and seven for TypeScript, Node 24 and a release
build with thin LTO on one x86-64 laptop, after every charge of section 7). Six cases and the
three `handlers` corpus lifecycles, initialized and dispatched:

- arithmetic: `for i in 0..1000000 { i * i }`
- concatenation: 200,000 concatenations producing 80 code units each
- splicing: 200,000 iterations that each splice a five-item list
- records: 200,000 constructions of a two-field record
- comparison: two lists of 100,000 integers that differ in their first item, compared 200 times
- patches: one dispatched batch that stores 50,000 elements in an `object` state field and then
  patches a counter 4,000 times

Each range spans the median of the runs and the median of the paired differences:

| Case | TypeScript, no budget | TypeScript, `maxOperations: MAX_SAFE_INTEGER` | Rust, no budget |
| --- | --- | --- | --- |
| Arithmetic loop | 131 → 134 ms, +3 to +4% | no difference | 63 → 66 ms, +4% |
| Concatenation | 50 → 52 ms, +3% | **+7 to +9%** | 35 → 35 ms, no difference |
| Splicing | 99 → 95 ms, 2 to 4% faster | 1 to 2% faster | 55 → 59 ms, **+7%** |
| Records | 109 → 112 ms, +3% | **+9%** | 152 → 153 ms, no difference |
| Comparison | 0.06 ms, no difference | 0.06 ms, no difference | 2.3 ms, no difference |
| Patches | 9.6 → 9.6 ms, +1 to +2% | **+13 to +20%** | 236 → 8 ms, 30 times faster |
| Lifecycles | 98 → 93 µs, 1 to 5% faster | +2% | 49 → 49 µs, no difference |

The tasks set 5 percent for the arithmetic loop and the lifecycles, with no budget and with one,
and those hold. What does not:

- **Rust splicing, +7 percent with no budget.** Not explained. It is not the charges: a build with
  every charge compiled out, and one that also restored the original `push_item` call, were still
  4 to 5 percent slower than the base on that case and within 1 percent on the others. Skipping
  the counter when no budget is set, forcing the item charge inline at its call sites, and moving
  failure construction out of line each changed nothing. No profiler was to hand, and an
  independent run measured it at +12.6 percent; it is left as measured. Accepted by the project's
  owner on 2026-10-03 for now, and recorded in `specs/future.md` to be profiled.
- **TypeScript under a budget: concatenation, records and patches, +7 to +20 percent.** These are
  the charges doing their work where a budget is set: the type check of each field, the result
  and the state written value by value. A host with no budget does not pay them.

The comparison case was 121 ms under a budget in the TypeScript runtime when it was first
measured, because a comparison of two sequences then went on to the end as one of two records
does. Sequences now stop at the first pair that differs (decision 2), and the case was measured
again on its own: 0.03 ms for the 200 comparisons under a budget, 0.08 ms with none. The other
rows are from before that change, which touches no other case.

The Rust patches case is faster than before the change because the state depth check no longer
walks the whole state after every patch. The TypeScript splice is faster because `evalItemInto`
appends in a loop where it spread.

## Risks / Trade-offs

- [A walk over a value that is not charged, now or added later, reopens the hole RF1 found] → The
  format's rule is general: work that is not charged must not grow with a value's size as a tree.
  Both runtimes' tests run the RF1 probes at forty levels, for each walk: checked, compared,
  written, and held in state. A new walk has to be charged in both runtimes or follow sharing.
- [Typed data is checked again at each enclosing construction and each call it passes through, so
  a program that moves large typed values through many calls costs more than its nodes suggest] →
  That is the work the runtimes do. A function that returns a list of `n` items through `k` calls
  with a declared result costs about `k × n`. Stated in both READMEs; a host sizes its budget by
  measuring its own programs.
- [An equality of two unequal records compares every pair of fields, where it could have stopped
  at the first] → Bounded by its charge and by the record's width, and the price of a count that
  does not depend on field order. Sequences and handler captures, whose order is fixed, stop at
  the first difference.
- [A count belongs to an image, not to source: an emitter change that adds or removes nodes changes
  what a program costs] → Documented. Hosts that store images (ReachMe's runtime bundles) get
  stable counts for a stored image. The corpus diff shows every count an emitter change moves.
- [Changing a charging rule later changes every recorded count and every host's effective limit]
  → The rules are specified, the corpus pins them, and a change to them is a spec change.
- [The two runtimes could count differently in a path the corpus does not reach] → One corpus
  entrypoint per rule, the `n` / `n - 1` check on every existing entrypoint and lifecycle step,
  and recorded failing nodes at budgets below the count.
- [Per-node overhead in a hot interpreter loop] → Measured before merging (tasks 6.x) with a
  threshold; decision 10 keeps it to one subtraction and one compare.
- [The 1,000 nesting bound could refuse a TypeScript program that runs today] → The Rust runtime
  already enforces it over the same corpus and the differential tests. A program that nests
  deeper already fails natively.
- [Converting `RangeError` could hide a runtime bug that raises one] → Only at public evaluation
  entry points, only `RangeError`, and the diagnostic's message carries the engine's message.
- [`nx-ir-resource-limit` for an engine limit is not deterministic] → It is named `engine`, which
  tells a host this failure depends on where it ran.
- [A Rust caller with an exhaustive `RuntimeOptions` or `Diagnostic` literal stops compiling] →
  The repository's own callers build options with struct update over a default, and
  `Diagnostic` values are built only inside the crate; noted in the release notes.

## Migration Plan

Additive. No image, schema, ABI or feature change, so stored images run unchanged. Hosts opt in by
setting the option. Rollback is reverting the change; `operations.json` files are ignored by a
runtime that does not read them.

## Open Questions

- Whether a call should report the operations it used, for a host that meters or tunes its
  budget. Default taken: no; the host package can add it when a host asks.
