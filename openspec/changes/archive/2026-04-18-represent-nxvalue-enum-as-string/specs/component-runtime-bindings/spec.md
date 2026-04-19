## MODIFIED Requirements

### Requirement: Runtime bindings support caller-selected component result formats
The public NX component lifecycle bindings SHALL allow a host to request either MessagePack or JSON
for initialization and dispatch results on a per-call basis. Output-format selection SHALL affect
only the returned payload. Hosts SHALL continue to supply props and action batches in MessagePack,
and saved component snapshots SHALL continue to be passed back as opaque raw bytes. Props, action
batches, rendered values, and effect values that travel through the raw `NxValue` contract SHALL
represent enum values as the bare authored member string in both host input and runtime output;
the runtime SHALL resolve those strings against the declared NX type for each prop, action
argument, rendered field, or effect field.

#### Scenario: Component initialization returns JSON with an opaque snapshot
- **WHEN** a host initializes `SearchBox` and requests JSON output
- **THEN** initialization SHALL return a UTF-8 JSON object containing `rendered` and
  `state_snapshot`
- **AND** `state_snapshot` SHALL be a base64 string that preserves the opaque snapshot bytes

#### Scenario: Component dispatch returns JSON while consuming the saved raw snapshot
- **WHEN** a host dispatches actions using raw snapshot bytes returned from an earlier
  initialization call and requests JSON output
- **THEN** dispatch SHALL accept the raw snapshot bytes and MessagePack action list
- **AND** SHALL return a UTF-8 JSON object containing `effects` and `state_snapshot`
- **AND** the returned `state_snapshot` SHALL be a base64 string for the next opaque snapshot

#### Scenario: Component MessagePack output remains available
- **WHEN** a host initializes or dispatches a component and requests MessagePack output
- **THEN** the runtime SHALL return the existing MessagePack result payload for that call

#### Scenario: Raw enum values in component props and results are bare authored member strings
- **WHEN** a host initializes or dispatches a component whose props, rendered output, actions, or
  effects contain an enum value such as `ThemeMode.dark`
- **THEN** the raw MessagePack host input SHALL carry that enum as the bare string `"dark"` in the
  slot whose declared NX type is `ThemeMode`
- **AND** any returned raw JSON or MessagePack payload that contains that enum SHALL carry the
  bare string `"dark"` in the corresponding slot
- **AND** the payloads SHALL NOT wrap the enum value in a `"$enum"` / `"$member"` object

#### Scenario: Unknown enum member in component input is rejected
- **WHEN** a host supplies a bare string in a prop slot whose declared NX type is `ThemeMode` and
  the string does not match any authored member of `ThemeMode`
- **THEN** the binding SHALL reject the call with a type-mismatch error
- **AND** SHALL NOT silently treat the unknown member as a plain string value
