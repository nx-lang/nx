## ADDED Requirements

### Requirement: Managed raw-value and typed-model enum workflows remain distinct
The managed NX binding SHALL preserve canonical raw `NxValue` enum payloads for raw runtime result
workflows, while schema-aware typed model workflows SHALL use strong managed enums encoded as plain
member strings. The binding SHALL document and test that raw JSON/MessagePack payloads and typed
DTO serialization are intentionally different layers rather than interchangeable enum contracts.

#### Scenario: Managed JSON raw-value workflow preserves canonical enum identity
- **WHEN** a C# caller evaluates NX source to `JsonElement` and the result is an enum value such as
  `ThemeMode.dark`
- **THEN** the returned JSON SHALL expose the canonical raw enum object with
  `"$enum": "ThemeMode"` and `"$member": "dark"`
- **AND** the binding SHALL NOT collapse that raw JSON result to the bare string `"dark"`

#### Scenario: Managed typed MessagePack workflow uses strong enums as plain member strings
- **WHEN** a C# caller serializes or deserializes a generated typed DTO that contains
  `ThemeMode.Dark`
- **THEN** the managed typed workflow SHALL use the plain member string `dark` for MessagePack and
  JSON
- **AND** the typed DTO workflow SHALL NOT require the canonical raw enum object shape

