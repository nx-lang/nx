## MODIFIED Requirements

### Requirement: A component signature declares type parameters as leading `type`-typed properties
A component signature SHALL accept a property definition whose declared type is the keyword
`type`, and SHALL treat each such definition as a type parameter of the component. Type
parameter definitions MUST precede every other property definition in the signature, MUST NOT
carry a default value, MUST NOT carry a modifier such as `content`, MUST NOT carry an occurrence
suffix on `type` or the `?` mark on the name, MUST NOT take the name of a primitive type or of the
built-in `Element` type, and MUST have a name distinct from every other type parameter and prop of
the component. The system SHALL reject a violation of any of these rules with a diagnostic that
names the offending definition. A type parameter is a type in the component's props, state, and
body only: an emitted action declared inline in the signature is an action record of its own,
checked and generated outside the component, and a payload field typed by a type parameter SHALL be
rejected with a diagnostic that names the action, the field, and the parameter.

#### Scenario: Component declares a type parameter and uses it in a later prop
- **WHEN** a file contains `external component <SkiaLayout TItem:type itemsSource?:TItem+ />`
- **THEN** analysis SHALL accept the declaration
- **AND** SHALL treat `TItem` as a type parameter of `SkiaLayout` and `itemsSource` as its only prop

#### Scenario: Type parameter is usable in the component body
- **WHEN** a file contains `component <First TItem:type items:TItem+ /> = { state { first?:TItem } <Label text="ok" /> }`
- **THEN** analysis SHALL accept the optional state field and SHALL read `first` as `TItem?` inside the body

#### Scenario: Type parameter after a regular prop is rejected
- **WHEN** a file contains `component <Bad items:object+ TItem:type /> = { <Label /> }`
- **THEN** analysis SHALL reject `TItem` because type parameters must precede every other property

#### Scenario: Type parameter with a default is rejected
- **WHEN** a file contains `component <Bad TItem:type = object /> = { <Label /> }`
- **THEN** parsing or analysis SHALL reject the definition because a type parameter cannot have a default

#### Scenario: Type parameter with a suffix or a modifier is rejected
- **WHEN** a file contains `component <Bad TItem:type? /> = { <Label /> }`, `component <Worse content TItem:type /> = { <Label /> }` and `component <Worst TItem?:type /> = { <Label /> }`
- **THEN** parsing or analysis SHALL reject all three definitions

#### Scenario: Type parameter named after a primitive is rejected
- **WHEN** a file contains `external component <List string:type items?:string+ />`
- **THEN** analysis SHALL reject the definition because a type parameter cannot take the name of a primitive type
- **AND** the diagnostic SHALL name `string`

#### Scenario: Type parameter named after the built-in Element type is rejected
- **WHEN** a file contains `component <List Element:type slot?:Element /> = { <Label /> }`
- **THEN** analysis SHALL reject the definition because a type parameter cannot take the name of the built-in `Element` type

#### Scenario: Duplicate type parameter or prop name is rejected
- **WHEN** a file contains `component <Bad TItem:type TItem:type /> = { <Label /> }` and `component <Worse TItem:type TItem:string /> = { <Label /> }`
- **THEN** analysis SHALL reject both declarations because `TItem` is declared twice

#### Scenario: A type parameter in an emitted action payload is rejected
- **WHEN** a file contains `component <List TItem:type items?:TItem+ emits { pick { item:TItem } } /> = { <Label /> }`
- **THEN** analysis SHALL reject the payload field `item` because an emitted action cannot be typed by a type parameter
- **AND** the diagnostic SHALL name `pick`, `item`, and `TItem`

#### Scenario: An inherited type parameter in an emitted action payload is rejected
- **WHEN** a file contains `abstract component <Base TItem:type items?:TItem+ /> component <List extends Base emits { pick { item:TItem } } /> = { <Label /> }`
- **THEN** analysis SHALL reject the payload field `item` on the same terms as a declared parameter

### Requirement: A type parameter is a rigid nominal type within its declaration
Within the signature that declares it and the body of that component, a type parameter name SHALL
denote a nominal type identified by the declaring component and the parameter name. That type
SHALL be satisfied only by itself and by the bottom type, SHALL satisfy `object`, and SHALL
compose with the occurrence suffixes like any other exactly-one type: `?`, `+` and `*` in a
non-property position, `+` in a property's type slot, and the `?` mark on a property's name, as
`occurrence-types` and `optional-properties` define. Two type parameters, whether of one
component or of two, SHALL be different types. Within the declaration the parameter name SHALL
shadow any other type of the same name. Outside the declaration the name SHALL NOT denote a type.

#### Scenario: A concrete value is not a type parameter
- **WHEN** a file contains `component <Bad TItem:type /> = { state { x:TItem = "text" } <Label /> }`
- **THEN** analysis SHALL reject the default as a type mismatch between `string` and `TItem`

