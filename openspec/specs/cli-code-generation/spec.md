# cli-code-generation Specification

## Purpose
Define the `nxlang typegen` CLI behavior for single-file and library code generation while
honoring NX export visibility.
## Requirements
### Requirement: `typegen` infers file versus library generation from the input path
The `nxlang typegen` command SHALL inspect the input path and select generation behavior from the
filesystem entry kind. A `.nx` file SHALL trigger single-file generation. A directory SHALL trigger
library generation. Any other input kind or unsupported file extension MUST be rejected.

#### Scenario: NX file input triggers single-file generation
- **WHEN** a user runs `nxlang typegen ./models/user.nx --language typescript`
- **THEN** the CLI SHALL treat `user.nx` as a single source module input

#### Scenario: Directory input triggers library generation
- **WHEN** a user runs `nxlang typegen ./question-flow --language csharp --output ./generated`
- **THEN** the CLI SHALL treat `question-flow` as a library input rather than as a source file

#### Scenario: Non-NX file input is rejected
- **WHEN** a user runs `nxlang typegen ./README.md --language typescript`
- **THEN** the CLI SHALL report an error instead of attempting code generation

### Requirement: Single-file generation emits only exported type declarations
When `nxlang typegen` targets a single `.nx` file, the generated output SHALL include source
declarations marked `export` in that file plus companion state contracts synthesized from any
exported external components in that file that declare state. The generated type surface SHALL
cover exported type aliases, exported discriminated unions, exported record-like declarations,
exported action records, and generated external-component state contracts.

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
When `nxlang typegen` targets a directory, the CLI SHALL analyze that directory as an NX library
and SHALL generate code from every library module that contributes exported type declarations or
exported external-component state contracts. The command MUST reject a directory that cannot be
analyzed as a valid NX library.

#### Scenario: Exported declarations from multiple files are generated together
- **WHEN** library `./ui` contains `button.nx` with `export type ButtonSize = string` and
  `theme.nx` with `export type ThemeMode = light | dark`
- **THEN** library generation SHALL include generated output for both `ButtonSize` and `ThemeMode`

#### Scenario: Exported external component state from a library module is generated
- **WHEN** library `./ui` contains `search-box.nx` with `export external component <SearchBox /> = { state { query:string } }`
- **THEN** library generation SHALL include generated output for `SearchBox_state`

#### Scenario: Non-export library declarations are omitted
- **WHEN** library `./ui` contains `private type Hidden = string`, `type InternalThing = string`,
  and `export type PublicThing = string`
- **THEN** library generation SHALL omit `Hidden` and `InternalThing` from the generated output

#### Scenario: Invalid library directory is rejected
- **WHEN** a user runs `nxlang typegen ./empty-dir --language csharp --output ./generated`
- **THEN** the CLI SHALL report a library-analysis error if `empty-dir` is not a valid NX library

### Requirement: Library generation uses per-module multi-file output
When `nxlang typegen` targets a directory, generated output SHALL be written as multiple files
using one generated file per contributing NX module. Library generation SHALL require `--output`,
and that output path SHALL be treated as a directory root.

#### Scenario: Library generation requires an output directory
- **WHEN** a user runs `nxlang typegen ./ui --language typescript` without `--output`
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
When TypeScript generation references an exported type owned by an imported dependency library, the
generated file SHALL emit a type-only package import for that dependency type. TypeScript package
import targets SHALL be derived from the dependency library name and the optional
`--typescript-package-prefix` value until explicit package metadata exists.

#### Scenario: TypeScript emits relative imports for cross-module references
- **WHEN** generated file `forms.ts` contains a declaration referencing exported type `ThemeMode`
  owned by generated file `theme.ts`
- **THEN** `forms.ts` SHALL include a relative `import type` for `ThemeMode` from `theme.ts`

#### Scenario: TypeScript emits relative imports for external component state contracts
- **WHEN** library module `theme.nx` exports `type ThemeMode = light | dark`
- **AND** library module `search-box.nx` exports `external component <SearchBox /> = { state { theme:ThemeMode } }`
- **THEN** generated file `search-box.ts` SHALL include a relative `import type` for `ThemeMode`
  from `theme.ts`
- **AND** SHALL include generated type `SearchBox_state` that references `ThemeMode`

#### Scenario: TypeScript emits package imports for cross-library references
- **WHEN** library `chat-link` imports `../question-flow`
- **AND** `chat-link` exports `type QuestionFlowInitialExperience = { questionFlow:QuestionFlow }`
- **AND** `../question-flow` exports `type QuestionFlow = { id:string }`
- **AND** the user runs `nxlang typegen ./chat-link --language typescript --typescript-package-prefix @org/nx- --output ./generated`
- **THEN** generated file `QuestionFlowInitialExperience.ts` SHALL include
  `import type { QuestionFlow } from "@org/nx-question-flow";`
- **AND** the generated `questionFlow` field SHALL reference `QuestionFlow` without requiring a
  manual edit

#### Scenario: TypeScript warns for assumed dependency package target
- **WHEN** TypeScript generation emits an import for dependency library `../question-flow`
- **THEN** the generator SHALL emit a warning that the dependency import target is assumed from the
  dependency directory name
