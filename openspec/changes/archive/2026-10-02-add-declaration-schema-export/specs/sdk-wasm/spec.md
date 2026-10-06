## ADDED Requirements

### Requirement: A program artifact exports function and type schemas
A program artifact built by the wasm SDK SHALL answer the two queries `declaration-schema-export`
defines: the schema of a function and the schema of a declared type, each named by a module
identity, which defaults to the entry module, and a name. The function query SHALL accept the list
of host-supplied types, and the type query SHALL accept the direction. Both SHALL return plain JSON
data: the schema documents as JSON Schema objects a caller can store or serialize as they are, the
documentation as strings, and diagnostics in the shape the SDK's build diagnostics take.

A type with no JSON form SHALL be answered as data, with the diagnostic in the answer and the
affected schema absent, never thrown. A reference that names nothing SHALL throw the SDK's
evaluation error carrying the `schema-unknown-declaration` diagnostic. Neither query SHALL evaluate
the program, and the artifact SHALL remain usable after either. The queries SHALL follow the SDK's
disposed-resource and crashed-host rules, and SHALL be answered for functions and types in the
modules of in-memory libraries the artifact was built against.

#### Scenario: A host derives a tool definition while compiling
- **WHEN** a host builds a workspace artifact whose entry module declares a documented function
  `findPlans`, and calls the function query for it
- **THEN** it SHALL receive the function's description, its parameter entries, an input schema and
  an output schema as JSON objects
- **AND** the same artifact SHALL then emit the NX IR image that `@nx-lang/ir-runtime` runs the
  function from

#### Scenario: The schemas outlive the artifact
- **WHEN** a caller has received a function's schemas and disposes the artifact
- **THEN** the schema objects SHALL remain readable and serializable

#### Scenario: A library function is described
- **WHEN** an artifact is built against a build context whose registry loaded `libraries/catalog`
- **AND** the caller asks for the schema of a function in `libraries/catalog/Plans.nx`
- **THEN** the query SHALL answer as it does for a function of the entry module

#### Scenario: A type with no JSON form is data, not an exception
- **WHEN** a caller asks for the schema of a function with a function-typed parameter
- **THEN** the call SHALL return an answer with no input schema and a `schema-inexpressible-type`
  diagnostic
- **AND** it SHALL NOT throw

#### Scenario: An unknown function throws
- **WHEN** a caller asks for the schema of a function the program does not declare
- **THEN** the call SHALL throw an `NxEvaluationError` whose diagnostic has the code
  `schema-unknown-declaration`

#### Scenario: A disposed artifact is refused
- **WHEN** a caller asks a disposed artifact for a schema
- **THEN** the call SHALL throw the SDK's disposed-resource error

#### Scenario: The ABI version moves with the new exports
- **WHEN** the wrapper for this version loads a module built before these exports existed
- **THEN** loading SHALL fail with the version mismatch error, naming both versions

### Requirement: The schema export is released with doc comments
The published `@nx-lang/sdk-wasm` package that first carries the schema export SHALL also carry the
`///` doc comment support the descriptions come from, so that a host installing that version gets
descriptions from documented source with no other package or setting. The package's documentation
SHALL describe the two queries, state that they are the supported way to read a program's types and
documentation, and direct a host that only executes IR to derive schemas when it compiles and to
store them with the image.

#### Scenario: A consumer gets descriptions from the registry package
- **WHEN** a JavaScript project installs the published package at that version and builds source
  whose function and parameters carry `///` comments
- **THEN** the function query SHALL return those comments as descriptions

#### Scenario: The documentation names the supported path
- **WHEN** a consumer reads the package's documentation
- **THEN** it SHALL find the schema queries, their options, and the diagnostics they report
