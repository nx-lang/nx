# component-state-type Specification

## Purpose
Defines `T.State`, the derived record that holds a component's `state` group, with its own
`T.State.Property` and `T.State.Update`. A bare `State` names it inside the component. Components
themselves derive no `T.Update` or `T.Property`.

## Requirements
### Requirement: A component with state has a derived state record
For every component `T` that declares a `state` group, the system SHALL make a derived record named
`T.State` available without any declaration by the author, whether `T` is concrete or external.
`T.State` SHALL have exactly the fields of `T`'s state group, in declaration order, each with its
declared type and its optional mark, and SHALL carry no field defaults, so a `T.State` literal
SHALL give every field that is not optional. `T.State` SHALL be a plain record in every other
respect: it SHALL have its own derived `T.State.Update` and `T.State.Property`, SHALL be accepted
by the update intrinsics as a record, SHALL be visible and importable wherever `T` is, and SHALL NOT
be abstract, an action record, or usable as the base of an `extends` clause. A component that
declares no state SHALL have no `T.State`, and a reference to one SHALL be a diagnostic saying the
component declares no state. No other declaration SHALL have a derived `State`, so `T.State.State`,
`T.Update.State`, `T.Property.State` and `R.State` for a record `R` SHALL be rejected as unknown
types.

#### Scenario: State record has the state fields and not the props
- **WHEN** a file contains `component <Counter step:int = 1 /> = { state { count:int = 0 note?:string } <Label text={count} /> }` and `let s = <Counter.State count=3 />`
- **THEN** type checking SHALL accept `Counter.State` as a record with fields `count:int` and `note?:string`
- **AND** type checking SHALL reject `<Counter.State count=3 step=2 />` because `step` is a prop, not a state field

#### Scenario: State record carries no defaults
- **WHEN** a file contains `component <Counter /> = { state { count:int = 0 } <Label text={count} /> }` and `let s = <Counter.State />`
- **THEN** type checking SHALL reject the literal because `count` is required in `Counter.State`

#### Scenario: State record has its own update record and property union
- **WHEN** a file contains `component <Counter /> = { state { count:int = 0 label:string = "" } <Label text={label} /> }` and `let p = <Counter.State.Update count=1 />` and `let k: Counter.State.Property = {Counter.State.Property.label}`
- **THEN** type checking SHALL accept `Counter.State.Update` with fields `count` and `label`, each able to be absent
- **AND** SHALL accept `Counter.State.Property` with cases `count` then `label`

#### Scenario: Intrinsics work on a state record
- **WHEN** a file contains `component <Counter /> = { state { count:int = 0 } <Label text={count} /> }` and `let after = {apply(<Counter.State count=1 />, <Counter.State.Update count=2 />)}` and `let d = {diff(<Counter.State count=1 />, <Counter.State count=4 />)}`
- **THEN** type checking SHALL give `after` the type `Counter.State` and `d` the type `Counter.State.Update`
- **AND** evaluating `after` SHALL produce a `Counter.State` whose `count` is `2`

#### Scenario: A component without state has no state record
- **WHEN** a file contains `component <Form /> = { <Panel /> } let s: Form.State = <Form.State />`
- **THEN** type checking SHALL report a diagnostic saying `Form` declares no state and so has no state record

#### Scenario: Only a component has a derived state record
- **WHEN** a file contains `type User = { name:string } component <Counter /> = { state { count:int = 0 } <Label /> }` and `let a: User.State = <User.State />` and `let b: Counter.State.State = <Counter.State.State />`
- **THEN** type checking SHALL reject `User.State` and `Counter.State.State` as unknown types

#### Scenario: An external component with state has a state record
- **WHEN** a file contains `external component <SearchBox /> = { state { query:string } }` and `let s = <SearchBox.State query="docs" />`
- **THEN** type checking SHALL accept `SearchBox.State` as a record with the field `query:string`

#### Scenario: A state record of an imported component is available through the import
- **WHEN** library `ui` exports `component <Counter /> = { state { count:int = 0 } <Label /> }` and a module imports `Counter` from it and contains `let s = <Counter.State count=1 />`
- **THEN** type checking SHALL accept the reference without the library exporting anything named `Counter.State`

### Requirement: The bare `State` name resolves to the enclosing component's state record
A bare `State` inside a component body SHALL denote that component's own `<Component>.State`. This
holds in the component's state defaults, its rendered body expression, and any handler body bound
within it, wherever `State` is used as a type reference, as an element or record-literal tag, or as
the first segment of a member access, and regardless of any other declaration named `State`. So `State.Update` SHALL denote
`<Component>.State.Update` and `State.Property` SHALL denote `<Component>.State.Property`. Inside a
component that declares no state, a bare `State` SHALL be a diagnostic saying the component
declares no state. Outside every component body `State` SHALL be an ordinary name with no special
meaning. A resolved bare `State` SHALL be indistinguishable from the qualified form in every
downstream artifact.

