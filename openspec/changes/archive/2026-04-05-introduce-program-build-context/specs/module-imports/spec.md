## MODIFIED Requirements

### Requirement: Local library imports resolve recursive directory contents
The compiler SHALL treat a local library path as a directory and SHALL resolve it through a loaded
library snapshot in `LibraryRegistry`. Import resolution SHALL use that library snapshot's export
and interface metadata while preserving per-file `LoweredModule` boundaries rather than flattening
the library into one merged lowered module or copying imported declarations into the importing
module's stored artifact.

#### Scenario: Wildcard import resolves through a loaded library snapshot
- **WHEN** a `LibraryRegistry` contains a loaded `../ui` library whose files declare `Button` and
  `Input`
- **AND** a source file contains `import "../ui"`
- **THEN** analysis SHALL make both `Button` and `Input` available from that import
- **AND** the imported declarations SHALL remain associated with their owning library snapshot and
  source modules rather than being persisted as copied items in the importing file's stored module

#### Scenario: Selective import resolves through library export metadata
- **WHEN** a `LibraryRegistry` contains a loaded `../ui` library whose `forms/input.nx` file
  declares `Input`
- **AND** a source file contains `import { Input } from "../ui"`
- **THEN** analysis SHALL resolve `Input` successfully
- **AND** the library snapshot export metadata SHALL identify `forms/input.nx` as the owning module
  for `Input`

#### Scenario: Library snapshot analysis reuses dependency interfaces without foreign HIR copies
- **WHEN** a loaded `../widgets` library imports `../ui`
- **AND** `../ui` is already loaded in the same `LibraryRegistry`
- **THEN** analysis of `../widgets` SHALL use the `../ui` snapshot's interface metadata during its
  transient analysis preparation
- **AND** the stored `LibraryArtifact` for `../widgets` SHALL remain file-local rather than storing
  copied `../ui` HIR items

### Requirement: Library artifacts record dependency metadata for program resolution
When the system builds a `LibraryArtifact`, it SHALL record the normalized set of library
dependencies required by that library so that `LibraryRegistry` can maintain a reusable dependency
graph and later program construction can select the exact loaded closure without rescanning source
files ad hoc.

#### Scenario: Library dependency metadata feeds the registry dependency graph
- **WHEN** `search-box.nx` imports `../ui` and `indexing.nx` imports `../core` within the same
  library root
- **THEN** the resulting `LibraryArtifact` SHALL record both normalized dependency roots
- **AND** `LibraryRegistry` SHALL be able to use that dependency metadata to maintain the loaded
  snapshot graph without merging or embedding dependent libraries into the artifact
