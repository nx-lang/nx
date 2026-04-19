## MODIFIED Requirements

### Requirement: Native runtime calls support caller-selected output formats
The public NX native runtime SHALL allow hosts to request either MessagePack or JSON from
value-returning runtime calls on a per-call basis. The selected format SHALL apply to both
successful result payloads and diagnostic payloads returned for that call. When a returned payload
contains canonical NX values, JSON and MessagePack SHALL agree on the same value semantics,
including a single canonical shape for enum values and polymorphic records. Raw enum values SHALL
serialize as the bare authored member string in both formats; consumers recover enum identity from
the target schema, not from the payload. Raw polymorphic records SHALL serialize as object/map
payloads that carry a string `$type` discriminator key plus declared fields in both formats.

#### Scenario: Source evaluation returns JSON directly
- **WHEN** a host evaluates `let root() = { 42 }` and requests JSON output
- **THEN** the runtime SHALL return the UTF-8 JSON payload `42`
- **AND** SHALL NOT require a separate MessagePack-to-JSON conversion call

#### Scenario: Program-artifact evaluation returns JSON diagnostics directly
- **WHEN** a host evaluates a previously built `ProgramArtifact`, the call fails with diagnostics,
  and the host requests JSON output
- **THEN** the runtime SHALL return a UTF-8 JSON diagnostics array for that failed call
- **AND** SHALL NOT return MessagePack diagnostics for that request

#### Scenario: MessagePack output remains available
- **WHEN** a host evaluates NX source or a previously built `ProgramArtifact` and requests
  MessagePack output
- **THEN** the runtime SHALL return the existing canonical MessagePack payload for that call

#### Scenario: Raw enum values serialize as bare authored member strings across JSON and MessagePack
- **WHEN** a host evaluates `let root() = { Status.active }`
- **AND** requests either JSON or MessagePack output
- **THEN** the returned canonical raw value SHALL be the bare authored member string `"active"`
- **AND** the JSON and MessagePack payloads SHALL agree on that bare-string shape
- **AND** the payload SHALL NOT wrap the enum value in a `"$enum"` / `"$member"` object

#### Scenario: Polymorphic records serialize with `$type` in both JSON and MessagePack
- **WHEN** a host evaluates a value containing a polymorphic record such as
  `SearchRequested { query: "docs" }`
- **AND** requests either JSON or MessagePack output
- **THEN** the returned canonical raw value SHALL include a string `$type` discriminator with value
  `SearchRequested`
- **AND** the remaining record fields SHALL be serialized as normal object/map entries
- **AND** the payload SHALL NOT use an alternate typed-union envelope shape for MessagePack
