## ADDED Requirements

### Requirement: Managed binding exposes byte-oriented workspace APIs
The managed NX binding SHALL expose `NxWorkspace` and `NxWorkspaceModule` public APIs for
workspace-backed validation and program artifact construction. `NxWorkspaceModule` SHALL store the
logical identity as a string and source content as `ReadOnlyMemory<byte>`.

#### Scenario: C# caller constructs workspace from UTF-8 bytes
- **WHEN** a C# caller creates `NxWorkspaceModule("chat-link-config.nx", sourceBytes)`
- **AND** creates an `NxWorkspace` containing that module
- **THEN** the managed binding SHALL preserve the identity string and source byte payload for the
  native workspace call

#### Scenario: Managed API offers string convenience without replacing byte model
- **WHEN** a C# caller has source text as a string
- **THEN** the managed binding MAY offer a convenience factory or overload that encodes the source
  text as UTF-8
- **AND** the primary workspace module representation SHALL remain byte-oriented

### Requirement: Managed binding validates workspace arguments before FFI
The managed binding SHALL validate null workspaces, null modules, null build contexts, and empty
module or entry identities before invoking native workspace FFI.

#### Scenario: Empty module identity is rejected in managed code
- **WHEN** a C# caller creates or submits a workspace module with an empty identity
- **THEN** the managed binding SHALL throw a managed argument exception before invoking native code

#### Scenario: Null build context is rejected in managed code
- **WHEN** a C# caller invokes workspace validation with a null `NxProgramBuildContext`
- **THEN** the managed binding SHALL throw a managed argument exception before invoking native code

### Requirement: Managed workspace validation returns diagnostics
The managed binding SHALL expose a workspace validation API that returns an
`IReadOnlyList<NxDiagnostic>` translated through the existing managed diagnostic model.

#### Scenario: Valid managed workspace returns empty diagnostics list
- **WHEN** a C# caller validates a valid `NxWorkspace`
- **THEN** the managed validation API SHALL return an empty diagnostics list

#### Scenario: Invalid managed workspace returns structured diagnostics
- **WHEN** a C# caller validates a workspace containing type-invalid NX source
- **THEN** the managed validation API SHALL return `NxDiagnostic` values whose label files preserve
  the normalized workspace identities reported by native NX

### Requirement: Managed workspace builds pin buffers only for the native call
The managed binding SHALL pin workspace module descriptors, identity bytes, source bytes, and entry
identity bytes only for the duration of the native workspace call. A returned
`NxProgramArtifact` SHALL remain valid after those managed buffers are unpinned or collected.

#### Scenario: Workspace artifact remains executable after build buffers are released
- **WHEN** a C# caller builds an `NxProgramArtifact` from an `NxWorkspace`
- **AND** the managed workspace source buffers are no longer pinned after the build call returns
- **THEN** evaluating the returned artifact SHALL still succeed
