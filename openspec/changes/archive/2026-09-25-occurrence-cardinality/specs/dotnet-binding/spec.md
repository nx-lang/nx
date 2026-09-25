## MODIFIED Requirements

### Requirement: Managed typed models can express absence in update records
The managed SDK SHALL let a generated update DTO distinguish a field that is unset from one that is
cleared, and SHALL ship JSON and MessagePack serialization for update DTOs that omits unset fields
on write and leaves missing keys unset on read. `null` is the .NET spelling of a cleared field —
the NX empty value — and a cleared field is legal only for a field that is optional (`?:`) in the
target, as `update-records` requires. A typed accessor on a generated update DTO SHALL expose a
field as an optional value that is either unset or set to a value, where the value type is nullable
only for a clearable field. Constructing an update DTO with an object initializer SHALL remain the
supported way to build one, so `new User_update { Email = null }` SHALL mean "clear email" and
SHALL leave every other field unset. Because a non-clearable field's value type is non-nullable, a
`null` for one is a nullability diagnostic in C#; a `null` that reaches the NX runtime for such a
field anyway SHALL be rejected when the `T.Update` value is constructed, naming the field, as the
language requires. Serialization SHALL be driven by the DTO's declared field schema rather than by
runtime reflection over its members. Raw-value workflows SHALL see an absent field as a missing key
and a cleared field as a `null` value, consistent with the canonical encoding, which writes `null`
only there.

#### Scenario: Typed update DTO round-trips absence through JSON
- **WHEN** a C# caller serializes a generated `User_update`, for `export type User = { name:string email?:string }`, with only `Email` set to `null`
- **THEN** the JSON SHALL contain `"$type"` and `"email": null` and SHALL NOT contain `"name"`
- **AND** deserializing that JSON SHALL yield `Name` unset and `Email` set to `null`

#### Scenario: Typed update DTO round-trips absence through MessagePack
- **WHEN** a C# caller serializes a generated `User_update` with only `Name` set to `"Ada"` as MessagePack and passes it as an effect or prop value
- **THEN** the native runtime SHALL decode a `User.Update` value containing only `name`

#### Scenario: Raw effect carrying an update record preserves absence
- **WHEN** a dispatch returns an update record effect with one present field
- **THEN** the raw JSON effect payload SHALL contain only `$type` and that field

#### Scenario: A cleared field is rejected where the target cannot be cleared
- **WHEN** a C# caller passes a `User_update` payload carrying `"name": null` to the native runtime, for a `User` whose `name` is `string`
- **THEN** the runtime SHALL reject it with a diagnostic naming `name`
- **AND** the same payload carrying `"email": null` for `email?:string` SHALL be accepted as a cleared `email`

#### Scenario: Update DTO serialization does not reflect over the generated type
- **WHEN** an update DTO is serialized or deserialized in either format
- **THEN** the SDK SHALL read the field names and value types from the DTO's declared schema
- **AND** SHALL NOT enumerate or invoke the DTO's members through reflection

### Requirement: Managed update records expose the fields they carry
A generated update DTO SHALL let a host read which fields the patch carries without knowing them at
compile time: enumerate the set fields with their values, test whether a named field is set, and
unset a field that is set. A field set to `null` is a carried field: `null` is how .NET spells a
cleared field, and clearing is a change the patch carries. Unsetting a field SHALL be
indistinguishable from never having set it, in memory and on the wire. Reading or enumerating
fields SHALL NOT resurrect a field the patch does not carry.

#### Scenario: Enumerating a partly filled patch yields only its set fields
- **WHEN** a C# caller builds a `User_update` with `Name` set to `"Ada"` and `Email` left alone
- **THEN** enumerating its fields SHALL yield exactly one entry, for `name`
- **AND** testing whether `email` is set SHALL report that it is not

#### Scenario: A field set to null is a carried field
- **WHEN** a C# caller builds a `User_update` with `Email` set to `null`, for a `User` whose `email` is `?:string`
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

### Requirement: Managed update records support apply, merge, diff, and changed
The managed SDK SHALL offer the four update operations the NX language and the TypeScript runtime
offer, with the same results for the same inputs.

- Applying a patch to a record SHALL return a record equal to the original except for the fields the
  patch carries, where a carried `null` clears the field: the record's property becomes `null`, the
  .NET reading of the empty value, which serializes as an omitted key.
- Merging two patches SHALL return a patch carrying every field either carries, with the second
  patch winning a field both carry.
- Diffing two records SHALL return a patch carrying exactly the fields whose values differ, reading
  a field either record leaves out as the empty value, and treating a field whose value changes to a
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
- **AND** serializing the result SHALL omit the `email` key

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

### Requirement: A function value is read through a managed function reference type
The managed NX binding SHALL provide a function reference type that a host types a function-valued
property of a rendered element with, exposing the declaring module's identity and the function's
declared name — the two fields of the rendered `Function` record. It SHALL serialize in both output
formats the binding supports, MessagePack and JSON, including when the property carries no value,
and generated C# SHALL type a function-typed member with it rather than with a host delegate, which
neither format can serialize. Calling a function value from .NET is out of scope: a host reads
which function it was handed and passes the record on.

#### Scenario: A rendered function-typed property is read as a function reference
- **WHEN** a C# caller evaluates a module `templates.nx` that declares `external component <List ItemTemplate?:<function Item:object Index:int />: string /> let <Row Item:object Index:int />: string = "r" let root() = <List ItemTemplate={Row} />` into a typed element whose `ItemTemplate` property is typed as the managed function reference type
- **THEN** the property SHALL expose the module `templates.nx` and the name `Row`
- **AND** the same evaluation as JSON SHALL render `ItemTemplate` as a `Function` record with those
  two fields

#### Scenario: A contract with a function-typed member serializes in both formats
- **WHEN** a typed element with a function-typed property is serialized and deserialized through
  MessagePack and through JSON
- **THEN** both round trips SHALL preserve the module and the name
- **AND** both SHALL succeed when the property is `null` — the .NET reading of an optional property
  the element omits — because a member type that cannot be resolved would otherwise make the whole
  containing contract unserializable

#### Scenario: Generated C# types a function-typed member as the function reference
- **WHEN** C# types are generated for `export external component <DataTable RowTemplate?:<function Item:object Index:int />: string />`
- **THEN** the generated member SHALL be the managed function reference type
- **AND** SHALL NOT be a host delegate type
