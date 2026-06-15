## RENAMED Requirements

FROM: `generate` infers file versus library generation from the input path
TO: `typegen` infers file versus library generation from the input path

## MODIFIED Requirements

### Requirement: `generate` infers file versus library generation from the input path
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
When `nxlang typegen` targets a directory, the CLI SHALL analyze that directory as an NX library and
SHALL generate code from every library module that contributes exported type declarations or
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
- **WHEN** a user runs `nxlang typegen ./empty-dir --language csharp --output ./generated`
- **THEN** the CLI SHALL report a library-analysis error if `empty-dir` is not a valid NX library

### Requirement: Library generation uses per-module multi-file output
When `nxlang typegen` targets a directory, generated output SHALL be written as multiple files using
one generated file per contributing NX module. Library generation SHALL require `--output`, and
that output path SHALL be treated as a directory root.

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
- **WHEN** library module `theme.nx` exports `enum ThemeMode = | light | dark`
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
