## MODIFIED Requirements

### Requirement: Program artifacts expose source text for codegen
The public `nx-program` surface SHALL expose read-only source text access for `ProgramArtifact`
inputs so `nx-codegen` can produce source maps and source-aware diagnostics without re-reading files
or depending on artifact-internal storage details.

#### Scenario: Source text lookup by identity
- **WHEN** a `ProgramArtifact` preserves source text for a source-provider module
- **THEN** public `nx-program` accessors SHALL allow `nx-codegen` to retrieve that source text by
  the module's normalized identity

#### Scenario: Source entries are iterable
- **WHEN** `nx-codegen` builds source maps for a generated program
- **THEN** public `nx-program` accessors SHALL allow it to iterate the preserved source entries
  needed for source-map construction
