## ADDED Requirements

### Requirement: Managed binding emits NX IR from program artifacts
The managed .NET binding SHALL expose artifact-first APIs that emit NX IR JSON and structured
metadata from an `NxProgramArtifact`. The managed API SHALL reuse the native artifact-first IR
emission path, SHALL preserve the program fingerprint and runtime ABI metadata, and SHALL surface
IR emission diagnostics through the existing managed diagnostic exception path.

#### Scenario: Managed artifact emits IR
- **WHEN** a .NET caller builds a valid `NxProgramArtifact`
- **AND** the caller requests NX IR output from that artifact
- **THEN** the managed API SHALL return NX IR JSON
- **AND** it SHALL return metadata identifying the program fingerprint, IR schema version, runtime
  ABI, and exported function/component entrypoints

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