- **AND** the warning SHALL include the resolved package target that was emitted in generated source

#### Scenario: TypeScript aliases imported dependency names when local generated name differs
- **WHEN** a source module imports `QuestionFlow` as a visible qualified name that generates local
  TypeScript name `Flow_QuestionFlow`
- **AND** the referenced dependency exports the type as `QuestionFlow`
- **THEN** the generated TypeScript import SHALL alias the dependency export with
  `import type { QuestionFlow as Flow_QuestionFlow } from "<dependency-package>";`
- **AND** local generated type references SHALL use `Flow_QuestionFlow`

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
advertise polymorphism using `$type` and their concrete descendants for both serializers, and
MessagePack polymorphism SHALL use the same `$type`-keyed map contract as canonical `NxValue`.

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
- **AND** `Question` SHALL advertise polymorphism using `$type` and its concrete descendants
- **AND** generated `ShortTextQuestion` SHALL not declare a generated member mapped to `$type`

#### Scenario: Intermediate abstract C# records inherit the root metadata without redeclaring a member
- **WHEN** source contains `export abstract type Question = { label:string } export abstract type TextQuestion extends Question = { placeholder:string? } export type ShortTextQuestion extends TextQuestion = { maxLength:int? }`
- **THEN** the generated root abstract type SHALL advertise polymorphism for its concrete
  descendants using `$type`
- **AND** intermediate abstract generated records SHALL inherit that metadata without redeclaring a
  generated member mapped to `$type`

#### Scenario: Abstract C# root without concrete descendants omits invalid polymorphism metadata and warns
- **WHEN** source contains `export abstract type Question = { label:string }`
- **THEN** generated C# SHALL emit `Question` without `[JsonPolymorphic]`, `[JsonDerivedType]`, or
  polymorphic MessagePack formatter metadata (`NxPolymorphicMessagePackFormatter` /
  `NxPolymorphicConcreteMessagePackFormatter`)
- **AND** generated C# SHALL include a comment explaining that no polymorphism metadata (JSON or
  MessagePack) was generated because the abstract type had no concrete exported descendants at
  code-generation time
- **AND** the generator SHALL emit a warning that `Question` has no concrete exported descendants
  for C# polymorphic generation

#### Scenario: User field names do not collide with a synthetic discriminator member
- **WHEN** source contains `export type Payload = { nx_type:string }`
- **THEN** generated C# SHALL emit a property for wire name `nx_type`
- **AND** generated C# SHALL not emit any extra `__NxType` or `$type` data member on `Payload`

### Requirement: Generated C# enums use authored member strings across JSON and MessagePack
Generated C# enums SHALL preserve the authored NX case spellings across both `System.Text.Json` and
MessagePack. Generated C# enum properties and values SHALL serialize as the plain authored case
string rather than as a canonical raw `NxValue` map, and typed generated enum deserialization SHALL
use that same string form for both serializers.

A generated C# enum SHALL be produced for a constant union — one whose cases all declare no fields
and which declares no base. A union with any payload case SHALL NOT generate a C# enum.

#### Scenario: Generated C# JSON enum serialization uses the authored member string
- **WHEN** source contains `export type DealStage = draft | pending_review | closed_won`
- **THEN** generated C# SHALL include JSON enum serialization support that emits
  `"pending_review"` for `DealStage.PendingReview`
- **AND** SHALL NOT require a `"$enum"` or `"$member"` wrapper for the typed JSON value

#### Scenario: Generated C# MessagePack enum serialization uses the authored member string
- **WHEN** source contains `export type DealStage = draft | pending_review | closed_won`
- **THEN** generated C# SHALL include MessagePack enum serialization support that emits the string
  `pending_review`
- **AND** typed MessagePack handling SHALL use that string-based wire shape rather than a canonical
  raw map shape

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

### Requirement: Generated type surfaces preserve nullable list structure
The generator SHALL preserve the same nullability structure when exported aliases, record-like
fields, action fields, or generated external-component state contracts use composed NX list and
nullable suffixes. The generator SHALL continue to distinguish `T?[]` from `T[]?` instead of
normalizing them to the same target-language shape. Because an NX sequence never contains a
sequence, the generator SHALL NOT be asked to emit a nested list type, and SHALL NOT synthesize one.

#### Scenario: TypeScript aliases preserve nullable lists and lists of nullables
- **WHEN** source contains `export type MaybeNames = string[]?` and `export type Aliases = string?[]`
- **THEN** TypeScript generation SHALL emit `export type MaybeNames = string[] | null;`
- **AND** SHALL emit `export type Aliases = (string | null)[];`

#### Scenario: TypeScript fields preserve list-of-nullable elements
- **WHEN** source contains `export type Payload = { aliases:string?[] }`
- **THEN** generated TypeScript for `Payload` SHALL include field `aliases: (string | null)[]`

#### Scenario: C# fields preserve outer nullable list structure and nullable elements
- **WHEN** source contains `export type Payload = { names:string[] maybeNames:string[]? aliases:string?[] }`
- **THEN** generated C# for `Payload` SHALL include property `Names` with type `string[]`
- **AND** SHALL include property `MaybeNames` with type `string[]?`
- **AND** SHALL include property `Aliases` with type `string?[]`

