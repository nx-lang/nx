## ADDED Requirements

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

## REMOVED Requirements

### Requirement: Generated type surfaces preserve composed list and nullable type references
**Reason**: NX no longer has a nested list type, so the generator requirement no longer specifies nested-list output.
**Migration**: None for generated consumers; `T?[]` and `T[]?` emit as before.