#### Scenario: Two type parameters are distinct types
- **WHEN** a file contains `component <Pair TKey:type TValue:type key:TKey value:TValue /> = { state { v:TValue = {key} } <Label /> }`
- **THEN** analysis SHALL reject the default as a type mismatch between `TKey` and `TValue`

#### Scenario: A type parameter satisfies object and accepts the empty list
- **WHEN** a file contains `component <Ok TItem:type item:TItem items?:TItem+ /> = { state { o:object = {item} } <Label /> }` and `let v = <Ok TItem=int item=1 items={} />`
- **THEN** analysis SHALL accept the default `{}` at `TItem*`, the default `{item}` at `object`, and the binding `items={}`

#### Scenario: A type parameter shadows a same-named type inside the component
- **WHEN** a file contains `type TItem = { id:int } component <List TItem:type items:TItem+ /> = { <Label /> }` and `let v = <List TItem=string items={"a"} />`
- **THEN** analysis SHALL resolve `TItem` in `items:TItem+` to the type parameter
- **AND** SHALL accept `items={"a"}` because the argument `string` is what `TItem+` expects, lifted to a sequence of one

#### Scenario: A type parameter is not visible outside its component
- **WHEN** a file contains `type TItem = { id:int } component <List TItem:type /> = { <Label /> } let x:TItem = <TItem id=1 />`
- **THEN** analysis SHALL accept `x`, resolving the top-level `TItem` to the record rather than to the component's parameter

### Requirement: A use site supplies a type argument as a bare type name
An element that targets a component with type parameters MAY bind each type parameter by name to
an unbraced single identifier. The system SHALL resolve that identifier against the types visible
at the use site — primitive type names, and every record, union, alias, and type parameter in
scope there — and SHALL NOT consult value bindings. The resolved type MUST be an exactly-one type:
an identifier that resolves, directly or through an alias chain, to a type carrying an occurrence
SHALL be rejected with a diagnostic explaining that a type argument must be exactly one value and
naming the alias, as `occurrence-types` requires. A type parameter binding MUST be a plain,
unconditional property entry: a braced expression, a quoted string, a numeric literal, or an entry
inside an `if`, condition-list, or `match` property fragment at a type-parameter site SHALL be
rejected. The effective prop types of the component SHALL then be checked with every occurrence of
the parameter replaced by the argument.

#### Scenario: A type argument fixes the element type of a list prop
- **WHEN** a file contains `type Contact = { name:string } external component <SkiaLayout TItem:type itemsSource?:TItem+ />` and `let contacts:Contact* = {} let v = <SkiaLayout TItem=Contact itemsSource={contacts} />`
- **THEN** analysis SHALL accept the element, checking `itemsSource` against `Contact*`

#### Scenario: A mismatch is reported against the substituted type
- **WHEN** a file contains `type Contact = { name:string } external component <SkiaLayout TItem:type itemsSource?:TItem+ />` and `let v = <SkiaLayout TItem=Contact itemsSource={ "a" "b" } />`
- **THEN** analysis SHALL reject `itemsSource` as a mismatch between `string+` and `Contact*`

#### Scenario: A primitive, an alias, and a union are all acceptable arguments
- **WHEN** a file contains `type Fit = fill | cover type Label = string external component <List TItem:type items?:TItem+ />` and `let a = <List TItem=int items={ 1 2 } /> let b = <List TItem=Label items={ "x" } /> let c = <List TItem=Fit items=fill /> let d = <List TItem=object items={ 1 "two" } />`
- **THEN** analysis SHALL accept all four elements, `object` being a primitive type name like `int`

#### Scenario: An alias to a sequence type is not an acceptable argument
- **WHEN** a file contains `type Names = string+ type Maybe = string? external component <List TItem:type items?:TItem+ />` and `let v = <List TItem=Names items={ "x" } /> let w = <List TItem=Maybe items={ "x" } />`
- **THEN** analysis SHALL reject the bindings `TItem=Names` and `TItem=Maybe`
- **AND** each diagnostic SHALL explain that a type argument must be exactly one value and SHALL name `Names` or `Maybe`

#### Scenario: An enclosing component forwards its own type parameter
- **WHEN** a file contains `external component <SkiaLayout TItem:type itemsSource?:TItem+ /> component <Section TItem:type items:TItem+ /> = { <SkiaLayout TItem=TItem itemsSource={items} /> }`
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

### Requirement: An unspecified type parameter is the bottom type
When a use site binds no argument for a type parameter, the system SHALL check that element as if
the parameter were the bottom type. A use site that binds nothing typed by the parameter SHALL
produce no diagnostic. When a binding fails against a prop type that an unspecified parameter
fixed, the diagnostic SHALL name the parameter, SHALL say it was not specified, and SHALL show the
`Name=` form, rather than reporting a mismatch against `never` or a type such as `never*`.

