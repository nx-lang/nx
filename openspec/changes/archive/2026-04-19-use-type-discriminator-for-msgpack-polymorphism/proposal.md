## Why

MessagePack polymorphic records currently diverge between the canonical `NxValue` map shape
(`$type` + fields) and generated C# typed DTO polymorphism (`[Union(...)]` discriminator metadata).
This split makes cross-layer round-trips fragile and forces hosts to reason about two wire
contracts for the same conceptual values.

## What Changes

- Define a single MessagePack polymorphic record contract based on the existing canonical `NxValue`
  map shape, where `$type` is the discriminator key.
- Update C# generation/runtime typed workflows so polymorphic records serialize and deserialize
  using the same `$type` map contract instead of MessagePack `Union` metadata.
- Align raw runtime MessagePack output and strongly typed managed MessagePack workflows so both
  consume/produce equivalent record shapes for polymorphic NX record families.
- Add/adjust conformance tests for abstract roots, concrete descendants, nested polymorphic values,
  and mixed raw/typed round-trips.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `runtime-output-format`: Clarify/require that canonical MessagePack polymorphic records use a
  `$type` discriminator map shape consistently across output workflows.
- `cli-code-generation`: Replace C# MessagePack polymorphism guidance based on `[Union(...)]` with
  `$type`-keyed map serialization behavior that matches canonical `NxValue`.
- `dotnet-binding`: Require managed typed MessagePack polymorphic record handling to match the same
  `$type` discriminator wire shape used by raw runtime values.

## Impact

- Affected code: `crates/nx-cli` C# generator, `bindings/dotnet` MessagePack polymorphism handling,
  and related test suites.
- Affected behavior: C# typed MessagePack payload shape for abstract record families.
- Compatibility: This is a wire-format breaking change for consumers relying on the current
  `Union`-based MessagePack polymorphism contract in generated C# DTOs.
