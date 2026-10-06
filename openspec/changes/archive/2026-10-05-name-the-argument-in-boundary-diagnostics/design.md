## Context

See `proposal.md` for the motivation. What shapes the approach:

- A runtime diagnostic is `{ severity, code, message, source?, declaration?, limit? }` in the
  TypeScript runtime (`NxIrDiagnostic`) and the same without `severity` in the Rust runtime
  (`Diagnostic`). `limit` is the precedent for this change: a structured member one code carries,
  added to both runtimes together, with a requirement in each runtime's spec and a scenario that
  the two agree.
- Nothing else carries a runtime diagnostic to a host. `bindings/dotnet`'s `NxDiagnostic` is the
  compiler's and has no `limit`; `bindings/node` and `bindings/wasm` emit the images
  `@nx-lang/ir-runtime` reads and call none of its evaluation APIs; and only `nx-codegen` depends
  on the `nx-ir-runtime` crate. So the member is added in the two runtimes and read in one package.
- Both runtimes bind arguments in one place. In TypeScript it is `invokeFunction`: for each
  parameter it takes the argument, its default or nothing, raises `nx-ir-arguments` for a missing
  required one, and passes the value to `normalizeValue(context, param.ty, value, param.name)`,
  which raises the `nx-ir-boundary-*` codes. In Rust it is the same loop in `eval.rs`, ending in
  `self.normalize(cx, &param.ty, value, &Path::Root(param.name))`. The parameter's name is in hand
  at the point the check is called.
- That one place serves every call, the host's and the ones a function body makes. A failure
  bound to an inner call is not about anything the host passed.
- The check is not only a comparison. It evaluates record field defaults, charges the budget and
  resolves function records, so a failure raised inside it is not always about the host's value
  (decision 2).
- `@nx-lang/agent` classifies a failure in `packages/agent/src/classify.ts` from the diagnostics'
  codes: any `nx-ir-resource-limit` is `resource-limit`; all boundary-family is `invalid-input`;
  anything else is `evaluation-failed`. It knows each tool's context parameters by name
  (`config.contextParameters`). Since `add-agent-host-package`, its AI SDK adapter shows the model
  the full message only for `invalid-input`.
- Both runtimes stop at the first failure, so a failed call has one diagnostic. The package's
  rule is written for a list all the same.

## Goals / Non-Goals

**Goals:**

- A program can read which argument of the called function a failure is in, in both runtimes,
  with the same answer.
- `@nx-lang/agent` reports a context record that does not fit as `invalid-context`, without
  reading a message.
- A boundary-family failure that is not in an argument stops being reported to the model as its
  mistake.

**Non-Goals:**

- A path inside the argument (`request.items[2].quantity`). The message has it for a person, and
  no consumer needs it as data yet. The member is named so a path can be added beside it later.
- Component entry points. A failure in a prop, in state or in an action names no `argument`;
  those have their own callers and none of them is asking.
- New codes, new message wording, or a change to when a call fails.
- Giving failures inside a function a code outside the boundary family. `add-agent-host-package`
  decision 14 wants that of the change that adds constrained types; this change makes the package
  correct without it.

## Decisions

### 1. One optional member, `argument`, holding the parameter's name

`NxIrDiagnostic` gains `readonly argument?: string`, and the Rust `Diagnostic` gains
`pub argument: Option<String>`. It holds the declared name of the parameter. A host that called
by position reads a name too, since a name is what it has to report and an index is not stable
under a default or an optional parameter.

Alternatives considered:

- *A path, as a list of segments.* More than is needed, and the two runtimes build their paths
  differently today (a string in TypeScript, a linked `Path` in Rust); making those agree
  segment for segment is a larger change with no consumer.
- *A flag, `fromHost: true`.* It answers "was this in the arguments" and not "in which", which is
  the question the package has.
- *A second code for a context failure.* The runtime does not know what a context is, and should
  not.

### 2. Set for a failure in a value the host passed, and for nothing else it happens to run

The binding loop is where the name is known, but "raised while a parameter is bound" is too wide
a rule. Checking a host's value does more than compare it with a type:

