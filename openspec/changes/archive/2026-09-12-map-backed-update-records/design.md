## Context

See proposal.md — Why. The facts that shape the approach:

- `emit_update` in `crates/nx-cli/src/typegen/languages/csharp.rs` emits one
  `NxOptional<T>` auto-property per field, each carrying `[Key]`, `[JsonPropertyName]`, and
  `[JsonIgnore(Condition = WhenWritingDefault)]`, on a class annotated with
  `[MessagePackFormatter(typeof(NxUpdateRecordMessagePackFormatter<T>))]`.
- That formatter walks `PropertyInfo` at runtime to find the keyed properties, decide whether each
  is an unset `NxOptional<>`, and `GetValue`/`SetValue` them.
- Update companions are emitted for exported **records, actions, and components with declared
  state**. `emit_declaration` emits a class only for `ExportedType::Record` and
  `ExportedType::ExternalState`, so a component's update companion has no generated plain type
  holding its fields. This is the constraint that splits the SDK base class in two.
- `<T>_property` already exists as a C# `enum` with a generated `INxEnumWireFormat<T>`
  implementation that maps each case to and from its bare wire name. Any new key surface must build
  on that rather than introduce a second naming.
- `bindings/dotnet/tests/NxLang.Sdk.Tests/Generated/UpdateRecords.g.cs` is checked in and asserted
  byte-equal to typegen's output by `checked_in_dotnet_update_record_fixture_matches_typegen_output`
  in `crates/nx-cli/src/typegen.rs`, and the .NET tests compile against it. Emitter and fixture move
  together or CI fails on both sides.

## Goals / Non-Goals

**Goals:**

- One storage model for update DTOs that serves both the typed front door and generic access.
- Wire bytes unchanged, in both formats, including key order.
- The `<T>_property` enum stays the single naming of a record's fields.
- No reflection over generated members on any serialization path.

**Non-Goals:**

- No change to `NxOptional<T>`'s public shape or to how a host writes a patch.
- No source generator. Typegen already emits the code a source generator would.
- No general-purpose `Delta<T>`-style dynamic API beyond what the four helpers need.

## Decisions

### D1: Map storage on a base class, with the generated class as a typed façade

`NxUpdateRecord` holds `Dictionary<string, object?>` keyed ordinally, and the generated class keeps
its `NxOptional<T>` properties as `Get<T>(name)` / `Set<T>(name, value)` pairs over it. The public
property surface does not move, so every existing host call site and every existing test keeps
compiling.

*Alternative considered:* keep per-property storage and emit a per-DTO converter pair from typegen,
which would also kill the reflection. Rejected: it solves serialization and nothing else. A host
still could not enumerate, unset, merge, or diff, which is the actual gap.

*Cost accepted:* value-typed fields box. At patch sizes — a handful of fields per user interaction —
that is not measurable.

### D2: `NxOptional<T>` survives as a pure in-memory value

The struct stays as the accessor type, because it is the only thing that expresses unset-versus-null
for a value type. It loses `[JsonConverter]`, `[MessagePackFormatter]`, both converters, and the
serialization half of its doc comment. Once no serializer can see it, the misuse case RF8 of the
`add-update-actions` review hardened against — writing an unset optional as `null`, silently turning
"unchanged" into "set to null" — stops being reachable, and the two `throw`s that guarded it go with
the converters.

*Alternative considered:* replace it with `TryGet`-style accessors or a nullable. Rejected: breaks
the object-initializer front door, and a nullable cannot distinguish unset from null for `int`.

### D3: Two base classes, split by whether an instantiable plain record type exists

- `NxUpdateRecord` — storage, `Fields`, `IsSet`, `Unset`, `Merge`, `Changed`, serialization, and
  the abstract schema. Every generated update companion derives from it, including a component's
  and an action's.
- `NxUpdate<TRecord> : NxUpdateRecord` — adds `Apply(TRecord)` and `Diff(before, after)`, and
  supplies the schema by projecting the generated key table. Used only where typegen emits a plain
  `TRecord`.

`Apply` and `Diff` are the only two operations that need to touch a plain record; `Merge` and
`Changed` see only patches. Putting all four on `NxUpdate<TRecord>` would leave `Counter_update`
unable to merge, which is exactly the component-state case two-way binding will want.

*Found in implementation:* `Apply` returns a new record, as `nxApplyUpdate` does, which needs
`new()` on `TRecord`. An abstract record's companion therefore also derives from `NxUpdateRecord`
and gets no key table: nothing can instantiate the record to apply a patch to, and a concrete
descendant's own companion carries the inherited fields with keys of its own. A component's
companion patches its *state*, so the plain record an external component gets under its own name
(its props contract) is never its target; the model marks a companion whose target is a component
for that reason. An external component with declared state does have a plain state type,
`<Name>_state`, with exactly the companion's fields, so that companion derives from
`NxUpdate<<Name>_state>` and gets a `<Name>_stateProperties` table. Only a non-external component's
companion, whose state has no generated type at all, takes the untyped base.

*Found in review (RF1):* the companion introduces six members of its own (`NxType`, `FieldSchema`,
`IsSet`, `Unset`, `Changed`, `Diff`) and inherits the base surface. A field accessor keeps the
record's property name, since that is the front door a host writes; each introduced member steps
around the field names with a trailing underscore, the way `NxType` already did; and an accessor
that lands on a non-generic base member is emitted `public new`, so the base stays reachable
through an `NxUpdateRecord`-typed reference and the generated bodies reach it through `base.`.

### D4: The key table is the schema where one exists

A key (`NxProperty<TRecord, TValue>`) already carries a field's wire name, value type, and
getter/setter. Rather than emit a name→type dictionary *and* a key table, typegen emits the key
table and `NxUpdate<TRecord>` projects the schema from it. For a companion with no plain type,
typegen emits the plain name→type schema instead. One generated table per companion either way.

