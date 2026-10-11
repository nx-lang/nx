## MODIFIED Requirements

### Requirement: TypeScript runtime applies a component update record to host-owned state
The state-patch operation SHALL accept the update record of the component's state record, `<Component>.State.Update`, in addition to a plain
partial state object, and SHALL apply it with the same semantics: present fields replace the
current value, absent fields keep it, a present empty value clears an optional state field so the
next state carries no key for it, and the result is validated against the state schema. A present
empty value for a state field that is not optional SHALL be rejected naming the field.

#### Scenario: A state update record patches state
- **WHEN** a caller applies `{ $type: "Counter.State.Update", count: 3 }` to current state `{ count: 1, label: "x" }` for prepared component `Counter`
- **THEN** the runtime SHALL return next state `{ count: 3, label: "x" }`

#### Scenario: A present empty clears an optional state field
- **WHEN** a caller applies `{ $type: "Search.State.Update", query: null }` to current state `{ query: "docs" }` for a prepared component whose state declares `query?:string`
- **THEN** the runtime SHALL return a next state with no `query` key
- **AND** the same patch against a component whose state declares `query:string` SHALL fail with a diagnostic naming `query`

#### Scenario: An update record for a different component is rejected
- **WHEN** a caller applies a value whose `$type` is `Other.Update` to `Counter` state
- **THEN** the runtime SHALL fail with a diagnostic naming the mismatched update record
- **AND** a value whose `$type` is `Counter.Update`, the component's props patch, SHALL fail the same way

### Requirement: TypeScript runtime dispatches action batches against a component instance
The TypeScript runtime SHALL expose a dispatch API that takes a prepared program, an instance
returned by initialization or by a previous dispatch, and an ordered batch. Each batch entry SHALL
be either an action record the component emits, which runs the handler the parent bound on the
instance's props, or a handler invocation, a record with `$type` `ActionHandlerInvocation`, a
`token` read from the instance's most recent rendered output, and the `action` record to feed it.
Dispatch SHALL process entries in order. A handler whose owner is the instance's component SHALL
read that component's state as it stands when the handler runs, so two patches in one batch
compound; every other captured value SHALL be the value captured when the handler was created.
Every update record such a handler returns for the instance's component SHALL be applied to the
working state in order with full validation, and every other returned value SHALL be collected as
an effect. A handler whose owner is another component, or none, SHALL NOT read or patch the
instance's state, and everything it returns, update records included, SHALL be an effect. After
the batch, the body SHALL be re-rendered once against the next state, its handlers SHALL receive
fresh tokens, and dispatch SHALL return the rendered output, the ordered effects, the next state,
and the next instance. The instance passed in SHALL NOT be modified.

#### Scenario: Handler invocation patches the component's state
- **WHEN** a caller initializes `component <Counter /> = { state { count:int = 0 } <Button onTapped=<State.Update count={count + 1} /> /> }`,
  reads the token from the rendered `Button`'s `onTapped`, and dispatches one invocation of that
  token with `<Button.Tapped />`
- **THEN** dispatch SHALL return state with `count` equal to `1`
- **AND** the rendered output SHALL reflect `count` equal to `1`
- **AND** the effect list SHALL be empty

#### Scenario: State reads in a handler are live within a batch
- **WHEN** a caller dispatches two invocations of the same `onTapped=<State.Update count={count + 1} />`
  token in one batch against a `Counter` at `count` `0`
- **THEN** dispatch SHALL return state with `count` equal to `2`

#### Scenario: A shadowed state name keeps its captured value
- **WHEN** a component body binds `for count in tens { <Button onTapped=<State.Update count={count + 1} /> /> }`
  with `tens` holding `10`, inside a component whose state field `count` is `0`
- **AND** a caller dispatches that handler
- **THEN** the update SHALL set `count` to `11`

#### Scenario: Non-update results of a handler invocation are effects
- **WHEN** a caller dispatches an invocation of a handler bound as
  `onTapped={<State.Update count=0 /> <Saved />}` inside a component that emits `Saved`
- **THEN** dispatch SHALL apply the update to the state
- **AND** SHALL return `Saved` as the only effect

#### Scenario: An emitted action runs the parent-bound handler
- **WHEN** a caller initializes `SearchBox` from a parent's rendered descriptor whose
  `onSearchSubmitted` is `<DoSearch search={action.searchString} />`, passing the parent's instance
