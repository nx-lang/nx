# workspace-programs Specification

## Purpose
Defines in-memory NX workspaces, workspace validation, workspace-backed program artifact builds,
and the native workspace ABI.

## Requirements
### Requirement: Workspaces model in-memory source modules
The system SHALL expose `NxWorkspace` as the public logical workspace abstraction for a set of NX
modules submitted together for validation, import resolution, and program construction. Each
initial workspace module SHALL be represented as `NxWorkspaceModule` with a logical identity and a
validated UTF-8 source payload.

#### Scenario: Workspace preserves module identities and byte payloads
- **WHEN** a caller creates a workspace containing `chat-link-config.nx` and
  `shared/questions.nx` with UTF-8 source bytes
- **THEN** NX SHALL treat those modules as workspace modules with those logical identities
- **AND** NX SHALL NOT require either module to exist on disk

#### Scenario: Source text is validated before analysis
- **WHEN** a workspace module contains valid UTF-8 source bytes
- **THEN** NX SHALL decode those bytes once before parsing, lowering, diagnostics, and type analysis
- **AND** the original workspace identity SHALL remain the module identity reported to callers

### Requirement: Workspace pipeline is the shared module-graph pipeline
The system SHALL implement workspace analysis and program construction through a shared logical
module-graph pipeline that can be populated by in-memory workspace modules or by filesystem-backed
source providers. The shared pipeline SHALL own parsing, lowering, preparation, import resolution,
diagnostic aggregation, artifact construction, fingerprinting, and entrypoint selection after
source modules have been loaded by a provider.

#### Scenario: In-memory source provider uses shared pipeline
- **WHEN** a caller validates an `NxWorkspace`
- **THEN** NX SHALL load submitted module bytes through an in-memory source provider
- **AND** analyze those modules through the shared module-graph pipeline

#### Scenario: Filesystem source provider uses shared pipeline
- **WHEN** a caller builds a program from filesystem-backed NX source
- **THEN** NX SHALL load source modules through a filesystem source provider
- **AND** analyze those modules through the same module-graph pipeline used for `NxWorkspace`

#### Scenario: Source provider owns IO behavior
- **WHEN** NX analyzes modules through the shared module-graph pipeline
- **THEN** file IO and filesystem canonicalization SHALL occur only inside filesystem-backed source
  providers
- **AND** in-memory workspace analysis SHALL NOT perform filesystem probing for workspace module
  identities

### Requirement: Workspace identities are normalized logical identities
The system SHALL normalize workspace module identities using forward-slash logical path semantics.
Normalization SHALL resolve `.` and `..` segments, reject identities that escape the workspace
root, reject duplicate normalized identities, and avoid filesystem existence or canonicalization
checks.

#### Scenario: Dot segments normalize to a stable identity
- **WHEN** a caller submits a module identity `tenant/./shared/../config.nx`
- **THEN** NX SHALL normalize the identity to `tenant/config.nx`
- **AND** diagnostics for that module SHALL use `tenant/config.nx`

#### Scenario: Escaping identity is rejected
- **WHEN** a caller submits a module identity `../outside.nx`
- **THEN** NX SHALL reject the workspace as invalid
- **AND** the rejection SHALL NOT depend on whether any file named `outside.nx` exists on disk

#### Scenario: Duplicate normalized identities are rejected
- **WHEN** a caller submits modules named `shared/config.nx` and `shared/./config.nx`
- **THEN** NX SHALL reject the workspace as invalid because both normalize to `shared/config.nx`

### Requirement: Workspace validation returns aggregated diagnostics
The system SHALL expose a Rust workspace validation API that analyzes the effective workspace as a
whole against a supplied `ProgramBuildContext` and returns all diagnostics for the submitted
workspace modules. A workspace with no diagnostics SHALL return an empty diagnostics array.

#### Scenario: Valid workspace produces no diagnostics
- **WHEN** a workspace contains `main.nx` and `shared/value.nx` with valid imports and type-correct
  NX source
- **AND** the caller validates that workspace against a build context
- **THEN** NX SHALL return an empty diagnostics array

#### Scenario: Invalid workspace aggregates diagnostics across modules
- **WHEN** `main.nx` contains a missing import
- **AND** `shared/value.nx` contains a type error
- **THEN** workspace validation SHALL return diagnostics for both modules in one result

