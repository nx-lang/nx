## 1. Create `nx-module` Crate

- [ ] 1.1 Create a new `nx-module` crate and add it to the Cargo workspace
- [ ] 1.2 Add `ModuleId` newtype (wrapping `String`) with `PartialEq`, `Eq`, `Hash`, `Clone`, `Debug`
- [ ] 1.3 Add `ModuleSource` struct with `source: String` and `file_name: String` fields
- [ ] 1.4 Add `ModuleLoadError` in `nx-module` for structured loader failures (invalid path, invalid module ID, case mismatch, I/O failure)
- [ ] 1.5 Define `ModuleLoader` trait with `resolve(&self, from: Option<&ModuleId>, module_path: &str) -> Result<Option<ModuleId>, ModuleLoadError>` and `load(&self, id: &ModuleId) -> Result<ModuleSource, ModuleLoadError>`
- [ ] 1.6 Implement `ModuleLoaderList` holding `Vec<Box<dyn ModuleLoader>>` that implements `ModuleLoader` with ordered fallback resolution and hard-error short-circuiting
- [ ] 1.7 Add unit tests for `ModuleLoaderList` (empty list returns None, first match wins, no match returns None, error short-circuits)

## 2. DirectoryModuleLoader (`nx-module`)

- [ ] 2.1 Implement `DirectoryModuleLoader` with a base directory path field and `new(base: PathBuf)` constructor
- [ ] 2.2 Implement `resolve` so relative paths (`./`, `../`) use the importing module's directory when `from` is present, and non-relative paths use the loader base directory
- [ ] 2.3 In `resolve`, try `<path>.nx` and then `<path>/mod.nx`, canonicalizing the match to an absolute `ModuleId`
- [ ] 2.4 Implement `load` to read the file at the canonical path and return `ModuleSource`
- [ ] 2.5 After canonicalization, compare canonical path components against the requested path to detect case mismatches; return `Err(ModuleLoadError::CaseMismatch { .. })` with enough structured data for later diagnostic conversion
- [ ] 2.6 Add unit tests for `DirectoryModuleLoader` (resolve with/without `.nx` extension, `mod.nx` fallback, missing file returns None, relative import resolution, case mismatch returns error, load returns correct content)

## 3. Wire Consumers to `nx-module`

- [ ] 3.1 Add `nx-module` as a dependency of `nx-api`, `nx-cli`, `nx-interpreter`, and `nx-ffi`
- [ ] 3.2 Re-export `nx-module` types from `nx-api` to keep the public Rust API ergonomic while leaving ownership in `nx-module`
- [ ] 3.3 Update workspace build/test configuration impacted by the new crate boundary

## 4. Evaluation Pipeline Integration (`nx-api`)

- [ ] 4.1 Update `eval_source` to accept a `&ModuleLoaderList` from `nx-module`
- [ ] 4.2 Pass the loader set through the multi-file evaluation path
- [ ] 4.3 Convert `ModuleLoadError` into `nx_diagnostics::Diagnostic`, attaching import-site context when available
- [ ] 4.4 Convert those diagnostics into `NxDiagnostic` at the `nx-api` boundary
- [ ] 4.5 Update all existing callers of `eval_source` to pass a `ModuleLoaderList`
- [ ] 4.6 Add integration tests covering successful imports and loader failures (e.g. case mismatch, I/O failure)

## 5. Interpreter Import Resolution (`nx-interpreter`)

- [ ] 5.1 Extend `Interpreter` to accept and store a reference or owned `ModuleLoaderList` from `nx-module`
- [ ] 5.2 Implement import resolution: when processing a module's imports, call `resolve(from, path)` → `load` → `parse_str` → `lower` for each import path
- [ ] 5.3 Add a module cache (`HashMap<ModuleId, Module>`) to avoid re-parsing the same module
- [ ] 5.4 Bind imported names into scope: wildcard imports add all public items, selective imports add only named items with optional aliases
- [ ] 5.5 Return or surface `ModuleLoadError` values so the evaluation pipeline can convert them into diagnostics
- [ ] 5.6 Add tests for interpreter import resolution (wildcard import, selective import, namespace import, missing module error)

## 6. CLI Integration (`nx-cli`)

- [ ] 6.1 Add `--module-path` repeatable flag to the `Run` command in clap
- [ ] 6.2 Build `ModuleLoaderList` from the file's parent directory (implicit) plus `--module-path` values
- [ ] 6.3 Pass `ModuleLoaderList` through the CLI run path; if the CLI continues to bypass `nx-api`, keep its loader behavior aligned with `nx-api`
- [ ] 6.4 Add CLI integration test: run a file that imports from a module path directory

## 7. C FFI (`nx-ffi`)

- [ ] 7.1 Update `nx_eval_source_json` and `nx_eval_source_msgpack` to accept an additional paths array parameter (pointer + count)
- [ ] 7.2 Parse the paths parameter into a `ModuleLoaderList` of `DirectoryModuleLoader` instances
- [ ] 7.3 Bump the FFI ABI version and regenerate the public C header
- [ ] 7.4 Add FFI tests verifying module paths work

## 8. .NET Binding (`bindings/dotnet`)

- [ ] 8.1 Add optional `modulePaths` parameter to `EvaluateToJson`, `EvaluateToMessagePack`, and `Evaluate<T>`
- [ ] 8.2 Marshal module paths to the FFI functions (encode as UTF-8 byte arrays with pointer + count)
- [ ] 8.3 Update `NxNativeMethods` P/Invoke declarations to match the updated FFI signatures
- [ ] 8.4 Add .NET tests verifying module path forwarding
