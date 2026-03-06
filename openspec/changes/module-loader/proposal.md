## Why

NX already supports import syntax (`import "./foo"`, `import { Bar } from "./lib"`), but there is no runtime mechanism to actually resolve and load modules from those paths. Without a ModuleLoader abstraction, the interpreter and tooling cannot execute multi-file NX programs. A pluggable loader trait also lays the groundwork for future sources (URLs, in-memory buffers, FFI callbacks) without coupling the core to any single resolution strategy.

## What Changes

- Add a new `nx-module` crate for shared module/source-loading types used across parsing, evaluation, CLI, and bindings.
- Introduce a `ModuleLoader` trait in `nx-module` with two operations: `resolve` (path string → `ModuleId`) and `load` (ModuleId → source text).
- Introduce `ModuleId` as a unique, opaque identifier for a resolved module.
- Introduce `ModuleSource` and importer-aware resolution in `nx-module` so imports can be resolved consistently before and during evaluation.
- Introduce `ModuleLoadError` in `nx-module` so loader failures stay as lower-level domain errors instead of depending on `NxDiagnostic`.
- Implement `DirectoryModuleLoader` — resolves `.nx` files relative to a given directory path.
- Add a `ModuleLoaderList` (ordered list of loaders) that tries each loader in sequence.
- Thread the loader set through `eval_source` / the interpreter so imports can be resolved for multi-file evaluation.
- Convert `ModuleLoadError` into `nx_diagnostics::Diagnostic`, and then into `NxDiagnostic`, at the `nx-api` boundary.
- Extend the CLI (`nxlang run`) with a `--module-path` flag (repeatable) to specify directories.
- Extend the C FFI and .NET binding to accept a list of module search directories.

## Capabilities

### New Capabilities
- `module-loader`: The `nx-module` crate, `ModuleLoader` trait, `ModuleId`, `ModuleSource`, `ModuleLoadError`, `DirectoryModuleLoader`, and `ModuleLoaderList` — the core abstraction for resolving and loading NX modules from external sources.

### Modified Capabilities
- `module-imports`: Import statements currently exist only in the parser/HIR with no runtime resolution. This change wires import paths to the ModuleLoader at evaluation time, making imports functional.

## Impact

- **Rust crates**: `nx-module` (new shared crate for module/source-loading types + `ModuleLoadError`), `nx-api` (consume `nx-module`, map `ModuleLoadError` to diagnostics, eval_source signature change), `nx-interpreter` (import resolution during execution), `nx-cli` (new CLI flag), `nx-ffi` (new FFI entry point or extended parameters).
- **.NET binding**: `NxRuntime` gains a new overload or options type accepting module directories.
- **Existing callers**: `eval_source` signature changes — current zero-loader callers must pass an empty loader set (or a default is provided). This is **BREAKING** for Rust API consumers but not for end-user NX source files.
- **Workspace layout**: Cargo workspace gains a new low-level crate instead of placing shared loader infrastructure in `nx-api`, avoiding dependency inversion between `nx-api`, `nx-interpreter`, and future parse-time consumers.
