## Context

- The playground (`sites/playground`) compiles in a Web Worker through `@nx-lang/sdk-wasm`. The
  worker has a deadline, trap recovery and a replaceable host (`src/worker/*`). The editor uses
  `@nx-lang/monaco` and the language service over the same worker, and hover in the source pane
  already shows types and declarations. None of that is DrawnUI-specific.
- Everything else is DrawnUI:
  - the gallery;
  - `src/render/*`, which evaluates the IR with `@nx-lang/ir-runtime`, coerces it, and draws with
    `drawnui-react` on CanvasKit;
  - the implicit catalog module and its build-time artifact;
  - about 3.4 MB of vendored assets.
- `nxlang run` prints values with `crates/nx-cli/src/format.rs`, which formats the interpreter's
  `Value` as NX text that reads back. `nx-api`, which the wasm module already links, holds the
  interpreter. The wasm ABI builds artifacts and emits IR, but has no evaluate export.
- `@nx-lang/monaco` highlights through Shiki with the TextMate grammar that `@nx-lang/language`
  (`src/vscode`) publishes. The website's Expressive Code uses Shiki too.
- `launch-nxlang-website` lands first. It puts the playground's static files on a Cloudflare Worker
  whose script serves the shell for `/playground` and one-segment paths. It also creates the
  `website` capability, the docs code-block check, and the header this playground copies. This
  change moves the playground to `/play`.

## Goals / Non-Goals

**Goals:**
- One playground that runs any NX a docs page shows and presents its value well: in the NX text the
  CLI prints, folded where it is long, and explained on hover.
- A value viewer other hosts can drop in without adopting React, Monaco or the playground.
- Links into the playground that any site can build with a few lines of code and no server.
- Examples that cannot rot, because their output is committed and checked.

**Non-Goals:**
- Drawing anything, or running interactive components and their handlers. That is the fiddle's job.
- Multiple files, imports between them, or library references in the playground.
- Stored shares or short links. The fragment carries the source, and nothing is kept on a server.
- Other views of a value: JSON, an inspector tree, or a table for a sequence of records. NX syntax
  is the one view. A table toggle is a candidate follow-up.
- Evaluating functions on hover in the language service. That is its own change, and it will reuse
  the formatter moved here. An LSP hover is markdown, so it shows NX text, not this element.
- Using the viewer on the website. Its `nx output` blocks stay checked Expressive Code text for now.

## Decisions

### Evaluate in Rust through the wasm module, not with the TypeScript IR runtime

The output has to match `nxlang run`, and the NX text form needs what only the interpreter's
`Value` still carries. For example, a constant case prints as `Status.active`, but the canonical
JSON the TypeScript runtime produces keeps only the bare `"active"` (`runtime-output-format`).
Evaluating in the wasm module means one evaluator and one formatter for the CLI, the playground and
the docs check. It costs module size: the interpreter was linked but unreachable before, so the
optimizer dropped it. `nx.wasm` goes from 2,340,407 to 2,610,396 bytes, 11.5% more (measured for
task 1.5).

*Alternative:* evaluate the IR with `@nx-lang/ir-runtime` and port `format.rs` to TypeScript. That
means two formatters to keep in step and output that can't match the CLI. Rejected.

### The formatter moves to `nx-api` and annotates what it writes

`format.rs` becomes `nx-api`'s `nx_text` module, with its tests. The formatter already walks every
value as it writes it, so it records a node for each one as it goes: where it starts and ends in the
text, and what it is. Two entry points share that walk:

- `format_nx_text(&Value) -> Result<String, …>` for the CLI, which ignores the nodes.
- `eval_program_artifact_nx_text(&ProgramArtifact) -> Result<NxValueText, Vec<NxDiagnostic>>`. It
  evaluates the entry module's `root` exactly as `eval_program_artifact` does, and stops before the
  conversion to `NxValue`. It then formats the interpreter `Value` with nodes, and fills in the type
  and declaration of each node from the artifact's resolved program.

