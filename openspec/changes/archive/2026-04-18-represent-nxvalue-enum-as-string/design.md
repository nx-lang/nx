## Context

`NxValue` is the serde-compatible data IR that crosses every NX API boundary
(Rust `nx-api`, FFI, `.NET` binding, JSON and MessagePack wire formats). Its
current enum variant `NxValue::EnumValue { type_name, member }` serializes as
the self-describing object `{"$enum": T, "$member": M}` in both JSON and
MessagePack, so a consumer with no schema can still recover the declaring type
of an enum member from the payload alone.

Every in-tree consumer already has a schema when it interprets an NxValue:
generated typed C# DTOs know their enum property types, the `.NET` typed wrapper
paths use `NxEnumJsonConverter` / `NxEnumMessagePackFormatter` with an explicit
`TWire` parser, and the NX interpreter has access to the declared NX type when
it converts `NxValue` → `Value`. Typed DTOs already emit enums as the bare
authored member string; only the raw `NxValue` IR still carries the
`$enum`/`$member` wrapper. That gap is the source of two persistent frictions
called out in the reviewed `share-dotnet-enum-serialization` change:

1. Raw and typed enum workflows are documented as "intentionally different
   layers" (see `dotnet-binding` spec, "Managed raw-value and typed-model enum
   workflows remain distinct"), which forces consumers to learn two shapes for
   the same conceptual value.
2. The canonical raw shape can't be achieved compactly in MessagePack without
   inventing an ext type, and it can't be matched by the strongly typed path
   because typed DTOs have no reason to wrap their enums.

This change collapses the two shapes to one by representing enums in the
`NxValue` IR, JSON, and MessagePack as the bare authored member string and
letting the target schema recover the declaring enum type at the boundary where
`NxValue` is consumed.

## Goals / Non-Goals

**Goals:**
- Single canonical wire shape for enum values across raw `NxValue`, typed DTOs,
  JSON, and MessagePack: the bare authored member string.
- Delete `NxValue::EnumValue` and its serde plumbing; reuse `NxValue::String`.
- Keep the interpreter's `Value::EnumValue { type_name, member }` intact — that
  is an internal runtime representation, not the IR, and it still needs the
  type identity to answer queries like `Value::get_type`.
- Preserve the authored member spelling (e.g., `pending_review`) exactly
  through every layer; `enum-values` semantics do not regress.
- Keep the typed-DTO path bit-for-bit compatible — it already emits bare
  strings, so it only gains the guarantee that raw and typed match.

**Non-Goals:**
- Not changing how the NX interpreter represents enum values internally.
- Not introducing an integer-based enum wire format. Authored strings remain
  the canonical identity across the system.
- Not providing a schema-less consumer path that recovers enum identity from
  the wire alone — schema-driven consumption is the contract.
- Not bumping a public compatibility promise; there is no external release to
  preserve.

## Decisions

### D1: Drop `NxValue::EnumValue` rather than keeping it and changing only its serde

**Decision:** Remove the `EnumValue` variant from `NxValue`. Any code that
previously constructed `NxValue::EnumValue { type_name, member }` constructs
`NxValue::String(member)` instead.

**Why:** Keeping the variant but serializing it as a string would leave a
second representation for the same logical value (both `String("dark")` and
`EnumValue { type_name: "ThemeMode", member: "dark" }` would serialize
identically), creating a Rust API surface that can't be deserialized back into
the variant without out-of-band context. Since the IR no longer needs to carry
type identity on the wire, the only correct modeling is to carry the string
itself and let schema-driven mappers reintroduce the enum type at the
boundary. The variant also has no unique use beyond serialization; its
accessors are one-to-one with `String`.

**Alternatives considered:**
- *Keep `EnumValue` internally, serialize as string.* Rejected because the
  deserializer would always produce `NxValue::String` regardless of whether the
  original author constructed `EnumValue` — equivalent values would no longer
  round-trip to equivalent variants.
- *Serialize as MessagePack ext type + keep `EnumValue`.* Rejected because it
  diverges JSON and MessagePack shapes and still doesn't match typed DTOs.

### D2: Schema-driven interpretation at the `NxValue` → typed-value boundary

