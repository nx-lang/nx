# NX Playground

The public playground for NX, at [nxlang.org/play](https://nxlang.org/play): NX source on the left,
and on the right what its `root` returns. NX has no `print`, so the value a program evaluates to is
its output, shown as NX text — the text `nxlang run` prints — by the `<nx-value>` element of
[`@nx-lang/value-view`](../../packages/value-view/README.md). Long values fold, and hovering a type
name, property or case in the output shows the same hover its declaration shows in the source.
Clicking one selects the declaration.

The NX pane's Visual and Source switch, Visual by default, shows the source itself visually, through
the `<nx-viewer>` element of [`@nx-lang/viewer`](../../packages/viewer/README.md): an element over
several lines as a group with a row to each property and no tags, an element on one line as its name
and properties, and the rest as written, in the editor's colors, for a reader who does not write NX.
The reading comes from the worker's `sourceTree` query, refreshed when typing pauses, as evaluation
is. Visual starts with the source's supporting declarations folded to a line each, so the content
comes first, and offers Collapse all and Expand all. Switching to Visual selects the node at the
cursor, and switching back puts the cursor at the start of the selected node.

Values edit in Visual too: click a value, or select a unit and press Enter. Each edit asks the
worker's `sourceEdit` query for its text edits and applies them to the editor, as one step of its
undo history, so Source shows them and the title bar's Undo and Redo, or Ctrl or Cmd+Z, take them
back and forth in either view; the reading's next tree is asked for at once. Double-clicking an expression, which Visual cannot edit,
switches to Source with the cursor at its start.

When a component can run, the optional Preview tab starts a preview
session of [`@nx-lang/previewer`](../../packages/previewer/README.md) in the worker, on the IR
runtime: for the component whose element `root` returns, with that element's props, or for the
first component the source declares, and a picker lists the rest. Every action handler in the
output carries a button named for its action; pressing one runs the handler, or opens a form built
from the action's input schema when the action has fields. A timeline keeps every step, so the
visitor can go back, take another branch, and follow either one again. Under the output are the
component's state and the actions the step emitted to its host. Each edit that compiles reloads the
session: the state is kept when the new program accepts it, the run is replayed when it does not,
and new props in `root`'s element are rendered on the state as it stands. Clicking a record's type
name selects the element expression that built it. The "A counter to click" example previews at
once.

```nx
external component <Button label:string emits { Tapped { } } />

component <Counter start:int = 0 /> = {
  state { count:int = {start} }
  <Button label={"Count: " + count} onTapped=<State.Update count={count + 1} /> />
}

<Counter start=3 />
```

Every part of that runs in the visitor's browser. The compiler and the language service are one
WebAssembly module, loaded into a Web Worker when the page mounts; nothing is compiled on a server.

```
NX source ──▶ sdk-wasm compiler ──▶ TypeScript IR workbench ──▶ current value + origins
   editor          (worker)                 (worker)                  │
     │                                                ┌───────────────┴──────────────┐
     │                                                ▼                              ▼
     │                                        structured Output            registered Preview
     │                                        (@nx-lang/value-view)        (graphics / component)
     └── hover / completion ──▶ @nx-lang/language-core ──▶ sdk-wasm language snapshot
```

One worker, one host: evaluating and answering hover share an instance, so a diagnostic and a hover
range land on the same line of the visitor's text. A request that traps or overruns its deadline
costs that one request — the worker is replaced and the editor stays live. Runaway recursion and
endless loops end sooner, at the IR runtime's own limits, as runtime errors. In the browser, calls
nest at most 200 deep, a limit set below what a browser worker's stack can hold. Preview runs every
call under the IR runtime's limits, the same depth and a budget of operations, so a handler that
never stops ends with the runtime's message and the session is unchanged.

To see NX draw interfaces, use the [DrawnUI fiddle](https://fiddle.drawnui.net), which the toolbar
links to.

## Prerequisites

- **Node 22.18+** — the tests import TypeScript directly.
- **A Rust toolchain and `clang`** — the compiler ships as a WebAssembly module built from the Rust
  crates, and it is gitignored, so it is built from source. The `wasm32-wasip1` target comes with the
  toolchain (`rust-toolchain.toml` lists it); clang compiles tree-sitter's C sources for that target,
  and the pinned WASI sysroot the build needs is fetched into a gitignored `.cache/` by
  `scripts/fetch-wasi-sysroot.mjs`, which `pnpm -r build` runs for you.

## Running it

The site is a member of the repository's root pnpm workspace, which links the wasm SDK and the
shared editor and viewer packages it depends on. pnpm comes through corepack.

```bash
# once, from the repository root
corepack enable
pnpm install
pnpm -r build     # the wasm module, the shared packages, then this site

cd sites/playground
pnpm run dev      # the whole site at http://localhost:5173/play
```

`pnpm run dev` is the whole development setup for the playground: there is no second process to
start, because the compiler is in the page.

The header's Docs link and each example's docs link are paths on the website, which production
serves from the same domain. To follow them locally, run the website's dev server as well; the
playground's dev server forwards every path outside `/play` to it, so one address behaves like
nxlang.org, with links working in both directions:

```bash
cd sites/website && pnpm run dev    # http://localhost:4321, in a second terminal
# then use http://localhost:5173/play for both sites
```

Set `NX_WEBSITE_DEV_URL` when the website runs somewhere other than `http://localhost:4321`. Without
it running, a docs link answers with a page saying how to start it.

To serve a build the way production does, through the same Worker script and headers:

```bash
pnpm run build                        # writes dist/play/ and dist/_headers
npx wrangler@4.141.0 dev              # http://localhost:8787/play
```

`pnpm run preview` serves the build with Vite instead, without the Worker's routing or headers.

```bash
pnpm run typecheck       # tsc over the site
pnpm test                # Worker, route, share, policy, IR session and dev-shell tests, then every example
pnpm run check-examples  # every example evaluates to its committed output
```

## Addresses

Everything the site serves lives under `/play`, and the website (`sites/website`) serves the rest of
the domain. The prefix is spelled once, in `base.mjs`, and the Worker script, the Vite config and the
client all take it from there.

| Address | Opens |
|---|---|
| `/play` | the default example, `hello` |
| `/play/<id>` | that example; an unknown id opens the default one with a notice |
| `/play#code=<payload>` | the source the payload carries, whatever the path names |

The payload is the source's UTF-8 bytes, compressed with raw DEFLATE, in unpadded base64url. Any
site can build a link to the playground with a few lines — the website does, at build time, for
every `nx` code block that is a complete program. `src/share/fixture.json` holds the playground's
decoder and the website's encoder to one encoding.

Typing updates the address, once the edit has been evaluated, without adding a history entry, so a
reload keeps the visitor's work. Choosing an example adds one, so Back returns to the edited source.
Share copies the address.

## Examples

The examples give a visitor something to start from, one or two per topic, and sit in a single
drop-down so they stay out of the way. Each is a program in `src/examples/nx/<id>.nx`, with what its
`root` prints committed beside it as `<id>.out.nx`, and an entry in `src/examples/examples.json`:

```json
{ "id": "records", "title": "Records and defaults", "topic": "Records", "docs": "language-tour/types#records-and-inheritance" }
```

- `topic` is the group the drop-down lists it under; the entries' order is the drop-down's order.
- `docs` is a page under `sites/website/src/content/docs`, with an optional heading anchor. The
  toolbar links to it.

To add one, write the `.nx` file and the entry, then generate its expected output and check it:

```bash
pnpm run check-examples -- --update   # writes every .out.nx from the current compiler
pnpm run check-examples               # fails on a diagnostic, a changed value or a missing docs page
```

`--update` is also how a deliberate language change updates every example; review the diff it
leaves. The check evaluates through the same module and the same IR workbench the browser uses,
so what is checked is what ships, and it keeps the set between 10 and 20 examples. The larger
examples, under "Larger examples", and "Visual test cases", under "Test cases", are not counted. The
test cases hold one or two of every reading and editing case Visual has, each with a comment saying
what to try, for checking Visual by hand after a change.

## Layout

| Path | What it is |
|---|---|
| `base.mjs` | the site's path prefix |
| `worker/index.mjs` | the Cloudflare Worker script: the shell for the client's addresses, not found for everything else |
| `wrangler.jsonc` | the Worker's name, routes and static-assets settings |
| `_headers` | the cache policy for the built files, copied to `dist/_headers` by the build |
| `scripts/check-examples.mjs` | one check over the whole example set |
| `src/paths.ts`, `src/routes.ts` | the prefix as the client sees it, and the address scheme under it |
| `src/share/` | the `#code=` encoding and its shared fixture |
| `src/compile/` | shared source/output limits, diagnostic types/classification and compiler-failure recovery; evaluation lives in the IR workbench |
| `src/worker/` | the compiler worker: its module load, its session, and the main thread's channel to it |
| `src/language/` | the language service the editor and the output's hover ask |
| `src/editor/` | Monaco through `@nx-lang/monaco` (grammar, highlighting, hover, completion) |
| `src/reading/` | the NX pane's Visual view: the source tree query and the selection carried to and from the editor |
| `src/output/` | the output pane, `<nx-value>` or why there is nothing to show, and its Output and Preview switch |
| `src/preview/` | Preview: the worker's session (`workbench.ts`), the calls to it, and the page's timeline, buttons, forms and state |
| `src/header/` | the website's header |
| `src/examples/` | the examples and their expected output |
| `src/theme.ts` | light or dark, following the website's choice |

## Deploying

The public site is static files on a Cloudflare Worker, `nxlang-playground`, with no server behind
it. A push to `main` that touches the site or a package it depends on runs
`.github/workflows/deploy-playground.yml`, which builds and tests the workspace, the WebAssembly
module included, then uploads `dist/` with `wrangler deploy`. The Worker's routes (`nxlang.org/play`
and `nxlang.org/play/*`) are in `wrangler.jsonc`. The day-to-day flow (deploy, verify, roll back) is
in `docs/deployment.md` at the repository root, and the one-time Cloudflare setup is in
`docs/deployment-setup.md`.

**Routing.** A request for a file that exists gets the file. Otherwise `worker/index.mjs` answers:
the shell for `/play`, `/play/` and one path segment below it, and not found for everything else, so
a missing asset is a clear 404 rather than HTML where a script was expected.

**Cache headers.** `_headers` makes hashed build output under `assets/` (scripts, styles and the NX
compiler module) `immutable` for a year and the shell `no-cache`. A deploy changes the hashes and
the shell names the new ones, so old assets can stay cached forever.

**One request at a time, in the visitor's own tab.** The compiler is single-threaded and answers one
request at a time, but that thread is a Web Worker in the visitor's browser: a slow evaluation costs
that visitor a moment and costs everyone else nothing. A request that overruns ten seconds has the
worker terminated and replaced; one that traps has its host replaced inside the worker. Either way
the editor stays live and the next evaluation runs.

## Preview and Output

The right pane always offers structured **Output**. A **Preview** tab appears only when a registered
previewer supports the evaluated result, and is selected initially. Choosing Output persists across
edits; opening another document resets the preference. Unsupported values show Output without a
redundant tab switcher. Both presentations share the worker's current TypeScript IR value, including
stateful component steps; the legacy HIR interpreter is not used by this browser flow.

Choose **Graphics: shapes and text** or open `/play/graphics`. Its graphics previewer prepares that
same value through the existing WASM compiler module and paints with HTML Canvas 2D. Rendering adds
no WASM, CanvasKit, native Skia or server dependency. The component inspector is another previewer,
retaining actions, state and timeline. Ordinary structured values are Output-only; a survey form
renderer can be registered separately in the future. Renderer matching uses resolved module and
declaration identities rather than display names. Compilation errors preserve preview availability
and mark retained output stale; graphics preparation errors clear the scene without hiding Preview.

The bundled DejaVu Sans face is exposed as `Fixture` (400, normal); both the worker and page load
those bytes explicitly. The font license is linked under the canvas. Unsupported selections and
missing glyphs fail clearly. Browser metrics/rasterization use the `browser-canvas2d-v2`
configuration and can differ from the native Rust/Skia preview.

Geometry, transforms, winding rules, bounds and resource limits share `nx-graphics` preparation.
Translucent subtrees paint to isolated, bounded offscreen canvases. Only a completed frame from
the current source is published. The element inspector shows revision-scoped identities, bounds,
text metrics and source provenance; canvas clicks select by visible bounds.

```sh
pnpm --dir bindings/wasm build
pnpm --dir sites/playground typecheck
pnpm --dir sites/playground test
# An installed Chromium/Chrome is required; no browser download is performed.
CHROME_PATH=/usr/bin/chromium pnpm --dir sites/playground test:graphics-browser
```

Set `NX_PLAYGROUND_PREVIEW=1` after `pnpm --dir sites/playground build` to test the production
bundle instead of the development server (CI uses this).

Set `NX_GRAPHICS_SCREENSHOT=/tmp/nx-drawing.png` to save the browser integration's final image.

### Shared source and preview selection

Selection is shared across preview adapters, rather than tied to graphics. `src/preview/selection.ts`
maps current renderer IDs to authored source spans, scoped to the exact source snapshot. Canvas
uses scene node IDs; structured Output uses formatted record indices. Helpers retain construction
identity separately from optional call-site ancestry, so selecting a Star navigates to the authored
Star invocation rather than a Polygon inside the graphics library.

Selecting a preview element keeps Visual or Source active and selects the corresponding authored
expression. Selecting an expression in Visual highlights every matching output, including loop
instances. The **A full survey** example opens structured Output at `/play/survey`; selecting a Survey
Question reveals/highlights its output records, and choosing a Choice selects its source.
A survey-specific form previewer can use the same contract later.

Scene preparation preflights declaration identities and node/depth/geometry limits before measuring
any text. The layer budget bounds peak concurrently allocated opacity layers, so sibling translucent
shapes reuse that budget. Source-mode cursor/selection changes also update the shared output
highlight, using Unicode-safe UTF-16 to UTF-8 conversion.

### Adding a previewer

Libraries declare type-to-renderer associations in NX, for example
`visualizer Drawing = <ElementVisualizer preview=<PreviewVisualizer renderer="nx.graphics"/>/>`.
The worker reads resolved visualizer metadata through a language snapshot, caches it per build,
and matches the evaluated result's nominal declaration. The opt-in
`includeUnreferencedDeclarations` query includes loaded visualizer targets constructed inside helpers.

`src/preview/registry.ts` is the host implementation allowlist: it associates supported logical
renderer IDs with bundled adapters, rather than listing NX type names. Unsupported IDs remain
Output-only. Adding an implementation still requires its preparation and browser view; descriptors
load no code and there is no dynamic plugin installation UI. Renderers share `preview/selection.ts`
for source mapping. The component inspector remains a host facility for runnable component sessions.

Browser graphics use physical-pixel backing stores at the device scale while keeping logical CSS
coordinates. The browser scene depth limit is 48, fitting the compiler SDK's bounded JSON bridge;
the neutral/native default remains 64. Canvas selection tests fill/stroke geometry and ancestor clips,
with conservative ink bounds used only for text. Text is measured and painted at a 1000px reference
size and scaled to local units to avoid browser font-size rounding and clamping.

Raster resolution is capped to the physical surface budget and reduced further for scenes with many
concurrent opacity layers, so high browser zoom does not reject a scene that fits at scale 1. Text
painted bounds include 1.5 physical pixels of host raster uncertainty after the world transform,
independently of intrinsic font measurement. Node/depth limits are checked with authored provenance
before JSON transport; pending valid edits show a quiet updating indicator without stale styling.
