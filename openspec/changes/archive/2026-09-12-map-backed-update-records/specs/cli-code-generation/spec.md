## MODIFIED Requirements

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

## ADDED Requirements

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
