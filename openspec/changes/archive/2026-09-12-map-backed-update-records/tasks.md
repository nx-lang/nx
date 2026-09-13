## 1. SDK: map-backed storage and serialization

- [x] 1.1 Add `NxUpdateRecord` in `bindings/dotnet/src/NxLang.Sdk/` with ordinal map storage, the abstract `NxType`, an ordered field schema (wire name → value type, in declared order), `Fields`, `IsSet`, `Unset`, and the protected `Get<T>`/`Set<T>` accessor pair; verify with a hand-written derived class in the test project that sets, reads, and unsets a field.
- [x] 1.2 Add the `NxUpdateRecord` JSON converter, driven by the schema: write `$type` then set fields in ordinal key order, skip unset, read a missing key as unset and a present `null` as set-to-null, and throw naming the key and the DTO when a key is not in the schema. Verify the JSON round-trip scenarios in the `dotnet-binding` delta pass against a hand-written DTO.
- [x] 1.3 Add the `NxUpdateRecord` MessagePack formatter with the same rules and the same ordinal key order, so bytes match what `NxUpdateRecordMessagePackFormatter<T>` produces today; verify by serializing the existing fixture patches and comparing against bytes captured from the current implementation before it is deleted.
- [x] 1.4 Strip `[JsonConverter]` and `[MessagePackFormatter]` from `NxOptional<T>` and rewrite its doc comment to describe an in-memory value only, dropping the serialization-position guidance; verify `dotnet build` on the SDK succeeds with no reference to the removed converters.
- [x] 1.5 Delete `NxOptionalSerialization.cs` — both `NxOptional` converters, the converter factory, and the reflection-based `NxUpdateRecordMessagePackFormatter<TRecord>` — and verify no file under `bindings/dotnet/` still names them.

## 2. SDK: typed property keys and the four helpers

- [x] 2.1 Add `NxProperty<TRecord>` (wire name, value type) and `NxProperty<TRecord, TValue>` (getter and setter over `TRecord`) with untyped `GetValue`/`SetValue` on the base; verify with unit tests that a key reads and writes a field of a hand-written record.
- [x] 2.2 Add typed indexers on `NxUpdateRecord` over `NxProperty<TRecord, TValue>` so `update[key] = value` and `NxOptional<TValue> v = update[key]` type check without a cast; verify with a test that a wrong-typed assignment does not compile (assert via a commented case and a compiling correct case). *Landed as `Get(key)`/`Set(key, value)` on `NxUpdate<TRecord>`: C# has no generic indexers (design D5).*
- [x] 2.3 Implement `Merge` and `Changed` on `NxUpdateRecord`: merge returns a new patch where the second wins a field both carry; changed returns the carried wire names in declared order. Verify against the same inputs `crates/nx-codegen/src/tests.rs` uses for `nxMergeUpdates` and `nxChangedFields`, including a field the schema orders last.
- [x] 2.4 Add `NxUpdate<TRecord> : NxUpdateRecord` with `Apply(TRecord)` and static `Diff(before, after)` built on the key table, and project the base schema from that table. Verify apply overwrites only carried fields, with a carried `null` setting null.
- [x] 2.5 Port `nxValuesEqual` semantics for `Diff`: an absent field on either side compares as `null`, arrays compare element-wise, nested records compare structurally. Verify against the `nxDiffRecords` cases in `crates/nx-codegen/src/tests.rs` (both directions, and the equal case producing an empty patch).

## 3. Typegen: emit the new C# shape

- [x] 3.1 Rewrite `emit_update` in `crates/nx-cli/src/typegen/languages/csharp.rs`: derive from `NxUpdate<Target>` when a plain type is emitted for the target and from `NxUpdateRecord` otherwise, keep the `$type` member and the collision-avoiding discriminator name, emit accessors as `Get`/`Set` pairs, and drop the per-property `[Key]`, `[JsonPropertyName]`, and `[JsonIgnore]` attributes and the `[MessagePackFormatter]` class attribute. Verify the updated `generates_csharp_update_companions_with_optional_properties` test. *The class keeps `[MessagePackFormatter]` and gains `[JsonConverter]`, both naming the schema-driven SDK types (design D8a); the test is now `generates_csharp_update_companions_as_map_backed_dtos`.*
- [x] 3.2 Emit the field schema for a companion with no plain target type (component state, and any action for which no record class is emitted) as an ordered name→type table; verify `Counter_update` and `Saved_update` in that same test carry a schema and derive from the right base.
- [x] 3.3 Emit the `<T>Properties` key table beside each generated record, with one `NxProperty<T, TValue>` per field and `Of(<T>_property)` mapping each enum case to its key, reusing the generated `<T>_propertyWireFormat` for wire names; verify with a new typegen test covering the `dotnet-binding` and `cli-code-generation` key scenarios.
- [x] 3.4 Confirm which `ExportedUpdate.target_name` values have a matching emitted plain type (records and actions yes, components no) and make the base-class choice follow that rather than the declaration kind; verify with a typegen test over a source containing a record, an action, and a stateful component.
- [x] 3.5 Adjust the `using` emission (`needs_update_helpers`) for the types the new shape actually references; verify generated output compiles as part of task 4.1.

## 4. Fixture and .NET tests

- [x] 4.1 Regenerate `bindings/dotnet/tests/NxLang.Sdk.Tests/Generated/UpdateRecords.g.cs` with the command recorded at the top of `update-records.nx`, in the same commit as task 3; verify `checked_in_dotnet_update_record_fixture_matches_typegen_output` passes and the .NET test project builds.
- [x] 4.2 Keep every existing assertion in `NxUpdateRecordTests.cs` and `NxPropertyReferenceTests.cs` passing unchanged, since the wire format does not move; verify `dotnet test` on `NxLang.Sdk.Tests` is green before adding anything new. *One test, `UnsetOptional_OutsideAnUpdateRecord_ThrowsInsteadOfWritingNull`, asserted the converters D2 deletes and went with them; the other 112 passed unchanged.*
- [x] 4.3 Add .NET tests for the `dotnet-binding` delta's new scenarios: enumerating a partly filled patch, a `null`-valued field counting as carried, unset removing a field from the wire, an unknown key on read throwing, indexing by a typed key, resolving a `User_property` to its key, and each of apply/merge/diff/changed including `Counter_update` merging with no plain type.
- [x] 4.4 Extend `update-records.nx` only if a new scenario needs a declaration it lacks (for example an action companion), regenerating the fixture if so; verify the Rust fixture test still passes. *Added the stateful `Counter` component for the no-plain-type merge scenario.*

## 5. Close out

- [x] 5.1 Run `cargo fmt --all --check`, `cargo test --workspace`, and `dotnet test` on the .NET solution; verify all green.
- [x] 5.2 Update `docs/add-update-actions-followsup.md` section 1 to record that step 1 of the sequencing landed and that two-way binding is the remaining step, matching how sections 5 and the header already record landed work.
