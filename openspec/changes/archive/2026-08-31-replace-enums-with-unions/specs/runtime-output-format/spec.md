## MODIFIED Requirements

### Requirement: Native runtime calls support caller-selected output formats
The public NX native runtime SHALL allow hosts to request either MessagePack or JSON from
value-returning runtime calls on a per-call basis. The selected format SHALL apply to both
successful result payloads and diagnostic payloads returned for that call. When a returned payload
contains canonical NX values, JSON and MessagePack SHALL agree on the same value semantics,
including a single canonical shape for constant union cases and polymorphic records. Raw constant
case values SHALL serialize as the bare authored case string in both formats; consumers recover the
declaring union from the target schema, not from the payload. Raw polymorphic records SHALL
serialize as object/map payloads that carry a string `$type` discriminator key plus declared fields
in both formats.

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
- **WHEN** a host evaluates `let root() = { Status.active }` where `Status` is a constant union
- **AND** requests either JSON or MessagePack output
- **THEN** the returned canonical raw value SHALL be the bare authored case string `"active"`
- **AND** the JSON and MessagePack payloads SHALL agree on that bare-string shape
- **AND** the payload SHALL NOT wrap the value in a `"$enum"` / `"$member"` object

#### Scenario: Polymorphic records serialize with `$type` in both JSON and MessagePack
- **WHEN** a host evaluates a value containing a polymorphic record such as
  `SearchRequested { query: "docs" }`
- **AND** requests either JSON or MessagePack output
- **THEN** the returned canonical raw value SHALL include a string `$type` discriminator with value
  `SearchRequested`
- **AND** the remaining record fields SHALL be serialized as normal object/map entries
- **AND** the payload SHALL NOT use an alternate typed-union envelope shape for MessagePack

## ADDED Requirements

### Requirement: Raw union cases serialize by constant-ness
Canonical raw JSON and MessagePack output SHALL represent a payload union case as an object/map
payload containing a string `$type` discriminator whose value is the fully scoped case name, plus
the case's declared and inherited fields. This SHALL include a case that declares no fields but
belongs to a union that extends an abstract base, because such a case carries the base's fields.

Canonical raw output SHALL represent a constant union case as the bare authored case string, with no
`$type` discriminator. A union MAY therefore have cases of both shapes, and the shape of a given
case SHALL be determined by that case's constant-ness rather than by any property of the union as a
whole. Consumers recover the declaring union from the target schema.

#### Scenario: Payload union case returns `$type` and fields
- **WHEN** a host evaluates `type LoadState = | failed { message:string retryable:boolean = true } let root() = { <LoadState.failed message={"Offline"} /> }`
- **AND** requests either JSON or MessagePack output
- **THEN** the returned canonical raw value SHALL include `$type` with value `LoadState.failed`
- **AND** it SHALL include fields `message` and `retryable`
- **AND** the payload SHALL NOT use an alternate union envelope shape

#### Scenario: Constant case returns a bare string
- **WHEN** a host evaluates `type LoadState = | idle let root() = { LoadState.idle }`
- **AND** requests either JSON or MessagePack output
- **THEN** the returned canonical raw value SHALL be the bare string `"idle"`
- **AND** it SHALL NOT include a `$type` discriminator

#### Scenario: A union with mixed cases serializes each case by its own shape
- **WHEN** a host evaluates `type Shape = circle | square { n:int } let root() = { <Box s={Shape.circle} t={<Shape.square n={2} />} /> }`
- **AND** requests either JSON or MessagePack output
- **THEN** the value at `s` SHALL be the bare string `"circle"`
- **AND** the value at `t` SHALL be a map carrying `$type` with value `Shape.square` and field `n`

#### Scenario: A fieldless case of a union with a base keeps the `$type` map
- **WHEN** a host evaluates `abstract type EventBase = { source:string = "ui" } type UiEvent extends EventBase = | closed let root() = { UiEvent.closed }`
- **AND** requests either JSON or MessagePack output
- **THEN** the returned canonical raw value SHALL include `$type` with value `UiEvent.closed`
- **AND** it SHALL include the inherited field `source`
- **AND** it SHALL NOT be the bare string `"closed"`

#### Scenario: Constant-union output is unchanged from the enum contract
- **WHEN** a host evaluates `type CardSortMode = closed | open let root() = { CardSortMode.closed }`
- **AND** requests either JSON or MessagePack output
- **THEN** the returned canonical raw value SHALL be the bare string `"closed"`
- **AND** the payload SHALL be byte-identical to the payload produced for the same declaration
  written as an enum before this change

## REMOVED Requirements

### Requirement: Raw discriminated union cases serialize as `$type` maps
**Reason**: The requirement mandated a `$type` map for every union case and reserved bare-string raw
output for the enum contract. With one declaration form, a fieldless case of a base-less union is
the same thing an enum member was, so it takes the bare-string shape. Replaced by *Raw union cases
serialize by constant-ness*.
**Migration**: A fieldless case of a base-less union now serializes as the bare authored case string
instead of `{"$type":"U.c"}`. Consumers that read such a value from a `$type` key must read the bare
string and recover the union from the target schema, as they already do for enum values. Payload
cases, and fieldless cases of a union with an abstract base, are unchanged.
