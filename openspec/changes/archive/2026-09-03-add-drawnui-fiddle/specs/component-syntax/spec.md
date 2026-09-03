## ADDED Requirements

### Requirement: A component body is type checked like a function body
The type checker SHALL infer and check a component's body, its prop defaults, and its state
defaults. The component's props and state SHALL be bound by name while its body is checked, the way
a function's parameters are bound while its body is checked, and SHALL NOT be visible outside the
component that declared them.

The props bound SHALL be the component's effective props, so a prop inherited from a base component
SHALL be bound at its declared type the way a directly declared one is.

A binding site inside a component body SHALL be checked against the same declared type it would be
checked against in a function body, and a contextual literal there SHALL resolve at that site. A
prop or state default SHALL be checked against the declared type of the field it defaults.

A default SHALL see the fields materialized before it and no others — the effective props in
declaration order, then the state — because that is the order both runtimes build them in. A default
naming a field materialized after it, or naming itself, SHALL be reported as an undefined identifier
rather than resolved. Code generation SHALL NOT emit a name that reaches neither a binding nor a
declaration.

#### Scenario: A contextual literal in a component body resolves like one in a function body
- **WHEN** a file contains `type Hue = Red | Green external component <Paint colour:Hue? /> abstract external component <Node /> component <A extends Node /> = { <Paint colour=Red /> }`
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
- **WHEN** a file contains `type Hue = Red | Green external component <Paint colour:Hue? /> abstract external component <Node /> component <A extends Node /> = { state { tint:Hue = Red } <Paint colour={tint} /> }`
- **THEN** type checking SHALL accept the default as the case `Hue.Red`

#### Scenario: A property type mismatch inside a component body is reported
- **WHEN** a file contains `type Alpha = Red | Green type Beta = Red | Blue external component <Paint colour:Alpha? /> abstract external component <Node /> component <Wrapper extends Node /> = { <Paint colour={Beta.Red} /> }`
- **THEN** type checking SHALL reject `colour` because `Beta.Red` is not a case of `Alpha`
- **AND** it SHALL report the same diagnostic it reports for the identical element at the top level

#### Scenario: A prop default that does not match its declared type is reported
- **WHEN** a file contains `type Hue = Red | Green abstract external component <Node /> component <Bad extends Node hue:Hue = "Green" /> = { <Node /> }`
- **THEN** type checking SHALL reject the default because a quoted string is never a case of `Hue`

#### Scenario: A component's props are not visible outside it
- **WHEN** a file contains `type Hue = Red | Green external component <Paint colour:Hue? /> abstract external component <Node /> component <A extends Node hue:Hue = Red /> = { <Paint colour={hue} /> } let root() = { <Paint colour={hue} /> }`
- **THEN** analysis SHALL accept `hue` inside `A`
- **AND** it SHALL reject `hue` in `root` as an undefined identifier

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
