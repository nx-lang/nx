# primitive-type-names Specification

## Purpose
TBD - created by archiving change rename-primitive-types. Update Purpose after archive.

## Requirements

### Requirement: NX defines exactly one spelling for each primitive type
The set of NX primitive type names SHALL be exactly `string`, `int`, `int32`, `int64`, `float32`,
`float64`, `boolean`, and `object`. The system SHALL NOT provide an alias, synonym, or alternate
spelling for any primitive type, and primitive names SHALL be matched case-sensitively: a name that
differs only in letter case SHALL be resolved as an ordinary named type, not as a primitive. Each
primitive SHALL have a single canonical name that the parser accepts and that every diagnostic,
formatter, and code generator renders.

`void` is no longer among them. It SHALL NOT resolve to a primitive type in type position, and the
system SHALL treat it as an ordinary named type reference, resolved by exactly the same rules as any
other name that is not a primitive.

This requirement governs source spellings only. How each primitive is represented internally is
unchanged: `object` continues to be carried as a named type rather than as a variant of the
`Primitive` model, a pre-existing mismatch that design.md lists as a non-goal.

#### Scenario: Canonical numeric names are accepted in type position
- **WHEN** a file contains `type Sizes = { n:int a:int32 b:int64 c:float32 d:float64 }`
- **THEN** parsing and type analysis SHALL accept all five field types
- **AND** SHALL preserve each as the corresponding primitive type

#### Scenario: `int` is a distinct type, not a spelling of `int64`
- **WHEN** analysis compares the primitive named `int` with the primitive named `int64`
- **THEN** they SHALL NOT be equal
- **AND** each SHALL render under its own name in diagnostics

#### Scenario: Non-numeric primitive names are accepted in type position
- **WHEN** a file contains `type Misc = { s:string b:boolean o:object }`
- **THEN** parsing and type analysis SHALL accept all three field types

#### Scenario: `void` no longer resolves in type position
- **WHEN** a file contains `type Handler = { result:void }` and no type named `void` is declared
- **THEN** analysis SHALL NOT treat the field as a primitive type
- **AND** SHALL resolve the name by exactly the rules it applies to any other undeclared name, so a
  value that does not satisfy the named type SHALL be rejected at the binding
- **AND** code generation SHALL NOT map the field to a host `void` type

#### Scenario: A user declaration may take the name `void`
- **WHEN** a file contains `type void = { value:int }` and a field declared `n:void`
- **THEN** analysis SHALL resolve `n` to the user-defined record type
- **AND** SHALL NOT treat `void` as a primitive

#### Scenario: A capitalized spelling is not a primitive
- **WHEN** a file contains `type Weird = { n:INT a:INT64 b:Boolean c:String o:Object }` and no type
  of any of those names is declared
- **THEN** analysis SHALL NOT treat any of the five fields as a primitive type
- **AND** code generation SHALL NOT map any of them to a host primitive type

### Requirement: The former spellings are no longer type names
The names `i32`, `i64`, `f32`, `f64`, `float`, and `bool` SHALL NOT resolve to primitive types. The system SHALL treat each of them as an ordinary named type reference, resolved by exactly
the same rules as any other name that is not a primitive.

#### Scenario: Width-suffixed shorthand no longer resolves
- **WHEN** a file contains `type Point = { x:f64 y:f64 }` and no type named `f64` is declared
- **THEN** analysis SHALL NOT treat the field as a floating-point primitive
- **AND** code generation SHALL NOT map the field to a host floating-point type

#### Scenario: The former float alias no longer resolves
- **WHEN** a file contains `type Ratio = { n:float }` and no type named `float` is declared
- **THEN** analysis SHALL NOT treat the field as a floating-point primitive

#### Scenario: The former boolean spelling no longer resolves
- **WHEN** a file contains `type Flags = { on:bool }` and no type named `bool` is declared
- **THEN** analysis SHALL NOT treat the field as a boolean primitive

#### Scenario: A user-defined type may take a former primitive name
- **WHEN** a file contains `type i64 = { value:int }` and a field declared `n:i64`
- **THEN** analysis SHALL resolve `n` to the user-defined record type
- **AND** SHALL NOT treat `i64` as a primitive

#### Scenario: A user declaration does not displace a primitive name
- **WHEN** a file contains `type int = { value:string }` and a property declared `n:int`
- **THEN** analysis SHALL resolve `n` to the `int` primitive rather than to the declared record
- **AND** SHALL reject a string value supplied for `n`