- It fills in a record's field defaults, by evaluating each default's expression, which can call
  functions, and then checks the default's value against the field's type
  (`runtime/typescript/src/index.ts`, where a record's fields are normalized;
  `crates/nx-ir-runtime/src/normalize.rs`). A division by zero in a default, a failure in a
  function a default calls, and a default whose value does not fit its field are all raised inside
  the binding. The last carries a boundary code, and the checker does not rule it out: it does
  not type-check a field default that reads another field (`specs/future.md`, *A record default
  of the wrong type passes the checker when it reads another field*). Closing that gap will not
  retire the case. Once a type can carry a constraint, a default computed from another field can
  miss its own field's constraint with nothing for the checker to find, and that failure has a
  boundary code too.
- It charges the budget, one operation for each value checked, so `nx-ir-resource-limit` for
  `maxOperations` can be raised there, and so can the call-depth limit through a default, and the
  Rust runtime's stack limit.
- It resolves a `Function` record it finds in the value, and refuses one that names no function
  with `nx-ir-function-value`.

So the rule is by what the failure is about, in three parts:

1. **Tagged:** a failure the check raises about the value the host passed for a parameter: a
   boundary code at any depth, `nx-ir-function-value` for a function record in it, and
   `nx-ir-arguments` for a required parameter given nothing.
2. **Not tagged, though raised inside the binding:** anything a default raises (its expression,
   a function it calls, or its value not fitting its type), and any `nx-ir-resource-limit`.
3. **Not tagged, raised elsewhere:** the body, the result, a count of positional arguments, the
   `Function` record `callFunction` is given to say which function to call (it is refused with
   `nx-ir-function-value` before anything is bound, and is not an argument), other entry points.

A way to build it that touches two places in each runtime: the entry call wraps the binding of a
parameter whose value the host supplied (and the missing-required failure) and adds the name to
what is thrown when the diagnostic's code is one of the three kinds of part 1 and it is not marked
as raised by a default; and the place a default is evaluated and its value checked marks what it
throws. Going by the code as well as the mark is what keeps out everything else the check can
raise that is not about the value: a limit, and a failure of the image or the program found on
the way (`nx-ir-schema`, `nx-ir-reference`). The mark is
private to the runtime: a flag on the evaluation, set where a default fails, which is enough
because neither runtime recovers from a failure inside one evaluation. A parameter the
host gave nothing for and that has a default is not wrapped at all, so its default's failures are
the function's. The implementer may thread an origin through the check instead, if that reads
better in one runtime; the spec's scenarios hold either way, and the same scenarios run in both.

Only the call the host made is wrapped. The two entry points say so explicitly, with a parameter
that reaches the binding loop. In TypeScript `evaluateFunction` calls `invokeFunction`, which
holds the loop, and `callFunction` reaches it through `invokeFunctionByName`. In Rust both entry
points are in `component.rs` and go through `call`; `evaluate_function` then enters the loop in
`eval.rs` directly and `call_function` through `invoke_by_name`. The
entry call does bind at depth 0, and a function a default calls runs at depth 1, so the depth
would tell the two apart today. An explicit parameter is used all the same, because the rule is
"the call the host made", and the depth is that only as long as nothing else enters a function
at depth 0, which task 1.2 checks and nothing guarantees.

`nx-ir-function-value` is tagged for the runtimes' sake, not the package's: it is a refusal of
the host's value, and leaving it out would make "a failure in the argument" mean something
narrower than it says. The package reports it as `evaluation-failed` either way, since the code
is not of the boundary family, and a tool's function cannot take a function from a model.

Alternatives considered. *By code alone*, at the binding loop: one place per runtime, and wrong
for a default whose value does not fit, which carries a boundary code and is not the host's.
*By the wrap alone, less limits and defaults*: it would name an argument for `nx-ir-schema` and
`nx-ir-reference`, which are about the image.

### 3. The package classifies by the member

`classifyRuntimeFailure` takes the names of the tool's context parameters beside the diagnostics:

