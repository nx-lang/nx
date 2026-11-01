## MODIFIED Requirements

### Requirement: Source-driven runtime execution stops on static analysis errors
CLI source execution SHALL complete shared static analysis before IR emission or runtime execution.
Any error diagnostic SHALL prevent emission and execution and SHALL be reported with the existing
source identity and spans. Tooling construction SHALL remain independent of execution.

#### Scenario: Root evaluation is gated by static analysis
- **WHEN** the CLI runs source with a lowering or type error and a root function
- **THEN** it SHALL report the static diagnostics
- **AND** SHALL NOT emit executable IR or execute root
