## MODIFIED Requirements

### Requirement: TypeScript runtime constructs component descriptors atomically
The TypeScript runtime SHALL evaluate component descriptor expressions as atomic descriptor
construction. Descriptor construction SHALL normalize props and content through the component's
effective prop contract and SHALL return a canonical descriptor payload without evaluating the
referenced component body.

Content binding SHALL respect the declared type of the content property. When that property's type
is a list, the bound value SHALL be a list regardless of how many children were supplied, including
exactly one. When that property's type is not a list, a single child SHALL bind to the child itself.

A handler property on a descriptor (`on<Emit>` for an emit the component declares) SHALL NOT be
treated as an unknown prop. It SHALL be kept on the descriptor beside the normalized props and
SHALL appear in canonical output as a record with `$type` `ActionHandler` carrying the public name
of the action the handler accepts. A handler property that names no emit of the component SHALL
fail with a diagnostic naming the property and the component.

#### Scenario: Descriptor construction does not render component body
- **WHEN** a prepared IR function returns `<Child label="Name" />`
- **AND** `Child` is a concrete NX component with an implementation body
- **THEN** evaluating the function SHALL return a descriptor with `$type` equal to `"Child"`
- **AND** it SHALL include normalized prop `label`
- **AND** it SHALL NOT evaluate `Child`'s implementation body

#### Scenario: External component descriptor applies inherited defaults
- **WHEN** a prepared IR program contains `external component <ShortTextQuestion extends Question />`
  where inherited prop `label` has default `"Untitled"`
- **AND** descriptor construction evaluates `<ShortTextQuestion />`
- **THEN** the returned descriptor SHALL include `$type` equal to `"ShortTextQuestion"`
- **AND** it SHALL include `label` equal to `"Untitled"`

#### Scenario: A single child of a list-typed content property binds as a list
- **WHEN** a component declares a content property typed as a list of components
- **AND** a descriptor for it is constructed with exactly one child
- **THEN** the content property SHALL be a list holding that one child
- **AND** descriptor construction SHALL NOT report a boundary type error

#### Scenario: Several children of a list-typed content property bind as a list
- **WHEN** a component declares a content property typed as a list of components
- **AND** a descriptor for it is constructed with more than one child
- **THEN** the content property SHALL be a list holding those children in the order supplied

#### Scenario: A single child of a non-list content property binds directly
- **WHEN** a component declares a content property whose type is not a list
- **AND** a descriptor for it is constructed with exactly one child
- **THEN** the content property SHALL be that child itself rather than a list

#### Scenario: List content binding matches the Rust interpreter
- **WHEN** the same NX program binds a single child to a list-typed content property
- **THEN** the value the TypeScript runtime produces for that property SHALL match the value the
  Rust interpreter produces for it

#### Scenario: A descriptor's handler property is carried, not rejected
- **WHEN** a prepared IR function returns `<Button label="Add" onTapped=<Log /> />` for
  `external component <Button label:string emits { Tapped { } } />`
- **THEN** evaluating the function SHALL return a descriptor with `label` equal to `"Add"`
- **AND** `onTapped` SHALL be a record with `$type` `ActionHandler` and `action` `Button.Tapped`
- **AND** the record SHALL carry no `token`

### Requirement: TypeScript runtime initializes and evaluates components with host-owned state
The TypeScript runtime SHALL expose component APIs that initialize a concrete component from props,
evaluate a concrete component from props and current state, validate/normalize a complete state
object, and apply a host-provided state patch to produce a normalized next state. These operations
SHALL be pure with respect to runtime-held component instances and SHALL NOT require hidden mutable
component state.

Initialization SHALL return, beside the rendered output and the initial state, an opaque instance
value the host passes back to dispatch. The instance SHALL hold everything dispatch needs: the
component, its normalized props including any handler props the parent bound, the state, and the
handlers the rendered output refers to. Every bound handler in rendered output returned by
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
- **WHEN** a caller initializes `component <Counter /> = { state { count:int = 0 } <Button onTapped=<Update count={count + 1} /> /> }`
- **THEN** the rendered `Button`'s `onTapped` SHALL be a record with `$type` `ActionHandler`,
  `action` `Button.Tapped`, and a non-empty `token`
- **AND** the result SHALL include an instance the caller can dispatch against

#### Scenario: Pure evaluation output carries no tokens
- **WHEN** a caller evaluates `Counter` through the explicit-state evaluation API
- **THEN** the rendered `Button`'s `onTapped` SHALL be an `ActionHandler` record without a `token`

#### Scenario: Tokens match the Rust runtime
- **WHEN** the same program is initialized with the same props by the TypeScript runtime and by the
  Rust runtime
- **THEN** every handler in the two rendered outputs SHALL carry the same token

## ADDED Requirements