`nxlang run --format nx` calls the first, so the CLI's output doesn't change; its existing tests
prove that. A value with no NX spelling becomes a diagnostic (code `nx-text-unspellable`) instead of
a bare string error.

`NxValueText` is serialized as:

```
{ text, nodes: [node] }          nodes in text order, each parent before its children
node = { start, end,             UTF-16 offsets into text, end exclusive
         parent,                 index of the enclosing node, absent at the top
         role,                   record | property | sequence | case | scalar | function | empty
         type,                   NX spelling of the type: "Task", "string", "Status", "int"
         name?,                  a property's name
         optional?,              true for a property declared optional
         count?,                 a sequence's length
         declaration? }          NxTextSpan in the entry module
```

A sequence's `type` is its items' common type followed by `*`, or `object*` when they differ.
`declaration` is the span of the record, component (external ones included), union case, property
or function declaration the node comes from, when the entry module declares it. A prelude type has none. `type` on a property is its
declared type, not the value's, so `done` reads `boolean`. An optional `subtitle?:string` reads
`string` with `optional` set, because the language service puts the mark on the name.

A number, string, boolean or `{}` written directly as a property's value has no node of its own:
the property's node says everything about it, so hovering the value explains the property.

Offsets are UTF-16 because the only reader is JavaScript. The nodes are not a second formatter:
nothing outside this walk decides layout, so the viewer's text is the CLI's text by construction.

*Alternative:* return a typed value tree and lay it out as NX in TypeScript. That is the second
formatter the previous decision rejects. Rejected.

### ABI 3: one new export, one new artifact method

`bindings/wasm/native` gains `nx_wasm_program_evaluate_nx(handle)`. On success its result payload is
the `NxValueText` above, and on failure the usual diagnostics status. `ABI_VERSION` goes from 2 to 3.
In TypeScript, `NxProgramArtifact.evaluateNx(): NxValueText` throws `NxEvaluationError`, mirroring
the Node SDK's `evaluateJson()`. The parity suite gains a case per corpus source with a `root`. It
compares `evaluateNx().text` with `nxlang run` output, which a test helper produces by running the
CLI binary that `cargo test` builds.

### The worker protocol swaps "compile to IR" for "evaluate to text"

The request stays `{ source }`. The answer becomes:

```
{ diagnostics, outcome }
outcome = { kind: "value",   text, nodes, truncated }
        | { kind: "noRoot" }
        | { kind: "error",   diagnostics }
```

In the worker, the source is built with `buildProgramArtifact`, with no implicit imports now that the
catalog is gone. Then `evaluateNx()` runs, and the artifact is disposed. The 100,000-character cap is
applied in the worker, so an enormous string is never structured-cloned to the page. The cap drops
every node that starts past the cut and ends the rest at it. The deadline, the trap recovery and the
load budget in `src/worker/channel.ts` and `session.ts` stay as they are.

The interpreter ends a runaway program itself: unbounded recursion at its depth limit, an endless
loop at its operation limit, each as a runtime error. Two stacks stand in the way in a browser:

- The module's linear-memory stack, 1 MiB by default, ran out about 220 plain calls deep. A build
  script links it with 16 MiB.
- The engine keeps the wasm frames themselves on its native stack, which the module cannot size.
  In a Chromium dedicated worker it ran out 325 to 517 NX calls deep, depending on what each call
  does, well short of the interpreter's default limit of 1,000.

