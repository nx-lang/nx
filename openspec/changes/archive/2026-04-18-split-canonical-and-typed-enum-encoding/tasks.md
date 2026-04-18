## 1. Canonical Enum Value Model

- [x] 1.1 Rename interpreter enum terminology from `EnumVariant`/`variant` to `EnumValue`/`member` across `crates/nx-interpreter` and any dependent runtime serialization helpers.
- [x] 1.2 Add `NxValue::EnumValue` in `crates/nx-value` and update custom JSON/MessagePack serde so canonical enum values write and read `"$enum"` plus `"$member"` as the only supported raw enum field names.
- [x] 1.3 Update `crates/nx-api/src/value.rs` so raw host conversion maps interpreter enum values to and from `NxValue::EnumValue` instead of treating enums as generic records.

## 2. Raw Runtime Contract Coverage

- [x] 2.1 Update raw `NxValue`, API, and FFI tests to assert the new canonical enum shape in JSON and MessagePack evaluation payloads.
- [x] 2.2 Add or update component lifecycle tests so enum-bearing props, rendered values, or effects preserve canonical self-describing enum payloads through the MessagePack input and JSON/MessagePack output paths.
- [x] 2.3 Refresh documentation comments and formatting/display tests that still describe canonical enum payloads with `"$variant"` or generic-record terminology.

## 3. Typed C# Enum Generation

- [x] 3.1 Update the C# generator to emit enum JSON converters that preserve exact authored NX member spellings and share parse/format helpers with MessagePack support.
- [x] 3.2 Simplify generated C# MessagePack enum deserialization to the typed string form and remove the raw enum-map fallback from generated formatters.
- [x] 3.3 Extend C# generator tests to verify authored member spellings, JSON string enum output, MessagePack string enum output, and the absence of canonical raw enum-map handling in typed DTO code.

## 4. .NET Binding Guidance And Verification

- [x] 4.1 Add or update .NET runtime tests that show raw JSON enum results remain canonical self-describing objects when consumed as `JsonElement`.
- [x] 4.2 Update `bindings/dotnet/README.md` and related binding guidance to distinguish canonical raw enum payloads from typed generated DTO enum strings across JSON and MessagePack workflows.
