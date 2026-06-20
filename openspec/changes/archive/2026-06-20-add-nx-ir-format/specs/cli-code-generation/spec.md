## ADDED Requirements

### Requirement: CLI codegen writes NX IR JSON artifacts
The `nxlang codegen` command SHALL support an explicit `nx-ir` executable target that builds a
`ProgramArtifact` through the existing source/workspace analysis pipeline and writes one
deterministic NX IR JSON artifact. NX IR output SHALL be host-neutral and SHALL NOT require a
JavaScript or TypeScript source-generation target.

#### Scenario: Source file codegen writes NX IR
- **WHEN** a user runs `nxlang codegen ./app/main.nx --target nx-ir --output ./generated`
- **THEN** the CLI SHALL build a `ProgramArtifact` for `app/main.nx`
- **AND** it SHALL write one NX IR JSON artifact under `./generated`
- **AND** it SHALL NOT write JavaScript runtime helper files or generated JavaScript modules for
  that request

#### Scenario: Workspace codegen writes NX IR for selected entry
- **WHEN** a user runs `nxlang codegen ./workspace --target nx-ir --entry app/main.nx --output ./generated`
- **THEN** the CLI SHALL build a workspace `ProgramArtifact` using `app/main.nx` as the selected
  entry identity
- **AND** it SHALL write one NX IR JSON artifact for that selected program

#### Scenario: Static diagnostics prevent IR output
- **WHEN** a user requests `--target nx-ir` for a source file that has static analysis errors
- **THEN** the CLI SHALL print diagnostics through the existing diagnostic rendering path
- **AND** it SHALL NOT write an NX IR artifact

#### Scenario: NX IR target rejects source output formats
- **WHEN** a user requests `--target nx-ir --format program-module`
- **THEN** the CLI SHALL report that NX IR codegen does not use source output formats
- **AND** it SHALL NOT write an NX IR artifact
