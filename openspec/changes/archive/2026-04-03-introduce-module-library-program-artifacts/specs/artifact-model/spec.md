## ADDED Requirements

### Requirement: Module artifacts cache per-source analysis products
The system SHALL represent the cached derived state for one NX source file as a `ModuleArtifact`.
A `ModuleArtifact` SHALL correspond to exactly one source file and SHALL preserve that file's
source identity or fingerprint, parse result metadata, `LoweredModule` when parsing succeeds,
`TypeEnvironment` when type analysis runs, diagnostics, and import or dependency metadata.

#### Scenario: Successful analysis populates a module artifact
- **WHEN** shared source analysis succeeds for `widgets/search-box.nx`
- **THEN** the result SHALL include one `ModuleArtifact` for `widgets/search-box.nx`
- **AND** that `ModuleArtifact` SHALL preserve the file's `LoweredModule`
- **AND** that `ModuleArtifact` SHALL preserve the file's `TypeEnvironment`
- **AND** that `ModuleArtifact` SHALL preserve diagnostics and import metadata for that file

#### Scenario: Parse failure still yields a file-scoped artifact record
- **WHEN** shared source analysis is run for malformed source such as `let root( =`
- **THEN** the result SHALL still identify the analyzed source file through a `ModuleArtifact`
- **AND** that `ModuleArtifact` SHALL preserve the parse diagnostics for that file
- **AND** that `ModuleArtifact` SHALL NOT include a `LoweredModule`

### Requirement: Library artifacts preserve one module artifact per source file
The system SHALL represent one library as a `LibraryArtifact`. A `LibraryArtifact` SHALL preserve
one `ModuleArtifact` per `.nx` source file in the library, along with library root identity or
fingerprint, export tables, dependency metadata, and library-level diagnostics. A `LibraryArtifact`
MUST NOT replace those file-scoped artifacts with one merged lowered module.

#### Scenario: Library artifact keeps nested files separate
- **WHEN** a library root contains `button.nx` declaring `Button` and `forms/input.nx` declaring
  `Input`
- **THEN** loading that library SHALL produce one `LibraryArtifact`
- **AND** the `LibraryArtifact` SHALL contain separate `ModuleArtifact`s for `button.nx` and
  `forms/input.nx`
- **AND** the `LibraryArtifact` SHALL expose exports for both `Button` and `Input` without merging
  those files into one `LoweredModule`

#### Scenario: Library artifact records library-level dependencies
- **WHEN** one file in a library imports `../ui` and another file in the same library imports
  `../core`
- **THEN** the resulting `LibraryArtifact` SHALL record both normalized library dependencies
- **AND** the dependency metadata SHALL remain associated with that library rather than being
  scattered across unrelated runtime caches

### Requirement: Program artifacts contain the resolved executable world
The system SHALL represent a fully resolved executable program as a `ProgramArtifact`. A
`ProgramArtifact` SHALL preserve the root module set, resolved `LibraryArtifact` dependencies,
whole-program diagnostics and fingerprint metadata, and an embedded `ResolvedProgram` used by the
interpreter.

#### Scenario: Program artifact combines root modules and resolved libraries
- **WHEN** a host builds a program from one root source file that imports `../ui`
- **THEN** the resulting `ProgramArtifact` SHALL preserve the root file as a `ModuleArtifact`
- **AND** the resulting `ProgramArtifact` SHALL preserve the imported library as a `LibraryArtifact`
- **AND** the resulting `ProgramArtifact` SHALL include a `ResolvedProgram` for interpreter entry
  point lookup

#### Scenario: Program artifact preserves whole-program diagnostics
- **WHEN** whole-program resolution finds a duplicate export or import ambiguity across resolved
  libraries
- **THEN** the resulting `ProgramArtifact` SHALL preserve those diagnostics as whole-program
  diagnostics
- **AND** the `ProgramArtifact` fingerprint metadata SHALL correspond to the exact root modules and
  resolved libraries used to build it
