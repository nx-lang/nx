## ADDED Requirements

### Requirement: Generated contracts refer to prelude types without redeclaring them per module
When an exported contract references a prelude declaration — the record, or one of its derived
companions — the C# `typegen` emitter SHALL render it as the hand-written SDK type for that
declaration, named by the `Nx` prefix over the name typegen gives it: `global::NxLang.Nx.NxRange<…>`
for `Range`, `global::NxLang.Nx.NxRange_update<…>` for `Range.Update`, and
`global::NxLang.Nx.NxRange_property` for `Range.Property`. Their type arguments SHALL be mapped as
`typegen` maps them anywhere else, and no declaration of a prelude type SHALL be emitted into the
generated namespace. The TypeScript emitter SHALL emit each prelude declaration the output
references, generated from the prelude's own declaration through the ordinary emitters, exactly once
per generation: into the shared helper module beside `NxRecord` for library output, and inline for
single-file output, and SHALL emit none when the output references none. A module that declares its
own type under a prelude name SHALL have that name, and its companions' names, rendered as its own
generated types rather than the prelude's. Resolution SHALL be per module: a declaration of the
generating library hides a prelude name in every one of its modules, because a same-library peer is
visible without an import, while an `import` hides it only in the module that wrote it.

#### Scenario: C# maps a range to the SDK type
- **WHEN** NX source declares `export type Slider = { range:<Range T=float64/> marks:<Range T=int/>[]? }`
- **AND** a caller requests C# output
- **THEN** the generated `Slider` SHALL declare `Range` as `global::NxLang.Nx.NxRange<double>` and `Marks` as a nullable list of `global::NxLang.Nx.NxRange<long>`
- **AND** the output SHALL NOT declare a type named `Range` or `NxRange`

#### Scenario: TypeScript library output declares `Range` once in the helper module
- **WHEN** a library has two modules that each export a record with a field typed `<Range T=int/>`
- **AND** a caller requests TypeScript output
- **THEN** the helper module SHALL export `Range<T>` with `start` and `end` typed `T` and `endInclusive` typed `boolean`
- **AND** each generated module SHALL import `Range` from the helper module and type its field `Range<number>`

#### Scenario: TypeScript single-file output inlines `Range`
- **WHEN** a single NX file exports a record with a field typed `<Range T=float64/>` and a caller requests TypeScript output
- **THEN** the one generated file SHALL declare `Range<T>` and type the field `Range<number>`

#### Scenario: Output that uses no prelude type is unchanged
- **WHEN** NX source references no prelude type
- **THEN** the generated C# and TypeScript SHALL be what they were before the prelude existed

#### Scenario: A module's own `Range` is generated as its own
- **WHEN** NX source declares `export type Range = { low:int high:int }` and `export type Chart = { bounds:Range }`
- **THEN** both languages SHALL generate that `Range` record and type `bounds` with it

#### Scenario: A contract field typed by a prelude companion
- **WHEN** NX source declares `export type Patch = { change:<Range.Update T=int/> which:Range.Property }`
- **THEN** the generated TypeScript SHALL type `change` as `Range_update<number>` and `which` as `Range_property`, and SHALL declare both beside `Range`
- **AND** the generated C# SHALL type `Change` as `global::NxLang.Nx.NxRange_update<long>` and `Which` as `global::NxLang.Nx.NxRange_property`
- **AND** neither output SHALL warn that a companion has nowhere to resolve to

#### Scenario: A module that imports a foreign `Range` leaves its sibling the prelude's
- **WHEN** a library has one module that imports a dependency's `Range` and a sibling module that declares a field typed `<Range T=int/>` with no import
- **THEN** the importing module's field SHALL be the dependency's generated type
- **AND** the sibling's field SHALL be the prelude's: `global::NxLang.Nx.NxRange<long>` in C#, and the helper module's `Range<number>` in TypeScript

### Requirement: A C# member typed by a generic update companion names a closed formatter
MessagePack's source generator cannot resolve the open generic a generic update companion carries,
so the C# emitter SHALL declare a closed formatter for each instantiation of such a companion that a
member's own type is, and the member SHALL name it with `[MessagePackFormatter]`. A formatter SHALL
be declared once per generated namespace, not once per module: single-file output declares it beside
the contracts, and library output — whose modules all land in the one namespace whatever their
directory — declares every formatter the library names in one shared file, since a second
declaration of the same class in a namespace does not compile. This SHALL apply only to a companion
another assembly declares — the prelude's, or a dependency's. A companion the *generating library*
declares needs no member attribute at all and SHALL NOT be given one: the source generator has the
declaration in the compilation and closes the companion's own open generic shim per instantiation,
wherever in the member's type it sits. A generated C# file SHALL suppress `MsgPack009`, which counts
formatters against the open generic and so reads two instantiations of one companion as two
formatters for one type; they are not, since the source generator resolves each member by the closed
type its attribute names. One shape has no formatter the generator can resolve: a companion another
assembly declares, reached *below* the member's own type, where there is nowhere to put the
attribute and the open generic on the companion is `MsgPack006` from the compiler itself, which no
file-level pragma reaches. A field of that shape SHALL be reported as a warning naming the field and
the companion, rather than generated into a file that does not compile.

#### Scenario: A member typed by the prelude's update companion
- **WHEN** NX source declares `export type Patch = { change:<Range.Update T=int/> }` and a caller requests C# output
- **THEN** the output SHALL declare a closed formatter over `global::NxLang.Nx.NxRange_update<long>`
- **AND** the `Change` member SHALL carry `[MessagePackFormatter]` naming it

#### Scenario: Two instantiations of one companion in one contract
- **WHEN** NX source declares `export type Patch = { narrow:<Range.Update T=int/> wide:<Range.Update T=float64/> }` and a caller requests C# output
- **THEN** the output SHALL declare a closed formatter for each instantiation, and each member SHALL name its own
- **AND** the output SHALL compile and round-trip both members, with no warning from the generator

#### Scenario: Two modules of a library name one instantiation
- **WHEN** a library has two modules that each declare a member typed `<Range.Update T=int/>` and a caller requests C# output
- **THEN** the output SHALL declare the closed formatter exactly once, in a shared file of its own
- **AND** both members SHALL carry `[MessagePackFormatter]` naming it

#### Scenario: A generic update companion C# cannot format
- **WHEN** NX source declares a field typed as a list of the prelude's `Range.Update`
- **THEN** C# output SHALL warn, naming the field and the companion
- **AND** a field typed by the update companion of a generic record the library declares itself SHALL NOT warn, at the member's own type or below it