### Requirement: Workspace program builds use an explicit entry identity
The system SHALL expose a Rust API that builds a `ProgramArtifact` from an `NxWorkspace`, an
explicit entry identity, and a supplied `ProgramBuildContext`. The entry identity SHALL be
normalized using the same logical identity rules as workspace module identities.

#### Scenario: Workspace program artifact builds for selected entry module
- **WHEN** a workspace contains `main.nx` with `let root() = { 42 }`
- **AND** the caller builds a workspace program artifact with entry identity `main.nx`
- **THEN** NX SHALL return a `ProgramArtifact` whose entry identity is `main.nx`

#### Scenario: Missing workspace entry identity reports diagnostics
- **WHEN** a workspace contains `main.nx`
- **AND** the caller requests entry identity `missing.nx`
- **THEN** NX SHALL fail the build with structured diagnostics naming `missing.nx`
- **AND** NX SHALL NOT create a program artifact handle for that request

### Requirement: Native workspace FFI uses borrowed descriptors
The native ABI SHALL expose C-compatible workspace module descriptors containing borrowed identity
and source byte slices. Native workspace functions SHALL read those slices only during the call and
SHALL copy any module data that must outlive the call into NX-owned memory.

#### Scenario: Program artifact outlives caller-owned workspace buffers
- **WHEN** a native caller builds a workspace program artifact from borrowed module descriptors
- **AND** the build succeeds
- **THEN** the returned artifact SHALL own all module data needed for later evaluation
- **AND** NX SHALL NOT retain pointers into the caller-owned descriptor or byte buffers

#### Scenario: Null descriptor array with modules is invalid argument
- **WHEN** a native caller passes `module_count > 0` and a null module descriptor pointer
- **THEN** the native function SHALL return `InvalidArgument`

#### Scenario: Non-empty null module fields are invalid argument
- **WHEN** a native caller passes a descriptor with a non-zero identity length and a null identity
  pointer
- **THEN** the native function SHALL return `InvalidArgument`
- **AND** the same rule SHALL apply to non-zero source lengths with null source pointers

#### Scenario: Invalid UTF-8 passed through FFI is invalid argument
- **WHEN** a native caller passes identity, source, or entry identity bytes that are not valid
  UTF-8
- **THEN** the native function SHALL return `InvalidArgument`

#### Scenario: Duplicate normalized descriptor identities are invalid argument
- **WHEN** a native caller passes descriptors named `shared/config.nx` and `shared/./config.nx`
- **THEN** the native function SHALL return `InvalidArgument`

### Requirement: Native workspace validation serializes diagnostics
The native workspace validation ABI SHALL serialize workspace diagnostics into `out_buffer`.
Validation SHALL return `Ok` when the validation request itself succeeds, even if the serialized
diagnostics array contains user-authored NX errors.

#### Scenario: Valid workspace validation returns empty diagnostics payload
- **WHEN** a native caller validates a valid workspace
- **THEN** the native function SHALL return `Ok`
- **AND** `out_buffer` SHALL contain a serialized empty diagnostics array

#### Scenario: Invalid NX returns diagnostics without interop failure
- **WHEN** a native caller validates a workspace containing type-invalid NX source
- **THEN** the native function SHALL return `Ok`
- **AND** `out_buffer` SHALL contain serialized `NxDiagnostic` values for the NX errors

### Requirement: Native workspace program builds follow artifact-build status conventions
The native workspace program build ABI SHALL return `Ok` with a non-null program artifact handle
when build succeeds. It SHALL return `Error` with serialized diagnostics when NX analysis or
entrypoint validation prevents artifact creation, and `InvalidArgument` for malformed native
inputs.

#### Scenario: Successful native workspace build returns artifact handle
- **WHEN** a native caller builds a valid workspace program artifact for `main.nx`
- **THEN** the native function SHALL return `Ok`
- **AND** the out handle SHALL be non-null

#### Scenario: Static NX errors return Error diagnostics
- **WHEN** a native caller builds a workspace program artifact whose entry module contains a type
  error
- **THEN** the native function SHALL return `Error`
- **AND** `out_buffer` SHALL contain serialized diagnostics for the static NX error
- **AND** the out handle SHALL remain null
