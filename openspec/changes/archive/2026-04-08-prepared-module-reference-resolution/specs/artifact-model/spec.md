## MODIFIED Requirements

### Requirement: Library artifacts remain local snapshots with dependency-analysis interfaces
`LibraryArtifact` SHALL remain a persistent snapshot of one library root only. It SHALL preserve
that library's local `ModuleArtifact`s, stable local definition identities for the library's own
top-level declarations, export and library-visible binding metadata derived from those definitions,
published interface metadata, dependency metadata, diagnostics, and fingerprint. A `LibraryArtifact`
SHALL NOT embed dependent library artifacts, persist prepared modules, or persist build-context-
specific visible namespaces.

#### Scenario: Loaded library preserves raw per-file modules and stable library-owned indexes
- **WHEN** a library root contains `button.nx` declaring `export component <Button/> = { <button/> }`
- **AND** `theme.nx` declaring `type Theme = string`
- **THEN** the resulting `LibraryArtifact` SHALL preserve separate raw `ModuleArtifact`s for
  `button.nx` and `theme.nx`
- **AND** the resulting `LibraryArtifact` SHALL preserve stable definition identities for the
  top-level declarations owned by those files
- **AND** the resulting `LibraryArtifact` SHALL preserve library-owned export and library-visible
  binding metadata for those declarations

#### Scenario: Loaded library does not persist prepared modules for peer visibility
- **WHEN** `helpers.nx` in one library declares `let answer() = 42`
- **AND** `main.nx` in that same library references `answer()`
- **THEN** the loaded `LibraryArtifact` SHALL preserve raw `ModuleArtifact`s for `helpers.nx` and
  `main.nx`
- **AND** the loaded `LibraryArtifact` SHALL NOT need to persist a prepared module for `main.nx`
  that embeds peer-visible `answer`

#### Scenario: Loaded library does not persist build-context-specific imported visibility
- **WHEN** a library imports `../ui`
- **AND** a host later builds two different `ProgramBuildContext`s that expose different direct
  roots from one shared `LibraryRegistry`
- **THEN** the loaded `LibraryArtifact` for that library SHALL preserve its raw modules,
  definition identities, and published interface metadata
- **AND** it SHALL NOT persist one caller-specific prepared imported namespace inside the artifact
