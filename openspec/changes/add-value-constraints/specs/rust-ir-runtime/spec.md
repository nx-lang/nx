## ADDED Requirements

### Requirement: Rust runtime enforces constraints in programs and at the host boundary
The Rust IR runtime SHALL support the `constraints-v1` feature. It SHALL evaluate check nodes and match
interval patterns as `nx-ir-format` defines them. When a host passes a value, as an argument, a component prop, or a
field nested anywhere inside either, whose declared type is a constrained type and whose value the
interval does not admit, the runtime SHALL refuse the call with the code
`nx-ir-boundary-constraint` and a message naming the path to the value and the interval, and SHALL
name the argument the value was passed for as it does for every boundary failure. A default the
runtime computes that misses its own field's constraint SHALL fail with `nx-ir-constraint` and SHALL
name no argument. For the same image and the same input the Rust and TypeScript runtimes SHALL
produce the same result or the same diagnostic code and argument.

#### Scenario: A host argument out of range
- **WHEN** a host calls `let pick(count: int where in 1..=10): int = { count }` with `count` bound to `11`
- **THEN** the call SHALL fail with `nx-ir-boundary-constraint`, and the diagnostic SHALL name the
  argument `count` and the interval `1..=10`

#### Scenario: A nested field out of range
- **WHEN** a host calls `let paint(p: Paint) = { p.opacity }`, where `type Paint = { opacity: float64 where in 0.0..=1.0 }`, with `p` bound to a record whose `opacity` is `1.5`
- **THEN** the call SHALL fail with `nx-ir-boundary-constraint`, naming the argument `p` and the
  path to `opacity`

#### Scenario: A check inside the program
- **WHEN** a host calls `twice(0.75)` where `let twice(x: float64) = { fade(x * 2.0) }` and `fade` takes `amount: float64 where in 0.0..=1.0`
- **THEN** the call SHALL fail with `nx-ir-constraint` and SHALL name no argument
