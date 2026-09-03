## ADDED Requirements

### Requirement: An element body may be empty
An element written with an opening and a closing tag SHALL parse when nothing appears between them.
An empty body SHALL mean the same as a self-closing tag: no body content is supplied, and a declared
content property is left unset rather than bound to an empty sequence.

Whitespace and comments SHALL NOT constitute body content. An empty body SHALL NOT count as
supplying the content property, so a target that declares no content property SHALL accept it.

#### Scenario: An element with an empty body parses
- **WHEN** a file contains `abstract external component <Node /> external component <App extends Node content Children:Node[]? /> <App></App>`
- **THEN** parsing SHALL accept the file
- **AND** it SHALL NOT report a syntax error

#### Scenario: An empty body leaves the content property unset
- **WHEN** two files differ only in that one writes `<App></App>` where the other writes `<App />`
- **THEN** evaluation SHALL produce the same value for both
- **AND** the declared content property SHALL be unset in both

#### Scenario: A body holding only whitespace and comments is empty
- **WHEN** a file contains an element whose body spans several lines and holds only a comment
- **THEN** parsing SHALL accept the element
- **AND** its declared content property SHALL be unset

#### Scenario: An empty body is accepted by a target that declares no content property
- **WHEN** a file contains `abstract external component <Node /> external component <Plain extends Node title:string? /> <Plain title="hi"></Plain>`
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
