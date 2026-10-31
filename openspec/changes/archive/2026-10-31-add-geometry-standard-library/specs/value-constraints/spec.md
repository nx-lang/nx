## MODIFIED Requirements

### Requirement: Numeric value patterns are ranges and comparisons joined by `&&`
A value pattern SHALL be one or more terms joined by `&&`, each term `finite`, `in a..b`,
`in a..=b`, `>= a`, `> a`, `<= a` or `< a`, where each bound is a numeric literal, optionally
preceded by `-`. The `finite` term SHALL be accepted only on float32/float64 bases and aliases
of those bases, and SHALL match precisely their finite values. It SHALL be a contextual pattern
term rather than reserving the name in ordinary declarations and fields. `in a..b`
SHALL match values from `a` up to but excluding `b`, `in a..=b` values from `a` through `b`, and a
comparison the values that compare so against its bound. The terms SHALL intersect, so a pattern
matches exactly one interval whose ends are each inclusive, exclusive or open. In a `where` clause,
anything else, including `||`, a comma, `!=`, a call, and a leading operand such as `value > 0`, SHALL
be rejected with `invalid-constraint`, whose message lists the supported terms and, for a leading
operand, shows the pattern without it; it SHALL NOT be a bare parse error.

#### Scenario: Terms intersect
- **WHEN** a file contains `type Unit = float64 where >= 0.0 && < 1.0`
- **THEN** `Unit` SHALL admit `0.0` and `0.5` and SHALL NOT admit `1.0`

#### Scenario: A negative bound
- **WHEN** a file contains `type Pan = float32 where in -1.0..=1.0`
- **THEN** analysis SHALL accept the declaration

#### Scenario: An unsupported pattern is named, not a parse error
- **WHEN** a file contains `type Edge = int where < 0 || > 9`
- **THEN** analysis SHALL report `invalid-constraint` on the pattern, listing `finite`, `in`, `<`, `<=`, `>`,
  `>=` and `&&` as what a constraint may use

#### Scenario: A leading operand gets a fix
- **WHEN** a file contains `type Count = int where value >= 1`
- **THEN** analysis SHALL report `invalid-constraint` and the message SHALL suggest `int where >= 1`


#### Scenario: Finiteness combines with an interval
- **WHEN** a file declares `type Extent = float64 where finite && >= 0.0`
- **THEN** Extent SHALL accept zero and positive finite values and reject negative values, both infinities, and NaN

#### Scenario: Finite is not a globally reserved identifier
- **WHEN** a program declares a field or function named finite outside a numeric pattern
- **THEN** it SHALL retain ordinary identifier parsing and resolution

## ADDED Requirements

### Requirement: Finite patterns preserve existing scalar and constraint behavior
The finite term SHALL accept positive/negative zero, subnormal values, and the maximum positive
and negative finite values of its floating base, and SHALL reject both infinities and all NaNs.
It SHALL compose through conjunction, alias intersection, structural equality, satisfaction,
widening, and joins using the same interval semantics as existing bounded constraints. It SHALL
be equivalent to the inclusive interval bounded by the selected floating base's maximum finite
magnitudes. It SHALL use existing constraint binding sites and diagnostic categories without
requiring a new runtime capability or public value encoding. Integer and nonnumeric finite
patterns SHALL produce invalid-constraint.

#### Scenario: Both floating precisions retain their exact domain
- **WHEN** finite constraints on float32 and float64 are evaluated with signed zero, subnormal values, their respective largest finite magnitudes, infinities, and NaN
- **THEN** analysis and supported runtime checks SHALL agree on membership at the selected precision
- **AND** maximum finite values SHALL survive emitted-image and schema serialization round trips

#### Scenario: Widening does not remove a finiteness check
- **WHEN** an unconstrained float32 argument is passed to a float64 where finite parameter
- **THEN** a runtime constraint check SHALL remain necessary because the source type admits infinities
- **WHEN** the source parameter is float32 where finite instead
- **THEN** the existing satisfaction rules SHALL permit widening without a redundant finiteness check

#### Scenario: Invalid input uses established diagnostics
- **WHEN** a statically known non-finite value is bound to a finite constrained site
- **THEN** analysis SHALL report constraint-violation
- **WHEN** a dynamic non-finite value reaches that site in Rust or TypeScript IR evaluation
- **THEN** evaluation SHALL report nx-ir-constraint with the constrained site identified

#### Scenario: Finite is rejected on an integer base
- **WHEN** a declaration uses int where finite or a numeric arm uses finite for an integer scrutinee
- **THEN** analysis SHALL report invalid-constraint explaining that finite applies to floating types

### Requirement: Finite is available in numeric matching and source presentation
The finite term SHALL be accepted wherever numeric value patterns are accepted, including numeric
is arms, using the same membership and narrowing semantics as a finite constraint. Hover and
source presentation SHALL preserve its authored spelling, including through aliases. Schema
export SHALL convey its effective finite interval through existing numeric-bound fields rather
than a new schema keyword. Existing runtime normalized-interval diagnostic spelling SHALL remain
permitted. Arithmetic SHALL continue to return unconstrained base types.

#### Scenario: Match narrows a floating value
- **WHEN** an is arm matching finite passes its local floating scrutinee to a parameter constrained by finite
- **THEN** analysis SHALL accept the call without a redundant check
- **AND** either infinity or NaN SHALL bypass the arm and match else if present

#### Scenario: A constraint survives compiler and tooling surfaces
- **WHEN** a finite constrained property is shown in hover, exported as a declaration schema, and evaluated from a stripped IR image
- **THEN** hover SHALL retain the finite spelling, schema bounds SHALL describe the equivalent interval, and evaluation SHALL enforce the same constraint

#### Scenario: A constrained arithmetic input does not guarantee a constrained output
- **WHEN** two finite values are multiplied and their product is bound to a finite constrained result
- **THEN** ordinary constraint rules SHALL require checking the unconstrained arithmetic result
- **AND** an infinite product SHALL fail that result binding
