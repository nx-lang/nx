# NX IR conformance corpus

NX programs with the images the compiler emits for them, the explained text of each image, the
values a runtime evaluates them to, and what a runtime renders and emits when a host drives their
component lifecycles. The emitter's tests pin the images byte for byte and keep the text in step;
the TypeScript runtime's tests and the Rust runtime's tests each evaluate them, dispatch them, and
refuse every truncation and cell overwrite of them; another runtime starts here. Together the programs cover every node, type,
constant and declaration kind of the schema, every binary operator and intrinsic, a program
spanning two images, derived declarations, a snippet compiled against an implicitly imported
catalog, a document that is a single trailing element, components that bind action handlers, two
programs that import the standard library `@nx/agent` and emit its image beside their own (the
library's worked example, and tool functions that take a host context by its subtype or its base),
a program that declares a function type, passes functions as values and calls them by name, one
that declares function reference types (`<function ... />: R`), binds functions of unlike
signatures to them and passes, returns and compares such values, and
one whose calls leave parameters out for the function to fill, including a default that reads a
value private to the called function's module, and `evaluation-cost`, which has one entrypoint per
rule of the evaluation cost model: what is evaluated, placed and concatenated, and what is checked
against a type, compared and written for the host.

Each program is a directory:

| Path | What it holds |
| --- | --- |
| `*.nx` | The workspace's modules; a module's identity is its path within the directory. |
| `program.json` | The entry, the implicit imports, the version each module is built with, which modules to emit, the entrypoints to evaluate, and the lifecycles to drive. |
| `expected/<identity>.nxir` | The module's image with its debug section. `/` in an identity is written `__`. |
| `expected/<identity>.stripped.nxir` | The same image without the debug section. |
| `expected/<identity>.nxir.txt`, `.stripped.nxir.txt` | Each image as `nxlang ir explain` renders it, so a review reads text and a diff names what changed. The emitter's tests fail when a text is not the explanation of its image. |
| `expected/results.json` | The canonical value of each entrypoint, keyed `identity::function`, as the interpreter evaluates it, and each lifecycle's record, keyed `identity::Component`. |
| `expected/operations.json` | What each evaluation costs in operations, as `docs/nx-ir-format.md` (*Evaluation cost*) defines them, and, for a program that asks, where smaller budgets stop it. |

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
runtime, found by doubling and bisecting the budget. A runtime that offers a budget checks every
count `n` by evaluating under a budget of `n`, which must give the recorded result, and of `n - 1`,
which must fail with `nx-ir-resource-limit` naming `maxOperations`; and it checks every failure by
evaluating the debug image under its budget, which must stop at the recorded declaration and span.
Equal counts show that two runtimes charge the same total; the failures show that they charge in
the same order.

`manifest.json` at the root lists the programs and which kinds each covers; the emitter's tests
fail when a kind is covered by none.

Generated JavaScript is the third engine held to `results.json`: the emitter's tests generate each
program as JavaScript and run every entrypoint of its entry module, whose value must equal the
recorded one. A program executable source codegen refuses (a match expression, a call of a
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
diff of `operations.json` as a number beside its entrypoint.
