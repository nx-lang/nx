## ADDED Requirements

### Requirement: Managed API exposes reusable library registries and program build contexts
The managed NX binding SHALL expose a disposable `NxLibraryRegistry` API that lets callers load
local NX library roots before any program exists, and a disposable `NxProgramBuildContext` API
created from that registry for building transient `NxProgramArtifact`s from source.

#### Scenario: Managed host preloads a shared library before building any program
- **WHEN** a .NET host creates an `NxLibraryRegistry`
- **AND** loads `../question-flow` into it
- **THEN** the managed API SHALL retain that analyzed library snapshot without requiring an
  `NxProgramArtifact` to exist yet

#### Scenario: Managed caller reuses one loaded library across multiple program builds
- **WHEN** a .NET host creates one `NxLibraryRegistry`
- **AND** loads `../question-flow` into it
- **AND** creates build contexts from that registry
- **AND** builds two `NxProgramArtifact`s from different source strings that each import
  `../question-flow`
- **THEN** both program builds SHALL succeed using that same managed registry-backed workflow

#### Scenario: Managed program build reports missing library from context
- **WHEN** a .NET host builds source that imports `../question-flow` against a build context that
  has not loaded that library
- **THEN** the managed API SHALL surface a build exception describing the missing library load

### Requirement: Managed reusable-library workflow is registry-based
The managed binding SHALL provide reusable library caching through `NxLibraryRegistry` and use
`NxProgramBuildContext` as the build-time selection scope rather than exposing a standalone
reusable library-artifact host API.

#### Scenario: Managed host loads a library for later program builds
- **WHEN** a .NET host wants to cache `../question-flow` for repeated use
- **THEN** the supported managed workflow SHALL be to load that library into an
  `NxLibraryRegistry`
- **AND** later source-based program builds SHALL consume an `NxProgramBuildContext` created from
  that registry

### Requirement: Managed source component convenience is implemented via transient program artifacts
The managed binding SHALL keep source-based component convenience APIs, but implement them by
building transient `NxProgramArtifact`s and then calling the native program-artifact component
entry points rather than depending on separate source-based component FFI entry points.

#### Scenario: Managed source component initialization uses a build context through a transient artifact
- **WHEN** a .NET host calls a source-based component initialization helper with a
  `NxProgramBuildContext`
- **THEN** the managed binding SHALL build a transient `NxProgramArtifact` with that context
- **AND** SHALL initialize the component through the native program-artifact component API

#### Scenario: Managed source component dispatch uses the same source revision through a transient artifact
- **WHEN** a .NET host calls a source-based component dispatch helper
- **THEN** the managed binding SHALL build a transient `NxProgramArtifact` for that source revision
- **AND** SHALL dispatch through the native program-artifact dispatch API
