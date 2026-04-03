# Review: introduce-module-library-program-artifacts

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, all 5 specs (artifact-model, resolved-program-runtime, source-analysis-pipeline, module-imports, component-runtime-bindings)  
**Reviewed code:** crates/nx-hir/src/{lib,db,import_resolution,lower,records,scope}.rs, crates/nx-types/src/{lib,check,infer}.rs, crates/nx-api/src/{lib,artifacts,eval,component,diagnostics}.rs, crates/nx-interpreter/src/{lib,interpreter,value,resolved_program}.rs, crates/nx-ffi/src/lib.rs, crates/nx-cli/src/{main,codegen,codegen/model,format,json}.rs, bindings/dotnet/src/NxLang.Runtime/{NxRuntime,NxProgramArtifact,Interop/NxNativeLibrary,Interop/NxNativeMethods}.cs, bindings/dotnet/tests/NxLang.Runtime.Tests/{NxEndToEndTests,NxRuntimeComponentTests}.cs

## Findings

### ✅ Verified - RF1 Missing `module_id` field in CLI test fixtures causes compilation failure

- **Severity:** High
- **Evidence:** `Value::ActionHandler` now requires a `module_id: RuntimeModuleId` field (crates/nx-interpreter/src/value.rs:88), but two test sites in the CLI crate construct `ActionHandler` without it:
  - [format.rs:358](crates/nx-cli/src/format.rs#L358): `test_format_action_handler`
  - [json.rs:23](crates/nx-cli/src/json.rs#L23): `test_format_value_json_pretty_action_handler`
- **Recommendation:** Add `module_id: RuntimeModuleId::new(0)` to both `ActionHandler` struct literals. This will require adding `use nx_interpreter::resolved_program::RuntimeModuleId;` (or the appropriate re-export path) to each test module.
- **Fix:** Added `module_id: RuntimeModuleId::new(0)` to both CLI `ActionHandler` test fixtures so `nx-cli` compiles against the new runtime value shape.
- **Verification:** Confirmed both test sites include `module_id` with proper imports. format.rs uses fully qualified `nx_interpreter::RuntimeModuleId::new(0)`, json.rs uses an import at line 15. All required `ActionHandler` fields are present in both locations.

### ✅ Verified - RF2 Unsafe fallback to `RuntimeModuleId::new(0)` when module identity is unknown

- **Severity:** Medium
- **Evidence:** Two locations in the interpreter silently fall back to module ID 0 when `current_module_id` returns `None`:
  - [interpreter.rs:502-504](crates/nx-interpreter/src/interpreter.rs#L502-L504): component initialization snapshot
  - [interpreter.rs:1131-1133](crates/nx-interpreter/src/interpreter.rs#L1131-L1133): action handler construction
  
  This violates the spec requirement that "bare local IDs SHALL NOT be used as the complete identity of a runtime reference" (resolved-program-runtime spec). If no resolved program is bound, the fallback creates snapshots and handlers with a fabricated module ID that may not correspond to any real module in a future program.
- **Recommendation:** Return an error when `current_module_id` returns `None` and a resolved program is expected, rather than silently substituting ID 0. If the bare-module (no resolved program) path is intentionally supported, document it as an invariant and ensure fingerprint validation catches any cross-revision mismatch.
- **Fix:** Replaced both `RuntimeModuleId::new(0)` fallbacks with hard errors when runtime-owned handler or snapshot creation is attempted without a bound `ResolvedProgram`. Bare handler/component tests now run through a single-module `ResolvedProgram`, and `nx-cli run` executes through a resolved `ProgramArtifact` so handler-producing roots still work without relying on synthetic module identity.
- **Verification:** Confirmed both fallback sites now call `require_current_module_id()` (interpreter.rs:521, 1161) which returns `RuntimeErrorKind::ResolvedProgramRequired`. The `nx-cli run` command uses `build_program_artifact_from_source()` and `Interpreter::from_resolved_program()` (main.rs:177). Interpreter tests use a `single_module_runtime()` helper wrapping modules in `ResolvedProgram`. No unsafe `RuntimeModuleId::new(0)` fallbacks remain in production code.

### ✅ Verified - RF3 Fingerprint validation is skipped when interpreter has no resolved program

- **Severity:** Medium
- **Evidence:** [interpreter.rs:875](crates/nx-interpreter/src/interpreter.rs#L875) — `if let Some(program) = self.program.as_ref()` means fingerprint validation only runs when a resolved program is bound. If the interpreter is constructed without one (via `Interpreter::new()` rather than `from_resolved_program()`), any snapshot is silently accepted regardless of its `program_fingerprint`.
  
  The spec states: "dispatch SHALL reject the snapshot as incompatible" when program revisions differ (resolved-program-runtime spec, scenario "Snapshot from a different program revision is rejected").
- **Recommendation:** When no resolved program is bound, either reject all snapshots that carry a program fingerprint, or require that snapshot dispatch always goes through a program-aware interpreter. The current silent pass-through creates a safety gap.
- **Fix:** `decode_component_snapshot` now rejects any snapshot that carries a `program_fingerprint` when the interpreter was created without a `ResolvedProgram`, and the new direct interpreter tests cover both the accepting and rejecting paths.
- **Verification:** Confirmed `decode_component_snapshot` (interpreter.rs:894-918) now has three branches: (1) program bound + fingerprint mismatch → reject, (2) no program + snapshot carries fingerprint → reject with "requires a resolved program runtime", (3) no program + no fingerprint → accept. Test `resolved_program_component_snapshots_accept_matching_program_and_reject_mismatches` covers all three paths including bare-interpreter rejection at lines 289-296.

### ✅ Verified - RF4 No interpreter-level tests for `ResolvedProgram` or cross-module execution

- **Severity:** Medium
- **Evidence:** `grep -r "ResolvedProgram\|from_resolved_program" crates/nx-interpreter/tests/` returns zero matches. All interpreter tests use bare `Interpreter::new()` and single-module execution. The `ResolvedProgram` code path — including cross-module function calls, module-qualified handler creation, and snapshot fingerprint validation — is only tested indirectly through `nx-api` integration tests.
- **Recommendation:** Add direct interpreter tests that construct a `ResolvedProgram` with multiple modules and verify: (1) cross-module function resolution, (2) module-qualified action handler round-trip, (3) snapshot fingerprint acceptance/rejection, (4) component initialization across module boundaries.
- **Fix:** Added [crates/nx-interpreter/tests/resolved_program.rs](crates/nx-interpreter/tests/resolved_program.rs) with direct multi-module `ResolvedProgram` coverage for cross-module entrypoint execution, module-qualified action handlers, imported component initialization, matching fingerprint dispatch, mismatched fingerprint rejection, and bare-interpreter rejection of program-stamped snapshots.
- **Verification:** File exists with two comprehensive tests backed by a `build_resolved_program()` helper that constructs a genuine two-module program (root + UI) with distinct `RuntimeModuleId`s and cross-module imports. Test 1 covers cross-module function calls (`calcDouble` from UI) and module-qualified action handler round-trip. Test 2 covers component initialization across modules, matching fingerprint dispatch, mismatched fingerprint rejection (0xCAFE_BABE vs 0xDEAD_BEEF), and bare-interpreter rejection of program-stamped snapshots. All claimed scenarios are covered.

### ✅ Verified - RF5 FFI `parse_program_artifact` returns `&'static` lifetime that misrepresents actual ownership

- **Severity:** Low
- **Evidence:** [nx-ffi/src/lib.rs:173-182](crates/nx-ffi/src/lib.rs#L173-L182) — the function signature claims `&'static ProgramArtifact`, but the actual lifetime is bounded by the heap-allocated `NxProgramArtifactHandle`. This is currently safe because the reference is only used within `panic::catch_unwind` closures and never escapes, but it is unsound in principle — a future refactor could store or return this reference, causing use-after-free.
- **Recommendation:** Use a proper lifetime parameter `fn parse_program_artifact<'a>(handle_ptr: *const NxProgramArtifactHandle) -> Result<&'a ProgramArtifact, String>` to express the true ownership relationship. This is a zero-cost change that prevents future misuse.
- **Fix:** Replaced the helper with `with_program_artifact(...)`, which keeps the borrow scoped to an internal closure instead of returning any reference from the raw handle.
- **Verification:** Confirmed `parse_program_artifact` with `&'static` return is completely removed. Replaced by `with_program_artifact(handle_ptr, |artifact| { ... })` (nx-ffi/src/lib.rs:174-184) which accepts a closure and scopes the `&ProgramArtifact` borrow internally. All three call sites (eval at line 317, component init at line 404, component dispatch at line 503) use the closure pattern. The reference cannot escape — the closure returns a computed `Result<T, String>`, never the reference itself.

## Summary
- The architectural changes are well-designed and correctly implement the artifact model spec. `LoweredModule` rename is complete across all crates, `ModuleArtifact`/`LibraryArtifact`/`ProgramArtifact` structures match design requirements, and the nx-api/FFI/.NET layers properly expose the new APIs with good memory safety and test coverage.
- **5 findings fixed:** RF1, RF2, RF3, RF4, RF5.
- **0 findings remain open.**
