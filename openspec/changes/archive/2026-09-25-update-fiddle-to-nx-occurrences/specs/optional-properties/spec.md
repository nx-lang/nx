## MODIFIED Requirements

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

#### Scenario: A property rejected in the type slot is not reported again where it is left out
- **WHEN** a file declares `component <Box width:float64? />`, `let <Row gap:float64? />`, `type Book = { title:string tags:string* }` or `let f(a:int, b:int?)`, and writes `<Box />`, `<Row />`, `<Book title="a" />` or `f(1)` twice
- **THEN** type checking SHALL report the declaration once, with its `name?:` fix-it
- **AND** SHALL NOT also report the property as required at either use
- **AND** SHALL still check a value written for it against the type it names

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
