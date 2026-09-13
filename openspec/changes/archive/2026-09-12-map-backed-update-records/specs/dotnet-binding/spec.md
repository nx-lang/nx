## MODIFIED Requirements

### Requirement: Managed typed models can express absence in update records
The managed SDK SHALL let a generated update DTO distinguish a field that is unset from one set to
`null`, and SHALL ship JSON and MessagePack serialization for update DTOs that omits unset fields on
write and leaves missing keys unset on read. A typed accessor on a generated update DTO SHALL expose
a field as an optional value that is either unset or set to a value, including `null`. Constructing
an update DTO with an object initializer SHALL remain the supported way to build one, so
`new User_update { Email = null }` SHALL mean "set email to null" and SHALL leave every other field
unset. Serialization SHALL be driven by the DTO's declared field schema rather than by runtime
reflection over its members. Raw-value workflows SHALL see an absent field as a missing key and a
present `null` as a `null` value, consistent with the canonical encoding.

#### Scenario: Typed update DTO round-trips absence through JSON
- **WHEN** a C# caller serializes a generated `User_update` with only `Email` set to `null`
- **THEN** the JSON SHALL contain `"$type"` and `"email": null` and SHALL NOT contain `"name"`
- **AND** deserializing that JSON SHALL yield `Name` unset and `Email` set to `null`

#### Scenario: Typed update DTO round-trips absence through MessagePack
- **WHEN** a C# caller serializes a generated `User_update` with only `Name` set to `"Ada"` as MessagePack and passes it as an effect or prop value
- **THEN** the native runtime SHALL decode a `User.Update` value containing only `name`

#### Scenario: Raw effect carrying an update record preserves absence
- **WHEN** a dispatch returns an update record effect with one present field
- **THEN** the raw JSON effect payload SHALL contain only `$type` and that field

#### Scenario: Update DTO serialization does not reflect over the generated type
- **WHEN** an update DTO is serialized or deserialized in either format
- **THEN** the SDK SHALL read the field names and value types from the DTO's declared schema
- **AND** SHALL NOT enumerate or invoke the DTO's members through reflection

## ADDED Requirements

### Requirement: Managed update records expose the fields they carry
A generated update DTO SHALL let a host read which fields the patch carries without knowing them at
compile time: enumerate the set fields with their values, test whether a named field is set, and
unset a field that is set. Unsetting a field SHALL be indistinguishable from never having set it,
in memory and on the wire. Reading or enumerating fields SHALL NOT resurrect a field the patch does
not carry.

#### Scenario: Enumerating a partly filled patch yields only its set fields
- **WHEN** a C# caller builds a `User_update` with `Name` set to `"Ada"` and `Email` left alone
- **THEN** enumerating its fields SHALL yield exactly one entry, for `name`
- **AND** testing whether `email` is set SHALL report that it is not

#### Scenario: A field set to null is a carried field
- **WHEN** a C# caller builds a `User_update` with `Email` set to `null`
- **THEN** enumerating its fields SHALL yield an entry for `email` whose value is `null`
- **AND** testing whether `email` is set SHALL report that it is

#### Scenario: Unsetting a field removes it from the patch
- **WHEN** a C# caller sets `Name` on a `User_update` and then unsets it
- **THEN** enumerating its fields SHALL yield no entry for `name`
- **AND** the serialized patch SHALL NOT contain a `name` key
- **AND** reading the `Name` accessor SHALL report it as unset

#### Scenario: An unknown key on read is rejected
- **WHEN** a C# caller deserializes a patch whose object carries a key the generated update DTO does not declare
- **THEN** the SDK SHALL fail with an error naming that key
- **AND** SHALL NOT silently drop it, matching how the NX runtime treats a field a record does not declare

#### Scenario: A discriminator naming another record is rejected
- **WHEN** a C# caller deserializes a `User_update` from a payload whose `$type` is `Form.Update`
- **THEN** the SDK SHALL fail with an error naming both discriminators
- **AND** SHALL NOT produce a `User_update`

### Requirement: Managed SDK exposes typed property keys over generated records
The managed SDK SHALL provide a typed property key that names one field of a generated record and
carries that field's value type, together with read and write access to that field on an instance of
the record. A key SHALL be obtainable from the generated `<T>_property` companion value for the same
field, so a property reference that crosses the wire as a `<T>_property` can be turned into a key
without a second naming of the field. Reading and writing an update DTO through a key SHALL use
the key's value type, so no cast is required and a mistyped value is a compile error.

#### Scenario: Accessing a patch through a typed key preserves the field's type
- **WHEN** a C# caller assigns `"Ada"` through the `name` key of a `User_update` and reads the `email` key back
- **THEN** the assignment SHALL type check against `string` and the read SHALL yield an optional nullable `string`
- **AND** the patch SHALL then carry `name` and SHALL NOT carry `email`

#### Scenario: A property companion value resolves to its key
- **WHEN** a C# caller holds a `User_property` value decoded from a dispatch result
- **THEN** the SDK SHALL resolve it to the key for that field of `User`
- **AND** testing whether the update carries that property SHALL be possible from the `User_property` value alone

### Requirement: Managed update records support apply, merge, diff, and changed
The managed SDK SHALL offer the four update operations the NX language and the TypeScript runtime
offer, with the same results for the same inputs.

- Applying a patch to a record SHALL return a record equal to the original except for the fields the
  patch carries, where a carried `null` sets the field to `null`.
- Merging two patches SHALL return a patch carrying every field either carries, with the second
  patch winning a field both carry.
- Diffing two records SHALL return a patch carrying exactly the fields whose values differ, comparing
  a field either record leaves out as `null`, and treating a field whose value changes to a
  different concrete type as differing even when the two values have equal fields.
- Asking which fields a patch changed SHALL return the carried fields as `<T>_property` values, in
  the record's declared field order.

Apply and diff SHALL be available where an instantiable generated plain record type exists for the
patch's target. Merge and changed SHALL be available on every update DTO, including those for
actions and component state, which have no plain record type, and those for abstract records, which
have no instantiable one.

#### Scenario: Apply overwrites only the carried fields
- **WHEN** a C# caller applies a `User_update` carrying only `Email` set to `null` to a `User` with a name and an email
- **THEN** the result SHALL carry the original name and a `null` email

#### Scenario: Merge lets the later patch win
- **WHEN** a C# caller merges a patch setting `Name` to `"Ada"` with a patch setting `Name` to `"Grace"` and `Email` to `null`
- **THEN** the result SHALL carry `Name` as `"Grace"` and `Email` as `null`

#### Scenario: Diff reports a polymorphic field that changes type
- **WHEN** a C# caller diffs two `Doc` values whose `shape` is a `Circle` and then a `Square` with the same `id`
- **THEN** the result SHALL carry `shape`

#### Scenario: Diff carries only the differing fields
- **WHEN** a C# caller diffs two `User` values that share a name and differ in email
- **THEN** the result SHALL carry `email` and SHALL NOT carry `name`

#### Scenario: Changed reports the carried fields in declared order
- **WHEN** a C# caller asks which fields a `User_update` carrying `email` then `name` changed
- **THEN** the result SHALL be the `User_property` values for `name` then `email`, in the order `User` declares them

#### Scenario: Merge and changed work on an update with no plain record type
- **WHEN** a C# caller merges two `Counter_update` values for a component's state
- **THEN** the merge SHALL succeed and asking which fields changed SHALL yield `Counter_property` values
