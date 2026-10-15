## ADDED Requirements

### Requirement: Every component has a derived props record
For every component `T`, concrete, abstract or external, the system SHALL make a derived record
named `T.Props` available without any declaration by the author. `T.Props` SHALL have one field per
prop of `T`, inherited props first and each list in declaration order, each with its declared type,
its optional mark and its `content` modifier, and SHALL carry no field defaults, so a `T.Props`
literal SHALL give every field that is not optional. `T.Props` SHALL have no field for a type
parameter, a state field or an emitted action; a generic component's `T.Props` SHALL declare the
component's type parameters, as a generic record's `R.Update` declares the record's. `T.Props` SHALL be a plain record in every other
respect: it SHALL have its own derived `T.Props.Update` and `T.Props.Property`, SHALL be accepted by
the update intrinsics as a record, SHALL be visible and importable wherever `T` is, and SHALL NOT be
abstract, an action record, or usable as the base of an `extends` clause. No other declaration SHALL
have a derived `Props`, so `T.Props.Props`, `T.State.Props`, `T.Update.Props` and `R.Props` for a
record `R` SHALL be rejected as unknown types.

#### Scenario: Props record has the props and not the state
- **WHEN** a file contains `component <Counter step:int = 1 label?:string /> = { state { count:int = 0 } <Label text={count} /> }` and `let p = <Counter.Props step=2 />`
- **THEN** type checking SHALL accept `Counter.Props` as a record with fields `step:int` and `label?:string`
- **AND** type checking SHALL reject `<Counter.Props step=2 count=1 />` because `count` is a state field, not a prop

#### Scenario: Props record carries no defaults
- **WHEN** a file contains `component <Counter step:int = 1 /> = { <Label /> }` and `let p = <Counter.Props />`
- **THEN** type checking SHALL reject the literal because `step` is required in `Counter.Props`

#### Scenario: Inherited props come first
- **WHEN** a file contains `abstract component <Question label:string help?:string />` and `component <SingleChoice extends Question allowsOther:boolean = false /> = { <Panel /> }` and `let keys: SingleChoice.Props.Property+ = { SingleChoice.Props.Property.label SingleChoice.Props.Property.allowsOther }`
- **THEN** type checking SHALL accept the binding
- **AND** the cases of `SingleChoice.Props.Property` SHALL be ordered `label`, `help`, `allowsOther`

#### Scenario: Props record has its own update record and property union
- **WHEN** a file contains `component <Card title:string subtitle?:string /> = { <Panel /> }` and `let patch = <Card.Props.Update title="New" />` and `let k: Card.Props.Property = {Card.Props.Property.subtitle}`
- **THEN** type checking SHALL accept both bindings
- **AND** evaluating `{apply(<Card.Props title="Old" />, patch)}` SHALL produce a `Card.Props` whose `title` is `New`

#### Scenario: A component with no props has an empty props record
- **WHEN** a file contains `component <Divider /> = { <Line /> }` and `let p = <Divider.Props />`
- **THEN** type checking SHALL accept the binding

#### Scenario: An external component has a props record
- **WHEN** a file contains `external component <SearchBox placeholder?:string /> = { state { query:string } }` and `let p = <SearchBox.Props placeholder="Search" />`
- **THEN** type checking SHALL accept `SearchBox.Props` as a record with the single field `placeholder?:string`

#### Scenario: A generic component's props record keeps its type parameters
- **WHEN** a file contains `component <List TItem:type items:TItem+ /> = { <Panel /> }` and `let p:<List.Props TItem=int/> = <List.Props TItem=int items={ 1 2 } />`
- **THEN** type checking SHALL accept the binding
- **AND** `List.Props.Property` SHALL have the single case `items`

#### Scenario: Only a component has a derived props record
- **WHEN** a file contains `type User = { name:string }` and `component <Card /> = { <Panel /> }` and `let a: User.Props = <User.Props />` and `let b: Card.Props.Props = <Card.Props.Props />`
- **THEN** type checking SHALL reject `User.Props` and `Card.Props.Props` as unknown types

#### Scenario: A props record of an imported component is available through the import
- **WHEN** library `ui` exports `component <Card title:string /> = { <Panel /> }` and a module imports `Card` from it and contains `let p = <Card.Props title="Hi" />`
- **THEN** type checking SHALL accept the reference without the library exporting anything named `Card.Props`
