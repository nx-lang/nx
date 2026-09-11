## ADDED Requirements

### Requirement: Managed dispatch accepts handler invocations and returns rendered output
The managed NX binding SHALL let a host build a dispatch batch that mixes emitted actions with
handler invocations — a handler token read from rendered output plus the action to feed it — and
SHALL return a dispatch result that exposes the rendered output, the ordered effects, and the next
opaque snapshot. Raw JSON and MessagePack dispatch workflows SHALL expose the same three parts.

#### Scenario: Typed dispatch of a handler invocation patches state and re-renders
- **WHEN** a C# caller initializes a `Counter` component whose rendered `Button` carries an `onTapped` handler token, then dispatches one invocation of that token with a `Button.Tapped` action
- **THEN** `NxRuntime.DispatchComponentActions` SHALL return a result whose `Rendered` reflects `count` equal to `1`
- **AND** whose `Effects` is empty
- **AND** whose `StateSnapshot` is the next opaque snapshot

#### Scenario: Managed dispatch reads handler tokens from typed rendered elements
- **WHEN** a C# caller deserializes rendered output into a typed element whose handler-bound property is typed as the managed handler reference type
- **THEN** the property SHALL expose the action name and the token
- **AND** the token SHALL be usable directly to build a handler invocation for the next dispatch

#### Scenario: Managed dispatch surfaces batch failure as an exception without a snapshot
- **WHEN** a C# caller dispatches a batch that contains a stale handler token
- **THEN** the managed binding SHALL throw `NxEvaluationException` carrying the native diagnostic
- **AND** SHALL NOT return a partial result

### Requirement: Managed typed models can express absence in update records
The managed SDK SHALL provide an optional-value type for generated update DTO properties that
distinguishes an unset property from one set to `null`, and SHALL ship JSON and MessagePack
serialization support for it that omits unset properties on write and leaves missing keys unset on
read. Raw-value workflows SHALL see an absent field as a missing key and a present `null` as a
`null` value, consistent with the canonical encoding.

#### Scenario: Typed update DTO round-trips absence through JSON
- **WHEN** a C# caller serializes a generated `User_update` with only `Email` set to `null`
- **THEN** the JSON SHALL contain `"$type"` and `"email": null` and SHALL NOT contain `"name"`
- **AND** deserializing that JSON SHALL yield `Name` unset and `Email` set to `null`

#### Scenario: Typed update DTO round-trips absence through MessagePack
- **WHEN** a C# caller serializes a generated `User_update` with only `Name` set to `"Ada"` as MessagePack and passes it as an effect or prop value
- **THEN** the native runtime SHALL decode a `User.Update` value containing only `name`

#### Scenario: Raw effect carrying an update record preserves absence
- **WHEN** a dispatch returns an update record effect with one present field
- **THEN** the raw JSON effect payload SHALL contain only `$type` and that field
