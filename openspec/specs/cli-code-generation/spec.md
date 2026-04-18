# cli-code-generation Specification

## Purpose
Define the `nxlang generate` CLI behavior for single-file and library code generation while
honoring NX export visibility.
## Requirements
### Requirement: `generate` infers file versus library generation from the input path
The `nxlang generate` command SHALL inspect the input path and select generation behavior from the
filesystem entry kind. A `.nx` file SHALL trigger single-file generation. A directory SHALL trigger
library generation. Any other input kind or unsupported file extension MUST be rejected.

#### Scenario: NX file input triggers single-file generation
- **WHEN** a user runs `nxlang generate ./models/user.nx --language typescript`
- **THEN** the CLI SHALL treat `user.nx` as a single source module input

#### Scenario: Directory input triggers library generation
- **WHEN** a user runs `nxlang generate ./question-flow --language csharp --output ./generated`
- **THEN** the CLI SHALL treat `question-flow` as a library input rather than as a source file

#### Scenario: Non-NX file input is rejected
- **WHEN** a user runs `nxlang generate ./README.md --language typescript`
- **THEN** the CLI SHALL report an error instead of attempting code generation

### Requirement: Single-file generation emits only exported type declarations
When `nxlang generate` targets a single `.nx` file, the generated output SHALL include source
declarations marked `export` in that file plus companion state contracts synthesized from any
exported external components in that file that declare state. The generated type surface SHALL
cover exported type aliases, exported enums, exported record-like declarations, exported action
records, and generated external-component state contracts.

#### Scenario: Internal and private declarations are omitted from file generation
- **WHEN** `types.nx` contains `private type Hidden = string`, `type InternalThing = string`, and
  `export type PublicThing = string`
- **THEN** generated output SHALL include `PublicThing` only

#### Scenario: Exported alias is generated for TypeScript
- **WHEN** `types.nx` contains `export type Theme = string`
- **THEN** TypeScript generation SHALL emit a corresponding exported type alias for `Theme`

#### Scenario: Exported action record is generated
- **WHEN** `actions.nx` contains `export action SearchRequested = { query:string }`
- **THEN** generated output SHALL include a generated type for `SearchRequested`

#### Scenario: Exported external component state contract is generated
- **WHEN** `components.nx` contains `export external component <SearchBox placeholder:string /> = { state { query:string } }`
- **THEN** generated output SHALL include a generated type `SearchBox_state`
- **AND** SHALL include field `query`

### Requirement: Library generation emits the exported type surface of the full library
When `nxlang generate` targets a directory, the CLI SHALL analyze that directory as an NX library
and SHALL generate code from every library module that contributes exported type declarations or
exported external-component state contracts. The command MUST reject a directory that cannot be
analyzed as a valid NX library.

#### Scenario: Exported declarations from multiple files are generated together
- **WHEN** library `./ui` contains `button.nx` with `export type ButtonSize = string` and
  `theme.nx` with `export enum ThemeMode = | light | dark`
- **THEN** library generation SHALL include generated output for both `ButtonSize` and `ThemeMode`

#### Scenario: Exported external component state from a library module is generated
- **WHEN** library `./ui` contains `search-box.nx` with `export external component <SearchBox /> = { state { query:string } }`
- **THEN** library generation SHALL include generated output for `SearchBox_state`

#### Scenario: Non-export library declarations are omitted
- **WHEN** library `./ui` contains `private type Hidden = string`, `type InternalThing = string`,
  and `export type PublicThing = string`
- **THEN** library generation SHALL omit `Hidden` and `InternalThing` from the generated output

#### Scenario: Invalid library directory is rejected
- **WHEN** a user runs `nxlang generate ./empty-dir --language csharp --output ./generated`
- **THEN** the CLI SHALL report a library-analysis error if `empty-dir` is not a valid NX library

### Requirement: Library generation uses per-module multi-file output
When `nxlang generate` targets a directory, generated output SHALL be written as multiple files
using one generated file per contributing NX module. Library generation SHALL require `--output`,
and that output path SHALL be treated as a directory root.

#### Scenario: Library generation requires an output directory
- **WHEN** a user runs `nxlang generate ./ui --language typescript` without `--output`
- **THEN** the CLI SHALL report that library generation requires an output directory

