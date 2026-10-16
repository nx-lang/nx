# property-references Specification

## Purpose
Defines the derived `T.Property` constant union every record-shaped declaration and every component
state block gets: its cases, how it behaves as a constant union in type, expression, pattern, and
bare-name positions, how the bare `Property` name resolves inside a component, and its wire form.
## Requirements
### Requirement: Every record-shaped declaration has a derived property union
The system SHALL make a derived constant union named `T.Property` available, without any
declaration by the author, for every record-shaped declaration `T` in scope: a `type` record, an
`action` record, an inline emitted action, and a component's derived `T.State` and `T.Props`
records.
`T.Property` SHALL have exactly one case per effective field of `T` (including inherited fields),
named by that field, in declaration order with inherited fields first; whether a field is optional,
defaulted or required SHALL NOT affect whether it has a case. A component is not a record-shaped
declaration, so it SHALL NOT have a `T.Property`, stateful or not, external or not; the property
union of a component's state is `T.State.Property` and of its props `T.Props.Property`. `T.Property` SHALL have no base and no payload case,
so that every case is a constant case. `T.Property` SHALL be visible and importable wherever `T`
is, SHALL NOT be usable as the base of an `extends` clause, and SHALL NOT itself have a derived
`Property`, `Update`, `State` or `Props`; nor SHALL `T.Update` have a derived `Property`. A type reference or
member access spelled `X.Property` that does not resolve to a derived property union SHALL be a
diagnostic naming `X`, since the `.Property` suffix always names a derived union. When `X` is a
component, the diagnostic SHALL say that a component has no property union and SHALL suggest
`X.State.Property` when the component has state and `X.Props.Property` when it has none.

#### Scenario: Property union of a plain record lists its fields
- **WHEN** a file contains `type User = { name:string email?:string } let key: User.Property = {User.Property.email}`
- **THEN** type checking SHALL accept `User.Property` as a constant union with cases `name` and `email`
- **AND** the value of `key` SHALL be the `email` case of `User.Property`

#### Scenario: A component has no property union of its own
- **WHEN** a file contains `component <Counter step:int = 1 /> = { state { count:int = 0 } <Label text={count} /> }` and `let key = {Counter.Property.step}`
- **THEN** type checking SHALL reject `Counter.Property` as an unknown name
- **AND** the diagnostic SHALL suggest `Counter.State.Property`

#### Scenario: Property union of a component's state is the state record's union
- **WHEN** a file contains `component <Counter step:int = 1 /> = { state { count:int = 0 } <Label text={count} /> }` and `let key: Counter.State.Property = {Counter.State.Property.count}`
- **THEN** type checking SHALL accept `Counter.State.Property` with the single case `count`

#### Scenario: Property union of a generic component's state leaves out its type parameters
- **WHEN** a file contains `component <List TItem:type items:TItem+ /> = { state { selected:int = 0 } <Label /> }` and `let k: List.State.Property = {List.State.Property.selected}`
- **THEN** type checking SHALL accept `List.State.Property` with the single case `selected`
- **AND** type checking SHALL reject `List.State.Property.TItem`

#### Scenario: Property union includes inherited fields in declaration order
- **WHEN** a file contains `abstract type Named = { name:string } type User extends Named = { email:string }` and `let keys: User.Property+ = { User.Property.name User.Property.email }`
- **THEN** type checking SHALL accept both cases on `User.Property`
- **AND** the cases of `User.Property` SHALL be ordered `name` then `email`

#### Scenario: A property union of an imported record is available through the import
- **WHEN** library `people` exports `type User = { name:string }` and a module imports `User` from it and contains `let key: User.Property = {User.Property.name}`
- **THEN** type checking SHALL accept the reference without the library exporting anything named `User.Property`

#### Scenario: Nested derived names do not exist
- **WHEN** a file contains `type User = { name:string } let a = {User.Property.Property.name} let b = {User.Update.Property.name} let c = <User.Property.Update name="Ada" />`
- **THEN** type checking SHALL reject `User.Property.Property`, `User.Update.Property`, and `User.Property.Update` as unknown types

#### Scenario: Property union for an unknown target is rejected
- **WHEN** a file contains `let key = {Icons.Property.size}` and nothing named `Icons` is declared
- **THEN** type checking SHALL reject the reference with a diagnostic saying `Icons` is not a record, action, or component

#### Scenario: A component without state has no property union
- **WHEN** a file contains `component <Form title:string /> = { <Panel /> } let key = {Form.Property.title}`
- **THEN** type checking SHALL reject `Form.Property` with a diagnostic saying the component has no property union
- **AND** the diagnostic SHALL suggest `Form.Props.Property`

