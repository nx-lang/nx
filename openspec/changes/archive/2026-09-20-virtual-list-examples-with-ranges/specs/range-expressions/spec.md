## MODIFIED Requirements

### Requirement: `..` and `..=` are binary operators between the additive and comparison levels
A value expression SHALL accept `a..b`, the half-open range from `a` up to but excluding `b`, and
`a..=b`, the closed range from `a` through `b`. Both SHALL bind more loosely than `+` and `-` and
more tightly than `<`, `>`, `<=`, `>=`, `==` and `!=`, so an arithmetic operand needs no
parentheses. A range expression is a binary expression: inside a braced value list it MUST be
parenthesized, as every binary expression must, and at a site where an unbraced value is a literal —
a binding initializer, a property value — it MUST be braced, as every binary expression must. The
tokens SHALL lex so that an integer literal directly before `..` stays an integer literal, and a
member access on either side binds before the operator.

#### Scenario: Arithmetic operands need no parentheses
- **WHEN** a file contains `let n = 4` and `let r = {0..n + 1}`
- **THEN** the expression SHALL parse as the range from `0` to `n + 1`

#### Scenario: Integer literals and member accesses lex as written
- **WHEN** a file contains `type Page = { first:int last:int }` and `let <Pager page:Page /> = { for i in page.first..=page.last { i } }` and `let r = {1..5}`
- **THEN** `page.first..=page.last` SHALL parse as a closed range between two member accesses
- **AND** `{1..5}` SHALL parse as a half-open range between the integer literals `1` and `5`

#### Scenario: A prefix minus is an operand
- **WHEN** a file contains `let r = {-5..-1}`
- **THEN** the expression SHALL parse as the range from `-5` to `-1`

#### Scenario: An unbraced range is rejected like any other expression
- **WHEN** a file contains `let r = 1..5`
- **THEN** parsing SHALL reject the initializer, as it rejects `let n = 1 + 2`, because an unbraced
  value is a literal and never an expression

#### Scenario: A range in a braced list is parenthesized
- **WHEN** a file contains `let rs:<Range T=int/>[] = { (0..5) (5..=9) }`
- **THEN** analysis SHALL accept the list of two ranges
- **AND** `{ 0..5 5..=9 }` SHALL be rejected as any unparenthesized binary expression in a list is

#### Scenario: A range of a range is a type error, not a parse
- **WHEN** a file contains `let bad = {1..5..9}`
- **THEN** analysis SHALL reject the expression because a `Range` is not a numeric operand

### Requirement: A range expression constructs the prelude's `Range`
A range expression SHALL have the applied type `<Range T=X/>` and SHALL evaluate to the same value
as the element `<Range T=X start={a} end={b} endInclusive={…} />`, with `endInclusive` false for
`..` and true for `..=`. Both operands SHALL be numeric. When the site expects a `<Range T=X/>`,
each operand SHALL be checked as a value bound to a field of type `X`, so a literal or an operand
that widens to `X` is accepted and one that does not is rejected. Otherwise `X` SHALL be the
operands' common numeric type under `implicit-primitive-conversions`, and operands with no common
type SHALL be rejected. Constructing a range SHALL never fail at runtime: a range whose start is
not below its end is a valid value. Equality, field access, `apply`, and passing a range to a host
SHALL behave as they do for any `Range` record.

#### Scenario: Integer bounds give an integer range
- **WHEN** a file contains `let r = {1..5}` and `let typed:<Range T=int/> = {r}` and `let first:int = {r.start}`
- **THEN** analysis SHALL accept all three bindings

#### Scenario: The expected type decides `T`
- **WHEN** a file contains `type Slider = { range:<Range T=float64/> }` and `let s = <Slider range={0..1} />`
- **THEN** analysis SHALL accept the element with `range` a `<Range T=float64/>` whose bounds are `float64` values

#### Scenario: Mixed operands take their common type
- **WHEN** a file contains `let small:int32 = 3` and `let a = {small..10}` and `let b = {0..2.5}`
- **THEN** `a` SHALL be a `<Range T=int/>` and `b` SHALL be a `<Range T=float64/>`

#### Scenario: Operands with no common type are rejected
- **WHEN** a file contains `let big:int64 = 3` and `let bad = {big..2.5}`
- **THEN** analysis SHALL reject the expression because `int64` and `float64` have no common type

#### Scenario: A non-numeric operand is rejected
- **WHEN** a file contains `let bad = {"a".."f"}`
- **THEN** analysis SHALL reject the expression with a diagnostic saying a range operand must be numeric
- **AND** the diagnostic SHALL point to the element form `<Range T=string … />` for other types

#### Scenario: The operator and the element build equal values
- **WHEN** a file contains `let a = {1..=5}` and `let b = <Range T=int start={1} end={5} endInclusive={true} />` and `let same = {a == b}`
- **THEN** `same` SHALL evaluate to `true`
- **AND** `{1..5 == 1..=4}` SHALL evaluate to `false`, since the two are different records

#### Scenario: A reversed range is a value
- **WHEN** a file contains `let r = {5..2}` and `let start = {r.start}`
- **THEN** evaluation SHALL succeed with `start` equal to `5`

### Requirement: A range expression is rejected where `Range` is not the prelude's
In a module where the name `Range` refers to anything other than the prelude's declaration, a range
expression SHALL be rejected with a diagnostic that names the declaration or import that hides the
prelude's `Range`. Whether the name is hidden SHALL NOT depend on the namespace the module's own
declaration occupies: a value, a function, a component, a type alias, a union and a record SHALL each
hide it. A range expression in a module that does not hide the name SHALL be unaffected by a `Range`
declared in another module it does not import.

#### Scenario: A local `Range` disables the operator in that module
- **WHEN** a file contains `type Range = { low:int high:int }` and `let r = {1..5}`
- **THEN** analysis SHALL reject `1..5` with a diagnostic naming the file's own `Range`

#### Scenario: A `Range` outside the type namespace disables the operator too
- **WHEN** a file contains `let Range = 5` and `let r = {1..5}`, and likewise for `let Range() = 5` and for a component named `Range`
- **THEN** analysis SHALL reject `1..5` in each case with a diagnostic naming the file's own `Range`
- **AND** the diagnostic SHALL carry a label at that declaration, whatever kind it is

#### Scenario: Another module's `Range` does not matter
- **WHEN** a workspace module `shapes.nx` declares `export type Range = { low:int high:int }` and another module, which does not import it, contains `let r = {1..5}`
- **THEN** analysis SHALL accept `{1..5}` as the prelude's `<Range T=int/>`
