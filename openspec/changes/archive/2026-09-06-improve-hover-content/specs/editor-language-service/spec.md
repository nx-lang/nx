## MODIFIED Requirements

### Requirement: Language service exposes hover information
The language service SHALL expose hover information at positions where NX can identify a declaration,
reference, type annotation, component tag, property, or expression with known metadata. Hover
results SHALL be conservative: if the language service cannot determine useful information, it
SHALL return no hover rather than fabricate incomplete semantic data.

Where the resolved position has a type known to NX static analysis, the hover result SHALL report
that type. Where the resolved position is a declaration, the hover result SHALL report the
declaration's signature in addition to its symbol kind. Hover SHALL be available at a reference to a
declaration, not only at the declaration itself.

Where the resolved position names a property of a component visible to the document, the hover
result SHALL report the type that component declares for that property. Where the resolved position
names a type, the hover result SHALL report the declaration that name resolves to, or identify the
type where NX defines it rather than the workspace. Where the position is a place a name may go but
none has been written — an empty property slot, an annotation with no type — the language service
SHALL report no result, because a position that names nothing has nothing to report.

Hover SHALL answer at every position a name is *written*, not only where a name is used. A parameter
of a function or of an element-style function, a field of a record type, a case of a union, and a
field of a union case's payload are each declarations written by the author, and the hover result at
one SHALL report what kind of thing it is and the type it declares.

Where the resolved position is the member of a member-access expression, the hover result SHALL
report that member and the type the access has. The member SHALL be answered on the same terms as
the expression it belongs to; the position SHALL NOT report the type of the receiver.

Where the resolved position is a property value in which a name has been written, the hover result
SHALL report what that name resolves to against the property's declared type, on the same terms as
the same name written at any other position of that type. Only a value slot with nothing written in
it reports no result. A declared default is such a position: a bare name written as the default of
a record field or a component property SHALL report what the same name reports in a value slot.

Where a declaration carries no written type annotation but NX static analysis inferred one for it,
the hover result SHALL report the inferred type. Where the resolved position names a union, the
hover result SHALL report the union's cases; where it names a record type, the hover result SHALL
report the record's fields and their declared types. This is the same obligation already met for a
component's properties.

#### Scenario: Hover over declaration shows declaration information
- **WHEN** a client requests hover on the name of a function declaration
- **THEN** the language service SHALL return hover content identifying the symbol kind and available
  signature or type information

#### Scenario: Hover over a component declaration reports its signature
- **WHEN** a client requests hover on the name of a component declaration that accepts properties
- **THEN** the language service SHALL return hover content identifying it as a component
- **AND** the content SHALL include the component's properties and their declared types

#### Scenario: Hover over a reference reports the referenced declaration
- **WHEN** a client requests hover on a component tag naming a component declared elsewhere in the
  workspace snapshot and visible to the document
- **THEN** the language service SHALL return hover content for the declaration that tag resolves to

#### Scenario: Hover over an expression reports its inferred type
- **WHEN** a client requests hover on an expression whose type NX static analysis inferred, such as
  a parameter referenced in a function body
- **THEN** the language service SHALL return hover content reporting that inferred type

#### Scenario: Hover over a literal reports its type
- **WHEN** a client requests hover on a literal value written in a position NX static analysis typed
- **THEN** the language service SHALL return hover content reporting that literal's type
- **AND** the result SHALL be the same wherever in the literal the position falls

#### Scenario: Hover over a property name reports its declared type
- **WHEN** a client requests hover on a property name written in the opening tag of an element whose
  tag names a component visible to the document
- **THEN** the language service SHALL return hover content identifying the position as a property
- **AND** the content SHALL report the type that component declares for that property

#### Scenario: Hover over a type annotation reports the type it names
- **WHEN** a client requests hover on a type name written in a type annotation
- **THEN** the language service SHALL return hover content for the declaration that name resolves
  to, or identify it as a primitive or built-in type where it names one

#### Scenario: Hover at a slot with no name written returns no result
- **WHEN** a client requests hover at an empty property slot in an opening tag, or at a type
  annotation with no type written in it yet
- **THEN** the language service SHALL return no hover result

#### Scenario: Hover on unknown syntax returns no result
- **WHEN** a client requests hover on a position where no useful NX metadata is available
- **THEN** the language service SHALL return no hover result

#### Scenario: Hover on an expression with no inferred type returns no result
- **WHEN** a client requests hover on an expression in a document whose analysis did not produce a
  type for it, such as an expression inside a declaration with a syntax error
- **THEN** the language service SHALL return no hover result rather than an incomplete one

#### Scenario: Hover answers inside an element whose tag resolves to nothing
- **WHEN** a document contains `let <Row count:int /> = <div>{count + 1}</div>` and `div` names no
  declaration visible to the document
