## ADDED Requirements

### Requirement: Workspace source analysis aggregates module diagnostics
The shared source-analysis pipeline SHALL support analyzing multiple in-memory workspace modules as
one effective workspace. Workspace analysis SHALL parse, lower, prepare, validate, resolve scopes,
and type-check submitted modules with the same semantic ordering used for program construction.

#### Scenario: Multiple workspace module errors are returned together
- **WHEN** a workspace contains `main.nx` with a lowering error
- **AND** the same workspace contains `shared/value.nx` with a type error
- **THEN** workspace analysis SHALL return diagnostics for both modules in one result

#### Scenario: Workspace analysis does not require per-module caller loops
- **WHEN** a caller validates an `NxWorkspace` containing three modules
- **THEN** NX SHALL analyze the effective workspace through one workspace validation call
- **AND** callers SHALL NOT be required to invoke single-source analysis once per module

### Requirement: Shared source analysis accepts module source providers
The shared source-analysis pipeline SHALL operate on a logical set of source modules provided by a
source provider rather than being tied directly to either in-memory workspace descriptors or
filesystem paths. In-memory and filesystem-backed callers SHALL use the same analysis ordering once
their source modules have been loaded.

#### Scenario: File-backed analysis uses the shared module set
- **WHEN** a caller analyzes filesystem-backed NX source
- **THEN** the filesystem source provider SHALL load the source modules
- **AND** shared source analysis SHALL parse, lower, prepare, validate, resolve scopes, and
  type-check those modules through the same module-set path used by workspace validation

#### Scenario: Provider differences do not change static analysis ordering
- **WHEN** equivalent NX modules are supplied once through `NxWorkspace` and once through a
  filesystem source provider
- **THEN** shared source analysis SHALL run the same static phases in the same order for both
  source providers

### Requirement: Workspace diagnostics use submitted source maps
Diagnostic conversion for workspace analysis SHALL calculate label spans from the submitted
workspace source text associated with each normalized identity before falling back to any
file-backed diagnostic behavior.

#### Scenario: Path-like workspace identity is not re-read from disk
- **WHEN** a workspace module identity is `shared/config.nx`
- **AND** a different file with that name exists on disk
- **AND** workspace validation reports a diagnostic in `shared/config.nx`
- **THEN** the diagnostic span SHALL be calculated from the submitted workspace source text
- **AND** NX SHALL NOT re-read the disk file to compute the line and column

#### Scenario: Workspace diagnostic labels preserve normalized identity
- **WHEN** a caller submits `shared/./config.nx`
- **AND** workspace analysis reports a diagnostic in that module
- **THEN** the diagnostic label file SHALL be `shared/config.nx`
