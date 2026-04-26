## ADDED Requirements

### Requirement: Managed binding supports discriminated union raw and typed workflows
The managed NX binding SHALL preserve discriminated union values through raw runtime-result
workflows and schema-aware generated typed DTO workflows using the canonical `$type` map shape.
JSON and MessagePack output from raw runtime calls, typed DTO serialization, and typed DTO
deserialization SHALL agree on the fully scoped case discriminator string and authored field wire
names.

#### Scenario: Managed raw JSON returns union case discriminator
- **WHEN** a C# caller evaluates NX source that returns `<LoadState.failed message={"Offline"} />`
- **AND** requests JSON output through the managed runtime API
- **THEN** the returned `JsonElement` SHALL contain `$type` with value `LoadState.failed`
- **AND** it SHALL contain field `message` with value `"Offline"`

#### Scenario: Managed typed MessagePack deserializes union case from `$type`
- **WHEN** a C# caller deserializes MessagePack bytes containing a map with `$type:
  "LoadState.failed"` and field `message`
- **AND** the target generated DTO type is the generated `LoadState` root
- **THEN** the managed typed workflow SHALL instantiate the generated `LoadState.failed` case DTO
- **AND** it SHALL populate the `message` property from the authored wire field

#### Scenario: Managed typed serialization matches raw union output
- **WHEN** a C# caller serializes a generated typed DTO value for the `LoadState.failed` case
  through JSON or MessagePack
- **THEN** the payload SHALL encode the value as a map containing `$type: "LoadState.failed"`
- **AND** the payload SHALL include the declared case fields using their authored NX wire names
- **AND** the payload SHALL NOT use a MessagePack union envelope

#### Scenario: Managed enum workflow remains separate
- **WHEN** a C# caller receives raw output for `CardSortMode.closed` and raw output for
  `LoadState.idle`
- **THEN** the managed enum workflow SHALL expose the enum value as the bare string `"closed"`
- **AND** the managed union workflow SHALL expose the union case as a map containing `$type:
  "LoadState.idle"`
