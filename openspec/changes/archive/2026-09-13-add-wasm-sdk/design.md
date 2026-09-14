## Context

See proposal.md — Why. What the spike established, and what this design builds on:

- The Rust core is wasm-clean except for tree-sitter's C runtime and the grammar's `parser.c` and
  `scanner.c`, which include libc headers. Given a WASI sysroot on the clang include path, `nx-ffi`
  and `nx-language-service` build for `wasm32-wasip1` unchanged. The resulting module imports 14
  WASI preview 1 functions and nothing else, and Node's `node:wasi` runs it.
- `nx-api` still references `std::fs` for file-backed fallbacks; under WASI those calls get
  `ENOSYS` from the shim and behave as "not a file", which the spike exercised by compiling every
  example. The language service parses `nx://` URIs without touching a filesystem.
- wasm targets are `panic=abort`. The `catch_unwind` in `nx-ffi` is inert there: a panic or an
  out-of-bounds access traps and the instance is unusable afterwards. Instantiating a fresh one from
  a compiled module takes 12 to 21 ms.
- The playground already has the two seams this change needs: the `Compile` type in
  `sites/playground/src/compile/types.ts` and the `NxLanguageService` interface from
  `@nx-lang/language-protocol`, consumed by `@nx-lang/monaco`. The server's compile route and the
  language handler share the prelude arithmetic in `@nx-lang/language-http`, and that arithmetic is
  what keeps diagnostics and hover on the same visitor line.
- `sites/playground/scripts/check-examples.mjs` compiles every example through `server/compile.mjs`,
  so today's tests exercise the server path; after this change they must exercise the browser path.
- Hosting is Railway behind Cloudflare, deployed by `railway up` from GitHub Actions; the Dockerfile
  builds from the repository root. The Cloudflare rate limit on `/playground/api/*` exists only
  because the server compiled on one thread.

## Goals / Non-Goals

**Goals:**
- One wasm module, one loader, used by the browser worker, by Node tests and by the example check,
  so what is tested is what ships.
- No coordinate logic exists twice: the HTTP handler and the in-browser service share the code that
  shifts positions across the prelude.
- A trap or a hang in the compiler is contained to the request that caused it, and recovery needs
  no page reload.
- The wasm build is reproducible from a clean checkout on a developer machine, in CI and in the
  Docker build, with every toolchain component pinned.

**Non-Goals:**
- wasm-bindgen, wasm-pack or emscripten. The ABI is JSON strings in and out; a hand-written loader is
  a few hundred lines and has no build-time dependency on a bindgen version.
- Threads inside the module, streaming or incremental analysis, or a persistent analysis across
  compiles. Each compile builds the catalog and the source together, as the server does today.
- Shrinking the NX IR JSON, or turning the catalog into a library artifact. Both are recorded as
  follow-ups; neither changes the seams this design uses.
- Retiring the Node server for a static host. The server keeps serving files and health; nothing in
  this design depends on it doing more, so the move is an independent change.
- Library registries, workspaces, evaluation to values, or component lifecycle calls through the
  wasm SDK. The playground needs build-from-source, NX IR and the language snapshot; the rest can be
  added behind the same ABI when a consumer needs it.

## Decisions

### D1. Target `wasm32-wasip1`, with a pinned WASI sysroot fetched on demand
The module targets `wasm32-wasip1`. Rust's standard library for that target bundles wasi-libc, so
tree-sitter's libc references resolve at link time with no shims, and the C sources compile with
`clang --target=wasm32-wasip1 --sysroot=<wasi-sysroot>`. The sysroot is the `wasi-sysroot` tarball
of a pinned wasi-sdk release (25.0 today), fetched by `scripts/fetch-wasi-sysroot.mjs` into a
gitignored cache directory under the repository when it is not already there, and verified by
checksum. The package's build script sets `CC_wasm32_wasip1=clang` and
`CFLAGS_wasm32_wasip1=--sysroot=<cache>` when it invokes cargo, so a developer runs `pnpm -r build`
and nothing else; the same script serves CI and the Dockerfile. The rustup target is added by the
setup action and the Dockerfile; `rust-toolchain.toml` lists it as a target so `rustup` installs it
with the toolchain.

