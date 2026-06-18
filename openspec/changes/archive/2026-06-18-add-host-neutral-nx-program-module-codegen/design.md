## Context

Executable NX JavaScript generation currently emits a file graph: a generated runtime helper file,
one ESM file per resolved NX module, and an index barrel. That layout is useful for readable output
and CLI workflows, but it is awkward for server-side caching because the code that represents the
NX program is mixed with file layout concerns and a runtime helper copy that should be shared
across many generated programs.

The target server workflow is different: compile an NX `ProgramArtifact` once, cache the generated
program code in a database, then later compose it into a V8 isolate with an isolate-host-provided
runtime module and a host-specific wrapper. The cached program must therefore represent NX
semantics only. It should not include Cloudflare Worker syntax, Rivet actor setup, HTTP routing,
database access, auth policy, or resource-limit configuration.

## Goals / Non-Goals

**Goals:**

- Emit a single JavaScript ESM source module from a `ProgramArtifact`.
- Keep the emitted module host-neutral and cacheable as an NX program artifact.
- Import NX runtime helpers from a stable runtime module specifier instead of embedding a runtime
  helper file in the cached artifact.
- Expose enough generated metadata for callers to cache, validate, and compose the module with a
  compatible runtime.
- Expose the new output style through an explicit CLI format selector while preserving the current
  CLI file-output default.
- Let .NET hosts invoke the same artifact-first program-module generation path through the managed
  runtime package.
- Preserve existing generated JavaScript semantics, component descriptor/schema APIs, diagnostics,
  deterministic ordering, and interpreter parity.
- Preserve the existing multi-file output style for CLI/readable generation.

**Non-Goals:**

- Generate Cloudflare Dynamic Worker `WorkerCode` objects, Worker `fetch` handlers, Rivet actors,
  or any other host adapter in this change.
- Add a database cache, isolate loader, bundler, import-map implementation, or runtime precompiler.
- Change NX runtime helper semantics beyond making the helper source available as a separately
  versioned runtime module.
- Add TypeScript single-module output in this change.
- Add reactive, dispatch, effect, action-handler, or lifecycle semantics beyond the current
  executable-codegen support.

## Decisions

### 1. Add a program-module output artifact instead of overloading file output

Introduce an explicit generated artifact for the host-neutral program module, such as
`GeneratedJsProgramModule`, with fields for:

- JavaScript ESM source text;
- a logical module name;
- the runtime import specifier used by the source;
- the NX JavaScript runtime ABI/version expected by the source;
- the originating program fingerprint;
- exported entrypoint and component/schema names.

This keeps database/cache metadata structured without forcing callers to parse generated source.
The source may also export a small manifest value for runtime introspection, but the Rust API
remains the authoritative metadata surface.

Alternatives considered:

- Return a `CodegenOutput` containing one `GeneratedFile`. Rejected because a single module cache
  artifact has different metadata and invariants from file-generation output.
- Require callers to bundle the existing file graph themselves. Rejected because the compiler owns
  generated naming, import aliasing, and deterministic module ordering, and it can produce a safer
  artifact directly.

### 2. Use one ESM module with only a runtime import

The new output should flatten the `CodegenProgram` into one ESM module. Cross-module NX references
become local generated declarations or aliases inside that module. The only allowed static import
from generated source is the NX runtime module, using a configurable specifier that defaults to a
stable virtual value such as `nx:runtime`.

This allows an isolate host to map the runtime efficiently however it wants: a precompiled module,
a cached file, an import hook, or a module entry in a platform-specific manifest. Tests can use a
relative runtime specifier such as `./nx-runtime.js` without changing the default server contract.

Alternatives considered:

- Inline the runtime helpers into every cached NX module. Rejected because it bloats database
  artifacts and prevents runtime helper precompilation/reuse.
- Emit a Cloudflare-style `{ mainModule, modules }` object from `nx-codegen`. Rejected because that
  is host packaging, not NX program semantics.

### 3. Keep host wrappers outside `nx-codegen`

The generated NX program module should expose entrypoints and component schemas; it should not
decide how a request becomes props, which bindings are available, what auth applies, how errors are
serialized over HTTP, or what isolate limits are set. The isolate host can generate a tiny wrapper
module at load time that imports the cached NX program and the shared runtime and then exposes the
platform's required entrypoint.

This separation lets ReachMe change wrapper policy without invalidating cached NX program modules.
It also avoids tying core NX codegen to Cloudflare, Rivet, or any specific V8 embedder.

