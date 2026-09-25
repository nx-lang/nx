## REMOVED Requirements

### Requirement: Generated type surfaces preserve nullable list structure
**Reason**: The language no longer has composed list and nullable suffixes. `T?[]` and `T[]?` are
both unspellable under `occurrence-types`: a type reference carries at most one occurrence, and a
suffixed type is never an item type. There is no nullable-list-versus-list-of-nullables structure
left for the generator to preserve.

**Migration**: Generated host types follow the new requirement "Generated type surfaces map
occurrences": `T?` maps to a nullable host type or an optional property, `T+` and `T*` map to the
host array type, and nothing nests.

## ADDED Requirements

### Requirement: Generated type surfaces map occurrences
The generator SHALL map each of the four occurrences to one host shape wherever an exported alias,
record-like field, action field, component prop, external-component state contract, or companion
mentions a type. `T?` in a type position SHALL map to TypeScript `T | null` and C# `T?`. A property
declared `name?:T` SHALL map to a TypeScript optional property `name?: T` and a C# nullable property
`T?`; one declared `name?:T+` SHALL map to `name?: T[]` and `T[]?` respectively, since its read type
is `T*`. `T+` and `T*` SHALL both map to TypeScript `T[]` and to the C# list or array mapping the
generator already uses for a sequence. The non-emptiness of `+` SHALL NOT be encoded in the static
type; it SHALL be validated when the value crosses the NX boundary, where the runtime rejects an
empty array at a `+` site as `occurrence-types` requires. Because an NX sequence never contains a
sequence and no suffixed type is an item type, the generator SHALL NOT be asked to emit a nested
list or a nullable element type, and SHALL NOT synthesize one. Generated output SHALL NOT mention
`null` for an NX value except as the host reading of an absent optional value or a cleared update
field.

#### Scenario: TypeScript aliases map each occurrence
- **WHEN** source contains `export type MaybeName = string?`, `export type Names = string+` and `export type Tags = string*`
- **THEN** TypeScript generation SHALL emit `export type MaybeName = string | null;`
- **AND** SHALL emit `export type Names = string[];` and `export type Tags = string[];`

#### Scenario: TypeScript fields map optional properties and sequences
- **WHEN** source contains `export type Payload = { names:string+ tags?:string+ nick?:string }`
- **THEN** generated TypeScript for `Payload` SHALL include `names: string[]`, `tags?: string[]` and `nick?: string`

#### Scenario: C# fields map optional properties and sequences
- **WHEN** source contains `export type Payload = { names:string+ tags?:string+ nick?:string }`
- **THEN** generated C# for `Payload` SHALL include property `Names` with type `string[]`
- **AND** SHALL include property `Tags` with type `string[]?`
- **AND** SHALL include property `Nick` with type `string?`

#### Scenario: Non-emptiness is validated at the boundary, not in the type
- **WHEN** NX source declares `type Book = { title:string authors:Person+ }` and a component taking `book:Book`
- **AND** a C# host passes as that prop a record whose `authors` is an empty array, in the shape typegen emits for a `+` field (a plain `Person[]`)
- **THEN** the host code SHALL compile without objection
- **AND** the NX runtime SHALL reject the value with a diagnostic naming `authors`

## MODIFIED Requirements

### Requirement: TypeScript generated records preserve concrete runtime discriminators
TypeScript code generation SHALL emit record-like declarations that preserve the NX `$type` payload
discriminator. Every generated concrete record or action record SHALL include a `$type` property
whose type is the string literal of that declaration's exported name. An abstract record or action
SHALL be generated under its own name as an open contract: an interface declaring its fields, whose
`$type` is any string. A record, action or union case that extends it SHALL extend that interface
and narrow `$type` to its own name. The generated output SHALL NOT generate a closed union of an
abstract base's descendants: an abstract base is an open set in NX, as it is in generated C#, and a
closed set of records is an NX union, generated as a TypeScript union.

#### Scenario: Concrete record includes a literal `$type`
- **WHEN** source contains `export type ShortTextQuestion = { label:string }`
- **THEN** generated TypeScript SHALL include a `ShortTextQuestion` contract with
  `$type: "ShortTextQuestion"`

#### Scenario: Abstract record family exposes a shared base and concrete runtime surface
- **WHEN** source contains `export abstract type Question = { label:string } export type ShortTextQuestion extends Question = { placeholder?:string } export type LongTextQuestion extends Question = { wordLimit?:int }`
- **THEN** generated TypeScript SHALL declare `interface Question` with the shared `label` field and
  a `$type` that is any string