### Requirement: Generated C# enums reuse shared runtime serialization helpers
Generated C# enums SHALL reference shared enum serialization helpers from `NxLang.Runtime` for both
`System.Text.Json` and MessagePack instead of emitting a dedicated converter and formatter type per
enum. Generated output SHALL continue to emit an enum-specific wire-format mapping type that
preserves the authored NX member strings explicitly.

#### Scenario: Generated C# enum references shared runtime helpers
- **WHEN** source contains `export type DealStage = draft | pending_review | closed_won`
- **THEN** generated C# SHALL include `using NxLang.Nx.Serialization;`
- **AND** SHALL annotate `DealStage` with `NxEnumJsonConverter<DealStage, DealStageWireFormat>`
- **AND** SHALL annotate `DealStage` with
  `NxEnumMessagePackFormatter<DealStage, DealStageWireFormat>`
- **AND** SHALL emit `DealStageWireFormat` with explicit mappings between
  `DealStage.PendingReview` and `"pending_review"`
- **AND** SHALL NOT emit dedicated `DealStageJsonConverter` or `DealStageMessagePackFormatter`
  types

#### Scenario: Generated C# enum mapping remains explicit when CLR names are normalized
- **WHEN** source contains `export type BuildTarget = web_api | ios_app`
- **THEN** generated C# SHALL emit CLR members `WebApi` and `IosApp`
- **AND** SHALL preserve the authored wire strings `"web_api"` and `"ios_app"` through the
  generated wire-format mapping type rather than inferring them from CLR member names at runtime

### Requirement: Generated TypeScript supports exported discriminated unions
TypeScript code generation SHALL include exported discriminated union declarations in single-file
and library generation. The generated TypeScript surface SHALL expose the source union name as a
closed union over its generated cases. Every generated case member SHALL carry a literal `$type`
property whose value is the fully scoped NX case name, SHALL include declared and inherited fields
with authored wire names, and SHALL allow TypeScript consumers to narrow by `$type`.

#### Scenario: TypeScript generation emits narrowable union cases
- **WHEN** source contains `export type LoadState = | idle | failed { message:string retryable:boolean = true }`
- **THEN** generated TypeScript SHALL include an exported `LoadState` type surface
- **AND** one generated case member SHALL have `$type: "LoadState.idle"`
- **AND** one generated case member SHALL have `$type: "LoadState.failed"` and field
  `message: string`
- **AND** TypeScript consumers SHALL be able to narrow `LoadState` by checking `$type`

#### Scenario: TypeScript generation includes shared inherited union fields
- **WHEN** source contains `export abstract type EventBase = { source:string } export type UiEvent extends EventBase = | clicked { x:int } | closed`
- **THEN** generated TypeScript SHALL include `source` on every generated `UiEvent` case member
- **AND** the exported `UiEvent` type surface SHALL remain narrowable by the case `$type`

#### Scenario: TypeScript library generation preserves cross-module field references
- **WHEN** library module `items.nx` exports `type Item = { name:string }`
- **AND** library module `state.nx` exports `type LoadState = | loaded { items:Item[] }`
- **THEN** TypeScript library generation SHALL emit any needed type-only imports so the generated
  `LoadState.loaded` case field can reference `Item`

### Requirement: Generated C# supports exported discriminated unions
C# code generation SHALL include exported discriminated union declarations in single-file and
library generation. The generated C# surface SHALL expose a root type for the source union and a
sealed generated DTO type for each case. The generated root and cases SHALL use the existing
`$type`-based JSON and MessagePack polymorphism support, with discriminator values equal to the
fully scoped NX case names. Generated case DTOs SHALL include declared and inherited fields with
authored NX wire names.

#### Scenario: C# generation emits polymorphic union root and cases
- **WHEN** source contains `export type LoadState = | idle | failed { message:string retryable:boolean = true }`
- **THEN** generated C# SHALL include a generated root type for `LoadState`
- **AND** generated C# SHALL include generated concrete case DTOs for `LoadState.idle` and
  `LoadState.failed`
- **AND** generated polymorphism metadata SHALL use `$type` values `LoadState.idle` and
  `LoadState.failed`
- **AND** the generated `LoadState.failed` DTO SHALL include property metadata for wire name
  `message`

#### Scenario: C# generation includes inherited union fields
- **WHEN** source contains `export abstract type EventBase = { source:string } export type UiEvent extends EventBase = | clicked { x:int } | closed`
- **THEN** generated C# SHALL expose `source` on every generated `UiEvent` case DTO through the
  generated inheritance or shared contract shape
- **AND** serializers SHALL write the field using wire name `source`

### Requirement: Generated C# DTO properties preserve supported literal defaults
C# type generation SHALL preserve authored NX literal defaults on generated DTO properties when the
literal can be represented as a C# property initializer. Supported literal defaults SHALL include
string, integer, floating-point, boolean, and null literals. When a generated C# property has a
supported literal default, that authored initializer SHALL take precedence over the generator's
non-null reference `default!` initializer. When a C# generated field has a non-literal default
expression, generation SHALL continue and SHALL emit a warning that the default could not be
preserved.

