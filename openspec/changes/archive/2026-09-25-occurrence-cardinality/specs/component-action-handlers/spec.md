## MODIFIED Requirements

### Requirement: Handler invocation returns a non-empty action list
The interpreter SHALL normalize the result of invoking a component action handler into an ordered
sequence of one or more values — a `+` sequence as `occurrence-types` defines it — each of which
SHALL be either an action record or an update record. A single record result SHALL be normalized to
a one-item sequence by the one-level lift; a braced result SHALL preserve source order and MAY mix
action records and update records. A result that is the empty value SHALL be rejected, which is the
runtime face of the static rule that a handler's result type must not admit zero.

#### Scenario: Single returned action is normalized to a one-item list
- **WHEN** a handler body evaluates to `<DoSearch search={action.searchString} />`
- **THEN** invocation SHALL return a sequence containing exactly one `DoSearch` action

#### Scenario: Multiple returned actions preserve order
- **WHEN** a handler body evaluates to `{ <LogSearch search={action.searchString} /> <DoSearch search={action.searchString} /> }`
- **THEN** invocation SHALL return both actions in source order

#### Scenario: Update records are accepted as handler results
- **WHEN** a handler bound inside `component <Counter /> = { state { count:int = 0 } ... }` evaluates to `{ <Update count={count + 1} /> <Saved /> }`
- **THEN** invocation SHALL return a two-item sequence whose first item is a `Counter.Update` value and whose second item is a `Saved` action

#### Scenario: Empty or non-action results are rejected
- **WHEN** a handler body evaluates to `{}`, `"docs"`, or a plain record such as `<User name="Ada" />`
- **THEN** invocation SHALL fail with a runtime error because action handlers must return one or more actions or update records

### Requirement: Dispatch preserves action-batch and handler-result order
The system SHALL preserve host-provided action order across component dispatch, and for each action
it SHALL preserve the normalized order of actions returned by the matching handler.

#### Scenario: Effects preserve both dispatch order and per-handler order
- **WHEN** a component instance created from `<SearchBox onSearchSubmitted={ <LogSearch search={action.searchString} /> <DoSearch search={action.searchString} /> } onValueChanged=<TrackSearch value={action.value} /> />` dispatches the batch `{ <SearchSubmitted searchString="docs" /> <SearchBox.ValueChanged value="docs" /> }`
- **THEN** dispatch SHALL return effect actions in this order: `LogSearch`, `DoSearch`, `TrackSearch`
- **AND** the `LogSearch` and `DoSearch` actions SHALL both use `search="docs"`
- **AND** the trailing `TrackSearch` action SHALL use `value="docs"`

### Requirement: Handler bodies are type checked in the scope of their binding site
The type checker SHALL infer and check every `on<ActionName>` handler body. The body SHALL be
checked with the same names in scope as the element it is bound on — the enclosing component's
effective props and state fields, enclosing `let` bindings and loop variables — plus the implicit
`action` identifier bound to the emitted action's record type. A handler body whose result is
neither an action record, nor an update record, nor a `+` sequence of those SHALL be rejected; in
particular a result whose occurrence admits zero — `{}`, an `if` with no `else`, a `?` or `*` value —
SHALL be rejected, naming the result type against the `+` the handler requires.

#### Scenario: Action payload fields are typed inside the body
- **WHEN** a file contains `external component <Slider emits { EndChanged { value:float64 } } />` and `component <Volume /> = { state { level:float64 = 0.5 } <Slider onEndChanged=<Update level={action.value} /> /> }`
- **THEN** type checking SHALL accept the handler because `action.value` is `float64` and `level` is `float64`

#### Scenario: A mistyped update field in a handler is rejected
- **WHEN** a file contains `external component <Slider emits { EndChanged { value:float64 } } />` and `component <Volume /> = { state { level:int = 0 } <Slider onEndChanged=<Update level={action.value} /> /> }`
- **THEN** type checking SHALL reject the handler because `action.value` is `float64` and `level` is `int`