- **AND** a client requests hover on `count` inside that interpolation
- **THEN** the language service SHALL return hover content reporting `int`
- **AND** the result SHALL be the same one reported for `count` written outside the element

#### Scenario: Hover over a member of a member access reports the member
- **WHEN** a document declares a record type with a `name` field of type `string`
- **AND** a client requests hover on `name` in an expression `user.name`
- **THEN** the language service SHALL return hover content identifying `name` as a property of that
  record and reporting `string`

#### Scenario: Hover over a function parameter declaration reports the parameter
- **WHEN** a client requests hover on the name of a parameter in a function declaration, or on the
  name of a parameter in an element-style function declaration
- **THEN** the language service SHALL return hover content identifying the position as a parameter
  and reporting its declared type

#### Scenario: Hover over a record field declaration reports the field
- **WHEN** a client requests hover on the name of a field written in a record type declaration
- **THEN** the language service SHALL return hover content identifying the position as a field of
  that record and reporting its declared type

#### Scenario: Hover over a union case declaration reports the case
- **WHEN** a client requests hover on the name of a case written in a union declaration
- **THEN** the language service SHALL return hover content identifying the position as a case of
  that union
- **AND** where the case declares a payload, hover on a payload field name SHALL report that field
  and its declared type

#### Scenario: Hover over a qualified union case reports the case
- **WHEN** a document declares a union `Role` with a case `admin`
- **AND** a client requests hover on `admin` in the qualified form `{Role.admin}`
- **THEN** the language service SHALL return hover content reporting that union case
- **AND** the result SHALL be the one reported at the case's own declaration, at a bare name
  written where `Role` is expected, and at a bare name written in a property's value slot — one
  concept reported one way at every position that names it
- **AND** an expression that merely has a union case as its type, such as a read of a value
  declared with one, SHALL NOT be reported as a case

#### Scenario: Hover over a bare property value reports what the name resolves to
- **WHEN** a document declares a union `Role` with a case `admin` and a component with a property
  of type `Role`
- **AND** a client requests hover on `admin` in `<Card role=admin />`
- **THEN** the language service SHALL return hover content reporting the union case that name
  resolves to
- **AND** the result SHALL be the one reported for the same bare name written at a declaration of
  type `Role`

#### Scenario: Hover over an unannotated value declaration reports its inferred type
- **WHEN** a client requests hover on the name of a value declaration written without a type
  annotation, whose type NX static analysis inferred
- **THEN** the language service SHALL return hover content reporting that inferred type

#### Scenario: Hover over a union declaration reports its cases
- **WHEN** a client requests hover on the name of a union declaration
- **THEN** the language service SHALL return hover content identifying it as a union
- **AND** the content SHALL list the union's cases

#### Scenario: Hover over a record type declaration reports its fields
- **WHEN** a client requests hover on the name of a record type declaration
- **THEN** the language service SHALL return hover content identifying it as a record
- **AND** the content SHALL list the record's fields and their declared types

## ADDED Requirements

### Requirement: Hover content is markdown written in NX
Hover content SHALL be markdown. Where the content includes a signature, a type, or any other
fragment of NX, that fragment SHALL be emitted in a fenced code block tagged `nx`, so that a client
which highlights NX renders it as code rather than as prose.

<para>Every such fragment SHALL be spelled the way NX spells it. A hover SHALL NOT introduce a
keyword, punctuation, or ordering that does not appear in the language — the fragment shown for a
declaration is the declaration as an author would write it.</para>

<para>A hover SHALL state what kind of thing the position is exactly once. Where the kind is not
evident from the NX fragment itself, it SHALL be written as a parenthesized prefix on the fragment's
own line rather than as a separate sentence repeating the fragment.</para>

#### Scenario: A function declaration's hover shows an NX signature
- **WHEN** a client requests hover on the name of a function declared `let add(count:int): int = ...`
- **THEN** the hover content SHALL contain a fenced code block tagged `nx`
- **AND** the fenced content SHALL spell the declaration as `let add(count:int): int`
- **AND** the hover content SHALL NOT contain the word `function`, which is not an NX keyword

#### Scenario: A type alias declaration's hover shows an NX signature
- **WHEN** a client requests hover on the name of a declaration `type Size = int`
- **THEN** the fenced content SHALL spell it as `type Size = int`

#### Scenario: An element-style function's hover shows an NX signature
- **WHEN** a client requests hover on the name of a declaration
  `let <Panel title:string /> = <div />`
- **THEN** the fenced content SHALL spell it as `let <Panel title:string />`

#### Scenario: A position whose kind is not evident states it once
- **WHEN** a client requests hover on a function parameter `count` of declared type `int`
- **THEN** the fenced content SHALL read `(parameter) count: int`
- **AND** the hover content SHALL state the word `parameter` exactly once
