## ADDED Requirements

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
