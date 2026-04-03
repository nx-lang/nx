## ADDED Requirements

### Requirement: Interpreter execution is defined by ResolvedProgram
The interpreter SHALL execute NX code through a `ResolvedProgram` rather than through one merged
lowered module. `ResolvedProgram` SHALL provide the executable view of the program across multiple
`LoweredModule`s, including entrypoint lookup and resolved symbol tables for runtime-visible items.

#### Scenario: Root evaluation crosses module boundaries through resolved program state
- **WHEN** a root source file imports a library function `answer()` from another file and `root()`
  returns `answer()`
- **THEN** the interpreter SHALL resolve `answer()` through `ResolvedProgram`
- **AND** the interpreter SHALL execute the call without requiring the source file and imported file
  to be merged into one `LoweredModule`

#### Scenario: Component initialization resolves component definitions through ResolvedProgram
- **WHEN** a host initializes a component whose definition is exported from an imported library file
- **THEN** the runtime SHALL locate that component through `ResolvedProgram`
- **AND** the runtime SHALL execute the component using the owning `LoweredModule` for that
  definition

### Requirement: Runtime references are module-qualified
Any runtime-owned reference to executable NX code SHALL identify both the owning module and the
local item, expression, or element within that module. Bare local IDs SHALL NOT be used as the
complete identity of a runtime reference once execution can cross module boundaries.

#### Scenario: Function references identify the owning lowered module
- **WHEN** runtime resolution records a callable reference for a function exported from
  `forms/input.nx`
- **THEN** that callable reference SHALL include the identity of `forms/input.nx`
- **AND** that callable reference SHALL include the local function item reference within that
  module

#### Scenario: Captured handler references identify the owning lowered module
- **WHEN** a component action handler captures an expression from `search-box.nx`
- **THEN** the runtime-owned handler reference SHALL include the identity of `search-box.nx`
- **AND** the runtime-owned handler reference SHALL include the local expression reference within
  that module

### Requirement: Serialized runtime state is program-specific and module-aware
Serialized component snapshots and any serialized handler payloads SHALL encode the
module-qualified runtime references needed by `ResolvedProgram`. Runtime entry points SHALL reject
serialized state that does not match the target program revision or that lacks the required module
identity.

#### Scenario: Snapshot from the same program revision can be reused
- **WHEN** a host stores a component snapshot produced from one `ProgramArtifact` revision and later
  dispatches actions against that same `ProgramArtifact` revision
- **THEN** dispatch SHALL accept the snapshot
- **AND** the runtime SHALL resolve the snapshot's code references through module-qualified runtime
  references

#### Scenario: Snapshot from a different program revision is rejected
- **WHEN** a host dispatches actions using a snapshot produced by a different `ProgramArtifact`
  revision
- **THEN** dispatch SHALL reject the snapshot as incompatible
- **AND** the runtime SHALL NOT silently reinterpret bare local IDs against the new program
