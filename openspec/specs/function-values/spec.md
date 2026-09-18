# function-values Specification

## Purpose

Define a function name as a value: its type, where it may be bound, how a function-typed value is
invoked, and the canonical record a runtime renders for one, so that a template can be passed to a
host or an authored component and called there.

## Requirements

### Requirement: A visible function name is a value of its function type
In expression position, an identifier that names a visible function — an element function or a
paren function, declared in the same file, a same-library peer, or an imported library — SHALL
evaluate to a value of that function's type: its parameters, in declared order, with their names,
types and `content` marking, and its declared or inferred result type. A lexical binding of the
same name SHALL shadow the function, as it shadows any top-level name. A function value SHALL
capture nothing: it refers to the declaration alone, so two function values SHALL be equal exactly
when both name the same declaration, in every runtime and under every comparison the language
shares — `==`, match patterns and `diff`.

#### Scenario: A function is bound to a function-typed property
- **WHEN** a file contains `abstract external component <DrawnNode /> type Contact = { name:string } external component <SkiaLayout TItem:type ItemsSource:TItem[]? ItemTemplate:(<function Item:TItem Index:int />: DrawnNode)? /> let <ContactRow Item:Contact Index:int />: DrawnNode = <DrawnNode /> let contacts:Contact[] = {} let v = <SkiaLayout TItem=Contact ItemsSource={contacts} ItemTemplate={ContactRow} />`
- **THEN** analysis SHALL accept `v`
- **AND** evaluation SHALL produce a `SkiaLayout` record whose `ItemTemplate` field is the function
  value for `ContactRow`

#### Scenario: A function is forwarded through an authored component
- **WHEN** a file contains `abstract external component <DrawnNode /> type Contact = { name:string } type RowTemplate = <function Item:Contact Index:int />: DrawnNode external component <SkiaLayout TItem:type ItemsSource:TItem[]? ItemTemplate:(<function Item:TItem Index:int />: DrawnNode)? /> component <Section extends DrawnNode Items:Contact[] Row:RowTemplate /> = { <SkiaLayout TItem=Contact ItemsSource={Items} ItemTemplate={Row} /> } let <ContactRow Item:Contact Index:int />: DrawnNode = <DrawnNode /> let s = <Section Items={} Row={ContactRow} />`
- **THEN** analysis SHALL accept both the component body and `s`

#### Scenario: A paren function is a value too
- **WHEN** a file contains `let double(n:int): int = {n * 2} let f: <function n:int />: int = {double}`
- **THEN** analysis SHALL accept `f`

#### Scenario: Function values compare by the declaration they name
- **WHEN** a file contains `let <A Item:object />: string = "a" let <B Item:object />: string = "b" let f: <function Item:object />: string = {A} let g: <function Item:object />: string = {A} let h: <function Item:object />: string = {B} let same() = {f == g} let other() = {f == h}`
- **THEN** evaluating `same` SHALL produce `true` and `other` SHALL produce `false` in every runtime

#### Scenario: A function bound where a non-function is expected is a mismatch
- **WHEN** a file contains `let <Row Item:object />: string = "x" let s:string = {Row}`
- **THEN** analysis SHALL reject `s` as a mismatch between `<function Item:object />: string` and
  `string`

#### Scenario: An undefined name is still undefined
- **WHEN** a file contains `let v = {NoSuchFunction}`
- **THEN** analysis SHALL report `NoSuchFunction` as undefined

#### Scenario: A lexical binding shadows a function of the same name
- **WHEN** a file contains `let <Row Item:object />: string = "x" let pick(Row:string): string = {Row}`
- **THEN** analysis SHALL resolve `Row` in the body of `pick` to the parameter

### Requirement: A function-typed value is invocable as an element
A binding whose type is a function type — a parameter, a component prop, or a `let`, local or
top-level — SHALL be invocable as an element, `<Name Prop={v} ... />`: arguments SHALL bind to the
type's parameters by name and body content to its content parameter, and a tag whose name is such
a binding SHALL be that call rather than a lookup of a declared element. The checker SHALL require
every parameter of the type, SHALL reject an argument the type does not declare, and SHALL give
the call the type's result type. At run time the arguments SHALL reach the function by name: a
parameter the function does not declare SHALL be dropped, and a parameter it declares SHALL always
be present, because the type supplied it. A paren-style call on a function-typed binding,
`Name(a, b)`, SHALL be rejected with a diagnostic that shows the element form, because arguments
by position would depend on the order of parameters the value's own declaration does not share.

