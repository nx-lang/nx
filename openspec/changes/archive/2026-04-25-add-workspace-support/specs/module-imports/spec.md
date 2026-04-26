## ADDED Requirements

### Requirement: Workspace imports resolve by logical module identity
When analyzing or building a workspace, NX SHALL resolve import paths against the importing
module's normalized logical identity. If the normalized target identity exists in the workspace,
NX SHALL bind the import to that workspace module without consulting the filesystem.

#### Scenario: Relative workspace import resolves to submitted module
- **WHEN** `app/main.nx` imports `../shared/questions.nx`
- **AND** the workspace contains `shared/questions.nx`
- **THEN** NX SHALL resolve the import to the submitted `shared/questions.nx` workspace module

#### Scenario: Workspace module takes precedence over library fallback
- **WHEN** a workspace contains `shared/questions.nx`
- **AND** `app/main.nx` imports `../shared/questions.nx`
- **AND** the supplied `ProgramBuildContext` also exposes a library that could otherwise match the
  import string
- **THEN** NX SHALL resolve the import to the workspace module

#### Scenario: Missing workspace import does not probe disk
- **WHEN** `app/main.nx` imports `../shared/missing.nx`
- **AND** the workspace does not contain `shared/missing.nx`
- **AND** the supplied build context does not resolve the import
- **THEN** NX SHALL report a missing import diagnostic for `shared/missing.nx`
- **AND** NX SHALL NOT call filesystem APIs to decide whether that workspace import exists

### Requirement: Filesystem imports use the shared resolver after source loading
Filesystem-backed NX code SHALL use the same logical import resolver as in-memory workspaces after
the filesystem source provider has loaded source modules and assigned normalized module identities.
Filesystem discovery, existence checks, and canonicalization SHALL remain provider responsibilities
and SHALL NOT be embedded in the shared resolver.

#### Scenario: File-backed relative import resolves through shared module graph
- **WHEN** a filesystem source provider loads `app/main.nx` and `shared/questions.nx`
- **AND** `app/main.nx` imports `../shared/questions.nx`
- **THEN** the shared resolver SHALL resolve the import to the normalized loaded module identity
  for `shared/questions.nx`

#### Scenario: Shared resolver does not perform filesystem discovery
- **WHEN** the shared resolver is asked to resolve an import
- **THEN** it SHALL use the loaded module graph and `ProgramBuildContext`
- **AND** it SHALL NOT scan directories, stat paths, or canonicalize filesystem paths itself

### Requirement: Workspace import normalization rejects root escapes
Workspace import resolution SHALL reject relative imports that normalize outside the workspace
root.

#### Scenario: Relative import escaping workspace root is rejected
- **WHEN** `app/main.nx` imports `../../outside.nx`
- **THEN** NX SHALL report a workspace import diagnostic that the target escapes the workspace root
- **AND** NX SHALL NOT attempt to resolve that escaped target through the filesystem
