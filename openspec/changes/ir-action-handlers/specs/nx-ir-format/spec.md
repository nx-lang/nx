## MODIFIED Requirements

### Requirement: NX IR encodes the supported eager expression set
NX IR SHALL encode the supported non-reactive expression forms needed for eager evaluation,
including literals, local slot references, top-level references, unary and binary operations,
function calls, intrinsic calls, `if`, match-style `if is` forms, `let`, blocks, arrays, loops,
index access, member access, record literals, union cases, intrinsic elements, component
descriptors, and action handlers. There SHALL be one union-case construct covering both constant
and payload cases rather than separate constructs for enum members and union cases, and that
construct SHALL mark a constant case as constant in expression position as well as in the
declaration, so a runtime produces the bare case name without consulting the declaration. A call to
one of the update intrinsics (`apply`, `merge`, `diff`, `changed`) SHALL be encoded as an intrinsic
call construct that names the intrinsic and carries its argument expressions, distinct from a call
to a declared function, and a program that contains one SHALL list a required feature naming
update-intrinsic support. A `changed` call SHALL also carry the declared field order of the target
record, so a runtime orders the result without consulting the declaration.

An action-handler binding (`onTapped=<Update count={count + 1} />`) SHALL be encoded as an
action-handler construct that carries the component and emit the handler answers, a reference to
the action record it accepts, the slot the handler's `action` binding occupies, the owner component
whose state the handler may patch (absent for a handler bound outside any component body), and the
body expression. The construct SHALL NOT carry an evaluated environment; captured locals are the
slots the body references. A program that contains one SHALL list a required feature naming
action-handler support, and a program that contains none SHALL emit exactly the IR it emitted
before the construct existed. Unsupported executable constructs SHALL be reported as IR build
diagnostics.

#### Scenario: Match expressions are preserved
- **WHEN** NX source contains a match-style `if value is { ... }` expression accepted by static
  analysis
- **AND** NX IR is emitted for the program
- **THEN** the IR SHALL contain an operation that preserves the scrutinee, ordered arms, patterns,
  and optional else branch

#### Scenario: Loop expressions are preserved
- **WHEN** NX source contains `for item, index in items { item }`
- **AND** NX IR is emitted for the program
- **THEN** the IR SHALL preserve the iterable expression, item slot, optional index slot, and loop
  body expression

#### Scenario: Index expressions preserve bounds-sensitive semantics
- **WHEN** NX IR contains an index expression over an array value
- **THEN** supported runtimes SHALL require an integer index
- **AND** an index outside the array bounds SHALL fail with a runtime diagnostic rather than
  evaluating to `null`

#### Scenario: Constant and payload cases use one IR construct
- **WHEN** NX source contains `type Shape = circle | square { n:int }` and constructs both cases
- **AND** NX IR is emitted for the program
- **THEN** both constructions SHALL be encoded by the same union-case construct
- **AND** the construct SHALL carry enough information for a runtime to produce the bare string for
  `circle` and the `$type` map for `square`

#### Scenario: A constant case expression is marked constant
- **WHEN** NX source contains `type Mode = light | dark let m = {Mode.dark}`
- **AND** NX IR is emitted for the program
- **THEN** the union-case expression for `Mode.dark` SHALL be marked as a constant case
- **AND** a runtime evaluating it SHALL produce the bare string `"dark"` rather than a `$type` map

#### Scenario: An intrinsic call is encoded distinctly from a function call
- **WHEN** NX source contains `type User = { name:string } let v = {apply(<User name="Ada" />, <User.Update name="Bo" />)}`
- **AND** NX IR is emitted for the program
- **THEN** the call SHALL be encoded with the intrinsic call construct naming `apply` and carrying two argument expressions
- **AND** the required feature list SHALL name update-intrinsic support
- **AND** for `let c = {changed(<User.Update name="Bo" />)}` the construct SHALL name `changed` and carry the field order `name`

#### Scenario: A handler bound inside a component body is encoded with its owner
- **WHEN** NX source declares `external component <Button emits { Tapped { } } />` and
  `component <Counter /> = { state { count:int = 0 } <Button onTapped=<Update count={count + 1} /> /> }`
