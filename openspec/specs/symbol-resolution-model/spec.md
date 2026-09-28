# symbol-resolution-model Specification

## Purpose
Defines the prepared visible namespace, stable definition identities, and binding-based lookup
contracts shared by analysis and runtime assembly in NX.

## Requirements

### Requirement: Prepared modules separate raw definitions from visible bindings
The system SHALL represent prepared analysis state separately from the raw file-local
`LoweredModule`. A prepared module SHALL preserve the analyzing file's raw definitions and SHALL
also expose a prepared visible namespace for the names visible from that file. Constructing that
visible namespace SHALL NOT require foreign peer or imported definitions to be copied into the raw
stored module artifact.

#### Scenario: Same-library peer visibility does not require cloning peer HIR into the raw module
- **WHEN** `helpers.nx` in one library declares `let answer() = 42`
- **AND** `main.nx` in that same library references `answer()`
- **THEN** prepared analysis for `main.nx` SHALL make `answer` visible from `helpers.nx`
- **AND** the stored raw `LoweredModule` for `main.nx` SHALL remain a file-local artifact for
  `main.nx` only

#### Scenario: Imported visibility uses interface metadata rather than foreign raw HIR
- **WHEN** a host analyzes `app/main.nx` containing `import "../ui"` and `let root() = Button`
- **AND** the supplied `ProgramBuildContext` exposes a loaded `../ui` library that exports `Button`
- **THEN** prepared analysis for `app/main.nx` SHALL make `Button` visible through imported library
  interface metadata
- **AND** the stored raw `LoweredModule` for `app/main.nx` SHALL NOT require copied raw HIR items
  from `../ui`

### Requirement: Visible names resolve to stable binding targets
Each prepared visible top-level name SHALL resolve to a stable binding target that identifies the
kind of symbol and the owning definition or interface origin. The prepared binding model SHALL
distinguish between local definitions, same-library peer definitions, and imported library
interface definitions.

#### Scenario: Local declaration resolves to a local binding target
- **WHEN** a file declares `type Theme = string`
- **THEN** prepared analysis for that file SHALL resolve `Theme` to a local type binding target
- **AND** that binding target SHALL identify the owning definition within the raw file-local module

#### Scenario: Imported declaration resolves to an imported binding target
- **WHEN** a host analyzes `app/main.nx` containing `import "../ui"` and references `Button`
- **AND** the supplied `ProgramBuildContext` exposes a loaded `../ui` that exports `Button`
- **THEN** prepared analysis for `app/main.nx` SHALL resolve `Button` to an imported binding target
- **AND** that binding target SHALL identify the owning imported library interface definition

### Requirement: Lexical scopes layer over prepared top-level bindings
The system SHALL resolve lexical bindings such as parameters, `let` bindings, and loop bindings
before falling back to prepared top-level visible bindings. A local lexical binding SHALL shadow a
prepared top-level binding with the same name.

#### Scenario: Function parameter shadows an imported top-level name
- **WHEN** a file imports `../ui` that exports `Button`
- **AND** the file declares `let render(Button:string) = Button`
- **THEN** the function body SHALL resolve `Button` to the parameter binding
- **AND** the imported top-level `Button` SHALL remain available only when not shadowed lexically

#### Scenario: Undefined lexical name falls back to prepared top-level visibility
- **WHEN** `helpers.nx` in one library declares `let answer() = 42`
- **AND** `main.nx` in that same library declares `let root() = answer()`
- **THEN** `root()` SHALL resolve `answer` through the prepared top-level binding visible from
  `helpers.nx`

### Requirement: Program and runtime lookup preserve module-qualified definition identity
When the system builds a `ProgramArtifact` and `ResolvedProgram`, runtime-visible item lookup SHALL
preserve the owning module identity and the stable local definition identity of the target
declaration. Runtime lookup SHALL NOT need to rediscover the target declaration by rescanning the
owning module for the visible string name.

#### Scenario: Imported function lookup preserves the exact owning definition
- **WHEN** a root source file imports `../math` and calls exported function `answer()`
- **THEN** the resulting `ResolvedProgram` SHALL record a module-qualified reference to the exact
  `answer` definition in the owning imported module
- **AND** runtime execution SHALL use that module-qualified definition reference to execute the call

#### Scenario: Runtime entry lookup preserves the exact local root definition
- **WHEN** a root source file defines `let root() = 42`
- **THEN** the resulting `ResolvedProgram` SHALL record a module-qualified reference to the exact
  local `root` definition in that file
- **AND** runtime execution SHALL use that module-qualified definition reference rather than
  rediscovering `root` by name in the module at execution time

### Requirement: Definitions are reachable by canonical identity as well as by visible name
The system SHALL support resolving a definition by its declaring origin without requiring a visible
name for it in the module performing the resolution. Lookup by visible name SHALL remain available
for names an author writes, and SHALL NOT be the only means by which analysis, code generation, or
evaluation reaches a definition.

#### Scenario: A definition resolves through an origin its module cannot name
- **WHEN** a lowered reference carries the declaring origin of a definition that the using module
  neither declares nor imports
- **THEN** code generation and evaluation SHALL resolve that reference to the declaration
- **AND** resolution SHALL NOT depend on a binding for that name being visible in the using module

#### Scenario: A visible name resolving to a different definition does not capture the reference
- **WHEN** the using module has a visible binding whose name matches the declared name carried by a
  reference to a different definition
- **THEN** the reference SHALL resolve to the definition its origin names
- **AND** the visible binding SHALL NOT be substituted for it

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
