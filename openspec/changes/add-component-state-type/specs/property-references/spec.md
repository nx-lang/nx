## MODIFIED Requirements

### Requirement: Every record-shaped declaration has a derived property union
The system SHALL make a derived constant union named `T.Property` available, without any
declaration by the author, for every record-shaped declaration `T` in scope: a `type` record, an
`action` record, an inline emitted action, and a component's derived `T.State` record.
`T.Property` SHALL have exactly one case per effective field of `T` (including inherited fields),
named by that field, in declaration order with inherited fields first; whether a field is optional,
defaulted or required SHALL NOT affect whether it has a case. A component is not a record-shaped
declaration, so it SHALL NOT have a `T.Property`, stateful or not, external or not; the property
union of a component's state is `T.State.Property`. `T.Property` SHALL have no base and no payload case,
so that every case is a constant case. `T.Property` SHALL be visible and importable wherever `T`
is, SHALL NOT be usable as the base of an `extends` clause, and SHALL NOT itself have a derived
`Property`, `Update` or `State`; nor SHALL `T.Update` have a derived `Property`. A type reference or
member access spelled `X.Property` that does not resolve to a derived property union SHALL be a
diagnostic naming `X`, since the `.Property` suffix always names a derived union. When `X` is a
component, the diagnostic SHALL say that a component has no property union and, when the component
has state, SHALL suggest `X.State.Property`.

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

#### Scenario: Property union cannot be extended
- **WHEN** a file contains `type User = { name:string } type Extra extends User.Property = | more`
- **THEN** type checking SHALL reject the `extends` clause because `User.Property` is a derived property union

## REMOVED Requirements

### Requirement: The bare `Property` name resolves to the enclosing component's property union
**Reason**: A component has no `T.Property` of its own, and its state has `T.State.Property`.
The bare `State` name inside a component body (see `component-state-type`) replaces the bare
`Property` shorthand, and `Property` is an ordinary name.
**Migration**: Write `State.Property` where a component body wrote `Property`, as a type or as the
head of a member access (`{State.Property.count}`), and `Counter.State.Property` where code outside
the component named the state union as `Counter.Property`.
