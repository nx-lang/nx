## ADDED Requirements

### Requirement: Language service resolves standard library declarations
Every query over a workspace snapshot SHALL see a standard library that a snapshot document imports,
by a written import or through the snapshot's implicit imports, whether or not the snapshot was
constructed with a build context. Diagnostics SHALL match those the compiler reports for the same
documents. Completions SHALL offer a standard library's declarations only in a document that has the
library in scope, under the names its import form gives them, and each completion item SHALL carry
the declaration's documentation. Hover on a standard library declaration, or on a field of one,
SHALL show its signature and documentation under a label that identifies it as a standard library
declaration and names the library. Completion of an import path SHALL NOT be required.

#### Scenario: Hover on an imported standard library type
- **WHEN** a snapshot with no build context holds one document containing `import "@nx/agent"` and `let a = <Agent name="support">Be brief.</Agent>`
- **AND** a client requests hover on the tag name `Agent`
- **THEN** the answer SHALL show the `Agent` record's declaration and documentation
- **AND** SHALL label it as a declaration of the standard library `@nx/agent`

#### Scenario: Completions inside a standard library element
- **WHEN** a client requests completions at a property-name position inside `<Agent name="support" ` in such a document
- **THEN** the answer SHALL offer `description`, `model`, `documents`, `tools`, `limits` and `instructions`, each with its documentation

#### Scenario: A subtype's inherited properties are offered
- **WHEN** a client requests completions at a property-name position inside `<FunctionTool ` in such a document
- **THEN** the answer SHALL offer `function`, `name` and `description`

#### Scenario: Nothing is offered without an import
- **WHEN** a snapshot holds one document with no import and no implicit imports, and a client requests completions at a tag-name position
- **THEN** the answer SHALL NOT offer `Agent`, `Tool` or any other declaration of `@nx/agent`

#### Scenario: An implicit import puts the library in scope
- **WHEN** a snapshot is constructed with implicit imports `["@nx/agent"]` and a document with no import line uses `<Agent name="support">Be brief.</Agent>`
- **THEN** diagnostics SHALL be empty
- **AND** hover on `Agent` SHALL answer as in the first scenario

#### Scenario: An unknown standard library is reported in the editor
- **WHEN** a snapshot document contains `import "@nx/automation"`
- **THEN** diagnostics SHALL carry the unknown-standard-library diagnostic at the import
