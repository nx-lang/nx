## REMOVED Requirements

### Requirement: Numeric compatibility is unchanged by the renaming
**Reason**: The requirement's name refers to a renaming that is long since archived, and its central
claim — that an integer value is rejected at a floating-point binding site — is what this change
narrows. Replaced by *Numeric compatibility between and within the numeric categories*, which keeps
every rule that survives and states the literal exception in terms of
`contextual-numeric-literals`. Its scenario *Integer literal is still rejected at a float site*
asserts the opposite of the new behavior and has no successor under that name.

**Migration**: None for authored source. Every program that satisfied the removed requirement still
type checks: the replacement accepts strictly more, adding integer literals at floating-point sites.
The scenario asserting rejection of `<B v=1 />` at `v:float64` is replaced by one asserting
acceptance, and the rejection it described survives for integer-typed *expressions* in the scenario
*Integer-typed expression is still rejected at a float site*.

## ADDED Requirements

### Requirement: Numeric compatibility between and within the numeric categories
The system SHALL treat any integer type as compatible with any other integer type, and any
floating-point type as compatible with any other floating-point type, in both directions. `int`
participates in integer compatibility exactly as `int32` and `int64` do.

The system SHALL reject an integer-typed *expression* at a floating-point binding site. An integer
*literal* at such a site is not governed by type compatibility at all: it is typed by context and
accepted, as specified by `contextual-numeric-literals`.

When the system promotes two integer operands to a common type, it SHALL follow the rank order
`int32` < `int` < `int64` and select the higher-ranked operand's type.

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

#### Scenario: Integer-typed expression is still rejected at a float site
- **WHEN** a file declares `external component <B v:float64 />` and binds `<B v={n} />` where `n` is
  typed `int`
- **THEN** type checking SHALL reject the binding

## MODIFIED Requirements

### Requirement: Diagnostics render one name per type on both sides of a message
When the system reports a type mismatch, it SHALL render both the expected and the found type using
canonical primitive names. The system SHALL NOT render two different names for the same primitive
type, and SHALL NOT vary the rendered name according to how the type was spelled at its declaration.

The requirement is unchanged. Its illustrating example is: an integer *literal* at a `float64`
property is now accepted, so `<B v=1 />` no longer produces a diagnostic of any kind and cannot
demonstrate how one is rendered. An `int`-typed expression at that same site is still rejected and
still produces "expects float64, found int", so the same two canonical names appear on the same two
sides of the same message.

#### Scenario: Mismatch message uses canonical names throughout
- **WHEN** a file declares `external component <B v:float64 />` and a component parameter `n: int`,
  and binds `<B v={n} />`
- **THEN** the diagnostic SHALL name the expected type `float64`
- **AND** SHALL name the found type `int`

#### Scenario: Two declarations of the same type produce the same message
- **WHEN** two files each declare a property of the same primitive type and each receives an
  incompatible value
- **THEN** both diagnostics SHALL render that primitive's name identically

### Requirement: Integer literals infer `int` and floating-point literals infer `float64`
The system SHALL infer `int` for an integer literal and `float64` for a floating-point literal where
no expected type applies. `int` is the default integer type: it is what an unannotated integer takes,
and what NX sources use unless a declaration has a specific reason to name a width.

Where a floating-point type is expected, an integer literal takes that type instead of `int`, as
specified by `contextual-numeric-literals`. Inference from the literal's own spelling is the
fallback, not the only rule.

"Takes that type" is a statement about the binding, which is what a reader of the declaration
observes. The type recorded for the literal *expression* is `contextual-numeric-literals`' subject,
and it is deliberately whatever an explicit real literal takes at the same site, so that the two
spellings stay indistinguishable.

#### Scenario: Integer literal infers int
- **WHEN** a file contains `let n = 42`
- **THEN** analysis SHALL infer the type of `n` as `int`

#### Scenario: Float literal infers float64
- **WHEN** a file contains `let x = 1.5`
- **THEN** analysis SHALL infer the type of `x` as `float64`

#### Scenario: An expected float type overrides the default
- **WHEN** a file contains `let x: float32 = 42`
- **THEN** analysis SHALL infer the type of `x` as `float32`
- **AND** it SHALL NOT infer `int`
- **AND** the result SHALL be the same as for `let x: float32 = 42.0`
