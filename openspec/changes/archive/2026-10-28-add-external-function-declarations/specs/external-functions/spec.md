## Purpose

Defines documented runtime-provided function signatures in NX source while requiring trusted implementations before execution.

## ADDED Requirements

### Requirement: External functions have explicit bodyless declarations
NX SHALL accept `[visibility] external name(parameters):Result` and its existing element-signature equivalent. An external declaration SHALL specify a result type and SHALL NOT contain an authored body. An ordinary function SHALL still require its body. Ordinary declaration documentation and parameter documentation SHALL attach to external functions.

#### Scenario: Documented runtime signature
- **WHEN** NX parses a `///` comment followed by `export external sin(x:float64):float64`
- **THEN** it SHALL preserve the signature, documentation and source locations without requiring a placeholder body

#### Scenario: Invalid external syntax
- **WHEN** an external function omits its return type or supplies a body
- **THEN** the syntax SHALL be rejected

#### Scenario: Incorrect let keyword is rejected locally
- **WHEN** a declaration uses `external let` before its function signature
- **THEN** syntax validation SHALL report that external functions are declared without `let`
- **AND** the diagnostic SHALL identify the incorrect `let` keyword and explicitly suggest removing it while subsequent declarations remain available
- **AND** this SHALL be a syntax error, with no deprecation warning or compatibility mode

### Requirement: External functions require a matching trusted implementation
Only compiler-owned module registry entries SHALL bind external declarations to runtime implementations. Binding SHALL validate the full declared signature and require exactly one matching external declaration for each registered implementation. An unregistered external function in user or host-library source SHALL fail analysis with a diagnostic instead of generating executable fallback code. An ordinary authored function SHALL NOT be rebound by matching a registered operation's spelling.

#### Scenario: Unregistered declaration
- **WHEN** ordinary source declares `external sin(x:float64):float64`
- **THEN** analysis SHALL report that no runtime implementation is registered

#### Scenario: Registry mismatch
- **WHEN** a trusted module declares a registered function with wrong names, parameter order/types/flags, result type or calling form, or omits or duplicates it
- **THEN** library analysis SHALL fail with a source-oriented diagnostic

### Requirement: Math documentation is carried by NX declarations
All public math functions and constants SHALL have source-authored NX documentation. Hover and declaration completions for imported math SHALL display that documentation through the existing documentation pipeline, including aliased/selective imports. Implementation body origins SHALL remain synthetic while declaration locations SHALL point to NX source.

#### Scenario: Imported math documentation
- **WHEN** an editor requests hover or completion documentation for imported sin, atan2, round, pi or tau
- **THEN** it SHALL receive the declaration's NX documentation, including units, argument order or rounding behavior where relevant

#### Scenario: Numerical diagnostics retain caller location
- **WHEN** a program invokes imported sqrt with negative input
- **THEN** it SHALL retain its math domain diagnostic and available authored call origin
