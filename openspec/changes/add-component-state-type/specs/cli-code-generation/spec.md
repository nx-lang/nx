## RENAMED Requirements

- FROM: `### Requirement: Generated external component state contracts use stable companion names`
- TO: `### Requirement: Generated component state contracts use stable companion names`

- FROM: `### Requirement: Generated external component state names warn and skip on collisions`
- TO: `### Requirement: Generated component state names warn and skip on collisions`

## MODIFIED Requirements

### Requirement: Generated component state contracts use stable companion names
Generated component state contracts SHALL use stable companion names. For every exported
component that declares state, external or not, the generator SHALL emit a companion state contract
named `<ComponentName>_state`, which SHALL also be what a type reference to `<ComponentName>.State`
in an exported contract generates. The contract SHALL include exactly the declared state fields
from that component, SHALL map those fields to the same wire names for both MessagePack and JSON, and SHALL
NOT include component props, emitted actions, or a `$type` discriminator.

#### Scenario: TypeScript companion state contract is a plain interface
- **WHEN** source contains `export external component <SearchBox /> = { state { query:string } }`
- **THEN** TypeScript generation SHALL emit `export interface SearchBox_state`
- **AND** SHALL include property `query: string`
- **AND** SHALL NOT emit `$type` on `SearchBox_state`

#### Scenario: C# companion state contract is a plain dual-annotated DTO
- **WHEN** source contains `export external component <SearchBox /> = { state { query:string } }`
- **THEN** C# generation SHALL emit a generated type `SearchBox_state`
- **AND** SHALL include the declared state field `query`
- **AND** SHALL annotate that field so both MessagePack and JSON use wire name `query`
- **AND** SHALL NOT emit a `$type` discriminator member on `SearchBox_state`

#### Scenario: A concrete component with state gets a state contract
- **WHEN** source contains `export component <Counter step:int /> = { state { count:int = 0 } <Label /> }`
- **THEN** TypeScript generation SHALL emit `export interface Counter_state` with the property `count: number`
- **AND** SHALL NOT include `step`

### Requirement: Generated component state names warn and skip on collisions
The generator SHALL warn when a synthesized component state companion name, `<Name>_state`,
`<Name>_state_update` or `<Name>_state_property`, would collide with another generated declaration
name or exported declaration name, and SHALL skip generation of
the synthesized companion instead of overwriting the conflicting declaration.

#### Scenario: Generated external state name collides with an exported declaration
- **WHEN** source contains `export type SearchBox_state = string` and `export external component <SearchBox /> = { state { query:string } }`
- **THEN** generation SHALL emit a warning about the `SearchBox_state` naming conflict
- **AND** SHALL omit the generated `SearchBox_state` companion contract
- **AND** SHALL preserve the explicit exported declaration `SearchBox_state`

### Requirement: Generated type surfaces include update records with distinct absence
For every exported record and action, `typegen` SHALL emit a companion update type named
`<Name>_update` beside the exported type, following the same companion-naming and collision rules
as `<ComponentName>_state`, and for every exported component that declares state it SHALL emit
`<Name>_state_update` for the component's `T.State.Update`. A component SHALL NOT get a
`<Name>_update` of its own, since it has no `T.Update`. The companion SHALL carry the target's
effective fields, which for `<Name>_state_update` are the component's state fields, each optional,
SHALL carry the update record's `$type` discriminator (`<Name>.Update` or `<Name>.State.Update`)
where the target language emits discriminators, and SHALL let a consumer distinguish a field that
is absent from a field that is present and `null`, where `null` is the host spelling of a cleared
field — the empty value — and is legal only for a field that is optional in the target, as
`update-records` requires. In TypeScript a clearable field SHALL be declared `name?: T | null` and
a non-clearable one `name?: T`. In C# an unset field SHALL serialize to no key and a cleared field
to `null`, and a non-clearable field's value type SHALL be non-nullable.

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
- **WHEN** source contains `export type User = { name:string email?:string }`
- **THEN** TypeScript generation SHALL emit `export interface User_update` with `$type: "User.Update"`
- **AND** SHALL declare `name?: string` and `email?: string | null`
- **AND** SHALL NOT declare either property as required

