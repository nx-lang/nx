# NX × DrawnUI — a fiddle

NX source on the left, what it draws on the right. The app opens on a gallery of the DrawnUI React
demo pages, ported to NX; any card opens in the fiddle and can be edited live.

It exists to run the whole NX pipeline end to end against a large real catalog: source → NX IR →
evaluated value tree → DrawnUI controls → CanvasKit.

```
NX source ──▶ @nx-lang/sdk-node ──▶ nx-ir-json ──▶ @nx-lang/ir-runtime ──▶ renderer ──▶ DrawnUI
   editor        (server)                             (browser)              (browser)     canvas
```

## Prerequisites

- **Node 22+**
- **A Rust toolchain** — `@nx-lang/sdk-node` is a native addon and its `.node` binary is gitignored,
  so it is built from source.
- **A checkout of DrawnUI React** at `~/src/DrawnUi.React`, but only to re-run the sync script. The
  vendored copy under `src/drawnui/` is committed, so a normal build needs nothing.

## Running it

```bash
# once, from the repository root
cd bindings/node && npm install && npm run build

cd ../../sample-apps/drawnui-react
npm install
npm run build     # bundles the SPA into dist/
npm start         # serves dist/ and POST /api/compile on http://localhost:5174
```

For development, `npm run dev:all` starts both halves: Vite on 5173 and the compile server on 5174,
which Vite proxies `/api` to. Either one exiting stops the other. `npm run dev` still starts Vite
alone, for running it beside a compile server of your own — without one, `/api` answers 502 saying
so rather than leaving compiles to time out. `PORT` moves the compile server and the proxy that
reaches it together.

```bash
npm run typecheck      # tsc over the app and the vendored DrawnUI source
npm test               # compile-service tests, then every example
npm run check-examples  # every example compiles, evaluates, and declares its coverage
```

## Layout

| Path | What it is |
|---|---|
| `catalog/skia.nx` | the generated NX catalog: external components for the DrawnUI control set |
| `catalog/catalog-meta.json` | which types are unions, which are records, which records are constructed |
| `scripts/generate-catalog.mjs` | generates both from the vendored TypeScript |
| `scripts/sync-drawnui.mjs` | re-copies DrawnUI's source, demo pages and assets |
| `scripts/check-examples.mjs` | one check over the whole example set |
| `scripts/emit-example-ir.mjs` | emits each example's NX IR, for proving an edit changed only notation |
| `scripts/dev.mjs` | runs Vite and the compile server together (`npm run dev:all`) |
| `server/compile.mjs` | NX source + catalog → NX IR, with diagnostics |
| `server/index.mjs` | serves `dist/` and `POST /api/compile` |
| `server/port.mjs` | the compile server's port, shared with the Vite proxy and `dev:all` |
| `src/compile/` | the client's one compile seam |
| `src/render/` | evaluated NX values → DrawnUI controls |
| `src/editor/` | Monaco with the repository's own NX TextMate grammar |
| `src/examples/` | the ported examples and their metadata |
| `src/drawnui/` | vendored DrawnUI runtime — see `UPSTREAM.md` |
| `reference/demo-pages/` | the original TSX pages, for comparison only; never built |

## Syncing DrawnUI

```bash
npm run sync-drawnui              # from ~/src/DrawnUi.React
npm run sync-drawnui -- --source /path/to/DrawnUi.React
npm run generate-catalog          # regenerate the catalog after a sync
```

The sync records the upstream commit in `src/drawnui/UPSTREAM.md`. The catalog is committed, so a
sync that changes it shows up as a reviewable diff. Local edits to the vendored copy are allowed
where they improve NX compatibility; `docs/CATALOG.md` is where they are recorded, because a sync
overwrites them.

## What it does not do

**Authored interaction is not supported.** The TypeScript IR runtime has no action dispatch, so
`Tapped`, `Toggled` and the rest are not in the catalog and authored NX renders statically. DrawnUI's
own behavior still works: scroll regions scroll, carousels swipe, drawers drag, ripples play,
switches toggle, sliders drag. What is missing is anything that would have to run authored NX in
response — counters, readouts, navigation, animation.

All twelve DrawnUI demo pages are ported — none is omitted — and each says where it stands:
**complete** (no note), **static** (drawn correctly, nothing responds), or **reduced** (scaled down,
because NX cannot express the mechanism the original demonstrates). Every non-complete example names the missing capability from a fixed vocabulary —
`event-handlers`, `animation`, `component-state`, `list-virtualization` — so the gallery can be read
as a coverage report on NX rather than a list of disclaimers.

See `docs/FINDINGS.md` for the toolchain gaps this app ran into, and `docs/CATALOG.md` for where the
catalog diverges from the DrawnUI object model.

## Deploying

One image, one process: the build stage compiles the native addon and bundles the SPA, and the
runtime stage serves both. Build it from the **repository root**, since the image needs the native
binding and the IR runtime alongside the app:

```bash
docker build -f sample-apps/drawnui-react/Dockerfile -t drawnui-fiddle .
docker run -p 8080:8080 drawnui-fiddle
```

`PORT` selects the port (8080 in the image). Nothing else is required at runtime — the catalog, the
examples and the grammar are all in the bundle.

**One compile at a time, with no deadline.** `POST /api/compile` calls the native compiler
synchronously on the Node server's only thread. A compile that never returns — or merely a slow one
— stops the service for everyone until it finishes, and nothing in the process can interrupt it: a
worker thread would not help, because `terminate()` cannot preempt a native call that never returns
to JavaScript. Only a child process can be killed. No known input hangs the compiler (the one that
did is fixed and fuzz-tested), so this is a structural exposure rather than a live one, but it is
the reason to put this behind something that limits request rate and body size before pointing
untrusted traffic at it.