#### Scenario: Record field literal defaults are emitted as C# initializers
- **WHEN** source contains `export type Settings = { enabled:boolean = true count:int = 42 title:string = "hello" maybe:string? = null }`
- **THEN** generated C# SHALL include `public bool Enabled { get; set; } = true;`
- **AND** generated C# SHALL include `public long Count { get; set; } = 42;`
- **AND** generated C# SHALL include `public string Title { get; set; } = "hello";`
- **AND** generated C# SHALL include `public string? Maybe { get; set; } = null;`

#### Scenario: Union case field literal defaults are emitted as C# initializers
- **WHEN** source contains `export type LoadState = | failed { retryable:boolean = true }`
- **THEN** generated C# SHALL include generated case DTO `LoadStateFailed`
- **AND** generated C# SHALL include `public bool Retryable { get; set; } = true;`

#### Scenario: External component prop literal defaults are emitted as C# initializers
- **WHEN** source contains `export external component <Toggle selected:boolean = true label:string = "On" />`
- **THEN** generated C# SHALL include generated component prop DTO `Toggle`
- **AND** generated C# SHALL include `public bool Selected { get; set; } = true;`
- **AND** generated C# SHALL include `public string Label { get; set; } = "On";`

#### Scenario: Unsupported default expressions warn instead of silently changing semantics
- **WHEN** source contains `export type Settings = { enabled:boolean = { !false } }`
- **THEN** C# generation SHALL emit a warning that the default for `Settings.enabled` could not be preserved
- **AND** generated C# SHALL omit a property initializer for `Enabled`

### Requirement: CLI codegen writes NX IR artifacts
The `nxlang codegen` command SHALL support an explicit `nx-ir` executable target that builds a
`ProgramArtifact` through the existing source/workspace analysis pipeline and writes one
deterministic NX IR image per module of the program, each carrying its debug section, as
`<identity>.nxir`. NX IR output SHALL be host-neutral and SHALL NOT require a JavaScript or
TypeScript source-generation target. Each artifact's file name SHALL derive from its module
identity.

#### Scenario: Source file codegen writes NX IR
- **WHEN** a user runs `nxlang codegen ./app/main.nx --target nx-ir --output ./generated`
- **THEN** the CLI SHALL build a `ProgramArtifact` for `app/main.nx`
- **AND** it SHALL write `main.nxir` under `./generated`
- **AND** it SHALL NOT write JavaScript runtime helper files or generated JavaScript modules for
  that request

#### Scenario: Workspace codegen writes NX IR for selected entry
- **WHEN** a user runs `nxlang codegen ./workspace --target nx-ir --entry app/main.nx --output ./generated`
- **THEN** the CLI SHALL build a workspace `ProgramArtifact` using `app/main.nx` as the selected
  entry identity
- **AND** it SHALL write one NX IR image for each module of the workspace program, `app/main.nx`
  included

#### Scenario: Written artifacts carry debug data
- **WHEN** the CLI writes an NX IR image
- **THEN** the image SHALL contain the debug section with spans and source text
- **AND** `nxlang ir explain` on that file SHALL annotate each declaration with line and column

#### Scenario: Static diagnostics prevent IR output
- **WHEN** a user requests `--target nx-ir` for a source file that has static analysis errors
- **THEN** the CLI SHALL print diagnostics through the existing diagnostic rendering path
- **AND** it SHALL NOT write an NX IR artifact

#### Scenario: NX IR target rejects source output formats
- **WHEN** a user requests `--target nx-ir --format program-module`
- **THEN** the CLI SHALL report that NX IR codegen does not use source output formats
- **AND** it SHALL NOT write an NX IR artifact

### Requirement: Generated host types for a union are chosen by constant-ness
Type generation SHALL decide a union's generated host shape from the union's declaration rather
than from which keyword declared it. A constant union SHALL generate the host language's idiomatic
closed constant type: a C# `enum` with its authored-string wire format, and a TypeScript union of
the authored string literals. Generated TypeScript type declarations SHALL remain a pure type
surface — the generated module SHALL NOT export runtime values — which is why a constant union does
not generate a value object here; executable code generation, whose modules carry runtime code, is
specified separately and does generate one. A union with any payload case SHALL generate the
polymorphic shape: a C# abstract base with one derived type per case and `$type` discriminator
metadata, and a TypeScript union of per-case types.

Within a union that generates the polymorphic shape, a constant case SHALL be generated so that its
wire form is the bare authored case string. Generated C# SHALL expose such a case as a singleton
instance of its case type, and generated TypeScript SHALL include the case's string literal as a
member of the union type.

#### Scenario: A constant union generates a C# enum
- **WHEN** source contains `export type ThemeMode = light | dark`
- **THEN** generated C# SHALL declare `ThemeMode` as an `enum` with the authored-string wire format
- **AND** generated TypeScript SHALL declare `ThemeMode` as the union of its authored string
  literals

#### Scenario: A union with a payload case generates the polymorphic shape
- **WHEN** source contains `export type LoadState = idle | failed { message:string }`
- **THEN** generated C# SHALL declare an abstract `LoadState` with a derived type per case
- **AND** generated TypeScript SHALL declare `LoadState` as a union of its per-case types

