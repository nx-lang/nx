# Review: introduce-program-build-context

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, all 8 delta specs (artifact-model, library-registry, program-build-context, module-imports, source-analysis-pipeline, component-runtime-bindings, dotnet-binding, enum-values)  
**Reviewed code:** crates/nx-api/src/artifacts.rs, crates/nx-hir/src/lib.rs, crates/nx-types/src/check.rs, crates/nx-types/src/lib.rs, crates/nx-ffi/src/lib.rs, crates/nx-ffi/cbindgen.toml, crates/nx-ffi/tests/ffi_smoke.rs, bindings/c/nx.h, bindings/dotnet/src/NxLang.Runtime/ (NxLibraryRegistry.cs, NxProgramBuildContext.cs, NxProgramArtifact.cs, NxRuntime.cs, Interop/NxNativeLibrary.cs, Interop/NxNativeMethods.cs), bindings/dotnet/tests/NxLang.Runtime.Tests/NxEndToEndTests.cs, crates/nx-cli/src/main.rs, crates/nx-interpreter/src/

## Findings

### ✅ Verified - RF1 C header is stale and cbindgen config omits new FFI exports
- **Severity:** High
- **Evidence:** `crates/nx-ffi/cbindgen.toml` `[export].include` lists only the pre-change functions (`nx_eval_source`, `nx_component_init`, `nx_component_dispatch_actions`, utility functions). All new entry points are missing: `nx_create_library_registry`, `nx_free_library_registry`, `nx_load_library_into_registry`, `nx_create_program_build_context`, `nx_free_program_build_context`, `nx_build_program_artifact`, `nx_free_program_artifact`, `nx_eval_program_artifact`, `nx_component_init_program_artifact`, `nx_component_dispatch_actions_program_artifact`. The handle types `NxProgramArtifactHandle`, `NxLibraryRegistryHandle`, `NxProgramBuildContextHandle` are also missing from the export list. As a result, `bindings/c/nx.h` does not expose any of the new APIs and still declares `NX_FFI_ABI_VERSION 2` while the Rust source defines version 6 at `crates/nx-ffi/src/lib.rs:20`. Task 4.1 requires replacing standalone library-artifact host handles with registry and build-context entry points and bumping the ABI version — the Rust FFI implementation is correct, but the C header was never regenerated.
- **Recommendation:** Update `crates/nx-ffi/cbindgen.toml` to include all new exported functions and handle types, then regenerate the header via `tools/generate-nx-ffi-header.sh`. Verify the resulting `bindings/c/nx.h` declares the current ABI version and exposes all three handle types plus the new entry points.
- **Fix:** Expanded `crates/nx-ffi/cbindgen.toml`, made the public FFI handle tags opaque in Rust so `cbindgen` emits forward declarations, and regenerated `bindings/c/nx.h` with the current registry/build-context/program-artifact entry points.
- **Verification:** Confirmed `cbindgen.toml` includes all 3 handle types and all new FFI functions, `bindings/c/nx.h` declares `NX_FFI_ABI_VERSION 7` with correct opaque forward declarations, and `crates/nx-ffi/src/lib.rs` sets `NX_FFI_ABI_VERSION: u32 = 7`.

### ✅ Verified - RF2 Program fingerprint hashes library fingerprints before sorting
- **Severity:** Medium
- **Evidence:** In `crates/nx-api/src/artifacts.rs:470-478`, `selected_program_libraries` returns libraries in dependency-graph traversal order. Their fingerprints are hashed (line 472: `hasher.write_u64(library.fingerprint)`) before the `libraries.sort_by(...)` at line 478. The hash therefore depends on traversal order rather than canonical sorted order. While the traversal is deterministic for the same inputs today, any upstream change to `selected_program_libraries` traversal order (e.g., different `dependency_roots` storage order) would silently change program fingerprints for identical library sets.
- **Recommendation:** Move `libraries.sort_by(|lhs, rhs| lhs.root_path.cmp(&rhs.root_path))` before the fingerprint hashing loop so the fingerprint is computed over a canonically sorted library list.
- **Fix:** Moved the canonical `root_path` sort ahead of the fingerprint hashing loop in `build_program_artifact_from_source`, so program fingerprints now hash a stable library order.
- **Verification:** Confirmed at line 978: `libraries.sort_by(|lhs, rhs| lhs.root_path.cmp(&rhs.root_path))` now precedes the fingerprint hashing loop at lines 979–981. Canonical sort order is guaranteed before any `hasher.write_u64(library.fingerprint)` call.

