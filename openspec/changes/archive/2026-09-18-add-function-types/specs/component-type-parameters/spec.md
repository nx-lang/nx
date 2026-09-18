## ADDED Requirements

### Requirement: A type argument is substituted through a function-typed prop
When a use site binds a type argument, the substitution of the argument for the parameter SHALL
reach every occurrence of the parameter inside a function-typed prop — its parameter types and its
result type — so a function bound to that prop is checked against the substituted function type.
When the use site binds no argument, the parameter SHALL be substituted by position: the bottom
type where the prop produces a value of it (a list of items, as elsewhere) and the top type where
the prop consumes one (a function type's parameter, which the host calls with items of a type the
site never named), so that no template assuming a particular item type satisfies the prop; a
diagnostic against such a prop SHALL name the parameter and show the `Name=` form.

#### Scenario: A template is checked at the substituted item type
- **WHEN** a file contains `abstract external component <DrawnNode /> type Contact = { name:string } type Post = { title:string } external component <SkiaLayout TItem:type ItemsSource:TItem[]? ItemTemplate:(<function Item:TItem Index:int />: DrawnNode)? /> let <ContactRow Item:Contact Index:int />: DrawnNode = <DrawnNode /> let <PostRow Item:Post Index:int />: DrawnNode = <DrawnNode /> let contacts:Contact[] = {}` and `let ok = <SkiaLayout TItem=Contact ItemsSource={contacts} ItemTemplate={ContactRow} /> let bad = <SkiaLayout TItem=Contact ItemsSource={contacts} ItemTemplate={PostRow} />`
- **THEN** analysis SHALL accept `ok`
- **AND** SHALL reject `bad` as a mismatch between `<function Item:Post Index:int />: DrawnNode` and
  `<function Item:Contact Index:int />: DrawnNode`

#### Scenario: A template bound without a type argument names the parameter
- **WHEN** a file contains `abstract external component <DrawnNode /> type Contact = { name:string } external component <SkiaLayout TItem:type ItemTemplate:(<function Item:TItem />: DrawnNode)? /> let <ContactRow Item:Contact />: DrawnNode = <DrawnNode /> let v = <SkiaLayout ItemTemplate={ContactRow} />`
- **THEN** analysis SHALL reject `ItemTemplate`
- **AND** the diagnostic SHALL say that `TItem` was not specified and SHALL show `TItem=`

#### Scenario: A forwarded parameter reaches a function-typed prop
- **WHEN** a file contains `abstract external component <DrawnNode /> external component <SkiaLayout TItem:type ItemsSource:TItem[]? ItemTemplate:(<function Item:TItem Index:int />: DrawnNode)? /> component <Section extends DrawnNode TItem:type Items:TItem[] Row:<function Item:TItem Index:int />: DrawnNode /> = { <SkiaLayout TItem=TItem ItemsSource={Items} ItemTemplate={Row} /> }`
- **THEN** analysis SHALL accept the body, checking `Row` against the prop with `Section`'s own
  parameter substituted
