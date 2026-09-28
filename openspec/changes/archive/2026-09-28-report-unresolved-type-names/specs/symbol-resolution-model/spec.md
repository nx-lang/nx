## ADDED Requirements

### Requirement: A written type reference resolves to a visible type
Analysis SHALL report an `unresolved-type` error for every type reference written in a module whose
name does not resolve, in that module's namespace, to a visible type: a primitive; a record, union,
alias, component or action declared in or imported into the module; a built-in or prelude type; a type
parameter in scope; or a derived companion (`<Name>.Property`, `<Name>.Update`) of a visible
declaration. The check SHALL apply to record, union-case and action fields, component props, state
fields and emits, function parameters and return types, `let` annotations, type aliases and function
types. The diagnostic SHALL label the reference, name it, and suggest the closest visible type name
when one is close enough to be a misspelling of it. A reference SHALL be reported only by the module
that wrote it, and only once. An `extends` clause and a type argument that name no visible type SHALL
keep their own diagnostics (`lowering-error` naming the declaration that extends, and
`unresolved-type-argument`), without an `unresolved-type` in addition. A type that did not parse, and
a name that an import would have made visible when the import failed to resolve or its target module
lost a declaration to a syntax error, SHALL NOT be reported as `unresolved-type`: the syntax error and
the import's own diagnostic are the reports. An error in the target that leaves every declaration in
place SHALL NOT keep the importer's own unresolved names from being reported.

#### Scenario: An unresolved field type is rejected
- **WHEN** a file contains `type Broken = { id: MissingType }`
- **THEN** analysis SHALL report `unresolved-type` naming `MissingType` with a label on the reference

#### Scenario: A return type is labelled where it is written
- **WHEN** a file contains `let f(x:string): MissingRet = { x }`
- **THEN** analysis SHALL report `unresolved-type` naming `MissingRet`
- **AND** its label SHALL cover `MissingRet`, not the parameters or the body

#### Scenario: An unresolved parameter type is rejected
- **WHEN** a file contains `let f(x: MissingType) = { x }`
- **THEN** analysis SHALL report `unresolved-type` naming `MissingType`

#### Scenario: A misspelled type suggests the visible name
- **WHEN** a file contains `type Contact = { name:string }` and `let people: Contatc+ = {}`
- **THEN** analysis SHALL report `unresolved-type` naming `Contatc`
- **AND** the message SHALL suggest `Contact`

#### Scenario: Visible names of every kind resolve
- **WHEN** a module refers to a primitive, an imported record, a union, an alias, a type parameter
  `T` of its own generic record, `Range`, and `Contact.Property`
- **THEN** analysis SHALL report no `unresolved-type` diagnostic

#### Scenario: A library naming an unresolved type does not load
- **WHEN** a library module contains `export type Broken = { id: MissingType }`
- **THEN** loading the library SHALL fail with the `unresolved-type` diagnostic against that module
- **AND** a consumer of a library that loaded SHALL NOT report `unresolved-type` for the library's own
  references

#### Scenario: An unresolved base or type argument is reported once
- **WHEN** a file contains `type User extends Bse = { name:string }`, or a type argument
  `<Box T=Contatc/>` for a generic record `Box`
- **THEN** analysis SHALL report the existing base-resolution error or `unresolved-type-argument`
- **AND** SHALL NOT also report `unresolved-type` for the same name

#### Scenario: A type that did not parse is reported only as a syntax error
- **WHEN** a file contains `type A = { n: }`
- **THEN** analysis SHALL report the syntax error
- **AND** SHALL NOT report `unresolved-type`

#### Scenario: A failed import's names are not reported again
- **WHEN** a file contains `import { Contact } from "../missing/types.nx"` and uses `Contact` as a
  type, and the imported module does not exist
- **THEN** analysis SHALL report that the import does not resolve
- **AND** SHALL NOT report `unresolved-type` for `Contact`

#### Scenario: An import of a module with a syntax error reports only that error
- **WHEN** a workspace module `shared/bad.nx` has a syntax error in its declaration of `Contact`, and
  another module contains `import "../shared/bad.nx"` and uses `Contact` as a type
- **THEN** analysis SHALL report the syntax error in `shared/bad.nx`
- **AND** SHALL NOT report `unresolved-type` for `Contact` in the importing module

#### Scenario: A declaration swallowed by an unclosed one is lost too
- **WHEN** a workspace module `shared/t.nx` contains `export type Contact = { name:string` (unclosed)
  followed by `export type B = { y:int }`, and another module contains `import "../shared/t.nx"`
  and uses `B` as a type
- **THEN** analysis SHALL report the syntax error in `shared/t.nx`
- **AND** SHALL NOT report `unresolved-type` for `B` in the importing module

#### Scenario: An error that leaves every declaration in place hides nothing
- **WHEN** a workspace module `shared/t.nx` contains `export type Contact = { name:string tags:string[] }`,
  and another module contains `import "../shared/t.nx"` and uses `Contact` and `Typo` as types
- **THEN** analysis SHALL report the validation error in `shared/t.nx`
- **AND** SHALL report `unresolved-type` for `Typo` in the importing module

#### Scenario: A union case is not a type
- **WHEN** a file contains `type Shape = | circle { r:int } | dot` and `let f(s: Shape.circle) = { s }`
- **THEN** analysis SHALL report `unresolved-type` naming `Shape.circle`
- **AND** the message SHALL name `Shape` as the type to write
