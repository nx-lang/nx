## Context

NX already has Rust host/compiler/runtime APIs in `nx-api`, a C ABI in `nx-ffi`, managed .NET
bindings under `bindings/dotnet`, and a pure TypeScript NX IR runtime under `runtime/typescript`.
The requested Node support is a separate native host/compiler SDK: Node callers need to construct
in-memory workspaces, validate them, build reusable program artifacts, generate deterministic NX
IR, and evaluate supported entrypoints without invoking .NET or the CLI.

The downstream ReachMe use case depends on database-backed NX source plus in-memory built-in
library modules. Its Node process needs stable logical module identities, deterministic IR JSON,
structured diagnostics, and explicit resource lifetimes for build contexts and program artifacts.

The existing .NET package/project name `NxLang.Runtime` no longer fits the managed surface because
it includes compilation, validation, diagnostics, workspace artifacts, and IR generation. The .NET
binding should be renamed to `NxLang.Sdk` as part of this change, with no compatibility package for
the old name.

## Goals / Non-Goals

**Goals:**

- Add a source-built Node package, `@nx-lang/sdk-node`, that exposes first-class NX
  host/compiler/runtime access through napi-rs / N-API.
- Rename the .NET package, project, and assembly from `NxLang.Runtime` to `NxLang.Sdk`.
- Keep the public JavaScript and TypeScript API high-level: callers use typed objects, options,
  buffers, JSON values, and diagnostics rather than raw native pointers.
- Preserve parity with the existing Rust/.NET host model for workspace validation, workspace
  program builds, deterministic NX IR generation, `root()` JSON and byte evaluation, structured
  diagnostics, and disposable build resources.
- Support in-memory workspace modules as normal input to validation and program artifact builds,
  including stable logical identity normalization and duplicate detection.
- Document local source consumption now and leave room for future npm publishing with prebuilt
  native binaries.

**Non-Goals:**

- Replace or merge with the pure TypeScript NX IR runtime under `runtime/typescript`.
- Add browser, WASM, or non-Node JavaScript support.
- Add ReachMe-specific chat-link APIs, schema shortcuts, or persistence logic.
- Expose unstable raw native handles as the main public API.
- Require temp files for ordinary workspace compilation.
- Preserve the previous `NxLang.Runtime` .NET package identity or add a compatibility package.

## Decisions

### Use napi-rs / N-API for the native Node module

The binding will add a Node package under `bindings/node` and a Rust N-API crate, for example
`bindings/node/native`, that depends on `nx-api` and `nx-codegen`. The Node package name will be
`@nx-lang/sdk-node`. The native crate will be added to the Cargo workspace and built through
napi-rs tooling from the Node package.

Alternatives considered:

- Reuse `nx-ffi` through a JavaScript FFI loader. This would expose low-level pointer and buffer
  concerns to Node, complicate packaging, and duplicate lifetime handling already solved by
  napi-rs classes.
- Shell out to `nx` CLI commands. This would not support normal in-memory workspaces, reusable
  artifacts, deterministic host-owned metadata capture, or robust resource reuse.
- Invoke the .NET binding from Node. This adds an unnecessary runtime dependency and keeps Node
  from being a first-class host.

### Use SDK naming for public language packages

The public .NET package/project/assembly will be renamed to `NxLang.Sdk`, and the public Node
package will be named `@nx-lang/sdk-node`. SDK is the broadest accurate term for the surface:
callers can compile, validate, generate IR, inspect diagnostics and metadata, build artifacts, and
evaluate supported entrypoints. The `sdk-node` suffix identifies the Node version of the JavaScript
NX SDK rather than an SDK for calling into Node itself. The repository can keep
implementation-oriented folders such as `bindings/dotnet` and `bindings/node`.

Alternatives considered:

- Keep `NxLang.Runtime` and use `@nx-lang/node-runtime`. This is too narrow and collides
  conceptually with the pure TypeScript IR runtime.
- Use `Bindings` in package names. This is accurate internally but sounds like raw FFI rather than
  the high-level API these packages expose.
- Use `Host` in package names. This captures embedding, but `SDK` is more familiar for consumers
  who need compiler, diagnostics, artifact, and evaluation APIs together.

### Wrap native resources in TypeScript-friendly classes

The public API will expose classes such as `NxWorkspace`, `NxLibraryRegistry`,
`NxProgramBuildContext`, and `NxProgramArtifact`. These classes will own underlying native
resources internally and provide `dispose()` plus `[Symbol.dispose]` where supported. Methods will
throw typed `NxNativeError` for interop failures and `NxEvaluationError` for NX diagnostics.

Native handles will not be public properties. The implementation may use napi-rs object finalizers
as a backstop, but deterministic disposal remains the documented lifecycle for long-lived servers.

Alternatives considered:

