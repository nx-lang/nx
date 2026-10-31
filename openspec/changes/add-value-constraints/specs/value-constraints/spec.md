## ADDED Requirements

### Requirement: A type may be followed by a `where` clause holding a value pattern
A type reference SHALL accept an optional clause `where` followed by a value pattern, after the base
type and any occurrence suffix, wherever a type is written: a record, action or union-case field, a
component prop or state field, a function parameter or result, a `let` annotation and an alias. The
pattern SHALL have no operand: it is matched against the value being constrained, as a match arm's
pattern is matched against the scrutinee. `where` SHALL be a keyword only after a type, so a
declaration, field or parameter named `where` elsewhere SHALL be unaffected. A default SHALL follow
the whole type, constraint included. The clause SHALL end at the first token that cannot continue the
pattern, so a whitespace-separated field list needs no braces or separators around it.

#### Scenario: A constrained field and the field after it
- **WHEN** a file contains `type Paint = { opacity: float64 where in 0.0..=1.0 = 1.0  layer: int }`
- **THEN** parsing SHALL produce two fields, `opacity` with a constraint and the default `1.0`, and
  `layer` with none

#### Scenario: A named constrained type
- **WHEN** a file contains `type Ratio = float32 where in 0.0..=1.0` and
  `type Fade = { amount: Ratio }`
- **THEN** analysis SHALL accept both, with `amount` constrained to `0.0..=1.0`

#### Scenario: Parameters, results and annotations
- **WHEN** a file contains `let fade(amount: float32 where in 0.0..=1.0, steps: int where >= 1): int where >= 0 = { steps }`
  and `let half: float64 where <= 1.0 = 0.5`
- **THEN** analysis SHALL accept both declarations

#### Scenario: A field may be named `where` or `value`
- **WHEN** a file contains `type Slider = { value: float64 where in 0.0..=1.0  where: string }`
- **THEN** analysis SHALL accept the record with the fields `value` and `where`

### Requirement: Numeric value patterns are ranges and comparisons joined by `&&`
A value pattern SHALL be one or more terms joined by `&&`, each term `in a..b`, `in a..=b`, `>= a`,
`> a`, `<= a` or `< a`, where each bound is a numeric literal, optionally preceded by `-`. `in a..b`
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
- **THEN** analysis SHALL report `invalid-constraint` on the pattern, listing `in`, `<`, `<=`, `>`,
  `>=` and `&&` as what a constraint may use

#### Scenario: A leading operand gets a fix
- **WHEN** a file contains `type Count = int where >= 1`
- **THEN** analysis SHALL report `invalid-constraint` and the message SHALL suggest `int where >= 1`

### Requirement: Only numeric types without an occurrence take a constraint
The base of a constrained type SHALL be `int`, `int32`, `int64`, `float32` or `float64`, or an alias
of one, constrained or not. Each bound SHALL be checked as a literal written at the base type, so a
real bound on an integer base and a bound outside the base's range SHALL be rejected with
`invalid-constraint`. A constraint on a type with an occurrence suffix, on a type reached through an
alias that carries one, on a non-numeric type, on a type argument and inside a function type SHALL
be rejected with `invalid-constraint`; for an occurrence the diagnostic SHALL show naming the item
type as the fix. A pattern that matches no value of the base type SHALL be rejected with
`empty-constraint`.

#### Scenario: A real bound on an integer
- **WHEN** a file contains `type Half = int where <= 0.5`
- **THEN** analysis SHALL report `invalid-constraint` on `0.5`

#### Scenario: A bound outside the base's range
- **WHEN** a file contains `type Big = int32 where <= 3000000000`
- **THEN** analysis SHALL report `invalid-constraint` saying the bound is out of range for `int32`

#### Scenario: A constraint on a sequence
- **WHEN** a file contains `type Scores = int+ where >= 0`
- **THEN** analysis SHALL report `invalid-constraint`, and the message SHALL suggest
  `type Score = int where >= 0` with `Score+`

#### Scenario: A constraint on a string
- **WHEN** a file contains `type Name = string where >= 1`
- **THEN** analysis SHALL report `invalid-constraint` saying constraints apply to numeric types

#### Scenario: A constrained type argument
- **WHEN** a file contains `type Ratio = float32 where in 0.0..=1.0` and `type Band = { r: <Range T=Ratio/> }`
- **THEN** analysis SHALL report `invalid-constraint` on the type argument

