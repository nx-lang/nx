## Why

NX currently mixes two different host contracts for enums. The raw `NxValue` surface is a
schema-free value tree that needs self-describing enum identity, while generated typed host models
can rely on schema context and should use plain enum member strings for JSON and MessagePack.
Keeping those concerns mixed makes the raw contract ambiguous, the typed contract inconsistent, and
the current terminology harder to reason about.

## What Changes

- **BREAKING** Make canonical raw-value enum encoding explicit and lossless across both JSON and
  MessagePack, using a first-class enum value shape instead of relying on bare strings.
- **BREAKING** Rename the canonical raw enum member field from `$variant` to `$member` and align
  runtime/value-model terminology around `EnumValue` and `member`.
- Do not preserve raw-wire compatibility for legacy `"$variant"` enum payloads; canonical raw enum
  readers and writers will move to `"$member"` together.
- Define a contract split between canonical `NxValue` payloads and schema-aware generated host
  models.
- Keep canonical `NxValue` payloads self-describing for raw runtime, FFI, storage, and tooling
  flows.
- Make generated C# enums serialize as plain member strings for both `System.Text.Json` and
  MessagePack, so typed host contracts use the same logical enum shape in both formats.
- Remove generated C# MessagePack enum deserialization complexity that only exists to accept the
  canonical raw enum map shape as a typed enum input.
- Update tests and documentation to distinguish canonical raw-value encoding from typed host-model
  encoding.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `enum-values`: Change first-party host value conversion and terminology so canonical raw enum
  values are represented as enum values with `$enum` plus `$member`, while preserving authored
  member spelling.
- `runtime-output-format`: Clarify that canonical raw `NxValue` JSON and MessagePack payloads stay
  self-describing across formats instead of depending on typed readers to infer enum identity.
- `component-runtime-bindings`: Keep component lifecycle props and action batches on the canonical
  raw `NxValue` contract, including self-describing enum payloads inside MessagePack inputs and
  outputs.
- `cli-code-generation`: Change generated C# enum contracts so typed JSON and MessagePack payloads
  both use plain enum member strings and no longer accept canonical raw enum maps as the primary
  typed input shape.
- `dotnet-binding`: Clarify that raw runtime payload APIs preserve the canonical `NxValue` enum
  shape, while typed managed DTO workflows use strong enums encoded as strings.

## Impact

- Affected Rust code in `crates/nx-value`, `crates/nx-api`, `crates/nx-interpreter`, and
  `crates/nx-ffi`.
- Affected C# generation in `crates/nx-cli/src/codegen/languages/csharp.rs` and related tests.
- Affected host-facing wire contracts for raw `NxValue` JSON/MessagePack payloads and generated C#
  enum DTOs.
- Consumer impact: raw payload consumers that inspect `"$variant"` or treat canonical enums as
  generic records will need to update to the new enum value contract.
