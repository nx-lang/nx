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
- **WHEN** a component instance created from `<SearchBox onSearchSubmitted={ <LogSearch search={action.searchString} /> <DoSearch search={action.searchString} /> } onValueChanged=<TrackSearch value={action.value} /> />` dispatches the batch `{ <SearchSubmitted searchString="docs" /> <SearchBox.ValueChanged value="docs" /> }`
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
neither an action record, nor an update record, nor a `+` sequence of those SHALL be rejected; in
particular a result whose occurrence admits zero — `{}`, an `if` with no `else`, a `?` or `*` value —
SHALL be rejected, naming the result type against the `+` the handler requires.

#### Scenario: Action payload fields are typed inside the body
- **WHEN** a file contains `external component <Slider emits { EndChanged { value:float64 } } />` and `component <Volume /> = { state { level:float64 = 0.5 } <Slider onEndChanged=<State.Update level={action.value} /> /> }`
- **THEN** type checking SHALL accept the handler because `action.value` is `float64` and `level` is `float64`

#### Scenario: A mistyped update field in a handler is rejected
- **WHEN** a file contains `external component <Slider emits { EndChanged { value:float64 } } />` and `component <Volume /> = { state { level:int = 0 } <Slider onEndChanged=<State.Update level={action.value} /> /> }`
- **THEN** type checking SHALL reject the handler because `action.value` is `float64` and `level` is `int`

#### Scenario: A misspelled state field in a handler is rejected
- **WHEN** a file contains `component <Counter /> = { state { count:int = 0 } <Button onTapped=<State.Update cont={count + 1} /> /> }`
- **THEN** type checking SHALL reject `cont` as an unknown field of `Counter.State.Update`

#### Scenario: A handler whose result is not an action or update is rejected statically
- **WHEN** a file contains `component <Counter /> = { state { count:int = 0 } <Button onTapped={count + 1} /> }`
- **THEN** type checking SHALL reject the handler because its result type `int` is neither an action nor an update record

#### Scenario: A handler whose result may be empty is rejected statically
- **WHEN** a file contains `component <Counter /> = { state { count:int = 0 } <Button onTapped={ if count < 10 { <State.Update count={count + 1} /> } } /> }`
- **THEN** type checking SHALL reject the handler, naming `Counter.State.Update?` against a result that must be `+`
- **AND** the same handler with an `else` arm returning `<State.Update />` SHALL be accepted

#### Scenario: Enclosing let bindings and loop variables are visible
- **WHEN** a file contains `component <List /> = { state { items?:string+ } for item in items { <Row onTapped=<State.Update items={remove(items, item)} /> /> } }` with a suitable `remove` function
- **THEN** type checking SHALL accept the reference to the loop variable `item` inside the handler

### Requirement: Handler results are routed by type to state, parent, or host
The system SHALL route each value a handler returns by its type. A value whose type is the
update record of the enclosing component's state, `<Component>.State.Update`, SHALL patch that
component's state. An action the enclosing component declares or inherits in `emits` SHALL be
emitted to the component's parent. Any other value SHALL be a host effect when the handler is bound
outside any component body, and SHALL be a compile error when the handler is bound inside a
component body. Type checking SHALL enforce this statically for every handler result.

#### Scenario: State update of the enclosing component is accepted
- **WHEN** a file contains `component <Counter /> = { state { count:int = 0 } <Button onTapped=<State.Update count={count + 1} /> /> }`
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
- **THEN** type checking SHALL reject the handler because `User.Update` is not `Form.State.Update`

#### Scenario: The component's name with `.Update` is an unknown name inside its body
- **WHEN** a file contains `component <Counter step:int = 1 /> = { state { count:int = 0 } <Button onTapped=<Counter.Update step=2 /> /> }`
- **THEN** type checking SHALL reject `Counter.Update` as an unknown type, since a component has no update record
- **AND** the diagnostic SHALL suggest `Counter.State.Update`

#### Scenario: Any action or update record is a host effect at the root
- **WHEN** a file contains `type User = { name:string } action DoSearch = { search:string } let render() = <SearchBox onSearchSubmitted={ <DoSearch search={action.searchString} /> <User.Update name="Ada" /> } />`
- **THEN** type checking SHALL accept the handler
- **AND** dispatching the action SHALL return both values in the effect list

#### Scenario: A mixed result list is routed item by item
- **WHEN** a file contains `action Saved = { } component <Form emits { Saved } /> = { state { dirty:boolean = true } <Button onTapped={ <State.Update dirty=false /> <Saved /> } /> }` and the tap is dispatched
- **THEN** the component's `dirty` state SHALL become `false`
- **AND** `Saved` SHALL be emitted to the parent