- **AND** the generated `ShortTextQuestion` and `LongTextQuestion` contracts SHALL each extend
  `Question` and narrow `$type` to their own name
- **AND** a value of either descendant SHALL be accepted where a `Question` is expected
- **AND** generated TypeScript SHALL NOT declare a union of the descendants

#### Scenario: Cross-module abstract record family remains generated as a coherent TypeScript surface
- **WHEN** library module `questions/base.nx` exports `abstract type Question = { label:string }`
- **AND** library module `questions/short-text.nx` exports
  `type ShortTextQuestion extends Question = { placeholder?:string }`
- **THEN** `questions/short-text.ts` SHALL import `Question` as a type from `questions/base.ts` and
  declare `ShortTextQuestion` extending it
- **AND** `questions/base.ts` SHALL NOT import or name `ShortTextQuestion`

#### Scenario: Exported action record includes a literal `$type`
- **WHEN** source contains `export action SearchRequested = { query:string }`
- **THEN** generated TypeScript SHALL include `$type: "SearchRequested"` on the generated
  `SearchRequested` contract

#### Scenario: Abstract action family exposes a shared base and concrete runtime surface
- **WHEN** source contains `export abstract action SearchAction = { source:string } export action SearchRequested extends SearchAction = { query:string } export action SearchSubmitted extends SearchAction = { submittedAt:string }`
- **THEN** generated TypeScript SHALL declare `interface SearchAction` with the shared `source`
  field and a `$type` that is any string
- **AND** the generated `SearchRequested` and `SearchSubmitted` contracts SHALL each extend
  `SearchAction` and narrow `$type` to their own name
- **AND** a value of either descendant SHALL be accepted where a `SearchAction` is expected

#### Scenario: Cross-module abstract action family remains generated as a coherent TypeScript surface
- **WHEN** library module `actions/base.nx` exports `abstract action SearchAction = { source:string }`
- **AND** library module `actions/requested.nx` exports
  `action SearchRequested extends SearchAction = { query:string }`
- **THEN** `actions/requested.ts` SHALL import `SearchAction` as a type from `actions/base.ts` and
  declare `SearchRequested` extending it
- **AND** `actions/base.ts` SHALL NOT import or name `SearchRequested`

### Requirement: C# generated records use serializer metadata without emitted discriminator members
C# code generation SHALL emit generated record and action DTO classes that can serialize and
deserialize with both MessagePack and `System.Text.Json` without declaring a generated data member
mapped to wire key `$type`. Generated data members SHALL preserve their NX wire names across both
serializers, generated abstract records and actions SHALL remain inheritable, abstract roots SHALL
advertise polymorphism using `$type` and their concrete descendants for both serializers, and
MessagePack polymorphism SHALL use the same `$type`-keyed map contract as canonical `NxValue`.

#### Scenario: Concrete C# record emits only declared fields
- **WHEN** source contains `export type ShortTextQuestion = { label:string }`
- **THEN** generated C# SHALL emit `ShortTextQuestion` without a generated member mapped to `$type`
- **AND** generated property `Label` SHALL be annotated so both serializers use wire name `label`

#### Scenario: Concrete C# action emits only declared fields
- **WHEN** source contains `export action SearchRequested = { query:string }`
- **THEN** generated C# SHALL emit `SearchRequested` without a generated member mapped to `$type`
- **AND** generated property `Query` SHALL be annotated so both serializers use wire name `query`

#### Scenario: Abstract C# record root advertises polymorphism without a discriminator member
- **WHEN** source contains `export abstract type Question = { label:string } export type ShortTextQuestion extends Question = { placeholder?:string }`
- **THEN** generated C# SHALL emit `Question` as an inheritable abstract generated record type
- **AND** `Question` SHALL advertise polymorphism using `$type` and its concrete descendants
- **AND** generated `ShortTextQuestion` SHALL not declare a generated member mapped to `$type`

#### Scenario: Intermediate abstract C# records inherit the root metadata without redeclaring a member
- **WHEN** source contains `export abstract type Question = { label:string } export abstract type TextQuestion extends Question = { placeholder?:string } export type ShortTextQuestion extends TextQuestion = { maxLength?:int }`
- **THEN** the generated root abstract type SHALL advertise polymorphism for its concrete
  descendants using `$type`
