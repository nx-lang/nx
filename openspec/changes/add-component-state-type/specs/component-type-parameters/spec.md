## MODIFIED Requirements

### Requirement: Type parameters are not props and leave no trace below type checking
A type parameter SHALL NOT be a prop: it SHALL NOT be a value in the component body, SHALL NOT be
required at a use site, SHALL NOT be a field of the component's runtime record, SHALL NOT be a
case of any derived property union, and SHALL NOT be a field of any update companion. A
type-argument binding SHALL be removed from the element once type checking has consumed it, so
that interpretation, code generation, and every other consumer below the type checker observe an
element with no such binding. An element that binds a type argument SHALL have the component's
nominal type, the same type as an element of that component with a different argument or none.

#### Scenario: A type parameter is not a value in the body
- **WHEN** a file contains `component <List TItem:type /> = { <Label text={TItem} /> }`
- **THEN** analysis SHALL reject `TItem` as an undefined variable

#### Scenario: A type parameter is never a missing property
- **WHEN** a file contains `external component <List TItem:type items?:TItem+ />` and `let v = <List items={} />`
- **THEN** analysis SHALL NOT report `TItem` as a required property

#### Scenario: The runtime record has no type-parameter field
- **WHEN** a file contains `type Contact = { name:string } external component <SkiaLayout TItem:type itemsSource?:TItem+ />` and `let v = <SkiaLayout TItem=Contact itemsSource={} />`
- **AND** `v` is evaluated
- **THEN** the resulting external component record SHALL have `itemsSource` empty and SHALL NOT have a `TItem` field
- **AND** its JSON serialization SHALL NOT contain a `TItem` key

#### Scenario: The property unions of a generic stateful component leave out its type parameters
- **WHEN** a file contains `component <List TItem:type items:TItem+ /> = { state { selected:int = 0 } <Label /> }` and `let k:List.Property = {List.Property.items}` and `let s:List.State.Property = {List.State.Property.selected}`
- **THEN** type checking SHALL accept `List.Property` with the single case `items` and `List.State.Property` with the single case `selected`
- **AND** SHALL reject `List.Property.TItem`

#### Scenario: Elements with different arguments have the same type
- **WHEN** a file contains `type Contact = { name:string } external component <List TItem:type items?:TItem+ />` and `let a:List = <List TItem=Contact /> let b:List = <List TItem=int /> let c:List = <List />`
- **THEN** analysis SHALL accept all three bindings at the type `List`