*Alternative*: `wasm32-unknown-unknown`. It also links once the headers are found, but the module
then imports fifteen raw `env` symbols (`malloc`, `free`, `fprintf`, `clock_gettime`, ...) that a
loader must implement by hand, with the allocator shims routed into Rust's heap. More work for no
gain. *Alternative*: emscripten, as tree-sitter's own web binding uses. A second toolchain to pin,
and a JavaScript runtime layer far larger than a WASI shim.

### D2. A dedicated crate with a JSON ABI, not `nx-ffi`
`bindings/wasm/native` is a `cdylib` crate, `nx-sdk-wasm-native`, over `nx-api`, `nx-codegen` and
`nx-language-service`, shaped like `bindings/node/native`. Its ABI:

- `nx_wasm_abi_version() -> u32`, checked by the loader before any other call.
- `nx_wasm_alloc(len) -> ptr` and `nx_wasm_free(ptr, len)`: the loader allocates every input in the
  module's memory through these and never grows memory itself.
- Every operation returns a pointer to a result record `{ status: u32, ptr: u32, len: u32 }` whose
  payload is UTF-8 JSON: a value on success, the SDK's diagnostics array or an error message
  otherwise. `nx_wasm_result_free(ptr)` releases the record and its payload.
- Operations: `nx_wasm_program_build(source, file_name) -> handle`, `nx_wasm_program_nx_ir(handle)`,
  `nx_wasm_program_free(handle)`, `nx_wasm_snapshot_new(documents_json) -> handle`, and
  `nx_wasm_snapshot_{hover,completions,diagnostics,document_symbols}` plus `nx_wasm_snapshot_free`.
  Handles are `Box::into_raw` pointers, opaque to the loader.

No `catch_unwind`: it cannot work on this target, and pretending otherwise would hide the fact that
a trap ends the instance (D4). The JSON payload shapes for NX IR and diagnostics are the ones the
Node binding serializes, so the parity tests can compare text directly.

*Alternative*: build `nx-ffi` itself, as the spike did. It works, but its by-value struct ABI,
msgpack default, base64 and out-parameter conventions are P/Invoke-shaped, and it has no language
service exports. Adding those there would grow the .NET ABI surface for the browser's sake.

### D3. `@nx-lang/sdk-wasm` loads the module and mirrors the Node SDK's API subset
The package at `bindings/wasm` exports:

- `compileNxModule(source)`: from bytes, a `Response`, or an existing `WebAssembly.Module`, checks
  nothing yet; compilation is what the browser caches.
- `createNxHost(module)`: instantiates with a WASI provider, checks the ABI version, and returns an
  `NxHost` whose methods are `buildProgramArtifact(source, { fileName })` returning an
  `NxProgramArtifact` with `generateNxIr()` and `dispose()`, and `createLanguageSnapshot(documents)`
  returning an `NxLanguageSnapshot` with the four queries and `dispose()`. Names, option shapes,
  result shapes and error classes (`NxEvaluationError`, `NxDisposedResourceError`) follow
  `@nx-lang/sdk-node` so a consumer can move between them.
- Two entry points selected by package `exports` conditions: the default uses
  `@bjorn3/browser_wasi_shim`; the `node` condition uses `node:wasi`. Both provide the same
  `createNxHost`.

Strings cross the boundary through `TextEncoder`/`TextDecoder` into buffers from `nx_wasm_alloc`;
results are read through a `DataView`, copied out, and released before the call returns.

*Alternative*: publish TypeScript types from the Rust side with wasm-bindgen. Rejected under
Non-Goals; the API surface here is six methods.

