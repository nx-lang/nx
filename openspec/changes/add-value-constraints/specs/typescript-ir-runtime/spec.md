## ADDED Requirements

### Requirement: TypeScript runtime enforces constraints in programs and at the host boundary
The TypeScript IR runtime SHALL support the `constraints-v1` feature. It SHALL evaluate check nodes
as `nx-ir-format` defines them. When a host passes a value, as an argument, a component prop, or a
field nested anywhere inside either, whose declared type is a constrained type and whose value the
interval does not admit, the runtime SHALL refuse the call with the code
`nx-ir-boundary-constraint` and a message naming the path to the value and the interval, and SHALL
name the argument the value was passed for as it does for every boundary failure. A default the
runtime computes that misses its own field's constraint SHALL fail with `nx-ir-constraint` and SHALL
name no argument. A `float32` value SHALL be compared after it is rounded to `float32`, as the
runtime already rounds one, so the result agrees with the Rust runtime.

#### Scenario: A host argument out of range
- **WHEN** a host calls `let pick(count: int where value in 1..=10): int = { count }` with `count` bound to `11`
- **THEN** the call SHALL fail with `nx-ir-boundary-constraint`, and the diagnostic SHALL name the
  argument `count` and the interval `1..=10`

#### Scenario: A boundary failure reaches an agent as input to correct
- **WHEN** `@nx-lang/agent` runs a tool whose function takes `count: int where value in 1..=10` and the model sends `11`
- **THEN** the tool SHALL fail with `invalid-input`, because the code begins `nx-ir-boundary-` and
  names the argument

#### Scenario: A float32 bound
- **WHEN** a host calls `let f(x: float32 where value <= 0.1) = { x }` with `x` bound to `0.1`
- **THEN** the call SHALL succeed in both runtimes
