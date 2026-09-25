## Purpose

Defines the `name?:type` property form: the one way a property, field or parameter admits no value,
what may follow its colon, why it has no default, what reading it yields, and how construction
treats a property that was omitted, written, or written empty.

## ADDED Requirements

### Requirement: A property admits zero only through a mark on its name
Every property definition — a record or action field, a component prop, a `state` field, an
`emits` payload field, an element-function parameter, a parenthesized function parameter, and a
function-type parameter — SHALL have the form `[content] name [?] : type [= default]`. The type SHALL
be an exactly-one type or a `+` type. A `?` or `*` occurrence in the type slot SHALL be rejected,
whether spelled there or reached through an alias, with a diagnostic that shows the `name?:` form
with the same base type. A type parameter definition (`T:type`) SHALL NOT take the `?` mark.

#### Scenario: The four property shapes are accepted
- **WHEN** a file contains `type Book = { title:string subtitle?:string authors:Person+ tags?:string+ }`
- **THEN** parsing, lowering and type checking SHALL accept the declaration

#### Scenario: An occurrence that admits zero is rejected in the type slot
- **WHEN** a file contains `type Book = { subtitle:string? tags:string* }`
- **THEN** the system SHALL reject both fields
- **AND** the diagnostic for `subtitle` SHALL show `subtitle?:string`
- **AND** the diagnostic for `tags` SHALL show `tags?:string+`

#### Scenario: An alias that carries zero is rejected in the type slot
- **WHEN** a file contains `type Maybe = string?` and `type Book = { subtitle:Maybe }`
- **THEN** type checking SHALL reject `subtitle`, naming `Maybe` as carrying an occurrence that admits zero
- **AND** SHALL continue to accept `type Names = string+` used as `authors:Names`

#### Scenario: A type parameter is never optional
- **WHEN** a file contains `type Box = { T?:type value:T }`
- **THEN** the system SHALL reject the definition on the same terms it rejects a suffix on `type`

#### Scenario: Every declaration site takes the mark
- **WHEN** a file contains `component <Card title:string subtitle?:string /> = { state { note?:string } <Label text={title} /> }`, `let <Row gap:float64 = 0.0 content child?:Element />`, `let f(a:int, b?:int) = { a }` and `type Render = <function Item:Person Index?:int />: string`
- **THEN** parsing and lowering SHALL accept every `?` mark
- **AND** `subtitle`, `note`, `child`, `b` and `Index` SHALL each be optional

### Requirement: An optional property has no default
A property that carries the `?` mark SHALL NOT have a default. The system SHALL reject `name?:T = x`,
explaining that a default already makes the property omissible and gives it a value, so the two
forms are exclusive.

#### Scenario: A default on an optional property is rejected
- **WHEN** a file contains `type Book = { subtitle?:string = "none" }`
- **THEN** the system SHALL reject the field
- **AND** the diagnostic SHALL offer `subtitle:string = "none"` and `subtitle?:string` as the two forms

### Requirement: Reading an optional property yields an occurrence that admits zero
A reference to a property declared `p?:T` SHALL have type `T?`. A reference to one declared `p?:T+`
SHALL have type `T*`. A reference to a required or defaulted property SHALL have its declared type.
This SHALL hold for a field read through a record value, a prop or state field read inside a
component body, and a parameter read inside a function body.

#### Scenario: An optional field reads as optional
- **WHEN** a file contains `type Book = { subtitle?:string tags?:string+ }` and `let f(b:Book) = { b.subtitle }`, `let g(b:Book) = { b.tags }`
- **THEN** `f` SHALL infer a result of `string?`
- **AND** `g` SHALL infer a result of `string*`

#### Scenario: An optional parameter reads as optional inside its function
- **WHEN** a file contains `let greet(name?:string): string = { "Hi " + name }`
- **THEN** type checking SHALL reject the body, naming `string?` at a `string` site

### Requirement: Construction treats an optional property that is not written as empty
At a record construction, component use, function call or element body, a property that is not
written SHALL bind: the default, when the property has one; the empty value, when the property is
optional; otherwise a missing-required-field diagnostic. A property that is written SHALL be
checked against its read type — `T?` for `p?:T`, `T*` for `p?:T+` — so an optional property
accepts `{}`, a `?` value, a `*` value where its type is `+`, and an exactly-one value. A required
or defaulted property SHALL be checked against its declared type, so writing `{}` or a value that
admits zero to it is a type error. Host-supplied construction SHALL apply the same rules.

An element body SHALL count as written when it contains at least one item, so an empty body leaves
a content property not written, as `content-properties` requires; a body that is written and
evaluates to the empty value is the empty value.

#### Scenario: An omitted optional property is empty
- **WHEN** a file contains `type Book = { title:string subtitle?:string }` and `let b = <Book title="A" />`
- **THEN** type checking SHALL accept the construction
- **AND** `b.subtitle` SHALL evaluate to the empty value

#### Scenario: A defaulted property that is not written takes its default
- **WHEN** a file contains `type Book = { title:string = "Untitled" }` and `let b = <Book />`
- **THEN** `b.title` SHALL evaluate to `"Untitled"`

#### Scenario: A required property that is not written is still reported
- **WHEN** a file contains `type Book = { title:string }` and `let b = <Book />`
- **THEN** type checking SHALL reject the construction because `title` is missing

#### Scenario: An optional property accepts what admits zero
- **WHEN** a file contains `type Book = { subtitle?:string tags?:string+ }`, `let o:string? = {}`, `let ts:string* = {}` and `let b = <Book subtitle={o} tags={ts} />`, `let c = <Book subtitle={} tags={} />`
- **THEN** type checking SHALL accept both constructions
- **AND** `b` and `c` SHALL be equal values

#### Scenario: A defaulted property rejects the empty value
- **WHEN** a file contains `type Book = { title:string = "Untitled" }` and `let b = <Book title={} />`
- **THEN** type checking SHALL reject `title`, naming `{}` and `string`
- **AND** SHALL NOT apply the default

#### Scenario: A one-or-more property rejects a value that may be empty
- **WHEN** a file contains `type Box = { items:string+ }`, `let maybe:string* = {}` and `let b = <Box items={maybe} />`, `let c = <Box items={if c { "a" }} />`
- **THEN** type checking SHALL reject both, naming `string*` and `string?` respectively against `string+`

#### Scenario: An empty body is not written and a written body may be empty
- **WHEN** a file contains `type Box = { content items:A+ = { <A/> } }` and `<Box></Box>`, `<Box>{if c { <A/> }}</Box>`
- **THEN** the first SHALL bind `items` to the default
- **AND** the second SHALL be rejected, because a body typed `A?` does not satisfy `A+`

#### Scenario: A host omits an optional field
- **WHEN** a host constructs `type Book = { title:string subtitle?:string }` from JSON `{ "title": "A" }`
- **THEN** construction SHALL succeed with `subtitle` empty
- **AND** constructing from `{}` SHALL fail naming `title`

### Requirement: Optional state initializes empty
A component `state` field that carries the `?` mark SHALL be initialized to the empty value when it
has no default, and SHALL be readable in the body as `T?`. A required state field with no default
SHALL still fail initialization, as `component-syntax` requires.

#### Scenario: Optional state starts empty
- **WHEN** a file contains `component <Search /> = { state { query?:string } <Label text={query ?? ""} /> }`
- **THEN** type checking SHALL accept the component
- **AND** initialization SHALL bind `query` to the empty value and render the label with `""`