### D4. A trap marks the host crashed; the caller replaces it
Every call into the module is wrapped. A `WebAssembly.RuntimeError`, or the WASI shim's exit
exception raised by Rust's abort, is caught once, the host is flagged, and the call throws
`NxHostCrashedError` naming the operation. Every later call on that host throws the same error
without entering the module. The caller keeps the compiled `WebAssembly.Module` and calls
`createNxHost` again. Handles from the dead host are invalid by construction, since they pointed
into memory that no longer exists; the wrapper objects report themselves disposed.

### D5. The transport-free half of `language-http` moves to `@nx-lang/language-core`
A new package `packages/language-core` holds what is Node-independent today: the prelude offsets
and shifts, the query dispatcher that applies them and echoes versions, the snapshot cache, and a
`createSnapshotLanguageService({ createSnapshot, prelude, cacheSize })` that implements
`NxLanguageService` in-process over any `SnapshotLike` factory. `@nx-lang/language-http` keeps
request parsing, body limits, error responses and the Node listener adapter, and calls the core.
`@nx-lang/sdk-wasm` re-exports a `createLanguageService(host, options)` that binds the core to the
host's snapshots, which is the implementation the sdk-wasm spec requires.

The cache key stops being a SHA-256 over the documents, which needs `node:crypto`, and becomes the
document set's own content joined with separators. The cache holds eight entries of a few tens of
kilobytes each; hashing bought nothing but a dependency.

*Alternative*: put the logic into `@nx-lang/language-protocol`. That package is types and two
predicates, and every client depends on it; the core would drag a service implementation into the
Monaco bundle of hosts that only speak HTTP.

### D6. The playground runs the host in one module Web Worker
`sites/playground/src/worker/nx.worker.ts` imports the module URL with `?url` (a hashed asset, like
CanvasKit) and the catalog with `?raw`, compiles the module with `WebAssembly.compileStreaming`,
creates a host, and answers two message kinds: `compile { source }` and `language { query, request }`.
On the main thread, `src/compile/worker.ts` implements `Compile` and `src/language/worker.ts`
implements `NxLanguageService`, over one shared channel that correlates requests by id, turns an
`AbortSignal` into a cancel message whose late answer is dropped, and enforces a deadline. When a
request overruns the deadline the channel terminates the worker, rejects everything in flight with a
timeout error, and starts a new worker on the next request. That deadline measures the worker's own
work, not the wait for its module: the worker posts a readiness notice once the module is compiled,
and a request sent before that waits without a timer, because terminating a worker mid-download
discards the partial fetch and starts the same slow one over. The load carries its own budget
instead — two minutes, far above any real download — so a connection that stalls rather than fails
ends as a module that would not load rather than as a wait with no end. When the host traps inside the worker,
the worker answers that request with the crash, replaces its host from the module it already
compiled, and keeps going. A failure a fresh worker can answer — a deadline, a worker that stopped,
a trapped host — is retried once by the compile seam, since a gallery preview compiles once and
would otherwise carry the failure until the page is reloaded.

The worker starts when the editor view mounts, not on the gallery, so the module's 826 KB does not
precede the first paint of a page that never compiles.

*Alternative*: run the host on the main thread. Compiles take 40 to 100 ms for the catalog plus an
example, enough to drop frames while typing, and a hang could only be ended by reloading. The
worker also gives the one thing the native server never had: a preemptible cancel.

### D7. Catalog handling lives in one browser-safe module, used by the site and by its checks
`sites/playground/src/compile/catalog.ts` exports `compileWithCatalog(host, catalog, source)`: it
prepends the catalog through the core's prelude offsets, builds, emits IR, and classifies each
diagnostic as `source`, `catalog` or `program` with the byte and line shifts `server/compile.mjs`
applies today. The worker calls it; so does `scripts/check-examples.mjs`, under Node with the
package's `node` entry, so the example check compiles through the shipped compiler and catalog path.
`server/compile.mjs`, `server/language.mjs`, `server/watchdog.mjs` and their tests are deleted.

