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
- **A checkout of DrawnUI React** at `~/src/DrawnUi.React`, but only to re-run the sync script. The
  vendored copy under `src/drawnui/` is committed, so a normal build needs nothing.

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
compiler is in the page. `pnpm start` serves a built `dist/` the way production does, on 8080.

Everything the site serves lives under `/playground` — the gallery at `/playground`, an example's
editor view at `/playground/<id>`, the health route under `/playground/api/`, every asset under the
same prefix — so that the rest of the domain can later be served by something else. `/` redirects to
the gallery; any other address outside the prefix is not found. The prefix is spelled once, in
`base.mjs`, and the server, the Vite config and the client all take it from there.

```bash
pnpm run typecheck       # tsc over the site and the vendored DrawnUI source
pnpm test                # server, route, compile, worker and dev-shell tests, then every example
pnpm run check-examples  # every example compiles, evaluates, and declares its coverage
```

The example check compiles through the same module and the same catalog path the browser uses, and
links each example against the catalog artifact emitted the way the build emits it, so what is
checked is what ships.

The server tests serve a stand-in `dist/` from a temporary directory (`PLAYGROUND_DIST`), so they
do not need a build to have run.

## Layout

| Path | What it is |
|---|---|
| `base.mjs` | the site's path prefix, and the API and health paths derived from it |
| `catalog/skia.nx` | the generated NX catalog: external components for the DrawnUI control set, compiled to its artifact at build time |
| `catalog/catalog-meta.json` | which types are unions, which are records, which records are constructed |
| `scripts/generate-catalog.mjs` | generates both from the vendored TypeScript |
| `scripts/sync-drawnui.mjs` | re-copies DrawnUI's source, demo pages and assets |
| `scripts/check-examples.mjs` | one check over the whole example set |
| `scripts/emit-example-ir.mjs` | emits each example's NX IR, for proving an edit changed only notation |
| `scripts/compile-example.mjs` | the wasm host and catalog those two scripts compile through |
| `server/index.mjs` | serves `dist/` under the prefix, the health route, and the root redirect |
| `src/paths.ts`, `src/routes.ts` | the prefix as the client sees it, and the address scheme under it |
| `src/compile/` | the compile seam, NX source + catalog → the visitor's NX IR with diagnostics, and the catalog's own artifact |
| `src/worker/` | the compiler worker: its module load, its session, and the main thread's channel to it |
| `src/language/` | the language service the Monaco integration is registered with |
| `src/render/` | the catalog prepared once, each compile linked against it, and evaluated NX values → DrawnUI controls |
| `src/editor/` | the editor view, and Monaco through `@nx-lang/monaco` (grammar, highlighting, hover, completion) over `src/language/` |
| `src/gallery/` | the gallery |
| `src/examples/` | the ported examples and their metadata |
| `src/drawnui/` | vendored DrawnUI runtime — see `UPSTREAM.md` |
| `reference/demo-pages/` | the original TSX pages, for comparison only; never built |

## Syncing DrawnUI

```bash
npm run sync-drawnui              # from ~/src/DrawnUi.React
npm run sync-drawnui -- --source /path/to/DrawnUi.React
npm run generate-catalog          # regenerate the catalog after a sync
```

The sync copies upstream's `src` whole, the demo pages for reference, and the asset trees the
examples read: `fonts/`, `images/`, and — since the preview.4 sync — `lottie/` (Skottie
animations), `anims/` (sprite sheets) and `shaders/` (SkSL, including the `transitions/` set the
shader carousel uses), each landing under `public/` by the same name. It records the upstream
commit in `src/drawnui/UPSTREAM.md`. The catalog is committed, so a sync that changes it shows up
as a reviewable diff. Local edits to the vendored copy are allowed where they improve NX
compatibility; `docs/CATALOG.md` is where they are recorded, because a sync overwrites them. The
vendored Vite plugin (`src/drawnui/vite/`, upstream's build-time crawler) imports an optional peer
the site does not install, so `tsconfig.json` excludes it from the type check rather than the sync
pruning it.

The vendored runtime loads CanvasKit's "full" build, roughly 0.9 MB more of WASM than the default
one, because Lottie playback (Skottie) lives only there. The binary is a hashed build asset and is
cached like the previous one.

## What it does not do

**Authored interaction is not supported.** The TypeScript IR runtime has no action dispatch, so
`Tapped`, `Toggled` and the rest are not in the catalog and authored NX renders statically. DrawnUI's
own behavior still works: scroll regions scroll, carousels swipe, drawers drag, ripples play,
switches toggle, sliders drag. What is missing is anything that would have to run authored NX in
response — counters, readouts, navigation, animation.

All twenty DrawnUI demo pages at the vendored commit are ported — none is omitted — and each says
where it stands: **complete** (no note), **static** (drawn correctly, nothing responds), or
**reduced** (scaled down, because NX cannot express the mechanism the original demonstrates). Only
SVG is complete today; the rest gained interaction or code-driven mechanisms upstream and say so.
Every non-complete example names the missing capability from a fixed vocabulary —
`event-handlers`, `animation`, `component-state`, `list-virtualization`, `code-behind` (an engine
object built or driven from code: a shader effect, a CanvasKit filter, a sprite set, a cell class
with drag logic) — so the gallery can be read as a coverage report on NX rather than a list of
disclaimers.

See `docs/FINDINGS.md` for the toolchain gaps this site ran into, and `docs/CATALOG.md` for where the
catalog diverges from the DrawnUI object model.

## Deploying

One image, one process: the build stage compiles the WebAssembly module and bundles the SPA, and the
runtime stage serves the bundle. Build it from the **repository root**, since the image needs the
crates, the wasm SDK and the IR runtime alongside the site:

```bash
docker build -f sites/playground/Dockerfile -t nx-playground .
docker run -p 8080:8080 nx-playground        # http://localhost:8080/playground
```

`PORT` selects the port (8080 in the image). Nothing else is required at runtime — the compiler, the
catalog, the examples and the grammar are all in the bundle.

The public site runs on Railway behind Cloudflare. A push to `main` that touches the site or
something the image copies runs `.github/workflows/deploy-playground.yml`, which builds and tests
the workspace and then uploads it with `railway up`; Railway builds this Dockerfile from the
repository root as declared in `.railway/railway.ts` — the Dockerfile path, the health check and
the restart policy come from there, and `railway config apply` puts a change to it into effect —
and Cloudflare proxies `nxlang.org`, terminates TLS and caches assets by the headers the server
sends. The day-to-day flow (deploy, verify, roll back) is in `docs/deployment.md` at the repository
root; the one-time Railway and Cloudflare setup, with every rule and its value, is in
`docs/deployment-setup.md`.

**Health.** `GET /playground/api/health` answers `{ "ok": true }`, and Railway polls it before
switching traffic to a new deployment. Nothing a visitor does reaches this process's event loop, so
there is no longer a way for it to be alive and unable to answer, and nothing for a watchdog to do.

**Cache headers.** Hashed build output under `assets/` (scripts, styles, the CanvasKit binary and
the NX compiler module) is `immutable` for a year; fonts and images are held for a day; the shell is
`no-cache`. A deploy changes the hashes and the shell names the new ones, so old assets can stay
cached forever.

**One request at a time, in the visitor's own tab.** The compiler is single-threaded and answers one
request at a time, but that thread is a Web Worker in the visitor's browser: a slow compile costs
that visitor a moment and costs everyone else nothing. A compile that overruns ten seconds has the
worker terminated and replaced; a compile that traps has its host replaced inside the worker. Either
way the editor stays live and the next compile runs.
