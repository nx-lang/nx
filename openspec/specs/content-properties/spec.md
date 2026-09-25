# content-properties Specification

## Purpose
Define explicit content-property semantics for NX declarations and markup body binding.

## Requirements

### Requirement: Function-like and record-like declarations can declare a content property
The system SHALL allow at most one property definition in any function-like or record-like
declaration surface to be prefixed with contextual keyword `content`. Parsing and lowering SHALL
preserve which property is designated as the declaration's content property.

#### Scenario: Record type marks one content property
- **WHEN** a file contains `type Foo = { prop1:int content label:string }`
- **THEN** parsing and lowering SHALL preserve `label` as the content property of `Foo`

#### Scenario: Action record marks one content property
- **WHEN** a file contains `action Submit = { requestId:string content label:string }`
- **THEN** parsing and lowering SHALL preserve `label` as the content property of `Submit`

#### Scenario: Element-style function marks one content property
- **WHEN** a file contains `let <Bar prop1:int content childItems:Baz+ /> = <div />`
- **THEN** parsing and lowering SHALL preserve `childItems` as the content property of `Bar`

#### Scenario: Paren-style function marks one content property
- **WHEN** a file contains `let Wrap(title:string, content body:Element) = <section>{body}</section>`
- **THEN** parsing and lowering SHALL preserve `body` as the content property of `Wrap`

#### Scenario: Component props can mark one content property
- **WHEN** a file contains `component <Panel title:string content body:Element /> = { <section>{body}</section> }`
- **THEN** parsing and lowering SHALL preserve `body` as the content property of `Panel`

#### Scenario: Component emitted action payload can mark one content property
- **WHEN** a file contains `component <Panel emits { Submitted { content payload:string } } /> = { <button /> }`
- **THEN** parsing and lowering SHALL preserve `payload` as the content property of inline emitted
  action `Submitted`

#### Scenario: Component state can mark one content property
- **WHEN** a file contains `component <Panel /> = { state { content body:Element } <section>{body}</section> }`
- **THEN** parsing and lowering SHALL preserve `body` as the content property of the component
  state declaration

#### Scenario: A content property takes the optional mark
- **WHEN** a file contains `let <Bar content childItems?:Baz+ /> = <div />`
- **THEN** parsing and lowering SHALL preserve `childItems` as the content property of `Bar` and as
  optional

#### Scenario: Declaration cannot mark two content properties
- **WHEN** a file contains `type Foo = { content title:string content body:string }`
- **THEN** analysis SHALL reject the declaration because a single declaration cannot expose more
  than one content property

### Requirement: Record inheritance preserves at most one effective content property
The effective shape of a plain record SHALL expose at most one content property after inheritance is
resolved.

#### Scenario: Derived record inherits a base content property
- **WHEN** a file contains `abstract type Base = { content body:string }` and
  `type Card extends Base = { title:string }`
- **THEN** the effective record shape for `Card` SHALL preserve `body` as its content property

#### Scenario: Derived record cannot add a second content property
- **WHEN** a file contains `abstract type Base = { content body:string }` and
  `type Card extends Base = { content footer:string }`
- **THEN** analysis SHALL reject `Card` because its effective record shape would expose more than
  one content property

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
- **AND** the interpreter, the TypeScript IR runtime and every code generation target SHALL bind
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

### Requirement: Body content requires an explicit content property on NX-defined targets
The system SHALL reject markup body content when a markup-style invocation resolves to an
NX-defined plain record, function, or component that has no declared content property. Body content
MUST NOT be routed by an implicit implementation convention.

#### Scenario: Unmarked property does not receive body content implicitly
- **WHEN** a file contains `let <Collect items:object+ />: object+ = { items }`
- **AND** a call site contains `<Collect><div /></Collect>`
- **THEN** analysis SHALL reject the invocation because `Collect` has no declared content property

#### Scenario: Component without a content prop rejects body content
- **WHEN** a file contains `component <Panel title:string /> = { <section>{title}</section> }`
- **AND** a call site contains `<Panel title="Docs"><Badge /></Panel>`
- **THEN** analysis SHALL reject the invocation because `Panel` has no declared content property

### Requirement: Named and body content sources are mutually exclusive
The system SHALL reject an invocation that supplies a declaration's content property both as
explicit named property input and as element body content.

#### Scenario: Content property passed twice is rejected
- **WHEN** a file contains `type Foo = { prop1:int content label:string }`
- **AND** a call site contains `<Foo prop1=32 label="named">body text</Foo>`
- **THEN** analysis SHALL reject the invocation because `label` is supplied both by named property
  and by body content

### Requirement: Property-list fragments participate in content-property binding rules
Conditional property-list fragments SHALL participate in the same content-property validation rules
as direct explicit properties. A content property supplied by a property-list fragment MUST be
treated as an explicit named property on every reachable path where that fragment is active.

#### Scenario: Content property supplied by every branch satisfies required content
- **WHEN** a component declares `content body:Element`
- **AND** a call site contains `<Panel if compact { body=<span /> } else { body=<section /> } />`
- **THEN** type checking SHALL accept the invocation because every reachable branch supplies
  `body`

