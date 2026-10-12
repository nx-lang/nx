## ADDED Requirements

### Requirement: The Read view uses the program's hints
The playground's Read view SHALL ask for the source tree with `viewers`, naming no host hint
modules, so that hints in the visitor's source and in the standard libraries it imports shape the
reading. Hint diagnostics SHALL be reported against the visitor's source like any other diagnostic.

#### Scenario: A hint in the visitor's source
- **WHEN** a visitor's source declares `type Choice = { value:string label:string }`, a hint
  `viewer Choice = <ElementViewer name="Option" title=label />`, and a value
  `let pick = <Choice value="a" label="Apples" />`, and the visitor switches to Read
- **THEN** the card for `pick` SHALL be headed "Option" with the title "Apples"

#### Scenario: A misspelled hint
- **WHEN** the visitor writes `viewer Choice = <ElementViewer title=lable />`
- **THEN** the source pane SHALL show a diagnostic on `lable`
