# runtime-standard-functions Specification

## Purpose

Enables a closed set of pure compiler-owned standard functions to have runtime implementations while preserving ordinary NX declarations, imports, function values and execution limits.

## Requirements

### Requirement: Runtime-provided standard functions preserve ordinary declaration identity
The system SHALL support compiler-owned runtime-provided standard function declarations with ordinary typed signatures, import/alias behavior, schema descriptions and first-class function references. Only the trusted standard-library registry SHALL assign implementations. User declarations with identical names/signatures SHALL NOT acquire that implementation. Public signatures and documentation SHALL be authored as NX external function declarations, with implementations assigned only by trusted module registry entries. No reserved math names or arbitrary host callback registration SHALL be introduced.

#### Scenario: Function reference invokes math
- **WHEN** an IR program binds the imported sin function to a function value, passes or returns it, and invokes that value
- **THEN** it SHALL invoke the same standard declaration and behavior as a direct sin call

#### Scenario: Spoofed standard function
- **WHEN** a user declares a function named sin with the same signature
- **THEN** its authored body SHALL execute without being replaced by the standard implementation

### Requirement: Runtime-provided standard function bodies are bounded and diagnostic
Rust and TypeScript IR execution SHALL evaluate recognized standard math bodies with shared arity/type/domain semantics and origin-aware diagnostics. Each body operation SHALL cost one operation in addition to existing argument and call/body costs, SHALL perform bounded numerical work, and SHALL respect host evaluation budgets. The supported legacy interpreter SHALL retain parity until retired through its own change.

#### Scenario: Evaluation budget applies
- **WHEN** a program repeatedly invokes imported math under an insufficient operation budget
- **THEN** evaluation SHALL stop with the normal budget diagnostic rather than run unmetered host callbacks

#### Scenario: Function-value error retains operation identity
- **WHEN** a passed sqrt function is invoked with -1.0
- **THEN** evaluation SHALL report nx-math-domain naming sqrt and the available call origin
