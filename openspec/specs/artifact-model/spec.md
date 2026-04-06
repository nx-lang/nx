# artifact-model Specification

## Purpose
Defines the file-preserving analysis and execution artifacts used to represent source files,
libraries, and whole programs in NX.

## Requirements
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
