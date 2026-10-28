## MODIFIED Requirements

### Requirement: Standard libraries are carried by the compiler and imported by name
The system SHALL carry each standard library as compiler-owned declarations inside the compiler, using NX source for authored declarations and trusted typed descriptors for runtime-provided standard functions and SHALL make it
available, under the logical root `@nx/<name>`, to every analysis and build: a single-source build, a
workspace build or validation, a directory library build, an in-memory library load, a
language-service snapshot, `typegen`, and every SDK entry point that reaches one of these. No caller
SHALL have to supply, load or enable a standard library, and resolving one SHALL NOT read a
filesystem. A standard library SHALL be analyzed at most once per process, SHALL be analyzed only
when something imports it, and SHALL itself be free of diagnostics.

A standard library's declarations SHALL NOT be in scope in a module that neither imports the library
nor receives it as an implicit import. Once imported, a standard library SHALL behave as any library
does: its `export` declarations are visible under the import form written, its other declarations
are not, and its names take part in the ordinary ambiguity and duplicate-import rules.

#### Scenario: A single source imports a standard library
- **WHEN** a single-source build with an empty build context is given `import "@nx/agent"` followed by `let a = <Agent name="support">Be brief.</Agent>`
- **THEN** analysis SHALL accept the source with `a` typed `Agent`

#### Scenario: A standard library is not in scope without an import
- **WHEN** a single-source build is given `let a: Agent = <Agent name="support">Be brief.</Agent>` with no import and no implicit import
- **THEN** analysis SHALL report `Agent` as unresolved

#### Scenario: A module's own declaration coexists with an unimported standard library
- **WHEN** a file declares `type Tool = { label:string }` and does not import `@nx/agent`
- **THEN** analysis SHALL accept the file exactly as it did before standard libraries existed
- **AND** the program's emitted NX IR SHALL be byte-identical to what it was before

#### Scenario: A library extends a standard library's types
- **WHEN** a host loads an in-memory library whose module contains `import "@nx/agent"` and `export type RecordSearchTool extends Tool = { recordKind:string maxResults:int = 5 }`
- **THEN** the load SHALL succeed with no host having loaded `@nx/agent`
- **AND** a program importing both libraries SHALL accept `<RecordSearchTool recordKind="company" />` where a `Tool` is expected

#### Scenario: A build context that limits visible roots still sees standard libraries
- **WHEN** a build context is created with visible roots naming one host library
- **AND** a workspace module built against it contains `import "@nx/agent"`
- **THEN** the import SHALL resolve

#### Scenario: A standard library is analyzed once
- **WHEN** one process builds 100 workspaces that each import `@nx/agent`, through any number of registries
- **THEN** `@nx/agent` SHALL be analyzed once

#### Scenario: Runtime-provided math declarations are ordinary imported functions
- **WHEN** a module imports @nx/math under an alias
- **THEN** its runtime-provided functions SHALL expose ordinary typed function signatures and declaration identities without user-visible external-function syntax
- **AND** user code SHALL NOT be able to register or replace those implementations

### Requirement: A standard library module is an ordinary NX IR module with a content-derived version
A standard library module SHALL be an NX IR module like any library module, identified by the
library root joined to the library-relative module identity (`@nx/agent/agent.nx`). A module that
references a standard library declaration SHALL list that module in its module table and reference
the declaration through that slot, and no standard library declaration SHALL be copied into another
module's image. The version recorded for a standard library module SHALL be derived from the
identities and source text of every module of that library and, when present, canonical typed runtime-function descriptors including operation mappings and semantic revisions, SHALL be the same on every platform and
in every binding built from the same sources, and SHALL differ whenever any of that text differs.

An emit request SHALL produce a standard library module's image when it names that module's
identity, and an emit request for every module SHALL include it exactly when the program links it.
The image SHALL be byte-identical whichever program it was emitted from. No IR runtime SHALL carry a
standard library's image: a host supplies it to linking as it supplies any library module's image.
A program's fingerprint SHALL depend on the source of every standard library it links.

#### Scenario: A program that uses the agent library names it in its module table
- **WHEN** a workspace whose entry constructs an `Agent` is built and the entry module is emitted
- **THEN** the entry image's module table SHALL name `@nx/agent/agent.nx` with the library's version
- **AND** the entry image SHALL hold none of the agent library's declarations

#### Scenario: The library image is emitted on request and links
- **WHEN** a caller emits the entry module and `@nx/agent/agent.nx` from that program
- **AND** prepares both with an IR runtime and links the entry with a resolver that returns the library image
- **THEN** linking SHALL succeed and evaluating `root` SHALL return the `Agent` record

#### Scenario: The library image does not depend on the program
- **WHEN** two different workspaces that import `@nx/agent` each emit `@nx/agent/agent.nx`
- **THEN** the two images SHALL be byte-identical

#### Scenario: A missing library image is a link error, not a fallback
- **WHEN** an entry image that names `@nx/agent/agent.nx` is linked with a resolver that returns nothing for it
- **THEN** linking SHALL fail naming `@nx/agent/agent.nx`, as it does for any unresolved library module

#### Scenario: An edit to the library changes its version
- **WHEN** the agent library's source differs between two compiler builds, in a declaration or in a comment
- **THEN** the versions the two builds record for `@nx/agent/agent.nx` SHALL differ
- **AND** an entry image compiled by one SHALL NOT link against the library image emitted by the other unless the host allows a version mismatch

#### Scenario: A program that imports no standard library is unchanged
- **WHEN** a program that imports no standard library is built and every module is emitted
- **THEN** the emitted images SHALL be byte-identical to those emitted before standard libraries existed

#### Scenario: Runtime descriptor changes affect math version
- **WHEN** a runtime-provided math signature, operation mapping or semantic revision changes
- **THEN** the math library version SHALL change deterministically across platforms
- **AND** versions of unchanged source-only libraries SHALL remain unchanged