- **AND** intermediate abstract generated records SHALL inherit that metadata without redeclaring a
  generated member mapped to `$type`

#### Scenario: Abstract C# root without concrete descendants omits invalid polymorphism metadata and warns
- **WHEN** source contains `export abstract type Question = { label:string }`
- **THEN** generated C# SHALL emit `Question` without `[JsonPolymorphic]`, `[JsonDerivedType]`, or
  polymorphic MessagePack formatter metadata (`NxPolymorphicMessagePackFormatter` /
  `NxPolymorphicConcreteMessagePackFormatter`)
- **AND** generated C# SHALL include a comment explaining that no polymorphism metadata (JSON or
  MessagePack) was generated because the abstract type had no concrete exported descendants at
  code-generation time
- **AND** the generator SHALL emit a warning that `Question` has no concrete exported descendants
  for C# polymorphic generation

#### Scenario: User field names do not collide with a synthetic discriminator member
- **WHEN** source contains `export type Payload = { nx_type:string }`
- **THEN** generated C# SHALL emit a property for wire name `nx_type`
- **AND** generated C# SHALL not emit any extra `__NxType` or `$type` data member on `Payload`

### Requirement: Generated TypeScript supports exported discriminated unions
TypeScript code generation SHALL include exported discriminated union declarations in single-file
and library generation. The generated TypeScript surface SHALL expose the source union name as a
closed union over its generated cases. Every generated case member SHALL carry a literal `$type`
property whose value is the fully scoped NX case name, SHALL include declared and inherited fields
with authored wire names, and SHALL allow TypeScript consumers to narrow by `$type`.

#### Scenario: TypeScript generation emits narrowable union cases
- **WHEN** source contains `export type LoadState = | idle | failed { message:string retryable:boolean = true }`
- **THEN** generated TypeScript SHALL include an exported `LoadState` type surface
- **AND** one generated case member SHALL have `$type: "LoadState.idle"`
- **AND** one generated case member SHALL have `$type: "LoadState.failed"` and field
  `message: string`
- **AND** TypeScript consumers SHALL be able to narrow `LoadState` by checking `$type`

#### Scenario: TypeScript generation includes shared inherited union fields
- **WHEN** source contains `export abstract type EventBase = { source:string } export type UiEvent extends EventBase = | clicked { x:int } | closed`
- **THEN** generated TypeScript SHALL include `source` on every generated `UiEvent` case member,
  each of which extends the open `EventBase` contract
- **AND** the exported `UiEvent` type surface SHALL remain narrowable by the case `$type`

#### Scenario: TypeScript library generation preserves cross-module field references
- **WHEN** library module `items.nx` exports `type Item = { name:string }`
- **AND** library module `state.nx` exports `type LoadState = | loaded { items:Item+ }`
- **THEN** TypeScript library generation SHALL emit any needed type-only imports so the generated
  `LoadState.loaded` case field can reference `Item`

### Requirement: Generated C# DTO properties preserve supported literal defaults
C# type generation SHALL preserve authored NX literal defaults on generated DTO properties when the
literal can be represented as a C# property initializer. Supported literal defaults SHALL include
string, integer, floating-point, and boolean literals. When a generated C# property has a supported
literal default, that authored initializer SHALL take precedence over the generator's non-null
reference `default!` initializer. An optional property (`name?:T`) has no default, as
`optional-properties` requires, and SHALL be emitted as a nullable property with no initializer.
When a C# generated field has a non-literal default expression, generation SHALL continue and
SHALL emit a warning that the default could not be preserved.

#### Scenario: Record field literal defaults are emitted as C# initializers
- **WHEN** source contains `export type Settings = { enabled:boolean = true count:int = 42 title:string = "hello" maybe?:string }`
- **THEN** generated C# SHALL include `public bool Enabled { get; set; } = true;`
- **AND** generated C# SHALL include `public long Count { get; set; } = 42;`
- **AND** generated C# SHALL include `public string Title { get; set; } = "hello";`
- **AND** generated C# SHALL include `public string? Maybe { get; set; }` with no initializer

#### Scenario: Union case field literal defaults are emitted as C# initializers
- **WHEN** source contains `export type LoadState = | failed { retryable:boolean = true }`
- **THEN** generated C# SHALL include generated case DTO `LoadStateFailed`
- **AND** generated C# SHALL include `public bool Retryable { get; set; } = true;`

