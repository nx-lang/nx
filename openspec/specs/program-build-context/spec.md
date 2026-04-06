# program-build-context Specification

## Purpose
Defines the registry-backed build scope used to select visible library snapshots for one program
build or tenant.

## Requirements
### Requirement: Program build contexts are registry-backed build scopes
The system SHALL expose a reusable `ProgramBuildContext` created from a `LibraryRegistry`.
`ProgramBuildContext` SHALL define which loaded library snapshots are visible for one program build
or tenant scope, but it SHALL NOT own the underlying library snapshot cache.

#### Scenario: Two build contexts can reuse one registry-owned shared library
- **WHEN** a host loads `../question-flow` into one `LibraryRegistry`
- **AND** creates two different `ProgramBuildContext`s from that registry
- **AND** each build context later builds source that imports `../question-flow`
- **THEN** both builds SHALL resolve `../question-flow` through the same registry-owned snapshot

#### Scenario: Build context can limit visible loaded libraries
- **WHEN** a `LibraryRegistry` contains loaded `../question-flow` and `../internal-admin`
  snapshots
- **AND** a `ProgramBuildContext` is created for a tenant that should only see `../question-flow`
- **THEN** builds performed with that context SHALL resolve `../question-flow`
- **AND** SHALL NOT treat `../internal-admin` as visible from that context

#### Scenario: Build context selects the transitive closure of a visible library
- **WHEN** a `LibraryRegistry` contains a loaded `../widgets` snapshot whose dependency metadata
  names `../ui`
- **AND** a `ProgramBuildContext` is created that exposes `../widgets` but not `../ui` as a direct
  visible root
- **AND** the host builds source that imports `../widgets`
- **THEN** program construction SHALL select the visible loaded `../widgets` snapshot from that
  build context
- **AND** SHALL include the loaded `../ui` snapshot from the underlying registry as part of the
  selected program library closure

### Requirement: Program builds resolve imports through the supplied build context
The system SHALL build imported source into a `ProgramArtifact` using a caller-supplied
`ProgramBuildContext`. Local library imports SHALL be normalized relative to the root source file
identity and looked up in the visible library snapshots exposed by that context instead of being
loaded ad hoc from disk during program construction.

#### Scenario: Program build succeeds when required library is preloaded
- **WHEN** a host loads `../question-flow` into a `LibraryRegistry`
- **AND** creates a `ProgramBuildContext` that exposes `../question-flow`
- **AND** the host builds source file `app/main.nx` that contains `import "../question-flow"`
- **THEN** program construction SHALL succeed using the visible loaded library snapshot from the
  registry-backed context

#### Scenario: Program build fails when required library is missing from context
- **WHEN** a host builds source file `app/main.nx` against a `ProgramBuildContext` that does not
  expose a loaded `../question-flow` snapshot
- **AND** that source contains `import "../question-flow"`
- **THEN** program construction SHALL fail with a library-load diagnostic for the missing
  normalized root
- **AND** the system SHALL NOT silently load `../question-flow` from disk during that build

#### Scenario: Program build reports an incomplete loaded dependency closure
- **WHEN** a `ProgramBuildContext` resolves a direct `import "../widgets"` through a visible loaded
  `../widgets` snapshot
- **AND** that loaded `../widgets` snapshot records `../ui` in its dependency metadata
- **AND** the underlying `LibraryRegistry` no longer contains the loaded `../ui` snapshot needed to
  complete that closure
- **THEN** program construction SHALL return a diagnostic that names the discovered dependency
  chain
- **AND** SHALL NOT silently omit `../ui` from the selected program library closure

### Requirement: Build contexts are build-time only
`ProgramBuildContext` SHALL participate only in program construction. Once a `ProgramArtifact` has
been built successfully, runtime evaluation and component lifecycle calls SHALL execute against
that `ProgramArtifact` without requiring the build context to remain alive.

#### Scenario: Program artifact remains executable after build context is released
- **WHEN** a host builds a `ProgramArtifact` from source using a `ProgramBuildContext`
- **AND** the host releases that `ProgramBuildContext`
- **THEN** evaluation of the resulting `ProgramArtifact` SHALL still succeed
