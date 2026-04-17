## Why

Generated C# DTOs currently synthesize a `__NxType` property mapped to wire key `$type` on concrete
records and actions. That member is redundant for the serializers we support: `System.Text.Json`
polymorphism already uses the abstract root's `[JsonPolymorphic]`/`[JsonDerivedType]` metadata, and
MessagePack polymorphism already uses `[Union(...)]` tags rather than an inner `$type` field.

Keeping the generated member adds payload noise, changes the wire shape for non-polymorphic DTOs,
and exposes a generated C# property that consumers do not need to populate or read. Removing it
aligns the generator with actual serializer behavior and avoids carrying a synthetic member through
every generated DTO.

## What Changes

- **BREAKING** Remove the generated C# discriminator property (`__NxType`) from concrete record and
  action DTOs.
- Stop generating abstract discriminator members whose only purpose was to support the removed
  concrete property.
- Keep JSON polymorphism metadata on abstract record and action roots so `System.Text.Json` still
  uses `$type` for abstract-family dispatch.
- Omit invalid JSON polymorphism metadata for abstract C# roots that have no concrete exported
  descendants, and warn when that generated contract cannot support STJ polymorphic dispatch yet.
- Keep MessagePack `[Union(...)]` metadata for record and action hierarchies so MessagePack
  polymorphism continues to work without an inner `$type` member.
- Update code generation tests and CLI integration coverage to assert the new discriminator-free C#
  output.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `cli-code-generation`: Change the generated C# DTO contract so serializer support no longer
  depends on an emitted `$type`/`__NxType` property.

## Impact

- Affected code: `crates/nx-cli/src/codegen/languages/csharp.rs`,
  `crates/nx-cli/src/codegen.rs`, and `crates/nx-cli/src/main.rs`.
- Affected API surface: generated C# DTOs for exported records and actions.
- Consumer impact: hosts that referenced the generated `__NxType` property directly will need to
  stop relying on it after regeneration.