#### Scenario: External component prop literal defaults are emitted as C# initializers
- **WHEN** source contains `export external component <Toggle selected:boolean = true label:string = "On" />`
- **THEN** generated C# SHALL include generated component prop DTO `Toggle`
- **AND** generated C# SHALL include `public bool Selected { get; set; } = true;`
- **AND** generated C# SHALL include `public string Label { get; set; } = "On";`

#### Scenario: Unsupported default expressions warn instead of silently changing semantics
- **WHEN** source contains `export type Settings = { enabled:boolean = { !false } }`
- **THEN** C# generation SHALL emit a warning that the default for `Settings.enabled` could not be preserved
- **AND** generated C# SHALL omit a property initializer for `Enabled`

### Requirement: Generated type surfaces include update records with distinct absence
For every exported record, action, and component with declared state, `typegen` SHALL emit a
companion update type named `<Name>_update` beside the exported type, following the same
companion-naming and collision rules as `<ComponentName>_state`. The companion SHALL carry the
target's effective fields, each optional, SHALL carry the `$type` discriminator `<Name>.Update`
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
- **WHEN** source contains `export type User = { name:string email?:string }`
- **THEN** TypeScript generation SHALL emit `export type User_property = "name" | "email";`
- **AND** SHALL NOT emit a runtime value for it

#### Scenario: C# property companion is an enum with the bare-string wire format
- **WHEN** source contains `export type User = { name:string email?:string }`
- **THEN** C# generation SHALL emit `enum User_property` with members for `name` and `email`
- **AND** the enum SHALL serialize each member as its authored field name in both JSON and MessagePack using the shared enum helpers

#### Scenario: Component property companion is derived from state and includes inherited fields
- **WHEN** source contains `abstract type Named = { name:string } export component <Counter step:int /> = { state { count:int = 0 } <Label /> }` and `export type User extends Named = { email:string }`
- **THEN** generation SHALL emit `Counter_property` with the single case `count` and SHALL NOT include `step`
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

### Requirement: C# generation emits typed property keys beside each property companion
For every declaration that gets a `<Name>_property` companion and whose fields are reachable on a
generated plain type, C# generation SHALL also emit a typed key per field. A key SHALL carry the
field's wire name, its value type, and read and write access to that field on the plain generated
type. A key for an optional field (`name?:T`) SHALL carry a nullable value type. Generation SHALL
provide a mapping from each `<Name>_property` case to its key, so the property companion remains
the single naming of a declaration's fields and no second spelling of those names is introduced.

Where a declaration has a property companion but no instantiable generated plain type carrying its
fields, such as a non-external component's state or an abstract record, generation SHALL emit the
property companion alone and SHALL NOT emit keys. An external component's props contract is not
the plain type of its state companion; its generated `<Name>_state` record is, so an external
component with declared state SHALL get keys over `<Name>_state`.

The key table's generated name SHALL follow the companion collision rule: when an exported
declaration already owns that name, generation SHALL warn naming the table and the companion,
SHALL omit the table, and SHALL emit the update companion without keys.

#### Scenario: Record gets a key per field
- **WHEN** source contains `export type User = { name:string email?:string }`
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

### Requirement: Generated contracts erase component type parameters in C# and carry them generically in TypeScript
The C# `typegen` emitter SHALL map a component type parameter to `object` wherever an exported
contract references it, SHALL NOT declare a generic parameter on the generated record, and SHALL
NOT include a member for the type parameter. The TypeScript `typegen` emitter SHALL declare the
exported contract type with one generic parameter per component type parameter, each defaulting
to `unknown`, SHALL reference the parameter by name inside the type, and SHALL NOT include a
member for it. An occurrence on a type-parameter reference, and the optional mark on the property
that carries it, SHALL be preserved around the erased or generic type as "Generated type surfaces
map occurrences" maps them. The exported contract's parameter list SHALL be the component's
effective one, including parameters inherited from an abstract base declared in another module of
the library. A generic component's `<Name>_update` companion and, for an external component, its
`<Name>_state` record SHALL erase a state field's type parameter in both languages — to `object`
in C# and `unknown` in TypeScript — with no generic parameter, because no host names that
instantiation: an NX use site fixed it.

#### Scenario: C# external contract erases the parameter
- **WHEN** NX source declares `export external component <SkiaLayout TItem:type itemsSource?:TItem+ />`
- **AND** a caller requests C# output
- **THEN** the generated `SkiaLayout` record SHALL declare `itemsSource` as a nullable list of `object`
- **AND** SHALL NOT declare a generic type parameter or a `TItem` property

