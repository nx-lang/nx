## Context

NX has full import syntax support in the parser and HIR (`import "./foo"`, `import { Bar } from "./lib"`), but no runtime resolution or loading. The current `eval_source` function in `nx-api` processes a single source string with no ability to follow imports. The interpreter (`nx-interpreter`) executes against a single `Module` with no multi-module awareness. The CLI reads one file and evaluates it. The C FFI and .NET binding accept a source string and return a result.

Module loading is not only a runtime concern. The same loader abstraction will be needed anywhere NX follows imports across source files, which includes parse/lower/evaluate flows and future multi-file checking flows. That makes `nx-api` the wrong home for the abstraction because `nx-api` already sits above `nx-interpreter`, while parse-time consumers should also be able to use the same types.

This design adds the minimal infrastructure to make directory-based imports work end-to-end while introducing a dedicated `nx-module` crate for shared module/source-loading concepts.

## Goals / Non-Goals

**Goals:**
- Add a new `nx-module` crate for shared module/source-loading types
- Define a `ModuleLoader` trait that abstracts module resolution and loading
- Implement `DirectoryModuleLoader` for resolving `.nx` files from a directory on disk
- Provide `ModuleLoaderList` to compose multiple loaders with ordered fallback
- Wire loaders through the evaluation pipeline so `import` statements resolve at runtime
- Expose module search paths via CLI flags and FFI/.NET parameters

**Non-Goals:**
- URL-based module loading (future work)
- FFI callback-based loaders (future work)
- Package manager or registry support
- Changing the import syntax or parser
- Introducing a catch-all `nx-core` crate

## Decisions

### 1. Add a dedicated `nx-module` crate

The shared loader abstractions will live in a new `nx-module` crate. That crate owns:
- `ModuleId`
- `ModuleSource`
- `ModuleLoadError`
- `ModuleLoader`
- `DirectoryModuleLoader`
- `ModuleLoaderList`
- Path-resolution helpers and module-loading error types

`nx-module` is intentionally narrow: it models loading and identifying modules, but it does not own parsing, lowering, evaluation, or binding APIs. It sits below `nx-api`, `nx-cli`, `nx-ffi`, and `nx-interpreter` in the dependency graph.

**Why**: `ModuleLoader` is needed by multi-file orchestration before and during evaluation, so it must be usable by parse-time and runtime consumers alike. Keeping it in `nx-api` would invert the dependency graph because `nx-api` already depends on `nx-interpreter`.

**Alternative**: Keep the loader in `nx-api`. Rejected because it creates a layering problem and makes the shared types unavailable to lower-level consumers without introducing crate cycles.

**Alternative**: Add a generic `nx-core` crate. Rejected because it is too broad and would quickly become a dumping ground for unrelated shared types.

### 2. ModuleId is a newtype over String

`ModuleId` wraps a `String` representing the canonical absolute path of the resolved module. This is simple, unique for filesystem modules, and easily extensible to URLs later.

**Alternative**: Numeric ID with a registry. Rejected as premature — adds complexity without benefit until caching is needed.

### 3. Two-phase resolve/load API with importer context

```rust
pub struct ModuleId(String);

pub enum ModuleLoadError {
    InvalidPath { path: String, reason: String },
    InvalidModuleId { id: String },
    CaseMismatch { requested: String, actual: String },
    Io { path: String, operation: String, message: String },
}

pub trait ModuleLoader {
    fn resolve(
        &self,
        from: Option<&ModuleId>,
        module_path: &str,
    ) -> Result<Option<ModuleId>, ModuleLoadError>;

    fn load(&self, id: &ModuleId) -> Result<ModuleSource, ModuleLoadError>;
}

pub struct ModuleSource {
    pub source: String,
    pub file_name: String,
}
```

Errors use `ModuleLoadError`, a lower-level domain error type in `nx-module`. It captures structured failure kinds such as invalid paths, invalid module IDs, case mismatches, and I/O failures without depending on `NxDiagnostic` or any API-layer serialization model.

`resolve` returns:
- `Ok(Some(ModuleId))` when the module is found
- `Ok(None)` when the loader does not recognize the path
- `Err(ModuleLoadError)` for hard failures (case mismatch, permission denied, I/O error)

The `from` parameter is the importing module. It is required so relative imports (`./foo`, `../bar`) can resolve from the importing module's directory rather than from the loader's base search path. `None` is used when there is no importing module context, such as the initial entry source.

