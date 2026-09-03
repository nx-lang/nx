## MODIFIED Requirements

### Requirement: Managed raw-value and typed-model enum workflows share a single bare-string wire shape
The managed NX binding SHALL represent constant union case values as the bare authored NX case
string across both raw `NxValue` runtime-result workflows and schema-aware typed-model workflows.
JSON and MessagePack output from raw runtime calls, typed DTO serialization, and the shared
`NxEnumJsonConverter` / `NxEnumMessagePackFormatter` helpers SHALL produce and consume the same
string representation for a given case. The binding SHALL document and test that the raw and typed
layers share this wire shape rather than presenting it as two distinct contracts.

#### Scenario: Managed JSON raw-value workflow emits a bare authored member string
- **WHEN** a C# caller evaluates NX source to `JsonElement` and the result is a constant case value
  such as `ThemeMode.dark`
- **THEN** the returned JSON SHALL be the bare string `"dark"` in the slot typed as `ThemeMode`
- **AND** the binding SHALL NOT wrap that raw JSON result in a `"$enum"` / `"$member"` object

#### Scenario: Managed typed MessagePack workflow matches the raw-value wire shape
- **WHEN** a C# caller serializes or deserializes a generated typed DTO that contains
  `ThemeMode.Dark`
- **THEN** the managed typed workflow SHALL use the plain case string `"dark"` for MessagePack
  and JSON
- **AND** the typed DTO wire output SHALL be bit-equivalent to the raw-value wire output for the
  same case at the same slot

#### Scenario: Managed consumer of a raw enum string resolves it through the target type
- **WHEN** a C# caller receives a raw JSON or MessagePack result that contains the bare string
  `"dark"` at a slot whose target typed DTO property is `ThemeMode`
- **THEN** the binding SHALL map that string to `ThemeMode.Dark` through the shared
  `NxEnumJsonConverter<ThemeMode, ThemeModeWireFormat>` / `NxEnumMessagePackFormatter<...>` helpers
- **AND** SHALL reject unknown case strings with the helpers' existing
  `JsonException` / `MessagePackSerializationException` error path

### Requirement: Managed raw-value and typed-model polymorphic record workflows share a single `$type` wire shape
The managed NX binding SHALL represent polymorphic NX records with the same `$type`-discriminated
map contract across both raw `NxValue` runtime-result workflows and schema-aware typed-model
MessagePack workflows. Generated typed DTO serialization and deserialization for polymorphic NX
record families SHALL align with the canonical raw runtime shape rather than a separate
MessagePack-specific union envelope.

A polymorphic family that is a discriminated union MAY contain constant cases, whose wire form is a
bare string rather than a `$type` map. The managed polymorphic reader SHALL accept a bare string at
a slot whose target type is such a union and SHALL resolve it to that union's constant case, and the
managed polymorphic writer SHALL emit a bare string for that case. This SHALL NOT change the wire
shape of any payload case.

#### Scenario: Typed MessagePack polymorphic record serialization matches raw runtime shape
- **WHEN** a C# caller serializes a generated typed DTO value for `SearchRequested` through
  MessagePack
- **THEN** the payload SHALL encode the record as a map containing `$type: "SearchRequested"` and
  the declared record fields
- **AND** the payload SHALL NOT use a MessagePack `Union` discriminator envelope

#### Scenario: Typed MessagePack polymorphic record deserialization accepts canonical `$type` map values
- **WHEN** a C# caller deserializes MessagePack bytes produced from canonical raw runtime output for
  a polymorphic record family
- **THEN** the managed typed workflow SHALL resolve the concrete CLR type from the `$type` field
- **AND** SHALL populate declared fields using their authored NX wire names

#### Scenario: Polymorphic reader accepts a bare string for a constant case
- **WHEN** a C# caller deserializes JSON or MessagePack containing the bare string `"idle"` at a
  slot whose target type is a union with cases `idle` and `failed { message:string }`
- **THEN** the managed typed workflow SHALL resolve the value to that union's `idle` case
- **AND** serializing that value again SHALL produce the bare string `"idle"`
- **AND** an unknown bare string SHALL be rejected through the existing error path

#### Scenario: Raw-to-typed round-trip preserves polymorphic record identity
- **WHEN** a C# caller receives a polymorphic record from raw runtime output and then maps it
  through a typed DTO MessagePack workflow
- **THEN** the resulting value SHALL preserve the same concrete record identity indicated by `$type`
- **AND** the typed and raw workflows SHALL remain wire-compatible for that value