#### Scenario: Property union cannot be extended
- **WHEN** a file contains `type User = { name:string } type Extra extends User.Property = | more`
- **THEN** type checking SHALL reject the `extends` clause because `User.Property` is a derived property union

### Requirement: A property union behaves as a constant union everywhere
`T.Property` SHALL be accepted and SHALL behave as a constant union at every site that accepts a
union type: a qualified case `T.Property.f` in expression position, a bare case name resolved
contextually against a site whose expected type is `T.Property` (including through an occurrence,
so at a site reading `T.Property?`, `T.Property+` or `T.Property*` the name resolves against
`T.Property`), a bare or qualified case pattern in a match, exhaustiveness checking over its
cases, and the diagnostics a constant union produces for an unknown case. Two property unions
derived from different declarations SHALL be distinct nominal types, and a case of one SHALL NOT be
accepted where the other is expected, even when the case names coincide.

#### Scenario: Bare case name resolves at a property-typed site
- **WHEN** a file contains `type Contact = { title:string subtitle:string } component <Table sortBy:Contact.Property /> = { <div /> } let v = <Table sortBy=subtitle />`
- **THEN** type checking SHALL accept `subtitle` as a value of type `Contact.Property`
- **AND** interpretation SHALL produce the same value as `sortBy={Contact.Property.subtitle}`

#### Scenario: Bare case name resolves at an optional property-typed site
- **WHEN** a file contains `type Contact = { title:string subtitle:string } component <Table sortBy?:Contact.Property /> = { <div /> } let v = <Table sortBy=subtitle /> let w = <Table />`
- **THEN** type checking SHALL accept `subtitle` by resolving against `Contact.Property`, the item type of the site's read type `Contact.Property?`
- **AND** `w` SHALL leave `sortBy` empty

#### Scenario: Bare case names resolve at a list-typed site
- **WHEN** a file contains `type Contact = { title:string subtitle:string } component <Table columns:Contact.Property+ /> = { <div /> } let v = <Table columns=title />`
- **THEN** type checking SHALL resolve `title` against the sequence's item type `Contact.Property`
- **AND** the resolved exactly-one value SHALL lift to a sequence of one at the `+` site

#### Scenario: Unknown case at a property-typed site names the candidates
- **WHEN** a file contains `type Contact = { title:string subtitle:string } component <Table sortBy:Contact.Property /> = { <div /> } let v = <Table sortBy=titel />`
- **THEN** type checking SHALL reject `titel` with a diagnostic that lists `title` and `subtitle` as the candidates

#### Scenario: Match over a property union is checked for exhaustiveness
- **WHEN** a file contains `type User = { name:string email?:string } let label(key:User.Property) = {if key is { name => "Name" email => "Email" }}`
- **THEN** type checking SHALL accept the match as exhaustive
- **AND** type checking SHALL reject the same match with the `email` arm removed and no `else` arm as non-exhaustive

#### Scenario: Qualified case pattern is accepted in a match
- **WHEN** a file contains `type User = { name:string } let label(key:User.Property) = {if key is { User.Property.name => "Name" }}`
- **THEN** type checking SHALL accept the arm

#### Scenario: Property unions of different records are distinct types
- **WHEN** a file contains `type User = { name:string } type Team = { name:string } let key: User.Property = {Team.Property.name}`
- **THEN** type checking SHALL reject the initializer because `Team.Property.name` is not a `User.Property`

#### Scenario: Property union values can be compared
- **WHEN** a file contains `type User = { name:string email?:string } let same = {User.Property.name == User.Property.name} let other = {User.Property.name == User.Property.email}`
- **THEN** evaluating `same` SHALL produce `true` and `other` SHALL produce `false`

### Requirement: A property union case has the wire form of a constant case
The canonical value encoding of a `T.Property` case SHALL be the bare string of the field name,
with no `$type` discriminator, in both the JSON and MessagePack forms. Decoding host input where a
`T.Property` is expected SHALL accept the bare string of any case and SHALL reject any other
string, naming the expected union and the candidates. A list of property union cases SHALL encode
as a list of bare strings.

#### Scenario: JSON encoding of a property union case is the bare field name
- **WHEN** `<Table sortBy={Contact.Property.subtitle} />` is evaluated and the result is encoded as JSON
- **THEN** the value at `sortBy` SHALL be the string `"subtitle"`

#### Scenario: Host input with a valid case decodes
- **WHEN** a host supplies the JSON string `"title"` for a prop typed `Contact.Property`
- **THEN** the runtime SHALL accept it as the `title` case of `Contact.Property`

#### Scenario: Host input with an unknown case is rejected
- **WHEN** a host supplies the JSON string `"nickname"` for a prop typed `Contact.Property`
- **THEN** the runtime SHALL reject the value with a diagnostic naming `Contact.Property` and the field `nickname`

