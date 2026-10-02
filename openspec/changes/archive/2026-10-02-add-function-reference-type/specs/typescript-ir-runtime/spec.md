## ADDED Requirements

### Requirement: TypeScript runtime accepts any function at a function reference site
The TypeScript runtime SHALL read the function reference type kind, validating its result operand,
and the `function-reference-type-v1` feature. Where it normalizes a value against a function
reference type — a host-supplied argument, prop, state value, patch or batch entry, a record or
component field, a parameter and a result — it SHALL accept a function value of the linked
program, and SHALL accept a host-supplied `Function` record,
`{ "$type": "Function", "module", "name" }`, when it names a function declaration of the linked
program, whatever that function's parameters. It SHALL NOT compare the function's result with the
type's result operand, as it compares no part of a signature at a site typed by a function type
with stated parameters. It SHALL refuse a `Function` record that names a module the program does
not link, or a name that module does not declare as a function, with the `nx-ir-function-value`
diagnostic naming the function, and SHALL refuse any other value — a string, a number, a boolean,
a record of another `$type`, a `Function` record without a string `module` and `name` — with the
`nx-ir-boundary-type` diagnostic naming the site. Occurrences over the type SHALL follow the rules
the runtime applies to an occurrence over any type. Canonical output SHALL render a value at such a
site as the `Function` record `function-values` defines. The runtime SHALL NOT call the function
when it normalizes, stores, compares or renders the value. `callFunction` SHALL be unchanged, so a
record read from such a field is callable with arguments keyed by the function's own parameter
names, each validated against that parameter's declared type. The package SHALL export a type
`NxFunctionRecord` describing the record: `$type` the literal `"Function"`, `module` a string and
`name` a string.

#### Scenario: A rendered function reference field carries the Function record
- **WHEN** a linked program declares, in `main.nx`, `type Tool = { fn: <function ... />: object* } let double(n:int): int = {n * 2} let root() = <Tool fn={double} />`
- **AND** `evaluateFunction(program, "root")` is called
- **THEN** the result SHALL be `{ "$type": "Tool", "fn": { "$type": "Function", "module": "main.nx", "name": "double" } }`

#### Scenario: A field with a stated result renders the same record
- **WHEN** a linked program declares, in `main.nx`, `type Args = { q:string } type Tool = { build: <function ... />: Args } let make(q:string): Args = <Args q={q} /> let root() = <Tool build={make} />`
- **AND** `evaluateFunction(program, "root")` is called
- **THEN** the result's `build` field SHALL be `{ "$type": "Function", "module": "main.nx", "name": "make" }`

#### Scenario: A host supplies a function of any signature
- **WHEN** the first program also declares `let greet(name:string, loud?:boolean): string = {name}` and `let pass(tool:Tool): Tool = {tool}`
- **AND** the host calls `evaluateFunction(program, "pass", [{ "$type": "Tool", "fn": { "$type": "Function", "module": "main.nx", "name": "greet" } }])`
- **THEN** the call SHALL succeed and the result's `fn` SHALL be that record

#### Scenario: A record naming no function is refused
- **WHEN** the host supplies `{ "$type": "Function", "module": "main.nx", "name": "Nope" }` at the `fn` field
- **THEN** the runtime SHALL fail with `nx-ir-function-value` naming `Nope`
- **AND** `{ "$type": "Function", "module": "other.nx", "name": "double" }`, where the program links no `other.nx`, SHALL fail with `nx-ir-function-value` naming `other.nx`

#### Scenario: A non-function at a function reference site is refused
- **WHEN** the host supplies `"double"`, `1`, `{ "$type": "Tool" }` or `{ "$type": "Function", "name": "double" }` at the `fn` field
- **THEN** each SHALL fail with `nx-ir-boundary-type` naming the field

#### Scenario: A value that names something other than a function is refused
- **WHEN** the host supplies `{ "$type": "Function", "module": "main.nx", "name": "Tool" }` at the `fn` field
- **THEN** the runtime SHALL fail with `nx-ir-function-value`, because `Tool` is a record and not a function

#### Scenario: Occurrences over a function reference type follow the ordinary rules
- **WHEN** a program declares `type AnyFn = <function ... />: object* type Kit = { all:AnyFn+ one?:AnyFn }` and the host supplies a `Kit` with `all` set to `[]`, and another with `one` omitted and `all` holding one record
- **THEN** the first SHALL be refused because `all` admits no empty value
- **AND** the second SHALL be accepted, with `one` normalized to the empty value

#### Scenario: A record from a function reference field is callable by its own parameters
- **WHEN** the host reads the `fn` record of the first scenario and calls `callFunction(program, record, { n: 4 })`
- **THEN** the call SHALL return `8`
- **AND** `callFunction(program, record, { n: "four" })` SHALL fail with a diagnostic naming `n`, because the argument does not have the parameter's declared type

#### Scenario: A module needing the type is refused by an older runtime
- **WHEN** a runtime that does not implement `function-reference-type-v1` prepares a module that lists it
- **THEN** preparation SHALL fail with a diagnostic naming the feature

#### Scenario: The corpus program passes
- **WHEN** the TypeScript runtime runs the function reference conformance program
- **THEN** every entrypoint SHALL produce the recorded result
