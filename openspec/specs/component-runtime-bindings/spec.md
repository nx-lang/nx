# component-runtime-bindings Specification

## Purpose
Defines the public runtime binding contract for initializing stateful component instances,
dispatching host-provided action batches, and round-tripping opaque component state snapshots.
## Requirements
### Requirement: Runtime bindings initialize components as stateful instances
The public NX runtime bindings SHALL allow a host to initialize a named component instance from a
`ProgramArtifact` using input prop values. Initialization SHALL resolve the component through the
embedded `ResolvedProgram` and SHALL return both the rendered element output and an opaque,
module-aware component-state snapshot for that instance.

#### Scenario: Initialization returns rendered output and initial state snapshot
- **WHEN** a host initializes `SearchBox` from a `ProgramArtifact` containing `component <SearchBox placeholder:string = "Find docs" /> = { state { query:string = placeholder } <TextInput value={query} placeholder={placeholder} /> }` without passing an explicit `placeholder`
- **THEN** initialization SHALL return a rendered `TextInput` element whose `value` and
  `placeholder` are both `"Find docs"`
- **AND** initialization SHALL return a component-state snapshot for that `SearchBox` instance

#### Scenario: Component without local state still initializes successfully
- **WHEN** a host initializes `Button` from a `ProgramArtifact` containing `component <Button text:string /> = { <button>{text}</button> }` with `text="Save"`
- **THEN** initialization SHALL return a rendered `button` element containing `"Save"`
- **AND** initialization SHALL return a component-state snapshot that can be used for later dispatch
  calls

### Requirement: Runtime bindings dispatch host-provided action batches
The public NX runtime bindings SHALL allow a host to dispatch an ordered list of actions against a
previously returned component-state snapshot for the same `ProgramArtifact`. Dispatch SHALL process
the action list in the order provided by the host and SHALL return both the ordered effect actions
and the next component-state snapshot.

#### Scenario: Dispatch preserves host-provided action order
- **WHEN** a host dispatches `[<SearchSubmitted searchString="docs" />, <SearchSubmitted searchString="guides" />]` against a previously returned `SearchBox` state snapshot
- **THEN** dispatch SHALL process the `"docs"` action before the `"guides"` action
- **AND** dispatch SHALL return an effect action list and a next component-state snapshot for the
  same component instance

#### Scenario: Dispatch carries state forward in this phase
- **WHEN** a host dispatches one or more actions against a component instance whose program does not
  yet support any declarative state-update actions
- **THEN** dispatch SHALL return a next component-state snapshot representing the same component
  state values as the prior snapshot
- **AND** any returned actions SHALL appear only in the effect action list

### Requirement: Runtime bindings rely on host-owned state snapshots
The public NX runtime bindings SHALL remain stateless between component lifecycle calls. Hosts SHALL
supply the component-state snapshot returned by initialization or an earlier dispatch when
dispatching later actions, and that snapshot SHALL only be valid for the same `ProgramArtifact`
revision that produced it.

#### Scenario: Saved snapshot can be reused with a fresh runtime instance
- **WHEN** a host initializes a component, stores the returned state snapshot, and later dispatches
  actions through a fresh runtime instance using that saved snapshot and the same `ProgramArtifact`
  revision
- **THEN** dispatch SHALL succeed without requiring any interpreter-held component instance from the
  initialization call
- **AND** SHALL treat the stored snapshot as the full prior component state

#### Scenario: Snapshot from a different program revision is rejected
- **WHEN** a host dispatches actions with a malformed snapshot or with a snapshot produced by a
  different `ProgramArtifact` revision
- **THEN** dispatch SHALL fail with a runtime or binding error rather than silently accepting the
  snapshot

### Requirement: Source-based component lifecycle calls gate on shared analysis
Source-based component lifecycle entry points SHALL run the shared source-analysis pipeline before
component-specific validation or interpreter execution. If source analysis returns any error
diagnostics, initialization and dispatch SHALL return those diagnostics, SHALL NOT build a
`ProgramArtifact` or `ResolvedProgram`, and SHALL NOT produce component lifecycle results.

#### Scenario: Component initialization returns aggregated static diagnostics
- **WHEN** `initialize_component_source` is called with source that contains both a lowering error
  and a component state type error
- **THEN** the call SHALL return both static diagnostics from the shared analysis phase
- **AND** the call SHALL not return rendered output
- **AND** the call SHALL not return a component-state snapshot

#### Scenario: Component dispatch rejects static errors before snapshot processing
- **WHEN** `dispatch_component_actions_source` is called with source that contains both a lowering
  error and a type error and the host also supplies an invalid snapshot
- **THEN** the call SHALL return the shared source-analysis diagnostics for the source
- **AND** the call SHALL not attempt to interpret the component dispatch
- **AND** the call SHALL not return an invalid-snapshot runtime diagnostic for that request

### Requirement: Native component lifecycle bindings are artifact-first
The native C ABI SHALL expose component initialization and dispatch only for previously built
`ProgramArtifact` handles. Hosts that need imported-library resolution SHALL first build a
transient `ProgramArtifact` against a caller-supplied `ProgramBuildContext` backed by a
`LibraryRegistry`, then execute component lifecycle calls against that artifact.

#### Scenario: Native component initialization uses a preloaded library registry through a program artifact
- **WHEN** a host loads `../question-flow` into a `LibraryRegistry`
- **AND** creates a `ProgramBuildContext` that exposes `../question-flow`
- **AND** builds a `ProgramArtifact` from source file `app/main.nx` that imports `../question-flow`
- **AND** initializes a component through the native program-artifact component API
- **THEN** initialization SHALL succeed without reloading `../question-flow` during that call

#### Scenario: Native component dispatch reuses the same program artifact
- **WHEN** a host loads `../question-flow` into a `LibraryRegistry`
- **AND** creates a `ProgramBuildContext` that exposes `../question-flow`
- **AND** builds a `ProgramArtifact` from component source that imports `../question-flow`
- **AND** dispatches actions through the native program-artifact dispatch API
- **THEN** dispatch SHALL reuse the already built artifact and its selected loaded library
  snapshots

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