### 4. Version the runtime boundary explicitly

The program-module artifact should include a runtime ABI identifier such as
`nx-js-runtime-v1`. Generated source may import named helpers from the runtime module, but callers
must compare the artifact's runtime ABI with the runtime module they provide before loading the
program.

When helper names or behavior change incompatibly, the runtime ABI changes. That gives hosts a
clear cache-invalidation and compatibility check without inspecting generated code.

### 5. Reuse the existing codegen model and emission logic

The implementation should continue to build a `CodegenProgram` from `ProgramArtifact`. The new
single-module emitter should reuse the existing declaration, expression, schema, component, and
runtime-helper collection logic as much as practical, but route imports through a different
linking strategy.

Expected implementation shape:

- extend codegen options with a program-module layout or add a dedicated API entry point;
- add an emit context mode that plans globally unique declaration names across all modules;
- emit declarations in deterministic module/declaration order into one source buffer;
- emit a generated manifest export and public entrypoint/schema exports;
- keep existing multi-file tests passing unchanged;
- add program-module tests for cross-module imports, runtime import isolation, component schemas,
  and host-neutral source constraints.

### 6. Add an explicit CLI output format

`nxlang codegen` should expose the program-module layout through a new explicit format selector,
for example `--format files|program-module`. The default remains `files`, so existing JavaScript
and TypeScript CLI workflows continue to write `nx-runtime.*`, generated module files, and
`index.*`.

Program-module CLI output is JavaScript-only in this change. When selected with
`--target javascript`, the CLI writes one generated program module source file and does not write a
runtime helper copy or index barrel. When selected with `--target typescript`, the CLI should fail
with a clear error instead of silently falling back to file output.

This CLI path is for inspection, local debugging, and simple host integration. The Rust API remains
the authoritative structured metadata surface for cache keys and host composition.

### 7. Surface program-module generation in .NET through the native FFI

The .NET runtime package already builds reusable `NxProgramArtifact` handles through `nx-ffi`.
Managed codegen should hang from that artifact surface, for example
`NxProgramArtifact.GenerateJSProgramModule(...)`, so managed hosts do not need to reparse source or
call the CLI.

The native FFI should expose an artifact-first function that accepts optional logical module name
and runtime import specifier inputs, invokes `nx-codegen`, and returns a serialized
program-module payload containing source text plus the same metadata exposed by Rust. Diagnostics
should reuse the existing native error payload conventions so managed callers get
`NxEvaluationException` for invalid artifacts or unsupported codegen inputs.

The managed API should deserialize the native payload into immutable C# model types rather than
forcing callers to parse generated source or untyped JSON.

## Risks / Trade-offs

- [Name collisions when flattening modules] -> Use a global generated-name planner rather than
  reusing per-module local names.
- [Generated source accidentally depends on host APIs] -> Add tests that reject `fetch`, `Request`,
  `Response`, `process`, `require`, and Node built-in imports in program-module output.
- [Runtime helper ABI drifts without cache invalidation] -> Include a runtime ABI value in both
  emitted metadata and tests, and require hosts to validate it before loading.
- [Single modules become large] -> Start with the simple cache artifact; introduce multi-module
  cached graphs later only if measurements show load/parse costs need it.
- [Virtual import specifiers are inconvenient in local tests] -> Make the runtime specifier
  configurable while keeping the default host contract stable.
- [CLI output could be mistaken for a runnable file graph] -> Require an explicit format and do not
  emit `nx-runtime.js` in program-module mode.
- [.NET metadata payload can drift from Rust metadata] -> Serialize the Rust result shape directly
  through `nx-ffi` and cover it with native and managed tests.

## Migration Plan

1. Add the program-module output API and metadata types behind the existing `nx-codegen` crate.
2. Implement single-module JavaScript emission using the current `CodegenProgram`.
3. Add tests that execute the generated module with a separately supplied runtime module.
4. Expose the layout through `nxlang codegen --format program-module` for inspection/debugging.
5. Expose artifact-first program-module generation through `nx-ffi` and the .NET runtime package.
6. Keep the existing file output as the default CLI behavior.

Rollback is straightforward: remove the new program-module API and tests. Existing file-based
TypeScript/JavaScript generation remains unchanged.

## Open Questions

- Should the generated source export a manifest value in addition to returning structured metadata
  from Rust, or is Rust-side metadata sufficient for all initial hosts?
- Should the default runtime import specifier be exactly `nx:runtime`, or should it include an ABI
  segment such as `nx:runtime/js-v1`?
