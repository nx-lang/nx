## MODIFIED Requirements

### Requirement: An integer literal takes the floating-point type expected at its binding site
When an integer literal appears at a site whose expected type is a floating-point primitive, the
system SHALL type that literal as the expected floating-point type rather than as `int`, and SHALL
accept the binding. `24` and `24.0` at a `float64` site SHALL be equivalent in every observable
respect: the same type, the same value, the same generated output.

The rule applies to the literal's spelling, not to the type of an arbitrary expression. A negated
integer literal such as `-1` SHALL be treated as an integer literal for this purpose, because
lowering folds the negation into the literal.

The expected type SHALL be the one the site declares after the occurrence is stripped, so an integer
literal SHALL be accepted at a site that reads `float64?`, `float64+` or `float64*` on the same
terms as at a `float64` site, and the exactly-one literal SHALL then satisfy the site by the
lattice in `occurrence-types`.

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
- **WHEN** a file declares `external component <B v?:float64 />` and binds `<B v=0 />`
- **THEN** type checking SHALL accept the binding
- **AND** the type of the literal SHALL be `float64`, and `v` SHALL read as `float64?`

### Requirement: Every site with a declared floating-point type supplies the expectation
The system SHALL apply contextual typing of an integer literal at every site where a floating-point
type is declared for the value being written, not only at component property bindings. Those sites
SHALL include component and external component property bindings, property defaults in a declaration
signature, record field defaults, record field values in a constructed record, annotated `let`
bindings, declared return types, arguments at a floating-point parameter, items of a sequence whose
item type is floating-point, and element body content bound to a declared content property —
whether that content is a single expression or several written as the items of a declared `+` or
`*` content property.

A site that supplies no expected type SHALL be unaffected: an integer literal there SHALL continue
to infer `int`.

#### Scenario: Property default accepts an integer literal
- **WHEN** a file contains `external component <C x: float64 = 0 />`
- **THEN** analysis SHALL accept the default
- **AND** the default value SHALL be `0.0` of type `float64`

#### Scenario: Record field default accepts an integer literal
- **WHEN** a file contains `type Opts = { x: float64 = 1 }`
- **THEN** analysis SHALL accept the default
- **AND** the default value SHALL be `1.0` of type `float64`

#### Scenario: Annotated let accepts an integer literal
- **WHEN** a file contains `let x: float64 = 5`
- **THEN** type checking SHALL accept the binding
- **AND** the type of `x` SHALL be `float64`

#### Scenario: List elements accept integer literals at a float element type
- **WHEN** a file declares a property of type `float64+` and binds it to a braced sequence of the
  literals `1`, `2` and `3`
- **THEN** type checking SHALL accept the binding
- **AND** the bound value SHALL be the sequence `1.0`, `2.0`, `3.0`

#### Scenario: Element body content accepts integer literals
- **WHEN** a file declares a content property of type `float64` and writes an integer literal as the
  element's body content
- **THEN** type checking SHALL accept the binding
- **AND** the same SHALL hold for several integer literals written as body content at a `float64+`
  content property

#### Scenario: Unannotated let still infers int
- **WHEN** a file contains `let n = 42`
- **THEN** analysis SHALL infer the type of `n` as `int`
- **AND** the inference SHALL NOT be affected by any float-typed site elsewhere in the file