#### Scenario: Conditional content property conflicts with body content
- **WHEN** a component declares `content body:Element`
- **AND** a call site contains `<Panel if compact { body=<span /> }><Badge /></Panel>`
- **THEN** type checking SHALL reject the invocation because `body` can be supplied by both a named
  property fragment and element body content on the `compact` path

#### Scenario: Mutually exclusive content property branches are accepted
- **WHEN** a component declares `content body:Element`
- **AND** a call site contains `<Panel if compact { body=<span /> } else { body=<section /> } />`
- **THEN** type checking SHALL NOT report a duplicate content-property diagnostic for the mutually
  exclusive `body` branches

### Requirement: `content` remains a contextual keyword
The token `content` SHALL act as a modifier only when it appears immediately before a property name
inside a content-capable declaration. In other positions it SHALL remain a normal identifier.

#### Scenario: Property can still be named content
- **WHEN** a file contains `type Note = { content:string }`
- **THEN** parsing SHALL treat `content` as the property name rather than as a reserved keyword

#### Scenario: Other declarations can still use content as an identifier
- **WHEN** a file contains `let render(content:string) = content`
- **THEN** parsing and lowering SHALL treat both uses of `content` as ordinary identifiers

### Requirement: Element body semantics use content terminology consistently
The implementation SHALL model element body semantics as `content` rather than `children` across
binding, runtime, diagnostics, and user-facing documentation.

#### Scenario: Diagnostic refers to content rather than children
- **WHEN** a call site provides both a named content property and element body content
- **THEN** the reported diagnostic SHALL describe the conflict in terms of `content`

#### Scenario: Intrinsic element runtime preserves generic content
- **WHEN** a file contains `<div><span /></div>`
- **THEN** the runtime representation SHALL preserve the nested body content under a generic
  `content` channel rather than a `children` channel

### Requirement: An element body may be empty
An element written with an opening and a closing tag SHALL parse when nothing appears between them.
An empty body SHALL mean the same as a self-closing tag: no body content is supplied, and a declared
content property is not written, so it binds as `optional-properties` defines for a property that is
not written — its default when it has one, the empty value when it is optional, and a
missing-required-field diagnostic otherwise.

Whitespace and comments SHALL NOT constitute body content. An empty body SHALL NOT count as
supplying the content property, so a target that declares no content property SHALL accept it.

A body that holds at least one item SHALL count as written even when that item may evaluate to the
empty value. Such a body is typed by the collecting rule of `occurrence-types` — `{if c { <A/> }}`
is `A?` — and is checked against the content property's read type, so it SHALL be a type error at
`content items:A+` and SHALL be accepted at `content items?:A+`, where it binds the empty value when
no item is produced. A written body that evaluates to the empty value SHALL NOT take the property's
default.

#### Scenario: An element with an empty body parses
- **WHEN** a file contains `abstract external component <Node /> external component <App extends Node content Children?:Node+ /> <App></App>`
- **THEN** parsing SHALL accept the file
- **AND** it SHALL NOT report a syntax error

#### Scenario: An empty body leaves the content property unset
- **WHEN** two files differ only in that one writes `<App></App>` where the other writes `<App />`
- **THEN** evaluation SHALL produce the same value for both
- **AND** the declared content property SHALL be the empty value in both, because `Children` is
  optional and not written

#### Scenario: An empty body takes the content property's default
- **WHEN** a file contains `type Box = { content items:A+ = { <A/> } }` and `<Box></Box>`
- **THEN** evaluation SHALL bind `items` to the default

#### Scenario: A body holding only whitespace and comments is empty
- **WHEN** a file contains an element whose body spans several lines and holds only a comment
- **THEN** parsing SHALL accept the element
- **AND** its declared content property SHALL be treated as not written

#### Scenario: A written body that may be empty is rejected where one is required
- **WHEN** a file contains `type Box = { content items:A+ = { <A/> } }` and `<Box>{if c { <A/> }}</Box>`
- **THEN** type checking SHALL reject the body, naming `A?` against `A+`
- **AND** SHALL NOT apply the default

#### Scenario: A written body that may be empty is accepted at an optional content property
- **WHEN** a file contains `type Box = { content items?:A+ }` and `<Box>{if c { <A/> }}</Box>`
- **THEN** type checking SHALL accept the body
- **AND** evaluation with `c` false SHALL bind `items` to the empty value

#### Scenario: An empty body is accepted by a target that declares no content property
- **WHEN** a file contains `abstract external component <Node /> external component <Plain extends Node title?:string /> <Plain title="hi"></Plain>`
- **THEN** analysis SHALL accept the element
- **AND** a populated body on the same target SHALL still be rejected because `Plain` declares no
  content property

#### Scenario: An empty body is accepted wherever an element is written
- **WHEN** a file writes an element with an empty body inside `let root()` rather than as the file's
  trailing element
- **THEN** parsing SHALL accept it there as well

#### Scenario: A closing tag is still matched against its opening tag
- **WHEN** a file contains an element with an empty body whose closing tag names a different element
- **THEN** validation SHALL report the mismatch between the closing and opening tag names
