## Purpose

Defines how a numeric literal takes its type from the site it is written at, rather than only from
its own spelling, so that a whole number can be written as `24` wherever the declaration already
fixes the type as floating-point.

## ADDED Requirements

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

The declared type decides which floating-point type the value is *bound at*; it does not change the
type *recorded for the literal*, which SHALL be whatever a written real literal takes at the same
site. Today that is `float64` at every floating-point site, `float32` included, because a literal
node carries no width and the declared type is authoritative. Recording the narrower type for a
converted `24` would make it more precisely typed than the `24.0` it is required to be
indistinguishable from. Whether a real literal should take `float32` at a `float32` site is a real
question, but it is the same question for both spellings and this capability does not answer it.

#### Scenario: Integer literal binds at a float64 property
- **WHEN** a file declares `external component <B v:float64 />` and binds `<B v=1 />`
- **THEN** type checking SHALL accept the binding
- **AND** the type of the literal SHALL be `float64`

#### Scenario: Integer literal binds at a float32 property
- **WHEN** a file declares `external component <B v:float32 />` and binds `<B v=1 />`
- **THEN** type checking SHALL accept the binding
- **AND** the type recorded for the literal SHALL be the one a written `1.0` takes at that same
  site, so that the two spellings remain indistinguishable

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

### Requirement: Every site with a declared floating-point type supplies the expectation
The system SHALL apply contextual typing of an integer literal at every site where a floating-point
type is declared for the value being written, not only at component property bindings. Those sites
SHALL include component and external component property bindings, property defaults in a declaration
signature, record field defaults, record field values in a constructed record, annotated `let`
bindings, declared return types, arguments at a floating-point parameter, elements of a list whose
element type is floating-point, and element body content bound to a declared content property —
whether that content is a single expression or several written as the elements of a declared list.

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
- **WHEN** a file declares a property of type `float64[]` and binds it to a braced sequence of the
  literals `1`, `2` and `3`
- **THEN** type checking SHALL accept the binding
- **AND** the bound value SHALL be the list `1.0`, `2.0`, `3.0`

#### Scenario: Element body content accepts integer literals
- **WHEN** a file declares a content property of type `float64` and writes an integer literal as the
  element's body content
- **THEN** type checking SHALL accept the binding
- **AND** the same SHALL hold for several integer literals written as body content at a `float64[]`
  content property

#### Scenario: Unannotated let still infers int
- **WHEN** a file contains `let n = 42`
- **THEN** analysis SHALL infer the type of `n` as `int`
- **AND** the inference SHALL NOT be affected by any float-typed site elsewhere in the file

### Requirement: A literal that is not exactly representable in the target float type is rejected
The system SHALL accept an integer literal at a floating-point site only when the literal's value is
representable exactly in that floating-point type, and SHALL report an error otherwise. The system
SHALL NOT silently round an authored constant.

A `float64` site SHALL accept integer literals with absolute value up to and including 2^53, and a
`float32` site up to and including 2^24; beyond those bounds a value SHALL be accepted only when it
is exactly representable in the target type.

The diagnostic SHALL name the literal, the floating-point type it could not be represented in, and
SHALL state that the value is not exactly representable, so the author can choose between a
different constant and an explicit floating-point spelling.

#### Scenario: A literal beyond float64 exact-integer range is rejected
- **WHEN** a file declares `external component <B v:float64 />` and binds `<B v=9007199254740993 />`
- **THEN** type checking SHALL reject the binding
- **AND** the diagnostic SHALL state that the value is not exactly representable as `float64`

#### Scenario: A literal beyond float32 exact-integer range is rejected
- **WHEN** a file declares `external component <B v:float32 />` and binds `<B v=16777217 />`
- **THEN** type checking SHALL reject the binding
- **AND** the diagnostic SHALL state that the value is not exactly representable as `float32`

#### Scenario: A large literal that is exactly representable is accepted
- **WHEN** a file declares `external component <B v:float32 />` and binds `<B v=16777216 />`
- **THEN** type checking SHALL accept the binding

### Requirement: The converted literal is carried as a floating-point value end to end
A converted literal SHALL be a floating-point value everywhere the program is observed, not an
integer that a consumer is expected to widen. Evaluation SHALL produce a floating-point runtime
value, NX IR SHALL encode the literal as a floating-point literal rather than an integer literal,
and generated host code SHALL emit it with a floating-point spelling appropriate to the host type.

A program written with `24` at a floating-point site SHALL be indistinguishable, in evaluated
values, emitted NX IR, and generated code, from the same program written with `24.0`, apart from
source provenance. The two spellings are two source texts of different lengths, so byte offsets,
retained source, and any fingerprint over them differ; NX IR carries that provenance deliberately,
for diagnostics and source maps, and it SHALL continue to describe the text the author actually
wrote.

#### Scenario: Evaluation produces a float value
- **WHEN** a program binds an integer literal at a `float64` property and is evaluated
- **THEN** the runtime value of that property SHALL be a floating-point value
- **AND** it SHALL NOT be an integer value

#### Scenario: NX IR encodes the literal as a float
- **WHEN** NX IR is emitted for a program that binds an integer literal at a `float64` property
- **THEN** the IR SHALL encode that literal as a floating-point literal
- **AND** the recorded type of that literal SHALL be a floating-point type rather than an integer one
- **AND** setting aside source spans, retained source, and the program fingerprint, the emitted IR
  SHALL equal the IR for the same program written with an explicit real literal

#### Scenario: Generated host code uses a floating-point spelling
- **WHEN** C# and TypeScript are generated for a declaration whose `float64` property default is
  written as the integer literal `0`
- **THEN** the generated C# default SHALL be a `double` value
- **AND** the generated output SHALL be equivalent to that for the same declaration written `0.0`

### Requirement: Contextual typing does not widen integer-typed expressions
The system SHALL NOT accept an expression whose type is an integer primitive at a floating-point
site merely because a floating-point type is expected there. Only a literal is typed by context. An
integer-typed variable, parameter, field access, function result, or arithmetic expression at a
floating-point site SHALL continue to be rejected.

This boundary is deliberate: a literal's value is known when the program is analyzed, so exactness
can be decided then, while a value of type `int` spans a 64-bit range whose upper reaches cannot be
represented exactly in any floating-point type and whose loss could not be detected until run time.

#### Scenario: An integer-typed parameter is rejected at a float site
- **WHEN** a file declares `external component <B v:float64 />` and a component parameter `n: int`,
  and binds `<B v={n} />`
- **THEN** type checking SHALL reject the binding
- **AND** the diagnostic SHALL name the expected type `float64` and the actual type `int`

#### Scenario: An integer arithmetic expression is rejected at a float site
- **WHEN** a file declares `external component <B v:float64 />` and binds `<B v={1 + 2} />` where
  both operands are typed `int`
- **THEN** type checking SHALL reject the binding

### Requirement: A floating-point literal at an integer site remains rejected
The system SHALL continue to reject a floating-point literal at a site whose expected type is an
integer primitive. Contextual typing SHALL apply in one direction only.

#### Scenario: Float literal is rejected at an int site
- **WHEN** a file declares `external component <B v:int />` and binds `<B v=1.5 />`
- **THEN** type checking SHALL reject the binding

#### Scenario: A whole-valued float literal is still rejected at an int site
- **WHEN** a file declares `external component <B v:int />` and binds `<B v=1.0 />`
- **THEN** type checking SHALL reject the binding