**Decision:** Every code path that converts a consumed `NxValue` into a typed
NX `Value` or a typed `.NET` enum already walks a target NX type or CLR type in
parallel with the `NxValue`. At each step where the target is an enum type and
the `NxValue` is a string, that path looks up the per-enum wire format for the
target and parses the string via the existing
`INxEnumWireFormat<TEnum>.Parse` contract (C#) or the equivalent NX-side
resolution by `type_name` + `member`. When the target is `any` / `NxValue` /
an untyped slot, the string stays a string — no interpretation is performed.

**Why:** This reuses the exact mechanism typed DTO deserialization already
uses and matches how JSON deserialization works in every other ecosystem
(Serde with `#[serde(from/into)]`, System.Text.Json, Jackson, etc.). It also
means the `NxValue` layer itself stays "dumb" — it does not need a type
registry. Registries live at the consumer boundary, which already has them.

**Alternatives considered:**
- *Have `NxValue` hold a reference to an external type registry.* Rejected:
  violates the "stable, serde-compatible data IR" role of `NxValue`.
- *Require every consumer to provide an explicit `EnumContext` wrapper value.*
  Rejected: too intrusive and ruins the JSON convention parity.

### D3: Native runtime emits bare strings in both JSON and MessagePack

**Decision:** The native runtime's `nx-api`, `nx-ffi`, and CLI format paths
produce `NxValue::String(member)` (and therefore bare strings on the wire) for
every enum value reachable through `Value::EnumValue`. MessagePack and JSON
outputs agree byte-for-byte on the enum's representation.

**Why:** The `runtime-output-format` spec currently requires
`"$enum"`/`"$member"` for raw output specifically to preserve enum identity on
the wire. With schema-driven interpretation (D2) that guarantee is no longer
needed — any consumer that needs the enum identity already has the schema.
Dropping the wrapper gives us JSON parity with strong-typed DTOs for free and
keeps the two output formats symmetric.

**Alternatives considered:**
- *Leave the wrapper only in MessagePack.* Rejected: defeats the main
  simplification goal and leaves the managed binding documenting two different
  enum contracts.

### D4: Host input accepts bare strings where the target type is an enum

**Decision:** Callers that supply props / action payloads / evaluation inputs
as `NxValue` (or raw MessagePack) provide a bare authored member string where
the target NX type is an enum. The API validation layer (the same code that
today enforces the `"$enum"`/`"$member"` shape for inputs) looks up the
declared target enum type and resolves the string to
`Value::EnumValue { type_name, member }` by validating membership against the
known enum type. Unknown members are rejected at the boundary with the existing
argument-validation error pathway.

**Why:** Symmetric with D3 — inputs must match outputs, and the NX interpreter
already knows the declared input type for every parameter/prop.

### D5: Document the IR-level ambiguity and the mitigation

**Decision:** Add a rustdoc note on `NxValue::String` that a string may
represent either a plain string or an enum member, distinguishable only
against the target schema. The `runtime-output-format`,
`component-runtime-bindings`, and `dotnet-binding` specs explicitly state that
schema-less consumers cannot recover enum identity from raw output alone.

**Why:** The only cost of the new model is that a purely schema-less pretty
printer or diff tool cannot tell a string apart from an enum member in raw
output. That's a real cost and it deserves a visible call-out so future
consumers understand the contract.

## Risks / Trade-offs

- **Schema-less consumers lose enum identity on the wire** → Documented as
  explicit non-goal (D5). No in-tree consumer relies on this.
- **Existing fixtures and golden snapshots break** → Regenerate all
  JSON/MessagePack fixtures under `crates/` and `bindings/` that captured raw
  enum outputs (`ffi_smoke.rs`, `NxRuntimeBasicTests.cs`, README examples); a
  single search for `"$enum"` finds them all.
- **Unknown-member strings arriving where an enum is expected** → Handled by
  the same error pathway typed DTO deserialization already uses
  (`FormatException` from `TWire.Parse` → wrapped as
  `JsonException`/`MessagePackSerializationException`). Rust-side mapping
  produces the existing `RuntimeErrorKind::TypeMismatch` variant.
- **Two-step mental model for authors writing new FFI endpoints** ("remember
  to pass target type when converting") → Mitigated by keeping the converter
  API signature explicitly type-driven; the type-less helpers stay type-less
  and just pass strings through.
- **Wire break for any out-of-tree consumer built against the current raw
  shape** → Accepted per proposal; we have no external release. If a staging
  host ships before the runtime rolls over, regenerate its fixtures.

## Migration Plan

1. Land the Rust-side change in `nx-value`: delete variant, update
   `Serialize`/`Deserialize`, update tests in place. Interior callers
   (`nx-interpreter`, `nx-api`, `nx-cli`, `nx-ffi`) switch their `NxValue`
   construction and pattern matches to `NxValue::String`.
2. Update `nx-api` / `nx-ffi` input validation to resolve strings against the
   declared target enum type (D4).
3. Update `.NET` binding: remove the raw-vs-typed dichotomy documentation,
   regenerate `NxRuntimeBasicTests` raw-enum assertions, and rewrite the
   README "Enum Encoding" section to a single paragraph covering both layers.
4. Regenerate fixtures and review the final grep for `$enum` / `$member` —
   should return zero hits outside the archived proposal's history.
5. Update the four affected specs (`enum-values`, `runtime-output-format`,
   `component-runtime-bindings`, `dotnet-binding`) via delta specs in this
   change.

Rollback is a straight git revert; there is no data persisted in the old shape.
