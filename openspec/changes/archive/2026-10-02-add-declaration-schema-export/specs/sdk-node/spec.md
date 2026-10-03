## ADDED Requirements

### Requirement: Node program artifacts export function and type schemas with wasm parity
A program artifact built by the Node SDK SHALL answer the function schema and type schema queries
`declaration-schema-export` defines, with the same method names, options, answer shapes and error
behavior as the wasm SDK. For the same sources, libraries, entry, declaration and options, the Node
SDK and the wasm SDK SHALL return equal answers: schema documents that serialize to identical JSON
text, equal documentation, equal parameter entries and equal diagnostics. The queries SHALL answer
for artifacts built from a single source, from an in-memory workspace, and against libraries loaded
from memory or from a directory.

#### Scenario: Node and wasm agree on a function's schema
- **WHEN** the same workspace is built by both SDKs and each is asked for the schema of the same
  function with the same host-supplied types
- **THEN** the two input schemas SHALL serialize to identical JSON text, and so SHALL the two output
  schemas
- **AND** the descriptions, parameter entries and diagnostics SHALL be equal

#### Scenario: Node and wasm agree on a type with no JSON form
- **WHEN** both SDKs are asked for the schema of a function with a function-typed parameter
- **THEN** both SHALL return an answer with no input schema and equal `schema-inexpressible-type`
  diagnostics

#### Scenario: A directory-loaded library function is described
- **WHEN** a Node program artifact is built against a library loaded from a directory
- **AND** the caller asks for the schema of a function that library declares, by the library
  module's identity in the program
- **THEN** the query SHALL answer with that function's schema

#### Scenario: A disposed artifact is refused
- **WHEN** a caller asks a disposed Node program artifact for a schema
- **THEN** the call SHALL throw the SDK's disposed-resource error
