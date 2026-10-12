## MODIFIED Requirements

### Requirement: Lifecycle rendered output identifies bound handlers by token
Rendered output returned by component initialization and by dispatch SHALL represent every bound
handler as a record with `$type` `ActionHandler` carrying the public name of the action it accepts
and an opaque string token. A token SHALL be valid only for dispatch against the snapshot returned
by the same call. Rendered output returned by pure component evaluation SHALL represent handlers
the same way but SHALL NOT carry a token, since there is no snapshot to dispatch against.

#### Scenario: Initialization output carries handler tokens
- **WHEN** a host initializes a component whose body renders `<Button onTapped=<State.Update count={count + 1} /> />`
- **THEN** the rendered `Button` element's `onTapped` SHALL be a record with `$type` `ActionHandler`, `action` `Button.Tapped`, and a non-empty `token`

#### Scenario: A token from a stale snapshot is rejected
- **WHEN** a host dispatches a handler invocation using a token read from an earlier rendered output than the one the supplied snapshot was returned with
- **THEN** dispatch SHALL fail with a diagnostic naming the unknown handler token
- **AND** SHALL NOT change state or produce effects

#### Scenario: An invocation with the wrong action type is rejected
- **WHEN** a host dispatches a token bound for `Button.Tapped` together with a `Slider.EndChanged` action
- **THEN** dispatch SHALL fail with a type-mismatch diagnostic naming the expected action

#### Scenario: Pure evaluation output carries no tokens
- **WHEN** a host evaluates a component through the pure evaluation operation
- **THEN** every handler in the rendered output SHALL be an `ActionHandler` record without a `token`

### Requirement: Runtime bindings dispatch host-provided action batches
The public NX runtime bindings SHALL allow a host to dispatch an ordered batch against a previously
returned component-state snapshot for the same `ProgramArtifact`. Each batch entry SHALL be either
an action the component emits, which runs the handler its parent bound on the component's props, or
a handler invocation naming a handler token from the snapshot's most recent rendered output together
with the action to feed it. Dispatch SHALL process entries in the order provided, SHALL apply every
update record for the snapshot's component to its state in order, SHALL collect every other returned
value as an effect, SHALL re-render the component body against the next state once after the whole
batch, and SHALL return the rendered output, the ordered effect list, and the next component-state
snapshot.

#### Scenario: Dispatch preserves host-provided action order
- **WHEN** a host dispatches the batch `{ <SearchSubmitted searchString="docs" /> <SearchSubmitted searchString="guides" /> }` against a previously returned `SearchBox` state snapshot
- **THEN** dispatch SHALL process the `"docs"` action before the `"guides"` action
- **AND** dispatch SHALL return an effect action list and a next component-state snapshot for the
  same component instance

#### Scenario: Dispatch carries state forward in this phase
- **WHEN** a host dispatches a batch whose handlers return no update record for the snapshot's
  component
- **THEN** dispatch SHALL return a next component-state snapshot representing the same component
  state values as the prior snapshot
- **AND** every returned value SHALL appear only in the effect list

#### Scenario: Handler invocation patches the component's state
- **WHEN** a host initializes `component <Counter /> = { state { count:int = 0 } <Button onTapped=<State.Update count={count + 1} /> /> }`, reads the handler token from the rendered `Button`'s `onTapped`, and dispatches one invocation of that token with `<Button.Tapped />`
- **THEN** dispatch SHALL return a next snapshot whose `count` is `1`
- **AND** the returned rendered output SHALL reflect `count` equal to `1`
- **AND** the effect list SHALL be empty

#### Scenario: Non-update results of a handler invocation are effects
- **WHEN** a host dispatches an invocation of a handler bound as `onTapped={ <State.Update count=0 /> <Saved /> }` inside a component that emits `Saved`
- **THEN** dispatch SHALL apply the update to the snapshot's state
- **AND** SHALL return `Saved` as the only effect

#### Scenario: Update records returned by a parent-bound handler are effects
- **WHEN** a host dispatches `<SearchSubmitted searchString="docs" />` against a `SearchBox` instance whose parent bound `onSearchSubmitted=<State.Update query={action.searchString} />` inside the parent's own body
- **THEN** dispatch SHALL NOT change the `SearchBox` state
- **AND** SHALL return the parent's update record in the effect list for the host to apply

#### Scenario: Dispatch returns rendered output in every case
- **WHEN** a host dispatches a batch that produces only effects and no state change
- **THEN** dispatch SHALL still return the rendered output for the unchanged state
