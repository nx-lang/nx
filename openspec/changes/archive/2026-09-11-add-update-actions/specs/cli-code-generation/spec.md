## ADDED Requirements

### Requirement: Generated type surfaces include update records with distinct absence
For every exported record, action, and component with declared state, `typegen` SHALL emit a
companion update type named `<Name>_update` beside the exported type, following the same
companion-naming and collision rules as `<ComponentName>_state`. The companion SHALL carry the
target's effective fields, each optional, SHALL carry the `$type` discriminator `<Name>.Update`
where the target language emits discriminators, and SHALL let a consumer distinguish a field that
is absent from a field that is present and `null`.

#### Scenario: TypeScript update companion uses optional properties
- **WHEN** source contains `export type User = { name:string email:string? }`
- **THEN** TypeScript generation SHALL emit `export interface User_update` with `$type: "User.Update"`
- **AND** SHALL declare `name?: string` and `email?: string | null`
- **AND** SHALL NOT declare either property as required

#### Scenario: C# update companion distinguishes absent from null
- **WHEN** source contains `export type User = { name:string email:string? }`
- **THEN** C# generation SHALL emit a generated type `User_update` whose properties are typed so that an unset property serializes to no key and a property set to `null` serializes to a `null` value
- **AND** deserializing an object without the `email` key SHALL leave that property unset rather than `null`
- **AND** the type SHALL be annotated so both MessagePack and JSON use the wire names `name` and `email`

#### Scenario: Component update companion is derived from state
- **WHEN** source contains `export component <Counter step:int /> = { state { count:int = 0 } <Label /> }`
- **THEN** generation SHALL emit `Counter_update` with the single optional field `count`
- **AND** SHALL NOT include `step`

#### Scenario: Update companion name collision warns and skips
- **WHEN** source contains `export type User_update = string` and `export type User = { name:string }`
- **THEN** generation SHALL emit a warning about the `User_update` naming conflict
- **AND** SHALL omit the generated companion
- **AND** SHALL preserve the explicit exported declaration `User_update`
