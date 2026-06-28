## ADDED Requirements

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
- **AND** `generateNxIr()` SHALL return deterministic IR JSON and metadata
- **AND** the generated IR SHALL include module-qualified nominal references for `QuestionFlow` and
  `FlowStep`

#### Scenario: Missing library semantic data remains diagnostic

- **WHEN** `generateNxIr()` cannot emit IR because required semantic binding data is genuinely absent
  from the analyzed artifact
- **THEN** the Node SDK SHALL surface a typed NX evaluation error with structured diagnostics
- **AND** it SHALL NOT return partial IR JSON

### Requirement: Node SDK exposes lossless IR fingerprint metadata

The Node SDK SHALL expose generated IR `programFingerprint` metadata without JavaScript numeric
precision loss.

#### Scenario: TypeScript metadata uses string fingerprint

- **WHEN** a TypeScript consumer calls `generateNxIr()` or `generateNxIrFromSource()`
- **THEN** `NxIrMetadata.programFingerprint` SHALL be typed as `string`
- **AND** the runtime value SHALL match the decimal string in the generated IR JSON
- **AND** SDK examples and tests SHALL compare the value as a string rather than a JavaScript
  `number`

