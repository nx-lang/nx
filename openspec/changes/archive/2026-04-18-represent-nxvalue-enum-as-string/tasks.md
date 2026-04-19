## 1. Remove the `EnumValue` variant from the `NxValue` IR

- [x] 1.1 Delete the `NxValue::EnumValue { type_name, member }` variant in `crates/nx-value/src/lib.rs`, its custom `Serialize` branch, and the `$enum` / `$member` handling in the `visit_map` deserializer
- [x] 1.2 Update the rustdoc on `NxValue` / `NxValue::String` to state that enum values flow as the bare authored member string and are resolved against the target schema
- [x] 1.3 Update the unit tests in `crates/nx-value/src/lib.rs` so that JSON and MessagePack enum round trips assert a bare string payload and remove every fixture that reads `"$enum"` / `"$member"`

## 2. Update interior Rust callers that construct or match `NxValue::EnumValue`

- [x] 2.1 Audit `crates/nx-api`, `crates/nx-cli`, `crates/nx-ffi`, and `crates/nx-interpreter` for `NxValue::EnumValue` construction and pattern matches (grep for `NxValue::EnumValue` and `$enum`)
- [x] 2.2 Replace every `NxValue::EnumValue { type_name: _, member }` producer with `NxValue::String(member)` (dropping the type identity because downstream consumers recover it from the target NX type)
- [x] 2.3 Replace every pattern match that dispatched on `NxValue::EnumValue` with the corresponding `NxValue::String` branch, collapsing redundant arms
- [x] 2.4 Leave `nx-interpreter`'s internal `Value::EnumValue { type_name, member }` intact; only its conversion layer into/out of `NxValue` changes

## 3. Teach the host-input path to resolve bare strings against target enum types

- [x] 3.1 Update the prop / action / evaluation-argument validation path in `nx-api` / `nx-ffi` so that when the declared target NX type is an enum and the incoming `NxValue` is a string, the validator produces `Value::EnumValue { type_name, member }` after confirming the member exists in the declared type
- [x] 3.2 Route unknown enum members through the existing argument-validation type-mismatch error path (same surface hosts already see for mismatched scalar types)
- [x] 3.3 Leave bare strings as bare strings when the target NX type is `any` / untyped / `NxValue`

## 4. Update the native runtime output path to emit bare strings

- [x] 4.1 Ensure the canonical raw JSON and MessagePack outputs produced by `nx-api` evaluation, component initialization, and component dispatch carry enum values as `NxValue::String(member)` (follows automatically from task 2 once `NxValue::EnumValue` is gone, but verify via smoke tests in `crates/nx-ffi/tests/ffi_smoke.rs`)
- [x] 4.2 Regenerate any golden fixtures under `crates/` whose recorded bytes used the `"$enum"` / `"$member"` wrapper
- [x] 4.3 Update the CLI formatters in `crates/nx-cli/src/format.rs` so that `Value::EnumValue` renders consistently with the new canonical wire shape (bare member string) while keeping human-facing formatting (`DealStage.pending_review`) unchanged for non-JSON human output

## 5. Update the .NET binding

- [x] 5.1 Update `bindings/dotnet/tests/NxLang.Runtime.Tests/NxRuntimeBasicTests.cs` so `EvaluateJson_EnumValue_ReturnsCanonicalEnumObject` (and any other raw-enum tests) asserts the bare authored member string instead of `"$enum"` / `"$member"` properties, renaming the test if it improves clarity
- [x] 5.2 Rewrite the "Enum Encoding" section of `bindings/dotnet/README.md` to describe a single bare-authored-member-string contract across raw and typed layers; delete the "Raw runtime payloads keep the canonical self-describing `$enum`/`$member` form" paragraph
- [x] 5.3 Add a raw-value test that feeds bare-string input through a raw runtime call and asserts the enum slot is accepted, plus a negative test for unknown members

## 6. Update specs in openspec/specs/

- [x] 6.1 Confirm delta specs in this change cover every existing requirement that mentioned `"$enum"` or `"$member"` (`enum-values`, `runtime-output-format`, `component-runtime-bindings`, `dotnet-binding`)
- [x] 6.2 After implementation, archive this change and sync the delta specs into `openspec/specs/` per the standard archive workflow

## 7. Verification

- [x] 7.1 Run `cargo test` for `nx-value`, `nx-api`, `nx-ffi`, `nx-interpreter`, and `nx-cli` and confirm all tests pass
- [x] 7.2 Run `dotnet test bindings/dotnet/NxLang.sln` and confirm all existing typed and raw tests pass
- [x] 7.3 Run `rg '\$enum|\$member|NxValue::EnumValue'` across the repo (excluding `openspec/changes/archive`) and confirm remaining hits are limited to synced spec text that documents or forbids the removed wrapper; no implementation code or live tests still use the old shape
- [x] 7.4 Manually verify a raw-runtime JSON call for an enum-returning program produces a bare-string payload
