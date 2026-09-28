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
- **AND** SHALL resolve the name by exactly the rules it applies to any other undeclared name, so it
  SHALL report `unresolved-type` naming `void` at the reference
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
