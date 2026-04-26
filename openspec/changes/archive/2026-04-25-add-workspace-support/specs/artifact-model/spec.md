## ADDED Requirements

### Requirement: Program artifacts preserve workspace root modules and entry module
A `ProgramArtifact` built from an `NxWorkspace` SHALL preserve the analyzed workspace root modules
needed for program execution, the selected normalized entry identity, the selected runtime module id
when the entry module lowers successfully, diagnostics, resolved libraries selected through
`ProgramBuildContext`, fingerprint metadata, and the embedded `ResolvedProgram`.

#### Scenario: Workspace artifact records selected entry identity and module id
- **WHEN** a caller builds a workspace program artifact for entry identity `app/main.nx`
- **THEN** the resulting `ProgramArtifact` SHALL record `app/main.nx` as the selected entry
  identity
- **AND** the artifact SHALL record the `RuntimeModuleId` for the selected entry module when that
  module lowers successfully

#### Scenario: Workspace artifact preserves submitted modules separately
- **WHEN** a workspace contains `app/main.nx` and `shared/questions.nx`
- **AND** the caller builds a workspace program artifact for `app/main.nx`
- **THEN** the resulting `ProgramArtifact` SHALL preserve separate `ModuleArtifact`s for the
  submitted workspace modules that participate in analysis and execution
- **AND** those module artifacts SHALL use normalized workspace identities rather than filesystem
  paths

### Requirement: Program artifacts use provider-neutral entry identity
A `ProgramArtifact` built from any source provider SHALL preserve a selected normalized entry
module identity. Runtime entrypoint lookup SHALL use the selected entry module's runtime id for
both in-memory and filesystem-backed builds.

#### Scenario: Filesystem artifact records normalized entry identity
- **WHEN** a caller builds a program artifact from filesystem-backed source file `app/main.nx`
- **THEN** the resulting `ProgramArtifact` SHALL record the normalized module identity for
  `app/main.nx` as its selected entry identity

#### Scenario: Single-source helper still produces explicit entry identity
- **WHEN** a caller builds a program from one source string using a convenience helper
- **THEN** the resulting `ProgramArtifact` SHALL still preserve an explicit selected entry module
  identity

### Requirement: Resolved modules separate runtime identity from source provenance
`ResolvedProgram` SHALL identify executable modules by `RuntimeModuleId` and SHALL represent module
source provenance with a typed value rather than overloading one string field for both logical
workspace identities and filesystem paths.

#### Scenario: Source-provider modules use logical identities
- **WHEN** a workspace program artifact includes source-provider module `app/main.nx`
- **THEN** the corresponding `ResolvedModule` SHALL preserve `app/main.nx` as source-provider
  provenance
- **AND** source-provider lookup by `app/main.nx` SHALL return that module's `RuntimeModuleId`

#### Scenario: Library modules use library provenance
- **WHEN** a workspace program artifact includes a resolved library module from a filesystem path
- **THEN** the corresponding `ResolvedModule` SHALL preserve library root and module path
  provenance
- **AND** source-provider lookup SHALL NOT treat that library module path as a logical workspace
  identity

### Requirement: Workspace artifact fingerprints include in-memory module inputs
The fingerprint metadata for a `ProgramArtifact` built from an `NxWorkspace` SHALL reflect the
normalized workspace module identities, their submitted source text, the selected entry identity,
and any selected library snapshot revisions from the supplied build context.

#### Scenario: Source text change changes workspace artifact fingerprint
- **WHEN** two workspace program artifacts use the same identities and entry identity
- **AND** `shared/questions.nx` has different source text in the second workspace
- **THEN** the two program artifacts SHALL preserve different fingerprint metadata

#### Scenario: Entry identity change changes workspace artifact fingerprint
- **WHEN** one workspace contains `a.nx` and `b.nx`
- **AND** one artifact is built with entry identity `a.nx`
- **AND** another artifact is built from the same module bytes with entry identity `b.nx`
- **THEN** the two program artifacts SHALL preserve different fingerprint metadata
