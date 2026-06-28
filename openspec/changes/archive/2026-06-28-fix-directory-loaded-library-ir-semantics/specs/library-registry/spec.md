## ADDED Requirements

### Requirement: Loaded library snapshots preserve semantic bindings for IR generation

`LibraryRegistry` SHALL retain or expose the semantic binding targets needed for downstream
`ProgramArtifact` IR generation when libraries are loaded from directories. The retained semantics
SHALL cover exported declarations, same-library peer declarations, and declarations imported from
loaded dependency libraries.

#### Scenario: Directory-loaded library retains dependency type binding

- **WHEN** a host loads `../question-flow` into a `LibraryRegistry`
- **AND** a host loads `../chat-link` whose exported declarations reference types from
  `../question-flow`
- **THEN** the `../chat-link` library snapshot SHALL retain semantic binding targets for those
  dependency type references
- **AND** a later program artifact that imports `../chat-link` SHALL have enough semantic data to
  generate IR for those references without re-reading the library directories

#### Scenario: Reused build contexts preserve library binding semantics

- **WHEN** a host loads a library graph into one `LibraryRegistry`
- **AND** the host creates multiple `ProgramBuildContext`s from that registry
- **THEN** each build context SHALL expose equivalent semantic binding targets for the loaded
  library graph
- **AND** IR generation from program artifacts built through those contexts SHALL resolve the same
  module-qualified nominal type references

