## MODIFIED Requirements

### Requirement: NX IR encodes component contracts, descriptors, and state metadata
NX IR SHALL preserve effective component prop contracts, declared state fields, defaults, content
field metadata, abstract/external/concrete component flags, the effective emits of the component
(each emit's name and a reference to the action record it carries, inherited emits included, in
declaration order), component body expressions where available, schemas needed to normalize
props and state at runtime, and, for a component whose program references its derived state
record or that record's update record or property union, a reference to the `<Component>.State`
record declaration. Component descriptor expressions SHALL remain atomic and SHALL encode
normalized descriptor construction rather than deep-rendering the referenced component body. A
descriptor's handler properties SHALL be encoded as properties of the descriptor, alongside its
declared props, so a runtime can carry the parent's bindings with the instance.

#### Scenario: Stateful component emits state metadata
- **WHEN** NX source declares `component <SearchBox placeholder:string = "Find docs" /> = { state { query:string = placeholder } <TextInput value={query} /> }`
- **AND** NX IR is emitted for the program
- **THEN** the component declaration in IR SHALL include a prop schema for `placeholder`
- **AND** it SHALL include a state schema for `query`
- **AND** it SHALL preserve the default expression used to materialize the initial `query` value

#### Scenario: A component names its state record when the program uses it
- **WHEN** NX source declares `component <Counter /> = { state { count:int = 0 } <Button onTapped=<State.Update count={count + 1} /> /> }`
- **AND** NX IR is emitted for the program
- **THEN** the `Counter` declaration SHALL reference the `Counter.State` record declaration
- **AND** the IR SHALL declare `Counter.State` with the field `count:int` and no default
- **AND** a component whose program never names its state record, its update or its property union SHALL carry no such reference

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

#### Scenario: An inherited emit references its declaring module
- **WHEN** a component extends a base declared in another module whose inline emit is `Tapped`
- **AND** NX IR is emitted for the extending component's module
- **THEN** the extending component's `emits` SHALL list `Tapped` referencing the base module's
  `<Base>.Tapped` record

#### Scenario: A descriptor keeps its handler property beside its props
- **WHEN** NX source constructs `<Counter step=2 onReset=<Log /> />` for a component that declares
  prop `step` and emits `Reset`
- **AND** NX IR is emitted for the program
- **THEN** the descriptor SHALL carry both `step` and `onReset` as properties
- **AND** `onReset`'s value SHALL be the action-handler node

### Requirement: NX IR declares the update records a program references
Emitted NX IR SHALL contain a declaration for every derived update record `T.Update` the program
references, whether by constructing one, by naming it in a type annotation, or by declaring a
state or prop field of that type. The declaration SHALL identify itself as an update record, SHALL name its
target `T`, and SHALL carry its own field schemas so that a runtime can normalize a value of the
type without consulting the target: every field optional in the sense that it may be absent, no
default expressions, and the clearable mark copied from the target field's optional mark, so the
runtime accepts a present empty value only where `T` declares the field `?:`. When `T` is a
component's derived state record, the IR SHALL declare `T` as well, and the component's
declaration SHALL reference it, so a runtime recognizes a state patch by its target alone. Update
record declarations that the program does not reference SHALL NOT be emitted, so that programs which never
use a patch produce the same IR as before. A program that references an update record SHALL list a
required feature naming update-record support, so a runtime that predates this change rejects the
program rather than misreading the declaration.

#### Scenario: A referenced update record is declared with its own schema
- **WHEN** NX source contains `type User = { name:string = "anon" email?:string } let patch = <User.Update email={} />`
- **AND** NX IR is emitted for the program
- **THEN** the IR SHALL contain a declaration for `User.Update` marked as an update record targeting `User`
- **AND** that declaration SHALL list fields `name` and `email`, both able to be absent, with `email` clearable, `name` not clearable, and neither carrying a default

#### Scenario: Unreferenced update records are not emitted
- **WHEN** NX source contains `type User = { name:string } let user = <User name="Ada" />`
- **AND** NX IR is emitted for the program
- **THEN** the IR SHALL NOT contain a declaration for `User.Update`
- **AND** the required feature list SHALL be unchanged from a program without update records

#### Scenario: Construction of an update record is encoded as record construction
- **WHEN** NX source constructs `<Counter.State.Update count=1 />` for a component `Counter` with state `count:int`
- **AND** NX IR is emitted for the program
- **THEN** the expression SHALL be encoded with the existing record construction operation targeting the `Counter.State.Update` declaration
- **AND** the emitted canonical value SHALL carry `$type` `Counter.State.Update` and only the `count` key

