# @nx-lang/sdk-wasm

Browser and Node access to the NX compiler, NX IR generation, and the language service, through one
WebAssembly module. No server, no native addon.

This package is separate from `@nx-lang/ir-runtime` under `runtime/typescript`: use `@nx-lang/sdk-wasm`
when JavaScript needs to *compile* NX source or answer editor queries over it, and the IR runtime when
JavaScript only needs to *execute* an already persisted NX IR JSON document. Filesystem-backed
workflows — library registries loaded from disk, workspace directories, `root()` evaluation to values —
belong to [`@nx-lang/sdk-node`](../node/README.md); this package sees only the source text it is handed.

## Scope

| Provided                                          | Not provided                                  |
| ------------------------------------------------- | --------------------------------------------- |
| Build a program artifact from one NX source text  | Library registries and workspace directories  |
| Emit deterministic NX IR JSON and metadata        | Evaluating NX to values (use the IR runtime)  |
| Hover, completions, diagnostics, document symbols | Multi-module workspace builds                 |
| An in-process `NxLanguageService` with a prelude  | Threads, streaming, incremental analysis      |

Everything here answers over in-memory documents. Nothing needs a file to exist.

## Toolchain

The module targets `wasm32-wasip1`, whose standard library bundles wasi-libc, so tree-sitter's C
runtime and the NX grammar link with no shims. Building it needs:

- The `wasm32-wasip1` rustup target — listed in `rust-toolchain.toml`, so `rustup` installs it with
  the toolchain.
- `clang`, to compile the C sources for that target.
- The pinned wasi-sdk `wasi-sysroot`, which `scripts/fetch-wasi-sysroot.mjs` downloads into a
  gitignored `.cache/` under the repository, verifies by SHA-256, and reuses afterwards.

`pnpm --filter @nx-lang/sdk-wasm build` runs all of that: the fetch, cargo for the wasm target under
the `wasm-release` profile, and a copy of the module to `dist/nx.wasm`. There is no emscripten, no
wasm-bindgen and no wasm-pack; the ABI is JSON in and JSON out over a hand-written loader.

```bash
pnpm install
pnpm --filter @nx-lang/sdk-wasm build
pnpm --filter @nx-lang/sdk-wasm test
```

## Loading: compile once, host many

The two halves of loading are separate on purpose. `compileNxModule` produces a `WebAssembly.Module`
— the expensive, cacheable half — and `createNxHost` instantiates it, which takes tens of
milliseconds. A caller keeps the module and creates hosts from it as often as it needs to.

```ts
import { compileNxModule, createNxHost } from "@nx-lang/sdk-wasm";

// In a browser: stream the module straight from the network.
const module = await compileNxModule(fetch(nxModuleUrl));
const host = createNxHost(module);

const artifact = host.buildProgramArtifact(source, { fileName: "input.nx" });
try {
  const { json, metadata } = artifact.generateNxIr();
  console.log(metadata.programFingerprint);
} finally {
  artifact.dispose();
}
```

`compileNxModule` also accepts the module's bytes or an already-compiled `WebAssembly.Module`, so a
caller that has one already can pass it through without a special case.

Two entry points are selected by the package's `exports` conditions, and both expose the same
`createNxHost`:

- the default entry supplies WASI through `@bjorn3/browser_wasi_shim`;
- the `node` entry supplies it through `node:wasi`, and adds `loadNxModule()` for callers reading the
  module from the filesystem. Node marks `node:wasi` experimental and warns on import; the package's
  own tests and the playground's example check run with `--no-warnings`.

## Language snapshots

```ts
const uri = "nx://tenant/form.nx";
const snapshot = host.createLanguageSnapshot([{ uri, version: 1, source }]);
try {
  const hover = snapshot.hover(uri, { line: 2, character: 3 });
  const completions = snapshot.completions(uri, { line: 2, character: 7 });
  const report = snapshot.diagnostics();
  const symbols = snapshot.documentSymbols(uri);
} finally {
  snapshot.dispose();
}
```

A snapshot is immutable: build a new one when a document changes. Analysis runs on the first query
and is cached for the snapshot's lifetime, so several queries against unchanged text cost one
analysis. Positions count UTF-16 code units, the way JavaScript strings and browser editors count,
and results are the `@nx-lang/language-protocol` shapes, re-exported here.

For an editor that prepends a prelude to the queried document, `createLanguageService(host, options)`
implements the protocol's `NxLanguageService` over these snapshots, shifting positions into the
document's own coordinates. It shares that arithmetic with the HTTP handler, so the two cannot
disagree about a line number.

## A trap ends the host; the caller replaces it

The module targets `wasm32-wasip1`, where a panic aborts rather than unwinds. A panic or an
out-of-bounds access inside the module therefore ends the instance and its memory — there is no
`catch_unwind` that could hide it, and the request that trapped is lost.

The SDK makes that explicit rather than papering over it:

- the call that trapped throws `NxHostCrashedError`, naming the operation;
- every later call on that host throws the same error without entering the module, and `host.crashed`
  is `true`;
- `dispose()` on resources from a crashed host is tolerated, so cleanup paths never mask the crash;
- the caller recovers by calling `createNxHost` again on the compiled module it already has. That
  costs an instantiation, not a download or a recompilation.

```ts
try {
  return compile(host, source);
} catch (error) {
  if (error instanceof NxHostCrashedError) {
    host = createNxHost(module); // same module; no fetch
  }
  throw error;
}
```

The playground does exactly this inside its worker, so a compiler crash costs one request and the
editor stays live.

## Errors

| Error                      | Thrown when                                                          |
| -------------------------- | -------------------------------------------------------------------- |
| `NxEvaluationError`        | NX reports diagnostics: source that does not compile, an unparseable snapshot URI, duplicate identities. Carries `diagnostics`. |
| `NxDisposedResourceError`  | An operation is attempted on a disposed artifact, snapshot or host. Disposing twice is allowed. |
| `NxHostCrashedError`       | The module trapped, and on every later call to that host. Carries `operation`. |
| `NxWasmError`              | The module's ABI version is not the loader's, or it answered in a shape the loader cannot read. |

The names and shapes match `@nx-lang/sdk-node`, so code can move between the two bindings.

## ABI

The module exports `nx_wasm_abi_version`, which the loader checks before any other call and refuses
when it disagrees, naming both versions. Arguments cross as UTF-8 JSON in buffers from
`nx_wasm_alloc`; every operation answers with a pointer to a `{ status, ptr, len }` record whose
payload is UTF-8 JSON, released through `nx_wasm_result_free` before the call returns. Handles to
artifacts and snapshots are opaque to the loader.

The Rust side is `bindings/wasm/native` (crate `nx-sdk-wasm-native`), over `nx-api`, `nx-codegen` and
`nx-language-service`. Its NX IR and diagnostic payloads are the ones the Node binding serializes, and
the package's parity tests compare the two bindings' answers on every test run, so a divergence fails
the build rather than reaching a site.

## Layout

| Path            | What it holds                                                       |
| --------------- | ------------------------------------------------------------------- |
| `native/`       | The Rust crate and its ABI                                          |
| `src/`          | The loader, the two WASI entry points, errors and types             |
| `scripts/`      | The wasm build and clean scripts                                    |
| `test/`         | Loader, SDK, trap, entry-point and Node-SDK parity tests            |
| `dist/nx.wasm`  | The built module (gitignored; produced by `pnpm run build`)         |