### ✅ Verified - RF3 Library module analysis no longer uses the legacy disk-loading resolver
- **Severity:** Medium
- **Evidence:** The legacy `analyze_library_module` path is gone. `LibraryRegistry::load_library_from_directory_internal` now preloads dependency roots into the registry, `build_library_artifact_with_registry` prepares each file by applying registry-backed dependency interfaces, and library-file analysis finishes through `analyze_prepared_module` instead of `resolve_local_library_imports`.
- **Recommendation:** None further for this finding.
- **Fix:** Migrated library analysis in `crates/nx-api/src/artifacts.rs` to a registry-backed preparation flow, deleted the old `analyze_library_module` path, and removed the obsolete public import-resolution module from `nx-hir`.
- **Verification:** Confirmed `analyze_library_module` no longer exists anywhere in `crates/nx-api/src`. Library analysis flows through `build_library_artifact_with_registry` (line 680). `resolve_local_library_imports` is absent from `crates/nx-hir/src/lib.rs` — the obsolete public surface has been removed.

### ✅ Verified - RF4 nx-types no longer loads libraries from disk during analysis
- **Severity:** Medium
- **Evidence:** `crates/nx-types/src/check.rs` now analyzes parse results by lowering once and handing the preserved plus prepared modules to `analyze_prepared_module`. The path-aware `analyze_str_with_path` entry point is removed, and `crates/nx-types/src/lib.rs` re-exports the prepared-module seam instead of a disk-loading helper.
- **Recommendation:** None further for this finding.
- **Fix:** Refactored `nx-types` so source/file helpers parse and lower only, added the public prepared-module analysis seam, and moved all registry/build-context import preparation policy up into `nx-api`.
- **Verification:** Confirmed no `read_dir`, `fs::`, `PathBuf`, `resolve_local_library_imports`, or `analyze_str_with_path` patterns remain in `crates/nx-types/src`. `analyze_prepared_module` is public at `crates/nx-types/src/check.rs:124`.

### 🔴 Open - RF5 Raw FFI stale-handle behavior remains unsupported even though null-handle coverage is now in place
- **Severity:** Low
- **Evidence:** The managed .NET binding now routes native ownership through `SafeHandle`, and `crates/nx-ffi/tests/ffi_smoke.rs` now covers null-handle rejection for `nx_load_library_into_registry`, `nx_create_program_build_context`, `nx_eval_program_artifact`, `nx_component_init_program_artifact`, and `nx_component_dispatch_actions_program_artifact`. However, the raw native free functions still reconstruct ownership with `Box::from_raw(...)`, so true double-free or use-after-free on a non-null stale pointer remains undefined behavior rather than a recoverable `InvalidArgument` path.
- **Recommendation:** Keep the new null-handle checks and managed `SafeHandle` wrappers, but do not promise raw stale-handle safety unless the native ABI moves to a validated handle table or similar generation-checked indirection.
- **Status:** Partially addressed in this pass: null-handle behavior is now tested and managed callers are protected by `SafeHandle`, but raw C-level stale-handle safety still requires a larger native handle redesign.

### ✅ Verified - RF6 Circular library dependency coverage and diagnostics are now explicit
- **Severity:** Low
- **Evidence:** The original registry loader guarded against recursive re-entry during dependency loading, but there was no dedicated regression test proving an `A -> B -> A` library cycle failed cleanly, and the error text did not identify the dependency chain that caused the cycle.
- **Recommendation:** Add explicit coverage for mutually importing libraries and include the discovered library cycle chain in the diagnostic so hosts can identify the offending roots quickly.
- **Fix:** Added `library_registry_rejects_circular_library_dependencies` in `crates/nx-api/src/artifacts.rs`, which creates two temp libraries that import each other, asserts the load fails, and verifies the registry does not retain partial snapshots. The loader now tracks the active recursion stack and renders the full canonical cycle chain in the error message.
- **Verification:** Confirmed test `library_registry_rejects_circular_library_dependencies` at artifacts.rs:2030–2089 creates mutually importing libraries, asserts a single diagnostic containing "Circular library dependency detected" and the full `a -> b -> a` canonical chain, and verifies neither library is retained in the registry. The cycle detection logic at artifacts.rs:301–309 uses `loading_stack` to track active recursion and `circular_library_dependency_message` at artifacts.rs:841–846 renders the chain.

## Questions
- None.

