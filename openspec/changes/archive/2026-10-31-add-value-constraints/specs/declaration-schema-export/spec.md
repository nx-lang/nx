## ADDED Requirements

### Requirement: Constraints map to JSON Schema bound keywords
The schema of a constrained type SHALL be its base type's schema with the interval's ends added: an
inclusive low end as `minimum`, an exclusive one as `exclusiveMinimum`, an inclusive high end as
`maximum` and an exclusive one as `exclusiveMaximum`, each the bound's value as a JSON number; an
open end SHALL add nothing. On an integer base an exclusive end SHALL be written as the inclusive end
next to it, `minimum` or `maximum`. On a `float32` base every bound SHALL be written as an inclusive
`minimum` or `maximum` holding a `float32` value: an inclusive end as itself, and an exclusive end as
the nearest `float32` inside the interval, so that no number the schema admits rounds, as a runtime
rounds a `float32` it is given, to a value outside the interval. A bound SHALL replace the base's own `minimum` or `maximum`, as
`int32` has, on the same side, since it always lies within it. A constrained alias SHALL map as the type it denotes, constraint
included. Each parameter entry's NX spelling SHALL include the constraint as written. Because the
runtimes refuse a value outside the interval, the requirement that a schema is never wider than the
runtimes' boundary SHALL hold for constrained types, and the corpus that checks it SHALL include
them; in that corpus no schema-valid argument SHALL fail with `nx-ir-boundary-constraint`.

#### Scenario: A closed range
- **WHEN** a host asks for the input schema of `let pick(count: int where in 1..=10): int = { count }`
- **THEN** the `count` property SHALL be `{ "type": "integer", "minimum": 1, "maximum": 10 }`, and
  the parameter entry's spelling SHALL be `int where in 1..=10`

#### Scenario: Exclusive and open ends
- **WHEN** a host asks for the schema of `type Unit = float64 where > 0.0 && < 1.0`
- **THEN** it SHALL be `{ "type": "number", "exclusiveMinimum": 0, "exclusiveMaximum": 1 }`
- **AND** the schema of `type Count = int where >= 1` SHALL carry `minimum` and no maximum
- **AND** the schema of `type Pos = int where > 0` SHALL be `{ "type": "integer", "minimum": 1 }`

#### Scenario: A float32 exclusive end
- **WHEN** a host asks for the schema of `type Below = float32 where < 1.0`
- **THEN** it SHALL be `{ "type": "number", "maximum": 0.9999999403953552 }`, the largest `float32`
  below `1.0`
