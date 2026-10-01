## ADDED Requirements

### Requirement: Hover shows a declaration's documentation
Where a hover reports a documented declaration or member — at the place it is declared or at any
position that refers to it, including a reference into another module, an imported library, or the
prelude — the hover content SHALL include the documentation after the fenced NX fragment. The
fragment SHALL remain first and unchanged. A declaration or member with no documentation SHALL hover
exactly as it did before documentation existed.

In the documentation a hover shows, a doc link that resolves SHALL be shown as its label in a code
span, without its brackets. A doc link that does not resolve SHALL be shown as written.

#### Scenario: Hover on a documented declaration shows its documentation
- **WHEN** a client requests hover on the name of `let add(count:int): int = ...` whose documentation
  is `Adds one.`
- **THEN** the hover content SHALL contain the fenced block spelling `let add(count:int): int`
- **AND** the hover content SHALL contain `Adds one.` after that block

#### Scenario: Hover on a reference shows the declaration's documentation
- **WHEN** a client requests hover on a call of `add` in another module that imports it
- **THEN** the hover content SHALL contain the documentation of `add`

#### Scenario: Hover on a property in an element tag shows the property's documentation
- **WHEN** a component declares the property `placeholder:string   /// Hint text.` and a client
  requests hover on `placeholder` in `<SearchBox placeholder="Find" />`
- **THEN** the hover content SHALL contain `Hint text.`

#### Scenario: Hover on a union case shows the case's documentation
- **WHEN** a union case `idle` has the documentation `No search has run yet.` and a client requests
  hover on `LoadState.idle` in an expression
- **THEN** the hover content SHALL contain `No search has run yet.`

#### Scenario: Hover on a library declaration shows its documentation
- **WHEN** a library declaration has documentation and a client requests hover on a reference to it
  from a document that imports the library
- **THEN** the hover content SHALL contain that documentation

#### Scenario: A resolved doc link is shown as code
- **WHEN** a hover shows documentation containing `See [renderNotice].` and the link resolves
- **THEN** the hover content SHALL contain ``See `renderNotice`.``

### Requirement: Hover on a doc link reports its target
A hover at a doc link inside a doc comment SHALL report what a hover at a reference to the link's
target reports. A hover at a doc link that does not resolve SHALL report no result.

#### Scenario: Hover on a doc link reports the linked declaration
- **WHEN** a doc comment contains `[renderNotice]` and a client requests hover on `renderNotice`
  inside it
- **THEN** the hover content SHALL be the hover content for a reference to `renderNotice`

#### Scenario: Hover on an unresolved doc link reports nothing
- **WHEN** a client requests hover inside `[Missing]` in a doc comment and no visible name `Missing`
  exists
- **THEN** the language service SHALL report no result

### Requirement: Completion items carry documentation
A completion item for a documented declaration, component property, or union case SHALL carry that
declaration's documentation as markdown, rendered as hover renders it. An item for something with no
documentation SHALL carry none.

#### Scenario: A property completion carries the property's documentation
- **WHEN** a client requests completions inside the opening tag of a component that declares
  `placeholder:string   /// Hint text.`
- **THEN** the `placeholder` item SHALL carry the documentation `Hint text.`
- **AND** its detail SHALL be unchanged

#### Scenario: A declaration completion carries its documentation
- **WHEN** a client requests completions where a documented top-level declaration is visible
- **THEN** that declaration's item SHALL carry its documentation

#### Scenario: A case completion carries the case's documentation
- **WHEN** a client requests completions after `=` for a property whose type is a union with a
  documented constant case
- **THEN** that case's item SHALL carry its documentation

### Requirement: Doc link names are completed
Inside a `///` doc comment, after an open `[` outside a code span, the language service SHALL
complete the name of a doc link the way the link is resolved. With no `.` written yet, it SHALL
offer the members of the declarations the documentation belongs to or belongs within, innermost
first, and then every visible declaration. After `Type.` or `Type.case.`, it SHALL offer the members
of what that names: a record's fields and a component's properties, `state` fields, and `emits`
entries, inherited ones included, a union's cases, and a case's payload fields. Each item SHALL
carry the documentation of what it names. Anywhere else in a comment, ordinary or doc, the service
SHALL offer no completions.

#### Scenario: A doc link completes the owner's members, then visible declarations
- **WHEN** a client requests completions after `[` in a trailing doc comment on a component's
  property `start`, in a component with a `state` field `count`, in a document that also declares
  `helper`
- **THEN** the items SHALL begin with `start` and `count`, and SHALL include `helper`
- **AND** they SHALL NOT include keywords

#### Scenario: A qualified doc link completes the members of what it names
- **WHEN** a client requests completions after `[LoadState.i` in a doc comment, where `LoadState`
  is a union of `idle`, documented `Nothing has run.`, and `failed { message:string }`
- **THEN** the items SHALL be `idle` and `failed`, and `idle` SHALL carry `Nothing has run.`
- **AND** after ``[`LoadState.failed.`` the only item SHALL be `message`

#### Scenario: A comment offers no completions outside a doc link
- **WHEN** a client requests completions in a `//` comment, in a doc comment outside a `[`, inside a
  code span, in a link's destination, or in a block comment
- **THEN** the service SHALL offer no completions
