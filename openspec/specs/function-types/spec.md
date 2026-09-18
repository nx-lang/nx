# function-types Specification

## Purpose

Define the function type reference — an element function's signature with `function` in the name
slot — and how a function is checked against one, so that a property or binding can be declared to
take a function such as a UI template.

## Requirements

### Requirement: A function type is spelled as an element function signature
Anywhere NX accepts a type reference, the parser SHALL accept a function type written as `<`, the
contextual keyword `function`, zero or more parameter definitions, `/>`, `:` and a result type.
Each parameter definition SHALL be a property definition — a name, `:` and a type, with an optional
`content` modifier — matched by name at every use. The result type SHALL be required. A default
value on a parameter, a second `content` parameter, and a `type`-typed parameter SHALL each be
rejected with a diagnostic. `function` SHALL be a keyword only in that position: an identifier
named `function` elsewhere SHALL keep its current meaning.

#### Scenario: A property is declared at a function type
- **WHEN** a file contains `abstract external component <DrawnNode /> external component <DataTable RowTemplate: <function Item:object Index:int />: DrawnNode />`
- **THEN** parsing and lowering SHALL accept `RowTemplate` as a property whose type is a function
  with the parameters `Item:object` and `Index:int` and the result `DrawnNode`

#### Scenario: A function type takes no parameters
- **WHEN** a file contains `external component <DataTable HeaderTemplate: <function />: DrawnNode />`
- **THEN** parsing and lowering SHALL accept `HeaderTemplate` as a function type with no parameters

#### Scenario: A function type is aliased
- **WHEN** a file contains `type Contact = { name:string } type RowTemplate = <function Item:Contact Index:int />: DrawnNode`
- **THEN** parsing and lowering SHALL accept `RowTemplate` as an alias of that function type
- **AND** a property declared `Row:RowTemplate` SHALL have the same type as one declared with the
  function type written inline

#### Scenario: A suffix after the result binds to the result
- **WHEN** a file contains `type Maybe = <function Count:int />: string?`
- **THEN** analysis SHALL treat `Maybe` as a non-nullable function whose result is nullable `string`

#### Scenario: A parameter default is rejected
- **WHEN** a file contains `type T = <function Index:int = 0 />: string`
- **THEN** parsing SHALL produce a validation error on the default
- **AND** the diagnostic SHALL say a function type cannot carry a default value

#### Scenario: A type parameter in a function type is rejected
- **WHEN** a file contains `type T = <function TItem:type />: string`
- **THEN** parsing SHALL reject the parameter on the same terms as a `type` parameter in a
  function parameter list

#### Scenario: A content parameter is accepted once
- **WHEN** a file contains `type Wrap = <function content Children:object[] />: string`
- **THEN** parsing and lowering SHALL accept `Children` as the function type's content parameter
- **AND** a second `content` parameter in the same function type SHALL be rejected

#### Scenario: The word function is still an identifier elsewhere
- **WHEN** a file contains `let function = 1 let v = {function}`
- **THEN** parsing and analysis SHALL accept both declarations

### Requirement: A function type is displayed in NX spelling
Wherever the system shows a function type to an author — a diagnostic, a hover, an explained
artifact — it SHALL spell it as a function type is written in source, `<function Name:Type ... />:
Result`, and SHALL parenthesize it under a `?` or `[]` suffix. It SHALL NOT use an arrow form
such as `(int) => string`.

#### Scenario: A mismatch diagnostic shows the function type
- **WHEN** analysis reports a value bound to a property typed `<function Item:Contact />: DrawnNode`
  as a mismatch
- **THEN** the diagnostic SHALL show the expected type as `<function Item:Contact />: DrawnNode`

#### Scenario: A nullable function type is parenthesized
- **WHEN** the system displays the type of a property declared `(<function />: DrawnNode)?`
- **THEN** it SHALL show `(<function />: DrawnNode)?`

### Requirement: A function satisfies a function type by parameter name
A function SHALL be compatible with a function type when: every parameter the function declares
is declared by the type under the same name, and the type's parameter type is compatible with the
function's parameter type; a content parameter of the function is the content parameter of the
type; and the function's result type is compatible with the type's result type. A function MAY
declare fewer parameters than the type — a caller supplies every parameter of the type and the
function ignores those it does not declare. A function that declares a parameter the type does not
SHALL NOT be compatible. Parameter order SHALL NOT affect compatibility. The same rule SHALL relate
two function types.

#### Scenario: An exact match is compatible
- **WHEN** a file contains `type Contact = { name:string } let <Row Item:Contact Index:int />: string = {Item.name} let r: <function Item:Contact Index:int />: string = {Row}`
- **THEN** analysis SHALL accept `r`

#### Scenario: A function that ignores a parameter is compatible
- **WHEN** a file contains `type Contact = { name:string } let <Compact Item:Contact />: string = {Item.name} let r: <function Item:Contact Index:int />: string = {Compact}`
- **THEN** analysis SHALL accept `r`, because every parameter `Compact` declares is one the type
  supplies

#### Scenario: A function that needs a parameter the type lacks is rejected
- **WHEN** a file contains `type Contact = { name:string } let <Row Item:Contact Index:int />: string = {Item.name} let r: <function Item:Contact />: string = {Row}`
- **THEN** analysis SHALL reject `r`
- **AND** the diagnostic SHALL name `Index` as a parameter the type does not supply

#### Scenario: A parameter type is checked contravariantly
- **WHEN** a file contains `let <Show Value:object />: string = "x" let r: <function Value:int />: string = {Show}` and `let <Count Value:int />: string = "x" let bad: <function Value:object />: string = {Count}`
- **THEN** analysis SHALL accept `r`, because `int` is compatible with `object`
- **AND** SHALL reject `bad`, because `object` is not compatible with `int`

#### Scenario: A result type is checked covariantly
- **WHEN** a file contains `abstract external component <DrawnNode /> external component <SkiaLabel extends DrawnNode Text:string? /> let <Row Item:object />: SkiaLabel = <SkiaLabel /> let r: <function Item:object />: DrawnNode = {Row}`
- **THEN** analysis SHALL accept `r`

#### Scenario: A parameter name mismatch is rejected
- **WHEN** a file contains `let <Row Item:object />: string = "x" let r: <function Entry:object />: string = {Row}`
- **THEN** analysis SHALL reject `r`
- **AND** the diagnostic SHALL name `Item` as a parameter the type does not supply

#### Scenario: Parameter order does not matter
- **WHEN** a file contains `let <Row Index:int Item:object />: string = "x" let r: <function Item:object Index:int />: string = {Row}`
- **THEN** analysis SHALL accept `r`
