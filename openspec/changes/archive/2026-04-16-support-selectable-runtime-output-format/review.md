# Review: support-selectable-runtime-output-format

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/runtime-output-format/spec.md, specs/component-runtime-bindings/spec.md, specs/dotnet-binding/spec.md
**Reviewed code:**
- [crates/nx-ffi/src/lib.rs](crates/nx-ffi/src/lib.rs)
- [crates/nx-ffi/cbindgen.toml](crates/nx-ffi/cbindgen.toml)
- [crates/nx-ffi/tests/ffi_smoke.rs](crates/nx-ffi/tests/ffi_smoke.rs)
- [bindings/c/nx.h](bindings/c/nx.h)
- [bindings/dotnet/src/NxLang.Runtime/NxOutputFormat.cs](bindings/dotnet/src/NxLang.Runtime/NxOutputFormat.cs)
- [bindings/dotnet/src/NxLang.Runtime/NxRuntime.cs](bindings/dotnet/src/NxLang.Runtime/NxRuntime.cs)
- [bindings/dotnet/src/NxLang.Runtime/Interop/NxNativeMethods.cs](bindings/dotnet/src/NxLang.Runtime/Interop/NxNativeMethods.cs)
- [bindings/dotnet/src/NxLang.Runtime/Interop/NxNativeLibrary.cs](bindings/dotnet/src/NxLang.Runtime/Interop/NxNativeLibrary.cs)
- [bindings/dotnet/tests/NxLang.Runtime.Tests/*.cs](bindings/dotnet/tests/NxLang.Runtime.Tests/)
- [bindings/dotnet/README.md](bindings/dotnet/README.md)

## Findings

### ✅ Verified - RF1 Missing Rust FFI test for JSON success from `nx_eval_program_artifact`
- **Severity:** Medium
- **Evidence:** Task 2.1 requires FFI tests for "source **and program-artifact** evaluation that verify **both MessagePack and JSON success payloads** plus JSON diagnostic payloads on failure." In [crates/nx-ffi/tests/ffi_smoke.rs](crates/nx-ffi/tests/ffi_smoke.rs), source evaluation is tested in both MessagePack (`ffi_msgpack_success_round_trip`) and JSON (`ffi_json_success_round_trip`), and program-artifact evaluation is tested for MessagePack success (`ffi_registry_backed_program_build_reuses_preloaded_library`) and JSON diagnostics on failure (`ffi_eval_program_artifact_returns_json_diagnostics_directly`), but there is no test that exercises `nx_eval_program_artifact` with `NxOutputFormat::Json` on a successful evaluation (e.g., decoding the JSON body via `eval_json_with_program_artifact`, which is defined but never called in a success path).
- **Recommendation:** Add a test that builds a `ProgramArtifact` for `let root() = { 42 }` (or similar), calls `nx_eval_program_artifact` with `NxOutputFormat::Json`, and asserts the UTF-8 JSON payload — mirroring `ffi_json_success_round_trip` but from the artifact entry point.
- **Fix:** Added `ffi_eval_program_artifact_returns_json_success_directly` in [crates/nx-ffi/tests/ffi_smoke.rs](crates/nx-ffi/tests/ffi_smoke.rs) to verify successful JSON output through `nx_eval_program_artifact`.
- **Verification:** Test present at ffi_smoke.rs:350-365. Builds artifact from `let root() = { 42 }`, calls `eval_json_with_program_artifact`, and asserts `json == "42"`. Correct and complete.

### ✅ Verified - RF2 Public `NxOutputFormat` enum lacks defensive guard against unknown integer values
- **Severity:** Low
- **Evidence:** [crates/nx-ffi/src/lib.rs:63-68](crates/nx-ffi/src/lib.rs#L63-L68) defines `NxOutputFormat` as `#[repr(u32)]` with variants `MessagePack = 0` and `Json = 1`. The managed enum in [bindings/dotnet/src/NxLang.Runtime/NxOutputFormat.cs](bindings/dotnet/src/NxLang.Runtime/NxOutputFormat.cs) is also an open `enum : int` so callers can legally pass `(NxOutputFormat)42`. On the Rust side, constructing such a value across FFI and then matching on it is undefined behavior. The serialize functions use exhaustive `match output_format { MessagePack => ..., Json => ... }`, so unknown discriminants would produce UB rather than a clean `InvalidArgument`.
- **Recommendation:** Either (a) validate the discriminant at each entry point (e.g., convert from `u32` and map unknown values to `NxEvalStatus::InvalidArgument`) or (b) document in the public header/managed docs that passing a value not in the enum is UB and have `NxRuntime` reject unknown values before the DllImport call. A header-only comment is the lowest-effort option; validation is the safer one.
- **Fix:** Changed the native value-returning entry points in [crates/nx-ffi/src/lib.rs](crates/nx-ffi/src/lib.rs) to accept raw `u32` output-format values, validate them before use, and return `NxEvalStatus::InvalidArgument` for unknown discriminants. Added `ffi_value_entry_points_reject_unknown_output_format` coverage and regenerated [bindings/c/nx.h](bindings/c/nx.h).
- **Verification:** All four entry points (`nx_eval_source`, `nx_eval_program_artifact`, `nx_component_init_program_artifact`, `nx_component_dispatch_actions_program_artifact`) now accept `u32` and validate via `parse_output_format` (lib.rs:158-160) using `TryFrom<u32>` (lib.rs:70-80). C header correctly reflects `uint32_t output_format`. Test `ffi_value_entry_points_reject_unknown_output_format` (ffi_smoke.rs:600-665) covers all four with discriminant 42. Complete.

### ✅ Verified - RF3 Managed JSON workflow lacks overloads for props-less source evaluation parity
- **Severity:** Low
- **Evidence:** `EvaluateJson` in [bindings/dotnet/src/NxLang.Runtime/NxRuntime.cs:105-136](bindings/dotnet/src/NxLang.Runtime/NxRuntime.cs#L105-L136) provides three overloads (source/fileName, source/buildContext/fileName, programArtifact). By contrast, `InitializeComponentJson` and `DispatchComponentActionsJson` each come in six overloads (props-less × 3 + typed props × 3). This asymmetry is fine, but the README and error tests do not exercise `EvaluateJson(source, buildContext)` or `EvaluateJson(programArtifact)` — only `EvaluateJson(source)`. Task 4.1 asks tests to cover "`JsonElement` evaluation results" generally, and test coverage here is thin on the build-context and program-artifact JSON paths.
- **Recommendation:** Add a managed test for `EvaluateJson(NxProgramArtifact)` and ideally also for `EvaluateJson(source, NxProgramBuildContext)` to confirm the JSON path works through all three entry points. A single artifact-based test would materially reduce the risk of a future regression being caught only in Rust tests.
- **Fix:** Added `EvaluateJson_WithBuildContext_ReturnsCorrectJsonElement` and `EvaluateJson_WithProgramArtifact_ReturnsCorrectJsonElement` in [bindings/dotnet/tests/NxLang.Runtime.Tests/NxRuntimeBasicTests.cs](bindings/dotnet/tests/NxLang.Runtime.Tests/NxRuntimeBasicTests.cs).
- **Verification:** Both tests present at NxRuntimeBasicTests.cs:48-68. `EvaluateJson_WithBuildContext` creates a registry/build-context and asserts `GetInt32() == 42`. `EvaluateJson_WithProgramArtifact` builds via `NxProgramArtifact.Build` and asserts the same. Both overloads now covered. Complete.

### 🔴 Open - RF4 Unrelated codegen changes present in working tree
- **Severity:** Low
- **Evidence:** `git status` shows modifications in [crates/nx-cli/src/codegen.rs](crates/nx-cli/src/codegen.rs), [crates/nx-cli/src/codegen/languages/csharp.rs](crates/nx-cli/src/codegen/languages/csharp.rs), and [crates/nx-cli/src/codegen/model.rs](crates/nx-cli/src/codegen/model.rs). None of these files are referenced in proposal.md, design.md, tasks.md, or any of the spec deltas. The diff contents (external component props codegen, union discriminators) match the concerns of the already-committed `6de51b6 Allow external components to declare host-managed state` work and not the output-format change.
- **Recommendation:** Before archiving this change, either (a) land the codegen changes as a separate commit/change, or (b) confirm they are meant to be part of a different in-flight change and exclude them from this one. Do not bundle them into the output-format change's commit.
- **Status:** Left open. These existing workspace changes are unrelated to `support-selectable-runtime-output-format` and were intentionally not modified in this review-fix pass.

### ✅ Verified - RF5 Unused `JsonOptions` field
- **Severity:** Low
- **Evidence:** [bindings/dotnet/src/NxLang.Runtime/NxRuntime.cs:21](bindings/dotnet/src/NxLang.Runtime/NxRuntime.cs#L21) defines `private static readonly JsonSerializerOptions JsonOptions = new();`. It is used only in `CreateEvaluationExceptionFromJson` for `JsonSerializer.Deserialize<NxDiagnostic[]>(payload, JsonOptions)`. Every other JSON path uses `JsonDocument.Parse` directly without options. Since `JsonOptions` is the default instance, it adds no value — `JsonSerializer.Deserialize<T>(payload)` is equivalent.
- **Recommendation:** Drop `JsonOptions` and pass no options to `JsonSerializer.Deserialize`, or — if you anticipate needing shared configuration later — leave a short comment explaining the intent. As written it's dead weight.
- **Fix:** Removed the redundant `JsonOptions` field from [bindings/dotnet/src/NxLang.Runtime/NxRuntime.cs](bindings/dotnet/src/NxLang.Runtime/NxRuntime.cs) and now deserialize JSON diagnostics with the default serializer configuration directly.
- **Verification:** No references to `JsonOptions` remain in NxRuntime.cs. `CreateEvaluationExceptionFromJson` now calls `JsonSerializer.Deserialize<NxDiagnostic[]>(payload)` without options. Complete.

## Questions
- Should `nx_build_program_artifact` and `nx_load_library_into_registry` also accept `NxOutputFormat`, or is the decision to keep them MessagePack-only final? Design.md calls this out as an open question; worth resolving before archive, since build/load failures in a JSON-first host today still force the caller to decode MessagePack diagnostics.

## Summary
Implementation matches the design well: the native ABI adds a clean `NxOutputFormat` enum, the serialization branches directly from NxValue/diagnostics without going through MessagePack first, the converter exports are fully removed, and the managed API offers a clear split between MessagePack generics and JSON/`JsonElement` convenience APIs. RF1, RF2, RF3, and RF5 are fixed in this pass. RF4 remains open because the unrelated `nx-cli` working-tree changes are still present and should stay out of this change's archive/commit.
