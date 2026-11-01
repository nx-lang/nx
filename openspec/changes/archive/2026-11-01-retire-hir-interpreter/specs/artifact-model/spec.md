## MODIFIED Requirements

### Requirement: Program artifacts contain the resolved executable world
The system SHALL represent a fully resolved executable program as a `ProgramArtifact`. A
`ProgramArtifact` SHALL preserve the root module set, resolved `LibraryArtifact` dependencies,
whole-program diagnostics and fingerprint metadata, and an embedded `ResolvedProgram` from which NX IR is emitted.

#### Scenario: Program artifact combines root modules and resolved libraries
- **WHEN** a host builds a program from one root source file that imports `../ui`
- **THEN** the resulting `ProgramArtifact` SHALL preserve the root file as a `ModuleArtifact`
- **AND** the resulting `ProgramArtifact` SHALL preserve the imported library as a `LibraryArtifact`
- **AND** the resulting `ProgramArtifact` SHALL include a `ResolvedProgram` for IR compilation and entry point lookup

#### Scenario: Program artifact preserves whole-program diagnostics
- **WHEN** whole-program resolution finds a duplicate export or import ambiguity across resolved
  libraries
- **THEN** the resulting `ProgramArtifact` SHALL preserve those diagnostics as whole-program
  diagnostics
- **AND** the `ProgramArtifact` fingerprint metadata SHALL correspond to the exact root modules and
  resolved libraries used to build it

### Requirement: Runtime module IDs are assigned only when building a program artifact
The runtime module IDs used for NX compilation SHALL be assigned only when assembling
the `ResolvedProgram` inside one `ProgramArtifact`. Loaded `LibraryArtifact` snapshots and the
`LibraryRegistry` SHALL NOT persist or expose program runtime module IDs.

#### Scenario: One loaded library snapshot can participate in multiple programs
- **WHEN** a host loads `../ui` into a `LibraryRegistry`
- **AND** later builds two different `ProgramArtifact`s that each reuse that same loaded
  `LibraryArtifact` snapshot
- **THEN** each program build SHALL assign its own runtime module IDs when assembling its
  `ResolvedProgram`
- **AND** the loaded `LibraryArtifact` snapshot SHALL remain free of program-scoped runtime IDs