### Requirement: State reads inside a handler body are live at dispatch time
A state field read in a handler body SHALL evaluate to the field's value at the moment the handler
runs. When a handler body is invoked during component dispatch, every identifier that names a
state field of the enclosing component SHALL evaluate to that field's value as it stands at that
moment, including changes applied earlier in the same dispatch batch. Identifiers that name
props, enclosing `let` bindings, or loop variables SHALL keep the values captured when the handler
was created.

#### Scenario: Two increments in one batch move twice
- **WHEN** a component `Counter` with state `count:int = 0` renders `<Button onTapped=<State.Update count={count + 1} /> />` and one dispatch batch invokes that handler twice
- **THEN** the next state SHALL have `count` equal to `2`

#### Scenario: A later handler sees an earlier patch
- **WHEN** a component with state `count:int = 0` renders two buttons, one bound to `<State.Update count=10 />` and the other to `<State.Update count={count * 2} />`, and one batch invokes the first then the second
- **THEN** the next state SHALL have `count` equal to `20`

#### Scenario: Captured locals keep their snapshot
- **WHEN** a component renders `for item in items { <Row onTapped=<State.Update selected={item} /> /> }` and the state field `items` is patched to a different list earlier in the same batch
- **THEN** a handler created for a row of the original list SHALL still bind `item` to the original row's value

### Requirement: A handler captures only the slots its body reads
When a runtime makes an action handler, it SHALL capture the values of the slots the handler's node
and the nodes beneath it read, as they stand at that moment, and no other slot of the frame. The
runtime SHALL find those slots from the image, as it finds a formula's captured locals, at no
charge, and MAY keep them for each handler node. The runtime SHALL hold the captured slots, and
SHALL compare them, in ascending slot index, whatever order it finds them in. The Rust and
TypeScript IR runtimes SHALL capture alike, so that two handlers made by one node compare equal
exactly when the slots that node reads held equal values, and a handler's comparison and its written
form SHALL cover only what it captured. A state field the handler reads SHALL still be read live
when the handler runs against the component that bound it. This SHALL change what `==` gives for two
handlers made by one node that differ only in a slot the node does not read: they SHALL be equal,
where they were unequal when a handler captured the whole frame. What a handler does when it runs
SHALL NOT change.

#### Scenario: A handler is equal when only a slot it does not read changed
- **WHEN** a component's body binds `<State.Update taps={taps + 1} />` in two renders between which
  only its prop `note` changed
- **THEN** the two handlers SHALL compare equal
- **AND** they SHALL compare unequal when `taps` changed

#### Scenario: Both runtimes capture alike
- **WHEN** both IR runtimes render a body that binds a handler reading one of three bound slots,
  and compare it with the handler of another render
- **THEN** both SHALL compare only that slot and count the same operations

#### Scenario: Captured slots are compared in ascending slot index
- **WHEN** both IR runtimes compare two handlers made by one node that reads slots 3 and 1, whose
  values differ in both slots
- **THEN** both SHALL compare slot 1 first and stop there, counting the same operations

### Requirement: A handler that can read the environment captures the overrides around it
When a runtime makes an action handler whose node carries the environment flag, it SHALL capture
the overrides of the `envProvide` nodes being evaluated around the node, outermost first, in the
form of `$env` entries. Each entry SHALL be one of three kinds. An override evaluated where it was
written is held with its value. A formula is held as its captured data, with the locals it captured.
A bundle applied by a block is held as one entry with its entries. The runtime SHALL capture neither
the overrides of the component's descriptor chain nor the host's root values. A handler whose node
does not carry the flag SHALL capture no overrides. The compiler SHALL set the flag exactly when the
handler body's static read set is not empty, by the rule that makes an override a formula. Capturing
SHALL cost nothing beyond evaluating the node. The Rust and TypeScript IR runtimes SHALL capture
alike.

#### Scenario: A handler that reads the environment captures the provide around it
- **WHEN** a body renders `{ provide env.mode="block" { <Button onTapped=<State.Update last={env.mode} /> /> } }`
- **THEN** the handler SHALL hold one captured override, of `mode`, with an empty path and the value
  `"block"`

#### Scenario: A formula is captured as a formula
- **WHEN** a body renders `{ provide env.gap={env.space * 2} { <Button onTapped=<State.Update last={env.gap} /> /> } }`
- **THEN** the handler SHALL hold the `gap` override as a formula, not as a value