#### Scenario: TypeScript external contract is generic with an unknown default
- **WHEN** NX source declares `export external component <SkiaLayout TItem:type itemsSource?:TItem+ />`
- **AND** a caller requests TypeScript output
- **THEN** the generated type SHALL be declared as `SkiaLayout<TItem = unknown>` with `itemsSource` as an optional property whose type is an array of `TItem`
- **AND** SHALL NOT declare a `TItem` property
- **AND** a caller writing `SkiaLayout` with no argument SHALL get `itemsSource` typed as an optional array of `unknown`

#### Scenario: Derived contract carries a parameter inherited across modules
- **WHEN** a library's `base.nx` declares `export abstract external component <ItemsBase TItem:type items?:TItem+ />`
- **AND** its `derived.nx` declares `export external component <ContactList extends ItemsBase extra?:TItem+ />`
- **THEN** the generated TypeScript SHALL declare `ContactList<TItem = unknown>` extending `ItemsBase<TItem>` with `extra` as an optional array of `TItem`
- **AND** the generated C# `ContactList` SHALL declare `extra` as a nullable list of `object` with no `TItem` anywhere

#### Scenario: State and update companion erase the parameter
- **WHEN** NX source declares `export external component <Picker TItem:type items?:TItem+ /> = { state { sel?:TItem } }`
- **THEN** the generated TypeScript `Picker_state` SHALL type `sel` as the optional property `sel?: unknown` and `Picker_update` SHALL type it as `sel?: unknown | null`, since `sel` is clearable
- **AND** the generated C# `Picker_state` SHALL declare `Sel` as nullable `object` and `Picker_update` SHALL expose it as `NxOptional<object?>`
- **AND** neither language's output SHALL declare a `TItem` member on either type

### Requirement: Generated contracts carry a generic record as a generic type in both languages
The C# and TypeScript `typegen` emitters SHALL declare an exported generic record with one generic
parameter per record type parameter, in declaration order and under the parameter's authored name,
SHALL type each field that mentions a parameter with that generic parameter, and SHALL NOT emit a
member for a type parameter. Both emitters SHALL render an applied type as the instantiation of
that generic type, with arguments in the record's declaration order regardless of the order the
source wrote them, and with each argument mapped as that language maps the same type anywhere else.
Unlike a component contract, a C# generic record SHALL NOT be erased, because a host names the
concrete instantiation at its own deserialization site. An applied type whose argument is a
component type parameter SHALL follow the component rule for that argument: `object` in C#, the
generic parameter in TypeScript. A generic record's `<Name>_update` companion SHALL declare the
same generic parameters the record does and SHALL type each of its fields as the record types the
field it patches, so that a patch of one instantiation is usable at that instantiation; in C# it
SHALL extend `NxUpdate<Name<T, …>>` and its `<Name>Properties` key table SHALL declare the same
parameters. A component's state companion SHALL continue to erase the *component's* type
parameters, because the type it patches is concrete. Neither companion SHALL put a type argument
on the wire: the discriminator and the field names are unchanged by the parameters.

#### Scenario: C# declares a generic record
- **WHEN** NX source declares `export type Range = { T:type start:T end:T endInclusive:boolean }`
- **AND** a caller requests C# output
- **THEN** the generated record SHALL be declared as `Range<T>` with `Start` and `End` typed `T` and `EndInclusive` typed `bool`
- **AND** SHALL NOT declare a `T` property

#### Scenario: TypeScript declares a generic record
- **WHEN** NX source declares `export type Range = { T:type start:T end:T endInclusive:boolean }`
- **AND** a caller requests TypeScript output
- **THEN** the generated type SHALL be declared as `Range<T>` with `start` and `end` typed `T`
- **AND** SHALL NOT declare a `T` property

#### Scenario: An applied type is rendered as the instantiation
- **WHEN** NX source declares `export type Range = { T:type start:T end:T }` and `export type Slider = { range:<Range T=float64/> marks?:<Range T=int/>+ }`
- **THEN** the generated C# `Slider` SHALL declare `Range` as `Range<double>` and `Marks` as a nullable list of `Range<long>`
- **AND** the generated TypeScript `Slider` SHALL declare `range` as `Range<number>` and `marks` as an optional property whose type is an array of `Range<number>`

