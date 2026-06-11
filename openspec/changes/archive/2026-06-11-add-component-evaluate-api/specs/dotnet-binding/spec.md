## ADDED Requirements

### Requirement: Managed binding evaluates components with explicit state
The managed NX binding SHALL expose `EvaluateComponent` APIs that evaluate a named component with
caller-provided props and caller-provided current state, then return the rendered component body.
These APIs SHALL NOT expose or require `StateSnapshot`, SHALL NOT dispatch actions, and SHALL NOT
return effect actions.

#### Scenario: Managed typed component evaluation returns rendered element
- **WHEN** a C# caller evaluates `SearchBox` from source containing `component <SearchBox placeholder:string = "Find docs" /> = { state { query:string } <TextInput value={query} placeholder={placeholder} /> }` with state `{ query = "docs" }`
- **THEN** `NxRuntime.EvaluateComponent<SearchBoxProps, SearchBoxState, TextInputElement>` SHALL
  return a `TextInputElement` whose `Value` is `"docs"`
- **AND** the returned `TextInputElement.Placeholder` SHALL be `"Find docs"`

#### Scenario: Managed component evaluation does not return lifecycle fields
- **WHEN** a C# caller evaluates a component through `EvaluateComponent`
- **THEN** the managed result SHALL be the rendered element type requested by the caller
- **AND** the API SHALL NOT require a prior `StateSnapshot`
- **AND** the API SHALL NOT return a `StateSnapshot`
- **AND** the API SHALL NOT return an effect action collection

#### Scenario: Managed component evaluation rejects invalid state input
- **WHEN** a C# caller evaluates a component whose state declaration requires `query:string`
- **AND** the supplied state payload omits `query`
- **THEN** the managed binding SHALL throw `NxEvaluationException` with diagnostics from the native
  runtime instead of silently rendering with missing state

### Requirement: Managed component evaluation supports source and artifact workflows
The managed binding SHALL expose component evaluation overloads for both `NxProgramArtifact` and
source strings. Source-string overloads SHALL build a transient `NxProgramArtifact` using either the
default build context or a caller-provided `NxProgramBuildContext`, then invoke the artifact-first
native evaluation entry point.

#### Scenario: Managed program-artifact evaluation reuses imported libraries
- **WHEN** a C# caller builds a `NxProgramArtifact` from source that imports a library through a
  preconfigured `NxProgramBuildContext`
- **AND** the caller evaluates a component from that artifact
- **THEN** the managed binding SHALL evaluate the component using the already-built artifact and its
  selected library snapshots

#### Scenario: Managed source evaluation uses caller build context
- **WHEN** a C# caller evaluates a component from source using an explicit `NxProgramBuildContext`
- **THEN** the managed binding SHALL build a transient `NxProgramArtifact` with that context
- **AND** SHALL evaluate the component through the native program-artifact component evaluation API

#### Scenario: Managed source evaluation reports static diagnostics
- **WHEN** a C# caller evaluates a component from source that contains static diagnostics
- **THEN** the managed binding SHALL throw `NxEvaluationException` containing those diagnostics
- **AND** SHALL NOT invoke component body evaluation

### Requirement: Managed component evaluation exposes raw JSON and typed workflows
The managed binding SHALL support component evaluation results as typed MessagePack DTOs, raw bytes
with caller-selected output format, and parsed `JsonElement` values. JSON and MessagePack outputs
SHALL represent the rendered component body directly rather than wrapping it in a lifecycle result
object.

#### Scenario: Managed JSON component evaluation returns rendered JsonElement
- **WHEN** a C# caller evaluates `SearchBox` and requests JSON output
- **THEN** `NxRuntime.EvaluateComponentJson` SHALL return a `JsonElement` representing the rendered
  component body
- **AND** the JSON value SHALL NOT contain `rendered`, `state_snapshot`, or `effects` wrapper fields

#### Scenario: Managed raw-byte component evaluation returns selected format
- **WHEN** a C# caller evaluates `SearchBox` through a raw-byte component evaluation API and selects
  JSON output
- **THEN** the binding SHALL return UTF-8 JSON bytes for the rendered component body
- **AND** selecting MessagePack output SHALL return MessagePack bytes for the same rendered value

#### Scenario: Managed typed component evaluation preserves generated DTO wire rules
- **WHEN** a C# caller evaluates a component whose rendered body contains polymorphic records or enum
  values
- **THEN** typed DTO deserialization SHALL use the existing `$type` record and bare-string enum
  contracts shared by root evaluation and component initialization