### Requirement: Diagnostics render one name per type on both sides of a message
When the system reports a type mismatch, it SHALL render both the expected and the found type using
canonical primitive names. The system SHALL NOT render two different names for the same primitive
type, and SHALL NOT vary the rendered name according to how the type was spelled at its declaration.

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

### Requirement: Host language type mappings are preserved under the new names
Code generation SHALL map each canonical primitive to the same host type it produced before the
renaming. `int32` SHALL map to C# `int`, `int64` to C# `long`, `float32` to C# `float`, and
`float64` to C# `double`. `int` SHALL map to C# `long`, because its specified range does not fit a
C# `int`. All five SHALL map to TypeScript `number`. `boolean` SHALL map to C#
`bool` and to TypeScript `boolean`.

#### Scenario: C# generation preserves widths
- **WHEN** source contains `export type Sizes = { n:int a:int32 b:int64 c:float32 d:float64 }`
- **AND** C# types are generated for it
- **THEN** the generated members SHALL be typed `long`, `int`, `long`, `float`, and `double`
  respectively

#### Scenario: TypeScript generation maps every numeric primitive to number
- **WHEN** source contains `export type Sizes = { n:int a:int32 b:int64 c:float32 d:float64 }`
- **AND** TypeScript types are generated for it
- **THEN** all five generated members SHALL be typed `number`

#### Scenario: Boolean generation is unaffected by the rename
- **WHEN** source contains `export type Toggle = { enabled:boolean = true }`
- **AND** C# and TypeScript types are generated for it
- **THEN** the generated C# member SHALL be typed `bool`
- **AND** the generated TypeScript member SHALL be typed `boolean`

### Requirement: `int` has one specified range on every backend
`int` SHALL be exact over ±(2^53−1) on every backend. That range SHALL NOT vary by target,
by host word size, or by which NX implementation evaluates the program: `int` SHALL NOT be an
implementation-defined width. Backends MAY store an `int` in any slot that holds the whole
specified range — a C# `long`, a Rust `i64`, a JavaScript `number` — because a backend's choice
among those is unobservable to an NX program.

Arithmetic on `int` SHALL be defined as checked rather than wrapping: a result outside the
specified range is an error, not a silently truncated value.

Enforcement of both the range and the checked-arithmetic rule is deliberately deferred to a later
change, which also covers user-declared ranges (`1..10`) so that one bounds-check mechanism serves
both. Until that change lands, the range is a specified guarantee that the runtime does not yet
police, and integer arithmetic continues to wrap.

#### Scenario: `int` is specified independently of the evaluating backend
- **WHEN** the same NX program is evaluated by the Rust interpreter, by generated C#, and by
  generated TypeScript
- **THEN** the specified range of `int` SHALL be identical in all three

#### Scenario: A backend may choose any storage that covers the range
- **WHEN** a backend stores `int` in a C# `long`, a Rust `i64`, or a JavaScript `number`
- **THEN** the choice SHALL NOT change the specified range

### Requirement: `int64` remains a JavaScript `number` for now
`int64` SHALL continue to generate TypeScript `number`, as it does today, even though `number` is
exact only to 2^53−1 and therefore cannot represent the whole of `int64`. Carrying `int64` as a
JavaScript `bigint` is the intended direction and is deferred to its own change.

This is a known and deliberate gap, and it is the reason `int` rather than `int64` is the default
integer type: `int`'s specified range is exactly the range every backend — JavaScript included —
represents without loss.

#### Scenario: int64 generates a TypeScript number
- **WHEN** source contains `export type Wide = { v:int64 }`
- **AND** TypeScript types are generated for it
- **THEN** the generated member SHALL be typed `number`

### Requirement: Primitive type completions offer exactly the canonical names
Editor tooling SHALL offer the canonical primitive type names as completions in type position, and
SHALL NOT offer any name that is not an NX primitive type.

Every other first-party listing of the primitive names SHALL match the same set. Syntax
highlighting and first-party documentation each enumerate the names independently of the compiler,
so a name removed from the language stays visible to an author until each is updated — highlighting
a user type that legitimately takes a freed name as though it were a primitive, and advertising a
spelling the parser rejects.