#### Scenario: A function-typed prop is invoked as an element
- **WHEN** a file contains `abstract external component <DrawnNode /> type Contact = { name:string } component <Section extends DrawnNode Item:Contact Row:<function Item:Contact Index:int />: DrawnNode /> = { <Row Item={Item} Index=0 /> } let <ContactRow Item:Contact Index:int />: DrawnNode = <DrawnNode /> let s = <Section Item=<Contact name="a" /> Row={ContactRow} />`
- **THEN** analysis SHALL accept the body of `Section`, resolving the tag `Row` to the prop
- **AND** evaluating `s` SHALL render what `ContactRow` renders for that item

#### Scenario: A function-typed parameter of a paren function is invoked as an element
- **WHEN** a file contains `let invoke(f: <function n:int />: int, n:int): int = <f n={n} /> let double(n:int): int = {n * 2} let v = {invoke(double, 4)}`
- **THEN** analysis SHALL accept `v` at type `int`
- **AND** evaluation SHALL produce `8`

#### Scenario: A top-level `let` of function type is invoked as an element
- **WHEN** a file contains `external component <Box Label:string? /> let <Wrap Item:object />: string = "w" let F: <function Item:object />: string = {Wrap} let root() = <Box Label=<F Item="x" /> />`
- **THEN** analysis SHALL accept the call, resolving the tag `F` to the declaration
- **AND** evaluating `root` SHALL produce `<Box Label="w" />` in every runtime

#### Scenario: A paren-style call on a function-typed value is rejected
- **WHEN** a file contains `let invoke(f: <function n:int />: int, n:int): int = {f(n)}`
- **THEN** analysis SHALL reject the call `f(n)`
- **AND** the diagnostic SHALL show `<f n=... />` as the form to use

#### Scenario: A missing argument is reported against the type
- **WHEN** a file contains `component <Section Row:<function Item:object Index:int />: string /> = { <Row Item="a" /> }`
- **THEN** analysis SHALL reject the call
- **AND** the diagnostic SHALL say `Row` requires `Index`

#### Scenario: An argument the type does not declare is reported
- **WHEN** a file contains `component <Section Row:<function Item:object />: string /> = { <Row Item="a" Extra=1 /> }`
- **THEN** analysis SHALL reject `Extra` as a property `Row` does not have

#### Scenario: The function ignores a parameter the type supplied
- **WHEN** a file contains `type Contact = { name:string } component <Section Item:Contact Row:<function Item:Contact Index:int />: string /> = { <Row Item={Item} Index=3 /> } let <Compact Item:Contact />: string = {Item.name} let s = <Section Item=<Contact name="a" /> Row={Compact} />`
- **THEN** evaluating `s` SHALL produce `"a"`, with `Index` dropped

#### Scenario: A non-function binding is not a tag
- **WHEN** a file contains `external component <Label /> component <Section Label:string /> = { <Label /> }`
- **THEN** analysis SHALL resolve the tag `Label` to the declared component, not to the `string` prop

### Requirement: A function value renders as a Function record
Where a runtime renders a function value into canonical output — a rendered element or component
record, a function result, a serialized value — it SHALL render a record with `$type`
`"Function"`, `module` set to the identity of the module that declares the function, and `name` set
to the function's declared name. Both the Rust and TypeScript runtimes SHALL render the same
record for the same program. A host that supplies a `Function` record where a function-typed
value is expected SHALL have it resolved to the declaration it names, and one that names no
function declaration in the linked program SHALL be refused with a diagnostic.

#### Scenario: A function-typed field renders as a Function record
- **WHEN** the module `app/main.nx` declares `let <ContactRow Item:object Index:int />: string = "r"` and `external component <List ItemTemplate:(<function Item:object Index:int />: string)? /> let root() = <List ItemTemplate={ContactRow} />`
- **AND** `root` is evaluated by either runtime
- **THEN** the `ItemTemplate` field of the result SHALL be `{ "$type": "Function", "module": "app/main.nx", "name": "ContactRow" }`

#### Scenario: A Function record that names nothing is refused
- **WHEN** a host supplies `{ "$type": "Function", "module": "app/main.nx", "name": "Missing" }`
  where a function-typed value is expected
- **THEN** the runtime SHALL refuse it with a diagnostic naming `Missing`
