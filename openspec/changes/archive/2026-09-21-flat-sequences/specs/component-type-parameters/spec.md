## MODIFIED Requirements

### Requirement: A use site supplies a type argument as a bare type name
An element that targets a component with type parameters MAY bind each type parameter by name to
an unbraced single identifier. The system SHALL resolve that identifier against the types visible
at the use site — primitive type names, and every record, union, alias, and type parameter in
scope there — and SHALL NOT consult value bindings. The resolved type MUST be an item type: an
identifier that resolves, directly or through an alias chain, to a sequence type SHALL be rejected
with a diagnostic explaining that a type argument must not be a sequence and naming the alias. A
type parameter binding MUST be a plain, unconditional property entry: a braced expression, a quoted
string, a numeric literal, or an entry inside an `if`, condition-list, or `match` property fragment
at a type-parameter site SHALL be rejected. The effective prop types of the component SHALL then be
checked with every occurrence of the parameter replaced by the argument.

#### Scenario: A type argument fixes the element type of a list prop
- **WHEN** a file contains `type Contact = { name:string } external component <SkiaLayout TItem:type itemsSource:TItem[]? />` and `let contacts:Contact[] = {} let v = <SkiaLayout TItem=Contact itemsSource={contacts} />`
- **THEN** analysis SHALL accept the element, checking `itemsSource` against `Contact[]?`

#### Scenario: A mismatch is reported against the substituted type
- **WHEN** a file contains `type Contact = { name:string } external component <SkiaLayout TItem:type itemsSource:TItem[]? />` and `let v = <SkiaLayout TItem=Contact itemsSource={ "a" "b" } />`
- **THEN** analysis SHALL reject `itemsSource` as a mismatch between the list `string[]` and `Contact[]?`

#### Scenario: A primitive, an alias, and a union are all acceptable arguments
- **WHEN** a file contains `type Fit = fill | cover type Label = string external component <List TItem:type items:TItem[]? />` and `let a = <List TItem=int items={ 1 2 } /> let b = <List TItem=Label items={ "x" } /> let c = <List TItem=Fit items=fill /> let d = <List TItem=object items={ 1 "two" } />`
- **THEN** analysis SHALL accept all four elements, `object` being a primitive type name like `int`

#### Scenario: An alias to a sequence type is not an acceptable argument
- **WHEN** a file contains `type Names = string[] external component <List TItem:type items:TItem[]? />` and `let v = <List TItem=Names items={ "x" } />`
- **THEN** analysis SHALL reject the binding `TItem=Names`
- **AND** the diagnostic SHALL explain that a type argument must not be a sequence and SHALL name `Names`

#### Scenario: An enclosing component forwards its own type parameter
- **WHEN** a file contains `external component <SkiaLayout TItem:type itemsSource:TItem[]? /> component <Section TItem:type items:TItem[] /> = { <SkiaLayout TItem=TItem itemsSource={items} /> }`
- **THEN** analysis SHALL resolve the argument `TItem` to the type parameter of `Section`
- **AND** SHALL accept `itemsSource={items}`

#### Scenario: A braced or quoted argument is rejected
- **WHEN** a file contains `type Contact = { name:string } external component <List TItem:type />` and `let a = <List TItem={Contact} /> let b = <List TItem="Contact" />`
- **THEN** analysis SHALL reject both bindings
- **AND** each diagnostic SHALL direct the author to the bare form `TItem=Contact`

#### Scenario: A conditional argument is rejected
- **WHEN** a file contains `type Contact = { name:string } external component <List TItem:type /> let flag = true` and `let v = <List if flag { TItem=Contact } />`
- **THEN** analysis SHALL reject the binding because a type argument cannot be conditional

#### Scenario: An unknown type name suggests a near match
- **WHEN** a file contains `type Contact = { name:string } external component <List TItem:type />` and `let v = <List TItem=Contatc />`
- **THEN** analysis SHALL reject `Contatc` because it is not a visible type
- **AND** the diagnostic SHALL suggest `Contact`

#### Scenario: A value binding of the same name is not an argument
- **WHEN** a file contains `external component <List TItem:type /> let Contact = "a"` and `let v = <List TItem=Contact />`
- **THEN** analysis SHALL reject `Contact` because no visible type has that name

#### Scenario: Binding a type argument on a component without that parameter is an unknown property
- **WHEN** a file contains `component <Button text:string /> = { <Label /> }` and `let v = <Button text="ok" TItem=string />`
- **THEN** analysis SHALL reject `TItem` as a property `Button` does not have
