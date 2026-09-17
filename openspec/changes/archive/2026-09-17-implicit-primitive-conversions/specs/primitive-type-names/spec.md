## MODIFIED Requirements

### Requirement: Numeric compatibility between and within the numeric categories
The system SHALL accept a numeric value at a site of a different numeric type only when the value's
type widens to the site's type, along the one-directional lattice `implicit-primitive-conversions`
specifies: `int32` to `int` to `int64`, `float32` to `float64`, and `int32` or `int` to `float64`.
Compatibility SHALL NOT run the other way: a wider integer type SHALL NOT bind at a narrower one,
`float64` SHALL NOT bind at `float32`, and the lossy crossings `int64` to any floating-point type
and any integer type to `float32` SHALL be rejected. `int` participates in the lattice as the type
between `int32` and `int64`.

An integer-typed *expression* at a `float64` site is accepted by widening. An integer *literal* at
any numeric site is not governed by compatibility at all: it is typed by context, as specified by
`contextual-numeric-literals`, and that is how a literal reaches a site narrower than its default
type.

When the system promotes two numeric operands to a common type, it SHALL choose the narrowest type
both widen to. Within the integer category that follows the rank order `int32` < `int` < `int64`
and selects the higher-ranked operand's type.

#### Scenario: Integer literal binds to any integer width
- **WHEN** a file declares `external component <B v:int32 />` and binds `<B v=1 />`
- **THEN** type checking SHALL accept the binding

#### Scenario: Integer promotion follows the rank order
- **WHEN** the system promotes `int32` with `int`
- **THEN** the common type SHALL be `int`
- **AND** promoting `int` with `int64` SHALL give `int64`

#### Scenario: Float literal binds to any float width
- **WHEN** a file declares `external component <B v:float32 />` and binds `<B v=1.5 />`
- **THEN** type checking SHALL accept the binding

#### Scenario: Integer literal is accepted at a float site
- **WHEN** a file declares `external component <B v:float64 />` and binds `<B v=1 />`
- **THEN** type checking SHALL accept the binding
- **AND** the literal SHALL be typed `float64`

#### Scenario: Integer-typed expression widens to a float64 site
- **WHEN** a file declares `external component <B v:float64 />` and binds `<B v={n} />` where `n` is
  typed `int`
- **THEN** type checking SHALL accept the binding

#### Scenario: Integer-typed expression is still rejected at a float site
- **WHEN** a file declares `external component <B v:float32 />` and binds `<B v={n} />` where `n` is
  typed `int`, or declares `external component <C v:float64 />` and binds `<C v={w} />` where `w` is
  typed `int64`
- **THEN** type checking SHALL reject both bindings, because neither crossing is exact

#### Scenario: A wider integer expression is rejected at a narrower site
- **WHEN** a file declares `external component <B v:int32 />` and binds `<B v={n} />` where `n` is
  typed `int64`
- **THEN** type checking SHALL reject the binding
- **AND** the diagnostic SHALL name `int64` and `int32`

#### Scenario: A float64 expression is rejected at a float32 site
- **WHEN** a file declares `external component <B v:float32 />` and binds `<B v={x} />` where `x` is
  typed `float64`
- **THEN** type checking SHALL reject the binding
