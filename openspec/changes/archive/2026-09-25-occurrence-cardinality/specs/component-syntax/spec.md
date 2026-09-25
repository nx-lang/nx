## MODIFIED Requirements

### Requirement: Component initialization materializes props, state, and rendered output
The system SHALL lower a component declaration as an executable component definition. Initializing a
component SHALL bind prop values, apply any prop defaults, evaluate state default expressions once
in declaration order, and return the rendered body expression together with the logical initial
component state. A state field that is optional and has no default SHALL initialize to the empty
value, as `optional-properties` defines.

#### Scenario: Initialization applies prop and state defaults once
- **WHEN** a module contains `component <SearchBox placeholder:string = "Find docs" /> = { state { query:string = placeholder preview:string = query } <TextInput value={preview} placeholder={placeholder} /> }` and the component is initialized without an explicit `placeholder`
- **THEN** initialization SHALL bind `placeholder` as `"Find docs"`
- **AND** SHALL materialize initial state `query="Find docs"` and `preview="Find docs"`
- **AND** SHALL return a rendered `TextInput` element whose `value` and `placeholder` are both `"Find docs"`

#### Scenario: Initialization succeeds without a state group
- **WHEN** a module contains `component <Button text:string /> = { <button>{text}</button> }` and the component is initialized with `text="Save"`
- **THEN** initialization SHALL return a rendered `button` element containing `"Save"`
- **AND** SHALL produce an empty logical component state

#### Scenario: Missing required state initializer is rejected
- **WHEN** a module contains `component <SearchBox /> = { state { query:string } <TextInput value={query} /> }` and the component is initialized
- **THEN** initialization SHALL fail because required state field `query` has no initial value

### Requirement: Component state defaults are initialization-only
The system SHALL evaluate component state default expressions only during initialization. During
dispatch, the passed-in prior state snapshot SHALL be the current value of every state field, and
the only way a state field changes SHALL be an update record for that component returned by a
handler and applied by dispatch. Default expressions SHALL NOT be re-evaluated when state changes.

#### Scenario: Dispatch reuses stored state instead of reevaluating defaults
- **WHEN** a module contains `component <SearchBox placeholder:string /> = { state { query:string = placeholder } <TextInput value={query} placeholder={placeholder} /> }` and a later dispatch receives a prior state snapshot whose current `query` value differs from `placeholder`
- **THEN** dispatch SHALL use the stored `query` value from the prior state snapshot as the current component state
- **AND** SHALL NOT reevaluate `query:string = placeholder`

#### Scenario: An update record is the only way to change state
- **WHEN** a module contains `component <SearchBox placeholder:string emits { Cleared } /> = { state { query:string = placeholder } <TextInput value={query} onTextChanged=<Update query={action.text} /> onCleared={ <Update query="" /> <Cleared /> } /> }`
- **THEN** dispatching the `onTextChanged` handler with `text="docs"` SHALL leave the next snapshot's `query` as `"docs"`
- **AND** a field the update record does not name SHALL keep its prior value

### Requirement: A component body is type checked like a function body
The type checker SHALL infer and check a component's body, its prop defaults, and its state
defaults. The component's props and state SHALL be bound by name while its body is checked, the way
a function's parameters are bound while its body is checked, and SHALL NOT be visible outside the
component that declared them.

The props bound SHALL be the component's effective props, so a prop inherited from a base component
SHALL be bound at its declared type the way a directly declared one is. A prop or state field
declared `p?:T` SHALL be bound at its read type `T?`, as `optional-properties` defines.

A binding site inside a component body SHALL be checked against the same declared type it would be
checked against in a function body, and a contextual literal there SHALL resolve at that site. A
prop or state default SHALL be checked against the declared type of the field it defaults.

A default SHALL see the fields materialized before it and no others — the effective props in
declaration order, then the state — because that is the order both runtimes build them in. A default
naming a field materialized after it, or naming itself, SHALL be reported as an undefined identifier
rather than resolved. Code generation SHALL NOT emit a name that reaches neither a binding nor a
declaration.

#### Scenario: A contextual literal in a component body resolves like one in a function body
- **WHEN** a file contains `type Hue = Red | Green external component <Paint colour?:Hue /> abstract external component <Node /> component <A extends Node /> = { <Paint colour=Red /> }`
- **THEN** type checking SHALL resolve `Red` as the case `Hue.Red`
- **AND** code generation SHALL emit the component body without reporting an unresolved contextual name

#### Scenario: A component body and a function body emit the same case
- **WHEN** two files differ only in that one writes `colour=Red` inside a component body where the
  other writes `colour={Hue.Red}` there
- **THEN** code generation SHALL emit the same union case for both

#### Scenario: A prop default accepts a contextual literal
- **WHEN** a file contains `type Hue = Red | Green abstract external component <Node /> component <A extends Node hue:Hue = Red /> = { <Node /> }`
- **THEN** type checking SHALL accept the default as the case `Hue.Red`

