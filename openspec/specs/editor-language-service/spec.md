# editor-language-service Specification

## Purpose
Define the Rust editor language-service API for NX documents, including workspace snapshots,
diagnostics, document symbols, hover, completions, and stale-result rejection independent of LSP
protocol types.

## Requirements
### Requirement: Language service owns logical editor workspace snapshots
The system SHALL expose a Rust editor language-service API that accepts NX documents as logical
workspace modules independent of LSP protocol types and independent of filesystem-only paths. Each
document in a snapshot SHALL preserve a client URI, a normalized NX module identity, source text,
and a monotonically increasing document version when supplied by the editor client.

#### Scenario: Filesystem document maps to a workspace identity
- **WHEN** a client submits a `file://` NX document from a workspace folder
- **THEN** the language service SHALL preserve the source text under a normalized NX module identity
  derived from the workspace-relative path
- **AND** diagnostics and editor query results for that module SHALL retain enough information to
  map back to the original client URI

#### Scenario: Virtual document does not require filesystem access
- **WHEN** a client submits a virtual NX document with a logical URI such as `nx://tenant/form.nx`
- **THEN** the language service SHALL analyze the submitted source text without requiring a file
  with that identity to exist on disk
- **AND** diagnostics and editor query results SHALL use the submitted logical identity and URI

### Requirement: Language service returns editor diagnostics from static analysis
The language service SHALL expose diagnostics for a workspace snapshot by reusing NX static
analysis, including syntax validation, lowering, scope resolution, type checking, and workspace
import diagnostics. Returned diagnostics SHALL include severity, message, code when available,
primary range, and any secondary labels that can be represented as related locations.

#### Scenario: Static analysis diagnostics are projected for editors
- **WHEN** a workspace snapshot contains an NX type error
- **THEN** the language service SHALL return an editor diagnostic for the affected document
- **AND** the diagnostic SHALL preserve the NX diagnostic severity, code, message, and source range

#### Scenario: Diagnostics are cleared when a document becomes valid
- **WHEN** a document version previously produced diagnostics
- **AND** a later version of the same document analyzes with no diagnostics
- **THEN** the language service SHALL report an empty diagnostics list for that document version

### Requirement: Language service exposes document symbols
The language service SHALL expose document symbols for top-level NX declarations that can be
identified from the parsed and lowered module, including functions, values, records, unions,
actions, components, and top-level root elements where applicable.

#### Scenario: Top-level declarations become document symbols
- **WHEN** a document declares a component, a record type, and a root function
- **THEN** the language service SHALL return document symbols for each top-level declaration
- **AND** each symbol SHALL include the declaration name, kind, selection range, and enclosing range

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

### Requirement: Hover content is markdown written in NX
Hover content SHALL be markdown. Where the content includes a signature, a type, or any other
fragment of NX, that fragment SHALL be emitted in a fenced code block tagged `nx`, so that a client
which highlights NX renders it as code rather than as prose.

Every such fragment SHALL be spelled the way NX spells it. A hover SHALL NOT introduce a keyword,
punctuation, or ordering that does not appear in the language — the fragment shown for a declaration
is the declaration as an author would write it.

A hover SHALL state what kind of thing the position is exactly once. Where the kind is not evident
from the NX fragment itself, it SHALL be written as a parenthesized prefix on the fragment's own
line rather than as a separate sentence repeating the fragment.

#### Scenario: A function declaration's hover shows an NX signature
- **WHEN** a client requests hover on the name of a function declared
  `let add(count:int): int = ...`
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

### Requirement: Language service exposes conservative completions
The language service SHALL expose completion items based on the current document context. MVP
completion sources SHALL include NX keywords, primitive type names, visible top-level declarations,
component or tag names, component property names, and the members or payloadless cases of a
property's declared type, when the current syntax context and available metadata make those
completions valid.

Element and property lookup SHALL follow the import graph of the document being edited, preserving
import aliases and the identity of the declaring module. Completions SHALL NOT be drawn from
declarations that are not visible to that document, and SHALL NOT be selected by matching a
declaration name against an authored tag as plain text.

#### Scenario: Type position includes primitive and visible type completions
- **WHEN** a client requests completions in an NX type annotation position
- **THEN** the language service SHALL include primitive NX type names
- **AND** it SHALL include visible record, union, component, or action type names that are
  available in the current workspace snapshot

#### Scenario: Component property position includes property completions
- **WHEN** a client requests completions inside an element opening tag for a known component
- **THEN** the language service SHALL include undeclared properties accepted by that component
- **AND** it SHALL NOT include properties already supplied in that opening tag

