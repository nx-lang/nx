## MODIFIED Requirements

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

## ADDED Requirements

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
