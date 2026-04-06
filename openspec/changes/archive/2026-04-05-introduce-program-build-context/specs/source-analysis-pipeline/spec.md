## ADDED Requirements

### Requirement: Shared source analysis resolves imports through a supplied registry-backed build context
Shared source analysis SHALL resolve local library imports through a supplied `ProgramBuildContext` backed by a `LibraryRegistry`. Shared analysis SHALL NOT silently load missing local libraries from disk during that request.
Program construction SHALL reuse that same direct-import resolution result when selecting library
snapshots for the resulting `ProgramArtifact`.

#### Scenario: Shared analysis succeeds with a preloaded imported library
- **WHEN** a host loads `../question-flow` into a `LibraryRegistry`
- **AND** creates a `ProgramBuildContext` from that registry
- **AND** analyzes source file `app/main.nx` containing `import "../question-flow"`
- **THEN** shared analysis SHALL resolve the import through the supplied build context

#### Scenario: Shared analysis reports a missing library from the build context
- **WHEN** a host analyzes source file `app/main.nx` containing `import "../question-flow"`
- **AND** the supplied `ProgramBuildContext` does not expose a loaded `../question-flow` snapshot
- **THEN** shared analysis SHALL return a library-load diagnostic for the missing normalized root
- **AND** the analysis call SHALL NOT silently load `../question-flow` from disk

#### Scenario: Program construction does not re-select a direct library hidden from analysis
- **WHEN** a `LibraryRegistry` contains loaded `../ui` and `../internal-admin` snapshots
- **AND** a `ProgramBuildContext` exposes only `../ui`
- **AND** source file `app/main.nx` contains `import "../internal-admin"`
- **THEN** shared analysis SHALL report the missing loaded `../internal-admin` snapshot from that
  build context
- **AND** the resulting `ProgramArtifact` SHALL NOT silently select the hidden
  `../internal-admin` snapshot during later program assembly

### Requirement: Core type analysis operates on caller-prepared lowered modules
The shared type-analysis core SHALL treat import resolution as a caller-owned preparation step. Lower-level analysis entry points SHALL accept an already-lowered or already-prepared module and SHALL NOT perform implicit filesystem library loading themselves.

#### Scenario: Prepared-module analysis preserves imports without loading libraries
- **WHEN** a caller lowers `app/main.nx` that imports `../question-flow`
- **AND** passes that lowered module to the shared type-analysis core without an import-preparation step
- **THEN** the analysis result SHALL preserve the module's import metadata
- **AND** SHALL NOT silently load `../question-flow` from disk during that analysis call