#### Scenario: TypeScript library generation writes per-module files and a barrel
- **WHEN** library `./ui` contains exported types in `button.nx` and `theme.nx`
- **THEN** TypeScript generation SHALL write one generated `.ts` file for `button.nx`, one
  generated `.ts` file for `theme.nx`, and a root `index.ts` that re-exports those generated
  modules

#### Scenario: C# library generation writes per-module `.g.cs` files
- **WHEN** library `./ui` contains exported types in `button.nx` and `theme.nx`
- **THEN** C# generation SHALL write one generated `.g.cs` file per contributing NX module under
  the chosen output directory

### Requirement: Generated library files preserve cross-module type references
Generated library files SHALL preserve cross-module type references for any generated declaration,
including an external-component state companion contract. When a generated declaration references
an exported type owned by another generated module in the same library output, the generated files
SHALL include whatever language-specific linkage is needed to keep the generated output coherent.

#### Scenario: TypeScript emits relative imports for cross-module references
- **WHEN** generated file `forms.ts` contains a declaration referencing exported type `ThemeMode`
  owned by generated file `theme.ts`
- **THEN** `forms.ts` SHALL include a relative `import type` for `ThemeMode` from `theme.ts`

#### Scenario: TypeScript emits relative imports for external component state contracts
- **WHEN** library module `theme.nx` exports `enum ThemeMode = | light | dark`
- **AND** library module `search-box.nx` exports `external component <SearchBox /> = { state { theme:ThemeMode } }`
- **THEN** generated file `search-box.ts` SHALL include a relative `import type` for `ThemeMode`
  from `theme.ts`
- **AND** SHALL include generated type `SearchBox_state` that references `ThemeMode`

#### Scenario: C# cross-module references remain resolvable
- **WHEN** one generated `.g.cs` file references a generated type declared in another generated
  `.g.cs` file from the same library output
- **THEN** the generated C# output SHALL keep that reference resolvable without manual edits

### Requirement: TypeScript generated records preserve concrete runtime discriminators
TypeScript code generation SHALL emit record-like declarations that preserve the NX `$type` payload
discriminator. Every generated concrete record or action record SHALL include a `$type` property
whose type is the string literal of that declaration's exported name. When a concrete record or
action derives from an exported abstract base of the same family, the generated output SHALL
preserve the abstract base's shared fields through a reusable base contract while keeping each
concrete descendant discriminated by its own literal `$type`.

#### Scenario: Concrete record includes a literal `$type`
- **WHEN** source contains `export type ShortTextQuestion = { label:string }`
- **THEN** generated TypeScript SHALL include a `ShortTextQuestion` contract with
  `$type: "ShortTextQuestion"`

#### Scenario: Abstract record family exposes a shared base and concrete runtime surface
- **WHEN** source contains `export abstract type Question = { label:string } export type ShortTextQuestion extends Question = { placeholder:string? } export type LongTextQuestion extends Question = { wordLimit:int? }`
- **THEN** generated TypeScript SHALL preserve the shared `Question` fields in a generated base
  contract for descendants
- **AND** the generated `ShortTextQuestion` and `LongTextQuestion` contracts SHALL each include
  their own literal `$type`
- **AND** the exported `Question` type surface SHALL remain usable as the concrete runtime type for
  values of either descendant

#### Scenario: Cross-module abstract record family remains generated as a coherent TypeScript surface
- **WHEN** library module `questions/base.nx` exports `abstract type Question = { label:string }`
- **AND** library module `questions/short-text.nx` exports
  `type ShortTextQuestion extends Question = { placeholder:string? }`
- **THEN** library TypeScript generation SHALL emit any needed `import type` statements so the
  exported `Question` type surface in `questions/base.ts` can reference `ShortTextQuestion` without
  manual edits

#### Scenario: Exported action record includes a literal `$type`
- **WHEN** source contains `export action SearchRequested = { query:string }`
- **THEN** generated TypeScript SHALL include `$type: "SearchRequested"` on the generated
  `SearchRequested` contract

