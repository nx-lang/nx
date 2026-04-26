## ADDED Requirements

### Requirement: Generated TypeScript supports exported discriminated unions
TypeScript code generation SHALL include exported discriminated union declarations in single-file
and library generation. The generated TypeScript surface SHALL expose the source union name as a
closed union over its generated cases. Every generated case member SHALL carry a literal `$type`
property whose value is the fully scoped NX case name, SHALL include declared and inherited fields
with authored wire names, and SHALL allow TypeScript consumers to narrow by `$type`.

#### Scenario: TypeScript generation emits narrowable union cases
- **WHEN** source contains `export type LoadState = | idle | failed { message:string retryable:bool = true }`
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
- **WHEN** source contains `export type LoadState = | idle | failed { message:string retryable:bool = true }`
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

#### Scenario: C# generation keeps enums separate from fieldless unions
- **WHEN** source contains `export enum CardSortMode = closed | open` and `export type LoadState = | idle | loading`
- **THEN** generated C# SHALL use enum serialization helpers for `CardSortMode`
- **AND** generated C# SHALL use `$type` polymorphic DTO support for `LoadState`