## Summary
- The core architecture is well-implemented: `LibraryRegistry` correctly owns snapshots, `ProgramBuildContext` is build-time only, program artifacts survive context release, runtime module IDs are assigned at program-build time, and the .NET managed bindings now route native ownership through `SafeHandle`.
- The new `apply_build_context_imports` path for program root modules is clean and correctly synthesizes items from interface metadata instead of copying HIR.
- **RF1** and **RF2** are fixed in this pass: the generated C header now exposes the new ABI-6 registry/build-context surface, and program fingerprints now hash libraries in canonical order.
- **RF3** and **RF4** are now fixed as well: library analysis is prepared through the registry-backed interface path, `nx-types` no longer performs disk-backed import loading, and the obsolete public `resolve_local_library_imports` surface has been removed.
- **RF6** is now fixed as well: circular library dependencies have dedicated coverage, the error reports the discovered cycle chain, and failed loads leave no partial registry snapshots behind.
- Review status for the original six findings: 5 fixed (`RF1`-`RF4`, `RF6`), 1 remains open (`RF5`).

## New Findings Discovered During 2026-04-05 00:11 Review

### ✅ Verified - RF7 Double source analysis in `load_program_artifact_from_source`
- **Severity:** Medium
- **Evidence:** The original source-loading flow pre-analyzed the root module and then immediately rebuilt the same source into a `ProgramArtifact`, causing program build, eval, and source-based component convenience paths to parse, lower, resolve imports, and type-check the same source twice.
- **Recommendation:** Either (a) have `load_program_artifact_from_source` use the already-analyzed artifact to construct the `ProgramArtifact` without re-analyzing, or (b) remove the pre-analysis step and let `build_program_artifact_from_source` handle error-diagnostic surfacing in one pass. The component paths should be updated in parallel.
- **Fix:** Removed the pre-analysis step from the `load_*`, `eval_source`, and source-based component flows so they now build and surface diagnostics through a single `build_source_program_artifact` pass.
- **Verification:** Confirmed all four flows now delegate directly: `load_program_artifact_from_source` delegates to `build_source_program_artifact` in a single call; `eval_source` calls `load_program_artifact_from_source`; `initialize_component_source` and `dispatch_component_actions_source` both call `build_source_program_artifact` directly. No pre-analysis step exists in any path.

### ✅ Verified - RF8 Program library selection now reuses direct-import resolution and surfaces incomplete dependency closure
- **Severity:** Medium
- **Evidence:** `crates/nx-api/src/artifacts.rs:1027-1092` now routes root-module preparation through `prepare_root_module_with_context`, which captures successful direct import resolution from `apply_build_context_imports` and passes that transient selection into `selected_program_libraries`. `crates/nx-api/src/artifacts.rs:1142-1350` shows that import preparation now returns the exact direct libraries that analysis accepted, and `crates/nx-api/src/artifacts.rs:1376-1430` expands the transitive closure from that selection instead of re-looking up direct imports through the raw registry. When a dependency root recorded in a selected snapshot is missing, `crates/nx-api/src/artifacts.rs:1886-1912` now emits `library-dependency-closure-incomplete` instead of silently dropping it.
- **Recommendation:** None further for this finding.
- **Fix:** Program construction now uses one shared architecture for root-import analysis and library selection: direct imports are resolved once against the supplied `ProgramBuildContext`, `ProgramArtifact.libraries` is seeded from those resolved direct imports, and the transitive closure is expanded from snapshot dependency metadata. This removes the old policy split where analysis respected build-context visibility but program assembly independently re-queried the registry. Hidden direct libraries no longer leak into the assembled program, and incomplete transitive dependency closure is surfaced as a build diagnostic.
- **Verification:** Confirmed `prepare_root_module_with_context` (artifacts.rs:1051) calls `apply_build_context_imports` at line 1076 which returns `Vec<ResolvedBuildContextImport>`, then passes those direct imports to `selected_program_libraries` at line 1078. `selected_program_libraries` (artifacts.rs:1384) accepts `direct_imports: &[ResolvedBuildContextImport]` and seeds its queue from those resolved imports at lines 1397–1404. When a transitive dependency is missing from the registry, `incomplete_library_dependency_closure_diagnostic` is emitted at lines 1414–1419.