The multi-file evaluation pipeline converts `ModuleLoadError` into `nx_diagnostics::Diagnostic` once it has source-location context such as the importing file and import span. `nx-api` then converts those diagnostics into `NxDiagnostic` for the public API/FFI boundary. This matches the existing parse-error path, where lower-level crates return internal diagnostics and `nx-api` serializes them.

### 4. DirectoryModuleLoader resolution strategy

Given a base directory, an optional importing module, and a module path string:
1. If `module_path` is relative (`./` or `../`) and `from` is present, resolve from the importing module's parent directory
2. Otherwise resolve from the loader's configured base directory
3. If the path already ends in `.nx`, try it directly
4. Otherwise try `<path>.nx`
5. Then try `<path>/mod.nx` (directory module pattern)
6. Return the first match found, canonicalized to an absolute path

**Case sensitivity**: All path matching is case-sensitive on every platform. After resolving a path on a case-insensitive filesystem (macOS, Windows), the canonical path from `std::fs::canonicalize` is compared against the requested path components. If the case differs (e.g. `import "./Utils"` resolves to `utils.nx` on disk), `resolve` returns an error with a message indicating the case mismatch and the correct casing. This prevents silent cross-platform bugs.

### 5. ModuleLoaderList tries loaders in order

`ModuleLoaderList` holds a `Vec<Box<dyn ModuleLoader>>` and implements `ModuleLoader` itself. On `resolve`, it tries each loader in sequence and returns the first `Ok(Some(ModuleId))`. If any loader returns `Err(ModuleLoadError)`, the error propagates immediately — no further loaders are tried. Hard failures such as case mismatches, permission failures, or invalid paths should not be masked by falling through to another loader.

For the directory-based implementation, since `ModuleId` is a canonical path, any `DirectoryModuleLoader` can load it — so `load` simply reads the file at the canonical path.

### 6. `nx-api`, CLI, and bindings consume `nx-module`

`nx-api` will depend on `nx-module` rather than owning loader types itself. The public evaluation entry point becomes:

```rust
pub fn eval_source(source: &str, file_name: &str, loaders: &ModuleLoaderList) -> EvalResult
```

`nx-cli` and `nx-ffi` will also depend on `nx-module` to construct loader lists. `nx-api` may re-export `nx-module` types for ergonomics, but the ownership of the abstractions remains in `nx-module`.

When import resolution fails, `nx-api` converts `ModuleLoadError` into `nx_diagnostics::Diagnostic` with an appropriate code such as `"module-case-mismatch"` or `"module-io-error"`, attaches import-site context when available, and then serializes that into `NxDiagnostic`.

### 7. Import resolution uses `nx-module` types end-to-end

The initial implementation will pass a `ModuleLoaderList` through the multi-file evaluation path. When it encounters an import, it calls:

`resolve(from, path)` → `load(id)` → `parse_str` → `lower`

Imported modules are cached by `ModuleId` to avoid repeated work. Any `ModuleLoadError` values produced during this process are converted into internal diagnostics in the evaluation pipeline before crossing the `nx-api` boundary. The implementation may live partly in `nx-api` and partly in `nx-interpreter`, but the loader abstractions themselves remain in `nx-module` so the same model can be reused by future multi-file parsing or checking flows.

### 8. CLI adds `--module-path` flag

```text
nxlang run <file> --module-path ./libs --module-path ./vendor
```

Each `--module-path` value creates a `DirectoryModuleLoader`. The file's own parent directory is always implicitly the first search path so relative imports like `import "./sibling"` work without explicit flags.

### 9. FFI and .NET binding gain a paths parameter

The existing FFI functions (`nx_eval_source_json`, `nx_eval_source_msgpack`) are updated to accept an additional paths array parameter (pointer + count). The .NET binding updates its methods accordingly:

```csharp
public static string EvaluateToJson(string source, string? fileName = null, IReadOnlyList<string>? modulePaths = null)
```

## Risks / Trade-offs

- **[Circular imports]** → Not handled in this change. If module A imports B which imports A, it will infinite-loop. Mitigation: document as a known limitation; cycle detection is a follow-up task.
- **[Breaking Rust API]** → `eval_source` signature changes affect Rust callers, and the workspace gains a new crate. Acceptable per project policy — no backward compatibility needed yet.
- **[Breaking FFI ABI]** → FFI function signatures change. All FFI consumers (including .NET binding) must update together.
- **[More crates in the workspace]** → `nx-module` adds another crate, but the layering is clearer and avoids placing shared infrastructure in a top-level API crate.
- **[Path canonicalization]** → Platform-specific path behavior (Windows vs Unix). Mitigation: use `std::fs::canonicalize` and test on both platforms in CI.