#### Scenario: Arguments are emitted in declaration order
- **WHEN** NX source declares `export type Pair = { TKey:type TValue:type key:TKey value:TValue }` and `export type Entry = { p:<Pair TValue=int TKey=string/> }`
- **THEN** the generated C# SHALL type `P` as `Pair<string, long>` and the generated TypeScript SHALL type `p` as `Pair<string, number>`

#### Scenario: A component type parameter as an argument follows the component rule
- **WHEN** NX source declares `export type Range = { T:type start:T end:T }` and `export external component <Slider TValue:type range?:<Range T=TValue/> />`
- **THEN** the generated C# `Slider` SHALL declare `Range` as a nullable `Range<object>`
- **AND** the generated TypeScript `Slider<TValue = unknown>` SHALL declare `range` as the optional property `range?: Range<TValue>`

#### Scenario: The update companion of a generic record carries the parameter
- **WHEN** NX source declares `export type Range = { T:type start:T end:T }`
- **THEN** the generated TypeScript SHALL declare `Range_update<T>` with `start` typed as optional `T`
- **AND** the generated C# SHALL declare `Range_update<T> : NxUpdate<Range<T>>` with `Start` typed `NxOptional<T>`, `Diff(Range<T>, Range<T>)` returning `Range_update<T>`, and a `RangeProperties<T>` key table
- **AND** the serialized patch SHALL carry `"$type": "Range.Update"` and the field names alone, with no type argument

#### Scenario: A patch of a generic record applies to the instantiation it came from
- **WHEN** a C# host diffs two `Range<long>` values, serializes the patch and reads it back through either serializer
- **THEN** applying the patch SHALL produce a `Range<long>` carrying the changed fields
- **AND** reading a patched field SHALL give the record's field type rather than an untyped value

#### Scenario: A component's state companion still erases the component's parameters
- **WHEN** NX source declares `export external component <Picker TItem:type /> = { state { sel?:TItem } <Label /> }`
- **THEN** the generated `Picker_update` SHALL declare no generic parameter, and SHALL type `sel` as `sel?: unknown | null` in TypeScript and `NxOptional<object?>` in C#

### Requirement: Generated contracts refer to prelude types without redeclaring them per module
When an exported contract references a prelude declaration — the record, or one of its derived
companions — the C# `typegen` emitter SHALL render it as the hand-written SDK type for that
declaration, named by the `Nx` prefix over the name typegen gives it: `global::NxLang.Nx.NxRange<…>`
for `Range`, `global::NxLang.Nx.NxRange_update<…>` for `Range.Update`, and
`global::NxLang.Nx.NxRange_property` for `Range.Property`. Their type arguments SHALL be mapped as
`typegen` maps them anywhere else, and no declaration of a prelude type SHALL be emitted into the
generated namespace. The TypeScript emitter SHALL emit each prelude declaration the output
references, generated from the prelude's own declaration through the ordinary emitters, exactly once
per generation: into the shared helper module beside `NxRecord` for library output, and inline for
single-file output, and SHALL emit none when the output references none. A module that declares its
own type under a prelude name SHALL have that name, and its companions' names, rendered as its own
generated types rather than the prelude's. Resolution SHALL be per module: a declaration of the
generating library hides a prelude name in every one of its modules, because a same-library peer is
visible without an import, while an `import` hides it only in the module that wrote it.

#### Scenario: C# maps a range to the SDK type
- **WHEN** NX source declares `export type Slider = { range:<Range T=float64/> marks?:<Range T=int/>+ }`
- **AND** a caller requests C# output
- **THEN** the generated `Slider` SHALL declare `Range` as `global::NxLang.Nx.NxRange<double>` and `Marks` as a nullable list of `global::NxLang.Nx.NxRange<long>`
- **AND** the output SHALL NOT declare a type named `Range` or `NxRange`

#### Scenario: TypeScript library output declares `Range` once in the helper module
- **WHEN** a library has two modules that each export a record with a field typed `<Range T=int/>`
- **AND** a caller requests TypeScript output
- **THEN** the helper module SHALL export `Range<T>` with `start` and `end` typed `T` and `endInclusive` typed `boolean`
- **AND** each generated module SHALL import `Range` from the helper module and type its field `Range<number>`

