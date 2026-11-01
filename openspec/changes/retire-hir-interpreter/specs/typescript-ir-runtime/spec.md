## ADDED Requirements

### Requirement: TypeScript IR runtime behavior is validated against the native runtime
The implementation SHALL include automated tests that emit NX IR from source or program artifacts,
execute the IR through the TypeScript runtime, and compare results against native evaluation of
the same source, which the Rust IR runtime performs, for the supported non-reactive subset.
Component tests SHALL cover descriptor construction, initialization, explicit state evaluation,
state patch validation, conditional content based on state, lone-child content binding at each
occurrence, and dispatch.

#### Scenario: Function parity test compares the two IR runtimes
- **WHEN** a supported NX program uses primitives, arithmetic, conditionals, match expressions,
  sequences, optional values and the presence operators, loops, records, unions, enums, member
  access, and function calls
- **THEN** automated tests SHALL verify that TypeScript IR runtime output matches native output for
  the same program

#### Scenario: Component parity test compares the two IR runtimes
- **WHEN** a supported NX component uses props, state defaults, conditional content, child
  component descriptors, and explicit state evaluation
- **THEN** automated tests SHALL verify that TypeScript IR runtime rendered output matches native
  rendered output for the same props and state

#### Scenario: Dispatch parity test compares the two IR runtimes
- **WHEN** a component binds handlers that patch its state, emit to its parent, and return effects
- **AND** the same batches of handler invocations and emitted actions are dispatched through both
  runtimes
- **THEN** automated tests SHALL verify that the rendered output, including tokens, and the
  ordered effects match between the TypeScript runtime and the native runtime

## REMOVED Requirements

### Requirement: TypeScript IR runtime behavior is validated against existing NX semantics
**Reason**: The requirement defines "existing NX semantics" as the interpreter's, and the
interpreter is deleted.
**Migration**: Replaced by "TypeScript IR runtime behavior is validated against the native
runtime", with the same coverage.