#### Scenario: An empty constraint
- **WHEN** a file contains `type None = int where > 5 && < 6`
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
- **WHEN** a file contains `type Ratio = float32 where in 0.0..=1.0`, `let wide(x: float64) = { x }` and `let f(r: Ratio) = { wide(r) }`
- **THEN** analysis SHALL accept `wide(r)` and the emitted IR SHALL contain no check for it

#### Scenario: A narrower interval satisfies a wider one
- **WHEN** a file contains `type Ratio = float32 where in 0.0..=1.0`, `let g(x: float64 where >= 0.0) = { x }` and `let f(r: Ratio) = { g(r) }`
- **THEN** analysis SHALL accept `g(r)` and the emitted IR SHALL contain no check for it

#### Scenario: Arithmetic yields the base type
- **WHEN** a file contains `type Ratio = float32 where in 0.0..=1.0` and `let f(r: Ratio) = { r * 2 }`
- **THEN** the result type of `f` SHALL be `float32`

#### Scenario: An alias adds a term
- **WHEN** a file contains `type Ratio = float32 where in 0.0..=1.0` and `type Low = Ratio where <= 0.5`
- **THEN** `Low` SHALL admit `0.25` and SHALL NOT admit `0.75`

### Requirement: The checker rejects a constant outside its site's range
A literal, signed literal or constant expression bound to a constrained site SHALL be checked by
the type checker after it takes the site's numeric type, and one the interval does not admit SHALL
be reported as `constraint-violation` at the value, with a message naming the value, the interval as
written and the site. This SHALL apply to property values, record and union-case field values,
arguments, `let` initializers, returned values, sequence items whose item type is constrained, and
defaults where they are declared.

#### Scenario: A property value out of range
- **WHEN** a file contains `type Paint = { opacity: float64 where in 0.0..=1.0 }` and `let p = <Paint opacity=1.5 />`
- **THEN** analysis SHALL report `constraint-violation` at `1.5`, naming `0.0..=1.0` and `opacity`

#### Scenario: A default out of range
- **WHEN** a file contains `type Paint = { opacity: float64 where in 0.0..=1.0 = 2.0 }`
- **THEN** analysis SHALL report `constraint-violation` at `2.0`

#### Scenario: An argument and a constant expression
- **WHEN** a file contains `let fade(steps: int where >= 1) = { steps }` and `let n = { fade(1 - 1) }`
- **THEN** analysis SHALL report `constraint-violation` at `1 - 1`

#### Scenario: An exclusive end
- **WHEN** a file contains `type Unit = float64 where in 0.0..1.0` and `let u: Unit = 1.0`
- **THEN** analysis SHALL report `constraint-violation` at `1.0`

### Requirement: Any other value is checked where it enters a constrained site
The compiled program SHALL check a value when the program runs, at the binding, whenever the value
is bound to a constrained site its type does not satisfy with no check, as defined above, and it
SHALL fail with `nx-ir-constraint` when the interval does not admit it. The author SHALL write
nothing for the check. The sites SHALL be the ones the constant check covers, plus each item of a
`for` result or sequence bound to a constrained item type, and a field of an update record derived
from a record with a constrained field. `NaN` SHALL fail every constraint.

#### Scenario: A computed argument
- **WHEN** a file contains `let fade(amount: float64 where in 0.0..=1.0) = { amount }` and `let twice(x: float64) = { fade(x * 2.0) }`, and `twice(0.75)` runs
- **THEN** evaluation SHALL fail with `nx-ir-constraint`, naming `amount` and `0.0..=1.0`
- **AND** `twice(0.25)` SHALL return `0.5`

#### Scenario: A returned value
- **WHEN** a file contains `let clampLow(x: int): int where >= 0 = { x }` and `clampLow(-1)` runs
- **THEN** evaluation SHALL fail with `nx-ir-constraint`

#### Scenario: The body relies on its parameter
- **WHEN** a file contains `let fade(amount: float64 where in 0.0..=1.0) = { inner(amount) }` and `let inner(a: float64 where in 0.0..=1.0) = { a }`
- **THEN** the emitted IR SHALL check `amount` at the call of `fade` and SHALL NOT check it again at
  the call of `inner`

