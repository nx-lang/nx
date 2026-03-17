## ADDED Requirements

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
