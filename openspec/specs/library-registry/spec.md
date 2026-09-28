# library-registry Specification

## Purpose
Defines the reusable registry that owns analyzed library snapshots and their dependency graph.

## Requirements

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

### Requirement: Loaded library snapshots preserve semantic bindings for IR generation
`LibraryRegistry` SHALL retain or expose the semantic binding targets needed for downstream
`ProgramArtifact` IR generation when libraries are loaded from directories. The retained semantics
SHALL cover exported declarations, same-library peer declarations, and declarations imported from
loaded dependency libraries.

#### Scenario: Directory-loaded library retains dependency type binding
- **WHEN** a host loads `../question-flow` into a `LibraryRegistry`
- **AND** a host loads `../chat-link` whose exported declarations reference types from
  `../question-flow`
- **THEN** the `../chat-link` library snapshot SHALL retain semantic binding targets for those
  dependency type references
- **AND** a later program artifact that imports `../chat-link` SHALL have enough semantic data to
  generate IR for those references without re-reading the library directories

#### Scenario: Reused build contexts preserve library binding semantics
- **WHEN** a host loads a library graph into one `LibraryRegistry`
- **AND** the host creates multiple `ProgramBuildContext`s from that registry
- **THEN** each build context SHALL expose equivalent semantic binding targets for the loaded
  library graph
- **AND** IR generation from program artifacts built through those contexts SHALL resolve the same
  module-qualified nominal type references

### Requirement: Library registries load libraries from in-memory modules
A `LibraryRegistry` SHALL load a library from a logical root identity, an optional version string,
and a set of modules each given as a library-relative identity and source text, without reading a
filesystem. An in-memory library SHALL be analyzed, snapshotted, reused across build contexts,
cycle-checked, and SHALL preserve semantic bindings for IR generation exactly as the same modules
loaded from a directory would. Logical roots SHALL be normalized with the workspace identity rules,
and relative library imports inside an in-memory library SHALL resolve against the logical roots of
libraries loaded into the same registry. A loaded logical root SHALL reserve its namespace: a
workspace module whose identity lies under the root of a library the build context can see or the
program links SHALL fail validation and the build. An in-memory library that loads SHALL answer the
load with its own warnings, info and hints.

#### Scenario: A library loads without a filesystem
- **WHEN** a host loads `libraries/question-flow` from 29 in-memory modules
- **THEN** the registry SHALL retain an analyzed snapshot for `libraries/question-flow`
- **AND** no filesystem access SHALL occur

#### Scenario: One in-memory library imports another
- **WHEN** a host loads `libraries/question-flow` and then `libraries/chat-link`, whose module
  contains `import "../question-flow"`
- **THEN** the import SHALL resolve to the loaded `libraries/question-flow` snapshot
- **AND** the registry SHALL record the dependency between the two roots

#### Scenario: A missing dependency library is diagnosed
- **WHEN** a host loads `libraries/chat-link` before any library at `libraries/question-flow` is
  loaded
- **THEN** the load SHALL fail with a diagnostic naming the unresolved library import
- **AND** the registry SHALL NOT retain a partial snapshot for `libraries/chat-link`

#### Scenario: In-memory and directory loading agree
- **WHEN** the same library sources are loaded once from a directory and once from memory with the
  same version
- **THEN** a program built against each SHALL emit the same NX IR, differing only in how the library
  modules' identities spell the root (a canonical path for the directory, the logical root for
  memory) and in the fingerprints that hash those identities
- **AND** the program SHALL report equal diagnostics

#### Scenario: A root may not be loaded twice with different content
- **WHEN** a host loads `libraries/question-flow` and later loads the same root with different
  source text or version
- **THEN** the second load SHALL fail with a diagnostic naming the root
- **AND** the first snapshot SHALL remain loaded and unchanged

#### Scenario: A workspace module may not take a library module's place
- **WHEN** a registry holds `libraries/question-flow` with module `Step.nx`
- **AND** a workspace built against it holds a module `libraries/question-flow/Step.nx` or
  `libraries/question-flow/Other.nx`
- **THEN** validation and the build SHALL report a `workspace-module-in-library-root` error naming
  the module and the library root
- **AND** the library's declarations SHALL NOT resolve to the workspace module's

#### Scenario: A library's warnings are answered at load
- **WHEN** a host loads a library from memory whose analysis reports only warnings
- **THEN** the load SHALL succeed and answer with those warnings, labelled against the library's
  module identities and positions in its source text