#### Scenario: A state default accepts a contextual literal
- **WHEN** a file contains `type Hue = Red | Green external component <Paint colour?:Hue /> abstract external component <Node /> component <A extends Node /> = { state { tint:Hue = Red } <Paint colour={tint} /> }`
- **THEN** type checking SHALL accept the default as the case `Hue.Red`

#### Scenario: A property type mismatch inside a component body is reported
- **WHEN** a file contains `type Alpha = Red | Green type Beta = Red | Blue external component <Paint colour?:Alpha /> abstract external component <Node /> component <Wrapper extends Node /> = { <Paint colour={Beta.Red} /> }`
- **THEN** type checking SHALL reject `colour` because `Beta.Red` is not a case of `Alpha`
- **AND** it SHALL report the same diagnostic it reports for the identical element at the top level

#### Scenario: A prop default that does not match its declared type is reported
- **WHEN** a file contains `type Hue = Red | Green abstract external component <Node /> component <Bad extends Node hue:Hue = "Green" /> = { <Node /> }`
- **THEN** type checking SHALL reject the default because a quoted string is never a case of `Hue`

#### Scenario: A component's props are not visible outside it
- **WHEN** a file contains `type Hue = Red | Green external component <Paint colour?:Hue /> abstract external component <Node /> component <A extends Node hue:Hue = Red /> = { <Paint colour={hue} /> } let root() = { <Paint colour={hue} /> }`
- **THEN** analysis SHALL accept `hue` inside `A`
- **AND** it SHALL reject `hue` in `root` as an undefined identifier

#### Scenario: An optional prop reads as optional inside the body
- **WHEN** a file contains `type Hue = Red | Green external component <Paint colour:Hue /> abstract external component <Node /> component <A extends Node hue?:Hue /> = { <Paint colour={hue} /> }`
- **THEN** type checking SHALL reject `colour`, naming `Hue?` at a `Hue` site
- **AND** writing `colour={hue ?? Red}` there SHALL be accepted

#### Scenario: An inherited prop is checked like a declared one
- **WHEN** a file contains `abstract external component <Node /> abstract external component <Base extends Node n:int /> external component <Txt v:string /> component <A extends Base /> = { <Txt v={n} /> }`
- **THEN** type checking SHALL reject `v` because the inherited `n` is an `int`
- **AND** it SHALL report the same diagnostic it reports when `n` is declared on `A` itself

#### Scenario: A default naming a field declared after it is reported
- **WHEN** a file contains `abstract external component <Node /> external component <Leaf extends Node /> component <A extends Node a:int = {b} b:int = 1 /> = { <Leaf /> }`
- **THEN** analysis SHALL report `b` as an undefined identifier
- **AND** code generation SHALL NOT emit IR for the program

#### Scenario: A default naming a field declared before it is checked against its type
- **WHEN** a file contains `abstract external component <Node /> external component <Leaf extends Node /> component <A extends Node b:int = 1 a:string = {b} /> = { <Leaf /> }`
- **THEN** type checking SHALL reject the default for `A.a` because `b` is an `int`

#### Scenario: A default may name a prop it inherits
- **WHEN** a component's base declares a prop and the component's own default names it
- **THEN** analysis SHALL accept the name, because an inherited prop is materialized before any the
  component declares

### Requirement: Component signatures accept type parameter definitions
The parser SHALL accept, inside a component signature, a property definition whose type is the
keyword `type`, and SHALL produce for it a PROPERTY_DEFINITION whose type node is the `type`
keyword rather than a type reference. Such a definition MUST appear after the optional `extends`
clause and before every other property definition of the signature; parsing or validation SHALL
reject one that follows a regular property definition. A type parameter definition SHALL NOT carry
the optional `?` mark, as `optional-properties` requires. Parsing or validation SHALL reject a `type`
property type in a record definition, an action definition, an emitted action field list, a state
group, and a function parameter list.

#### Scenario: Leading type parameters parse
- **WHEN** a file contains `external component <SkiaLayout extends SkiaControl TItem:type layoutType?:string itemsSource?:TItem+ />`
- **THEN** the parser SHALL produce a COMPONENT_DEFINITION with base `SkiaControl` and three PROPERTY_DEFINITION nodes
- **AND** the first SHALL be named `TItem` with the `type` keyword as its type
- **AND** the remaining two SHALL carry ordinary type references, each marked optional

#### Scenario: Type parameter after a regular property is rejected
- **WHEN** a file contains `component <Bad items:object+ TItem:type /> = { <Label /> }`
- **THEN** parsing or validation SHALL reject `TItem` as a type parameter declared after a property

#### Scenario: Type parameter outside a component signature is rejected
- **WHEN** a file contains `type Box = { T:type value:T }` and `component <C /> = { state { T:type } <Label /> }` and `let f(T:type) = 1`
- **THEN** parsing or validation SHALL reject each `type`-typed definition as unsupported outside a component signature
