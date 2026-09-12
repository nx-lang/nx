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
list of one or more values, each of which SHALL be either an action record or an update record.
A single record result SHALL be normalized to a one-item list; a list result SHALL preserve source
order and MAY mix action records and update records.

#### Scenario: Single returned action is normalized to a one-item list
- **WHEN** a handler body evaluates to `<DoSearch search={action.searchString} />`
- **THEN** invocation SHALL return a list containing exactly one `DoSearch` action

#### Scenario: Multiple returned actions preserve order
- **WHEN** a handler body evaluates to `[<LogSearch search={action.searchString} />, <DoSearch search={action.searchString} />]`
- **THEN** invocation SHALL return both actions in source order

#### Scenario: Update records are accepted as handler results
- **WHEN** a handler bound inside `component <Counter /> = { state { count:int = 0 } ... }` evaluates to `[<Update count={count + 1} />, <Saved />]`
- **THEN** invocation SHALL return a two-item list whose first item is a `Counter.Update` value and whose second item is a `Saved` action

#### Scenario: Empty or non-action results are rejected
- **WHEN** a handler body evaluates to `[]`, `"docs"`, or a plain record such as `<User name="Ada" />`
- **THEN** invocation SHALL fail with a runtime error because action handlers must return one or more actions or update records

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

### Requirement: Handler bodies are type checked in the scope of their binding site
The type checker SHALL infer and check every `on<ActionName>` handler body. The body SHALL be
checked with the same names in scope as the element it is bound on — the enclosing component's
effective props and state fields, enclosing `let` bindings and loop variables — plus the implicit
`action` identifier bound to the emitted action's record type. A handler body whose result is
neither an action record, nor an update record, nor a non-empty list of those SHALL be rejected.

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

#### Scenario: Enclosing let bindings and loop variables are visible
- **WHEN** a file contains `component <List /> = { state { items:string[] = [] } for item in items { <Row onTapped=<Update items={remove(items, item)} /> /> } }` with a suitable `remove` function
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
- **WHEN** a file contains `type User = { name:string } action DoSearch = { search:string } let render() = <SearchBox onSearchSubmitted=[<DoSearch search={action.searchString} />, <User.Update name="Ada" />] />`
- **THEN** type checking SHALL accept the handler
- **AND** dispatching the action SHALL return both values in the effect list

#### Scenario: A mixed result list is routed item by item
- **WHEN** a file contains `action Saved = { } component <Form emits { Saved } /> = { state { dirty:boolean = true } <Button onTapped=[<Update dirty=false />, <Saved />] /> }` and the tap is dispatched
- **THEN** the component's `dirty` state SHALL become `false`
- **AND** `Saved` SHALL be emitted to the parent

### Requirement: State reads inside a handler body are live at dispatch time
When a handler body is invoked during component dispatch, every identifier that names a state field
of the enclosing component SHALL evaluate to that field's value as it stands at the moment the
handler runs, including changes applied earlier in the same dispatch batch. Identifiers that name
props, enclosing `let` bindings, or loop variables SHALL keep the values captured when the handler
was created.

#### Scenario: Two increments in one batch move twice
- **WHEN** a component `Counter` with state `count:int = 0` renders `<Button onTapped=<Update count={count + 1} /> />` and one dispatch batch invokes that handler twice
- **THEN** the next state SHALL have `count` equal to `2`

#### Scenario: A later handler sees an earlier patch
- **WHEN** a component with state `count:int = 0` renders two buttons, one bound to `<Update count=10 />` and the other to `<Update count={count * 2} />`, and one batch invokes the first then the second
- **THEN** the next state SHALL have `count` equal to `20`

#### Scenario: Captured locals keep their snapshot
- **WHEN** a component renders `for item in items { <Row onTapped=<Update selected={item} /> /> }` and the state field `items` is patched to a different list earlier in the same batch
- **THEN** a handler created for a row of the original list SHALL still bind `item` to the original row's value
