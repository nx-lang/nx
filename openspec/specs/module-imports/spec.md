# module-imports Specification

## Purpose
Define library-based import syntax and resolution for NX, including wildcard, namespace,
selective, and diagnostic behavior.

## Requirements
### Requirement: Wildcard import (global)
The parser SHALL support wildcard imports with the syntax `import "<library-path>"`. All
declarations visible to the importing file from the imported library are brought into scope
unqualified and can be referenced directly by name.

#### Scenario: Wildcard import without alias
- **WHEN** a file contains `import "../ui"`
- **THEN** the parser SHALL produce an IMPORT_STATEMENT containing a WILDCARD_IMPORT node with no
  alias and a LIBRARY_PATH with value `../ui`

#### Scenario: Wildcard import with HTTP zip path
- **WHEN** a file contains `import "https://cdn.example.com/ui.zip"`
- **THEN** the parser SHALL produce an IMPORT_STATEMENT with a WILDCARD_IMPORT node with no alias
  and a LIBRARY_PATH with value `https://cdn.example.com/ui.zip`

### Requirement: Namespace import
The parser SHALL support namespace imports with the syntax `import "<library-path>" as <Namespace>`,
where `<Namespace>` is an identifier. All declarations visible to the importing file from the
imported library SHALL be referenced as `<Namespace>.<identifier>`.

#### Scenario: Namespace import
- **WHEN** a file contains `import "../ui" as UI`
- **THEN** the parser SHALL produce an IMPORT_STATEMENT containing a WILDCARD_IMPORT node with
  namespace `UI` and a LIBRARY_PATH with value `../ui`

### Requirement: Selective imports
The parser SHALL support selective imports with the syntax `import { <Name1>, <Name2> } from
"<library-path>"`, where each name is an identifier. Each imported declaration SHALL be introduced
into scope using its original unqualified name unless the entry uses a qualified alias prefix.

#### Scenario: Single selective import
- **WHEN** a file contains `import { Button } from "../ui"`
- **THEN** the parser SHALL produce an IMPORT_STATEMENT containing a SELECTIVE_IMPORT_LIST with one
  SELECTIVE_IMPORT for `Button`

#### Scenario: Multiple selective imports
- **WHEN** a file contains `import { Button, Input, Label } from "../ui"`
- **THEN** the parser SHALL produce an IMPORT_STATEMENT containing a SELECTIVE_IMPORT_LIST with
  three SELECTIVE_IMPORT entries

### Requirement: Selective imports with aliases
The parser SHALL support selective import qualifier prefixes with the syntax
`import { <Name> as <Prefix>.<Name> } from "<library-path>"`. The alias after `as` MUST be a
single-dot qualified name whose final segment matches the imported declaration name.

#### Scenario: Selective import with qualified alias prefix
- **WHEN** a file contains `import { Stack as Layout.Stack } from "../layout"`
- **THEN** the parser SHALL produce an IMPORT_STATEMENT with a SELECTIVE_IMPORT_LIST containing a
  SELECTIVE_IMPORT with name `Stack` and alias `Layout.Stack`

#### Scenario: Selective import alias cannot rename the declaration
- **WHEN** a file contains `import { Stack as Layout.Panel } from "../layout"`
- **THEN** analysis SHALL reject the import because the alias does not end with `Stack`

#### Scenario: Selective import alias must contain exactly one dot
- **WHEN** a file contains `import { Stack as Layout.Components.Stack } from "../layout"`
- **THEN** analysis SHALL reject the import because the alias is not a single-dot qualified name

### Requirement: Module path supports file paths and URLs
The `<library-path>` in an import statement SHALL accept relative directory paths, absolute
directory paths, Git URL directory paths, and HTTP URL paths to zip files as string literals.

#### Scenario: Relative directory path
- **WHEN** a file contains `import { Foo } from "./foo"`
- **THEN** the parser SHALL accept the path `./foo`

#### Scenario: Absolute directory path
- **WHEN** a file contains `import { Foo } from "/lib/foo"`
- **THEN** the parser SHALL accept the path `/lib/foo`

#### Scenario: Git URL directory path
- **WHEN** a file contains `import { Foo } from "git://example.com/acme/ui"`
- **THEN** the parser SHALL accept the path `git://example.com/acme/ui`

#### Scenario: HTTP zip path
- **WHEN** a file contains `import { Foo } from "https://example.com/ui.zip"`
- **THEN** the parser SHALL accept the path `https://example.com/ui.zip`

### Requirement: Selective import requires from clause
Selective imports MUST include a `from "<library-path>"` clause. The old `import qualified.name`
syntax is removed.

#### Scenario: Import without from clause is a parse error
- **WHEN** a file contains `import ui.components`
- **THEN** the parser SHALL produce a parse error

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

### Requirement: Multiple import statements
A module SHALL support zero or more import statements, all appearing before definitions.

#### Scenario: Multiple imports
- **WHEN** a file contains two import statements from different library paths
- **THEN** the parser SHALL produce a valid module with both import statements

#### Scenario: No imports
- **WHEN** a file contains only definitions and an element
- **THEN** the parser SHALL produce a valid module with an empty imports list

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

### Requirement: Remote library paths are parsed before resolution support exists
The compiler SHALL accept Git and HTTP library paths syntactically but MUST report a diagnostic if
semantic resolution attempts to load them before remote resolution support is implemented.

#### Scenario: Git library path is not yet resolvable
- **WHEN** a file contains `import "git://example.com/acme/ui"`
- **THEN** parsing SHALL succeed
- **AND** semantic resolution SHALL report that Git library imports are not yet supported

#### Scenario: HTTP zip library path is not yet resolvable
- **WHEN** a file contains `import "https://example.com/ui.zip"`
- **THEN** parsing SHALL succeed
- **AND** semantic resolution SHALL report that HTTP zip library imports are not yet supported

### Requirement: Duplicate library imports are rejected per source file
The compiler SHALL report a compile error when the same normalized library path is imported more
than once in a single file, regardless of import form.

#### Scenario: Wildcard and selective imports from the same library are rejected
- **WHEN** a file contains `import "../ui"` and `import { Button } from "../ui"`
- **THEN** analysis SHALL report a duplicate-library-import compile error for `../ui`

#### Scenario: Equivalent normalized paths are rejected
- **WHEN** a file contains `import "../ui"` and `import "../theme/../ui"`
- **THEN** analysis SHALL report a duplicate-library-import compile error after path normalization

### Requirement: Ambiguous imported names are diagnosed on use
When two different imported libraries expose the same unqualified declaration name, the compiler
SHALL allow the imports to coexist and SHALL report a compile error only when that ambiguous
unqualified name is used.

#### Scenario: Unused ambiguity does not fail compilation
- **WHEN** a file imports `../ui` and `../forms`, both libraries export `Button`, and the file does
  not reference `Button`
- **THEN** analysis SHALL accept the imports without an ambiguity error

#### Scenario: Used ambiguous name reports both sources and remediation
- **WHEN** a file imports `../ui` and `../forms`, both libraries export `Button`, and the file
  references `Button`
- **THEN** analysis SHALL report a compile error that names both import sources
- **AND** SHALL suggest switching to a selective import with `as Prefix.Button` or using a
  namespace import
