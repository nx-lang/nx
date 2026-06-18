## 1. Public API And Options

- [x] 1.1 Add a JavaScript runtime ABI constant and default host-neutral runtime import specifier in `crates/nx-codegen`.
- [x] 1.2 Define a `GeneratedJsProgramModule` result type with source text, logical module name, runtime import specifier, runtime ABI, program fingerprint, function entrypoint exports, and component/schema exports.
- [x] 1.3 Add program-module options for logical module name and runtime import specifier, defaulting to a stable host-neutral runtime specifier.
- [x] 1.4 Add public API entry points that emit a `GeneratedJsProgramModule` from either `ProgramArtifact` or an already-built `CodegenProgram`.
- [x] 1.5 Preserve existing `CodegenOptions` and file-output APIs so current TypeScript/JavaScript file generation remains source-compatible.

## 2. Single-Module Emission

- [x] 2.1 Add an emitter context mode that plans globally unique generated names across all `CodegenProgram` modules.
- [x] 2.2 Emit declarations from all modules into one deterministic JavaScript ESM source module using stable module/declaration ordering.
- [x] 2.3 Replace cross-module generated imports with local generated references or aliases inside the single program module.
- [x] 2.4 Emit only the NX runtime helper import from the configured runtime import specifier.
- [x] 2.5 Emit public ESM exports for function entrypoints, concrete component descriptor functions, schema values, and state helper exports where applicable.
- [x] 2.6 Emit or expose generated manifest metadata without requiring hosts to parse the generated source for cache metadata.

## 3. Runtime Boundary

- [x] 3.1 Ensure program-module output never emits `nx-runtime.js` or embeds the full runtime helper implementation.
- [x] 3.2 Keep generated program-module source free of host wrappers, Cloudflare Worker exports, Dynamic Worker manifests, Rivet actor setup, CommonJS `require`, and Node built-in imports.
- [x] 3.3 Make runtime helper collection reuse the existing component/schema helper logic while routing helper imports through the configured runtime specifier.
- [x] 3.4 Add a way for tests and hosts to obtain the standalone JavaScript NX runtime helper source and its matching runtime ABI.

## 4. CLI And Documentation

- [x] 4.1 Expose program-module output through an explicit `nxlang codegen` format option without changing the default file layout.
- [x] 4.2 Document the Rust API workflow for caching `GeneratedJsProgramModule` source and composing it later with a separately supplied runtime and host wrapper.
- [x] 4.3 Document that Cloudflare, Rivet, database cache, isolate manifest, and host wrapper generation are intentionally outside this change.
- [x] 4.4 Document the CLI program-module format and .NET artifact-first generation API.

## 5. .NET And FFI API

- [x] 5.1 Add `nx-ffi` support for generating a JavaScript program module from an `NxProgramArtifact` handle.
- [x] 5.2 Serialize the generated source and metadata through the native buffer using the existing diagnostics/error conventions.
- [x] 5.3 Add managed .NET option/result model types for program-module codegen metadata.
- [x] 5.4 Add `NxProgramArtifact` APIs that invoke native program-module codegen with default and configured options.

## 6. Verification

- [x] 6.1 Add unit tests for program-module metadata, default options, configured runtime import specifier, and invalid artifact diagnostics.
- [x] 6.2 Add snapshot tests proving simple root programs emit one JavaScript ESM module with only the runtime import and expected public exports.
- [x] 6.3 Add cross-module tests proving imported functions, values, records, enums, unions, and components are flattened into the single program module with collision-free names.
- [x] 6.4 Add execution tests that load generated program-module output with a separately supplied `nx-runtime.js` module and compare results with interpreter evaluation.
- [x] 6.5 Add component schema execution tests proving generated component initialize/evaluate APIs work from the single program module.
- [x] 6.6 Add deterministic output tests for repeated program-module generation with equivalent `ProgramArtifact` inputs and options.
- [x] 6.7 Add host-neutrality tests proving program-module source omits host entrypoints, Node built-ins, CommonJS `require`, and non-runtime static imports.
- [x] 6.8 Add CLI tests for default file output, explicit program-module output, and TypeScript rejection.
- [x] 6.9 Add `nx-ffi` tests for program-module payload metadata and diagnostics.
- [x] 6.10 Add .NET tests for program-module generation from `NxProgramArtifact`.
- [x] 6.11 Run targeted `cargo test -p nx-codegen`, `cargo test -p nx-cli`, `cargo test -p nx-ffi`, and `dotnet test bindings/dotnet/NxLang.sln` if the native library can be staged.