#### Scenario: TypeScript single-file output inlines `Range`
- **WHEN** a single NX file exports a record with a field typed `<Range T=float64/>` and a caller requests TypeScript output
- **THEN** the one generated file SHALL declare `Range<T>` and type the field `Range<number>`

#### Scenario: Output that uses no prelude type is unchanged
- **WHEN** NX source references no prelude type
- **THEN** the generated C# and TypeScript SHALL be what they were before the prelude existed

#### Scenario: A module's own `Range` is generated as its own
- **WHEN** NX source declares `export type Range = { low:int high:int }` and `export type Chart = { bounds:Range }`
- **THEN** both languages SHALL generate that `Range` record and type `bounds` with it

#### Scenario: A contract field typed by a prelude companion
- **WHEN** NX source declares `export type Patch = { change:<Range.Update T=int/> which:Range.Property }`
- **THEN** the generated TypeScript SHALL type `change` as `Range_update<number>` and `which` as `Range_property`, and SHALL declare both beside `Range`
- **AND** the generated C# SHALL type `Change` as `global::NxLang.Nx.NxRange_update<long>` and `Which` as `global::NxLang.Nx.NxRange_property`
- **AND** neither output SHALL warn that a companion has nowhere to resolve to

#### Scenario: A module that imports a foreign `Range` leaves its sibling the prelude's
- **WHEN** a library has one module that imports a dependency's `Range` and a sibling module that declares a field typed `<Range T=int/>` with no import
- **THEN** the importing module's field SHALL be the dependency's generated type
- **AND** the sibling's field SHALL be the prelude's: `global::NxLang.Nx.NxRange<long>` in C#, and the helper module's `Range<number>` in TypeScript

### Requirement: A C# member typed by a generic update companion names a closed formatter
MessagePack's source generator cannot resolve the open generic a generic update companion carries,
so the C# emitter SHALL declare a closed formatter for each instantiation of such a companion that a
member's own type is, and the member SHALL name it with `[MessagePackFormatter]`. A formatter SHALL
be declared once per generated namespace, not once per module: single-file output declares it beside
the contracts, and library output — whose modules all land in the one namespace whatever their
directory — declares every formatter the library names in one shared file, since a second
declaration of the same class in a namespace does not compile. This SHALL apply only to a companion
another assembly declares — the prelude's, or a dependency's. A companion the *generating library*
declares needs no member attribute at all and SHALL NOT be given one: the source generator has the
declaration in the compilation and closes the companion's own open generic shim per instantiation,
wherever in the member's type it sits. A generated C# file SHALL suppress `MsgPack009`, which counts
formatters against the open generic and so reads two instantiations of one companion as two
formatters for one type; they are not, since the source generator resolves each member by the closed
type its attribute names. One shape has no formatter the generator can resolve: a companion another
assembly declares, reached *below* the member's own type — under a `+` or `*` occurrence — where
there is nowhere to put the attribute and the open generic on the companion is `MsgPack006` from the
compiler itself, which no file-level pragma reaches. A field of that shape SHALL be reported as a
warning naming the field and the companion, rather than generated into a file that does not
compile.

#### Scenario: A member typed by the prelude's update companion
- **WHEN** NX source declares `export type Patch = { change:<Range.Update T=int/> }` and a caller requests C# output
- **THEN** the output SHALL declare a closed formatter over `global::NxLang.Nx.NxRange_update<long>`
- **AND** the `Change` member SHALL carry `[MessagePackFormatter]` naming it

#### Scenario: Two instantiations of one companion in one contract
- **WHEN** NX source declares `export type Patch = { narrow:<Range.Update T=int/> wide:<Range.Update T=float64/> }` and a caller requests C# output
- **THEN** the output SHALL declare a closed formatter for each instantiation, and each member SHALL name its own
- **AND** the output SHALL compile and round-trip both members, with no warning from the generator

#### Scenario: Two modules of a library name one instantiation
- **WHEN** a library has two modules that each declare a member typed `<Range.Update T=int/>` and a caller requests C# output
- **THEN** the output SHALL declare the closed formatter exactly once, in a shared file of its own
- **AND** both members SHALL carry `[MessagePackFormatter]` naming it

#### Scenario: A generic update companion C# cannot format
- **WHEN** NX source declares a field typed `<Range.Update T=int/>+` or `<Range.Update T=int/>*`
- **THEN** C# output SHALL warn, naming the field and the companion
- **AND** a field typed by the update companion of a generic record the library declares itself SHALL NOT warn, at the member's own type or below it
