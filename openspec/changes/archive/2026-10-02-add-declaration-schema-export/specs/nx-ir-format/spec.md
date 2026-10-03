## MODIFIED Requirements

### Requirement: An artifact carries one module and names the modules it links against
An NX IR artifact SHALL carry exactly one module. It SHALL carry a module table whose first entry
describes the artifact's own module and whose remaining entries describe every module the artifact
references, each entry giving the module's logical identity, the version string the build was given
for it (empty when none was given) and a fingerprint of its source. A program that spans several
modules SHALL be emitted as one artifact per module, and a build SHALL let the caller choose which
modules to emit.

The entry module's artifact SHALL also list, after the modules it references and in the program's
module order, every module of the program that declares a concrete record, or a union, extending an
abstract record that the parameter or result type of a function reaches, through fields, union
cases, type aliases, type arguments and the shapes extending an abstract record. Only the functions
of the modules the entry reaches through its imports, directly or transitively, count, since no
other function is linked from the entry; the declaring modules may be any of the program's. A
host may name any such shape by `$type` in a value it supplies at that site, and a runtime resolves
a shape only from a module it linked; listing the declaring modules in the entry makes a program
linked from its entry resolve every one, including a shape declared in a module nothing references.
No other artifact SHALL list a module it does not reference, so a library module's artifact stays
the same whichever program emitted it.

#### Scenario: A snippet compiled against a catalog names the catalog
- **WHEN** a workspace holds `drawnui` with version `9` and `input.nx`, and `input.nx` uses a control
  `drawnui` declares
- **AND** the caller emits an artifact for `input.nx` only
- **THEN** the build SHALL return one artifact
- **AND** its module table SHALL list `input.nx` first and `drawnui` with version `9` second
- **AND** the artifact SHALL contain no declaration from `drawnui`

#### Scenario: A module nothing references is not in the table
- **WHEN** a workspace holds a module the entry never imports, directly or transitively, and that
  declares no shape extending an abstract record a function of the program takes or returns
- **THEN** the entry's artifact SHALL NOT list that module

#### Scenario: A catalog emits as its own artifact
- **WHEN** the caller emits an artifact for `drawnui` alone
- **THEN** the artifact's module table SHALL hold only `drawnui`
- **AND** it SHALL carry every external component, union and record `drawnui` declares

#### Scenario: A function nothing imports adds nothing to the entry
- **WHEN** the entry `main.nx` imports nothing, and `tools.nx`, which nothing imports, declares
  `let use(s:Base): int` over an abstract `Base` that `base.nx` extends
- **THEN** the entry's module table SHALL hold only `main.nx`

#### Scenario: The entry lists the modules that declare a function's subtypes
- **WHEN** the entry `main.nx` declares `let f(s:Base): string`, `base.nx` declares the abstract
  `Base` and `type A extends Base`, `x.nx` declares `type X extends Base` and is imported by
  `main.nx` but never referenced, and `y.nx` declares `type Y extends Base` and is imported by
  nothing
- **THEN** the entry's module table SHALL be `main.nx`, `base.nx`, `x.nx`, `y.nx`
- **AND** the artifact of `x.nx` SHALL list only `x.nx` and `base.nx`
- **AND** a runtime that links the program from `main.nx` SHALL accept `{ "$type": "X", ... }` and
  `{ "$type": "Y", ... }` as the argument `s`