- Expose opaque numeric or BigInt handles and free functions. This mirrors FFI internals but makes
  incorrect lifetime and cross-resource use easy.
- Return only plain objects with hidden global registries. This hides lifetime semantics and makes
  leaks harder to diagnose in persistent Node processes.

### Keep workspace input fully in memory

Node callers will submit modules as `{ identity, source }` values, where source may be a string,
`Buffer`, or `Uint8Array`. The binding will convert these inputs into the existing Rust workspace
model and rely on NX logical identity normalization rather than filesystem canonicalization.

Workspace validation will return diagnostics as data. Program build failures will throw an
`NxEvaluationError` carrying the same diagnostic array so callers can use normal exception flow
while still persisting structured failure metadata.

Alternatives considered:

- Require callers to materialize temporary files and point NX at a directory. This fails the
  database-backed source use case and risks nondeterminism from host filesystem state.
- Invent a Node-specific import resolver. That would bypass the existing workspace and
  `ProgramBuildContext` rules and make parity with .NET harder to maintain.

### Represent generated IR as JSON text plus metadata

`NxProgramArtifact.generateNxIr()` will return an object equivalent to the managed
`NxGeneratedNxIr`: deterministic IR JSON text and structured metadata such as runtime ABI,
program fingerprint, entrypoints, and references. The binding will treat the Rust/codegen output
as the source of truth and avoid reserializing parsed JSON in JavaScript.

Alternatives considered:

- Return only parsed JavaScript objects. This is convenient but risks changing stable byte output
  through JavaScript serializer behavior.
- Return only JSON text. That would force every caller to parse metadata that NX already emits in a
  structured form.

### Align evaluation scope with native host parity

The first Node API will support JSON and byte evaluation paths that exist in the native host and
.NET binding today, including `root()` evaluation from source or a reusable `NxProgramArtifact`.
The API can reserve explicit entrypoint options, but unsupported or missing entrypoints must report
diagnostics rather than falling back to global lookup or implicit CLI behavior.

Alternatives considered:

- Implement additional named-entrypoint lookup in JavaScript from emitted IR. That belongs to the
  pure TypeScript IR runtime and would mix host compilation with IR execution semantics.
- Promise broader evaluation support than native APIs expose. That would create parity gaps and
  make .NET/Node behavior drift.

## Risks / Trade-offs

- [Risk] napi-rs introduces a new toolchain and packaging surface. -> Mitigation: keep the native
  crate thin, document local source builds first, and isolate npm/prebuild work behind package
  scripts that can evolve.
- [Risk] native resource leaks are more likely in long-running Node servers. -> Mitigation:
  provide explicit disposal, finalizer backstops, disposal tests, and examples that use
  `try/finally` or explicit resource management.
- [Risk] diagnostics may diverge across Rust, .NET, CLI, and Node adapters. -> Mitigation:
  serialize from shared Rust diagnostic models and add parity tests that compare structured
  diagnostics for representative failures.
- [Risk] package naming may conflict with the pure TypeScript IR runtime. -> Mitigation:
  document `@nx-lang/sdk-node` as Node-only native SDK access and leave `@nx-lang/ir-runtime` for
  persisted IR execution.
- [Risk] renaming `NxLang.Runtime` to `NxLang.Sdk` breaks existing .NET package references. ->
  Mitigation: treat the rename as intentional for the current pre-compatibility phase, update
  tests/docs/project names together, and do not ship a compatibility package.
- [Risk] prebuilt binary distribution can lag initial source support. -> Mitigation: make source
  consumption the required first milestone and document prebuild distribution as follow-up-ready
  packaging work.

## Migration Plan

This is additive for Rust, CLI, and the TypeScript IR runtime, but breaking for the current .NET
package identity. Implementation should rename the managed project/package/assembly to
`NxLang.Sdk`, update .NET docs/tests/source-based consumption guidance, and avoid retaining
`NxLang.Runtime` compatibility artifacts. The Node SDK should land behind the new `bindings/node`
package, with tests and docs proving source-based local consumption before any published
npm/prebuild workflow is required.

Rollback is to remove the new Node package and revert the .NET project/package rename before
publication.

## Resolved Questions

- Initial source-built Node support targets Node 22 or newer, matching the package
  `engines.node >= 22` declaration now that Node 20 is past end-of-life. Future release validation
  should cover supported Node LTS majors, starting with Node 22 and Node 24, while the first
  prebuild matrix remains platform-focused on `linux-x64`, `osx-arm64`, and `win-x64`; expanding
  Node majors or platforms can be handled as packaging follow-up work.
- The initial native Node SDK remains `root()`-only. The public `entrypoint` option is reserved for
  future host support and intentionally throws a structured `unsupported-entrypoint` diagnostic for
  non-`root` values until the Rust host exposes named-entrypoint evaluation directly.