- **AND** dispatches `<SearchBox.SearchSubmitted searchString="docs" />` against the instance
- **THEN** dispatch SHALL return `<DoSearch search="docs" />` as the only effect
- **AND** the state SHALL be unchanged

#### Scenario: Update records returned by a parent-bound handler are effects
- **WHEN** a caller dispatches `<SearchBox.SearchSubmitted searchString="docs" />` against a
  `SearchBox` instance whose parent bound `onSearchSubmitted=<State.Update query={action.searchString} />`
  inside the parent's own body
- **THEN** dispatch SHALL NOT change the `SearchBox` state
- **AND** SHALL return the parent's update record in the effect list

#### Scenario: A handler in a content child is unowned by the child
- **WHEN** `Page`'s body renders `<Stack><Button onTapped=<State.Update count={count + 1} /> /></Stack>`
  and a caller initializes `Stack` from `Page`'s rendered output, passing `Page`'s instance
- **AND** dispatches an invocation of the `Button`'s token against the `Stack` instance
- **THEN** dispatch SHALL return `Page.State.Update` as the only effect
- **AND** `Stack`'s state SHALL be unchanged

#### Scenario: An emitted action with no bound handler is a no-op
- **WHEN** a caller dispatches an action the component emits against an instance whose parent
  bound no handler for it
- **THEN** dispatch SHALL succeed with no effects and unchanged state
- **AND** SHALL still return rendered output and a next instance

#### Scenario: Dispatch returns rendered output and fresh tokens in every case
- **WHEN** a caller dispatches an empty batch
- **THEN** dispatch SHALL return the rendered output for the unchanged state
- **AND** the tokens in it SHALL differ from the tokens in the previous rendered output
- **AND** a token from the previous output SHALL be rejected by the next dispatch

#### Scenario: A token from a stale instance is rejected
- **WHEN** a caller dispatches a handler invocation whose token was read from an earlier rendered
  output than the one the supplied instance was returned with
- **THEN** dispatch SHALL fail with a diagnostic naming the unknown handler token
- **AND** the supplied instance SHALL remain usable

#### Scenario: An invocation with the wrong action type is rejected
- **WHEN** a caller dispatches a token bound for `Button.Tapped` together with a
  `Slider.EndChanged` action
- **THEN** dispatch SHALL fail with a type-mismatch diagnostic naming the expected action

#### Scenario: An action the component does not emit is rejected
- **WHEN** a caller dispatches `<Other.Changed />` against a component that does not emit it
- **THEN** dispatch SHALL fail with a diagnostic naming the component and the action

#### Scenario: A dispatch batch is atomic
- **WHEN** a caller dispatches a batch whose first entry patches `count` to `5` and whose second
  entry names an unknown token
- **THEN** dispatch SHALL fail with the unknown-token diagnostic
- **AND** dispatching a valid batch afterwards against the original instance SHALL observe `count`
  at its original value

#### Scenario: Update validation failures abort the batch
- **WHEN** a handler invoked during dispatch returns an update record whose field value does not
  match the state field's type
- **THEN** dispatch SHALL fail with a diagnostic naming the field
- **AND** SHALL NOT return a next instance

### Requirement: TypeScript runtime initializes and evaluates components with host-owned state
The TypeScript runtime SHALL expose component APIs that initialize a concrete component from props,
evaluate a concrete component from props and current state, validate/normalize a complete state
object, and apply a host-provided state patch to produce a normalized next state. These operations
SHALL be pure with respect to runtime-held component instances and SHALL NOT require hidden mutable
component state.

Initialization SHALL return, beside the rendered output and the initial state, an opaque instance
value the host passes back to dispatch. The instance SHALL hold everything dispatch needs: the
component, its normalized props including any handler props the parent bound, the state, and the
handlers the rendered output refers to. Initialization MAY be given a parent instance; every
`ActionHandler` record carrying a token in the props, at any depth, SHALL then be replaced by the
handler that instance holds under the token, a token it does not hold SHALL fail with a diagnostic
naming it, and such a record given without a parent SHALL fail as an unknown field. The handler
substituted SHALL be the value the parent's table holds, so a host holding both instances can
recognize it in the child's table. Initialization MAY also be given a state object; it SHALL then
be validated as a complete state for the component and used in place of the initial state, with
tokens assigned as for any initialization, so a host can re-render an instance with new props and
the state it holds. Handler props SHALL NOT be bound in the body's scope. Every bound handler in rendered output returned by
initialization SHALL be a record with `$type` `ActionHandler` carrying the public name of the
action it accepts and an opaque string token valid only for dispatch against the instance returned
by the same call. Rendered output returned by pure evaluation SHALL represent handlers the same way
but SHALL NOT carry a token. Token assignment SHALL follow the Rust runtime's scheme, so the same
program initialized with the same props yields the same tokens in both runtimes.

