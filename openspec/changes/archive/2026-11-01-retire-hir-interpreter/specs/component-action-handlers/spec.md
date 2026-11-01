## MODIFIED Requirements

### Requirement: Handler invocation binds the implicit action value
The Rust and TypeScript IR runtimes SHALL allow a component action handler to be invoked with an emitted action
value and SHALL expose that value to the handler body through the implicit `action` identifier.

#### Scenario: Shared action payload fields are readable through `action`
- **WHEN** a lowered `onSearchSubmitted` handler is invoked with `<SearchSubmitted searchString="docs" />`
- **THEN** the runtime SHALL evaluate `action.searchString` inside the handler body as `"docs"`

#### Scenario: Inline emitted action payload fields are readable through `action`
- **WHEN** a lowered `onValueChanged` handler for `ValueChanged { value:string }` is invoked with `<SearchBox.ValueChanged value="docs" />`
- **THEN** the runtime SHALL evaluate `action.value` inside the handler body as `"docs"`

### Requirement: Handler invocation returns a non-empty action list
The Rust and TypeScript IR runtimes SHALL normalize the result of invoking a component action handler into an ordered
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
- **WHEN** a handler bound inside `component <Counter /> = { state { count:int = 0 } ... }` evaluates to `{ <State.Update count={count + 1} /> <Saved /> }`
- **THEN** invocation SHALL return a two-item sequence whose first item is a `Counter.State.Update` value and whose second item is a `Saved` action

#### Scenario: Empty or non-action results are rejected
- **WHEN** a handler body evaluates to `{}`, `"docs"`, or a plain record such as `<User name="Ada" />`
- **THEN** invocation SHALL fail with a runtime error because action handlers must return one or more actions or update records
