## ADDED Requirements

### Requirement: Workspace program evaluation uses entry-module scoped root
Runtime evaluation of a `ProgramArtifact` built from an `NxWorkspace` SHALL execute `root()` from
the artifact's selected entry module. Runtime entrypoint resolution SHALL NOT choose `root()` from
another workspace module merely because that module appears earlier in a global symbol table.

#### Scenario: Two workspace modules can define root
- **WHEN** a workspace contains `a.nx` with `let root() = { "a" }`
- **AND** the same workspace contains `b.nx` with `let root() = { "b" }`
- **AND** the caller builds a workspace program artifact with entry identity `b.nx`
- **THEN** evaluating that artifact SHALL return `"b"`

#### Scenario: Entry module without root reports that entry module
- **WHEN** a workspace contains `entry.nx` without a `root()` function
- **AND** another workspace module contains a valid `root()` function
- **AND** the caller builds and evaluates a workspace program artifact with entry identity
  `entry.nx`
- **THEN** NX SHALL report a no-root diagnostic for `entry.nx`
- **AND** NX SHALL NOT execute `root()` from the other module

### Requirement: Workspace entrypoints are module-qualified in resolved programs
`ResolvedProgram` SHALL provide enough module-qualified entrypoint information for workspace program
artifacts to resolve runtime entrypoints by selected module id and symbol name.

#### Scenario: Workspace root lookup uses selected runtime module reference
- **WHEN** a workspace program artifact records `app/main.nx` as its selected entry identity
- **AND** the selected entry module lowers successfully
- **THEN** runtime `root()` lookup SHALL use the selected entry module's `RuntimeModuleId`
- **AND** execution SHALL use the owning lowered module for that reference
