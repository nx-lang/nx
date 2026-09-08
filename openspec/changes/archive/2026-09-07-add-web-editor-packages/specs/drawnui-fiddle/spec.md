## MODIFIED Requirements

### Requirement: Source pane is an NX-aware editor
The source pane SHALL highlight NX using the syntax definition the repository already publishes for
editors, so that highlighting in the fiddle and in the editor extension cannot drift apart, and it
SHALL obtain that highlighting through the shared Monaco integration rather than a fiddle-local
bridge. The source pane SHALL offer hover and completion answered by the NX language service, with
catalog declarations visible to both, so that a visitor can discover the control set from inside the
editor.

#### Scenario: NX is syntax highlighted
- **WHEN** the source pane contains NX
- **THEN** it SHALL be highlighted according to the repository's published NX grammar

#### Scenario: Grammar is not duplicated
- **WHEN** the repository's published NX grammar changes
- **THEN** the fiddle SHALL pick up the change without a separate grammar being edited

#### Scenario: Highlighting is not fiddle-specific
- **WHEN** the fiddle's editor is inspected
- **THEN** it SHALL contain no TextMate bridge, tokenizer, or theme of its own
- **AND** highlighting SHALL come from the shared Monaco integration

#### Scenario: Hover on a catalog component
- **WHEN** a visitor hovers a tag naming a DrawnUI catalog component
- **THEN** the source pane SHALL show that component's signature, including its properties and their
  types, rendered as highlighted NX

#### Scenario: Hover on the visitor's own declarations
- **WHEN** a visitor hovers a name declared in their own source
- **THEN** the source pane SHALL show what the language service reports for it

#### Scenario: Completions inside a catalog tag
- **WHEN** a visitor requests completions inside the opening tag of a catalog component
- **THEN** the source pane SHALL offer that component's properties not yet supplied

#### Scenario: Positions are not shifted by catalog injection
- **WHEN** the visitor hovers or requests completions at a position
- **THEN** the query SHALL be answered for the position the visitor sees, and any returned range
  SHALL be in the visitor's own lines and columns

#### Scenario: Language features degrade without breaking editing
- **WHEN** the language route cannot be reached or fails
- **THEN** hover and completion SHALL silently offer nothing
- **AND** the source pane SHALL remain editable and compilation SHALL continue to work
