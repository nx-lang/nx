## MODIFIED Requirements

### Requirement: Runtime-provided standard function bodies are bounded and diagnostic
Rust and TypeScript IR execution SHALL evaluate recognized standard math bodies with shared arity/type/domain semantics and origin-aware diagnostics. Each body operation SHALL cost one operation in addition to existing argument and call/body costs, SHALL perform bounded numerical work, and SHALL respect host evaluation budgets.

#### Scenario: Evaluation budget applies
- **WHEN** a program repeatedly invokes imported math under an insufficient operation budget
- **THEN** evaluation SHALL stop with the normal budget diagnostic rather than run unmetered host callbacks

#### Scenario: Function-value error retains operation identity
- **WHEN** a passed sqrt function is invoked with -1.0
- **THEN** evaluation SHALL report nx-math-domain naming sqrt and the available call origin
