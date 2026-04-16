## ADDED Requirements

### Requirement: Managed runtime APIs expose direct JSON result workflows
The managed NX binding SHALL allow C# callers to request JSON output directly from evaluation and
component lifecycle calls. The managed JSON workflow SHALL support both pass-through raw bytes and
parsed `JsonElement` results without requiring a post-processing MessagePack-to-JSON conversion
step.

#### Scenario: C# caller requests raw JSON bytes for pass-through
- **WHEN** a C# caller evaluates NX source through the managed raw-byte API and requests JSON output
- **THEN** the binding SHALL request JSON from the native runtime for that call
- **AND** SHALL return UTF-8 JSON bytes suitable for forwarding to a client

#### Scenario: C# evaluation reads JSON as JsonElement
- **WHEN** a C# caller evaluates `let root() = { { answer: 42 } }` through the managed JSON
  workflow
- **THEN** the binding SHALL return the result as `JsonElement`
- **AND** the caller SHALL be able to read property `answer` as `42`

#### Scenario: C# component lifecycle reads JSON results with JsonElement payloads
- **WHEN** a C# caller initializes or dispatches a component through the managed JSON workflow
- **THEN** initialization SHALL return `NxComponentInitResult<JsonElement>`
- **AND** dispatch SHALL return `NxComponentDispatchResult<JsonElement>`
- **AND** the opaque `StateSnapshot` bytes SHALL remain available for later dispatch calls

### Requirement: Managed JSON support replaces debug conversion helpers
The managed NX binding SHALL expose JSON by requesting it from the runtime call itself rather than
by converting previously returned MessagePack bytes through public helper APIs.

#### Scenario: Debug conversion helpers are not part of the managed JSON workflow
- **WHEN** a consumer inspects the managed runtime API surface for JSON result support
- **THEN** the supported JSON path SHALL be direct JSON-returning runtime calls and raw-byte format
  selection
- **AND** `NxRuntime` SHALL NOT require public `ValueBytesToJson`,
  `DiagnosticsBytesToJson`, `ComponentInitResultBytesToJson`, or
  `ComponentDispatchResultBytesToJson` methods
