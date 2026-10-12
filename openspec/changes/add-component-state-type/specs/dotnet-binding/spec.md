## MODIFIED Requirements

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
patch's target, including a component's `<Name>_state` contract for its `<Name>_state_update`.
Merge and changed SHALL be available on every update DTO, including those for abstract records,
which have no instantiable plain record type. A component has no update DTO of its own.

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
- **WHEN** a C# caller merges two `Stepped_update` values for an abstract record `Stepped`
- **THEN** the merge SHALL succeed and asking which fields changed SHALL yield `Stepped_property` values
