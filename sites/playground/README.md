# NX Playground

The public playground for NX, at [nxlang.org/playground](https://nxlang.org/playground): NX source
on the left, what it draws on the right. It opens on a gallery of the DrawnUI React demo pages
ported to NX; any card opens in the editor view and can be edited live. Today it draws with
DrawnUI; the site is shaped so that further targets can join later.

It also runs the whole NX pipeline end to end against a large real catalog: source → NX IR →
evaluated value tree → DrawnUI controls → CanvasKit.

```
NX source ──▶ @nx-lang/sdk-node ──▶ nx-ir-json ──▶ @nx-lang/ir-runtime ──▶ renderer ──▶ DrawnUI
   editor        (server)                             (browser)              (browser)     canvas
     │
     └── hover / completion ──▶ @nx-lang/language-http ──▶ @nx-lang/sdk-node language snapshot
           (@nx-lang/monaco)     (server, /playground/api/language)     (server)
```

## Prerequisites

- **Node 22.18+** — the route tests import TypeScript directly, which Node strips unflagged from 22.18
- **A Rust toolchain** — `@nx-lang/sdk-node` is a native addon and its `.node` binary is gitignored,
  so it is built from source.
- **A checkout of DrawnUI React** at `~/src/DrawnUi.React`, but only to re-run the sync script. The
  vendored copy under `src/drawnui/` is committed, so a normal build needs nothing.

## Running it

The site is a member of the repository's root pnpm workspace, which links the native SDK, the IR
runtime and the shared editor packages it depends on. pnpm comes through corepack.

```bash
# once, from the repository root
corepack enable
pnpm install
pnpm -r build     # the native addon, the IR runtime, the shared packages, then this site

cd sites/playground
pnpm start        # serves the site at http://localhost:5174/playground
```

Everything the site serves lives under `/playground` — the gallery at `/playground`, an example's
editor view at `/playground/<id>`, the API under `/playground/api/`, every asset under the same
prefix — so that the rest of the domain can later be served by something else. `/` redirects to
the gallery; any other address outside the prefix is not found. The prefix is spelled once, in
`base.mjs`, and the server, the Vite config and the client all take it from there.

For development, `pnpm run dev:all` starts both halves: Vite on 5173, serving the site at
`http://localhost:5173/playground/`, and the compile server on 5174, which Vite proxies
`/playground/api` to — compiles and language queries alike. Either one exiting stops the other.
`pnpm run dev` still starts Vite alone, for running it beside a compile server of your own — without
one, the API answers 502 saying so rather than leaving compiles to time out, and hover and
completion fall silent. `PORT` moves the compile server and the proxy that reaches it together.

```bash
pnpm run typecheck      # tsc over the site and the vendored DrawnUI source
pnpm test               # server, route, proxy and watchdog tests, then every example
pnpm run check-examples  # every example compiles, evaluates, and declares its coverage
```

The server tests serve a stand-in `dist/` from a temporary directory (`PLAYGROUND_DIST`), so they
do not need a build to have run.

## Layout

| Path | What it is |
|---|---|
| `base.mjs` | the site's path prefix, and the API and health paths derived from it |
| `catalog/skia.nx` | the generated NX catalog: external components for the DrawnUI control set |
| `catalog/catalog-meta.json` | which types are unions, which are records, which records are constructed |
| `scripts/generate-catalog.mjs` | generates both from the vendored TypeScript |
| `scripts/sync-drawnui.mjs` | re-copies DrawnUI's source, demo pages and assets |
| `scripts/check-examples.mjs` | one check over the whole example set |
| `scripts/emit-example-ir.mjs` | emits each example's NX IR, for proving an edit changed only notation |
| `scripts/dev.mjs` | runs Vite and the compile server together (`pnpm run dev:all`) |
| `server/compile.mjs` | NX source + catalog → NX IR, with diagnostics |
| `server/language.mjs` | the `@nx-lang/language-http` handler with the catalog as its prelude |
| `server/index.mjs` | serves `dist/` under the prefix, the compile, language and health routes, and the root redirect |
| `server/watchdog.mjs` | ends the process if the main thread stops answering |
| `server/port.mjs` | the compile server's port, shared with the Vite proxy and `dev:all` |
| `src/paths.ts`, `src/routes.ts` | the prefix as the client sees it, and the address scheme under it |
| `src/compile/` | the client's one compile seam |
| `src/render/` | evaluated NX values → DrawnUI controls |
| `src/editor/` | the editor view, and Monaco through `@nx-lang/monaco` (grammar, highlighting, hover, completion) and `@nx-lang/language-client` |
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
because NX cannot express the mechanism the original demonstrates). Every non-complete example names
the missing capability from a fixed vocabulary — `event-handlers`, `animation`, `component-state`,
`list-virtualization` — so the gallery can be read as a coverage report on NX rather than a list of
disclaimers.

See `docs/FINDINGS.md` for the toolchain gaps this site ran into, and `docs/CATALOG.md` for where the
catalog diverges from the DrawnUI object model.

## Deploying

One image, one process: the build stage compiles the native addon and bundles the SPA, and the
runtime stage serves both. Build it from the **repository root**, since the image needs the native
binding and the IR runtime alongside the site:

```bash
docker build -f sites/playground/Dockerfile -t nx-playground .
docker run -p 8080:8080 nx-playground        # http://localhost:8080/playground
```

`PORT` selects the port (8080 in the image). Nothing else is required at runtime — the catalog, the
examples and the grammar are all in the bundle.

The public site runs on Railway behind Cloudflare. A push to `main` that touches the site or
something the image copies runs `.github/workflows/deploy-playground.yml`, which builds and tests
the workspace and then uploads it with `railway up`; Railway builds this Dockerfile from the
repository root as declared in `.railway/railway.ts` — the Dockerfile path, the health check and
the restart policy come from there, and `railway config apply` puts a change to it into effect —
and Cloudflare proxies `nxlang.org`, terminates TLS, rate limits the API path and caches assets by
the headers the server sends. The day-to-day
flow (deploy, verify, roll back) is in `docs/deployment.md` at the repository root; the one-time
Railway and Cloudflare setup, with every rule and its value, is in `docs/deployment-setup.md`.

**Health and the watchdog.** `GET /playground/api/health` answers `{ "ok": true }` from the same
thread that runs compiles, so a stuck process cannot report itself healthy; Railway polls it before
switching traffic to a new deployment. Railway does not poll it afterwards, so the process watches
itself: `server/watchdog.mjs` runs a worker thread that kills the process if the main thread has
not heartbeated for twenty seconds (on the monotonic clock, so a clock step cannot look like a
hang), and Railway's restart policy starts a fresh one.

**Cache headers.** Hashed build output under `assets/` (scripts, styles, the CanvasKit binary) is
`immutable` for a year; fonts and images are held for a day; the shell is `no-cache`; API answers
are `no-store`. A deploy changes the hashes and the shell names the new ones, so old assets can stay
cached forever.

**One request at a time.** `POST /playground/api/compile` and `POST /playground/api/language/*` call
the native binding synchronously on the Node server's only thread. A slow call stops the service for
everyone until it finishes, and nothing in the process can interrupt it: a worker thread would not
help, because `terminate()` cannot preempt a native call that never returns to JavaScript. Only a
child process can be killed. No known input hangs the compiler (the one that did is fixed and
fuzz-tested), and the language route caches its analysis per document set so a hover storm costs one
analysis. The watchdog bounds how long a hang lasts and Cloudflare's rate limit bounds how often one
can be provoked; running the compiler in a child process, so that one bad request costs one request
rather than a restart, is the next step and is noted in `specs/future.md`.