#### Scenario: C# update companion distinguishes absent from null
- **WHEN** source contains `export type User = { name:string email?:string }`
- **THEN** C# generation SHALL emit a generated type `User_update` whose properties are typed so that an unset property serializes to no key and a property set to `null` serializes to a `null` value
- **AND** deserializing an object without the `email` key SHALL leave that property unset rather than `null`
- **AND** the `Name` property's value type SHALL be non-nullable, since `name` cannot be cleared
- **AND** the generated type SHALL carry the wire names `name` and `email` in a form both MessagePack and JSON serialization use, without requiring a per-property attribute on each generated property

#### Scenario: C# update companion carries its field schema
- **WHEN** source contains `export type User = { name:string email?:string }`
- **THEN** the generated `User_update` SHALL expose, to the managed SDK, each field's wire name paired with its value type
- **AND** that schema SHALL be the only thing serialization needs to read or write the companion, so no runtime reflection over the generated type is required

#### Scenario: A component's only update companion is its state's
- **WHEN** source contains `export component <Counter step:int /> = { state { count:int = 0 } <Label /> }`
- **THEN** generation SHALL emit `Counter_state_update` with the single optional field `count` and the `$type` `Counter.State.Update`
- **AND** SHALL NOT emit `Counter_update`

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

### Requirement: Generated type surfaces include a property companion per record-shaped declaration
For every exported record and action, `typegen` SHALL emit a companion property type named
`<Name>_property` beside the exported type, following the same companion-naming and collision rules
as `<Name>_update`, and for every exported component that declares state it SHALL emit
`<Name>_state_property` for the component's `T.State.Property`. A component SHALL NOT get a
`<Name>_property` of its own, since it has no `T.Property`. The companion SHALL be generated exactly
as a constant union whose authored cases are the target's effective field names in `T.Property`
case order: a TypeScript union of the field-name string literals, which is assignable to `keyof`
the generated record type, and a C# `enum` with the authored-string wire format and the shared
enum serialization helpers. A type reference to `T.Property` in an exported contract SHALL be
generated as a reference to the `<T>_property` companion, including across library boundaries
with the same cross-library linkage a reference to `T` itself would produce.

#### Scenario: TypeScript property companion is a string literal union
- **WHEN** source contains `export type User = { name:string email?:string }`
- **THEN** TypeScript generation SHALL emit `export type User_property = "name" | "email";`
- **AND** SHALL NOT emit a runtime value for it

#### Scenario: C# property companion is an enum with the bare-string wire format
- **WHEN** source contains `export type User = { name:string email?:string }`
- **THEN** C# generation SHALL emit `enum User_property` with members for `name` and `email`
- **AND** the enum SHALL serialize each member as its authored field name in both JSON and MessagePack using the shared enum helpers

#### Scenario: Property companions include inherited fields, and a component's only one is its state's
- **WHEN** source contains `abstract type Named = { name:string } export component <Counter step:int /> = { state { count:int = 0 } <Label /> }` and `export type User extends Named = { email:string }`
- **THEN** generation SHALL emit `Counter_state_property` with the single case `count` and SHALL NOT emit `Counter_property`
- **AND** SHALL emit `User_property` with cases `name` then `email`

#### Scenario: A property-typed prop references the companion
- **WHEN** source contains `export type Contact = { title:string } export external component <Table sortBy?:Contact.Property columns:Contact.Property+ />`
- **THEN** generated TypeScript SHALL type `sortBy` as the optional property `sortBy?: Contact_property` and `columns` as `Contact_property[]`
- **AND** generated C# SHALL type `sortBy` as a nullable `Contact_property` and `columns` as a collection of `Contact_property`

#### Scenario: Property companion name collision warns and skips
- **WHEN** source contains `export type User_property = string` and `export type User = { name:string }`
- **THEN** generation SHALL emit a warning about the `User_property` naming conflict
- **AND** SHALL omit the generated companion
- **AND** SHALL preserve the explicit exported declaration `User_property`
