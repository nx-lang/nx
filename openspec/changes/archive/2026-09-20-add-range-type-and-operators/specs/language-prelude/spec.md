## Purpose

Gives NX one place where declarations every program can use are written in NX itself: a prelude
module that is part of every build and in scope in every module, so that a built-in such as `Range`
is an ordinary declaration rather than a name the compiler special-cases.

## ADDED Requirements

### Requirement: The prelude is part of every build
The system SHALL include a prelude module, written in NX source and carried inside the compiler, in
every analysis and build: a single-source build, a workspace build or validation, a library build, a
language-service snapshot, and every SDK entry point that reaches one of these. No caller SHALL have
to supply, name or enable it, and no caller SHALL be able to omit it. The prelude's exported
declarations SHALL be in scope in every other module without an import, in each namespace the
declaration occupies. The prelude SHALL be analyzed once per process and SHALL itself be free of
diagnostics.

#### Scenario: A single source uses a prelude declaration with no import
- **WHEN** a single-source build is given `let r:<Range T=int/> = <Range T=int start={1} end={5} endInclusive={false} />`
- **THEN** analysis SHALL accept the binding

#### Scenario: Every workspace module sees the prelude
- **WHEN** a workspace of two modules is built with no implicit imports and each module constructs a `Range`
- **THEN** analysis SHALL accept both modules

#### Scenario: A library module sees the prelude
- **WHEN** a library directory contains a module declaring `export type Slider = { range:<Range T=float64/> }`
- **THEN** the library SHALL build without diagnostics
- **AND** a program importing the library SHALL read `Slider.range` as `<Range T=float64/>`

#### Scenario: An editor snapshot offers prelude declarations
- **WHEN** a language-service snapshot holds one document and a completion is requested in a type position
- **THEN** the completions SHALL include `Range`
- **AND** a hover on a use of `Range` SHALL label it a built-in type and SHALL show its declaration with its type parameter and fields

### Requirement: Prelude names bind last and never conflict
A prelude declaration SHALL be bound in a module only under a name that the module has not
otherwise bound: a declaration of the module's own, a name from a written import of any form, and a
name from a host's implicit import SHALL each take the name instead, with no diagnostic. The
prelude SHALL NOT cause an ambiguous-import or duplicate-import diagnostic in any module. Where a
module's own name hides a prelude declaration, every use of that name in the module SHALL mean the
module's own.

#### Scenario: A local declaration hides the prelude's
- **WHEN** a file contains `type Range = { low:int high:int }` and `let r = <Range low={1} high={5} />`
- **THEN** analysis SHALL accept the file with `r` typed as the file's own `Range`

#### Scenario: A wildcard import hides the prelude's without ambiguity
- **WHEN** a workspace module `shapes.nx` declares `export type Range = { low:int high:int }` and another module contains `import "./shapes"` and `let r = <Range low={1} high={5} />`
- **THEN** analysis SHALL accept the module with `Range` meaning the declaration in `shapes.nx`
- **AND** SHALL NOT report that `Range` is provided by two modules

#### Scenario: A generic record declared before the prelude existed still checks
- **WHEN** a file contains `type Range = { T:type start:T end:T }` and `let r:<Range T=int/> = <Range T=int start={1} end={5} />`
- **THEN** analysis SHALL accept the file exactly as it did before the prelude declared `Range`

### Requirement: The prelude does not change a module's own text, exports or identity
A module's diagnostics and positions SHALL be those of the text the caller supplied, unchanged by
the prelude's presence. A library's exports and interface SHALL consist of its own declarations
only and SHALL NOT include a prelude declaration. The prelude SHALL have the reserved module identity
`@nx/prelude.nx`, a workspace that supplies a module under that identity SHALL be refused with a
diagnostic naming it, a diagnostic label that points into the prelude SHALL carry that identity,
and rendering such a diagnostic SHALL show the prelude's own source and SHALL NOT read a file from
disk. A program's fingerprint SHALL depend on the prelude's
source, so that two compilers with different preludes do not share a cached artifact.

#### Scenario: Positions are the document's own
- **WHEN** a single-source build is given a document whose third line has a type error
- **THEN** the diagnostic SHALL name the document and its third line

#### Scenario: A library does not export the prelude
- **WHEN** a library directory contains one module declaring `export type Slider = { range:<Range T=int/> }`
- **THEN** the library's exports SHALL be `Slider` and its derived companions and SHALL NOT include `Range`

#### Scenario: A workspace cannot supply the prelude's identity
- **WHEN** a workspace contains a module whose identity is `@nx/prelude.nx`
- **THEN** the build SHALL fail with a diagnostic saying the identity is reserved for the prelude

#### Scenario: A label inside the prelude names the prelude
- **WHEN** a diagnostic in a user module carries a secondary label at the declaration of `Range.start`
- **THEN** that label's file SHALL be the prelude's reserved identity
- **AND** rendering the diagnostic SHALL quote the prelude's source line

### Requirement: The prelude declares `Range`
The prelude SHALL export the generic record
`type Range = { T:type start:T end:T endInclusive:boolean }`, with no default on any field. It is an
ordinary generic record in every respect: it is constructed and applied as
`record-type-parameters` specifies, it has the derived companions `Range.Update` and
`Range.Property`, and its type parameter is unconstrained, so a `Range` of any type can be
constructed with the element syntax.

#### Scenario: `Range` is constructed and read like any record
- **WHEN** a file contains `let r = <Range T=string start="a" end="f" endInclusive={true} />` and `let first:string = r.start` and `let closed:boolean = r.endInclusive`
- **THEN** analysis SHALL accept all three bindings

#### Scenario: `Range` requires every field
- **WHEN** a file contains `let r = <Range T=int start={1} end={5} />`
- **THEN** analysis SHALL reject the element because `endInclusive` is missing

#### Scenario: `Range` has its companions
- **WHEN** a file contains `let r = <Range T=int start={1} end={5} endInclusive={false} />` and `let longer:<Range T=int/> = apply(r, <Range.Update T=int end={9} />)` and `let k:Range.Property = {Range.Property.end}`
- **THEN** analysis SHALL accept the bindings
