## ADDED Requirements

### Requirement: Managed SDK ships the prelude's types
`NxLang.Sdk` SHALL declare, in the `NxLang.Nx` namespace, a hand-written type for each prelude
declaration that generated C# can reference: `NxRange<T>` for `Range` — named to stay clear of
`System.Range` — and, under the same `Nx` prefix over the names typegen gives a companion,
`NxRange_update<T>` for `Range.Update` and `NxRange_property` with its `NxRangeProperties<T>` key
table for `Range.Property`. `NxRange<T>` SHALL expose `Start`, `End` and `EndInclusive` and SHALL
have value equality over the three; `NxRange_update<T>` SHALL expose each of those as an
`NxOptional<…>` patch with `IsSet`, `Unset`, `Changed` and `Diff`, as a generated companion does.
Each SHALL serialize through `System.Text.Json` and MessagePack to exactly the wire shape the C#
emitter would generate for a user-declared record with the same fields and the NX name `Range`, so a
range and a patch of one cross the host boundary like any other record. The SDK's tests SHALL fail
when these types stop matching the prelude's declarations.

#### Scenario: A range round-trips through both serializers
- **WHEN** a host serializes `new NxRange<long>(1, 5, false)` with `System.Text.Json` and with MessagePack and reads each result back
- **THEN** both SHALL yield a value equal to the original
- **AND** the JSON SHALL carry `start`, `end` and `endInclusive` under those names

#### Scenario: An evaluated range deserializes into the SDK type
- **WHEN** a host evaluates an NX function returning `1..=5` and deserializes the result as `NxRange<long>`
- **THEN** the value SHALL have `Start` 1, `End` 5 and `EndInclusive` true

#### Scenario: The SDK type tracks the prelude
- **WHEN** the prelude's `Range` declaration gains, loses or renames a field
- **THEN** an SDK test SHALL fail until `NxRange<T>` is updated to match

#### Scenario: A generated contract carries a patch of a range
- **WHEN** a host serializes a generated contract whose fields are typed `<Range T=int/>`, `<Range.Update T=int/>` and `Range.Property`, in both formats, and reads each result back
- **THEN** each SHALL round-trip, with the patch carrying `$type` `Range.Update` and only the fields that were set

#### Scenario: The SDK companion tracks the prelude
- **WHEN** the prelude's `Range` declaration renames a field
- **THEN** an SDK test SHALL fail until `NxRange_update<T>` is updated to match
