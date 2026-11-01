## MODIFIED Requirements

### Requirement: Type arguments leave no trace at runtime
A type-argument binding SHALL be removed from a record construction once type checking has
consumed it, so that the constructed value has no field for a type parameter and interpretation,
code generation and every other consumer below the type checker observe a construction with no
such binding. Two values of a generic record built with different type arguments and equal fields
SHALL be equal values, and a runtime that validates a host-supplied value against a field typed by
a type parameter SHALL treat that field as `object`.

#### Scenario: The constructed value has no type-parameter field
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `let r = <Range T=int start={1} end={5} />`
- **AND** `r` is evaluated
- **THEN** the resulting record SHALL have the fields `start` and `end` and SHALL NOT have a field `T`

#### Scenario: Evaluation agrees across backends
- **WHEN** a program constructs a generic record, reads one of its fields, and applies an update to it
- **THEN** the Rust IR runtime, the generated executable code and the TypeScript IR runtime SHALL produce the same value
