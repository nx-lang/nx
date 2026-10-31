## ADDED Requirements

### Requirement: NX IR encodes a constrained type as its own type kind behind a required feature
NX IR SHALL encode a constrained type as a type entry `[7, base, low, high, flags]`, where `base` is
the type-table index of the unconstrained base type, `low` and `high` are constant-table indexes of
the bounds written at the base type or `-1` for an open end, and `flags` bit 0 marks the low end
exclusive and bit 1 the high end. An entry SHALL describe the intersection of every term and alias
that contributed to the type, so no entry's base is itself a constrained entry. A module whose type
table holds such an entry or whose nodes hold a check node SHALL list the required feature
`constraints-v1`, and a reader that does not support it SHALL refuse the module by naming the
feature. Everywhere a rule reads a type, a constrained entry SHALL behave as its base, except where
a requirement on checks says otherwise.

#### Scenario: A constrained parameter's type
- **WHEN** a module declares `let fade(amount: float32 where value in 0.0..=1.0) = { amount }`
- **THEN** the emitted artifact SHALL list `constraints-v1`, and `amount`'s type SHALL be a kind-7
  entry whose base is `float32`, whose bounds are the `float32` constants `0.0` and `1.0`, and whose
  flags are `0`

#### Scenario: An open end and an exclusive end
- **WHEN** a module declares `type Unit = float64 where value > 0.0` and uses it as a parameter type
- **THEN** the entry SHALL have `high` equal to `-1` and `flags` equal to `1`

#### Scenario: A runtime without the feature
- **WHEN** a reader that does not support `constraints-v1` reads that artifact
- **THEN** it SHALL refuse the module with a diagnostic naming `constraints-v1`

### Requirement: NX IR encodes a constraint check as a node
NX IR SHALL encode a runtime constraint check as a node `[28, operand, type, site]`, where `type` is a
kind-7 type entry and `site` is a string-table index naming what is being bound, such as a parameter
or field name. Evaluating it SHALL evaluate `operand` and SHALL yield its value when the type's
interval admits it, and otherwise SHALL fail with the code `nx-ir-constraint` and a message naming
the site, the value and the interval. A `NaN` operand SHALL fail. The comparison SHALL be made at the
base type. The node SHALL count as one operation in the cost model. The emitter SHALL write the node
exactly where `value-constraints` requires a runtime check and nowhere else.

#### Scenario: A check node at a call
- **WHEN** a module declares `let fade(amount: float64 where value in 0.0..=1.0) = { amount }` and `let twice(x: float64) = { fade(x * 2.0) }`
- **THEN** the argument of the call in `twice` SHALL be a kind-28 node whose operand is the
  multiplication and whose site names `amount`

#### Scenario: The corpus covers checks
- **WHEN** the conformance corpus runs
- **THEN** it SHALL include programs whose checks pass, fail at an inclusive and an exclusive end,
  fail on `NaN`, and record each check as one operation
