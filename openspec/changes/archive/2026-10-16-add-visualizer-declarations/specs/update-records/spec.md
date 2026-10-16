## MODIFIED Requirements

### Requirement: Every record-shaped declaration has a derived update record
The system SHALL make a derived record named `T.Update` available, without any declaration by the
author, for every record-shaped declaration `T` in scope: a `type` record, an `action` record, an
inline emitted action, and a component's derived `T.State` and `T.Props` records. `T.Update` SHALL have exactly
the effective fields of `T` (including inherited fields), each with the same declared type as in
`T`, and every field SHALL be optional in the sense that it may be absent from the value. A field's
optional mark in `T` SHALL be remembered by `T.Update` as the field being *clearable*: only a field
that is optional in `T` may be present in the update with the empty value. A component is not a
record-shaped declaration, so it SHALL NOT have a `T.Update`, stateful or not, external or not; the
update record of a component's state is `T.State.Update` and of its props `T.Props.Update`. An update record SHALL NOT be an action
record, SHALL NOT be a plain record, SHALL NOT be abstract, and SHALL NOT be usable as the base of an
`extends` clause. An element or record-literal tag spelled `X.Update` that does not resolve to a
derived update record SHALL be a diagnostic naming `X`, rather than being treated as an unknown host
element, since the `.Update` suffix always names a derived record. When `X` is a component, the
diagnostic SHALL say that a component has no update record and SHALL suggest `X.State.Update` when
the component has state and `X.Props.Update` when it has none.

#### Scenario: Update record of a plain record has the same fields
- **WHEN** a file contains `type User = { name:string email?:string } let patch = <User.Update name="Ada" />`
- **THEN** type checking SHALL accept `User.Update` as a record type with fields `name:string` and `email?:string`
- **AND** the value of `patch` SHALL be of type `User.Update`

#### Scenario: A component has no update record of its own
- **WHEN** a file contains `component <Counter step:int = 1 /> = { state { count:int = 0 } <Label text={count} /> }` and `let more = <Counter.Update step=2 />`
- **THEN** type checking SHALL reject `Counter.Update` as an unknown type
- **AND** the diagnostic SHALL suggest `Counter.State.Update`

#### Scenario: Update record of a component's state is the state record's update
- **WHEN** a file contains `component <Counter step:int = 1 /> = { state { count:int = 0 } <Label text={count} /> }` and `let reset(): Counter.State.Update = <Counter.State.Update count=0 />`
- **THEN** type checking SHALL accept `Counter.State.Update` with the single field `count:int`

#### Scenario: Update record includes inherited fields
- **WHEN** a file contains `abstract type Named = { name:string } type User extends Named = { email:string }` and `let patch = <User.Update name="Ada" email="ada@example.com" />`
- **THEN** type checking SHALL accept both `name` and `email` on `User.Update`

#### Scenario: Update record is not an action and cannot be extended
- **WHEN** a file contains `type User = { name:string } action Rename extends User.Update = { }`
- **THEN** type checking SHALL reject the `extends` clause because `User.Update` is a derived update record

#### Scenario: Update record of a nested update record does not exist
- **WHEN** a file contains `type User = { name:string } let patch = <User.Update.Update name="Ada" />`
- **THEN** type checking SHALL reject `User.Update.Update` as an unknown type

#### Scenario: Update tag for an unknown target is rejected
- **WHEN** a file contains `let icon = <Icons.Update size=3 />` and nothing named `Icons` is declared
- **THEN** type checking SHALL reject the tag with a diagnostic saying `Icons` is not a record, action, or component

#### Scenario: A component without state has no update record
- **WHEN** a file contains `component <Form title:string /> = { <Panel /> }` and `let patch = <Form.Update title="Sign in" />`
- **THEN** type checking SHALL reject `Form.Update` with a diagnostic saying the component has no update record
- **AND** the diagnostic SHALL suggest `Form.Props.Update`
