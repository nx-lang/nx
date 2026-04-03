## MODIFIED Requirements

### Requirement: Imports in HIR
The HIR `LoweredModule` struct SHALL include an imports list. Each import SHALL store the library
path, import kind, optional alias for wildcard imports, and for selective imports the imported name
plus an optional qualifier prefix.

#### Scenario: Wildcard import without alias lowered to HIR
- **WHEN** a file contains `import "../ui"`
- **THEN** the HIR `LoweredModule` SHALL contain an import with path `../ui`, kind wildcard, and
  alias `None`

#### Scenario: Namespace import lowered to HIR
- **WHEN** a file contains `import "../ui" as UI`
- **THEN** the HIR `LoweredModule` SHALL contain an import with path `../ui`, kind wildcard, and
  alias `Some("UI")`

#### Scenario: Selective imports lowered to HIR
- **WHEN** a file contains `import { Button, Stack as Layout.Stack } from "../ui"`
- **THEN** the HIR `LoweredModule` SHALL contain an import with path `../ui`, kind selective, and
  entries `[("Button", None), ("Stack", Some("Layout"))]`

### Requirement: Local library imports resolve recursive directory contents
The compiler SHALL treat a local library path as a directory and SHALL resolve it to a
`LibraryArtifact` that loads declarations from every `.nx` file under that directory recursively as
separate `ModuleArtifact`s. Import resolution SHALL use the `LibraryArtifact` export metadata while
preserving per-file `LoweredModule` boundaries rather than flattening the library into one merged
lowered module.

#### Scenario: Wildcard import loads declarations from nested files without merging them
- **WHEN** `../ui` contains `button.nx` declaring `Button` and `forms/input.nx` declaring `Input`,
  and a file contains `import "../ui"`
- **THEN** analysis SHALL make both `Button` and `Input` available from that one import
- **AND** the resolved `LibraryArtifact` SHALL preserve separate `ModuleArtifact`s for `button.nx`
  and `forms/input.nx`

#### Scenario: Selective import resolves through a library export table
- **WHEN** `../ui/forms/input.nx` declares `Input` and a file contains `import { Input } from
  "../ui"`
- **THEN** analysis SHALL resolve `Input` successfully
- **AND** the `LibraryArtifact` export metadata SHALL identify `forms/input.nx` as the owning
  module for `Input`

## ADDED Requirements

### Requirement: Library artifacts record dependency metadata for program resolution
When import resolution builds a `LibraryArtifact`, it SHALL record the normalized set of library
dependencies required by the library's module artifacts so that higher-level program resolution can
construct a `ProgramArtifact` without rescanning source files ad hoc.

#### Scenario: Library dependency metadata aggregates imports across files
- **WHEN** `search-box.nx` imports `../ui` and `indexing.nx` imports `../core` within the same
  library root
- **THEN** the resulting `LibraryArtifact` SHALL record both `../ui` and `../core` as normalized
  library dependencies
- **AND** higher-level program resolution SHALL be able to use that dependency metadata without
  merging the library's lowered modules