#### Scenario: Bare State.Update in a handler patches the component
- **WHEN** a file contains `external component <Button emits { Tapped } />` and `component <Counter /> = { state { count:int = 0 } <Button onTapped=<State.Update count={count + 1} /> /> }`
- **THEN** type checking SHALL resolve `State.Update` inside the handler as `Counter.State.Update`
- **AND** SHALL check `count={count + 1}` against the state field `count:int`

#### Scenario: Bare State.Property names a state field
- **WHEN** a file contains `external component <Grid emits { Sorted { by:string } } />` and `component <People /> = { state { sortBy:State.Property = {State.Property.name} name:string = "" } <Grid onSorted=<State.Update sortBy=name /> /> }`
- **THEN** type checking SHALL resolve every `State.Property` as `People.State.Property`
- **AND** SHALL accept the bare case `name` at the site typed `People.State.Property`

#### Scenario: Bare State as a type
- **WHEN** a file contains `component <Counter /> = { state { count:int = 0 saved?:State } <Label /> }`
- **THEN** type checking SHALL resolve `saved?:State` as `saved?:Counter.State`

#### Scenario: Bare State takes precedence over a same-named declaration inside a component
- **WHEN** a file contains `type State = idle | busy` and `component <Counter /> = { state { count:int = 0 } <Button onTapped=<State.Update count=1 /> /> }` and `let s: State = idle`
- **THEN** inside the `Counter` body `State` SHALL denote `Counter.State`
- **AND** outside it `State` SHALL denote the declared union, so `let s: State = idle` SHALL type check

#### Scenario: Bare State in a component without state is rejected
- **WHEN** a file contains `component <Form /> = { <Button onTapped=<State.Update /> /> }`
- **THEN** type checking SHALL report exactly one diagnostic, saying `Form` declares no state

### Requirement: A bare `Update` or `Property` inside a component points to the `State` spelling
`Update` and `Property` SHALL have no special meaning as bare names anywhere: they SHALL resolve
like any other name. When a bare `Update` tag or a bare `Property` reference does not resolve to a
declaration, the diagnostic SHALL name it as unknown. Inside a component body that declares state,
the diagnostic SHALL also suggest `State.Update` or `State.Property`. Outside every component body,
it SHALL show the qualified `<Type>.Update` or `<Type>.Property` form.

#### Scenario: A bare Update inside a component suggests State.Update
- **WHEN** a file contains `component <Counter /> = { state { count:int = 0 } <Button onTapped=<Update count=1 /> /> }` and nothing named `Update` is declared
- **THEN** type checking SHALL reject the tag
- **AND** the diagnostic SHALL suggest `State.Update`

#### Scenario: A bare Property inside a component suggests State.Property
- **WHEN** a file contains `component <Counter /> = { state { count:int = 0 key?:Property } <Label /> }` and nothing named `Property` is declared
- **THEN** type checking SHALL reject the type reference
- **AND** the diagnostic SHALL suggest `State.Property`

#### Scenario: A declaration named Update is an ordinary record inside a component
- **WHEN** a file contains `type Update = { note:string }` and `component <Notes /> = { state { last?:Update } <Panel /> }`
- **THEN** type checking SHALL check `last` against the record `Update`

### Requirement: A state record has a canonical wire form under its own name
The canonical value encoding of a `T.State` value SHALL carry the `$type` discriminator
`<Component>.State` where its type requires one, and the encoding of a `T.State.Update` value
SHALL carry `$type` `<Component>.State.Update` and follow the update-record rules for absence. The
host-owned state snapshot a runtime binding passes in and out SHALL keep its existing shape, a
field map keyed by state field name with no `$type`.

#### Scenario: A state patch is encoded under the state record's name
- **WHEN** a handler of `component <Counter /> = { state { count:int = 0 label?:string } … }` returns `<State.Update label={} />` and the value is encoded as JSON
- **THEN** the JSON SHALL be an object with exactly the keys `$type` and `label`
- **AND** `$type` SHALL be `"Counter.State.Update"` and `label` SHALL be `null`

#### Scenario: The host state snapshot keeps its shape
- **WHEN** a host initializes `Counter` through a runtime binding and reads the returned state
- **THEN** the state SHALL be the object `{ "count": 0 }` with no `$type` key

