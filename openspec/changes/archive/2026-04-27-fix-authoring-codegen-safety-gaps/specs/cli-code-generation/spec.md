## MODIFIED Requirements

### Requirement: Generated library files preserve cross-module type references
Generated library files SHALL preserve cross-module and cross-library type references for any
generated declaration, including an external-component state companion contract. When a generated
declaration references an exported type owned by another generated module in the same library
output, the generated files SHALL include whatever language-specific linkage is needed to keep the
generated output coherent. When TypeScript generation references an exported type owned by an
imported dependency library, the generated file SHALL emit a type-only package import for that
dependency type. TypeScript package import targets SHALL be derived from the dependency library name
and the optional `--typescript-package-prefix` value until explicit package metadata exists.

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
- **AND** the user runs `nxlang generate ./chat-link --language typescript --typescript-package-prefix @org/nx- --output ./generated`
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

#### Scenario: C# aliases render transparently
- **WHEN** a module exports `type Count = int`
- **AND** another generated declaration references `Count`
- **THEN** C# generation SHALL render references to `Count` as the target C# type `long`
- **AND** SHALL NOT emit `global using Count = long;`
- **AND** a module that exports only transparent aliases SHALL NOT emit a namespace block solely for
  those aliases
