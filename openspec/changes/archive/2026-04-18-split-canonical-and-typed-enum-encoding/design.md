## Context

NX currently uses one enum concept across two different host-facing layers:

- The raw `NxValue` layer is a schema-free value tree used by runtime/FFI boundaries, generic
  tooling, and round-tripping between public values and interpreter values.
- Generated C# DTOs are schema-aware host models whose enum properties already know the target enum
  type at deserialization time.

Today those layers bleed into each other. Raw `NxValue` host conversion encodes enums as a generic
record containing `"$enum"` and `"$variant"`, while generated C# MessagePack enums serialize as a
string but still deserialize both strings and raw enum maps. JSON handling for generated C# enums
is also inconsistent with the intended string-based host contract.

This change needs to separate the canonical raw-value contract from the typed host-model contract
without giving JSON and MessagePack different semantics for the same `NxValue`.

## Goals / Non-Goals

**Goals:**
- Make canonical `NxValue` enum payloads explicit, lossless, and format-independent.
- Represent enum identity as a first-class value concept instead of a magic generic record.
- Align terminology on `EnumValue` and `member`.
- Make generated C# enums use the same string-based wire shape in both JSON and MessagePack.
- Remove typed C# MessagePack fallback logic that only exists to accept raw enum maps.
- Cut over the canonical raw enum contract directly without a compatibility shim.

**Non-Goals:**
- Making raw `NxValue` consumers schema-aware.
- Changing non-C# generated SDKs in this proposal.
- Reworking record/action discriminator behavior beyond enum-related terminology.
- Preserving raw-wire backward compatibility for legacy `"$variant"` enum payloads.

## Decisions

### 1. Add a first-class `NxValue::EnumValue` case

Canonical raw values will stop representing enums as `NxValue::Record` instances with magic keys.
Instead, `nx-value` will gain a first-class enum case:

```rust
EnumValue {
    type_name: String,
    member: String,
}
```

`nx-api` host conversion will map interpreter `Value::EnumValue` values to and from this variant
instead of folding them into generic records.

Rationale:
- The raw value model should reflect semantic value kinds, not require consumers to infer that a
  particular record shape means “enum”.
- Round-tripping becomes explicit instead of special-casing a generic record on only one side.
- This matches the design direction already used by the interpreter runtime value enum.

Alternatives considered:
- Keep using `NxValue::Record` with reserved keys. Rejected because it keeps enum identity hidden
  inside a generic map type and makes the public model harder to reason about.
- Collapse raw enums to bare strings. Rejected because `NxValue` is schema-free and cannot safely
  recover enum identity from a string alone.

### 2. Keep canonical JSON and MessagePack self-describing with the same enum map shape

`NxValue::EnumValue` will serialize to the same logical wire shape in both JSON and MessagePack:

```json
{ "$enum": "Status", "$member": "active" }
```

The custom `Serialize` / `Deserialize` implementation in `nx-value` will treat that reserved pair
as an enum value, just as `"$type"` is already reserved for typed records. This keeps the raw
contract format-independent: the only difference between JSON and MessagePack is the container
encoding, not the meaning of the value.

Rationale:
- Raw runtime payloads and storage/diff tooling need a lossless contract.
- FFI entry points already expose raw `NxValue` MessagePack flows, so MessagePack cannot assume a
  strongly typed reader.
- Matching semantics across JSON and MessagePack avoids format-specific surprises for the same
  public value model.

Alternatives considered:
- Make JSON self-describing but make MessagePack schema-dependent. Rejected because the same
  `NxValue` would have different semantics by format.
- Include the enum as an array tuple such as `["Status", "active"]`. Rejected because reserved
  object keys are clearer and align with existing `$type` metadata conventions.

### 3. Rename runtime/value terminology from `variant` to `member`

Interpreter values, public host conversion helpers, and canonical raw serialization will use the
terms `EnumValue` and `member`. Canonical raw readers and writers will use `"$member"` rather than
`"$variant"`.

