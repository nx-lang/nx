## ADDED Requirements

### Requirement: Library registries load and analyze libraries without a program
The system SHALL expose a reusable `LibraryRegistry` that can load and analyze a local NX library
root into a `LibraryArtifact` snapshot even when no `ProgramArtifact` exists yet.

#### Scenario: Server preloads a shared library before any tenant program exists
- **WHEN** a host process starts and loads `../question-flow` into a `LibraryRegistry`
- **THEN** the registry SHALL produce and retain an analyzed `LibraryArtifact` snapshot for
  `../question-flow`
- **AND** that library load SHALL NOT require creating a `ProgramArtifact`

### Requirement: Library registries own the dependency graph between loaded snapshots
`LibraryRegistry` SHALL own the dependency graph between analyzed library snapshots. Loading a
library root SHALL discover and record the exact dependency closure needed by that library without
embedding dependent library artifacts directly inside the loaded `LibraryArtifact`.

#### Scenario: Loading one library records the dependent snapshots it requires
- **WHEN** a host loads `../question-flow` into a `LibraryRegistry`
- **AND** `../question-flow` depends on `../ui`
- **THEN** the registry SHALL record that dependency relationship in its snapshot graph
- **AND** the loaded `LibraryArtifact` for `../question-flow` SHALL remain a snapshot of
  `../question-flow` itself rather than owning the `../ui` artifact directly

#### Scenario: Circular library dependencies fail without retaining partial snapshots
- **WHEN** a host loads `../a` into a `LibraryRegistry`
- **AND** `../a` depends on `../b`
- **AND** `../b` depends on `../a`
- **THEN** the registry SHALL fail that load with a circular dependency error
- **AND** that error SHALL identify the circular dependency chain between those library roots
- **AND** the registry SHALL NOT retain a partial loaded snapshot for either library root

### Requirement: Loaded library snapshots can be reused across build contexts
Library snapshots loaded into one `LibraryRegistry` SHALL be reusable across multiple
`ProgramBuildContext`s created from that registry.

#### Scenario: Two tenant build contexts reuse the same shared library snapshot
- **WHEN** a host loads `../question-flow` into a `LibraryRegistry`
- **AND** the host creates two different `ProgramBuildContext`s from that registry
- **AND** each build context later builds source that imports `../question-flow`
- **THEN** both builds SHALL resolve `../question-flow` through the same loaded library snapshot
