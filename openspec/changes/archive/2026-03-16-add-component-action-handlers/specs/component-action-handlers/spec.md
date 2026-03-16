## ADDED Requirements

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