#### Scenario: Property value position includes contextual member completions
- **WHEN** a client requests completions immediately after `=` in a property value position whose
  declared type is a discriminated union
- **THEN** the language service SHALL include the constant cases of that union, as bare names
- **AND** it SHALL NOT include lexically visible variables or unrelated declarations, which cannot
  appear unbraced in that position

#### Scenario: Property value position without a nominal type offers no member completions
- **WHEN** a client requests completions after `=` in a property value position whose declared type
  is not a discriminated union, or whose element or property is unknown
- **THEN** the language service SHALL offer no contextual member completions for that position

#### Scenario: Member completions are offered for an element reached through an import alias
- **WHEN** a client requests completions after `=` on a property of an element written under an
  import alias, such as `<ui.Img fit=`
- **THEN** the language service SHALL resolve the element through that alias
- **AND** it SHALL offer the members of the property's declared type

#### Scenario: Declaration completions do not offer the removed enum keyword
- **WHEN** a client requests completions in declaration position
- **THEN** the language service SHALL NOT offer `enum` as a declaration keyword

#### Scenario: Completions are not drawn from declarations the document cannot see
- **WHEN** another document in the workspace declares a type sharing a name with one used by the
  document being edited, and is not imported by it
- **THEN** the language service SHALL NOT offer members of that other declaration

### Requirement: Language service rejects stale editor results
The language service SHALL associate diagnostics and query results with the snapshot or document
version used to compute them. Editor integrations SHALL be able to determine whether a result is
stale relative to a newer submitted document version.

#### Scenario: Older diagnostic result is superseded by a newer edit
- **WHEN** document version 3 is submitted while analysis for version 2 is still running
- **THEN** the language service SHALL preserve enough version metadata for the integration layer to
  avoid publishing version 2 diagnostics over version 3 state

### Requirement: Language service analyzes a snapshot against a program build context
A workspace snapshot SHALL be constructible with a program build context, and every query over that
snapshot — diagnostics, document symbols, hover, and completions — SHALL see the library modules
that context makes visible under the same visibility rules the compiler applies when building a
program with that context. A snapshot constructed without a context SHALL behave as one built with an
empty context.

#### Scenario: Hover resolves a library component
- **WHEN** a snapshot is built with a build context whose registry loaded a library declaring a
  component
- **AND** a client requests hover on a tag naming that component in a snapshot document
- **THEN** the language service SHALL return that component's signature

#### Scenario: Completions offer library declarations
- **WHEN** a client requests completions at a tag-name position in a document of such a snapshot
- **THEN** the language service SHALL offer the library's visible components
- **AND** WHEN a client requests completions at a property-name position inside a tag naming a
  library component, it SHALL offer that component's undeclared properties

#### Scenario: Diagnostics honor library types
- **WHEN** a snapshot document uses a library component correctly
- **THEN** the language service SHALL NOT report the component or its properties as unresolved
- **AND** WHEN the document supplies a property value of the wrong type, the language service SHALL
  report the same type diagnostic the compiler reports

#### Scenario: Snapshot without a context sees no libraries
- **WHEN** a snapshot is built without a build context
- **THEN** names declared only in a library SHALL be treated as unresolved

### Requirement: Editor positions are UTF-16 code unit offsets
The language service SHALL interpret the character component of an incoming text position as a
zero-based offset within its line counted in UTF-16 code units, and SHALL express the character
component of every outgoing position the same way. Byte offsets carried alongside a range SHALL
remain UTF-8 byte offsets into the document text.

#### Scenario: Query after a non-BMP character resolves correctly
- **WHEN** a line reads `let s = "😀" + name` and a client requests hover with the character offset
  that an editor counting UTF-16 units reports for `name`
- **THEN** the language service SHALL resolve the position to `name`

#### Scenario: Returned range counts UTF-16 units
- **WHEN** the language service returns a range for a construct that follows a non-BMP character on
  its line
- **THEN** the range's character offsets SHALL count that character as two units
- **AND** the range's byte offsets SHALL count it as four bytes

#### Scenario: Offsets past the end of a line clamp
- **WHEN** a client requests a position whose character offset exceeds the line's length in UTF-16
  units
- **THEN** the language service SHALL treat it as the end of that line

#### Scenario: ASCII behavior is unchanged
- **WHEN** a document contains only ASCII text
- **THEN** every position and range SHALL be identical to the values reported before this
  requirement