#### Scenario: A use site that never touches the parameter needs no argument
- **WHEN** a file contains `external component <SkiaLayout TItem:type itemsSource?:TItem+ content children?:object+ />` and `let v = <SkiaLayout><Label /></SkiaLayout>`
- **THEN** analysis SHALL accept the element with no diagnostic

#### Scenario: An empty list is accepted without an argument
- **WHEN** a file contains `external component <SkiaLayout TItem:type itemsSource?:TItem+ />` and `let v = <SkiaLayout itemsSource={} />`
- **THEN** analysis SHALL accept the element, the empty value satisfying every type that admits zero

#### Scenario: A non-empty binding without an argument names the parameter
- **WHEN** a file contains `type Contact = { name:string } external component <SkiaLayout TItem:type itemsSource?:TItem+ />` and `let contacts:Contact* = {} let v = <SkiaLayout itemsSource={contacts} />`
- **THEN** analysis SHALL reject `itemsSource`
- **AND** the diagnostic SHALL say that `TItem` was not specified and SHALL show `TItem=`
- **AND** the diagnostic SHALL NOT describe the expected type as `never*`

#### Scenario: Each parameter falls back independently
- **WHEN** a file contains `external component <Grid TRow:type TCol:type rows?:TRow+ cols?:TCol+ />` and `let v = <Grid TRow=int rows={ 1 2 } />`
- **THEN** analysis SHALL accept the element, with `TCol` unspecified and `cols` unbound

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

#### Scenario: The property union of a generic stateful component is derived from state only
- **WHEN** a file contains `component <List TItem:type items:TItem+ /> = { state { selected:int = 0 } <Label /> }` and `let k:List.Property = {List.Property.selected}`
- **THEN** type checking SHALL accept `List.Property` with the single case `selected`
- **AND** SHALL reject `List.Property.TItem`

#### Scenario: Elements with different arguments have the same type
- **WHEN** a file contains `type Contact = { name:string } external component <List TItem:type items?:TItem+ />` and `let a:List = <List TItem=Contact /> let b:List = <List TItem=int /> let c:List = <List />`
- **THEN** analysis SHALL accept all three bindings at the type `List`

### Requirement: A type argument is substituted through a function-typed prop
When a use site binds a type argument, the substitution of the argument for the parameter SHALL
reach every occurrence of the parameter inside a function-typed prop — its parameter types and its
result type — so a function bound to that prop is checked against the substituted function type.
When the use site binds no argument, the parameter SHALL be substituted by position: the bottom
type where the prop produces a value of it (a sequence of items, as elsewhere) and the top type
where the prop consumes one (a function type's parameter, which the host calls with items of a type
the site never named), so that no template assuming a particular item type satisfies the prop; a
diagnostic against such a prop SHALL name the parameter and show the `Name=` form.

#### Scenario: A template is checked at the substituted item type
- **WHEN** a file contains `abstract external component <DrawnNode /> type Contact = { name:string } type Post = { title:string } external component <SkiaLayout TItem:type ItemsSource?:TItem+ ItemTemplate?:<function Item:TItem Index:int />: DrawnNode /> let <ContactRow Item:Contact Index:int />: DrawnNode = <DrawnNode /> let <PostRow Item:Post Index:int />: DrawnNode = <DrawnNode /> let contacts:Contact* = {}` and `let ok = <SkiaLayout TItem=Contact ItemsSource={contacts} ItemTemplate={ContactRow} /> let bad = <SkiaLayout TItem=Contact ItemsSource={contacts} ItemTemplate={PostRow} />`
- **THEN** analysis SHALL accept `ok`
- **AND** SHALL reject `bad` as a mismatch between `<function Item:Post Index:int />: DrawnNode` and
  `(<function Item:Contact Index:int />: DrawnNode)?`

#### Scenario: A template bound without a type argument names the parameter
- **WHEN** a file contains `abstract external component <DrawnNode /> type Contact = { name:string } external component <SkiaLayout TItem:type ItemTemplate?:<function Item:TItem />: DrawnNode /> let <ContactRow Item:Contact />: DrawnNode = <DrawnNode /> let v = <SkiaLayout ItemTemplate={ContactRow} />`
- **THEN** analysis SHALL reject `ItemTemplate`
- **AND** the diagnostic SHALL say that `TItem` was not specified and SHALL show `TItem=`

#### Scenario: A forwarded parameter reaches a function-typed prop
- **WHEN** a file contains `abstract external component <DrawnNode /> external component <SkiaLayout TItem:type ItemsSource?:TItem+ ItemTemplate?:<function Item:TItem Index:int />: DrawnNode /> component <Section extends DrawnNode TItem:type Items:TItem+ Row:<function Item:TItem Index:int />: DrawnNode /> = { <SkiaLayout TItem=TItem ItemsSource={Items} ItemTemplate={Row} /> }`
- **THEN** analysis SHALL accept the body, checking `Row` against the prop with `Section`'s own
  parameter substituted
