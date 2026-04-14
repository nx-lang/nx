## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: Generated external component state contracts use stable companion names
Generated external component state contracts SHALL use stable companion names. When generated
output includes a companion state contract for an exported external component, the generator SHALL
name it `<ComponentName>_state`, SHALL include exactly the declared state fields from that
component, and SHALL NOT include component props, emitted actions, or a `$type` discriminator.

#### Scenario: TypeScript companion state contract is a plain interface
- **WHEN** source contains `export external component <SearchBox /> = { state { query:string } }`
- **THEN** TypeScript generation SHALL emit `export interface SearchBox_state`
- **AND** SHALL include property `query: string`
- **AND** SHALL NOT emit `$type` on `SearchBox_state`

#### Scenario: C# companion state contract is a plain MessagePack object
- **WHEN** source contains `export external component <SearchBox /> = { state { query:string } }`
- **THEN** C# generation SHALL emit a generated type `SearchBox_state`
- **AND** SHALL include the declared state field `query`
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
