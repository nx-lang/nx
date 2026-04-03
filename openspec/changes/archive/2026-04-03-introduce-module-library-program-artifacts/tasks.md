## 1. Rename the core lowered HIR type

- [x] 1.1 Rename `nx-hir::Module` to `LoweredModule` and propagate the rename through `nx-types`, `nx-api`, `nx-interpreter`, `nx-ffi`, and `bindings/dotnet`
- [x] 1.2 Update HIR/type-analysis documentation, diagnostics, and tests to consistently use `LoweredModule` for the single-file lowered representation

## 2. Introduce module-level artifacts in the analysis pipeline

- [x] 2.1 Add `ModuleArtifact` to the shared analysis layer so one analyzed source file preserves parse metadata, `LoweredModule`, `TypeEnvironment`, diagnostics, and import metadata
- [x] 2.2 Update shared source-analysis entry points to return `ModuleArtifact` and to gate runtime execution on `ModuleArtifact` error diagnostics
- [x] 2.3 Update source-analysis tests to cover successful artifacts, parse-failure artifacts, and caller-provided file identity in diagnostics

## 3. Replace merged library preparation with file-preserving artifacts

- [x] 3.1 Implement `LibraryArtifact` so a library preserves one `ModuleArtifact` per source file together with export indexes, dependency metadata, and library-level diagnostics
- [x] 3.2 Update local library import resolution to build and consume `LibraryArtifact` data without flattening library files into one merged lowered module
- [x] 3.3 Implement `ProgramArtifact` so a resolved program preserves root modules, resolved libraries, whole-program diagnostics, and fingerprints

## 4. Add the resolved runtime program model

- [x] 4.1 Implement `ResolvedProgram` inside `ProgramArtifact` with symbol tables, entrypoint lookup, and module-qualified runtime references
- [x] 4.2 Update interpreter execution and lookup paths to run across multiple `LoweredModule`s through `ResolvedProgram` rather than one merged lowered module
- [x] 4.3 Update snapshot and handler serialization to store module-qualified runtime references and reject incompatible program revisions

## 5. Cut over public prepared/runtime APIs

- [x] 5.1 Replace `PreparedLibrary`-based runtime entry points in `nx-api` with `ModuleArtifact`, `LibraryArtifact`, `ProgramArtifact`, and `ResolvedProgram` terminology and plumbing
- [x] 5.2 Remove the legacy merged prepared-library implementation and any compatibility adapters from `nx-api`
- [x] 5.3 Update `nx-ffi` and `bindings/dotnet` to expose the new artifact/program APIs directly with no backward-compatibility shim layer

## 6. Update behavior tests and documentation

- [x] 6.1 Rewrite runtime, import-resolution, and component lifecycle tests to assert file-preserving artifacts, `ResolvedProgram` execution, and program-specific snapshots
- [x] 6.2 Update public docs, binding docs, and examples to use `LoweredModule`, `ModuleArtifact`, `LibraryArtifact`, `ProgramArtifact`, and `ResolvedProgram`
- [x] 6.3 Run the relevant Rust and .NET test suites and fix any regressions introduced by the terminology and architecture cutover