### Requirement: TypeScript runtime dispatches action batches against a component instance
The TypeScript runtime SHALL expose a dispatch API that takes a prepared program, an instance
returned by initialization or by a previous dispatch, and an ordered batch. Each batch entry SHALL
be either an action record the component emits, which runs the handler the parent bound on the
instance's props, or a handler invocation, a record with `$type` `ActionHandlerInvocation`, a
`token` read from the instance's most recent rendered output, and the `action` record to feed it.
Dispatch SHALL process entries in order. A handler bound inside the instance's own component body
SHALL read that component's state as it stands when the handler runs, so two patches in one batch
compound; every other captured value SHALL be the value captured when the handler was created.
Every update record such a handler returns for the instance's component SHALL be applied to the
working state in order with full validation, and every other returned value SHALL be collected as
an effect. A handler the parent bound SHALL NOT read or patch the instance's state, and everything
it returns, update records included, SHALL be an effect. After the batch, the body SHALL be
re-rendered once against the next state, its handlers SHALL receive fresh tokens, and dispatch
SHALL return the rendered output, the ordered effects, the next state, and the next instance. The
instance passed in SHALL NOT be modified.

#### Scenario: Handler invocation patches the component's state
- **WHEN** a caller initializes `component <Counter /> = { state { count:int = 0 } <Button onTapped=<Update count={count + 1} /> /> }`,
  reads the token from the rendered `Button`'s `onTapped`, and dispatches one invocation of that
  token with `<Button.Tapped />`
- **THEN** dispatch SHALL return state with `count` equal to `1`
- **AND** the rendered output SHALL reflect `count` equal to `1`
- **AND** the effect list SHALL be empty

#### Scenario: State reads in a handler are live within a batch
- **WHEN** a caller dispatches two invocations of the same `onTapped=<Update count={count + 1} />`
  token in one batch against a `Counter` at `count` `0`
- **THEN** dispatch SHALL return state with `count` equal to `2`

#### Scenario: A shadowed state name keeps its captured value
- **WHEN** a component body binds `let count = 10` around `<Button onTapped=<Update count={count + 1} /> />`
  inside a component whose state field `count` is `0`
- **AND** a caller dispatches that handler
- **THEN** the update SHALL set `count` to `11`

#### Scenario: Non-update results of a handler invocation are effects
- **WHEN** a caller dispatches an invocation of a handler bound as
  `onTapped={<Update count=0 /> <Saved />}` inside a component that emits `Saved`
- **THEN** dispatch SHALL apply the update to the state
- **AND** SHALL return `Saved` as the only effect

#### Scenario: An emitted action runs the parent-bound handler
- **WHEN** a caller initializes `SearchBox` from a descriptor whose parent bound
  `onSearchSubmitted=<DoSearch search={action.searchString} />`
- **AND** dispatches `<SearchBox.SearchSubmitted searchString="docs" />` against the instance
- **THEN** dispatch SHALL return `<DoSearch search="docs" />` as the only effect
- **AND** the state SHALL be unchanged

#### Scenario: Update records returned by a parent-bound handler are effects
- **WHEN** a caller dispatches `<SearchBox.SearchSubmitted searchString="docs" />` against a
  `SearchBox` instance whose parent bound `onSearchSubmitted=<Update query={action.searchString} />`
  inside the parent's own body
- **THEN** dispatch SHALL NOT change the `SearchBox` state
- **AND** SHALL return the parent's update record in the effect list

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

### Requirement: TypeScript runtime rejects a program that requires action handlers it cannot run
A prepared program SHALL only be produced when every required feature is known to the runtime.
Because a program with a handler lists the action-handler feature, a runtime built before this
change SHALL refuse it rather than silently drop the binding.

#### Scenario: An older runtime refuses a program with handlers
- **WHEN** NX IR listing the action-handler feature is loaded by a runtime that does not know it
- **THEN** preparation SHALL fail with a diagnostic naming that feature

## MODIFIED Requirements

### Requirement: TypeScript IR runtime behavior is validated against existing NX semantics
The implementation SHALL include automated tests that emit NX IR from source or program artifacts,
execute the IR through the TypeScript runtime, and compare results against native interpreter
evaluation for the supported non-reactive subset. Component tests SHALL cover descriptor
construction, initialization, explicit state evaluation, state patch validation, conditional
content based on state, and dispatch.

#### Scenario: Function parity test compares interpreter and IR runtime
- **WHEN** a supported NX program uses primitives, arithmetic, conditionals, match expressions,
  arrays, loops, records, unions, enums, member access, and function calls
- **THEN** automated tests SHALL verify that TypeScript IR runtime output matches native
  interpreter output for the same program

#### Scenario: Component parity test compares interpreter and IR runtime
- **WHEN** a supported NX component uses props, state defaults, conditional content, child
  component descriptors, and explicit state evaluation
- **THEN** automated tests SHALL verify that TypeScript IR runtime rendered output matches native
  interpreter rendered output for the same props and state

#### Scenario: Dispatch parity test compares interpreter and IR runtime
- **WHEN** a component binds handlers that patch its state, emit to its parent, and return effects
- **AND** the same batch of handler invocations and emitted actions is dispatched through both
  runtimes
- **THEN** automated tests SHALL verify that the rendered output, including tokens, the ordered
  effects, and the next state match between the TypeScript runtime and the native interpreter