### ✅ Verified - RF9 `NxProgramBuildContext` does not hold a reference to its parent `NxLibraryRegistry`
- **Severity:** Low
- **Evidence:** In [NxProgramBuildContext.cs](bindings/dotnet/src/NxLang.Runtime/NxProgramBuildContext.cs), the `Create` method calls `registry.DangerousGetHandle()` to obtain the native handle and passes it to `nx_create_program_build_context`, but does not retain a reference to the managed `NxLibraryRegistry` object. If the caller disposes or finalizes the `NxLibraryRegistry` before the `NxProgramBuildContext` is used, the Rust-side `ProgramBuildContext` still holds a valid `LibraryRegistry` clone (because the Rust `ProgramBuildContext` stores its own `LibraryRegistry` via `Arc`), so this is **not** a memory safety bug in practice. However, from a managed API design perspective, the `NxProgramBuildContext` could hold a reference to prevent the caller from accidentally disposing the registry while a build context appears active, which would be confusing during debugging.
- **Recommendation:** Consider storing the `NxLibraryRegistry` reference inside `NxProgramBuildContext` for clarity, even though the Rust side handles this correctly via `Arc`.
- **Fix:** `NxProgramBuildContext` now retains the parent `NxLibraryRegistry` reference in managed code, which keeps the registry object alive for the lifetime of the build context while preserving the Rust-side `Arc` ownership model.
- **Verification:** Confirmed `NxProgramBuildContext` stores `private readonly NxLibraryRegistry _registry;` (line 16) and the constructor accepts and retains the registry reference (lines 19–21). The `Create` factory passes the registry through to the constructor.

### ✅ Verified - RF10 Native component FFI is now artifact-first
- **Severity:** Low
- **Evidence:** The public native component ABI now exposes only `nx_component_init_program_artifact` and `nx_component_dispatch_actions_program_artifact`, while `nx_build_program_artifact` accepts a `NxProgramBuildContextHandle` and rejects null handles. This makes the supported native workflow explicit: preload libraries into a registry, create a build context, build a program artifact, then run component lifecycle calls against that artifact.
- **Recommendation:** None further for this finding.
- **Fix:** Removed `nx_component_init` and `nx_component_dispatch_actions` from `crates/nx-ffi`, from the cbindgen export list, and from `bindings/c/nx.h`; updated `nx_build_program_artifact` to require a non-null build-context handle; rewrote FFI smoke tests around the artifact-first native workflow; and updated the managed .NET source-based component convenience helpers to build transient `NxProgramArtifact`s internally before calling the native program-artifact component APIs.
- **Verification:** Confirmed old source-based `nx_component_init` and `nx_component_dispatch_actions` are absent from `crates/nx-ffi/src/lib.rs`, `cbindgen.toml`, and `bindings/c/nx.h` (only `_program_artifact` variants remain). `nx_build_program_artifact` rejects null `build_context_ptr` with `NxEvalStatus::InvalidArgument` at lib.rs:327–329. .NET managed component helpers in `NxRuntime.cs` now build transient `NxProgramArtifact`s before calling native artifact-based APIs.

## Questions
- None.

## Summary Update
- The 4 new findings (RF7–RF10) are now all verified.
- Total findings: 10. Current status: 8 verified (`RF1`, `RF2`, `RF3`, `RF4`, `RF6`, `RF7`, `RF8`, `RF9`, `RF10`), 1 open (`RF5`).

## New Findings Discovered During 2026-04-05 14:25 Verification

### ✅ Verified - RF11 `apply_build_context_imports` silently returns when root path canonicalization fails
- **Severity:** Low
- **Evidence:** `crates/nx-api/src/artifacts.rs:1074-1075` — `apply_build_context_imports` calls `fs::canonicalize(root_path)` and if it fails (permission denied, nonexistent path, filesystem error), the function silently returns without adding any diagnostic. The module's import declarations remain present but unresolved, causing downstream type analysis to report undefined-symbol errors rather than a specific "failed to resolve import root path" diagnostic. The most common case (virtual/synthetic file names like `"input.nx"`) is benign since those modules typically have no local library imports, but real filesystem failures would produce misleading error messages.
- **Recommendation:** When canonicalization fails and the module has imports that reference local library paths, add a diagnostic indicating that import resolution was skipped because the source file path could not be resolved.
- **Fix:** `apply_build_context_imports` now adds an explicit lowering diagnostic when source-path canonicalization fails in the presence of local library imports, and `crates/nx-api/src/artifacts.rs` now has a regression test covering that previously silent failure path.
- **Verification:** Confirmed at artifacts.rs:1136–1154: canonicalization failure now checks `module.imports` for local library paths and adds a diagnostic "Local library import resolution was skipped because source file path '...' could not be resolved: ...". Regression test `apply_build_context_imports_reports_unresolved_source_path_for_local_imports` at artifacts.rs:2162–2186 passes a non-existent path with a local import and asserts the diagnostic message.

## Final Verification Summary
- Total findings: 11. Current status: 10 verified (`RF1`, `RF2`, `RF3`, `RF4`, `RF6`, `RF7`, `RF8`, `RF9`, `RF10`, `RF11`), 1 open (`RF5`).