1. Any `nx-ir-resource-limit`: `resource-limit`, as today.
2. Every diagnostic is boundary-family and has an `argument`:
   `invalid-context` if any `argument` is a context parameter, otherwise `invalid-input`.
3. Otherwise `evaluation-failed`.

The third rule moves one case: a boundary-family diagnostic with no `argument` was
`invalid-input` and becomes `evaluation-failed`. What moves today is small and real: a record
field default whose value does not fit its field, which the checker accepts when the default
reads another field, fails with a boundary code although the argument fits, and is told to the
model as its mistake (`add-agent-host-package`, review finding RF16). A function whose result does not fit its
declared type, and a value a body passes on that does not fit, would move too, and the checker
rules those out. The rule makes "the model can correct this" mean exactly "it is in a value the
model sent".

Why a mixed failure is `invalid-context`: the model cannot make the call succeed by changing what
it sent while the context is wrong, and the host has to be told.

Alternative considered: keep an unnamed boundary failure as `invalid-input`. It keeps today's
answer for a case that should not happen, and keeps telling the model about a failure it did not
cause in the one case where it does.

### 4. The two runtimes are held to one answer by the conformance corpus, and by the differential test wherever it compares a diagnostic

Two tests run both runtimes on the same image, and each gets the member.

**The conformance corpus** (`specs/ir-conformance`) is where the named cases go. A program there
lists entrypoints, and an entrypoint with arguments is a *case*: its answer is recorded from the
Rust runtime and checked in the TypeScript runtime, which the corpus's README says is what a case
is for. A new program, `argument-diagnostics`, holds the cases of the Rust spec's scenario *The
two runtimes name an argument alike*.

A case that fails cannot be recorded today. `nx-ir-format`'s requirement *The conformance corpus
evaluates entrypoints with arguments* says a case uses only arguments every runtime accepts,
`expected/results.json` holds values, the generator (`crates/nx-codegen/src/ir_corpus_tests.rs`)
panics when a case fails in the Rust runtime, and both runners count a failing entrypoint as a
test failure. So this change modifies that requirement and adds a failing case to the corpus's
format:

- A case in `program.json` says it fails, with `"fails": true`. The member is a boolean, leaving
  it out is `false`, and the three readers of the corpus agree on that. The generator keeps its
  panic for a case that fails without saying so, and gains one for a case that says so and
  succeeds, so regenerating never turns a result into a recorded failure unasked.
- `expected/diagnostics.json`, written for a program that has such a case, holds for each one,
  keyed as `results.json` keys a case, the diagnostic's `code` and its `argument`, the member left
  out when the diagnostic names none. The case has no entry in `results.json`.
- A failing case has no entry in `operations.json`: no count, no failure under a smaller budget
  and no input size. Those say what a call that succeeds costs, and the checks of them (a budget
  equal to the count gives the recorded result) have nothing to compare. What a refused call uses
  is not this change's subject.
- Each runner evaluates a failing case from both images and compares the code and the argument,
  and makes none of its count, usage or input-size checks for it.

The cases that fail are: a wrong-typed argument, a wrong field three levels down, a missing
required argument and a `Function` record that names no function, each recorded with its
argument; and a record whose field default divides by zero and a division by zero in the body,
each recorded with none. A field default whose value does not fit its field is not among them: it
compiles only through the checker's gap, and a corpus program must not stop compiling when the gap
is closed. Each runtime's own tests hold that one.

Those two unnamed cases do not test decision 2, since a division by zero is a code no rule names.
Two more do, and need no gap. A `Function` record is accepted at a function type with its
parameters unchecked, so a host can pass one that names a function of two parameters where a
function of one is declared, and a default that calls it fails with `nx-ir-arguments`, a code the
rule does name. One case has that call in a record's field default, which the mark keeps unnamed;
the other has it in the default of a parameter the call leaves out, which is unnamed because that
binding is not wrapped. Each is recorded with no argument, and each fails in a runtime that drops
the condition it tests. If a mismatched function record is one day refused at the boundary, these
two need another trigger.

