## ADDED Requirements

### Requirement: The source pane reads as well as edits
The playground's source pane SHALL offer an Edit and Read switch in its title, Edit by default. Read
SHALL show the visitor's source in the viewer's `<nx-viewer>` element, from the `sourceTree` query the
playground's worker answers, refreshed after the visitor pauses typing as evaluation is and marked
stale while a newer tree is computed. Switching to Read SHALL select the node at the editor's cursor,
and switching to Edit SHALL put the cursor at the start of the selected node and reveal it. When the
query fails, Read SHALL say that the reading is unavailable, and Edit SHALL keep working.

#### Scenario: Reading an example
- **WHEN** a visitor opens the default example and switches to Read
- **THEN** the source pane SHALL show the example's reading view, and the output pane SHALL be
  unchanged

#### Scenario: The selection crosses over
- **WHEN** a visitor selects a card in Read and switches to Edit
- **THEN** the editor's cursor SHALL be at the start of that card's element in the source

#### Scenario: Read on a phone
- **WHEN** a visitor switches to Read at a viewport 375 pixels wide
- **THEN** the reading view SHALL fit the pane's width without the page scrolling horizontally
