## ADDED Requirements

### Requirement: Library artifacts remain local snapshots with dependency-analysis interfaces
`LibraryArtifact` SHALL remain a snapshot of one library root only. It SHALL preserve that
library's local `ModuleArtifact`s, export metadata, dependency metadata, diagnostics, fingerprint,
and the interface metadata needed for dependent analysis. A `LibraryArtifact` SHALL NOT embed or
own dependent library artifacts directly.

#### Scenario: Library artifact preserves its own modules while publishing dependency-analysis metadata
- **WHEN** a library root `../question-flow` depends on `../ui`
- **THEN** the resulting `LibraryArtifact` for `../question-flow` SHALL preserve only the modules
  that belong to `../question-flow`
- **AND** it SHALL publish the export and interface metadata needed for dependent analysis
- **AND** it SHALL record dependency metadata for `../ui` without embedding the `../ui` artifact

### Requirement: Program artifacts record the exact library snapshots selected through registry-backed build context
When a `ProgramArtifact` is built using a `ProgramBuildContext`, it SHALL preserve the exact loaded
library snapshots selected from the underlying `LibraryRegistry` rather than rebuilding those
libraries during program construction. The selected closure SHALL be seeded by the direct imports
that resolved successfully through that build context and then expanded through the dependency
metadata of those selected snapshots.

#### Scenario: Multiple program artifacts can reuse one loaded library snapshot
- **WHEN** a host loads `../ui` into one `LibraryRegistry`
- **AND** creates build contexts from that registry
- **AND** the host builds two different `ProgramArtifact`s from source that each import `../ui`
- **THEN** each `ProgramArtifact` SHALL preserve the same loaded `LibraryArtifact` snapshot from
  that registry
- **AND** the system SHALL NOT require `../ui` to be rebuilt separately for each program artifact

#### Scenario: Program artifact preserves the transitive closure of a selected direct import
- **WHEN** a host loads a `../widgets` snapshot that depends on `../ui` into one `LibraryRegistry`
- **AND** creates a `ProgramBuildContext` that exposes `../widgets` as a direct visible root
- **AND** the host builds source that imports `../widgets`
- **THEN** the resulting `ProgramArtifact` SHALL preserve both the selected `../widgets` snapshot
  and the loaded `../ui` snapshot required by its dependency metadata
- **AND** the system SHALL NOT perform a second direct-library lookup that could disagree with the
  original build-context visibility decision

### Requirement: Program artifact fingerprints include the selected library snapshot revisions
The fingerprint metadata for a `ProgramArtifact` built from a `ProgramBuildContext` SHALL reflect
both the root source inputs and the exact loaded library snapshot revisions selected from the
underlying registry through that build context.

#### Scenario: Different loaded library snapshots produce different program revisions
- **WHEN** one `ProgramArtifact` is built from source against a context containing one loaded
  snapshot of `../ui`
- **AND** another `ProgramArtifact` is built from equivalent source against a different context
  containing a different loaded snapshot of `../ui`
- **THEN** the two program artifacts SHALL preserve different fingerprint metadata

### Requirement: Runtime module IDs are assigned only when building a program artifact
The interpreter-visible runtime module IDs for NX execution SHALL be assigned only when assembling
the `ResolvedProgram` inside one `ProgramArtifact`. Loaded `LibraryArtifact` snapshots and the
`LibraryRegistry` SHALL NOT persist or expose program runtime module IDs.

#### Scenario: One loaded library snapshot can participate in multiple programs
- **WHEN** a host loads `../ui` into a `LibraryRegistry`
- **AND** later builds two different `ProgramArtifact`s that each reuse that same loaded
  `LibraryArtifact` snapshot
- **THEN** each program build SHALL assign its own runtime module IDs when assembling its
  `ResolvedProgram`
- **AND** the loaded `LibraryArtifact` snapshot SHALL remain free of program-scoped runtime IDs