#### Scenario: Abstract action family exposes a shared base and concrete runtime surface
- **WHEN** source contains `export abstract action SearchAction = { source:string } export action SearchRequested extends SearchAction = { query:string } export action SearchSubmitted extends SearchAction = { submittedAt:string }`
- **THEN** generated TypeScript SHALL preserve the shared `SearchAction` fields in a generated base
  contract for descendants
- **AND** the generated `SearchRequested` and `SearchSubmitted` contracts SHALL each include their
  own literal `$type`
- **AND** the exported `SearchAction` type surface SHALL remain usable as the concrete runtime type
  for values of either descendant

#### Scenario: Cross-module abstract action family remains generated as a coherent TypeScript surface
- **WHEN** library module `actions/base.nx` exports `abstract action SearchAction = { source:string }`
- **AND** library module `actions/requested.nx` exports
  `action SearchRequested extends SearchAction = { query:string }`
- **THEN** library TypeScript generation SHALL emit any needed `import type` statements so the
  exported `SearchAction` type surface in `actions/base.ts` can reference `SearchRequested` without
  manual edits

### Requirement: C# generated records use serializer metadata without emitted discriminator members
C# code generation SHALL emit generated record and action DTO classes that can serialize and
deserialize with both MessagePack and `System.Text.Json` without declaring a generated data member
mapped to wire key `$type`. Generated data members SHALL preserve their NX wire names across both
serializers, generated abstract records and actions SHALL remain inheritable, abstract roots SHALL
advertise JSON polymorphism using `$type` and their concrete descendants, and MessagePack
polymorphism SHALL continue to use generated `[Union(...)]` metadata.

#### Scenario: Concrete C# record emits only declared fields
- **WHEN** source contains `export type ShortTextQuestion = { label:string }`
- **THEN** generated C# SHALL emit `ShortTextQuestion` without a generated member mapped to `$type`
- **AND** generated property `Label` SHALL be annotated so both serializers use wire name `label`

#### Scenario: Concrete C# action emits only declared fields
- **WHEN** source contains `export action SearchRequested = { query:string }`
- **THEN** generated C# SHALL emit `SearchRequested` without a generated member mapped to `$type`
- **AND** generated property `Query` SHALL be annotated so both serializers use wire name `query`

#### Scenario: Abstract C# record root advertises polymorphism without a discriminator member
- **WHEN** source contains `export abstract type Question = { label:string } export type ShortTextQuestion extends Question = { placeholder:string? }`
- **THEN** generated C# SHALL emit `Question` as an inheritable abstract generated record type
- **AND** `Question` SHALL advertise JSON polymorphism using `$type` and its concrete descendants
- **AND** generated `ShortTextQuestion` SHALL not declare a generated member mapped to `$type`

#### Scenario: Intermediate abstract C# records inherit the root metadata without redeclaring a member
- **WHEN** source contains `export abstract type Question = { label:string } export abstract type TextQuestion extends Question = { placeholder:string? } export type ShortTextQuestion extends TextQuestion = { maxLength:int? }`
- **THEN** the generated root abstract type SHALL advertise JSON polymorphism for its concrete
  descendants using `$type`
- **AND** intermediate abstract generated records SHALL inherit that metadata without redeclaring a
  generated member mapped to `$type`

#### Scenario: Abstract C# root without concrete descendants omits invalid polymorphism metadata and warns
- **WHEN** source contains `export abstract type Question = { label:string }`
- **THEN** generated C# SHALL emit `Question` without `[JsonPolymorphic]` or `[JsonDerivedType]`
  metadata
- **AND** generated C# SHALL include a comment explaining that no `JsonPolymorphic` metadata was
  generated because the abstract type had no concrete exported descendants at code-generation time
- **AND** the generator SHALL emit a warning that `Question` has no concrete exported descendants
  for C# polymorphic generation

#### Scenario: User field names do not collide with a synthetic discriminator member
- **WHEN** source contains `export type Payload = { nx_type:string }`
- **THEN** generated C# SHALL emit a property for wire name `nx_type`
- **AND** generated C# SHALL not emit any extra `__NxType` or `$type` data member on `Payload`

