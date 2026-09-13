## Why

`add-property-references` gave the language and the TypeScript IR runtime four intrinsics over the
map an update record already is — `apply`, `merge`, `diff`, and `changed` — plus a `T.Property`
constant union that names one field of a record. The .NET SDK got none of it. A generated
`<T>_update` still stores one `NxOptional<T>` per field, so a C# host cannot iterate the fields a
patch carries, unset one, merge two patches, or build one from a form's dirty fields. The storage
that blocks this is also the storage that forces two `NxOptional<T>` serializers, a per-property
`[JsonIgnore]` attribute, and a reflection-based MessagePack formatter that walks `PropertyInfo` at
runtime and defeats NativeAOT.

Doing this now is what unblocks two-way binding (`<TextInput bind=query />`), the next change in the
sequence, which needs the .NET key and helper shapes as its typegen target.

## What Changes

- **BREAKING (generated code shape only):** a generated `<T>_update` stores its fields in a map
  behind an `NxUpdateRecord` base class instead of one `NxOptional<T>` property each. The public
  property surface is unchanged — `new User_update { Email = null }` still compiles and still means
  "set email to null" — but the class now derives from an SDK base, carries a generated schema
  table, and drops its per-property wire attributes.
- The SDK gains field-level access on every update: `Fields`, `IsSet(name)`, `Unset(name)`.
- The SDK gains `NxProperty<TRecord, TValue>`, a typed key pairing a field's wire name with its
  value type and a getter and setter over the plain record. The generated `<T>Properties` table
  exposes one key per field and maps the existing `<T>_property` enum onto them, so the enum stays
  the one naming of a record's fields.
- The SDK gains `Merge` and `Changed` on `NxUpdateRecord`, and `Apply` and `Diff` on
  `NxUpdate<TRecord>`, matching `nxMergeUpdates`, `nxChangedFields`, `nxApplyUpdate`, and
  `nxDiffRecords` in the TypeScript runtime.
- **Removed:** `NxOptionalJsonConverterFactory`, `NxOptionalJsonConverter<T>`,V
  `NxOptionalMessagePackFormatter<T>`, `NxUpdateRecordMessagePackFormatter<TRecord>`, the
  `[JsonIgnore(Condition = WhenWritingDefault)]` on each generated property, and the converter
  attributes on `NxOptional<T>` itself. One `NxUpdateRecord` JSON converter and one MessagePack
  formatter replace them, both driven by the generated schema rather than reflection.
  `NxOptional<T>` survives as a pure in-memory value that never reaches a serializer, which retires
  the misuse case RF8 of the `add-update-actions` review hardened against.
- An unknown key on read is rejected rather than silently dropped, which is what NX does with a
  field a record does not declare.

Out of scope, and staying out: nested paths and list operations (that is JSON Patch, a different
design), two-way binding, and computed keys in construction. The `T.Update` type, the
element-shaped construction syntax, the absent-versus-null rule, the `$type` discriminator, and both
wire encodings do not change.

## Capabilities

### New Capabilities

None. This extends two existing capabilities.

### Modified Capabilities

- `dotnet-binding`: the requirement that managed typed models express absence in update records
  changes from "an optional-value type per property, with serialization support for it" to
  "map-backed storage with typed accessors, serialized through one converter per format". New
  requirements cover field enumeration and unsetting, typed property keys indexed by the
  `<T>_property` enum, the four update helpers, and rejection of unknown keys on read.
- `cli-code-generation`: C# generation for `<Name>_update` emits a class derived from the SDK update
  base with a schema table rather than a per-property `NxOptional<T>` surface with wire attributes,
  and emits a property-key table beside each generated record.

## Impact

- `bindings/dotnet/src/NxLang.Sdk/Serialization/NxOptionalSerialization.cs` — replaced wholesale;
  new `NxUpdateRecord.cs`, `NxUpdate.cs`, and `NxProperty.cs` alongside it.
- `bindings/dotnet/src/NxLang.Sdk/NxOptional.cs` — the converter attributes and the serialization
  half of its doc comment come off.
- `crates/nx-cli/src/typegen/languages/csharp.rs` — `emit_update` rewritten; a property-key table
  added next to the `<T>_property` enum.
- `bindings/dotnet/tests/NxLang.Sdk.Tests/Generated/UpdateRecords.g.cs` — regenerated; the Rust
  fixture test in `crates/nx-cli/src/typegen.rs` keeps it honest.
- `bindings/dotnet/tests/NxLang.Sdk.Tests/NxUpdateRecordTests.cs` and `NxPropertyReferenceTests.cs`
  — extended for the new surface; existing round-trip assertions must keep passing unchanged, since
  the wire format does not move.
- No change to the Rust runtime, the interpreter, the IR, or the TypeScript runtime.
