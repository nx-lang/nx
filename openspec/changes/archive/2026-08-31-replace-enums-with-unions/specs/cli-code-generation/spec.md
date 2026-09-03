## MODIFIED Requirements

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

## ADDED Requirements

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
