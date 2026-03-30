# Review: add-component-runtime-support

## Bugs

- [x] **R-01**: .NET `DispatchComponentActionsToJson` passes raw binary `stateSnapshot` bytes to the Rust FFI, but `nx_component_dispatch_actions_json` expects base64-encoded UTF-8 text. The Rust side calls `slice_to_str` then `BASE64_STANDARD.decode`, so passing raw msgpack bytes will fail with an "invalid utf-8" error. The JSON dispatch method in `NxRuntime.cs` needs to base64-encode the snapshot before passing it to the native callback, or the dispatch helper needs a separate code path for the JSON variant.
  - [NxRuntime.cs:230-244](bindings/dotnet/src/NxLang.Runtime/NxRuntime.cs#L230-L244) (DispatchComponentActionsToJson)
  - [NxRuntime.cs:375-404](bindings/dotnet/src/NxLang.Runtime/NxRuntime.cs#L375-L404) (InvokeComponentDispatchNativeCall)
  - [nx-ffi lib.rs:475-484](crates/nx-ffi/src/lib.rs#L475-L484) (Rust expects base64)
  - Status: Fixed, then superseded. The intermediate JSON dispatch fix landed, and the runtime has since been simplified to a canonical MessagePack-only FFI with debug-only MessagePack-to-JSON converters, so this specific transport bug no longer applies to the current public API.

- [x] **R-02**: `from_nx_value` in `nx-api/src/value.rs` does not handle `ActionHandler` records. `to_nx_value` encodes `Value::ActionHandler` as an `NxValue::Record { type_name: Some("ActionHandler"), ... }`, but `from_nx_value` converts any Record back to `Value::Record`, losing the handler semantics. This isn't hit in the current lifecycle (handlers flow through the opaque snapshot, not through NxValue), but the asymmetry means any code path that round-trips an element tree containing handlers through NxValue will silently corrupt handler data. Fixed by making `from_nx_value` fallible, explicitly rejecting `ActionHandler` host input, and rejecting declared component records at the source-based component API boundary.
  - [value.rs:43-64](crates/nx-api/src/value.rs#L43-L64) (to_nx_value for ActionHandler)
  - [value.rs:69-89](crates/nx-api/src/value.rs#L69-L89) (from_nx_value — no ActionHandler case)
  - Status: Fixed. `from_nx_value` now fails explicitly for `ActionHandler`, and source-based component APIs reject component-typed host input instead of silently degrading it.

## Missing Test Coverage

- [ ] **R-03**: No .NET test exercises the handler effect path through the managed binding. `DispatchComponentActions_WithPersistedStateSnapshot_SucceedsAcrossCalls` initializes without handler bindings and asserts `Assert.Empty(dispatchResult.Effects)`. A test that binds an `onSearchSubmitted` handler at the source level and proves that effects round-trip through the .NET typed API would be more valuable.
  - [NxRuntimeComponentTests.cs:72-104](bindings/dotnet/tests/NxLang.Runtime.Tests/NxRuntimeComponentTests.cs#L72-L104)
  - Status: Follow-up discussion needed. After R-02, the public managed API intentionally cannot construct handler-bound component props or snapshots from host input, so adding this coverage cleanly would require either a test-only hook or a broader API decision.

- [x] **R-04**: No test dispatches an undeclared action type. `validate_component_action` returns `UnsupportedComponentAction` when the action's type_name doesn't match any emit, but no test at any layer (interpreter, API, FFI, .NET) exercises this path. Add a test that dispatches an action with a mismatched type_name and asserts the correct error.
  - [interpreter.rs:555-578](crates/nx-interpreter/src/interpreter.rs#L555-L578) (validate_component_action)
  - Status: Fixed. Added dispatch coverage for undeclared action types in the interpreter test suite, and the managed binding tests now also assert the surfaced diagnostic path.

- [x] **R-05**: No test covers the .NET `DispatchComponentActionsToJson` path. This is directly related to R-01 — the bug hasn't been caught because there's no test for JSON-based dispatch. Add a test that initializes via JSON, persists the snapshot, and dispatches via JSON.
  - [NxRuntimeComponentTests.cs](bindings/dotnet/tests/NxLang.Runtime.Tests/NxRuntimeComponentTests.cs)
  - Status: Fixed, then superseded. JSON lifecycle coverage was added when those APIs existed, and the runtime has since removed JSON init/dispatch entry points in favor of canonical MessagePack transport plus debug-only conversion helpers.

- [x] **R-06**: No interpreter test covers dispatching with an action value that is not a Record (e.g., passing a string or int as an action). `validate_component_action` has the check but it's untested.
  - [interpreter.rs:560-566](crates/nx-interpreter/src/interpreter.rs#L560-L566)
  - Status: Fixed. Added interpreter coverage for dispatching a non-record action value and asserting the `TypeMismatch` error.

## Design / Implementation Observations

- [x] **R-07**: `lower_component_fields` in `lower.rs` is a trivial wrapper around `lower_record_fields_from_node` with no additional logic. Consider calling `lower_record_fields_from_node` directly from `predeclare_component` where `lower_component_fields` is used, to reduce indirection.
  - [lower.rs:347-349](crates/nx-hir/src/lower.rs#L347-L349)
  - Status: Fixed. Removed the wrapper and now call `lower_record_fields_from_node` directly from `predeclare_component`.

- [x] **R-08**: FFI entry points (`nx_component_init_msgpack`, `nx_component_init_json`, `nx_component_dispatch_actions_msgpack`, `nx_component_dispatch_actions_json`) repeat the same `catch_unwind → match Ok/Err → serialize → write buffer` boilerplate pattern that already exists in `nx_eval_source_msgpack/json`. Six copies of this pattern exist in `lib.rs`. Consider extracting a helper (e.g., `ffi_entry_point<F>(out_buffer, f: F) -> NxEvalStatus`) that handles the `catch_unwind`, error mapping, and buffer writing, with the caller supplying just the closure that produces `(NxEvalStatus, payload)`.
  - [lib.rs:140-518](crates/nx-ffi/src/lib.rs#L140-L518)
  - Status: Fixed. Extracted narrow helpers for buffer initialization and MessagePack/JSON entry completion, while keeping each entry point's input parsing and success serialization explicit.

- [ ] **R-09**: `decode_component_snapshot` takes `module: &Module` but only uses it for `expr_count()` validation during ActionHandler deserialization and for recursive deserialization calls. The module dependency makes the signature heavier than needed. Consider whether a simple `expr_count: usize` parameter would suffice, or leave as-is if module access will be needed for future snapshot validation.
  - [interpreter.rs:606-654](crates/nx-interpreter/src/interpreter.rs#L606-L654)
  - Status: Follow-up discussion recommended. A lighter signature is possible today, but keeping `&Module` preserves room for future snapshot validation without another API churn point.

- [x] **R-10**: `dispatch_component_actions_with_limits` clones `limits` on each action handler invocation in the loop (`limits.clone()`). Since `ResourceLimits` is `Copy`-able (it's two `usize` fields), this should use `Copy` semantics rather than `Clone` to signal intent. Verify `ResourceLimits` derives `Copy`; if not, add it.
  - [interpreter.rs:330-335](crates/nx-interpreter/src/interpreter.rs#L330-L335)
  - Status: Fixed. `ResourceLimits` now derives `Copy`, and the dispatch loop passes `limits` by value instead of cloning it.

## JSON Removal Review (Round 3)

Reviewed the refactor that removes JSON as a wire format from both the Rust FFI and .NET binding layers, making MessagePack the canonical transport with debug-only msgpack-to-JSON converter utilities.

**Scope**: `nx-ffi/src/lib.rs`, `NxRuntime.cs`, `NxNativeMethods.cs`, `NxComponentInitResult.cs`, `NxComponentDispatchResult.cs`, `NxRuntimeComponentTests.cs`, `ffi_smoke.rs`, `nx-ffi/Cargo.toml`.

**Findings**: No new issues. The refactor is clean:
- FFI entry points correctly reduced to three msgpack-only functions (`nx_eval_source`, `nx_component_init`, `nx_component_dispatch_actions`) plus four converter utilities.
- ABI version bumped to 2, matching the breaking change.
- `serde_json` and `base64` dependencies correctly retained in `nx-ffi` for the converter utilities.
- .NET `NxRuntime` properly restructured: `*Bytes` methods return raw msgpack, typed generic methods deserialize via MessagePack, and `*BytesToJson` methods wrap the converter FFI calls.
- `[JsonPropertyName]` attributes retained on public DTOs (`NxComponentInitResult`, `NxComponentDispatchResult`, `NxDiagnostic`) — intentional for downstream consumers who may JSON-serialize these types.
- `System.Text.Json` removed from `NxRuntime.cs` imports; correctly retained in test file for `JsonDocument`-based assertions.
- FFI smoke tests updated with `json_from_msgpack` helper covering both init and dispatch converter paths.
- .NET `ComponentResultBytesToJson_DebugConvertersReturnExpectedJson` test exercises the full init→dispatch round-trip through bytes + JSON converters including base64 state snapshot handling.
- R-01 and R-05 correctly superseded by this change.

All 49 Rust tests pass (`cargo test -p nx-api -p nx-interpreter -p nx-ffi -p nx-hir`).
