## Why

NX record payloads carry a wire-level `$type` discriminator, but generated TypeScript record types
do not model it today. That leaves downstream consumers to rely on unsafe casts for renderer
dispatch and makes the generated contracts diverge from the actual payload shape. Generated C#
records already reserve the `$type` MessagePack key, but they do not yet model the discriminator as
the concrete record identity across inheritance.

## What Changes

- Update TypeScript code generation for exported records and actions so generated concrete runtime
  contracts include a `$type` discriminator that matches the NX payload.
- **BREAKING** Split exported abstract TypeScript record contracts from exported concrete runtime
  types so the original abstract record name can represent the discriminated runtime surface while a
  generated base contract carries shared fields for inheritance.
- Update C# code generation to keep the `$type` MessagePack key aligned with the concrete generated
  record name, including records that derive from exported abstract bases.
- Add code generation coverage for single-file and library output with abstract base records,
  concrete derived records, and action records.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `cli-code-generation`: Generated TypeScript and C# record contracts preserve the NX `$type`
  discriminator and keep concrete record identity usable across exported abstract record
  inheritance.

## Impact

- `crates/nx-cli/src/codegen/model.rs`
- `crates/nx-cli/src/codegen/languages/typescript.rs`
- `crates/nx-cli/src/codegen/languages/csharp.rs`
- `crates/nx-cli/src/codegen.rs` tests
- Generated TypeScript consumers that currently cast NX payloads manually