So the wasm build evaluates with `max_recursion_depth` lowered to 200 (`MAX_RECURSION_DEPTH` in
`bindings/wasm/native/src/lib.rs`, through `nx-api`'s `eval_program_artifact_nx_text_with_limits`),
below every shape measured, with room for heavier calls and smaller engine stacks. The CLI and
.NET keep 1,000. A test runs the module in a Node worker with a 0.5 MB stack, stricter than a
Chromium worker's, and fails if the limit stops holding. A program whose calls are heavier still can
reach the engine's limit anyway; the worker names that trap as the program recursing too deeply,
replaces the host, and does not retry it.

### The viewer is a custom element in its own package

`packages/value-view` publishes `@nx-lang/value-view`. It holds one custom element, `<nx-value>`, in
plain TypeScript and DOM with a shadow root, so its styles neither leak nor get overridden.

| Property or event | Meaning |
|---|---|
| `value` | an `NxValueText` |
| `truncated` | shows the cut-off notice after the text |
| `stale` | mutes the text and shows an "out of date" badge |
| `describe(node)` | host hook returning hover markdown, or nothing for the default |
| `highlighter` | a Shiki highlighter to use; without one the element loads its own |
| `nx-value-navigate` | event fired when a node with a `declaration` is clicked, carrying it |

Rendering:
- **Highlighting.** The text is tokenized by Shiki with the grammar `@nx-lang/language` publishes,
  the grammar `@nx-lang/monaco` uses, so output and source color alike. Light and dark follow the
  page through Shiki's dual themes and CSS custom properties.
- **Folding.** Every record, property and sequence node inside the value whose text spans more than
  one line and that starts its line gets a fold toggle in the gutter. A node that starts partway
  along a line has no gutter beside it, and folds with the node that starts the line; folding the
  whole value would hide everything. A folded node shows its first line, then an ellipsis that unfolds it. All
  nodes start unfolded.
- **Hover.** Hovering the innermost node under the pointer shows `describe(node)`, or else a default
  built from the node alone. The default follows the hover conventions of `nx-language-service`: a
  fenced `nx` fragment such as `Task`, `(property) done: boolean` or `(case) Status.active`, with a
  sequence adding its count. The element renders the small markdown subset hovers use: fenced `nx`
  blocks, highlighted, and paragraphs with inline code.
- **Copy.** A copy control puts the text on the clipboard.

In the playground, `describe` asks the language service for the hover at the node's `declaration`,
so hovering `Task` in the output shows exactly what hovering its declaration in the source shows.
When the source has changed since the value was produced, the spans no longer
line up, so `describe` returns nothing and the default hover shows. `nx-value-navigate` selects the
declaration in the source pane.

*Alternatives:*
- A read-only Monaco editor, as the playground's output. It folds and hovers, but no other host
  wants to load Monaco to show a value. Rejected.
- A React component. It is simplest for the playground, but every other host would bring React.
  React 19 uses custom elements directly, so the playground loses nothing. Rejected.
- Coloring from the node roles instead of the grammar. It needs no Shiki, but the roles don't cover
  punctuation, and the output would color differently from the source. Rejected.

### The address: `/play/<id>` for examples, `#code=` for everything else

The prefix is `/play`. The label everywhere stays "Playground". The Worker keeps its name,
`nxlang-playground`, and the site stays in `sites/playground`. Nothing redirects from
`/playground`; its links went public a day before this change was written.

| Part | Rule |
|---|---|
| Payload | `base64url(deflateRaw(utf8(source)))`, unpadded |
| Decoding in the browser | `DecompressionStream("deflate-raw")`, Baseline since 2023, no dependency |
| Encoding in the browser (Share) | `CompressionStream("deflate-raw")` |
| Encoding in the website build | `node:zlib`'s `deflateRawSync` |

DEFLATE output isn't canonical across implementations, so the round-trip contract is "any valid raw
DEFLATE stream decodes". The shared fixture (`sites/playground/src/share/fixture.json`: a source and
a payload) is generated with `node:zlib`. The website test encodes the source and must produce that
payload exactly. The playground test decodes the payload and must produce that source exactly.

History:
- Edits call `history.replaceState` after the pause that triggers evaluation.
- Choosing an example calls `pushState`, so Back returns to the edited source.
- When a fragment is present it wins over the path.

*Alternative:* lz-string's `compressToEncodedURIComponent`, which the TypeScript playground uses. It
is a dependency on both sides, and it compresses worse than DEFLATE on source text. Rejected.

### Examples live as files with committed output

```
src/examples/
  examples.json          [{ id, title, topic, docs }]   ordered, grouped by topic
  nx/<id>.nx             the source
  nx/<id>.out.nx         what `root` prints
```