Two things the corpus already checks gain "and names no argument", as an assertion in each runner
and no change to a file: that a case fails with `nx-ir-resource-limit` under a recorded budget
below its count, and under an input limit one less than its recorded input size. The new program
sets `recordFailures` and has a case that succeeds, fifty records for a parameter `items:Item+`,
whose recorded budgets run out while the argument is checked. The input limit is covered by the
cases `host-values` has today.

**The differential test** (`crates/nx-codegen/tests/cost_differential.rs`, with
`tests/cost/mod.rs` and `runtime/typescript/test/cost-runner.mjs`) runs generated host values
through a fixed probe library in both runtimes. Wherever it records a diagnostic it records the
argument too, on both sides: the refusal with no budget, the refusal under ample limits, the
refusal at the larger scale, and each entry of the list of failures under smaller budgets, which
is where a budget spent inside the binding lands. It runs with no input limit or an ample one,
so it has no input-limit case; the corpus has that.

(`crates/nx-codegen/src/ir_runtime_tests.rs` compares the Rust runtime with the interpreter and
does not run the TypeScript one, so it is not the place.)

Each runtime's own tests cover what neither shape reaches: a call by name (a corpus case passes
its arguments by position) and the entry points that must carry nothing.

### 5. `add-agent-host-package`'s decision 14 gains a condition

That decision lists what the change adding constrained types must hold, and `specs/future.md`
keeps the list reachable (*What a constrained-types change has to hold for the agent package*).
One condition is added there when this change is applied: a constraint checked on a value the
host passed is reported with the argument named, like every other boundary failure, or the
package will report it as `evaluation-failed`.

## Risks / Trade-offs

- [A boundary failure the runtime raises under an argument through a path this change misses is
  reported as `evaluation-failed`, and the model is not told what to fix] → Task 1.2 lists
  every site in both runtimes that raises a boundary-family code and says for each whether it can
  be reached while an entry call binds an argument. The comparison test then covers each kind.
- [The runtime refuses a value whose `$type` two shapes of the program share ("ambiguous
  subtype") with a boundary code, and it is tagged, so the package tells the model its input was
  wrong when the program is what is ambiguous] → It is a refusal of the value the host passed,
  which is what the member says, and the schema export reports the ambiguity when the tool is
  described (`schema-ambiguous-discriminator`), so a host that normalizes its agent has seen it.
  Left as it is.
- [A default is evaluated somewhere this change does not mark, and its failure leaves with the
  host's argument named] → Task 1.2 lists every place either runtime evaluates a default while it
  checks a value, and two corpus cases run a default that fails with `nx-ir-arguments` in both, a
  field's and a parameter's, which a runtime without the mark or without the exemption names an
  argument for.
- [The package and the runtime are separate npm packages, and a package reading `argument` from a
  runtime that does not set it would report every bad input as `evaluation-failed`] → They are
  released together at one version and the package depends on that version of the runtime. A test
  in the package fails if a wrong-typed argument is not `invalid-input`.
- [`treat-host-values-as-json` raises a new boundary failure, for a value in the input that is not
  canonical JSON, and modifies the same requirement of `agent-host-package`] → A value refused
  inside an argument is a failure in a value the host passed, so by decision 2 it names the
  argument. Whichever of the two changes is applied second adds the member there or the case
  here, and brings its `agent-host-package` delta up to the main spec first (task 1.1); neither
  design changes.
- [`invalid-context` now has two meanings: no record, or a record that does not fit] → Both are
  the host's to fix, and the diagnostics on the result say which. The README says so.
- [`runtime/typescript/dist` is committed] → Rebuilt and committed with the source, as for every
  runtime change.

## Migration Plan

The member is additive. The one behaviour change for a host is in `@nx-lang/agent`, which is
unstable: a context that does not fit, and a boundary failure in no argument, leave
`invalid-input`. ReachMe is the only host, and it reads the code in `onResult`.

This change's `agent-host-package` delta modifies a requirement that `add-agent-host-package`
added. That change is archived, and on 2026-10-05 a line diff of the delta against the main spec
showed this change's edits and nothing else.
