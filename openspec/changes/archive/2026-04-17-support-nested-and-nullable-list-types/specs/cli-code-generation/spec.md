## ADDED Requirements

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
