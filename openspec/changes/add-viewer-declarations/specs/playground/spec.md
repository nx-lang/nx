## MODIFIED Requirements

### Requirement: The source pane reads as well as edits
The playground's source pane SHALL offer an Edit and Read switch in its title, Edit by default. Read
SHALL show the visitor's source in the viewer's `<nx-viewer>` element, from the `sourceTree` query the
playground's worker answers, refreshed after the visitor pauses typing as evaluation is and marked
stale while a newer tree is computed. Read SHALL be colored by the editor's themes, Dark+ and Light+,
set in Geist for prose and Geist Mono for code, and draw its icons from Lucide, loaded apart from the
rest of the playground when Read first needs them. Switching to Read SHALL select the node at the
editor's cursor, and switching to Edit SHALL put the cursor at the start of the selected node and
reveal it. When the query fails, Read SHALL say that the reading is unavailable, and Edit SHALL keep
working.

#### Scenario: Reading an example
- **WHEN** a visitor opens the default example and switches to Read
- **THEN** the source pane SHALL show the example's reading, and the output pane SHALL be unchanged

#### Scenario: The selection crosses over
- **WHEN** a visitor selects an element in Read and switches to Edit
- **THEN** the editor's cursor SHALL be at the start of that element in the source

#### Scenario: Read on a phone
- **WHEN** a visitor switches to Read at a viewport 375 pixels wide
- **THEN** the reading SHALL fit the pane's width without the page scrolling horizontally

## ADDED Requirements

### Requirement: The Read view uses the program's viewers
The playground's Read view SHALL ask for the source tree with `viewers`, naming no host viewer declaration
modules, so that viewer declarations in the visitor's source and in the standard libraries it imports shape the
reading. Viewer diagnostics SHALL be reported against the visitor's source like any other diagnostic.

#### Scenario: A viewer declaration in the visitor's source
- **WHEN** a visitor's source declares `type Choice = { value:string label:string }`, a viewer declaration
  `viewer Choice = <ElementViewer title=label id=value />`, and a value
  `let pick = <Choice value="a" label="Apples" />`, and the visitor switches to Read
- **THEN** the reading of `pick` SHALL show `Apples` as its title and `a` as its id, without quotes

#### Scenario: A misspelled viewer declaration
- **WHEN** the visitor writes `viewer Choice = <ElementViewer title=lable />`
- **THEN** the source pane SHALL show a diagnostic on `lable`
