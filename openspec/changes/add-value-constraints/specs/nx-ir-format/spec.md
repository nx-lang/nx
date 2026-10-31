## ADDED Requirements

### Requirement: NX IR encodes a constrained type as its own type kind behind a required feature
NX IR SHALL encode a constrained type as a type entry `[7, base, low, high, flags]`, where `base` is
the type-table index of the unconstrained base type, `low` and `high` are constant-table indexes of
the bounds or `-1` for an open end, and `flags` bit 0 marks the low end
exclusive and bit 1 the high end. An entry SHALL describe the intersection of every term and alias
that contributed to the type, so no entry's base is itself a constrained entry. A module whose type
table holds such an entry or whose nodes hold a check node or an interval pattern SHALL list the required feature
`constraints-v1`, and a reader that does not support it SHALL refuse the module by naming the
feature. A bound SHALL be an integer constant for an integer base and a float constant for a floating-point
base; for `float32` it SHALL hold the bound already rounded to `float32`, so comparing a rounded value
against it is exact. An integer base's exclusive ends SHALL be written as the inclusive ends next to
them, in every kind-7 entry, including those an interval pattern refers to. Everywhere a rule reads a type, a constrained entry SHALL behave as its base, except where a
requirement on checks says otherwise. A constrained type reached through an alias declared in another
module, including one the build reads only as an interface, SHALL be written as a kind-7 entry, never
as a reference to the alias, so the constraint reaches the runtime.

#### Scenario: A constrained parameter's type
- **WHEN** a module declares `let fade(amount: float32 where in 0.0..=1.0) = { amount }`
- **THEN** the emitted artifact SHALL list `constraints-v1`, and `amount`'s type SHALL be a kind-7
  entry whose base is `float32`, whose bounds are the `float32` constants `0.0` and `1.0`, and whose
  flags are `0`

#### Scenario: An open end and an exclusive end
- **WHEN** a module declares `type Unit = float64 where > 0.0` and uses it as a parameter type
- **THEN** the entry SHALL have `high` equal to `-1` and `flags` equal to `1`

#### Scenario: A runtime without the feature
- **WHEN** a reader that does not support `constraints-v1` reads that artifact
- **THEN** it SHALL refuse the module with a diagnostic naming `constraints-v1`

### Requirement: NX IR encodes a constraint check as a node
NX IR SHALL encode a runtime constraint check as a node `[28, operand, type, site]`, where `type` is a
kind-7 type entry and `site` is a string-table index naming what is being bound, such as a parameter
or field name. Evaluating it SHALL evaluate `operand` and SHALL yield its value when the type's
interval admits it, and otherwise SHALL fail with the code `nx-ir-constraint` and a message naming
the site, the value and the interval. When the operand is a sequence the interval SHALL be checked
for each item, and the empty value SHALL pass, so one node serves an optional site and a `+` or `*`
site whose item type is constrained. A `NaN` operand SHALL fail. The comparison SHALL be made at the
base type. Each value checked, one for a single value and one per item for a sequence, SHALL count
as one operation in the cost model. The emitter SHALL write the node
exactly where `value-constraints` requires a runtime check and nowhere else.

#### Scenario: A check node at a call
- **WHEN** a module declares `let fade(amount: float64 where in 0.0..=1.0) = { amount }` and `let twice(x: float64) = { fade(x * 2.0) }`
- **THEN** the argument of the call in `twice` SHALL be a kind-28 node whose operand is the
  multiplication and whose site names `amount`

#### Scenario: A check node on a sequence
- **WHEN** a module declares `type Score = int where >= 0` and `let scores(xs: int+): Score+ = { for x in xs { x - 1 } }`
- **THEN** the body SHALL be a kind-28 node over the `for`, and evaluating it on `{ 1 0 }` SHALL fail
  at the second item

#### Scenario: The corpus covers checks
- **WHEN** the conformance corpus runs
- **THEN** it SHALL include programs whose checks pass, fail at an inclusive and an exclusive end,
  fail on `NaN`, and record one operation per value checked, item by item for a sequence

### Requirement: NX IR encodes a numeric match pattern as an interval node
NX IR SHALL encode a numeric value pattern in an `ifIs` arm's pattern list as a node `[29, type]`,
where `type` is a kind-7 type entry whose base is the scrutinee's base type and whose interval is the
pattern's. An arm SHALL match when one of its patterns matches; an interval node SHALL match when the
scrutinee lies in the interval, compared at the base type, and SHALL NOT match `NaN`. An interval node
SHALL appear only in a pattern list, and a reader SHALL refuse one anywhere else as malformed. It
SHALL cost what a literal pattern costs.

#### Scenario: Two alternatives in one arm
- **WHEN** a module declares `let flag(n: int) = { if n is { < 0, > 100 => "out" else => "in" } }`
- **THEN** the arm's pattern list SHALL hold two kind-29 nodes, one whose type has only a high end
  `-1`, inclusive, and one whose type has only a low end `101`, inclusive
