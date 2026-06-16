## Why

`nxlang types --language csharp` currently drops authored literal defaults from generated DTO
properties. This silently changes runtime behavior when a generated C# instance is constructed
without explicitly setting a field, for example `bool = true` becomes the CLR default `false`.

## What Changes

- Preserve literal defaults from exported records, actions, union case payloads, and external
  component props in generated C# property initializers.
- Continue emitting TypeScript interfaces without default values because interfaces have no runtime
  initialization behavior.
- Emit a generation warning when a C# field has a default expression that cannot be rendered as a
  target-language literal initializer.
- Keep existing `default!` nullable-suppression initializers only when no authored literal default
  is available.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `cli-code-generation`: C# generated DTO properties must preserve supported NX literal defaults.

## Impact

- Affects the type generation export model in `crates/nx-cli/src/typegen/model.rs`.
- Affects C# DTO emission in `crates/nx-cli/src/typegen/languages/csharp.rs`.
- Adds regression coverage for record fields, union case fields, and external component props with
  literal defaults.
