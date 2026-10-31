## ADDED Requirements

### Requirement: Constraints map to JSON Schema bound keywords
The schema of a constrained type SHALL be its base type's schema with the interval's ends added: an
inclusive low end as `minimum`, an exclusive one as `exclusiveMinimum`, an inclusive high end as
`maximum` and an exclusive one as `exclusiveMaximum`, each the bound's value as a JSON number; an
open end SHALL add nothing. A bound SHALL replace the base's own `minimum` or `maximum`, as
`int32` has, on the same side, since it always lies within it. A constrained alias SHALL map as the type it denotes, constraint
included. Each parameter entry's NX spelling SHALL include the constraint as written. Because the
runtimes refuse a value outside the interval, the requirement that a schema is never wider than the
runtimes' boundary SHALL hold for constrained types, and the corpus that checks it SHALL include
them.

#### Scenario: A closed range
- **WHEN** a host asks for the input schema of `let pick(count: int where in 1..=10): int = { count }`
- **THEN** the `count` property SHALL be `{ "type": "integer", "minimum": 1, "maximum": 10 }`, and
  the parameter entry's spelling SHALL be `int where in 1..=10`

#### Scenario: Exclusive and open ends
- **WHEN** a host asks for the schema of `type Unit = float64 where > 0.0 && < 1.0`
- **THEN** it SHALL be `{ "type": "number", "exclusiveMinimum": 0, "exclusiveMaximum": 1 }`
- **AND** the schema of `type Count = int where >= 1` SHALL carry `minimum` and no maximum
