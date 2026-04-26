## ADDED Requirements

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
