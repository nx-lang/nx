# NX IR conformance corpus

NX programs with the images the compiler emits for them, the explained text of each image, the
values a runtime evaluates them to, and what a runtime renders and emits when a host drives their
component lifecycles. The emitter's tests pin the images byte for byte and keep the text in step;
the TypeScript runtime's tests evaluate them, dispatch them, and refuse every truncation and cell
overwrite of them; a second runtime starts here. Together the programs cover every node, type,
constant and declaration kind of the schema, every binary operator and intrinsic, a program
spanning two images, derived declarations, a snippet compiled against an implicitly imported
catalog, a document that is a single trailing element, components that bind action handlers, and
a program that declares a function type, passes functions as values and calls them by name, and
one whose calls leave parameters out for the function to fill, including a default that reads a
value private to the called function's module.

Each program is a directory:

| Path | What it holds |
| --- | --- |
| `*.nx` | The workspace's modules; a module's identity is its path within the directory. |
| `program.json` | The entry, the implicit imports, the version each module is built with, which modules to emit, the entrypoints to evaluate, and the lifecycles to drive. |
| `expected/<identity>.nxir` | The module's image with its debug section. `/` in an identity is written `__`. |
| `expected/<identity>.stripped.nxir` | The same image without the debug section. |
| `expected/<identity>.nxir.txt`, `.stripped.nxir.txt` | Each image as `nxlang ir explain` renders it, so a review reads text and a diff names what changed. The emitter's tests fail when a text is not the explanation of its image. |
| `expected/results.json` | The canonical value of each entrypoint, keyed `identity::function`, as the interpreter evaluates it, and each lifecycle's record, keyed `identity::Component`. |

A lifecycle names the module and component to initialize, optional `props`, and `batches`: an
ordered list of dispatch batches, each a list of entries written as a host would send them, either
an action record the component emits or an `ActionHandlerInvocation` naming a handler by its
`token`. Tokens are literal (`h1-1`): the interpreter assigns them deterministically, generation
first and then position in a walk that visits lists in order and record fields by name, and a
runtime is checked against the very same strings. The record holds `initial`, the rendered output
of initialization, and one entry per batch with its `rendered` output and its ordered `effects`.
State is not recorded, since the interpreter returns it only inside an opaque snapshot; the
rendered output reflects it.

`manifest.json` at the root lists the programs and which kinds each covers; the emitter's tests
fail when a kind is covered by none.

Generated JavaScript is the third engine held to `results.json`: the emitter's tests generate each
program as JavaScript and run every entrypoint of its entry module, whose value must equal the
recorded one. A program executable source codegen refuses (a match expression, a call of a
function-typed value, an action handler) is skipped, and so is an entrypoint that reaches such a
construct at run time. `occurrences` and `occurrence-lifting` avoid all of them so that the
occurrence rules are checked in all three engines, which is why the `{}` pattern has a program of
its own, `occurrence-patterns`.

The size budget lives here too: every image emitted without its debug section is at most six
times the UTF-8 length of its module's source. The images are marked binary in `.gitattributes`;
read one with `nxlang ir explain`, or read its `.txt` sibling.

To regenerate the expected files after an intended change to the emitter or a program:

```bash
NX_UPDATE_CORPUS=1 cargo test -p nx-codegen --lib ir_corpus
```

then review the diff of the `.txt` files before committing it.
