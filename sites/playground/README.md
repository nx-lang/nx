# NX Playground

The public playground for NX, at [nxlang.org/playground](https://nxlang.org/playground): NX source
on the left, what it draws on the right. It opens on a gallery of the DrawnUI React demo pages
ported to NX; any card opens in the editor view and can be edited live. Today it draws with
DrawnUI; the site is shaped so that further targets can join later.

It also runs the whole NX pipeline end to end against a large real catalog: source → NX IR →
evaluated value tree → DrawnUI controls → CanvasKit.

Every part of that runs in the visitor's browser. The compiler and the language service are one
WebAssembly module, loaded into a Web Worker when the editor view mounts; nothing is compiled on a
server.

```
NX source ──▶ @nx-lang/sdk-wasm ──▶ NX IR image ──▶ @nx-lang/ir-runtime ──▶ renderer ──▶ DrawnUI
   editor        (worker)                            (browser)              (browser)     canvas
     │
     └── hover / completion ──▶ @nx-lang/language-core ──▶ sdk-wasm language snapshot
           (@nx-lang/monaco)          (worker)                    (worker)
```

One worker, one host: compiling and answering hover share an instance and the same catalog module,
so a diagnostic and a hover range land on the same line of the author's text. A compile that traps
or overruns its deadline costs that one request — the worker is replaced and the editor stays live.

The catalog (`catalog/skia.nx`) is a module of its own that every compile and every language query
imports implicitly, so the visitor's document is analyzed exactly as written. A compile answers
with the visitor's module alone: an NX IR artifact of a few kilobytes that names the catalog in its
module table and carries none of its declarations. The catalog's own artifact is emitted at build
time — a Vite plugin in `vite.config.ts` compiles it through the same wasm module and serves it as
`virtual:nx-catalog-artifact` — and the renderer prepares it once per page and links each compile
against it. Nothing derived is committed: the catalog text is the one source, and in development
an edit to it re-emits the artifact.

## Prerequisites

- **Node 22.18+** — the route tests import TypeScript directly, which Node strips unflagged from 22.18
- **A Rust toolchain and `clang`** — the compiler ships as a WebAssembly module built from the Rust
  crates, and it is gitignored, so it is built from source. The `wasm32-wasip1` target comes with the
  toolchain (`rust-toolchain.toml` lists it); clang compiles tree-sitter's C sources for that target,
  and the pinned WASI sysroot the build needs is fetched into a gitignored `.cache/` by
  `scripts/fetch-wasi-sysroot.mjs`, which `pnpm -r build` runs for you.
- **A checkout of DrawnUI React** at `~/src/DrawnUi.React`, but only to re-run the asset sync. The
  runtime is the `drawnui-react` npm package, and the copied assets are committed, so a normal
  build needs nothing more.

## Running it

The site is a member of the repository's root pnpm workspace, which links the wasm SDK, the IR
runtime and the shared editor packages it depends on. pnpm comes through corepack.

```bash
# once, from the repository root
corepack enable
pnpm install
pnpm -r build     # the wasm module, the IR runtime, the shared packages, then this site

cd sites/playground
pnpm run dev      # the whole site at http://localhost:5173/playground/
```

`pnpm run dev` is the whole development setup: there is no second process to start, because the
compiler is in the page. To serve a build the way production does, through the same Worker script
and headers:

```bash
pnpm run build                        # writes dist/playground/ and dist/_headers
npx wrangler@4.141.0 dev              # http://localhost:8787/playground
```

`pnpm run preview` serves the build with Vite instead, without the Worker's routing or headers.

Everything the site serves lives under `/playground` — the gallery at `/playground`, an example's
editor view at `/playground/<id>`, every asset under the same prefix — and the website
(`sites/website`) serves the rest of the domain. The build writes files under `dist/playground/`, so
a file's path under `dist/` is the path it is served at. The prefix is spelled once, in `base.mjs`,
and the Worker script, the Vite config and the client all take it from there.

```bash
pnpm run typecheck       # tsc over the site
pnpm test                # Worker, route, compile, worker and dev-shell tests, then every example
pnpm run check-examples  # every example compiles, evaluates, and declares its coverage
```

The example check compiles through the same module and the same catalog path the browser uses, and
links each example against the catalog artifact emitted the way the build emits it, so what is
checked is what ships.

## Layout

| Path | What it is |
|---|---|
| `base.mjs` | the site's path prefix |
| `catalog/skia.nx` | the generated NX catalog: external components for the DrawnUI control set, compiled to its artifact at build time |
| `catalog/catalog-meta.json` | which types are unions, which are records, which records are constructed |
| `scripts/generate-catalog.mjs` | generates both from the pinned `drawnui-react` package's declarations |
| `scripts/sync-drawnui.mjs` | copies DrawnUI's assets and demo pages from the tag of the pinned release |
| `scripts/check-examples.mjs` | one check over the whole example set |
| `scripts/emit-example-ir.mjs` | emits each example's NX IR, for proving an edit changed only notation |
| `scripts/compile-example.mjs` | the wasm host and catalog those two scripts compile through |
| `worker/index.mjs` | the Cloudflare Worker script: the shell for the client router's addresses, not found for everything else |
| `wrangler.jsonc` | the Worker's name, routes and static-assets settings |
| `_headers` | the cache policy for the built files, copied to `dist/_headers` by the build |
| `src/paths.ts`, `src/routes.ts` | the prefix as the client sees it, and the address scheme under it |
| `src/compile/` | the compile seam, NX source + catalog → the visitor's NX IR with diagnostics, and the catalog's own artifact |
| `src/worker/` | the compiler worker: its module load, its session, and the main thread's channel to it |
| `src/language/` | the language service the Monaco integration is registered with |
| `src/render/` | the catalog prepared once, each compile linked against it, and evaluated NX values → DrawnUI controls |
| `src/editor/` | the editor view, and Monaco through `@nx-lang/monaco` (grammar, highlighting, hover, completion) over `src/language/` |
| `src/gallery/` | the gallery |
| `src/examples/` | the ported examples and their metadata |
| `reference/demo-pages/` | the original TSX pages, for comparison only; never built |
| `docs/UPSTREAM.md` | the upstream tag and commit the assets and demo pages were copied from |

## Moving the DrawnUI pin

The site draws with the `drawnui-react` npm package, pinned to one exact version in
`package.json`, the same version the DrawnUI fiddle pins. Moving it takes three steps, and the result
is one reviewable diff:

```bash
pnpm add drawnui-react@<version> --save-exact   # move the pin
pnpm run generate-catalog                        # regenerate the catalog from the new declarations
pnpm run sync-drawnui                            # copy the assets from ~/src/DrawnUi.React at v<version>
```

The sync reads `~/src/DrawnUi.React` by default. For a checkout elsewhere, pass
`pnpm run sync-drawnui -- --source /path/to/DrawnUi.React`.

`catalog/catalog-meta.json` records the version the catalog was generated from, and
`docs/UPSTREAM.md` the release the assets were copied from. `pnpm test` fails when either differs
from the pin, naming both versions and the command that brings it level.

The package ships only its compiled runtime. The asset trees the examples read are not in it, so the
sync copies them: `fonts/`, `images/`, `lottie/` (Skottie animations), `anims/` (sprite sheets) and
`shaders/` (SkSL, including the `transitions/` set the shader carousel uses), each landing under
`public/` by the same name. It also copies the demo pages into `reference/`. It reads them at the tag
`v<version>` with `git archive`, so they match the code that draws them, and the checkout's working
tree and `HEAD` are left alone. If the tag is missing, it says to run `git fetch --tags` there.
`docs/UPSTREAM.md` records the tag and commit.

Nothing in the package is patched. Where NX needs DrawnUI to behave differently, `docs/CATALOG.md`
records the difference and the upstream change it waits on.

The runtime loads CanvasKit's "full" build, roughly 0.9 MB more of WASM than the default one,
because Lottie playback (Skottie) lives only there. The binary is a hashed build asset and is cached
like the previous one.

## What it does not do

**Handlers and state run inside components; the root is evaluated once.** The renderer holds an
instance for every use of an authored component, keyed by its position in the drawn tree. A handler
bound on a control inside a component body becomes that control's DrawnUI event (`onTapped` is
`Tapped`, `onToggled` is `Toggled`, and so on through the catalog's emits); the event dispatches
the handler against the instance whose body bound it, its `<Update ... />` patches that instance's
state, and the drawing redraws from the root without a compile. An action a component emits goes
to the handler its parent bound, and the parent's state is patched in turn. An action nothing in
the tree handles — an emit nobody bound, or an action outside the component's contract that a
handler returns, such as `<DoSearch />` from a page component — is a host effect, listed in the
diagnostics pane with the instance that produced it; the site has no host to receive it. A dispatch
that fails is reported there too, and leaves the drawing as it was.
Editing the source recompiles and starts every instance again.

The root function is evaluated, not instantiated, so a handler bound outside any component has
nothing to run it: the control draws without a callback and the site says so. The pattern every
interactive example uses is a page component:

```nx
component <Page /> = {
  state { count:int = 0 }
  <SkiaStack>
    <SkiaLabel Text={if count > 0 { "tapped" } else { "untapped" }} />
    <SkiaButton Text="Tap" onTapped=<Update count={count + 1} /> />
  </SkiaStack>
}

<Page />
```

DrawnUI's own behavior works as before: scroll regions scroll, carousels swipe, drawers drag,
ripples play, switches toggle, sliders drag, whether or not a handler is bound.

All twenty demo pages DrawnUI had at 0.1.0-preview.4 are ported, and each says where it stands:
**complete** (no note), **static** (drawn correctly, with some of the original's motion or
interaction absent), or **reduced** (scaled down, because NX cannot express the mechanism the
original demonstrates). SVG, Text, Shapes and Common Controls are complete; the rest gained
interaction or code-driven mechanisms upstream and say so. Every non-complete example names its gap
from a fixed vocabulary, and the vocabulary separates what NX lacks from what a port has not used:
`animation` and `code-behind` (an engine object built, driven or read from code: a shader effect,
a CanvasKit filter, a sprite set, a cell class with drag logic, a method called on a control) are
capabilities NX does not have, while `event-handlers` and `component-state` are capabilities NX has
and the port does not use yet. List virtualization is not a gap: a templated `SkiaLayout` binds
`ItemsSource` and an element function through `ItemTemplate`, and DrawnUI realizes, recycles and
measures the cells, which is how Recycled cells and Uneven cells are ported. The gallery can be
read as a coverage report on NX rather than a list of disclaimers, and a landed capability is never
presented as missing.

One demo page added upstream since preview.4 has no NX example, and the gallery shows no entry for
it: **Pong** (`reference/demo-pages/PongPage.tsx`). It is a `DrawnGame`, with a game loop that
moves sprites every frame from code and reads the keyboard and touch. Every part of it is
`animation` or `code-behind`, so even a reduced port would draw a still field and two paddles that
do not move.

The readouts are wired: a tap count, a selected index, a slider's value, a speed, `IsOpen`. Each is
a number or a boolean held in a page component's state, and `+` converts it to text where it joins
the words around it (`"Tapped " + taps + "×"`). What a wired example still leaves out is what the
original does by calling into a control — `Seek(30)`, `SelectAll()`, the accessibility manager's
node count — which is `code-behind`, and the example says so at the point the call would appear.

See `docs/FINDINGS.md` for the toolchain gaps this site ran into, and `docs/CATALOG.md` for where the
catalog diverges from the DrawnUI object model.

## Deploying

The public site is static files on a Cloudflare Worker, `nxlang-playground`, with no server behind
it. A push to `main` that touches the site or a package it depends on runs
`.github/workflows/deploy-playground.yml`, which builds and tests the workspace, the WebAssembly
module included, then uploads `dist/` with `wrangler deploy`. The Worker's routes
(`nxlang.org/playground` and `nxlang.org/playground/*`) are in `wrangler.jsonc`. The day-to-day flow
(deploy, verify, roll back) is in `docs/deployment.md` at the repository root, and the one-time
Cloudflare setup is in `docs/deployment-setup.md`.

**Routing.** A request for a file that exists gets the file. Otherwise `worker/index.mjs` answers:
the shell for `/playground`, `/playground/` and one path segment below it, and not found for
everything else, so a missing asset is a clear 404 rather than HTML where a script was expected.

**Cache headers.** `_headers` makes hashed build output under `assets/` (scripts, styles, the
CanvasKit binary and the NX compiler module) `immutable` for a year, holds fonts and images for a
day, and makes the shell `no-cache`. A deploy changes the hashes and the shell names the new ones,
so old assets can stay cached forever.

**One request at a time, in the visitor's own tab.** The compiler is single-threaded and answers one
request at a time, but that thread is a Web Worker in the visitor's browser: a slow compile costs
that visitor a moment and costs everyone else nothing. A compile that overruns ten seconds has the
worker terminated and replaced; a compile that traps has its host replaced inside the worker. Either
way the editor stays live and the next compile runs.