### Requirement: NX IR encodes the supported eager expression set
NX IR SHALL encode the supported non-reactive expression forms needed for eager evaluation,
including literals, local slot references, top-level references, unary and binary operations,
function calls, intrinsic calls, `if`, match-style `if is` forms, `let`, blocks, braced sequences,
loops, index access, member access, the presence operators and the `{}` pattern, record literals,
union cases, intrinsic elements, component descriptors, and action handlers. There SHALL be one
union-case construct covering both constant and payload cases rather than separate constructs for
enum members and union cases, and that construct SHALL mark a constant case as constant in
expression position as well as in the declaration, so a runtime produces the bare case name without
consulting the declaration. A call to one of the update intrinsics (`apply`, `merge`, `diff`,
`changed`) SHALL be encoded as an intrinsic call construct that names the intrinsic and carries its
argument expressions, distinct from a call to a declared function, and a program that contains one
SHALL list a required feature naming update-intrinsic support. A `changed` call SHALL also carry
the declared field order of the target record, so a runtime orders the result without consulting
the declaration.

An action-handler binding (`onTapped=<State.Update count={count + 1} />`) SHALL be encoded as an
action-handler node kind that carries a reference to the component and the name of the emit the
handler answers, a reference to the action record it accepts, the slot the handler's `action`
binding occupies, a reference to the owner component whose state the handler may patch (absent for
a handler bound outside any component body), and the body node. The node SHALL NOT carry an
evaluated environment; captured locals are the slots the body references. The public name of the
action a runtime reports for the handler SHALL be the action reference's declaration name. A
module that contains the node SHALL list a required feature naming action-handler support, and a
module that contains none SHALL NOT list it. Unsupported executable constructs SHALL be reported
as IR build diagnostics.

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
- **WHEN** NX IR contains an index expression over a sequence value
- **THEN** supported runtimes SHALL require an integer index
- **AND** an index outside the sequence bounds SHALL fail with a runtime diagnostic rather than
  evaluating to the empty value

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
  `component <Counter /> = { state { count:int = 0 } <Button onTapped=<State.Update count={count + 1} /> /> }`
- **AND** NX IR is emitted for the program
- **THEN** the `onTapped` property of the `Button` descriptor SHALL be encoded with the
  action-handler node kind
- **AND** the node SHALL reference component `Button`, name emit `Tapped`, and reference the
  `Button.Tapped` action record
- **AND** the node SHALL reference `Counter` as the owner
- **AND** the body SHALL be the `Counter.State.Update` construction, referencing `count` through the
  slot of `Counter`'s `count` state field
- **AND** the required feature list SHALL name action-handler support

#### Scenario: A handler bound outside a component body has no owner
- **WHEN** NX source contains `let root() = { <SearchBox onSearchSubmitted=<DoSearch search={action.searchString} /> /> }`
- **AND** NX IR is emitted for the program
- **THEN** the node SHALL carry no owner
- **AND** the body SHALL reference `action` through the slot the node declares for it
- **AND** that slot SHALL be the next slot of the enclosing frame, as a `let` binding's would be

#### Scenario: A program without handlers lists no handler feature
- **WHEN** NX source binds no action handler
- **AND** NX IR is emitted for the program
- **THEN** the required feature list SHALL NOT name action-handler support

#### Scenario: Unsupported action handler is rejected in v1
- **WHEN** NX source binds an action handler
- **AND** NX IR is emitted for the program
- **THEN** emission SHALL succeed, carrying the handler as the action-handler node
- **AND** the same source SHALL still fail executable TypeScript or JavaScript generation as
  `executable-code-generation` requires

### Requirement: NX IR erases component type parameters
NX IR SHALL NOT carry component type parameters. A component's prop schema and state schema in IR
SHALL describe each field with the type parameter replaced by the top type `object`, keeping the
field's occurrence around the erased type, the component's derived state record and that record's
update record SHALL describe each field the same way, and a component descriptor in IR SHALL NOT include a property for a type
argument the source bound. IR emitted for a program with generic components SHALL remain
deterministic and boundary-clean.

#### Scenario: Prop schema carries the erased type
- **WHEN** NX source declares `external component <SkiaLayout TItem:type itemsSource?:TItem+ />`
- **AND** NX IR is emitted for the program
- **THEN** the component declaration in IR SHALL include a prop schema for `itemsSource` typed `object*`: the `seq` type kind over `object` with the `*` occurrence
- **AND** it SHALL NOT include a prop schema or any other entry for `TItem`

#### Scenario: Descriptor omits the type argument
- **WHEN** NX source declares `type Contact = { name:string } external component <SkiaLayout TItem:type itemsSource?:TItem+ /> let v = <SkiaLayout TItem=Contact itemsSource={} />`
- **AND** NX IR is emitted for the program
- **THEN** the descriptor expression for `v` SHALL carry a property for `itemsSource` and no property for `TItem`

#### Scenario: Update record carries the erased type
- **WHEN** NX source declares `component <List TItem:type items?:TItem+ /> = { state { sel?:TItem } <Label /> } let u = <List.State.Update sel={} />`
- **AND** NX IR is emitted for the program
- **THEN** the `List.State.Update` declaration in IR SHALL type `sel` as `object?`, mark it clearable, and SHALL NOT mention `TItem`
- **AND** the `List.State` declaration it targets SHALL type `sel` the same way
