## ADDED Requirements

### Requirement: Language service resolves editor queries against the analyzed document
The language service SHALL determine the context of a position query from the analyzed document —
its parsed syntax tree and lowered module — rather than from the text preceding the position on its
own line. Resolution SHALL identify the innermost NX construct enclosing the position, including a
declaration, a reference to one, a component tag, a property name, a property value, a type
annotation, and an expression.

Because context comes from analysis rather than from line text, results SHALL NOT depend on how a
construct is distributed across lines: a construct written across several lines SHALL resolve to the
same context as the same construct written on one line. Where a position encloses no construct the
language service can identify, it SHALL report no result for that query rather than fall back to a
context-free answer.

#### Scenario: Multi-line opening tag offers the same property completions as a single-line one
- **WHEN** a client requests completions at a property-name position inside an element opening tag
  for a known component
- **AND** the opening tag is written across several lines, so the tag's `<` is not on the line the
  position is on
- **THEN** the language service SHALL offer that component's undeclared properties
- **AND** the completions SHALL match those offered for the same tag written on a single line

#### Scenario: Multi-line opening tag offers contextual member completions
- **WHEN** a client requests completions immediately after `=` in a property value position whose
  declared type is a discriminated union
- **AND** the opening tag is written across several lines
- **THEN** the language service SHALL offer the constant cases of that union, as bare names

#### Scenario: Supplied properties are recognized anywhere in the opening tag
- **WHEN** a client requests completions at a property-name position in an opening tag written
  across several lines
- **AND** a property has already been supplied on a different line of that same opening tag
- **THEN** the language service SHALL NOT offer that already-supplied property

#### Scenario: Type completions are offered in a multi-line signature
- **WHEN** a client requests completions at a type annotation position in a component or function
  signature whose property list spans several lines
- **THEN** the language service SHALL offer primitive NX type names and visible type names
- **AND** it SHALL NOT offer the declaration keyword completions used outside a type position

#### Scenario: A position inside no identifiable construct yields no contextual result
- **WHEN** a client requests a position query at a position the language service cannot resolve to
  an NX construct
- **THEN** the language service SHALL report no result for that query

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
