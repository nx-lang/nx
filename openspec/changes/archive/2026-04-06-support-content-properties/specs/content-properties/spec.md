## ADDED Requirements

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
- **WHEN** a file contains `let <Bar prop1:int content childItems:Baz[] /> = <div />`
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
When a markup-style invocation resolves to an NX-defined plain record, function, or component with
a declared content property, the invocation body SHALL bind to that property using the same
scalar-or-sequence normalization applied to other element body content.

#### Scenario: Text body binds to a scalar content property
- **WHEN** a file contains `type Foo = { prop1:int content label:string }` and
  `let root(): Foo = { <Foo prop1=32>label text</Foo> }`
- **THEN** constructing `Foo` SHALL bind `label` to `"label text"`

#### Scenario: Element body binds to an array content property
- **WHEN** a file contains `let <Bar prop1:int content childItems:Baz[] />: Baz[] = { childItems }`
- **AND** a call site contains `<Bar prop1=32><Baz/> <Baz/></Bar>`
- **THEN** the invocation SHALL bind both `Baz` body items to `childItems` in source order

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
When a markup-style invocation resolves to an NX-defined plain record, function, or component that
has no declared content property, markup body content SHALL be rejected rather than being routed by
an implicit implementation convention.

#### Scenario: Unmarked property does not receive body content implicitly
- **WHEN** a file contains `let <Collect items:object[] />: object[] = { items }`
- **AND** a call site contains `<Collect><div /></Collect>`
- **THEN** analysis SHALL reject the invocation because `Collect` has no declared content property

#### Scenario: Component without a content prop rejects body content
- **WHEN** a file contains `component <Panel title:string /> = { <section>{title}</section> }`
- **AND** a call site contains `<Panel title="Docs"><Badge /></Panel>`
- **THEN** analysis SHALL reject the invocation because `Panel` has no declared content property

### Requirement: Named and body content sources are mutually exclusive
If an invocation supplies a declaration's content property both as explicit named property input and
as element body content, the invocation SHALL be rejected.

#### Scenario: Content property passed twice is rejected
- **WHEN** a file contains `type Foo = { prop1:int content label:string }`
- **AND** a call site contains `<Foo prop1=32 label="named">body text</Foo>`
- **THEN** analysis SHALL reject the invocation because `label` is supplied both by named property
  and by body content

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
