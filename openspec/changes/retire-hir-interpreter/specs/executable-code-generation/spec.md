## ADDED Requirements

### Requirement: Generated behavior is validated against the native runtime
The executable code generation implementation SHALL include tests that compare generated
TypeScript/JavaScript behavior with native evaluation of the same program for supported
non-reactive programs. The parity tests SHALL cover primitives, arithmetic, conditionals, arrays,
loops, function calls, records, discriminated unions, enum values, element expressions that
serialize to `NxValue::Record`, and cross-module imports.

#### Scenario: Native parity for supported program
- **WHEN** a supported NX program is evaluated natively
- **AND** the same program is emitted and executed as generated JavaScript or TypeScript
- **THEN** both executions SHALL produce equivalent `NxValue` payloads
- **AND** object property order SHALL NOT be treated as semantic during the comparison

#### Scenario: Unsupported executable construct reports diagnostic
- **WHEN** codegen encounters an NX construct outside the supported initial executable generation
  subset
- **THEN** codegen SHALL return a diagnostic that identifies the unsupported construct
- **AND** codegen SHALL NOT emit silently incomplete executable output

### Requirement: NX IR emission is validated against source-level expectations
Executable generation tests SHALL verify that NX IR emitted from source evaluates, on the Rust IR
runtime and on the TypeScript IR runtime, to a value the test states. These tests SHALL cover
functions, component descriptor construction, component initialization, explicit state component
evaluation, and host-owned state update validation. Because both runtimes evaluate the same emitted
IR, agreement between them SHALL NOT be the only check of what the emitter produced.

#### Scenario: IR emission is checked for functions
- **WHEN** executable generation emits NX IR for a supported function program
- **THEN** tests SHALL execute the emitted IR through both IR runtimes
- **AND** SHALL compare each result with the expected value the test states

#### Scenario: IR emission is checked for components
- **WHEN** executable generation emits NX IR for a supported component program
- **THEN** tests SHALL execute descriptor construction, initialization, explicit state evaluation,
  and state patch validation through both IR runtimes
- **AND** SHALL compare rendered values with the expected values the test states

## REMOVED Requirements

### Requirement: Generated behavior is validated against the interpreter
**Reason**: The interpreter is deleted.
**Migration**: Replaced by "Generated behavior is validated against the native runtime", with the
same coverage and the Rust IR runtime as the native side.

### Requirement: NX IR emission is validated against interpreter semantics
**Reason**: The interpreter is deleted, and it was the one engine that did not consume emitted IR.
**Migration**: Replaced by "NX IR emission is validated against source-level expectations".