#### Scenario: A constant case in a polymorphic union carries the bare string wire form
- **WHEN** source contains `export type LoadState = idle | failed { message:string }`
- **THEN** the generated TypeScript `LoadState` union SHALL include the string literal `"idle"` as
  the form of the `idle` case
- **AND** generated C# SHALL expose the `idle` case as a singleton whose serialized form is the bare
  string `"idle"`

#### Scenario: Generated TypeScript type declarations export no runtime values
- **WHEN** `types.nx` contains `export type ThemeMode = light | dark`
- **THEN** the generated TypeScript module SHALL declare `ThemeMode` as `"light" | "dark"`
- **AND** SHALL NOT emit an `as const` value object or any other runtime export

#### Scenario: Generated output for a constant union is unchanged from the enum form
- **WHEN** a declaration previously written `export enum ThemeMode = light | dark` is rewritten as
  `export type ThemeMode = light | dark`
- **THEN** the generated C# and TypeScript SHALL be byte-identical to the output produced for the
  enum form before this change

### Requirement: Generated type surfaces include update records with distinct absence
For every exported record, action, and component with declared state, `typegen` SHALL emit a
companion update type named `<Name>_update` beside the exported type, following the same
companion-naming and collision rules as `<ComponentName>_state`. The companion SHALL carry the
target's effective fields, each optional, SHALL carry the `$type` discriminator `<Name>.Update`
where the target language emits discriminators, and SHALL let a consumer distinguish a field that
is absent from a field that is present and `null`.

A companion field the target inherits from a base declared in another module SHALL have its type
resolved in the namespace of the module that declared the field, not the module that generates
the companion. When that resolution reaches a type exported by an imported dependency library, the
generated companion SHALL reference that library's generated declaration with the same
cross-library linkage an explicit import of the type would produce, whether or not the generating
module imports the type itself, and whether the declaring module named the type by its exported
name or by a local alias. When the resolution reaches a type the dependency does not export, one
typegen cannot reference across libraries, or one the generating module cannot name unambiguously
because an import of its own claims the same visible name, generation SHALL warn, naming the
companion, the field, and the type, and SHALL leave the field's written type name in the generated
output.

#### Scenario: TypeScript update companion uses optional properties
- **WHEN** source contains `export type User = { name:string email:string? }`
- **THEN** TypeScript generation SHALL emit `export interface User_update` with `$type: "User.Update"`
- **AND** SHALL declare `name?: string` and `email?: string | null`
- **AND** SHALL NOT declare either property as required

#### Scenario: C# update companion distinguishes absent from null
- **WHEN** source contains `export type User = { name:string email:string? }`
- **THEN** C# generation SHALL emit a generated type `User_update` whose properties are typed so that an unset property serializes to no key and a property set to `null` serializes to a `null` value
- **AND** deserializing an object without the `email` key SHALL leave that property unset rather than `null`
- **AND** the generated type SHALL carry the wire names `name` and `email` in a form both MessagePack and JSON serialization use, without requiring a per-property attribute on each generated property

#### Scenario: C# update companion carries its field schema
- **WHEN** source contains `export type User = { name:string email:string? }`
- **THEN** the generated `User_update` SHALL expose, to the managed SDK, each field's wire name paired with its value type
- **AND** that schema SHALL be the only thing serialization needs to read or write the companion, so no runtime reflection over the generated type is required

#### Scenario: Component update companion is derived from state
- **WHEN** source contains `export component <Counter step:int /> = { state { count:int = 0 } <Label /> }`
- **THEN** generation SHALL emit `Counter_update` with the single optional field `count`
- **AND** SHALL NOT include `step`

#### Scenario: Update companion name collision warns and skips
- **WHEN** source contains `export type User_update = string` and `export type User = { name:string }`
- **THEN** generation SHALL emit a warning about the `User_update` naming conflict
- **AND** SHALL omit the generated companion
- **AND** SHALL preserve the explicit exported declaration `User_update`

#### Scenario: Inherited companion field typed by a dependency export resolves without a local import
- **WHEN** library `named` exports `type Tag = a | b` and `abstract type Named = { name:string tag:Tag }`
- **AND** library `people` contains `import { Named } from "../named"` and `export type User extends Named = { email:string }`, and does not import `Tag`
- **THEN** TypeScript generation for `people` SHALL declare `tag?: Tag` on `User_update` and SHALL emit a type-only package import of `Tag` from the `named` dependency package
- **AND** C# generation for `people` SHALL declare the `Tag` property of `User_update` with the `Tag` type qualified by the `named` dependency namespace
- **AND** generation SHALL NOT warn that `Tag` is not imported

#### Scenario: Inherited companion field typed through the declaring module's alias resolves to the origin
- **WHEN** library `tags` exports `type Tag = a | b`
- **AND** library `named` contains `import { Tag as t.Tag } from "../tags"` and exports `abstract type Named = { name:string tag:t.Tag }`
- **AND** library `people` imports `Named` from `../named` and exports `type User extends Named = { email:string }`
- **THEN** generation for `people` SHALL reference the `tags` library's generated `Tag` declaration for the `tag` field of `User_update`
- **AND** SHALL NOT emit a reference to a declaration named `t.Tag`