- **AND** NX IR is emitted for the program
- **THEN** the `onTapped` property of the `Button` descriptor SHALL be encoded with the
  action-handler construct
- **AND** the construct SHALL name component `Button`, emit `Tapped`, and the `Button.Tapped` action
  record by reference
- **AND** the construct SHALL name `Counter` as the owner
- **AND** the body SHALL be the `Counter.Update` construction, referencing `count` through the
  slot of `Counter`'s `count` state field
- **AND** the required feature list SHALL name action-handler support

#### Scenario: A handler bound outside a component body has no owner
- **WHEN** NX source contains `let root() = { <SearchBox onSearchSubmitted=<DoSearch search={action.searchString} /> /> }`
- **AND** NX IR is emitted for the program
- **THEN** the construct SHALL carry no owner
- **AND** the body SHALL reference `action` through the slot the construct declares for it

#### Scenario: A program without handlers is unchanged
- **WHEN** NX source binds no action handler
- **AND** NX IR is emitted for the program
- **THEN** the required feature list SHALL NOT name action-handler support
- **AND** the emitted IR SHALL be identical to the IR emitted before the action-handler construct
  existed

#### Scenario: Unsupported action handler is rejected in v1
- **WHEN** NX source binds an action handler
- **AND** a caller requests executable TypeScript or JavaScript output, the one target that still
  cannot represent a handler
- **THEN** codegen SHALL fail with a diagnostic identifying the unsupported construct
- **AND** it SHALL NOT silently drop the handler
- **AND** the same source SHALL emit NX IR successfully, carrying the handler as the action-handler
  construct

### Requirement: NX IR encodes component contracts, descriptors, and state metadata
NX IR SHALL preserve effective component prop contracts, declared state fields, defaults, content
field metadata, abstract/external/concrete component flags, the effective emits of the component
(each emit's name and a reference to the action record it carries, inherited emits included),
component body expressions where available, and schemas needed to normalize props and state at
runtime. Component descriptor expressions SHALL remain atomic and SHALL encode normalized
descriptor construction rather than deep-rendering the referenced component body. A descriptor's
handler properties SHALL be encoded as properties of the descriptor, alongside its declared props,
so a runtime can carry the parent's bindings with the instance.

#### Scenario: Stateful component emits state metadata
- **WHEN** NX source declares `component <SearchBox placeholder:string = "Find docs" /> = { state { query:string = placeholder } <TextInput value={query} /> }`
- **AND** NX IR is emitted for the program
- **THEN** the component declaration in IR SHALL include a prop schema for `placeholder`
- **AND** it SHALL include a state schema for `query`
- **AND** it SHALL preserve the default expression used to materialize the initial `query` value

#### Scenario: Component descriptor remains atomic
- **WHEN** NX source declares `component <Parent /> = { <Child /> }`
- **AND** NX IR is emitted for `Parent`
- **THEN** the expression for `<Child />` SHALL be represented as descriptor construction
- **AND** it SHALL NOT inline or pre-render `Child`'s body into `Parent`

#### Scenario: Component declarations carry their emits
- **WHEN** NX source declares `action Reset = { }` and
  `component <Counter emits { Reset ValueChanged { value:int } } /> = { <Label /> }`
- **AND** NX IR is emitted for the program
- **THEN** the `Counter` declaration SHALL list emits `Reset` and `ValueChanged` in declaration
  order
- **AND** `Reset` SHALL reference the `Reset` action record and `ValueChanged` SHALL reference the
  inline `Counter.ValueChanged` record
- **AND** a component that declares no emits SHALL list none

#### Scenario: A descriptor keeps its handler property beside its props
- **WHEN** NX source constructs `<Counter step=2 onReset=<Log /> />` for a component that declares
  prop `step` and emits `Reset`
- **AND** NX IR is emitted for the program
- **THEN** the descriptor SHALL carry both `step` and `onReset` as properties
- **AND** `onReset`'s value SHALL be the action-handler construct
