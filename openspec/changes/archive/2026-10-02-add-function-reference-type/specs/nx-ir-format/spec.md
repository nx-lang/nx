## ADDED Requirements

### Requirement: NX IR encodes the function reference type
The type table SHALL have a function reference type kind, `anyFunction`, numbered `6`, whose entry
carries one operand, the result type. A prop, field, state field, parameter, result or value typed
`<function ... />: R` in source, written out or reached through an alias, SHALL be typed by that entry in IR: the top type SHALL NOT stand in for it, and a
function type with no parameters SHALL NOT stand in for it. An entry SHALL be written at most once
per result type in a module's type table and referred to by index, like every other type, and an
occurrence over it SHALL be a `seq` entry over it. A module whose type table holds an entry of the
kind SHALL list the required feature `function-reference-type-v1`, so that a runtime that predates
the type refuses the module by name rather than by an unknown kind, and a module without one SHALL
NOT list it. The schema version SHALL remain 5. A reader SHALL validate the entry's layout like any
other kind's: an entry of the kind with no operand, with more than one, or with an operand that is
not a type of the module's table SHALL be malformed. No node kind SHALL change: a function value
bound at such a site SHALL be the `reference` node `function-values` already uses, and a module
that binds one SHALL list `function-values-v1` on the terms that feature already has. The explained form of an artifact
SHALL render the type as `<function ... />: R`, parenthesized under an occurrence.

#### Scenario: A function reference field is typed by the function reference kind
- **WHEN** NX source declares `type AnyFn = <function ... />: object* type Tool = { fn:AnyFn extra?:AnyFn+ }`
- **AND** NX IR is emitted for the program
- **THEN** the field schema for `fn` SHALL be an `anyFunction` type entry whose operand is the `*` occurrence over the `object` primitive
- **AND** the field schema for `extra` SHALL be the `*` occurrence over that same entry
- **AND** the type table SHALL contain that `anyFunction` entry once
- **AND** the explained text SHALL show the types as `<function ... />: object*` and `(<function ... />: object*)*`

#### Scenario: A stated result is the entry's operand
- **WHEN** NX IR is emitted for `type Args = { q:string } type Tool = { build: <function ... />: Args any: <function ... />: object* }`
- **THEN** the type table SHALL hold two `anyFunction` entries, one whose operand is the nominal type `Args` and one whose operand is `object*`
- **AND** the explained text SHALL show `build` as `<function ... />: Args`

#### Scenario: A module that uses the type lists its feature
- **WHEN** NX IR is emitted for `type Tool = { fn: <function ... />: object* } let double(n:int): int = {n * 2} let root() = <Tool fn={double} />`
- **THEN** the required feature list SHALL name `function-reference-type-v1`
- **AND** SHALL name `function-values-v1`, because `double` is referenced as a value
- **AND** the `fn` property in `root` SHALL be a `reference` node naming `double`

#### Scenario: A module that does not use the type lists nothing new
- **WHEN** NX IR is emitted for a program that declares function types with stated parameters and binds functions to them but never writes `...`
- **THEN** the artifact SHALL be byte-for-byte what it was before the type existed

#### Scenario: A reader refuses a malformed entry
- **WHEN** an image's type table holds an entry of kind `6` with no operand, with two operands, or with an operand that indexes past the type table
- **THEN** every reader SHALL refuse the image with a diagnostic rather than reading it

#### Scenario: The conformance corpus covers the function reference type
- **WHEN** the conformance corpus is inspected
- **THEN** it SHALL contain a program that declares a record with a field typed `<function ... />: object*`, an optional one, one under `+`, and a field typed `<function ... />: R` for a record type `R`, binds functions of unlike signatures to them, passes a function reference value through a parameter and a result, compares two such values, and reads such a field's default
- **AND** the recorded results SHALL show the `Function` record for each rendered value