#### Scenario: Inherited companion field whose type the dependency does not export still warns
- **WHEN** library `named` contains `type Tag = a | b` without `export` and exports `abstract type Named = { name:string tag:Tag }`
- **AND** library `people` imports `Named` from `../named` and exports `type User extends Named = { email:string }`
- **THEN** generation for `people` SHALL emit a warning naming `User_update`, the field `tag`, and the type `Tag`
- **AND** the generated companion SHALL carry the written name `Tag` for that field

#### Scenario: Inherited companion field whose peer type an import shadows still warns
- **WHEN** library `ui` contains `tag.nx` exporting `type Tag = a | b` and `named.nx` exporting `abstract type Named = { name:string tag:Tag }`
- **AND** its `user.nx` contains `import { Tag } from "../other"`, where library `other` exports a different `Tag`, and exports `type User extends Named = { email:string other:Tag }`
- **THEN** generation for `ui` SHALL emit a warning naming `User_update`, the field `tag`, and the type `Tag`
- **AND** the generated companion SHALL carry the written name `Tag` for that field

### Requirement: Generated type surfaces include a property companion per record-shaped declaration
For every exported record, action, and component with declared state, `typegen` SHALL emit a
companion property type named `<Name>_property` beside the exported type, following the same
companion-naming and collision rules as `<Name>_update`. The companion SHALL be generated exactly
as a constant union whose authored cases are the target's effective field names in `T.Property`
case order: a TypeScript union of the field-name string literals, which is assignable to `keyof`
the generated record type, and a C# `enum` with the authored-string wire format and the shared
enum serialization helpers. A type reference to `T.Property` in an exported contract SHALL be
generated as a reference to the `<T>_property` companion, including across library boundaries
with the same cross-library linkage a reference to `T` itself would produce.

#### Scenario: TypeScript property companion is a string literal union
- **WHEN** source contains `export type User = { name:string email:string? }`
- **THEN** TypeScript generation SHALL emit `export type User_property = "name" | "email";`
- **AND** SHALL NOT emit a runtime value for it

#### Scenario: C# property companion is an enum with the bare-string wire format
- **WHEN** source contains `export type User = { name:string email:string? }`
- **THEN** C# generation SHALL emit `enum User_property` with members for `name` and `email`
- **AND** the enum SHALL serialize each member as its authored field name in both JSON and MessagePack using the shared enum helpers

#### Scenario: Component property companion is derived from state and includes inherited fields
- **WHEN** source contains `abstract type Named = { name:string } export component <Counter step:int /> = { state { count:int = 0 } <Label /> }` and `export type User extends Named = { email:string }`
- **THEN** generation SHALL emit `Counter_property` with the single case `count` and SHALL NOT include `step`
- **AND** SHALL emit `User_property` with cases `name` then `email`

#### Scenario: A property-typed prop references the companion
- **WHEN** source contains `export type Contact = { title:string } export external component <Table sortBy:Contact.Property? columns:Contact.Property[] />`
- **THEN** generated TypeScript SHALL type `sortBy` as `Contact_property | null` and `columns` as `Contact_property[]`
- **AND** generated C# SHALL type `sortBy` as a nullable `Contact_property` and `columns` as a collection of `Contact_property`

#### Scenario: Property companion name collision warns and skips
- **WHEN** source contains `export type User_property = string` and `export type User = { name:string }`
- **THEN** generation SHALL emit a warning about the `User_property` naming conflict
- **AND** SHALL omit the generated companion
- **AND** SHALL preserve the explicit exported declaration `User_property`

### Requirement: C# generation emits typed property keys beside each property companion
For every declaration that gets a `<Name>_property` companion and whose fields are reachable on a
generated plain type, C# generation SHALL also emit a typed key per field. A key SHALL carry the
field's wire name, its value type, and read and write access to that field on the plain generated
type. Generation SHALL provide a mapping from each `<Name>_property` case to its key, so the
property companion remains the single naming of a declaration's fields and no second spelling of
those names is introduced.

Where a declaration has a property companion but no instantiable generated plain type carrying its
fields, such as a non-external component's state or an abstract record, generation SHALL emit the
property companion alone and SHALL NOT emit keys. An external component's props contract is not
the plain type of its state companion; its generated `<Name>_state` record is, so an external
component with declared state SHALL get keys over `<Name>_state`.

The key table's generated name SHALL follow the companion collision rule: when an exported
declaration already owns that name, generation SHALL warn naming the table and the companion,
SHALL omit the table, and SHALL emit the update companion without keys.

#### Scenario: Record gets a key per field
- **WHEN** source contains `export type User = { name:string email:string? }`
- **THEN** C# generation SHALL emit a typed key for `name` whose value type is the C# spelling of `string`, and one for `email` whose value type is nullable
- **AND** each key SHALL be able to read and write that field of a generated `User` instance
- **AND** generation SHALL emit a mapping from each `User_property` case to its key

#### Scenario: Abstract record gets a property companion but no keys
- **WHEN** source contains `export abstract type Named = { name:string }`
- **THEN** C# generation SHALL emit `Named_property` with the single case `name`
- **AND** SHALL NOT emit keys for it, since no instance of `Named` can be constructed to apply a patch to

