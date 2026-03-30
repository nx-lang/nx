# component-action-handlers Specification

## Purpose
TBD - created by archiving change add-component-action-handlers. Update Purpose after archive.
## Requirements
### Requirement: Matching handler bindings lower as lazy callbacks
The system SHALL lower a matching component action handler binding as a lazy callback expression
instead of eagerly evaluating the handler body when the surrounding component invocation is
interpreted.

#### Scenario: Handler body is not executed when the component invocation is evaluated
- **WHEN** a file contains `action SearchSubmitted = { searchString:string }`, `component <SearchBox emits { SearchSubmitted } /> = { <TextInput /> }`, and `let render(userId:string) = <SearchBox onSearchSubmitted=<DoSearch userId={userId} search={action.searchString} /> />`
- **THEN** interpreting `render("u1")` SHALL succeed without requiring `action` to exist at component invocation time and SHALL retain a callable handler value that captures `userId`

### Requirement: Handler bindings must match declared emitted actions
The system SHALL validate component action handler bindings against the target component's declared
emits and SHALL reject ambiguous signature shapes.

#### Scenario: Unknown emitted action handler is rejected
- **WHEN** a file contains `component <SearchBox emits { SearchSubmitted } /> = { <TextInput /> }` and `<SearchBox onSearchRequested=<DoSearch search={action.searchString} /> />`
- **THEN** lowering SHALL fail because `SearchBox` does not emit `SearchRequested`

#### Scenario: Prop names cannot collide with generated handler names
- **WHEN** a file contains `component <SearchBox onSearchSubmitted:string emits { SearchSubmitted } /> = { <TextInput /> }`
- **THEN** lowering SHALL fail because the declared prop name collides with the handler binding name for emitted action `SearchSubmitted`

### Requirement: Handler invocation binds the implicit action value
The interpreter SHALL allow a lowered component action handler to be invoked with an emitted action
value and SHALL expose that value to the handler body through the implicit `action` identifier.

#### Scenario: Shared action payload fields are readable through `action`
- **WHEN** a lowered `onSearchSubmitted` handler is invoked with `<SearchSubmitted searchString="docs" />`
- **THEN** the interpreter SHALL evaluate `action.searchString` inside the handler body as `"docs"`

#### Scenario: Inline emitted action payload fields are readable through `action`
- **WHEN** a lowered `onValueChanged` handler for `ValueChanged { value:string }` is invoked with `<SearchBox.ValueChanged value="docs" />`
- **THEN** the interpreter SHALL evaluate `action.value` inside the handler body as `"docs"`

### Requirement: Handler invocation returns a non-empty action list
The interpreter SHALL normalize the result of invoking a component action handler into an ordered
list of one or more action values.

#### Scenario: Single returned action is normalized to a one-item list
- **WHEN** a handler body evaluates to `<DoSearch search={action.searchString} />`
- **THEN** invocation SHALL return a list containing exactly one `DoSearch` action

#### Scenario: Multiple returned actions preserve order
- **WHEN** a handler body evaluates to `[<LogSearch search={action.searchString} />, <DoSearch search={action.searchString} />]`
- **THEN** invocation SHALL return both actions in source order

#### Scenario: Empty or non-action results are rejected
- **WHEN** a handler body evaluates to `[]` or `"docs"`
- **THEN** invocation SHALL fail with a runtime error because action handlers must return one or more actions

### Requirement: Component dispatch applies bound emitted-action handlers
The system SHALL invoke a component instance's bound `on<ActionName>` handler whenever component
dispatch receives a matching emitted action. Dispatch SHALL append the handler's normalized returned
actions to the dispatch effect action list.

#### Scenario: Shared emitted action contributes an effect during dispatch
- **WHEN** a component instance created from `<SearchBox onSearchSubmitted=<DoSearch search={action.searchString} /> />` dispatches `<SearchSubmitted searchString="docs" />`
- **THEN** dispatch SHALL return an effect action list containing exactly one `DoSearch` action with `search="docs"`

#### Scenario: Inline emitted action contributes an effect during dispatch
- **WHEN** a component instance created from `<SearchBox onValueChanged=<TrackSearch value={action.value} /> />` dispatches `<SearchBox.ValueChanged value="docs" />`
- **THEN** dispatch SHALL return an effect action list containing exactly one `TrackSearch` action with `value="docs"`

### Requirement: Dispatch preserves action-batch and handler-result order
The system SHALL preserve host-provided action order across component dispatch, and for each action
it SHALL preserve the normalized order of actions returned by the matching handler.

#### Scenario: Effects preserve both dispatch order and per-handler order
- **WHEN** a component instance created from `<SearchBox onSearchSubmitted=[<LogSearch search={action.searchString} />, <DoSearch search={action.searchString} />] onValueChanged=<TrackSearch value={action.value} /> />` dispatches `[<SearchSubmitted searchString="docs" />, <SearchBox.ValueChanged value="docs" />]`
- **THEN** dispatch SHALL return effect actions in this order: `LogSearch`, `DoSearch`, `TrackSearch`
- **AND** the `LogSearch` and `DoSearch` actions SHALL both use `search="docs"`
- **AND** the trailing `TrackSearch` action SHALL use `value="docs"`

### Requirement: Unbound emitted actions do not produce effects
The system SHALL allow component dispatch to receive emitted actions for which the current component
instance has no bound handler. Such actions SHALL contribute no effect actions in this phase.

#### Scenario: Omitted handler yields no effect actions
- **WHEN** a component instance created from `<SearchBox />` dispatches `<SearchSubmitted searchString="docs" />`
- **THEN** dispatch SHALL return an empty effect action list for that action