#### Scenario: Component initialization materializes initial state
- **WHEN** a prepared IR program contains `component <SearchBox placeholder:string = "Find docs" /> = { state { query:string = placeholder } <TextInput value={query} /> }`
- **AND** a caller initializes `SearchBox` without props
- **THEN** the runtime SHALL materialize prop `placeholder` as `"Find docs"`
- **AND** it SHALL return state with `query` equal to `"Find docs"`
- **AND** it SHALL return rendered output whose `TextInput` value is `"Find docs"`

#### Scenario: Explicit state controls evaluation
- **WHEN** a caller evaluates prepared component `SearchBox` with state `{ query: "docs" }`
- **THEN** the runtime SHALL render the component body with `query` equal to `"docs"`
- **AND** it SHALL NOT replace the supplied state field with the default expression value

#### Scenario: Host-owned state patch is validated
- **WHEN** a caller applies state patch `{ query: "guides" }` to current state
  `{ query: "docs" }` for prepared component `SearchBox`
- **THEN** the runtime SHALL return normalized next state `{ query: "guides" }`
- **AND** it SHALL validate the patched state against `SearchBox`'s declared state schema

#### Scenario: Invalid state patch is rejected
- **WHEN** a caller applies state patch `{ query: 123 }` to a component whose `query` state field
  is `string`
- **THEN** the runtime SHALL fail with a diagnostic identifying the invalid state field
- **AND** it SHALL NOT return a partially updated state object

#### Scenario: Initialization output carries handler tokens
- **WHEN** a caller initializes `component <Counter /> = { state { count:int = 0 } <Button onTapped=<State.Update count={count + 1} /> /> }`
- **THEN** the rendered `Button`'s `onTapped` SHALL be a record with `$type` `ActionHandler`,
  `action` `Button.Tapped`, and token `h1-1`
- **AND** the result SHALL include an instance the caller can dispatch against

#### Scenario: A parent's handler reaches the child through the parent's instance
- **WHEN** a caller initializes `Page`, whose body renders
  `<SearchBox onSearchSubmitted=<DoSearch search={action.searchString} /> />`
- **AND** initializes `SearchBox` with the rendered descriptor's fields as props and `Page`'s
  instance as the parent
- **THEN** initialization SHALL succeed
- **AND** `SearchBox`'s body SHALL NOT see `onSearchSubmitted` in scope

#### Scenario: A handler record without a parent instance is refused
- **WHEN** a caller initializes `SearchBox` with `onSearchSubmitted` set to an `ActionHandler`
  record and no parent instance
- **THEN** initialization SHALL fail with a diagnostic naming the field

#### Scenario: Pure evaluation output carries no tokens
- **WHEN** a caller evaluates `Counter` through the explicit-state evaluation API
- **THEN** the rendered `Button`'s `onTapped` SHALL be an `ActionHandler` record without a `token`

#### Scenario: Tokens match the Rust runtime
- **WHEN** the same program is initialized with the same props by the TypeScript runtime and by the
  Rust runtime
- **THEN** every handler in the two rendered outputs SHALL carry the same token

#### Scenario: Initialization with a supplied state keeps it
- **WHEN** a caller initializes `SearchBox` with props `{ placeholder: "Find" }` and state
  `{ query: "docs" }`
- **THEN** the runtime SHALL render the body with `query` equal to `"docs"` rather than the default
  `"Find"`
- **AND** it SHALL return state `{ query: "docs" }` and an instance whose tokens start at
  generation 1

#### Scenario: A supplied state is validated
- **WHEN** a caller initializes `SearchBox` with state `{ query: 123 }` or with a state missing
  `query`
- **THEN** the runtime SHALL fail with a diagnostic identifying the invalid or missing state field

#### Scenario: A handler resolved from the parent is the parent's own
- **WHEN** a `Page` instance's rendered output places a `Page`-bound handler inside the content of
  an authored `Stack`, and a caller initializes `Stack` from that descriptor's fields with `Page`'s
  instance as parent
- **THEN** the handler `Stack`'s instance holds under its own token SHALL be the same value `Page`'s
  instance holds under `Page`'s token
