# NX Playground

The public playground for NX, at [nxlang.org/play](https://nxlang.org/play): NX source on the left,
and on the right what its `root` returns. NX has no `print`, so the value a program evaluates to is
its output, shown as NX text — the text `nxlang run` prints — by the `<nx-value>` element of
[`@nx-lang/value-view`](../../packages/value-view/README.md). Long values fold, and hovering a type
name, property or case in the output shows the same hover its declaration shows in the source.
Clicking one selects the declaration.

Every part of that runs in the visitor's browser. The compiler and the language service are one
WebAssembly module, loaded into a Web Worker when the page mounts; nothing is compiled on a server.

```
NX source ──▶ @nx-lang/sdk-wasm ──▶ root's value as annotated NX text ──▶ @nx-lang/value-view
   editor        (worker)                                                   (output pane)
     │
     └── hover / completion ──▶ @nx-lang/language-core ──▶ sdk-wasm language snapshot
           (@nx-lang/monaco)          (worker)                    (worker)
```

One worker, one host: evaluating and answering hover share an instance, so a diagnostic and a hover
range land on the same line of the visitor's text. A request that traps or overruns its deadline
costs that one request — the worker is replaced and the editor stays live. Runaway recursion and
endless loops end sooner, at the interpreter's own limits, as runtime errors. In the browser, calls
nest at most 200 deep, a limit set below what a browser worker's stack can hold.

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
pnpm test                # Worker, route, share, evaluate, worker and dev-shell tests, then every example
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
leaves. The check evaluates through the same module and the same `evaluateSource` the browser uses,
so what is checked is what ships, and it keeps the set between 10 and 20 examples.

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
| `src/compile/` | the evaluate seam: NX source → diagnostics and root's value, with the site's input and output limits |
| `src/worker/` | the compiler worker: its module load, its session, and the main thread's channel to it |
| `src/language/` | the language service the editor and the output's hover ask |
| `src/editor/` | Monaco through `@nx-lang/monaco` (grammar, highlighting, hover, completion) |
| `src/output/` | the output pane, `<nx-value>` or why there is nothing to show |
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
