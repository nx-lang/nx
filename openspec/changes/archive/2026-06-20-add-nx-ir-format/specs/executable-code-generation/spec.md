## ADDED Requirements

### Requirement: Program artifacts emit NX IR as a public executable artifact
The executable generation API SHALL expose an artifact-first operation that emits a versioned NX IR
JSON program artifact from a successful `ProgramArtifact`. This operation SHALL be separate from
existing JavaScript and TypeScript source emission and SHALL preserve existing source generation
behavior unless a caller explicitly requests IR output.

#### Scenario: Valid artifact emits NX IR
- **WHEN** a caller builds a valid `ProgramArtifact` for an NX source file with `root()`
- **AND** the caller requests NX IR output
- **THEN** executable generation SHALL return NX IR JSON and structured metadata identifying the
  program fingerprint, IR schema version, runtime ABI, and exported entrypoints

#### Scenario: Existing JavaScript output is unchanged
- **WHEN** a caller requests existing JavaScript executable file output for a valid NX program
- **THEN** the system SHALL emit the existing JavaScript file layout
- **AND** it SHALL NOT emit NX IR unless the caller explicitly requests IR output

#### Scenario: Unsupported IR construct reports diagnostics
- **WHEN** a valid `ProgramArtifact` contains a construct outside the supported NX IR feature set
- **AND** a caller requests NX IR output
- **THEN** executable generation SHALL fail with diagnostics that identify the unsupported
  construct
- **AND** it SHALL NOT emit silently incomplete IR

### Requirement: NX IR emission is validated against interpreter semantics
Executable generation tests SHALL verify that supported NX IR output can be executed by the
TypeScript IR runtime with results equivalent to native interpreter evaluation. These tests SHALL
cover functions, component descriptor construction, component initialization, explicit state
component evaluation, and host-owned state update validation.

#### Scenario: IR emission parity covers functions
- **WHEN** executable generation emits NX IR for a supported function program
- **THEN** tests SHALL execute the emitted IR through the TypeScript runtime
- **AND** they SHALL compare the result with native interpreter evaluation

#### Scenario: IR emission parity covers components
- **WHEN** executable generation emits NX IR for a supported component program
- **THEN** tests SHALL execute descriptor construction, initialization, explicit state evaluation,
  and state patch validation through the TypeScript runtime
- **AND** they SHALL compare rendered values with native interpreter behavior where applicable
