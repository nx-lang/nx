## 1. Native Output Format Contract

- [x] 1.1 Add a native output-format enum to `crates/nx-ffi`, update the value-returning runtime entry point signatures to accept it, bump `NX_FFI_ABI_VERSION`, and regenerate the exported C header metadata.
- [x] 1.2 Refactor runtime response writing in `crates/nx-ffi` so evaluation and component lifecycle calls serialize MessagePack or JSON directly from the requested call path instead of converting from an intermediate MessagePack payload.
- [x] 1.3 Remove the public MessagePack-to-JSON converter exports and update any native/header references that still expose them.

## 2. Runtime Coverage

- [x] 2.1 Add or update Rust FFI tests for source and program-artifact evaluation that verify both MessagePack and JSON success payloads plus JSON diagnostic payloads on failure.
- [x] 2.2 Add or update Rust FFI tests for component initialization and dispatch that verify selectable output formats, base64 `state_snapshot` behavior in JSON, and unchanged MessagePack-only input handling for props/actions/snapshots.

## 3. Managed .NET API

- [x] 3.1 Add managed interop support for the native output-format enum and update raw `NxRuntime` byte-returning methods so callers can request MessagePack or JSON per call.
- [x] 3.2 Add explicit managed JSON convenience APIs that return `JsonElement`, `NxComponentInitResult<JsonElement>`, and `NxComponentDispatchResult<JsonElement>` while preserving the existing MessagePack generic APIs.
- [x] 3.3 Parse evaluation diagnostics from the selected runtime output format and remove the public `ValueBytesToJson`, `DiagnosticsBytesToJson`, `ComponentInitResultBytesToJson`, and `ComponentDispatchResultBytesToJson` helper methods.

## 4. Managed Tests And Documentation

- [x] 4.1 Update .NET tests to cover JSON pass-through bytes, `JsonElement` evaluation results, JSON component lifecycle results, and helper API removal.
- [x] 4.2 Update `bindings/dotnet/README.md` and other runtime-facing documentation/examples to describe selectable output formats, direct JSON output, `JsonElement` usage in C#, and the continued MessagePack-only input contract.
