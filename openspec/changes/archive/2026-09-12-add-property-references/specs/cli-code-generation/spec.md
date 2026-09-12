## ADDED Requirements

### Requirement: Generated type surfaces include a property companion per record-shaped declaration
For every exported record, action, and component with declared state, `typegen` SHALL emit a
companion property type named `<Name>_property` beside the exported type, following the same
companion-naming and collision rules as `<Name>_update`. The companion SHALL be generated exactly
as a constant union whose authored cases are the target's effective field names in `T.Property`
case order: a TypeScript union of the field-name string literals, which is assignable to `keyof`
the generated record type, and a C# `enum` with the authored-string wire format and the shared
enum serialization helpers. A type reference to `T.Property` in an exported contract SHALL be
generated as a reference to the `<T>_property` companion, including across library boundaries
with the same cross-library linkage a reference to `T` itself would produce.

#### Scenario: TypeScript property companion is a string literal union
- **WHEN** source contains `export type User = { name:string email:string? }`
- **THEN** TypeScript generation SHALL emit `export type User_property = "name" | "email";`
- **AND** SHALL NOT emit a runtime value for it

#### Scenario: C# property companion is an enum with the bare-string wire format
- **WHEN** source contains `export type User = { name:string email:string? }`
- **THEN** C# generation SHALL emit `enum User_property` with members for `name` and `email`
- **AND** the enum SHALL serialize each member as its authored field name in both JSON and MessagePack using the shared enum helpers

#### Scenario: Component property companion is derived from state and includes inherited fields
- **WHEN** source contains `abstract type Named = { name:string } export component <Counter step:int /> = { state { count:int = 0 } <Label /> }` and `export type User extends Named = { email:string }`
- **THEN** generation SHALL emit `Counter_property` with the single case `count` and SHALL NOT include `step`
- **AND** SHALL emit `User_property` with cases `name` then `email`

#### Scenario: A property-typed prop references the companion
- **WHEN** source contains `export type Contact = { title:string } export external component <Table sortBy:Contact.Property? columns:Contact.Property[] />`
- **THEN** generated TypeScript SHALL type `sortBy` as `Contact_property | null` and `columns` as `Contact_property[]`
- **AND** generated C# SHALL type `sortBy` as a nullable `Contact_property` and `columns` as a collection of `Contact_property`

#### Scenario: Property companion name collision warns and skips
- **WHEN** source contains `export type User_property = string` and `export type User = { name:string }`
- **THEN** generation SHALL emit a warning about the `User_property` naming conflict
- **AND** SHALL omit the generated companion
- **AND** SHALL preserve the explicit exported declaration `User_property`
