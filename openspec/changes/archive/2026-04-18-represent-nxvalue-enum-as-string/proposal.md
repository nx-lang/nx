## Why

The canonical `NxValue` enum encoding carries `{"$enum": T, "$member": M}` across
JSON and MessagePack so a schema-less consumer can recover enum identity from the
payload alone. In practice every real consumer already has a target schema (a
typed DTO, a resolved NX type, or a rule contract), so the extra wrapper only
adds bytes, asymmetry with record-field serialization, and a gap between the
dynamic `NxValue` IR and the typed-DTO wire shape. Typed DTOs already serialize
enums as the bare authored member string via the shared
`NxEnumJsonConverter`/`NxEnumMessagePackFormatter`, and the JSON convention for
every mainstream ecosystem is a bare member string interpreted against the
target property type.

Collapsing the `NxValue` enum representation to the same bare string gives one
canonical wire shape for enums across every layer (raw `NxValue`, typed DTO,
NX-object mapping, JSON, MessagePack), deletes a whole branch of serde plumbing,
and keeps round-trips lossless whenever a schema is available, which is the
only realistic use case.

## What Changes

- **BREAKING**: `NxValue::EnumValue { type_name, member }` is removed. Enum values
  are represented in the `NxValue` IR as `NxValue::String(member)`, matching
  how records' non-discriminated fields already serialize.
- **BREAKING**: Canonical raw JSON and MessagePack payloads emitted by the native
  runtime no longer wrap enum values with `"$enum"`/`"$member"`. An `NxValue`
  that holds an enum member serializes as the bare authored member string.
- NX-object ↔ `NxValue` mapping gains a schema-driven step: when the target
  property (or other context) declares an enum type, the mapper parses the string
  through the existing per-enum wire format (same path typed DTOs use). When the
  context is `NxValue` itself, no parsing happens — the value stays a string.
- `.NET` raw-runtime consumers that previously observed `"$enum"`/`"$member"` in
  JSON/MessagePack output now see a bare string and must rely on the target
  schema to recover enum identity. The typed-DTO path is unchanged (it already
  used bare strings).
- Documentation is updated so the `NxValue` rustdoc, the runtime-output-format
  spec, and the `.NET` binding README state that enum values flow as authored
  member strings and are interpreted against the target type.
- The `enum-values` convention ("exact authored member spelling preserved") is
  unchanged — only the *container* around that string is removed.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `enum-values`: canonical raw payloads now represent enum members as bare
  authored strings rather than as `"$enum"`/`"$member"` objects; authored
  spelling preservation is unchanged.
- `runtime-output-format`: raw enum values in native-runtime JSON and MessagePack
  output SHALL collapse to the bare authored member string instead of the
  self-describing `"$enum"`/`"$member"` shape.
- `component-runtime-bindings`: raw `NxValue` props, actions, rendered values,
  and effects SHALL carry enum values as bare authored strings in both
  MessagePack host input and JSON/MessagePack output.
- `dotnet-binding`: raw-value and typed-model enum workflows converge on the
  bare authored member string; the managed binding no longer documents them as
  distinct enum contracts. Typed DTO serialization via the shared helpers is
  unchanged.

## Impact

- **Rust**: `crates/nx-value/src/lib.rs` (delete `EnumValue` variant, its
  `Serialize` branch, and the visit_map handling that recognizes
  `"$enum"`/`"$member"`); any callers that construct `NxValue::EnumValue` must
  switch to `NxValue::String` plus out-of-band type context.
- **Rust codegen & runtime**: any path in `nx-api`, `nx-ffi`, or `nx-cli` that
  emits canonical enum payloads must emit bare strings; any path that consumes
  user-supplied props/actions must accept bare strings and resolve enum identity
  from the target NX type.
- **.NET runtime binding**: raw-result consumers observe bare strings for enum
  values in `NxValue`, JSON, and MessagePack. Managed code that mapped
  `"$enum"`/`"$member"` objects into typed enums is removed; typed DTO paths are
  untouched.
- **Generated C# code**: no change — typed C# enums already use bare strings via
  `NxEnumJsonConverter`/`NxEnumMessagePackFormatter`.
- **Fixtures and golden snapshots**: any JSON/MessagePack fixture that captured
  raw-runtime enum output in the `"$enum"`/`"$member"` shape needs to be
  regenerated.
- **Docs**: `bindings/dotnet/README.md`, the `NxValue` rustdoc, and
  `runtime-output-format` / `component-runtime-bindings` / `dotnet-binding` /
  `enum-values` specs must reflect the new bare-string convention.
- **Backwards compatibility**: this is a wire-format break on the raw `NxValue`
  contract. Hosts on older runtime builds that still emit `"$enum"`/`"$member"`
  objects will round-trip as records through the new value model; we accept
  this break because there is no external release yet.
