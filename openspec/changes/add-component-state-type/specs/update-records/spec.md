## MODIFIED Requirements

### Requirement: Every record-shaped declaration has a derived update record
The system SHALL make a derived record named `T.Update` available, without any declaration by the
author, for every record-shaped declaration `T` in scope: a `type` record, an `action` record, an
inline emitted action, a component, and a component's derived `T.State` record. `T.Update` SHALL
have exactly the effective fields of `T` (including inherited fields), each with the same declared
type as in `T`, and every field SHALL be optional in the sense that it may be absent from the value.
A field's optional mark in `T` SHALL be remembered by `T.Update` as the field being *clearable*:
only a field that is optional in `T` may be present in the update with the empty value. A
component's `T.Update` SHALL be derived from its effective props, inherited props first, SHALL
exclude its type parameter definitions and its state fields, and SHALL exist for every component
whether or not it declares state; the update record of a component's state is `T.State.Update`. An
update record SHALL NOT be an action record, SHALL NOT be a plain record, SHALL NOT be abstract, and
SHALL NOT be usable as the base of an `extends` clause. An element or record-literal tag spelled
`X.Update` that does not resolve to a derived update record SHALL be a diagnostic naming `X`, rather
than being treated as an unknown host element, since the `.Update` suffix always names a derived
record.

#### Scenario: Update record of a plain record has the same fields
- **WHEN** a file contains `type User = { name:string email?:string } let patch = <User.Update name="Ada" />`
- **THEN** type checking SHALL accept `User.Update` as a record type with fields `name:string` and `email?:string`
- **AND** the value of `patch` SHALL be of type `User.Update`

#### Scenario: Update record of a component is derived from its props
- **WHEN** a file contains `component <Counter step:int = 1 label?:string /> = { state { count:int = 0 } <Label text={count} /> }` and `let more(): Counter.Update = <Counter.Update step=2 label={} />`
- **THEN** type checking SHALL accept `Counter.Update` with the fields `step:int` and `label?:string`, with `label` clearable
- **AND** type checking SHALL reject `<Counter.Update count=0 />` because `count` is a state field, not a prop

#### Scenario: Update record of a component's state is the state record's update
- **WHEN** a file contains `component <Counter step:int = 1 /> = { state { count:int = 0 } <Label text={count} /> }` and `let reset(): Counter.State.Update = <Counter.State.Update count=0 />`
- **THEN** type checking SHALL accept `Counter.State.Update` with the single field `count:int`

#### Scenario: Update record includes inherited fields
- **WHEN** a file contains `abstract type Named = { name:string } type User extends Named = { email:string }` and `let patch = <User.Update name="Ada" email="ada@example.com" />`
- **THEN** type checking SHALL accept both `name` and `email` on `User.Update`

#### Scenario: Update record of a component includes inherited props
- **WHEN** a file contains `abstract component <Control enabled:boolean = true />` and `component <Toggle extends Control on:boolean /> = { <Label /> }` and `let patch = <Toggle.Update enabled=false on=true />`
- **THEN** type checking SHALL accept both `enabled` and `on` on `Toggle.Update`

#### Scenario: Update record is not an action and cannot be extended
- **WHEN** a file contains `type User = { name:string } action Rename extends User.Update = { }`
- **THEN** type checking SHALL reject the `extends` clause because `User.Update` is a derived update record

#### Scenario: Update record of a nested update record does not exist
- **WHEN** a file contains `type User = { name:string } let patch = <User.Update.Update name="Ada" />`
- **THEN** type checking SHALL reject `User.Update.Update` as an unknown type

#### Scenario: Update tag for an unknown target is rejected
- **WHEN** a file contains `let icon = <Icons.Update size=3 />` and nothing named `Icons` is declared
- **THEN** type checking SHALL reject the tag with a diagnostic saying `Icons` is not a record, action, or component

#### Scenario: A component without state has a props update record
- **WHEN** a file contains `component <Form title:string /> = { <Panel /> }` and `let patch = <Form.Update title="Sign in" />`
- **THEN** type checking SHALL accept `Form.Update` with the single field `title:string`

## REMOVED Requirements

### Requirement: The bare `Update` tag resolves to the enclosing component's update record
**Reason**: A component's `T.Update` now patches its props, and a handler patches state with the
component's `T.State.Update`. The bare `State` name inside a component body (see
`component-state-type`) replaces the bare `Update` shorthand, and `Update` is an ordinary name.
**Migration**: Write `<State.Update …/>` where a component body wrote `<Update …/>`, and
`<Counter.State.Update …/>` where code outside the component wrote a state patch as
`<Counter.Update …/>`.
