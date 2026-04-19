## ADDED Requirements

### Requirement: Managed raw-value and typed-model enum workflows share a single bare-string wire shape
The managed NX binding SHALL represent enum values as the bare authored NX member string across
both raw `NxValue` runtime-result workflows and schema-aware typed-model workflows. JSON and
MessagePack output from raw runtime calls, typed DTO serialization, and the shared
`NxEnumJsonConverter` / `NxEnumMessagePackFormatter` helpers SHALL produce and consume the same
string representation for a given enum member. The binding SHALL document and test that the raw
and typed layers share this wire shape rather than presenting it as two distinct enum contracts.

#### Scenario: Managed JSON raw-value workflow emits a bare authored member string
- **WHEN** a C# caller evaluates NX source to `JsonElement` and the result is an enum value such as
  `ThemeMode.dark`
- **THEN** the returned JSON SHALL be the bare string `"dark"` in the slot typed as `ThemeMode`
- **AND** the binding SHALL NOT wrap that raw JSON result in a `"$enum"` / `"$member"` object

#### Scenario: Managed typed MessagePack workflow matches the raw-value wire shape
- **WHEN** a C# caller serializes or deserializes a generated typed DTO that contains
  `ThemeMode.Dark`
- **THEN** the managed typed workflow SHALL use the plain member string `"dark"` for MessagePack
  and JSON
- **AND** the typed DTO wire output SHALL be bit-equivalent to the raw-value wire output for the
  same enum member at the same slot

#### Scenario: Managed consumer of a raw enum string resolves it through the target type
- **WHEN** a C# caller receives a raw JSON or MessagePack result that contains the bare string
  `"dark"` at a slot whose target typed DTO property is `ThemeMode`
- **THEN** the binding SHALL map that string to `ThemeMode.Dark` through the shared
  `NxEnumJsonConverter<ThemeMode, ThemeModeWireFormat>` / `NxEnumMessagePackFormatter<...>` helpers
- **AND** SHALL reject unknown member strings with the helpers' existing
  `JsonException` / `MessagePackSerializationException` error path

## REMOVED Requirements

### Requirement: Managed raw-value and typed-model enum workflows remain distinct
**Reason**: The raw `NxValue` IR now represents enum values as bare authored member strings,
matching the typed DTO wire shape. Maintaining the two layers as intentionally distinct enum
contracts is no longer correct — they produce and consume identical bytes for any given enum
member at a given slot.

**Migration**: Consumers that previously depended on the `"$enum"` / `"$member"` shape in raw JSON
or MessagePack output must switch to reading the bare member string at that slot and interpreting
it against the target schema's declared enum type. The shared
`NxEnumJsonConverter<TEnum, TWire>` and `NxEnumMessagePackFormatter<TEnum, TWire>` helpers already
implement this mapping and can be reused for hand-written raw-value consumers.