`scripts/check-examples.mjs` is rewritten. For each entry it compiles and evaluates through
`@nx-lang/sdk-wasm`'s Node entry, and fails on any diagnostic or on output that differs from
`.out.nx`. It checks that `docs` names a page under `sites/website/src/content/docs/`, and that
there are between 10 and 20 examples. With `--update` it rewrites the `.out.nx` files, for a deliberate
language change. The default example is `hello`. The set follows the spec's topic list, one or two
examples per topic. The components example uses components without handlers, because an action
handler has no NX spelling.

### UI: the same React shell, one view

- `App.tsx` drops the router's gallery branch.
- **Layout:** header (copied from the website: same links, same CSS custom properties for color and
  type); a slim toolbar; then the two panes.
- **Toolbar:** at the start, an "Examples" drop-down: a native `<select>` whose options are grouped
  by topic with `<optgroup>`, showing the current example or "Edited" once the source no longer
  matches one. Next to it, a small link to the current example's docs page. Share at the end, then a
  quiet "Draw with NX: DrawnUI fiddle" link. The examples are there to get a visitor started, so
  nothing else in the page points at them.
- **Output pane:** `<nx-value>`, fed from the worker's outcome. A missing `root`, a runtime error
  and an unspellable value are shown as messages in the pane in its place.
- **Narrow screens:** below 768 px the panes stack. The toolbar is the same, since a native
  `<select>` already suits a phone.

### The website side: an Expressive Code plugin and `nx output`

- **The button.** A small Expressive Code plugin in `sites/website` reads each `nx` block's meta
  words. On unmarked and `invalid` blocks it adds an "Open in playground" link just below the
  block, where it cannot collide with the copy button or a title. The link's `href` is computed at build time from the block's exact code with the encoder
  above, so it needs no client script.
- **The check.** The code-block check from `launch-nxlang-website` learns `output`. For an `nx output`
  block it evaluates the preceding unmarked block with `evaluateNx()` and compares the text with the
  block's, trailing whitespace trimmed.
- **The landing page.** The hero's hand-written value becomes an `nx output` block.

## Risks / Trade-offs

- **[The DrawnUI playground was the only in-repo end-to-end test of `@nx-lang/ir-runtime` against a
  large, real catalog]** → The runtime keeps its conformance corpus, and the fiddle's own tests run
  it against the fiddle's catalog on every NX update. The gap is recorded in `specs/future.md` under
  the fiddle's section, not papered over.
- **[`/playground` links break]** → They reach the website's 404 page, whose header links to
  `/play`. The site was public for a day under that address.
- **[The nodes roughly double what crosses from wasm to the page]** → They are small next to the
  text, and the worker's cap bounds both. Example-sized values stay far below it.
- **[The element loads Shiki and the grammar]** → Shiki's code is shared with the editor's in the
  bundle; what the element adds is a second highlighter instance. It cannot share the editor's:
  coloring with a light and a dark theme switches a highlighter's current theme, and the Monaco
  bridge tokenizes with whatever theme that is, which recolored the source from the wrong palette
  when tried. Another host pays for Shiki once.
- **[Evaluating on every pause makes a slow `root` feel sluggish]** → The same debounce and deadline
  as today's compile. Evaluation of example-sized programs is well under the compile time already
  paid.
- **[Some docs blocks marked `invalid` produce a confusing first error when opened in the
  playground]** → That is accurate, and it is the error the page is describing. No mitigation is
  needed.
- **[The ABI bump breaks a host that pairs an old wrapper with a new module]** → The wrapper and
  module ship in one package, and the version check already refuses a mismatch with a clear error.

## Migration Plan

This ships as one PR after `launch-nxlang-website` is archived. On merge both deploy workflows run.
Until the playground's deploy claims the `/play` routes, `/play` reaches the website's 404 page
for a few minutes.

`wrangler rollback` restores a Worker's files but not its routes, so it would leave files built for
`/playground` answering on `/play`. Roll back by reverting the merge and letting both workflows
deploy again.

The next `v0.x` package release carries `evaluateNx()` and the first `@nx-lang/value-view` to npm.
The fiddle can stay on its current pin.