#### Scenario: A misspelled state field in a handler is rejected
- **WHEN** a file contains `component <Counter /> = { state { count:int = 0 } <Button onTapped=<Update cont={count + 1} /> /> }`
- **THEN** type checking SHALL reject `cont` as an unknown field of `Counter.Update`

#### Scenario: A handler whose result is not an action or update is rejected statically
- **WHEN** a file contains `component <Counter /> = { state { count:int = 0 } <Button onTapped={count + 1} /> }`
- **THEN** type checking SHALL reject the handler because its result type `int` is neither an action nor an update record

#### Scenario: A handler whose result may be empty is rejected statically
- **WHEN** a file contains `component <Counter /> = { state { count:int = 0 } <Button onTapped={ if count < 10 { <Update count={count + 1} /> } } /> }`
- **THEN** type checking SHALL reject the handler, naming `Counter.Update?` against a result that must be `+`
- **AND** the same handler with an `else` arm returning `<Update />` SHALL be accepted

#### Scenario: Enclosing let bindings and loop variables are visible
- **WHEN** a file contains `component <List /> = { state { items?:string+ } for item in items { <Row onTapped=<Update items={remove(items, item)} /> /> } }` with a suitable `remove` function
- **THEN** type checking SHALL accept the reference to the loop variable `item` inside the handler

### Requirement: Handler results are routed by type to state, parent, or host
The system SHALL route each value a handler returns by its type. A value whose type is the
enclosing component's own update record SHALL patch that component's state. An action the enclosing
component declares or inherits in `emits` SHALL be emitted to the component's parent. Any other
value SHALL be a host effect when the handler is bound outside any component body, and SHALL be a
compile error when the handler is bound inside a component body. Type checking SHALL enforce this
statically for every handler result.

#### Scenario: Update of the enclosing component is accepted
- **WHEN** a file contains `component <Counter /> = { state { count:int = 0 } <Button onTapped=<Update count={count + 1} /> /> }`
- **THEN** type checking SHALL accept the handler

#### Scenario: Emitted action is accepted and re-emitted to the parent
- **WHEN** a file contains `action Saved = { } component <Form emits { Saved } /> = { <Button onTapped=<Saved /> /> }`
- **THEN** type checking SHALL accept the handler
- **AND** dispatching the tap SHALL emit `Saved` from `Form` rather than applying it to `Form`'s state

#### Scenario: Action the component does not emit is rejected inside a component body
- **WHEN** a file contains `action Saved = { } component <Form /> = { <Button onTapped=<Saved /> /> }`
- **THEN** type checking SHALL reject the handler because `Form` does not emit `Saved`
- **AND** the diagnostic SHALL suggest adding `Saved` to the component's `emits`

#### Scenario: Update record of another type is rejected inside a component body
- **WHEN** a file contains `type User = { name:string } component <Form /> = { state { draft:string = "" } <Button onTapped=<User.Update name="Ada" /> /> }`
- **THEN** type checking SHALL reject the handler because `User.Update` is not `Form.Update`

#### Scenario: Any action or update record is a host effect at the root
- **WHEN** a file contains `type User = { name:string } action DoSearch = { search:string } let render() = <SearchBox onSearchSubmitted={ <DoSearch search={action.searchString} /> <User.Update name="Ada" /> } />`
- **THEN** type checking SHALL accept the handler
- **AND** dispatching the action SHALL return both values in the effect list

#### Scenario: A mixed result list is routed item by item
- **WHEN** a file contains `action Saved = { } component <Form emits { Saved } /> = { state { dirty:boolean = true } <Button onTapped={ <Update dirty=false /> <Saved /> } /> }` and the tap is dispatched
- **THEN** the component's `dirty` state SHALL become `false`
- **AND** `Saved` SHALL be emitted to the parent
