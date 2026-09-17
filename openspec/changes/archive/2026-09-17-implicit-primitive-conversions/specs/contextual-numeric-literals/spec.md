## MODIFIED Requirements

### Requirement: An integer literal takes the floating-point type expected at its binding site
When an integer literal appears at a site whose expected type is a floating-point primitive, the
system SHALL type that literal as the expected floating-point type rather than as `int`, and SHALL
accept the binding. `24` and `24.0` at a `float64` site SHALL be equivalent in every observable
respect: the same type, the same value, the same generated output.

The rule applies to the literal's spelling, not to the type of an arbitrary expression. A negated
integer literal such as `-1` SHALL be treated as an integer literal for this purpose, because
lowering folds the negation into the literal.

The expected type SHALL be the one the site declares after nullability is stripped, so an integer
literal SHALL be accepted at a `float64?` site on the same terms as at a `float64` site.

The declared type decides both the type the value is *bound at* and the type *recorded for the
literal*, and the recorded type SHALL be the same one a written real literal takes at that site.
Under `implicit-primitive-conversions` a real literal takes the floating-point width of its site,
so a converted `24` at a `float32` site SHALL record `float32`, exactly as `24.0` there does. The
two spellings remain indistinguishable, now at the site's own width rather than at `float64`.

#### Scenario: Integer literal binds at a float64 property
- **WHEN** a file declares `external component <B v:float64 />` and binds `<B v=1 />`
- **THEN** type checking SHALL accept the binding
- **AND** the type of the literal SHALL be `float64`

#### Scenario: Integer literal binds at a float32 property
- **WHEN** a file declares `external component <B v:float32 />` and binds `<B v=1 />`
- **THEN** type checking SHALL accept the binding
- **AND** the type recorded for the literal SHALL be `float32`, the one a written `1.0` takes at
  that same site, so that the two spellings remain indistinguishable

#### Scenario: Integer literal and float literal spellings agree
- **WHEN** a file declares `external component <B v:float64 />` and binds `<B v=24 />` in one program
  and `<B v=24.0 />` in another
- **THEN** both programs SHALL type check
- **AND** the two programs SHALL produce the same value for `v`

#### Scenario: Negative integer literal binds at a float site
- **WHEN** a file declares `external component <B v:float64 />` and binds `<B v=-1 />`
- **THEN** type checking SHALL accept the binding
- **AND** the bound value SHALL equal `-1.0`

#### Scenario: Integer literal binds at a nullable float site
- **WHEN** a file declares `external component <B v:float64? />` and binds `<B v=0 />`
- **THEN** type checking SHALL accept the binding
- **AND** the type of the literal SHALL be `float64`

## REMOVED Requirements

### Requirement: Contextual typing does not widen integer-typed expressions
**Reason**: The requirement's rationale was that a value of type `int` "spans a 64-bit range whose
upper reaches cannot be represented exactly in any floating-point type". `int` has since been
specified as exact over ±(2^53−1), which is precisely the integer range `float64` represents
without loss, so widening an `int` or `int32` expression to `float64` is exact and
`implicit-primitive-conversions` now admits it. The lossy crossings the old requirement was guarding
against (`int64` to any float, any integer to `float32`) remain rejected there.
**Migration**: None for authors; programs that were rejected now type check. The scenarios that
rejected an `int` parameter and an `int` arithmetic expression at a `float64` site are replaced by
the widening scenarios in `implicit-primitive-conversions`.
