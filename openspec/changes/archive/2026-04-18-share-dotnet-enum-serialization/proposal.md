## Why

Generated C# enums currently emit a dedicated wire-format helper, JSON converter, and MessagePack
formatter for every NX enum. That duplicates the same serializer plumbing across every generated
file, bloats generated output, and forces the runtime binding to maintain the same pattern again for
its own enums.

## What Changes

- Add reusable generic enum serialization helpers to `NxLang.Runtime` for `System.Text.Json` and
  MessagePack.
- Update generated C# enums to reference the shared runtime helpers instead of emitting a dedicated
  JSON converter and MessagePack formatter per enum.
- Keep enum-specific NX member string mappings generated so authored wire names remain explicit and
  lossless even when CLR enum member names are Pascal-cased.
- Update the managed binding's own `NxSeverity` enum to use the same shared helper infrastructure.
- **BREAKING** Generated C# enums will no longer be fully self-contained and will require a
  reference to `NxLang.Runtime` for enum serialization support.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `cli-code-generation`: C# enum generation will emit shared-helper-based serialization metadata
  instead of full per-enum converter and formatter types.
- `dotnet-binding`: `NxLang.Runtime` will expose reusable enum serialization helpers for generated
  typed DTO workflows.

## Impact

- Affected code: `crates/nx-cli` C# code generation, `bindings/dotnet/src/NxLang.Runtime`
  serialization helpers and `NxSeverity`, generated C# tests, and .NET binding documentation.
- Affected APIs: generated C# enum support types and the public `NxLang.Runtime` serialization
  surface used by generated code.
- Affected consumers: C# projects compiling generated NX enums will need a `NxLang.Runtime`
  reference alongside their existing serializer dependencies.
