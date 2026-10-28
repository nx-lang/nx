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
- **THEN** its runtime-provided functions SHALL expose ordinary typed function signatures and declaration identities
- **AND** those declarations SHALL be authored as documented `external` signatures in compiler-owned NX source
- **AND** user code SHALL NOT be able to register or replace those implementations