### Requirement: Generated C# enums use authored member strings across JSON and MessagePack
Generated C# enums SHALL preserve the authored NX enum member spellings across both
`System.Text.Json` and MessagePack. Generated C# enum properties and values SHALL serialize as the
plain authored enum member string rather than as the canonical raw `NxValue` enum map, and typed
generated enum deserialization SHALL use that same string form for both serializers.

#### Scenario: Generated C# JSON enum serialization uses the authored member string
- **WHEN** source contains `export enum DealStage = | draft | pending_review | closed_won`
- **THEN** generated C# SHALL include JSON enum serialization support that emits
  `"pending_review"` for `DealStage.PendingReview`
- **AND** SHALL NOT require a `"$enum"` or `"$member"` wrapper for the typed JSON enum value

#### Scenario: Generated C# MessagePack enum serialization uses the authored member string
- **WHEN** source contains `export enum DealStage = | draft | pending_review | closed_won`
- **THEN** generated C# SHALL include MessagePack enum serialization support that emits the string
  `pending_review`
- **AND** typed MessagePack enum handling SHALL use that string-based wire shape rather than the
  canonical raw enum map shape

### Requirement: Generated external component state contracts use stable companion names
Generated external component state contracts SHALL use stable companion names. When generated
output includes a companion state contract for an exported external component, the generator SHALL
name it `<ComponentName>_state`, SHALL include exactly the declared state fields from that
component, SHALL map those fields to the same wire names for both MessagePack and JSON, and SHALL
NOT include component props, emitted actions, or a `$type` discriminator.

#### Scenario: TypeScript companion state contract is a plain interface
- **WHEN** source contains `export external component <SearchBox /> = { state { query:string } }`
- **THEN** TypeScript generation SHALL emit `export interface SearchBox_state`
- **AND** SHALL include property `query: string`
- **AND** SHALL NOT emit `$type` on `SearchBox_state`

#### Scenario: C# companion state contract is a plain dual-annotated DTO
- **WHEN** source contains `export external component <SearchBox /> = { state { query:string } }`
- **THEN** C# generation SHALL emit a generated type `SearchBox_state`
- **AND** SHALL include the declared state field `query`
- **AND** SHALL annotate that field so both MessagePack and JSON use wire name `query`
- **AND** SHALL NOT emit a `$type` discriminator member on `SearchBox_state`

### Requirement: Generated external component state names warn and skip on collisions
The generator SHALL warn when a synthesized external-component state companion name would collide
with another generated declaration name or exported declaration name, and SHALL skip generation of
the synthesized companion instead of overwriting the conflicting declaration.

#### Scenario: Generated external state name collides with an exported declaration
- **WHEN** source contains `export type SearchBox_state = string` and `export external component <SearchBox /> = { state { query:string } }`
- **THEN** generation SHALL emit a warning about the `SearchBox_state` naming conflict
- **AND** SHALL omit the generated `SearchBox_state` companion contract
- **AND** SHALL preserve the explicit exported declaration `SearchBox_state`

### Requirement: Generated type surfaces preserve composed list and nullable type references
The generator SHALL preserve the same nested-list and nullability structure when exported aliases,
record-like fields, action fields, or generated external-component state contracts use composed NX
list and nullable suffixes. The generator SHALL continue to distinguish `T?[]` from `T[]?`
instead of normalizing them to the same target-language shape.

#### Scenario: TypeScript aliases preserve nested lists and nullable lists
- **WHEN** source contains `export type Matrix = string[][]` and `export type MaybeNames = string[]?`
- **THEN** TypeScript generation SHALL emit `export type Matrix = string[][];`
- **AND** SHALL emit `export type MaybeNames = string[] | null;`

#### Scenario: TypeScript fields preserve list-of-nullable elements
- **WHEN** source contains `export type Payload = { aliases:string?[] }`
- **THEN** generated TypeScript for `Payload` SHALL include field `aliases: (string | null)[]`

#### Scenario: C# fields preserve nested and outer nullable list structure
- **WHEN** source contains `export type Payload = { matrix:string[][] maybeNames:string[]? aliases:string?[] }`
- **THEN** generated C# for `Payload` SHALL include property `Matrix` with type `string[][]`
- **AND** SHALL include property `MaybeNames` with type `string[]?`
- **AND** SHALL include property `Aliases` with type `string?[]`
