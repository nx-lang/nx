## MODIFIED Requirements

### Requirement: Runtime-provided standard functions preserve ordinary declaration identity
The system SHALL support compiler-owned runtime-provided standard function declarations with ordinary typed signatures, import/alias behavior, schema descriptions and first-class function references. Only the trusted standard-library registry SHALL assign implementations. User declarations with identical names/signatures SHALL NOT acquire that implementation. Public signatures and documentation SHALL be authored as NX external function declarations, with implementations assigned only by trusted module registry entries. No reserved math names or arbitrary host callback registration SHALL be introduced.

#### Scenario: Function reference invokes math
- **WHEN** an IR program binds the imported sin function to a function value, passes or returns it, and invokes that value
- **THEN** it SHALL invoke the same standard declaration and behavior as a direct sin call

#### Scenario: Spoofed standard function
- **WHEN** a user declares a function named sin with the same signature
- **THEN** its authored body SHALL execute without being replaced by the standard implementation
