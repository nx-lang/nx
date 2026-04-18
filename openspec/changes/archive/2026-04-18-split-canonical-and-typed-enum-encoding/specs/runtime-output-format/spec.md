## MODIFIED Requirements

### Requirement: Native runtime calls support caller-selected output formats
The public NX native runtime SHALL allow hosts to request either MessagePack or JSON from
value-returning runtime calls on a per-call basis. The selected format SHALL apply to both
successful result payloads and diagnostic payloads returned for that call. When a returned payload
contains canonical NX values, JSON and MessagePack SHALL preserve the same self-describing value
semantics, including explicit enum identity for raw enum values.

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

#### Scenario: Raw enum values stay self-describing across JSON and MessagePack
- **WHEN** a host evaluates `let root() = { Status.active }`
- **AND** requests either JSON or MessagePack output
- **THEN** the returned canonical raw value SHALL preserve enum identity explicitly rather than
  collapsing the value to a bare string
- **AND** the JSON and MessagePack payloads SHALL agree on the logical enum shape with
  `"$enum": "Status"` and `"$member": "active"`

