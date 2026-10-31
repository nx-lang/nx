## ADDED Requirements

### Requirement: A type may be followed by a `where` clause about `value`
A type reference SHALL accept an optional clause `where` followed by a condition, after the base
type and any occurrence suffix, wherever a type is written: a record, action or union-case field, a
component prop or state field, a function parameter or result, a `let` annotation and an alias.
Inside the condition the name `value` SHALL denote the value being constrained. `where` and `value`
SHALL be keywords only inside the clause, so a declaration, field or parameter named `where` or
`value` elsewhere SHALL be unaffected. A default SHALL follow the whole type, constraint included.
The clause SHALL end at the first token that cannot continue the condition, so a whitespace-separated
field list needs no braces or separators around it.

#### Scenario: A constrained field and the field after it
- **WHEN** a file contains `type Paint = { opacity: float64 where value in 0.0..=1.0 = 1.0  layer: int }`
- **THEN** parsing SHALL produce two fields, `opacity` with a constraint and the default `1.0`, and
  `layer` with none

#### Scenario: A named constrained type
- **WHEN** a file contains `type Ratio = float32 where value in 0.0..=1.0` and
  `type Fade = { amount: Ratio }`
- **THEN** analysis SHALL accept both, with `amount` constrained to `0.0..=1.0`

#### Scenario: Parameters, results and annotations
- **WHEN** a file contains `let fade(amount: float32 where value in 0.0..=1.0, steps: int where value >= 1): int where value >= 0 = { steps }`
  and `let half: float64 where value <= 1.0 = 0.5`
- **THEN** analysis SHALL accept both declarations

#### Scenario: `value` stays a name elsewhere
- **WHEN** a file contains `type Slider = { value: float64 where value in 0.0..=1.0 }` and
  `let value = 3`
- **THEN** analysis SHALL accept both, with the field and the binding named `value`

### Requirement: A condition is one or more numeric bounds joined by `&&`
A condition SHALL be one or more terms joined by `&&`, each term `value in a..b`, `value in a..=b`,
`value >= a`, `value > a`, `value <= a` or `value < a`, where each bound is a numeric literal,
optionally preceded by `-`. `a..b` SHALL admit values from `a` up to but excluding `b`, and `a..=b`
values from `a` through `b`. The terms SHALL intersect, so the condition admits exactly one interval
whose ends are each inclusive, exclusive or open. Any other condition, including `||`, `!=`, `!`, a
call, a member access and a name other than `value`, SHALL be rejected with `invalid-constraint`,
whose message lists the supported terms; it SHALL NOT be a parse error.

#### Scenario: Terms intersect
- **WHEN** a file contains `type Unit = float64 where value >= 0.0 && value < 1.0`
- **THEN** `Unit` SHALL admit `0.0` and `0.5` and SHALL NOT admit `1.0`

#### Scenario: A negative bound
- **WHEN** a file contains `type Pan = float32 where value in -1.0..=1.0`
- **THEN** analysis SHALL accept the declaration

#### Scenario: An unsupported condition is named, not a parse error
- **WHEN** a file contains `type Odd = int where value % 2 == 1` or `type Edge = int where value < 0 || value > 9`
- **THEN** parsing SHALL succeed and analysis SHALL report `invalid-constraint` on the condition,
  listing `in`, `<`, `<=`, `>`, `>=` and `&&` as what a condition may use

### Requirement: Only numeric types without an occurrence take a constraint
The base of a constrained type SHALL be `int`, `int32`, `int64`, `float32` or `float64`, or an alias
of one, constrained or not. Each bound SHALL be checked as a literal written at the base type, so a
real bound on an integer base and a bound outside the base's range SHALL be rejected with
`invalid-constraint`. A constraint on a type with an occurrence suffix, on a type reached through an
alias that carries one, on a non-numeric type, on a type argument and inside a function type SHALL
be rejected with `invalid-constraint`; for an occurrence the diagnostic SHALL show naming the item
type as the fix. A condition that admits no value of the base type SHALL be rejected with
`empty-constraint`.

#### Scenario: A real bound on an integer
- **WHEN** a file contains `type Half = int where value <= 0.5`
- **THEN** analysis SHALL report `invalid-constraint` on `0.5`

#### Scenario: A bound outside the base's range
- **WHEN** a file contains `type Big = int32 where value <= 3000000000`
- **THEN** analysis SHALL report `invalid-constraint` saying the bound is out of range for `int32`

#### Scenario: A constraint on a sequence
- **WHEN** a file contains `type Scores = int+ where value >= 0`
- **THEN** analysis SHALL report `invalid-constraint`, and the message SHALL suggest
  `type Score = int where value >= 0` with `Score+`

#### Scenario: A constraint on a string
- **WHEN** a file contains `type Name = string where value >= 1`
- **THEN** analysis SHALL report `invalid-constraint` saying constraints apply to numeric types

#### Scenario: A constrained type argument
- **WHEN** a file contains `type Ratio = float32 where value in 0.0..=1.0` and `type Band = { r: <Range T=Ratio/> }`
- **THEN** analysis SHALL report `invalid-constraint` on the type argument

#### Scenario: An empty constraint
- **WHEN** a file contains `type None = int where value > 5 && value < 6`
- **THEN** analysis SHALL report `empty-constraint`