#### Scenario: External component state gets keys over its state record
- **WHEN** source contains `export external component <Ticker step:int /> = { state { count:int = 0 } }`
- **THEN** C# generation SHALL emit a typed key for `count` over the generated `Ticker_state`
- **AND** `Ticker_update` SHALL apply to and diff `Ticker_state` values
- **AND** SHALL NOT emit keys over the `Ticker` props contract

#### Scenario: Key table name collision warns and skips
- **WHEN** source contains `export type User = { name:string }` and `export type UserProperties = { x:string }`
- **THEN** generation SHALL emit a warning naming `UserProperties` and `User_update`
- **AND** SHALL preserve the explicit exported declaration `UserProperties`
- **AND** SHALL emit `User_update` without keys

#### Scenario: Component state gets a property companion but no keys
- **WHEN** source contains `export component <Counter step:int /> = { state { count:int = 0 } <Label /> }`
- **THEN** C# generation SHALL emit `Counter_property` with the single case `count`
- **AND** SHALL NOT emit keys for it, since no generated plain type carries the component's state

### Requirement: Generated contracts erase component type parameters in C# and carry them generically in TypeScript
The C# `typegen` emitter SHALL map a component type parameter to `object` wherever an exported
contract references it, SHALL NOT declare a generic parameter on the generated record, and SHALL
NOT include a member for the type parameter. The TypeScript `typegen` emitter SHALL declare the
exported contract type with one generic parameter per component type parameter, each defaulting
to `unknown`, SHALL reference the parameter by name inside the type, and SHALL NOT include a
member for it. List and nullable suffixes on a type-parameter reference SHALL be preserved around
the erased or generic type. The exported contract's parameter list SHALL be the component's
effective one, including parameters inherited from an abstract base declared in another module of
the library. A generic component's `<Name>_update` companion and, for an external component, its
`<Name>_state` record SHALL erase a state field's type parameter in both languages — to `object`
in C# and `unknown` in TypeScript — with no generic parameter, because no host names that
instantiation: an NX use site fixed it.

#### Scenario: C# external contract erases the parameter
- **WHEN** NX source declares `export external component <SkiaLayout TItem:type itemsSource:TItem[]? />`
- **AND** a caller requests C# output
- **THEN** the generated `SkiaLayout` record SHALL declare `itemsSource` as a nullable list of `object`
- **AND** SHALL NOT declare a generic type parameter or a `TItem` property

#### Scenario: TypeScript external contract is generic with an unknown default
- **WHEN** NX source declares `export external component <SkiaLayout TItem:type itemsSource:TItem[]? />`
- **AND** a caller requests TypeScript output
- **THEN** the generated type SHALL be declared as `SkiaLayout<TItem = unknown>` with `itemsSource` as a nullable array of `TItem`
- **AND** SHALL NOT declare a `TItem` property
- **AND** a caller writing `SkiaLayout` with no argument SHALL get `itemsSource` typed as a nullable array of `unknown`

#### Scenario: Derived contract carries a parameter inherited across modules
- **WHEN** a library's `base.nx` declares `export abstract external component <ItemsBase TItem:type items:TItem[]? />`
- **AND** its `derived.nx` declares `export external component <ContactList extends ItemsBase extra:TItem[]? />`
- **THEN** the generated TypeScript SHALL declare `ContactList<TItem = unknown>` extending `ItemsBaseBase<TItem>` with `extra` as a nullable array of `TItem`
- **AND** the generated C# `ContactList` SHALL declare `extra` as a nullable list of `object` with no `TItem` anywhere

#### Scenario: State and update companion erase the parameter
- **WHEN** NX source declares `export external component <Picker TItem:type items:TItem[]? /> = { state { sel:TItem? } }`
- **THEN** the generated TypeScript `Picker_state` SHALL type `sel` as nullable `unknown` and `Picker_update` SHALL type it as optional nullable `unknown`
- **AND** the generated C# `Picker_state` SHALL declare `Sel` as nullable `object` and `Picker_update` SHALL expose it as `NxOptional<object?>`
- **AND** neither language's output SHALL declare a `TItem` member on either type

### Requirement: Generated contracts carry a generic record as a generic type in both languages
The C# and TypeScript `typegen` emitters SHALL declare an exported generic record with one generic
parameter per record type parameter, in declaration order and under the parameter's authored name,
SHALL type each field that mentions a parameter with that generic parameter, and SHALL NOT emit a
member for a type parameter. Both emitters SHALL render an applied type as the instantiation of
that generic type, with arguments in the record's declaration order regardless of the order the
source wrote them, and with each argument mapped as that language maps the same type anywhere else.
Unlike a component contract, a C# generic record SHALL NOT be erased, because a host names the
concrete instantiation at its own deserialization site. An applied type whose argument is a
component type parameter SHALL follow the component rule for that argument: `object` in C#, the
generic parameter in TypeScript. A generic record's `<Name>_update` companion SHALL declare the
same generic parameters the record does and SHALL type each of its fields as the record types the
field it patches, so that a patch of one instantiation is usable at that instantiation; in C# it
SHALL extend `NxUpdate<Name<T, …>>` and its `<Name>Properties` key table SHALL declare the same
parameters. A component's state companion SHALL continue to erase the *component's* type
parameters, because the type it patches is concrete. Neither companion SHALL put a type argument
on the wire: the discriminator and the field names are unchanged by the parameters.

