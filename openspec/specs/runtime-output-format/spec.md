# runtime-output-format Specification

## Purpose
TBD - created by archiving change support-selectable-runtime-output-format. Update Purpose after archive.
## Requirements
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

### Requirement: Raw discriminated union cases serialize as `$type` maps
Canonical raw JSON and MessagePack output SHALL represent discriminated union case values as
object/map payloads containing a string `$type` discriminator whose value is the fully scoped case
name, plus the case's declared and inherited fields. Fieldless discriminated union cases SHALL use
the same object/map shape with `$type` and no case-specific fields. Discriminated union cases SHALL
NOT serialize as bare strings; bare-string raw output remains the enum value contract.

#### Scenario: Payload union case returns `$type` and fields
- **WHEN** a host evaluates `type LoadState = | failed { message:string retryable:bool = true } let root() = { <LoadState.failed message={"Offline"} /> }`
- **AND** requests either JSON or MessagePack output
- **THEN** the returned canonical raw value SHALL include `$type` with value `LoadState.failed`
- **AND** it SHALL include fields `message` and `retryable`
- **AND** the payload SHALL NOT use an alternate union envelope shape

#### Scenario: Fieldless union case returns `$type` map instead of enum string
- **WHEN** a host evaluates `type LoadState = | idle let root() = { LoadState.idle }`
- **AND** requests either JSON or MessagePack output
- **THEN** the returned canonical raw value SHALL include `$type` with value `LoadState.idle`
- **AND** the returned canonical raw value SHALL NOT be the bare string `"idle"`

#### Scenario: Enum bare-string output remains unchanged
- **WHEN** a host evaluates `enum CardSortMode = closed | open let root() = { CardSortMode.closed }`
- **AND** requests either JSON or MessagePack output
- **THEN** the returned canonical raw value SHALL remain the bare string `"closed"`
- **AND** the value SHALL NOT include a `$type` discriminator