#### Scenario: Completion list matches the primitive set
- **WHEN** an editor requests primitive type completions
- **THEN** the offered names SHALL be `string`, `int`, `int32`, `int64`, `float32`, `float64`,
  `boolean`, and `object`
- **AND** SHALL NOT include `void`, `long`, `double`, `bool`, or any former alias

#### Scenario: Syntax highlighting matches the primitive set
- **WHEN** editor syntax highlighting classifies a name in type position as a primitive type
- **THEN** the names it classifies that way SHALL be exactly the primitive set
- **AND** `void` SHALL NOT be among them, so a user type named `void` is not coloured as a primitive

#### Scenario: First-party documentation matches the primitive set
- **WHEN** first-party documentation lists the primitive type names
- **THEN** the list SHALL be exactly the primitive set
- **AND** SHALL NOT include `void`

### Requirement: The bottom type is inference-internal and has no source spelling
The system SHALL provide a bottom type that is below every type: it SHALL satisfy every expected
type, and joining it with any type SHALL yield that type. It exists for the empty list, whose type
is a list of it, and which is usable at every list-typed site for exactly that reason.

That type SHALL NOT be nameable in NX source. `never` SHALL NOT be a primitive type name, SHALL NOT
be offered as a completion, and SHALL NOT be highlighted as a primitive, so the primitive set stays
the eight names above. A user declaration MAY take the name `never`, resolved by the same rules that
govern any non-primitive name.

The system SHALL render it as `never` in a diagnostic that names a type it inferred, on the same
terms as the unit type. Where a diagnostic reports a type the author wrote as `{}`, it SHALL spell
it `{}` rather than naming the bottom type, because that is the form the author can act on.

No value SHALL have the bottom type, so it SHALL NOT appear in any runtime representation, in a
value crossing the host boundary, or in a runtime type test. Its surface is therefore smaller than
the unit type's, which does appear in each of those.

Code generation SHALL render it in every target it can reach. It can reach only the targets that
render *inferred* types; a target that maps from source type annotations SHALL NOT be able to reach
it at all, because it has no source spelling to map from.

#### Scenario: An empty list is usable at every list-typed site
- **WHEN** type inference analyzes an empty braced list
- **THEN** its type SHALL be a list of the bottom type
- **AND** it SHALL satisfy `string[]`, `int[]`, and any other list type, without an expected type
  having been supplied to determine an element type

#### Scenario: The bottom type cannot be written in source
- **WHEN** a file annotates a binding as `never` and no type named `never` is declared
- **THEN** analysis SHALL NOT resolve the annotation to the bottom type
- **AND** SHALL treat it as a reference to an undeclared name

#### Scenario: A user declaration may take the name `never`
- **WHEN** a file contains `type never = { value:int }` and a field declared `n:never`
- **THEN** analysis SHALL resolve `n` to the user-defined record type

#### Scenario: The primitive set is unchanged by the bottom type
- **WHEN** an editor requests primitive type completions, or syntax highlighting classifies a name
  in type position
- **THEN** `never` SHALL NOT be among the names offered or highlighted

### Requirement: The unit type is inference-internal and has no source spelling
The system SHALL retain a unit type that type inference assigns where an expression produces no
meaningful value — an `if` with no `else`, a block with no trailing expression, and a match that may
match nothing. That type SHALL NOT be nameable in NX source, and its absence from the primitive set
SHALL be a property of the language rather than an omission from the list.

The system SHALL continue to render the unit type as `void` in diagnostics and in any other output
that names a type it inferred. Rendering a name an author cannot write is deliberate: the author
receives the name, they do not supply it.

No NX source construct requires the unit type to be written. Functions are expression-bodied, so a
function always produces a value, and NX has no expression that fails to produce one.

#### Scenario: A no-else conditional still has the unit type
- **WHEN** type inference analyzes an `if` expression with no `else` branch
- **THEN** the expression SHALL take the unit type
- **AND** removing `void` from the primitive set SHALL NOT change that

#### Scenario: A diagnostic may name the unit type
- **WHEN** the system reports a type mismatch whose found type is the unit type
- **THEN** the diagnostic SHALL render that type as `void`

#### Scenario: The unit type cannot be written in source
- **WHEN** a file annotates a binding or a function return as `void` and no type named `void` is
  declared
- **THEN** analysis SHALL NOT resolve the annotation to the unit type
- **AND** SHALL treat it as a reference to an undeclared name, exactly as it treats any other name
  it cannot resolve
