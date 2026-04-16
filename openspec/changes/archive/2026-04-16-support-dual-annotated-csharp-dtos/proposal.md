## Why

Generated C# DTOs currently target MessagePack serialization only, which forces hosts that also use
`System.Text.Json` to hand-maintain parallel models or add JSON annotations after generation. Now
that NX runtime workflows can return JSON directly, generated DTOs need to serialize and deserialize
cleanly in both MessagePack and JSON without requiring a second model layer.

## What Changes

- Update C# code generation to emit DTOs that are annotated for both MessagePack and
  `System.Text.Json`.
- Add JSON property-name metadata for generated record, action, and external-component-state
  members so JSON payload keys stay aligned with the existing MessagePack contract.
- Add JSON polymorphism support for generated abstract C# record and action hierarchies so the same
  generated DTO family can deserialize concrete `$type`-discriminated JSON payloads.
- Keep the dual-format change focused on generated DTO classes rather than changing the separate
  JSON policy for standalone generated enums.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `cli-code-generation`: C# generated DTO contracts now need to support both MessagePack and JSON
  serialization/deserialization from the same generated type definitions.

## Impact

- Affected code: `crates/nx-cli/src/codegen.rs`, `crates/nx-cli/src/codegen/languages/csharp.rs`,
  and related generator model/tests.
- Affected output: generated C# DTO classes for exported records, actions, abstract families, and
  external component companion contracts.
- Affected dependencies/APIs: generated C# output will add `System.Text.Json.Serialization`
  attributes and JSON polymorphism metadata while preserving existing MessagePack behavior.
