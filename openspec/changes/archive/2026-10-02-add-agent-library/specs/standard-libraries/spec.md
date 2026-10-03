## Purpose

Gives NX libraries of its own: NX source the compiler carries and a module imports by a reserved
name, `@nx/<name>`, so that product-neutral declarations more than one host needs are spelled,
versioned and documented once, and are available in every tool with nothing supplied by the host.

## ADDED Requirements

### Requirement: Standard libraries are carried by the compiler and imported by name
The system SHALL carry each standard library as NX source inside the compiler and SHALL make it
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

### Requirement: A standard library can be an implicit import
A build context's or a workspace build's implicit imports SHALL accept the root of a standard
library, and naming one SHALL behave exactly as the written wildcard import `import "@nx/<name>"`
does in every module that receives implicit imports. A module that already writes that alias-less
wildcard import SHALL NOT receive it a second time.

#### Scenario: A host puts the agent library in scope for tenant source
- **WHEN** a workspace is built with implicit imports `["@nx/agent"]` and its entry module contains `let root() = { <Agent name="support">Be brief.</Agent> }` with no import line
- **THEN** the build SHALL succeed and `root` SHALL evaluate to an `Agent` record

#### Scenario: An implicit import of an unknown standard library is refused
- **WHEN** a workspace is built with implicit imports `["@nx/nope"]`
- **THEN** the build SHALL fail with a diagnostic naming `@nx/nope` and listing the standard libraries that exist

### Requirement: The `@nx/` root is reserved
Every identity under the root `@nx/` SHALL be reserved for the prelude and the standard libraries. A
workspace that supplies a module whose identity lies under `@nx/` SHALL be refused with a diagnostic
naming the identity, and a host that loads a library whose root lies under `@nx/` SHALL be refused
with a diagnostic naming the root. A diagnostic label that points into a standard library SHALL carry
the module's reserved identity, and rendering it SHALL show the library's own source and SHALL NOT
read a file from disk.

#### Scenario: A workspace cannot supply a module under the reserved root
- **WHEN** a workspace contains a module whose identity is `@nx/agent/agent.nx` or `@nx/other.nx`
- **THEN** the build SHALL fail with a diagnostic saying the `@nx/` root is reserved and naming the identity

#### Scenario: A host cannot load a library under the reserved root
- **WHEN** a host loads an in-memory library with root `@nx/agent` or `@nx/mine`
- **THEN** the load SHALL fail with a diagnostic naming the root
- **AND** the registry SHALL retain nothing for it

#### Scenario: A label inside a standard library names its module
- **WHEN** a diagnostic in a user module carries a secondary label at the declaration of `Agent.name`
- **THEN** that label's file SHALL be `@nx/agent/agent.nx`
- **AND** rendering the diagnostic SHALL quote that module's source line

### Requirement: A standard library module is an ordinary NX IR module with a content-derived version
A standard library module SHALL be an NX IR module like any library module, identified by the
library root joined to the library-relative module identity (`@nx/agent/agent.nx`). A module that
references a standard library declaration SHALL list that module in its module table and reference
the declaration through that slot, and no standard library declaration SHALL be copied into another
module's image. The version recorded for a standard library module SHALL be derived from the
identities and source text of every module of that library, SHALL be the same on every platform and
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

### Requirement: A standard library declares its stability
Each standard library SHALL have a stability, `unstable` or `stable`, stated in its source header, in
its reference documentation and in the release notes of every release that changes it. An `unstable`
library's declarations MAY change incompatibly in any release, including a patch release, with no
deprecation period. A `stable` library's declarations SHALL change incompatibly only in a release
whose version number marks a breaking change under the repository's release versioning. A change to
a standard library SHALL reach hosts only through a release of the packages that carry the compiler.
The `agent` library SHALL be `unstable`.

#### Scenario: The agent library says it is unstable
- **WHEN** a reader opens the agent library's source or its reference page
- **THEN** each SHALL state that the library is unstable and what that permits

#### Scenario: A release that changes an unstable library says so
- **WHEN** a release changes a declaration of `@nx/agent`
- **THEN** that release's release notes SHALL list the change under the library's name

#### Scenario: A pinned host is unaffected by a library change
- **WHEN** a host stays on one release of the NX packages
- **THEN** the `@nx/agent` declarations and the version recorded in its images SHALL NOT change
