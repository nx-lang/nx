## ADDED Requirements

### Requirement: NX IR encodes function types
The type table SHALL have a function type kind. A function type entry SHALL record its result type
and, in declared order, each parameter's name, type and whether it is the content parameter, every
name and type by table index. A function type SHALL be written once in the type table and referred
to by index, like every other type. A prop, field, parameter, return or local typed by a function
type in source SHALL be typed by that entry in IR; the top type SHALL NOT stand in for it. The
explained form of an artifact SHALL render a function type in NX spelling.

#### Scenario: A function-typed prop is typed as a function
- **WHEN** NX source declares `external component <List ItemTemplate:(<function Item:object Index:int />: string)? />`
- **AND** NX IR is emitted for the program
- **THEN** the component declaration's prop schema for `ItemTemplate` SHALL be a nullable type
  whose inner type is a function type with the parameters `Item` of `object` and `Index` of `int`
  and the result `string`
- **AND** the explained text SHALL show it as `(<function Item:object Index:int />: string)?`

#### Scenario: Two identical function types share one entry
- **WHEN** two props are declared at the same function type
- **THEN** the type table SHALL contain that function type once

### Requirement: NX IR erases type parameters inside function types
Where a component's type parameter occurs inside a function type — as a parameter type or the
result type — the erasure that replaces the parameter with `object` SHALL reach it, so no emitted
type mentions the parameter.

#### Scenario: A template prop is erased through the function type
- **WHEN** NX source declares `external component <SkiaLayout TItem:type ItemTemplate:(<function Item:TItem Index:int />: object)? />`
- **AND** NX IR is emitted for the program
- **THEN** the prop schema for `ItemTemplate` SHALL be a nullable function type whose `Item`
  parameter is typed `object`
- **AND** the artifact SHALL NOT mention `TItem`

### Requirement: NX IR carries a function as a value
A `reference` node that names a function declaration SHALL be a value in any expression position,
not only as a call's callee: a property value, a list element, a function result, a field, a
call argument. A module whose node table contains such a reference in a position other than a
callee, or whose type table contains a function type, SHALL list a required feature naming
function-value support, `function-values-v1`, so a runtime that predates function values refuses
the module by name rather than by an unknown kind. A module with neither SHALL NOT list it. The
explained form SHALL render the reference by the function's module-qualified name.

#### Scenario: A function bound to a prop is a reference node
- **WHEN** NX source declares `let <Row Item:object />: string = "r" external component <List ItemTemplate:(<function Item:object />: string)? /> let root() = <List ItemTemplate={Row} />`
- **AND** NX IR is emitted for the program
- **THEN** the `ItemTemplate` property of the descriptor in `root` SHALL be a `reference` node
  naming `Row`
- **AND** the required feature list SHALL name function-value support

#### Scenario: A program without function values lists no feature
- **WHEN** NX IR is emitted for a program that declares functions and calls them but never binds
  one as a value and declares no function type
- **THEN** the required feature list SHALL be unchanged from before this feature existed

### Requirement: NX IR encodes a call of a function-typed value by name
An element invocation whose tag is a function-typed binding SHALL be encoded as a named-call node:
the callee expression, then each argument as a name and a node, sorted by argument name as a
property list is, since binding is by name and a stable order keeps the artifact canonical. A runtime SHALL
evaluate the callee to a function value and bind the arguments to that function's parameters by
name, dropping a name the function does not declare and failing with a diagnostic when a declared
parameter has no argument. A module containing a named-call node SHALL list the
`function-values-v1` feature. The existing positional call node SHALL be unchanged.

#### Scenario: Invoking a function-typed prop emits a named call
- **WHEN** NX source declares `component <Section Row:<function Item:object Index:int />: string /> = { <Row Item="a" Index=1 /> }`
- **AND** NX IR is emitted for the program
- **THEN** the body of `Section` SHALL contain a named-call node whose callee is the `Row` slot and
  whose arguments are `Item` and `Index`
- **AND** the explained text SHALL show the call with its argument names

#### Scenario: The conformance corpus covers function values
- **WHEN** the conformance corpus is inspected
- **THEN** it SHALL contain a program that declares a function type, binds a function to a
  function-typed prop and to a function-typed parameter, invokes the parameter by element and by
  call, and renders a function value in a function result
- **AND** the recorded results SHALL show the `Function` record for the rendered value and the
  results of both invocations