### D5: Keys are reached through the existing property enum

Typegen emits `<T>Properties` beside the record: one `NxProperty<T, TValue>` per field, plus
`Of(<T>_property)` mapping each enum case to its key. Nothing introduces a second spelling of a
field name — the enum's generated `INxEnumWireFormat<T>` remains the only place a wire name is
written, and the key's name comes from it.

The `<T>Properties` name follows the companion collision rule: when an exported declaration
already owns it, generation warns, omits the table, and the companion falls back to the untyped
base with a name→type schema (found in review, RF2).

`Changed` returns wire names from the base, and the generated class narrows that to
`<T>_property[]` through the same wire-format `Parse`. The base class cannot name the enum type, and
making it generic over the enum would push a second type parameter onto every companion for the sake
of one method.

*Found in implementation:* C# has no generic indexers, so `update[key]` cannot carry the key's
value type. `NxUpdate<TRecord>` exposes the typed access as `Get(key)` and `Set(key, value)`
instead, which type check the same way (`update.Set(UserProperties.Name, 42)` does not compile),
plus `IsSet(key)` and `Unset(key)`. Every generated companion also gets `IsSet` and `Unset`
overloads over its `<T>_property` enum, so a decoded property value tests a patch directly.

### D6: Wire order is preserved exactly, and is not the declared order

Today's formatter writes keys in **ordinal** order, which puts `$type` first because `$` sorts below
every letter. The new formatter keeps that, so MessagePack bytes are identical for identical
patches. The order is an artifact of the old formatter, not something the native decoder reads by
position; it is kept so bytes a host recorded against the old SDK stay identical, and a .NET test
pins the exact bytes for a multi-field patch so a later change is a deliberate one. The **declared** field order is a separate, ordered list on the schema, used only by
`Changed`, which the TypeScript `nxChangedFields` also orders by declaration.

### D7: An unknown key on read throws

The generated schema makes unknown keys detectable for the first time; today's reflection formatter
silently skips them. NX itself rejects a field a record does not declare — that is what the existing
`DriftedUserPatch` test asserts on the interpreter side — so the SDK matching it removes a
divergence rather than adding a rule. The error names the key and the DTO.

The same rule covers the discriminator: a `$type` that is not the schema's `NxType` is rejected,
naming both, rather than letting a patch of one record deserialize as another whose fields happen
to overlap (found in review, RF6).

*Alternative considered:* skip unknown keys for forward compatibility with a newer peer. Rejected
for now: an update record is a patch against a schema both sides generate from the same NX source,
and silently dropping a field of a patch is a data-loss bug that surfaces far from its cause.

### D8a: The converter and formatter are named on the generated class

Neither serializer discovers a converter through a base class: System.Text.Json reads
`[JsonConverter]` with `inherit: false`, and MessagePack closes an open generic formatter over the
*annotated* type's type arguments, which a sealed companion has none of. So the companion keeps a
class-level `[MessagePackFormatter(typeof(NxUpdateRecordMessagePackFormatter<T>))]` and gains a
matching `[JsonConverter(typeof(NxUpdateRecordJsonConverter<T>))]`, the same pattern the generated
enums use. Those two attributes replace the per-property `[Key]`, `[JsonPropertyName]`, and
`[JsonIgnore]` on every field. The task list's wording that the class attribute goes away was
written before this was checked; the proposal's goal — no per-property wire attributes and no
reflection over members — holds.

### D8: What "no reflection" means precisely

The change removes reflection over the *generated type's members* — `GetProperties`, `GetValue`,
`SetValue`, and the `NxOptional<>` generic-definition probing — which is the part that defeats
trimming and NativeAOT. Field **values** are still serialized through `JsonSerializer` /
`MessagePackSerializer` by the `Type` the schema carries, which is what nested values already go
through today. The spec scenario is worded to that boundary; the design does not claim full AOT
compatibility beyond it.

## Risks / Trade-offs

- **The fixture and the emitter must land in the same commit** → `UpdateRecords.g.cs` is asserted
  byte-equal by a Rust test and compiled by the .NET tests. Regenerate with the command recorded in
  `update-records.nx` as part of the emitter task, not after it.
- **D7 is a behavior change for a drifted host DTO** → previously silent, now an exception. Both
  sides generate from the same NX source, and the message names the key. Called out in the spec so
  it is a decision, not a surprise.
- **Boxing on every field access** → bounded by patch size; no hot path serializes update records in
  bulk.
- **`Diff` must match `nxDiffRecords` exactly**, including comparing an absent field as `null` and
  structural equality for arrays and nested records → port the comparison rather than reach for
  `object.Equals`, and test the same cases the TypeScript runtime tests cover.
- **Two ways to read a field** (the named accessor and the key accessor) → acceptable; they are
  the static and dynamic halves of the same map, which is the point of the change.
- **The `UnsetOptional_OutsideAnUpdateRecord_ThrowsInsteadOfWritingNull` test goes** → it asserted
  the two `throw`s D2 retires with the converters. Every other existing assertion in the two .NET
  test files passes unchanged; JSON keys now come out in ordinal rather than declaration order
  (D6), which no existing assertion observed.

## Migration Plan

Not applicable as a deployment concern — the wire format does not move, so a host built against the
old SDK interoperates with one built against the new. Source-level, a host that referenced
`NxOptionalJsonConverterFactory` or `NxUpdateRecordMessagePackFormatter<T>` directly must drop the
reference; nothing else in the public surface changes. The repository has no backward-compatibility
obligation yet (AGENTS.md), so the old types are deleted rather than obsoleted.