### Requirement: A constrained type is its base type plus a check
A value of a constrained type SHALL be a value of its base type, with the same encoding, equality,
host representation and operations. A constrained type SHALL be structural: an alias of one and the
same constraint written inline SHALL be the same type. An alias of a constrained type MAY add terms,
which intersect with its own. A constrained type SHALL satisfy its base type, and every type the base
widens to, with no check. A type `A` SHALL satisfy a constrained type `B` with no check when `A`'s
base widens to `B`'s base and every value `A` admits lies in `B`'s interval. The result of an
arithmetic operator on constrained operands SHALL have the base type's result type, unconstrained.

#### Scenario: A constrained value passes where its base is expected
- **WHEN** a file contains `type Ratio = float32 where value in 0.0..=1.0`, `let wide(x: float64) = { x }` and `let f(r: Ratio) = { wide(r) }`
- **THEN** analysis SHALL accept `wide(r)` and the emitted IR SHALL contain no check for it

#### Scenario: A narrower interval satisfies a wider one
- **WHEN** a file contains `type Ratio = float32 where value in 0.0..=1.0`, `let g(x: float64 where value >= 0.0) = { x }` and `let f(r: Ratio) = { g(r) }`
- **THEN** analysis SHALL accept `g(r)` and the emitted IR SHALL contain no check for it

#### Scenario: Arithmetic yields the base type
- **WHEN** a file contains `type Ratio = float32 where value in 0.0..=1.0` and `let f(r: Ratio) = { r * 2 }`
- **THEN** the result type of `f` SHALL be `float32`

#### Scenario: An alias adds a term
- **WHEN** a file contains `type Ratio = float32 where value in 0.0..=1.0` and `type Low = Ratio where value <= 0.5`
- **THEN** `Low` SHALL admit `0.25` and SHALL NOT admit `0.75`

### Requirement: The checker rejects a constant outside its site's range
A literal, signed literal or constant expression bound to a constrained site SHALL be checked by
the type checker after it takes the site's numeric type, and one the interval does not admit SHALL
be reported as `constraint-violation` at the value, with a message naming the value, the interval as
written and the site. This SHALL apply to property values, record and union-case field values,
arguments, `let` initializers, returned values, sequence items whose item type is constrained, and
defaults where they are declared.

#### Scenario: A property value out of range
- **WHEN** a file contains `type Paint = { opacity: float64 where value in 0.0..=1.0 }` and `let p = <Paint opacity=1.5 />`
- **THEN** analysis SHALL report `constraint-violation` at `1.5`, naming `0.0..=1.0` and `opacity`

#### Scenario: A default out of range
- **WHEN** a file contains `type Paint = { opacity: float64 where value in 0.0..=1.0 = 2.0 }`
- **THEN** analysis SHALL report `constraint-violation` at `2.0`

#### Scenario: An argument and a constant expression
- **WHEN** a file contains `let fade(steps: int where value >= 1) = { steps }` and `let n = { fade(1 - 1) }`
- **THEN** analysis SHALL report `constraint-violation` at `1 - 1`

#### Scenario: An exclusive end
- **WHEN** a file contains `type Unit = float64 where value in 0.0..1.0` and `let u: Unit = 1.0`
- **THEN** analysis SHALL report `constraint-violation` at `1.0`

### Requirement: Any other value is checked where it enters a constrained site
The compiled program SHALL check a value when the program runs, at the binding, whenever the value
is bound to a constrained site its type does not satisfy with no check, as defined above, and it
SHALL fail with `nx-ir-constraint` when the interval does not admit it. The author SHALL write
nothing for the check. The sites SHALL be the ones the constant check covers, plus each item of a
`for` result or sequence bound to a constrained item type, and a field of an update record derived
from a record with a constrained field. `NaN` SHALL fail every constraint.

#### Scenario: A computed argument
- **WHEN** a file contains `let fade(amount: float64 where value in 0.0..=1.0) = { amount }` and `let twice(x: float64) = { fade(x * 2.0) }`, and `twice(0.75)` runs
- **THEN** evaluation SHALL fail with `nx-ir-constraint`, naming `amount` and `0.0..=1.0`
- **AND** `twice(0.25)` SHALL return `0.5`

#### Scenario: A returned value
- **WHEN** a file contains `let clampLow(x: int): int where value >= 0 = { x }` and `clampLow(-1)` runs
- **THEN** evaluation SHALL fail with `nx-ir-constraint`

#### Scenario: The body relies on its parameter
- **WHEN** a file contains `let fade(amount: float64 where value in 0.0..=1.0) = { inner(amount) }` and `let inner(a: float64 where value in 0.0..=1.0) = { a }`
- **THEN** the emitted IR SHALL check `amount` at the call of `fade` and SHALL NOT check it again at
  the call of `inner`

### Requirement: Hover and signature help show the constraint
Hover and signature help SHALL spell the type with its constraint as written, for hover on a
constrained property, parameter, field or alias and for signature help on a constrained parameter.
Hover on a property whose type is an
alias of a constrained type SHALL show the alias name and its constraint.

#### Scenario: Hover through an alias
- **WHEN** a file contains `type Ratio = float32 where value in 0.0..=1.0` and `type Fade = { amount: Ratio }`, and hover is asked on `amount`
- **THEN** the hover SHALL show `amount: Ratio` and `float32 where value in 0.0..=1.0`

### Requirement: The interpreter checks constraints only at compile time
The tree-walking interpreter SHALL run programs with constrained types and SHALL report every
diagnostic the type checker reports for them, and it SHALL NOT be required to check constraints when
a program runs.

#### Scenario: The interpreter accepts a constrained program
- **WHEN** the interpreter evaluates a program declaring `type Ratio = float32 where value in 0.0..=1.0` and `let r: Ratio = 0.5`
- **THEN** evaluation SHALL succeed with `r` equal to `0.5`