Rationale:
- “member” is more natural for simple NX enums than “variant”.
- First-class enum value modeling is easier to discuss when type and field names match the intended
  semantics.
- A direct cutover keeps the canonical contract simple and avoids carrying transitional complexity
  into the new design.

Alternatives considered:
- Keep `"$variant"` forever for wire compatibility. Rejected because it preserves terminology the
  change is explicitly trying to simplify.
- Read both `"$variant"` and `"$member"` during a transition window. Rejected because the user does
  not need backward compatibility and the extra branch would complicate the canonical raw contract.
- Rename internal Rust types but keep `"$variant"` on the wire. Rejected because it keeps the raw
  contract inconsistent with the public terminology.

### 4. Treat typed C# enums as string-valued in both serializers

Generated C# enums will continue to use plain enum member strings for MessagePack and will add an
explicit `System.Text.Json` converter so JSON uses the same authored member strings. The generator
will emit a custom JSON converter per NX enum, following the same pattern already used by
`NxSeverity`, rather than relying on `JsonStringEnumConverter`.

The generated MessagePack formatter will deserialize only strings. It will no longer accept the raw
canonical enum map shape as a typed enum input.

Rationale:
- Typed DTO properties already know the enum type and should not need redundant raw type metadata.
- A custom JSON converter preserves exact NX member spellings even when generated C# enum members
  are sanitized or renamed.
- Matching JSON and MessagePack shapes simplifies typed host contracts and makes tests clearer.

Alternatives considered:
- Use `JsonStringEnumConverter`. Rejected because default enum-name policies do not guarantee exact
  authored NX member spellings.
- Keep the MessagePack string-or-map reader. Rejected because it couples typed SDKs to the raw
  `NxValue` contract and keeps unnecessary deserialization complexity.

### 5. Preserve a strict boundary between raw payload APIs and typed SDK APIs

Raw runtime/FFI payload APIs will continue to use canonical `NxValue` JSON and MessagePack. Typed
SDK and generated-model workflows will continue to rely on schema-aware serializers and plain enum
strings. Documentation and tests will call out that these are intentionally different layers.

Rationale:
- The raw contract optimizes for stability, tooling, and round-trip safety.
- The typed contract optimizes for ergonomic host usage.
- Mixing them produces the current ambiguity where a typed enum formatter partly implements the raw
  map contract.

## Risks / Trade-offs

- [Raw wire compatibility break] Existing consumers may inspect `"$variant"` directly or treat raw
  enum payloads as generic records. → Mitigation: make the breaking cutover explicit in the
  proposal/spec/task artifacts and update first-party tests/docs in the same change.
- [Reserved-key ambiguity] A raw object containing `"$enum"` and `"$member"` will now decode as an
  enum value instead of a generic record. → Mitigation: treat these keys as reserved canonical
  metadata, consistent with existing `"$type"` handling.
- [Generator complexity] Per-enum JSON converters add emitted code. → Mitigation: reuse the same
  parse/format helpers used by the MessagePack formatter so the generated logic stays localized and
  mechanically testable.
- [Partial rollout confusion] Raw payload consumers and typed DTO consumers will intentionally see
  different enum shapes. → Mitigation: document the contract split explicitly in specs, README
  guidance, and tests.

## Migration Plan

1. Rename interpreter/public value terminology to `EnumValue` and `member`.
2. Add `NxValue::EnumValue` plus custom serde support that writes and reads `"$member"` as the only
   canonical raw enum member field.
3. Update `nx-api` raw host conversion and FFI tests to use the new canonical enum value contract.
4. Update C# code generation to emit JSON enum converters and simplify MessagePack enum parsing to
   string-only.
5. Refresh .NET runtime docs/tests so raw payload APIs and typed DTO workflows are described as
   separate contracts.

This is a direct breaking cutover rather than a phased migration. Rollback is straightforward
before downstream regeneration: restore the old enum naming/serde behavior and regenerate affected
C# outputs.

## Open Questions

- Should future non-C# generators adopt the same explicit typed-enum string policy, or should that
  remain language-specific until each generator has concrete host requirements?
