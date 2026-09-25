# component-contract-inheritance

## MODIFIED Requirements

### Requirement: Derived components inherit type parameters open
The effective contract of a derived component SHALL include every type parameter declared along
its abstract base chain, in declaration order with inherited parameters first, followed by the
derived component's own. An inherited type parameter SHALL remain open: the derived signature MUST
NOT bind it to a type, and a use site of the derived component supplies or omits it exactly as it
would on the base. A derived component that declares a type parameter or a prop with the same name
as an inherited type parameter MUST be rejected.

#### Scenario: Derived component accepts an argument for an inherited type parameter
- **WHEN** a file contains `abstract external component <ItemsBase TItem:type itemsSource?:TItem+ /> external component <ContactList extends ItemsBase spacing?:int />` and `type Contact = { name:string } let contacts:Contact* = {} let v = <ContactList TItem=Contact itemsSource={contacts} spacing=4 />`
- **THEN** analysis SHALL accept the element, checking `itemsSource` against `Contact*`, the write type of `itemsSource?:TItem+` with `TItem` bound to `Contact`

#### Scenario: Derived component body sees the inherited type parameter
- **WHEN** a file contains `abstract component <ItemsBase TItem:type items:TItem+ /> component <Count extends ItemsBase /> = { state { first?:TItem } <Label /> }`
- **THEN** analysis SHALL accept the optional state field `first?:TItem` inside the derived body, reading `first` as `TItem?`

#### Scenario: Derived component adds its own type parameter after the inherited one
- **WHEN** a file contains `abstract component <ItemsBase TItem:type items:TItem+ /> component <Keyed extends ItemsBase TKey:type keys:TKey+ /> = { <Label /> }` and `let v = <Keyed TItem=string TKey=int items={ "a" } keys={ 1 } />`
- **THEN** analysis SHALL accept both the declaration and the element

#### Scenario: Redeclaring an inherited type parameter is rejected
- **WHEN** a file contains `abstract component <ItemsBase TItem:type /> component <Bad extends ItemsBase TItem:type /> = { <Label /> }` and `component <Worse extends ItemsBase TItem:string /> = { <Label /> }`
- **THEN** analysis SHALL reject both `Bad` and `Worse` because `TItem` duplicates an inherited type parameter
