# Review: nx-api-source-analysis-pipeline

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/source-analysis-pipeline/spec.md, specs/component-runtime-bindings/spec.md  
**Reviewed code:** crates/nx-types/src/check.rs, crates/nx-types/src/infer.rs, crates/nx-types/src/lib.rs, crates/nx-api/src/eval.rs, crates/nx-api/src/component.rs, crates/nx-api/src/lib.rs, crates/nx-api/Cargo.toml, crates/nx-ffi/tests/ffi_smoke.rs, bindings/dotnet/tests/NxLang.Runtime.Tests/NxRuntimeErrorTests.cs, bindings/dotnet/README.md, nx-rust-plan.md

## Findings

### ✅ Verified - RF1 Library import resolution silently dropped from source-driven API entry points
- **Severity:** Medium
- **Evidence:** The old `eval.rs:lower_source_module` checked whether `file_name` pointed to a real file and, if so, passed it as `source_path` for `link_local_libraries`. The new `analyze_source_module` calls `nx_types::analyze_str`, which always passes `source_path: None` to `analyze_parse_result` (check.rs:64). Unlike `nx_hir::lower_source_module` (lib.rs:102-114), which returns a `library-imports-require-path` diagnostic when imports are present but no path is given, the new analysis path silently skips library linking with no error. Any caller using `eval_source`, `initialize_component_source`, or `dispatch_component_actions_source` with source that has `import` statements will get confusing downstream errors (undefined identifiers) instead of a clear diagnostic.
- **Recommendation:** Either (a) add the same "imports require a file path" guard in `analyze_parse_result` when `source_path` is None and the module has imports, or (b) add an `analyze_str_with_path` variant that `nx-api` can call with a path when `file_name` resolves to an existing file. The current silent skip is the worst outcome — an explicit error or a working path would both be better.
- **Fix:** Added an explicit `library-imports-require-path` diagnostic to string-based analysis without a source path, introduced `analyze_str_with_path`, and updated `nx-api` to use the path-aware analysis flow when `file_name` points to a real file. Added regression tests for both the virtual-path failure case and successful on-disk import resolution.
- **Verification:** Confirmed. `analyze_str_with_path` (check.rs:69-76) correctly passes the source path for library linking. `analyze_string_parse_result` (check.rs:157-165) emits `library-imports-require-path` when no path is given and imports exist, and handles `link_local_libraries` errors via `library_load_error_diagnostic`. `analyze_source_module` in eval.rs:29-34 dispatches to the path-aware variant when `file_name` exists on disk. Three new tests cover the virtual-path failure, on-disk import resolution, and the nx-types layer guard. `analyze_str_with_path` is re-exported from lib.rs. All tests pass.

### ✅ Verified - RF2 Component dispatch round-trip test bypasses the new analysis pipeline for setup
- **Severity:** Low
- **Evidence:** `component.rs:287-298` (`dispatch_component_actions_source_round_trips_effects_and_snapshot`) constructs its state snapshot by calling `lower_hir` and `Interpreter::initialize_component` directly, bypassing `analyze_source_module`. The snapshot is then fed to `dispatch_component_actions_source`, which creates its own module via the full analysis pipeline. If the parser's `SourceId` allocator ever diverges between the two parse calls (e.g., from test ordering or global state), the snapshot and module could become incompatible. The identical pattern appears in `ffi_smoke.rs:317-325`.
- **Recommendation:** Use `initialize_component_source` to obtain the snapshot in these tests, so both setup and dispatch go through the same pipeline. This also validates the full init-then-dispatch lifecycle end-to-end through the public API.
- **Fix:** Updated the `nx-api` and `nx-ffi` round-trip tests to build their setup modules through `nx_types::analyze_str` with the same file name used by dispatch. That moves test setup onto the shared analysis path and keeps `SourceId` derivation aligned with the public runtime entry points.
- **Verification:** Confirmed. `component.rs:287` now calls `analyze_str(source, "component-dispatch.nx")` matching the file name at dispatch (line 310). `ffi_smoke.rs:122-130` replaces `lower_module` with `analyze_module` which delegates to `analyze_str`. Both tests use the shared analysis pipeline for setup, aligning `SourceId` derivation. All tests pass.

## Questions
- None

## Summary
- The core change is well-structured: `analyze_str` in `nx-types` is a clean shared analysis boundary, `check_str`/`check_file` delegate to it properly, and `nx-api` enforces the analyze-then-execute contract correctly.
- Test coverage is thorough across all layers (nx-types, nx-api, nx-ffi, .NET bindings) and the diagnostic aggregation + file-name fidelity scenarios match the spec requirements.
- The main risk is RF1: silent loss of library import resolution for source-driven API callers. RF2 is a test hygiene issue that could cause flaky failures under specific conditions.
