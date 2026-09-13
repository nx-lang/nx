## ADDED Requirements

### Requirement: NX IR erases component type parameters
NX IR SHALL NOT carry component type parameters. A component's prop schema and state schema in IR
SHALL describe each field with the type parameter replaced by the top type `object`, the
component's derived update record SHALL describe each field the same way, and a component
descriptor in IR SHALL NOT include a property for a type argument the source bound. IR emitted for
a program with generic components SHALL remain deterministic and boundary-clean.

#### Scenario: Prop schema carries the erased type
- **WHEN** NX source declares `external component <SkiaLayout TItem:type itemsSource:TItem[]? />`
- **AND** NX IR is emitted for the program
- **THEN** the component declaration in IR SHALL include a prop schema for `itemsSource` typed as a nullable list of `object`
- **AND** it SHALL NOT include a prop schema or any other entry for `TItem`

#### Scenario: Descriptor omits the type argument
- **WHEN** NX source declares `type Contact = { name:string } external component <SkiaLayout TItem:type itemsSource:TItem[]? /> let v = <SkiaLayout TItem=Contact itemsSource={} />`
- **AND** NX IR is emitted for the program
- **THEN** the descriptor expression for `v` SHALL carry a property for `itemsSource` and no property for `TItem`

#### Scenario: Update record carries the erased type
- **WHEN** NX source declares `component <List TItem:type items:TItem[]? /> = { state { sel:TItem? = null } <Label /> } let u = <List.Update sel=null />`
- **AND** NX IR is emitted for the program
- **THEN** the `List.Update` declaration in IR SHALL type `sel` as nullable `object` and SHALL NOT mention `TItem`