#### Scenario: A handler that reads no environment captures nothing
- **WHEN** a body renders `{ provide env.mode="block" { <Button onTapped=<State.Update count={count + 1} /> /> } }`
- **THEN** the handler node SHALL NOT carry the environment flag, and the handler SHALL hold no
  captured overrides

#### Scenario: A read through a function sets the flag
- **WHEN** a module declares `let current() = { env.mode }` and a handler's body is
  `<State.Update last={current()} />`
- **THEN** the handler node SHALL carry the environment flag

### Requirement: A handler runs with its captured overrides inside the environment of its call
When a runtime runs an action handler, it SHALL evaluate the body in the environment the call runs
in, with the handler's captured overrides inside it, as the overrides of `envProvide` nodes around
the body would be, applied in the order they were captured. A captured value SHALL NOT be checked
against its type again. A captured formula SHALL be evaluated at each read that reaches it, in the
environment of that read, with its captured locals bound. While the formula is evaluated, a read of
the value it sets SHALL see only the sources after it. The captured overrides SHALL count as the
call's own layers for the reuse of derived results. A dispatch SHALL run each handler of its batch
in the environment the host gives for the dispatch. That is the environment of the handler's creator
for a handler the dispatched instance bound, and for a handler the host dispatches by its token
against the instance whose output holds it. A handler that runs against the instance that bound it
SHALL still read that instance's state live.

#### Scenario: A handler sees a value set around it
- **WHEN** a host dispatches the token of a handler made inside `provide env.mode="block"` against
  the instance that rendered it, in that instance's environment
- **THEN** the handler's reads of `env.mode` SHALL be `"block"`

#### Scenario: A captured formula follows the environment of the dispatch
- **WHEN** a handler is made inside `provide env.gap={env.space * 2}`, and the host dispatches it
  with the root value of `space` set to `5`
- **THEN** the handler's read of `env.gap` SHALL be `10`, whatever `space` was when it was made

#### Scenario: Both runtimes run handlers alike
- **WHEN** both IR runtimes dispatch the lifecycles of the `environment-handlers` corpus program
- **THEN** they SHALL give the same rendered output, effects and state, and count the same
  operations

#### Scenario: An emit dispatched through the parent's token is exact
- **WHEN** `Page` renders `<Card env.space={env.space * 2} onLogged=<State.Update last={env.space} /> />`
  with `space` equal to `4` at `Page`, and a host dispatches the `onLogged` token from `Page`'s output
  against `Page`'s instance, in `Page`'s environment, with a `Card.Logged` action
- **THEN** `Page`'s `last` SHALL be `8`

#### Scenario: A child passes its environment to its parent in an action field
- **WHEN** `LocaleList` declares `emits { Picked { item:string locale:string } }` and its row
  handler is `<LocaleList.Picked item={item} locale={env.locale} />`, and a parent renders
  `<LocaleList onPicked=<State.Update picked={action.locale + " " + env.locale} /> />` with
  `env.locale` set to `"fr-FR"` around the `LocaleList` and `"en-US"` at the parent
- **THEN** the parent's handler SHALL read `action.locale` as `"fr-FR"`, the plain string the
  child's handler computed where it was written, and `env.locale` as `"en-US"`, its own value
- **AND** `action.locale` SHALL NOT be a formula: an action field is an ordinary value

### Requirement: Handler equality covers the captured overrides
Two action handlers SHALL be equal when they were made by one node and both their captured slots
and their captured overrides are equal. Captured slots SHALL be compared first, in ascending slot
index, as before. Then, for handlers of a node with the environment flag, the two lists of captured
overrides SHALL be compared as two sequences of values are compared, entry by entry, up to and
including the first pair that differs. Handlers of a node without the flag SHALL compare nothing
more than their slots. The Rust and TypeScript IR runtimes SHALL compare alike and count the same
operations. This SHALL change what `==` gives for two handlers made by one flagged node whose
captured overrides differ: they SHALL be unequal, where before this change they were equal.

#### Scenario: Handlers under different overrides are unequal
- **WHEN** a body binds `<Button onTapped=<State.Update last={env.mode} /> />` inside
  `provide env.mode={label}` in two renders between which only `label` changed
- **THEN** the two handlers SHALL compare unequal
- **AND** they SHALL compare equal when `label` is the same in both renders

#### Scenario: A handler that reads no environment ignores the overrides around it
- **WHEN** a body binds `<Button onTapped=<State.Update count={count + 1} /> />` inside
  `provide env.mode={label}` in two renders between which only `label` changed
- **THEN** the two handlers SHALL compare equal, at the cost they compared at before this change

