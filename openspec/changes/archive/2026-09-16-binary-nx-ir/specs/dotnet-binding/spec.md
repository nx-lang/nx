## ADDED Requirements

### Requirement: Managed binding explains an NX IR artifact
The managed .NET binding SHALL render an NX IR image it is given as the same readable text the
CLI's `ir explain` prints for that image, and SHALL surface a malformed image through the existing
managed diagnostic exception path.

#### Scenario: A .NET host reads an artifact it holds
- **WHEN** a .NET caller passes an artifact's bytes to the managed explain API
- **THEN** it SHALL receive the text the CLI would print for that artifact

## MODIFIED Requirements

### Requirement: Managed binding emits NX IR from program artifacts
The managed .NET binding SHALL expose artifact-first APIs that emit NX IR images and structured
metadata from an `NxProgramArtifact`. The managed API SHALL reuse the native artifact-first IR
emission path, SHALL preserve the module fingerprint and runtime ABI metadata, and SHALL surface
IR emission diagnostics through the existing managed diagnostic exception path.

#### Scenario: Managed artifact emits IR
- **WHEN** a .NET caller builds a valid `NxProgramArtifact`
- **AND** the caller requests NX IR output from that artifact
- **THEN** the managed API SHALL return each artifact's bytes
- **AND** it SHALL return metadata identifying the module fingerprint, IR schema version, runtime
  ABI, and exported function/component entrypoints
- **AND** the bytes SHALL be identical to what the Node SDK emits for the same modules and options

#### Scenario: Managed artifact IR emission surfaces diagnostics
- **WHEN** a .NET caller requests NX IR output for an artifact that cannot be represented by the
  supported IR feature set
- **THEN** the managed API SHALL throw an `NxEvaluationException`
- **AND** the exception SHALL contain the native IR emission diagnostics

#### Scenario: Managed source convenience uses transient artifact
- **WHEN** a .NET caller requests NX IR output from source text using a build context convenience
  API
- **THEN** the managed binding SHALL build a transient `NxProgramArtifact` with that context
- **AND** it SHALL emit IR through the same artifact-first path as direct artifact calls
