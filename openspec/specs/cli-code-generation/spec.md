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
When `nxlang generate` targets a single `.nx` file, the generated output SHALL include only type
declarations marked `export` in that file. The generated type surface SHALL cover exported type
aliases, exported enums, and exported record-like declarations, including exported action records.

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

### Requirement: Library generation emits the exported type surface of the full library
When `nxlang generate` targets a directory, the CLI SHALL analyze that directory as an NX library
and SHALL generate code from every library module that contributes exported type declarations. The
command MUST reject a directory that cannot be analyzed as a valid NX library.

#### Scenario: Exported declarations from multiple files are generated together
- **WHEN** library `./ui` contains `button.nx` with `export type ButtonSize = string` and
  `theme.nx` with `export enum ThemeMode = | light | dark`
- **THEN** library generation SHALL include generated output for both `ButtonSize` and `ThemeMode`

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
When a generated declaration references an exported type owned by another generated module in the
same library output, the generated files SHALL include whatever language-specific linkage is needed
to keep the generated output coherent.

#### Scenario: TypeScript emits relative imports for cross-module references
- **WHEN** generated file `forms.ts` contains a declaration referencing exported type `ThemeMode`
  owned by generated file `theme.ts`
- **THEN** `forms.ts` SHALL include a relative `import type` for `ThemeMode` from `theme.ts`

#### Scenario: C# cross-module references remain resolvable
- **WHEN** one generated `.g.cs` file references a generated type declared in another generated
  `.g.cs` file from the same library output
- **THEN** the generated C# output SHALL keep that reference resolvable without manual edits

### Requirement: TypeScript generated records preserve concrete runtime discriminators
TypeScript code generation SHALL emit record-like declarations that preserve the NX `$type` payload
discriminator. Every generated concrete record or action record SHALL include a `$type` property
whose type is the string literal of that record's exported name. When a concrete record derives
from an exported abstract record, the generated output SHALL preserve the abstract record's shared
fields through a reusable base contract while keeping the concrete record discriminated by its own
literal `$type`.

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

### Requirement: C# generated records keep a concrete `$type` value
C# code generation SHALL map the NX `$type` payload key to a non-null discriminator member whose
runtime value matches the concrete generated record name. Generated abstract records SHALL remain
inheritable, and derived concrete records SHALL expose their own discriminator value rather than
reusing the abstract base name.

#### Scenario: Concrete C# record initializes its discriminator to the concrete record name
- **WHEN** source contains `export type ShortTextQuestion = { label:string }`
- **THEN** generated C# SHALL include a non-null discriminator member mapped to `$type`
- **AND** that member SHALL default to `ShortTextQuestion`

#### Scenario: Abstract C# base remains inheritable and derived record keeps its own discriminator
- **WHEN** source contains `export abstract type Question = { label:string } export type ShortTextQuestion extends Question = { placeholder:string? }`
- **THEN** generated C# SHALL emit `Question` as an inheritable abstract generated record type
- **AND** generated `ShortTextQuestion` SHALL expose a discriminator member mapped to `$type`
- **AND** that discriminator member SHALL default to `ShortTextQuestion` rather than `Question`
