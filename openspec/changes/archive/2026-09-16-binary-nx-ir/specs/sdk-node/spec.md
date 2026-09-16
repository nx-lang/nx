## ADDED Requirements

### Requirement: Node SDK explains an NX IR artifact
The Node SDK SHALL render an NX IR image it is given as the same readable text the CLI's
`ir explain` prints for that image, and SHALL report a malformed image as an SDK error carrying the
diagnostic.

#### Scenario: A Node host reads an artifact it holds
- **WHEN** a Node caller passes an artifact's bytes to the SDK's explain API
- **THEN** it SHALL receive the text the CLI would print for that artifact

## MODIFIED Requirements

### Requirement: Node SDK generates deterministic NX IR JSON and metadata
The Node SDK SHALL expose deterministic NX IR generation from a reusable program artifact and from
source convenience APIs by delegating to the shared Rust IR emission pipeline, returning each
artifact as bytes with its metadata.

#### Scenario: Artifact emits IR JSON and metadata
- **WHEN** a Node caller generates NX IR from a valid program artifact containing `root()`
- **THEN** the result SHALL include the artifact's bytes as a `Buffer`
- **AND** the result SHALL include structured metadata such as runtime ABI, module fingerprint and
  entrypoints exposed by the shared IR generator

#### Scenario: Equivalent workspace inputs produce equivalent IR
- **WHEN** two Node callers build program artifacts from equivalent workspace module identities,
  source payloads, entry identities, and build contexts
- **THEN** the generated NX IR bytes SHALL be identical
- **AND** generated metadata SHALL identify the same module fingerprint

#### Scenario: Invalid artifact does not emit partial IR
- **WHEN** NX analysis prevents creation of a valid program artifact
- **THEN** the Node SDK SHALL surface structured diagnostics from the build failure
- **AND** it SHALL NOT return partial NX IR

### Requirement: Node SDK generates IR for directory-loaded cross-library type graphs
The Node SDK SHALL support `NxProgramArtifact.generateNxIr()` for program artifacts that import
libraries loaded through `NxLibraryRegistry.loadFromDirectory`, including library graphs where one
loaded library references nominal types from another loaded library.

#### Scenario: Directory-loaded libraries validate, evaluate, and emit IR
- **WHEN** a Node caller loads `question-flow` and `chat-link` library directories through
  `NxLibraryRegistry.loadFromDirectory`
- **AND** `chat-link` declarations reference `QuestionFlow`
- **AND** `QuestionFlow` declarations reference `FlowStep` from the loaded library graph
- **AND** the caller builds a program artifact that imports those declarations
- **THEN** SDK validation SHALL return no user-authored diagnostics
- **AND** JSON evaluation SHALL succeed for the supported entrypoint
- **AND** `generateNxIr()` SHALL return deterministic IR bytes and metadata
- **AND** the generated IR SHALL include module-qualified nominal references for `QuestionFlow` and
  `FlowStep`

#### Scenario: Missing library semantic data remains diagnostic
- **WHEN** `generateNxIr()` cannot emit IR because required semantic binding data is genuinely absent
  from the analyzed artifact
- **THEN** the Node SDK SHALL surface a typed NX evaluation error with structured diagnostics
- **AND** it SHALL NOT return partial IR