### Requirement: A match arm may use a numeric value pattern
A match arm of `if x is { … }`, in its value, element and property-list forms, SHALL accept the
numeric value patterns a `where` clause accepts, beside the patterns it already accepts, and a comma
SHALL separate alternatives as it does today, so an arm may read `< 0, > 100 =>`. A numeric pattern
SHALL require a scrutinee of a numeric type, possibly constrained, and SHALL be rejected otherwise;
each bound SHALL be checked as a literal written at the scrutinee's base type. An arm SHALL match when
the scrutinee lies in its interval; `NaN` SHALL match none. Numeric patterns SHALL follow the rules
for literal matches: an `else` MAY be omitted, and a match with no arm taken is the empty value. A
pattern that matches no value of the scrutinee's type SHALL be reported as `empty-constraint`. Within
an arm whose scrutinee is a local identifier and whose patterns are one numeric pattern, the type
checker SHALL narrow that identifier to the scrutinee's type constrained by the pattern, so passing it
to a site whose interval contains the pattern's needs no runtime check. An unbraced arm body SHALL
end where the next arm's pattern begins, decided by the `=>` after that pattern, so a body followed
by an arm starting with `<`, `>`, `<=`, `>=` or `-` SHALL NOT be read as one comparison or
subtraction; this SHALL hold for the signed literal patterns arms already accept.

#### Scenario: Dispatch on ranges
- **WHEN** a file contains `let describe(x: float64) = { if x is { in 0.0..1.0 => "valid" > 1.0 => "too big" < 0.0 => "too small" else => "one" } }`
- **THEN** `describe(0.5)` SHALL be `"valid"`, `describe(2.0)` `"too big"`, `describe(-1.0)`
  `"too small"` and `describe(1.0)` `"one"`

#### Scenario: Alternatives and conjunctions
- **WHEN** a file contains `let flag(n: int) = { if n is { < 0, > 100 => "out" > 0 && <= 10 => "low" } }`
- **THEN** `flag(-5)` and `flag(200)` SHALL be `"out"`, `flag(3)` SHALL be `"low"`, and `flag(50)`
  SHALL be the empty value

#### Scenario: A body is not continued by the next arm's pattern
- **WHEN** a file contains `let g(n: int) = { if n is { 1 => 5 -1 => 6 > 9 => 7 else => 8 } }`
- **THEN** parsing SHALL produce three arms and an `else`, and `g(-1)` SHALL be `6` and `g(10)` `7`

#### Scenario: A numeric pattern on a string
- **WHEN** a file contains `let bad(s: string) = { if s is { > 3 => 1 else => 0 } }`
- **THEN** analysis SHALL reject `> 3` because the scrutinee is not numeric

#### Scenario: An arm narrows its scrutinee
- **WHEN** a file contains `let fade(amount: float64 where in 0.0..=1.0) = { amount }` and `let safe(x: float64) = { if x is { in 0.0..=1.0 => fade(x) else => 0.0 } }`
- **THEN** analysis SHALL accept `fade(x)` and the emitted IR SHALL contain no check for it

### Requirement: Hover and signature help show the constraint
Hover and signature help SHALL spell the type with its constraint as written, for hover on a
constrained property, parameter, field or alias and for signature help on a constrained parameter.
Hover on a property whose type is an
alias of a constrained type SHALL show the alias name and its constraint.

#### Scenario: Hover through an alias
- **WHEN** a file contains `type Ratio = float32 where in 0.0..=1.0` and `type Fade = { amount: Ratio }`, and hover is asked on `amount`
- **THEN** the hover SHALL show `amount: Ratio` and `float32 where in 0.0..=1.0`

### Requirement: The interpreter checks constraints only at compile time
The tree-walking interpreter SHALL run programs with constrained types and SHALL report every
diagnostic the type checker reports for them, and it SHALL NOT be required to check constraints when
a program runs. It SHALL evaluate numeric match patterns with the results the IR runtimes give.

#### Scenario: The interpreter accepts a constrained program
- **WHEN** the interpreter evaluates a program declaring `type Ratio = float32 where in 0.0..=1.0` and `let r: Ratio = 0.5`
- **THEN** evaluation SHALL succeed with `r` equal to `0.5`

#### Scenario: The interpreter matches a range pattern
- **WHEN** the interpreter evaluates `if 0.5 is { in 0.0..1.0 => "valid" else => "no" }`
- **THEN** the result SHALL be `"valid"`
