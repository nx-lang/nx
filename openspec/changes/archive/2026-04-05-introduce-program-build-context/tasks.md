## 1. Library Registry And Snapshot Graph

- [x] 1.1 Introduce `LibraryRegistry` in `nx-api` as the owner of analyzed `LibraryArtifact` snapshots keyed by canonical root path.
- [x] 1.2 Teach registry loading to analyze libraries without any program build and to maintain the dependency graph between loaded library snapshots.
- [x] 1.3 Extend `LibraryArtifact` with the export and interface metadata needed for dependent analysis while keeping it a local library snapshot with dependency metadata only.

## 2. Registry-Backed Analysis And Program Construction

- [x] 2.1 Replace copied imported HIR in analysis paths with registry-backed import resolution over loaded library export and interface metadata.
- [x] 2.2 Introduce `ProgramBuildContext` as a registry-backed build scope that selects visible libraries for one build or tenant.
- [x] 2.3 Update source analysis and program-building entry points to resolve imports through `ProgramBuildContext` and report missing libraries instead of silently loading them from disk.
- [x] 2.4 Update `ProgramArtifact` and `ResolvedProgram` assembly so program artifacts preserve the selected library snapshots, fingerprint revisions correctly, and assign runtime module IDs only at program-build time.

## 3. Runtime Convenience APIs

- [x] 3.1 Update source-based evaluation and component convenience flows to build transient `ProgramArtifact`s against a caller-supplied registry-backed `ProgramBuildContext`.
- [x] 3.2 Add Rust tests covering library preload without a program, repeated builds across multiple build contexts, missing-library failures, and execution after build-context release.

## 4. FFI And .NET Bindings

- [x] 4.1 Replace standalone library-artifact host handles in `nx-ffi` with library-registry and build-context create/free/load/build entry points and bump the ABI version.
- [x] 4.2 Replace `NxLibraryArtifact` with `NxLibraryRegistry` plus `NxProgramBuildContext` in the managed binding and update managed program-building APIs accordingly.
- [x] 4.3 Add or update FFI/.NET tests and documentation for the preload/analyze/build/execute workflow, including shared-library reuse across build contexts and missing-library failures.

## 5. Enum Value Convention Verification

- [x] 5.1 Verify the already-present unstaged enum-member `snake_case` changes across parser/lowering, typing, runtime formatting, host value conversion, code generation, grammar tests, examples, and docs; do not implement new enum-casing behavior as part of this task unless verification exposes a concrete gap.
