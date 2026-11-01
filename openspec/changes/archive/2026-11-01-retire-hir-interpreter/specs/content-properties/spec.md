## MODIFIED Requirements

### Requirement: Markup body content binds to the declared content property
The system SHALL bind markup invocation body content to the declared content property when a
markup-style invocation resolves to an NX-defined plain record, function, or component with a
declared content property. Body content SHALL be a collecting position as `occurrence-types`
defines: its items contribute their types and occurrences, and the collected value SHALL be checked
against the content property's read type. A lone child at a `+` or `*` content property SHALL bind
as the child's value lifted once to the declared type, the same one-level lift the language applies
at every sequence site; every engine SHALL bind a lone child the same way.

#### Scenario: Text body binds to a scalar content property
- **WHEN** a file contains `type Foo = { prop1:int content label:string }` and
  `let root(): Foo = { <Foo prop1=32>label text</Foo> }`
- **THEN** constructing `Foo` SHALL bind `label` to `"label text"`

#### Scenario: Element body binds to an array content property
- **WHEN** a file contains `let <Bar prop1:int content childItems:Baz+ />: Baz+ = { childItems }`
- **AND** a call site contains `<Bar prop1=32><Baz/> <Baz/></Bar>`
- **THEN** the invocation SHALL bind both `Baz` body items to `childItems` in source order

#### Scenario: A lone child binds a one-item sequence
- **WHEN** a file contains `let <Bar content childItems:Baz+ />: Baz+ = { childItems }`
- **AND** a call site contains `<Bar><Baz/></Bar>`
- **THEN** the invocation SHALL bind `childItems` to a one-item sequence holding the `Baz`
- **AND** the Rust IR runtime, the TypeScript IR runtime and every code generation target SHALL bind
  the same value

#### Scenario: Paren-style function receives body content through markup invocation
- **WHEN** a file contains `let Wrap(title:string, content body:Element) = <section>{body}</section>`
- **AND** a call site contains `<Wrap title="Docs"><Badge /></Wrap>`
- **THEN** the invocation SHALL bind `<Badge />` to `body`

#### Scenario: Component invocation binds body content to the declared content prop
- **WHEN** a file contains `component <Panel title:string content body:Element /> = { <section>{body}</section> }`
- **AND** a call site contains `<Panel title="Docs"><Badge /></Panel>`
- **THEN** the invocation SHALL bind `<Badge />` to `body`

#### Scenario: Content property can still be passed explicitly
- **WHEN** a file contains `type Foo = { prop1:int content label:string }` and
  `let root(): Foo = { <Foo prop1=32 label="label text" /> }`
- **THEN** constructing `Foo` SHALL accept the explicit `label` property without requiring element
  body content
