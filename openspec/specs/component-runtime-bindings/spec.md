# component-runtime-bindings Specification

## Purpose
Defines the public runtime binding contract for initializing stateful component instances,
dispatching host-provided action batches, and round-tripping opaque component state snapshots.

## Requirements
### Requirement: Runtime bindings initialize components as stateful instances
The public NX runtime bindings SHALL allow a host to initialize a named component instance from a
lowered module using input prop values. Initialization SHALL return both the rendered element output
and an opaque component-state snapshot for that instance.

#### Scenario: Initialization returns rendered output and initial state snapshot
- **WHEN** a host initializes `SearchBox` from a module containing `component <SearchBox placeholder:string = "Find docs" /> = { state { query:string = placeholder } <TextInput value={query} placeholder={placeholder} /> }` without passing an explicit `placeholder`
- **THEN** initialization SHALL return a rendered `TextInput` element whose `value` and `placeholder` are both `"Find docs"`
- **AND** initialization SHALL return a component-state snapshot for that `SearchBox` instance

#### Scenario: Component without local state still initializes successfully
- **WHEN** a host initializes `Button` from a module containing `component <Button text:string /> = { <button>{text}</button> }` with `text="Save"`
- **THEN** initialization SHALL return a rendered `button` element containing `"Save"`
- **AND** initialization SHALL return a component-state snapshot that can be used for later dispatch calls

### Requirement: Runtime bindings dispatch host-provided action batches
The public NX runtime bindings SHALL allow a host to dispatch an ordered list of actions against a
previously returned component-state snapshot. Dispatch SHALL process the action list in the order
provided by the host and SHALL return both the ordered effect actions and the next
component-state snapshot.

#### Scenario: Dispatch preserves host-provided action order
- **WHEN** a host dispatches `[<SearchSubmitted searchString="docs" />, <SearchSubmitted searchString="guides" />]` against a previously returned `SearchBox` state snapshot
- **THEN** dispatch SHALL process the `"docs"` action before the `"guides"` action
- **AND** dispatch SHALL return an effect action list and a next component-state snapshot for the same component instance

#### Scenario: Dispatch carries state forward in this phase
- **WHEN** a host dispatches one or more actions against a component instance whose module does not yet support any declarative state-update actions
- **THEN** dispatch SHALL return a next component-state snapshot representing the same component state values as the prior snapshot
- **AND** any returned actions SHALL appear only in the effect action list

### Requirement: Runtime bindings rely on host-owned state snapshots
The public NX runtime bindings SHALL remain stateless between component lifecycle calls. Hosts SHALL
supply the component-state snapshot returned by initialization or an earlier dispatch when
dispatching later actions.

#### Scenario: Saved snapshot can be reused with a fresh runtime instance
- **WHEN** a host initializes a component, stores the returned state snapshot, and later dispatches actions through a fresh runtime instance using that saved snapshot
- **THEN** dispatch SHALL succeed without requiring any interpreter-held component instance from the initialization call
- **AND** SHALL treat the stored snapshot as the full prior component state

#### Scenario: Invalid state snapshot is rejected
- **WHEN** a host dispatches actions with a malformed or incompatible component-state snapshot
- **THEN** dispatch SHALL fail with a runtime or binding error rather than silently accepting the snapshot

### Requirement: Source-based component lifecycle calls gate on shared analysis
Source-based component lifecycle entry points SHALL run the shared source-analysis pipeline before
component-specific validation or interpreter execution. If source analysis returns any error
diagnostics, initialization and dispatch SHALL return those diagnostics and SHALL not produce
component lifecycle results.

#### Scenario: Component initialization returns aggregated static diagnostics
- **WHEN** `initialize_component_source` is called with source that contains both a lowering error and a component state type error
- **THEN** the call SHALL return both static diagnostics from the shared analysis phase
- **AND** the call SHALL not return rendered output
- **AND** the call SHALL not return a component-state snapshot

#### Scenario: Component dispatch rejects static errors before snapshot processing
- **WHEN** `dispatch_component_actions_source` is called with source that contains both a lowering error and a type error and the host also supplies an invalid snapshot
- **THEN** the call SHALL return the shared source-analysis diagnostics for the source
- **AND** the call SHALL not attempt to interpret the component dispatch
- **AND** the call SHALL not return an invalid-snapshot runtime diagnostic for that request