### D8. The server keeps static files and health; the edge keeps TLS and cache
`server/index.mjs` loses the compile and language routes and the watchdog. It still redirects `/`,
answers not-found outside the prefix, serves `dist/` with the same cache headers, and answers
`/playground/api/health` as the deploy gate. `vite.config.ts` drops the API proxy and the probe;
`pnpm run dev` alone runs the site, and `dev:all` and `server/port.mjs` go. The Railway service's
health check stays; the restart policy stays `ALWAYS` because nothing is gained by changing it. The
Cloudflare rate-limit rule is removed from the documented setup and from the zone, since no request
under `/playground/api/` costs the origin anything now.

### D9. The build pipeline produces the module in three places from one script
`pnpm -r build` builds `@nx-lang/sdk-wasm` before the playground, running cargo for the wasm target
with a dedicated `wasm-release` profile that inherits `release` and strips symbols, then copies the
module into the package's `dist/`. The CI Rust job and the deploy workflow's validate job get the
target through `rust-toolchain.toml` and clang from the runner image; the Dockerfile's build stage
installs clang, and the `pnpm -r build` it already runs fetches the sysroot and builds the module.
The runtime image no longer needs the napi addon or the crates, so the post-build cleanup shrinks to
removing the Rust sources and target directory.

## Risks / Trade-offs

- [The module is 2.6 MB, 826 KB gzipped, fetched on the first editor view] → served as a hashed,
  immutable asset from the edge, compiled by the browser as it streams, and started when the editor
  mounts rather than on the gallery. Symbol stripping in the `wasm-release` profile trims the name
  section; `wasm-opt` is a possible later win and is not a dependency.
- [Node's WASI module is marked experimental and prints a warning] → the package's tests and the
  example check run with `--no-warnings`; the API surface used, `WASI` with `version: "preview1"`, has
  been stable in practice for years and nothing else in the loader depends on Node internals.
- [Memory inside the worker never shrinks; each compile produces a multi-megabyte IR string] → the
  allocator reuses freed memory, so steady state is tens of megabytes; the worker is replaced on any
  overrun, and can later be recycled after N compiles if measurements say so.
- [salsa's locks cannot park on wasm] → the module is single-threaded, so no lock is ever contended.
  Adding threads inside the module would require revisiting this; it is not planned.
- [clang and sysroot drift] → clang comes from the CI runner image and the Docker base image, both
  named in the Dockerfile and workflows; the sysroot version and checksum live in the fetch script.
  A version bump is one edit and a green build.
- [A trap is a lost request, not a recovered one] → by design; the alternative is a runtime that
  unwinds, which the target does not offer. The failure is reported in the diagnostics pane and the
  next compile works.
- [Three bindings now serialize the NX IR payload and diagnostics the same way by convention] →
  the wasm parity tests compare the wasm SDK's output with the Node SDK's byte for byte, so drift
  fails the build.

## Migration Plan

1. Land the crate, the packages and the playground change together; `pnpm -r build` and
   `pnpm -r test` prove parity and the example check before the deploy workflow runs.
2. Deploy as any other push to `main`. The new image serves the module and a server without API
   routes; the health check gates the switch as before. Clients holding the old shell keep working
   until they reload, since the old server keeps answering until the new deployment is live and the
   old client never asked for the module.
3. Remove the Cloudflare rate-limit rule after the deploy is live. Leaving it in place is harmless.
4. Rollback is Railway's: redeploy the previous deployment. Nothing outside the service changed
   shape.

### D10. The package is private, like the Node SDK
`@nx-lang/sdk-node` is `private`; `@nx-lang/sdk-wasm` starts the same way. It is built and consumed
from the workspace, and publication can follow whenever the Node SDK's does, with the same
package-verification scripts.
