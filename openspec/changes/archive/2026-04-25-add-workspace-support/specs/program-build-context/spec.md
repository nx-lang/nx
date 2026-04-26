## ADDED Requirements

### Requirement: Workspace program builds reuse supplied build context
Workspace validation and workspace program construction SHALL use the supplied
`ProgramBuildContext` to resolve imports that are not satisfied by submitted workspace modules.
The workspace path SHALL NOT load missing local libraries from disk ad hoc during that request.

#### Scenario: Workspace import to preloaded library succeeds
- **WHEN** a host loads `../question-flow` into a `LibraryRegistry`
- **AND** creates a `ProgramBuildContext` that exposes `../question-flow`
- **AND** a workspace module imports `../question-flow`
- **THEN** workspace validation and workspace program construction SHALL resolve that import
  through the supplied build context

#### Scenario: Workspace build reports missing library from context
- **WHEN** a workspace module imports `../question-flow`
- **AND** the supplied `ProgramBuildContext` does not expose a loaded `../question-flow` snapshot
- **THEN** workspace validation and workspace program construction SHALL report a missing-library
  diagnostic
- **AND** NX SHALL NOT silently load `../question-flow` from disk

#### Scenario: Workspace modules satisfy imports before build context lookup
- **WHEN** a workspace module import resolves to a submitted workspace module identity
- **THEN** NX SHALL use the workspace module for that import
- **AND** NX SHALL NOT require a matching library root to be visible in the supplied build context
