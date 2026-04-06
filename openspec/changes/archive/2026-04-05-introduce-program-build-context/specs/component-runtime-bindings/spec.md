## ADDED Requirements

### Requirement: Native component lifecycle bindings are artifact-first
The native C ABI SHALL expose component initialization and dispatch only for previously built
`ProgramArtifact` handles. Hosts that need imported-library resolution SHALL first build a
transient `ProgramArtifact` against a caller-supplied `ProgramBuildContext` backed by a
`LibraryRegistry`, then execute component lifecycle calls against that artifact.

#### Scenario: Native component initialization uses a preloaded library registry through a program artifact
- **WHEN** a host loads `../question-flow` into a `LibraryRegistry`
- **AND** creates a `ProgramBuildContext` that exposes `../question-flow`
- **AND** builds a `ProgramArtifact` from source file `app/main.nx` that imports `../question-flow`
- **AND** initializes a component through the native program-artifact component API
- **THEN** initialization SHALL succeed without reloading `../question-flow` during that call

#### Scenario: Native component dispatch reuses the same program artifact
- **WHEN** a host loads `../question-flow` into a `LibraryRegistry`
- **AND** creates a `ProgramBuildContext` that exposes `../question-flow`
- **AND** builds a `ProgramArtifact` from component source that imports `../question-flow`
- **AND** dispatches actions through the native program-artifact dispatch API
- **THEN** dispatch SHALL reuse the already built artifact and its selected loaded library snapshots
