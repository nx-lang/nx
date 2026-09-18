## ADDED Requirements

### Requirement: TypeScript runtime renders function values and calls them
The TypeScript runtime SHALL read the function type kind, the named-call node and the
`function-values-v1` feature. A `reference` node naming a function SHALL evaluate to a function
value wherever it appears, a named-call node SHALL invoke the function its callee evaluates to
with the arguments bound by name under the subset rule, and canonical output SHALL render a
function value as the `Function` record `function-values` defines. A
`Function` record in host-supplied props — to descriptor construction or component
initialization — SHALL be accepted at a function-typed prop when it names a function declaration
of the linked program, and SHALL be refused with a diagnostic otherwise. The runtime SHALL export
`callFunction(program, value, args)`, which takes a `Function` record and an object of arguments
keyed by parameter name, binds each argument to the parameter of that name, drops an argument the
function does not declare, fails with a diagnostic naming the first parameter that is missing,
and returns the canonical result. A function value SHALL be equal to another only when both name
the same declaration.

#### Scenario: A rendered descriptor carries the Function record
- **WHEN** a linked program declares `let <Row Item:object Index:int />: string = "r"` in `main.nx` and `external component <List ItemTemplate:(<function Item:object Index:int />: string)? /> let root() = <List ItemTemplate={Row} />`
- **AND** `evaluateFunction(program, "root")` is called
- **THEN** the result's `ItemTemplate` SHALL be `{ "$type": "Function", "module": "main.nx", "name": "Row" }`

#### Scenario: callFunction invokes by name and drops extras
- **WHEN** the host passes that record to `callFunction(program, record, { Item: item, Index: 3, Extra: 1 })`
- **THEN** the runtime SHALL invoke `Row` with `Item` and `Index` bound and `Extra` dropped
- **AND** SHALL return the canonical result `"r"`

#### Scenario: callFunction reports a missing parameter
- **WHEN** the host passes that record to `callFunction(program, record, { Item: item })`
- **THEN** the call SHALL fail with a diagnostic naming `Index`

#### Scenario: A Function record reaches a child instance through its props
- **WHEN** a parent's rendered output carries a `Function` record on a function-typed prop of an
  authored child component
- **AND** the host initializes the child from those fields
- **THEN** initialization SHALL accept the record
- **AND** the child's body invoking that prop SHALL call the named function

#### Scenario: A Function record naming no declaration is refused
- **WHEN** the host supplies `{ "$type": "Function", "module": "main.nx", "name": "Nope" }` at a
  function-typed prop
- **THEN** the runtime SHALL fail with a diagnostic naming `Nope`

#### Scenario: A module needing function values is refused by an older runtime
- **WHEN** a runtime that does not implement `function-values-v1` prepares a module that lists it
- **THEN** preparation SHALL fail with a diagnostic naming the feature
