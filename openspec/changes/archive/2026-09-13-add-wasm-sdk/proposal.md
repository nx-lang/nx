## Why

The playground compiles NX and answers hover and completion on a Railway server because no
WebAssembly build of the compiler or language service exists. That server is the site's only moving
part: every compile and language call runs synchronously on its one thread, a hang costs every
visitor a restart, the edge needs a rate limit to protect it, and the deployment needs a watchdog,
a health check tied to the compile thread and a Rust-capable Docker build. The Rust core is already
wasm-clean: a spike built `nx-ffi` and `nx-language-service` for `wasm32-wasip1` with nothing but a
WASI sysroot, and compiled all twenty playground examples against the catalog in under 100 ms each
from a 2.6 MB module (826 KB gzipped). Moving the tooling into the browser removes the server's
reason to exist, makes every compile local and instant, and gives the language a browser SDK that
other sites can use.

## What Changes

- Add a `bindings/wasm` Rust crate that builds the NX compiler, NX IR codegen and language service
  as a `wasm32-wasip1` module with a small JSON-in, JSON-out C ABI, and the build tooling (clang, the
  pinned WASI sysroot, the rustup target) that produces it reproducibly on a developer machine, in
  CI and in the playground's Docker build.
- Add an `@nx-lang/sdk-wasm` TypeScript package that loads that module in a browser or in Node,
  supplies the WASI imports it needs, and exposes the subset of the Node SDK's API the playground
  uses: build a program artifact from source, emit NX IR, and answer language queries over in-memory
  documents. A trapped module is reported as a crashed host and can be replaced by loading a fresh
  one; the SDK never hides a trap behind a stale instance.
- Lift the transport-free half of `@nx-lang/language-http` (query answering, prelude shifting,
  version echo, the snapshot cache) into a shared package both the HTTP handler and an in-process,
  worker-hosted implementation of `NxLanguageService` use, so the browser and the server cannot
  disagree about coordinates.
- Switch the playground to compile and answer language queries in a Web Worker in the browser,
  through the existing `Compile` seam and `NxLanguageService` interface. The catalog ships as part
  of the client bundle. A compile that traps or overruns its deadline costs that one request: the
  worker is terminated and replaced, and the editor stays live.
- **BREAKING** for the playground's deployment: the compile and language routes, the watchdog, the
  Node SDK dependency and the native addon build leave the server. It serves the built client and
  its health route only. The Railway service, the Dockerfile and the deploy workflow stay, now
  building the wasm module instead of the napi addon. Moving to a static host is a follow-up.
- Update the Node SDK's documentation so that browser and wasm hosts are directed to the new package
  rather than described as unsupported.

Out of scope, recorded for the follow-ups list: shrinking the NX IR JSON (pretty-printed, and
carrying the whole flattened catalog on every compile), turning the catalog into a library artifact
loaded once, library registries in the browser, and retiring the Node server for a static host.

## Capabilities

### New Capabilities
- `sdk-wasm`: the `bindings/wasm` crate and the `@nx-lang/sdk-wasm` package: how the module is
  built, what its ABI guarantees, how a host loads and replaces it, the program-artifact and NX IR
  API, the in-memory language snapshot, the in-process `NxLanguageService` implementation with a
  prelude, and how a trap is surfaced.

### Modified Capabilities
- `playground`: compilation and language queries run in the browser rather than on the server, so
  the requirements that compilation runs on the server, that the service reports health from the
  compile thread, that a stuck service ends itself, that one service serves compilation, and that
  the edge rate-limits the API change or go; the API prefix carries only the health route; a crashed
  or overrunning compile is contained to one request.
- `sdk-node`: the package's documentation directs browser and wasm consumers to `@nx-lang/sdk-wasm`
  instead of stating that no such runtime exists.

## Impact

- **New code**: `bindings/wasm/` (Rust crate `nx-sdk-wasm-native` and the `@nx-lang/sdk-wasm`
  package), `packages/language-core/` (the shared query-answering logic), a sysroot-fetching script
  under `scripts/`, and a Web Worker plus in-browser `Compile` and language service implementations
  under `sites/playground/src/`.
- **Changed code**: `packages/language-http` becomes a thin HTTP and Node adapter over the shared
  core; `sites/playground/server/` loses `compile.mjs`, `language.mjs` and `watchdog.mjs` and their
  tests; `scripts/check-examples.mjs` compiles through the wasm SDK so what is tested is what ships;
  `vite.config.ts` loses the API proxy and probe; the Dockerfile's build stage builds the wasm
  module; `.github/workflows/build.yml` and `deploy-playground.yml` install the wasm toolchain;
  `.railway/railway.ts` keeps its health check but the restart policy no longer serves a watchdog.
- **Toolchain**: `wasm32-wasip1` rustup target, clang, and the wasi-sdk 25 sysroot, pinned by
  version and fetched on demand into a gitignored cache. No emscripten, no wasm-bindgen.
- **Specs and docs**: `openspec/specs/sdk-wasm` is new; `playground` and `sdk-node` get deltas;
  `specs/future.md`'s playground section, `docs/deployment-setup.md`, the playground README and the
  Node SDK README are updated. The Cloudflare rate-limit rule on `/playground/api/*` becomes
  unnecessary and is removed from the documented setup.
- **Dependencies**: the playground drops `@nx-lang/sdk-node` and `@nx-lang/language-http`; it gains
  `@nx-lang/sdk-wasm` and `@nx-lang/language-core`. `@nx-lang/sdk-wasm` depends on
  `@bjorn3/browser_wasi_shim` for the browser and on `node:wasi` in Node.