#### Scenario: C# declares a generic record
- **WHEN** NX source declares `export type Range = { T:type start:T end:T endInclusive:boolean }`
- **AND** a caller requests C# output
- **THEN** the generated record SHALL be declared as `Range<T>` with `Start` and `End` typed `T` and `EndInclusive` typed `bool`
- **AND** SHALL NOT declare a `T` property

#### Scenario: TypeScript declares a generic record
- **WHEN** NX source declares `export type Range = { T:type start:T end:T endInclusive:boolean }`
- **AND** a caller requests TypeScript output
- **THEN** the generated type SHALL be declared as `Range<T>` with `start` and `end` typed `T`
- **AND** SHALL NOT declare a `T` property

#### Scenario: An applied type is rendered as the instantiation
- **WHEN** NX source declares `export type Range = { T:type start:T end:T }` and `export type Slider = { range:<Range T=float64/> marks:<Range T=int/>[]? }`
- **THEN** the generated C# `Slider` SHALL declare `Range` as `Range<double>` and `Marks` as a nullable list of `Range<long>`
- **AND** the generated TypeScript `Slider` SHALL declare `range` as `Range<number>` and `marks` as a nullable array of `Range<number>`

#### Scenario: Arguments are emitted in declaration order
- **WHEN** NX source declares `export type Pair = { TKey:type TValue:type key:TKey value:TValue }` and `export type Entry = { p:<Pair TValue=int TKey=string/> }`
- **THEN** the generated C# SHALL type `P` as `Pair<string, long>` and the generated TypeScript SHALL type `p` as `Pair<string, number>`

#### Scenario: A component type parameter as an argument follows the component rule
- **WHEN** NX source declares `export type Range = { T:type start:T end:T }` and `export external component <Slider TValue:type range:<Range T=TValue/>? />`
- **THEN** the generated C# `Slider` SHALL declare `Range` as a nullable `Range<object>`
- **AND** the generated TypeScript `Slider<TValue = unknown>` SHALL declare `range` as a nullable `Range<TValue>`

#### Scenario: The update companion of a generic record carries the parameter
- **WHEN** NX source declares `export type Range = { T:type start:T end:T }`
- **THEN** the generated TypeScript SHALL declare `Range_update<T>` with `start` typed as optional `T`
- **AND** the generated C# SHALL declare `Range_update<T> : NxUpdate<Range<T>>` with `Start` typed `NxOptional<T>`, `Diff(Range<T>, Range<T>)` returning `Range_update<T>`, and a `RangeProperties<T>` key table
- **AND** the serialized patch SHALL carry `"$type": "Range.Update"` and the field names alone, with no type argument

#### Scenario: A patch of a generic record applies to the instantiation it came from
- **WHEN** a C# host diffs two `Range<long>` values, serializes the patch and reads it back through either serializer
- **THEN** applying the patch SHALL produce a `Range<long>` carrying the changed fields
- **AND** reading a patched field SHALL give the record's field type rather than an untyped value

#### Scenario: A component's state companion still erases the component's parameters
- **WHEN** NX source declares `export external component <Picker TItem:type /> = { state { sel:TItem? } <Label /> }`
- **THEN** the generated `Picker_update` SHALL declare no generic parameter, and SHALL type `sel` as optional `unknown` in TypeScript and `NxOptional<object?>` in C#

### Requirement: CLI explains an NX IR artifact as readable text
Because an NX IR artifact is a binary image, `nxlang ir explain <artifact>` SHALL be the supported
way to read one. The command SHALL read an image and print it as text in which every table index is
replaced by what it names: declarations by name and kind, references by module identity and
declaration name, types spelled as NX would spell them, nodes as an indented expression tree, and
spans as line and column when the debug section is present. The output SHALL be deterministic for a
given artifact. The command SHALL fail with a diagnostic and a failure exit code on an artifact of
an unsupported schema version and on a malformed artifact, and SHALL never panic on any input.

#### Scenario: A snippet artifact reads as its source would
- **WHEN** a user runs `nxlang ir explain input.nxir` on the artifact of a program whose root
  returns `<SkiaLabel Text="hi" />`
- **THEN** the output SHALL show a function `root` whose body is a component descriptor of
  `drawnui`'s `SkiaLabel` with property `Text` equal to the string `"hi"`
- **AND** no table index SHALL appear in the output

#### Scenario: Debug data is shown when present
- **WHEN** the artifact carries a debug section
- **THEN** each declaration in the output SHALL be annotated with its source line and column

#### Scenario: An unsupported artifact is refused
- **WHEN** the artifact's schema version is not the one the CLI supports
- **THEN** the command SHALL print a diagnostic naming both versions and exit with a failure code

#### Scenario: A malformed artifact is reported, not fatal
- **WHEN** the artifact is truncated or one of its indices is out of range
- **THEN** the command SHALL print a diagnostic identifying what is malformed
- **AND** it SHALL exit with a failure code rather than panicking

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
