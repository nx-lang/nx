# NX IR conformance corpus

NX programs with the images the compiler emits for them, the explained text of each image, the
values a runtime evaluates them to, and what a runtime renders and emits when a host drives their
component lifecycles. The emitter's tests pin the images byte for byte and keep the text in step;
the TypeScript runtime's tests and the Rust runtime's tests each evaluate them, dispatch them, and
refuse every truncation of them and every cell overwrite of the small ones; another runtime starts
here. Together the programs cover every node, type,
constant and declaration kind of the schema, every binary operator and intrinsic, a program
spanning two images, derived declarations, a snippet compiled against an implicitly imported
catalog, a document that is a single trailing element, components that bind action handlers,
`held-records`, a component that holds records it built in its state and renders one, whose origins
must survive every dispatch and which takes a record from the host in an action (the program
also holds function values in state, which the runtimes' own tests pass back), two
programs that import the standard library `@nx/agent` and emit its image beside their own (the
library's worked example, and tool functions that take a host context by its subtype or its base),
a program that declares a function type, passes functions as values and calls them by name, one
that declares function reference types (`<function ... />: R`), binds functions of unlike
signatures to them and passes, returns and compares such values, and
one whose calls leave parameters out for the function to fill, including a default that reads a
value private to the called function's module, `evaluation-cost`, which has one entrypoint per
rule of the evaluation cost model: what is evaluated, placed and concatenated, and what is checked
against a type, compared and written for the host, `host-values`, whose entrypoints are
evaluated with arguments: the values a host passes, which no entrypoint without arguments can
supply, with the input size of each, and `argument-diagnostics`, whose cases are calls every
runtime fails alike, each with the code of its diagnostic and the argument it names.

Two programs are of the size a host runs, where the others are as small as what they cover allows.
`large-catalog` is a library of 45 external components, 11 of them abstract bases three levels
deep, with 37 types and 399 properties, and a screen compiled against it as an implicit import: 61
elements in its source, 92 once its loops have run. Its catalog is written by `large-catalog/generate-catalog.mjs`, which takes no
input and writes the same bytes every time; change the script's tables, not `catalog.nx`, which
the TypeScript runtime's tests hold to the script's output.
`question-flow` is a library of 31 question kinds in three modules, a flow as data, and a flow as
a component with state whose lifecycle answers one question per batch, 30 in all; six of its steps
are shown only for some earlier answers. They are what `runtime/typescript/bench` times, and they
are held to their recorded results, counts and input sizes as every program is.

Overwriting a cell of an image links and runs its whole program again, so that sweep grows with
the square of a program's size. A program with an image above 16,384 bytes is left out of it, and
the test names the programs it leaves out, the two large ones: another program that grows past the
limit fails the test until it is added to that list or made smaller. Every image of every program
is still cut at every four-byte boundary.

Each program is a directory:

| Path | What it holds |
| --- | --- |
| `*.nx` | The workspace's modules; a module's identity is its path within the directory. |
| `program.json` | The entry, the implicit imports, the version each module is built with, which modules to emit, the entrypoints to evaluate, each with the arguments to pass it if it is a case and whether the case is one that fails, and the lifecycles to drive. |
| `expected/<identity>.nxir` | The module's image with its debug section. `/` in an identity is written `__`. |
| `expected/<identity>.stripped.nxir` | The same image without the debug section. |
| `expected/<identity>.nxir.txt`, `.stripped.nxir.txt` | Each image as `nxlang ir explain` renders it, so a review reads text and a diff names what changed. The emitter's tests fail when a text is not the explanation of its image. |
| `expected/results.json` | The canonical value of each entrypoint, keyed `identity::function`, as the interpreter evaluates it; of each case, keyed `identity::function#case`, as the Rust IR runtime evaluates it; and each lifecycle's record, keyed `identity::Component`. |
| `expected/diagnostics.json` | For each case marked as one that fails, keyed as `results.json` keys a case, the `code` of the diagnostic the call fails with and the `argument` it names, as the Rust IR runtime reports them; `argument` is left out when the diagnostic names none. A program with no such case has no such file. |
| `expected/operations.json` | What each evaluation costs in operations, as `docs/nx-ir-format.md` (*Evaluation cost*) defines them, and, for a program that asks, where smaller budgets stop it and how large the input of each case and lifecycle step is. |
| `expected/origins.json` | What the origins report gives for each evaluation from the images with their debug sections, as `docs/nx-ir-format.md` (*Where records came from*) defines it, as the Rust IR runtime reports it. |

An entrypoint names a module and a function. It may also name `arguments`, a list of canonical
values to pass by position, written as a host that read them from JSON holds them, together with
`case`, a name: the two go together, and an entrypoint with one and not the other fails the tests.
Such an entrypoint is a *case*. One function may have several cases, and a case's result, count,
failures and input size are kept under `identity::function#case`; an entrypoint that is not a case
keeps the key `identity::function`. The interpreter's source evaluation takes no arguments, so a
case's expected result is recorded from the Rust IR runtime, as the operation counts are, and
checked in the TypeScript runtime: that comparison is what a case is for. A case uses only
arguments that every runtime treats alike, spelled alike: each accepts them and gives one result,
or, for a case marked as one that fails, each fails the call with one diagnostic. So a `null`
inside a value at `object`, which the two runtimes return differently, is not in the corpus.

A case with `"fails": true` is one every runtime fails; `fails` is a boolean, leaving it out is
`false`, and only a case may set it to `true`. Such a case has no entry in `results.json` and none
in `operations.json`, which say what a call that succeeds gives and costs; `diagnostics.json`
holds the code of its diagnostic and the argument the diagnostic names, the parameter whose value
the failure is in, or no `argument` when the failure is not in a value the host passed. Each
runtime evaluates the case from both images and must fail with that code and name that argument,
or none. A case fails only where its program says so: one that fails without the mark, and one
with the mark that succeeds, fail the emitter's tests, regeneration included, so a result never
turns into a recorded failure unasked. `argument-diagnostics` holds these cases: a value of the
wrong type, a wrong field three levels down, a missing required argument and a `Function` record
that names no function, each with its argument; and, each with none, a record whose field default
divides by zero, a division by zero in the body, and two defaults that fail with
`nx-ir-arguments`, a code a failure in an argument also has: a record's field default, and the
default of a parameter the call leaves out. Those two are what hold a runtime to telling a
default's failure from an argument's by where it was raised and not by its code.

A lifecycle names the module and component to initialize, optional `props`, and `batches`: an
ordered list of dispatch batches, each a list of entries written as a host would send them, either
an action record the component emits or an `ActionHandlerInvocation` naming a handler by its
`token`. Tokens are literal (`h1-1`): the interpreter assigns them deterministically, generation
first and then position in a walk that visits lists in order and record fields by name, and a
runtime is checked against the very same strings. The record holds `initial`, the rendered output
of initialization, and one entry per batch with its `rendered` output and its ordered `effects`.
State is not recorded, since the interpreter returns it only inside an opaque snapshot; the
rendered output reflects it.

`operations.json` holds `counts`, keyed as `results.json` is: an entrypoint's count, or a
lifecycle's `{ "initial", "batches" }`, the count of initialization and of each batch in order, a
batch's count covering every handler it runs and the render after it. A program whose
`program.json` sets `recordFailures` also gets `failures`: for each entrypoint, the budgets half its
count and one less than its count, each with the `declaration` and, from the image with its debug
section, the `source` span of the diagnostic that budget produces. A failure with no `source` is a
charge that belongs to no node: a check against a type, or a result written for the host. The counts come from the Rust
runtime, read from its usage report in one evaluation each. A runtime that offers a budget checks
every count `n` by evaluating under a budget of `n`, which must give the recorded result, and of
`n - 1`, which must fail with `nx-ir-resource-limit` naming `maxOperations`, which is what says
that the recorded number is the least that works; it checks that its own usage report gives `n`;
and it checks every failure by evaluating the debug image under its budget, which must stop at the
recorded declaration and span. Equal counts show that two runtimes charge the same total; the
failures show that they charge in the same order. A diagnostic for a limit names no argument,
wherever the limit is reached, and each of these checks requires that too: the `fifty` case of
`argument-diagnostics` has a recorded budget that runs out while its argument is checked.

`origins.json` is keyed as `results.json` is: an entrypoint's entries, or a lifecycle's
`{ "initial", "batches" }`, the entries of initialization and of each batch in order. Each entry
is the record's JSON pointer within the value, the module identity and the start and end byte
offsets of the node that constructed it, one entry to a line so that a diff names the record that
moved. The entries come from the Rust runtime, read from its origins report over the images with
their debug sections, and are what makes the two runtimes' reports comparable: each runtime
evaluates every entrypoint and lifecycle with a report and without one, and must give the same
value and the same operation count both ways, the recorded entries from the debug images, no entries from the stripped ones, and
an empty report for a case that fails.

A program whose `program.json` sets `recordInputSizes` also gets `inputSizes`, keyed as `counts`
is: the input size of each case, as `docs/nx-ir-format.md` (*Input size*) defines it, and a
lifecycle's `{ "initial", "batches" }`, the input size of its initialization, which is its props,
and of each batch. A runtime that offers an input limit checks every size `n` as it checks a
count: under a limit of `n` the call must proceed and give the recorded result, and under `n - 1`
it must fail with `nx-ir-resource-limit` naming `maxInputSize` and no argument; and its usage
report must give `n`. Only `host-values` records them. The kinds of input the corpus cannot express, arguments by
name, content, a state passed in and a state patch, are checked in each runtime's own tests against
the sizes the format document works out.

`manifest.json` at the root lists the programs and which kinds each covers; the emitter's tests
fail when a kind is covered by none.

Generated JavaScript is the third engine held to `results.json`: the emitter's tests generate each
program as JavaScript and run every entrypoint of its entry module, whose value must equal the
recorded one. A case is not run there: its result comes from one IR runtime and is checked in the
other. A program executable source codegen refuses (a match expression, a call of a
function-typed value, an action handler) is skipped, and so is an entrypoint that reaches such a
construct at run time. A function value in generated JavaScript is the JavaScript function, which
knows its name and not its module, so a recorded `Function` record is compared with it by name.
`occurrences` and `occurrence-lifting` avoid all of them so that the
occurrence rules are checked in all three engines, which is why the `{}` pattern has a program of
its own, `occurrence-patterns`.

The size budget lives here too: every image emitted without its debug section is at most six
times the UTF-8 length of its module's source. The images are marked binary in `.gitattributes`;
read one with `nxlang ir explain`, or read its `.txt` sibling.

To regenerate the expected files after an intended change to the emitter or a program:

```bash
NX_UPDATE_CORPUS=1 cargo test -p nx-codegen --lib ir_corpus
```

then review the diff of the `.txt` files before committing it. A count that changed shows in the
diff of `operations.json` as a number beside its entrypoint, and an origin that moved shows in the
diff of `origins.json` as a span beside its record's pointer.

## Generated cases

The corpus pins the host values it names. The cost model's claim, that a step which is not charged
does not grow with the size of a value, is also checked against host values nobody chose:
`crates/nx-codegen/tests/cost/` holds a library of probes, one small NX function or component for
each operation a runtime performs on a host value, and a seeded generator of values biased toward
the shapes that have broken the claim before (wide objects, long names, long and non-ASCII
strings, nested and empty lists, `null`s, lengths just under and over 64 code units, and the names
a runtime gives a meaning to). Every generated case is run in both runtimes at two scales, the
second with its value eight times the size and its step repeated eight times as often, and:

- both runtimes must give the same result, operation count, input size and failures under smaller
  budgets, and name the same argument, or none, in every diagnostic
  (`cargo test -p nx-codegen --test cost_differential`);
- the bytes the Rust runtime allocates must stay within a multiple of the count and at most double
  for each unit between the scales (`cargo test -p nx-codegen --test cost_allocation`);
- a case whose time grows more than twice as fast as its count is reported, in a run that does not
  block (`cargo test --release -p nx-codegen --test cost_differential -- --ignored --nocapture time_report`).

A failure names its seed and its case and prints the command that reproduces it, of the form
`NX_COST_SEED=<seed> NX_COST_CASES=<count> cargo test -p nx-codegen --test cost_differential the_runtimes_agree`.
A step the tests find is listed as a known finding in `tests/cost/mod.rs` with a concrete case
until a change to the cost model fixes it, and an entry whose case stops reproducing fails the
run. `docs/nx-ir-format.md`, under *Validation against generated host values*, has the details.
