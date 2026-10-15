## MODIFIED Requirements

### Requirement: The source pane reads as well as edits
The playground's source pane, titled "NX", SHALL offer a Source and Visual switch in its title,
Source by default. Source SHALL show the editor, and Visual SHALL show the visitor's source in the
viewer's `<nx-viewer>` element, from the `sourceTree` query the playground's worker answers,
refreshed after the visitor pauses typing as evaluation is and marked stale while a newer tree is
computed. Visual SHALL be colored by the editor's themes, Dark+ and Light+, set in Geist for prose
and Geist Mono for code, and draw its icons from Lucide, loaded apart from the rest of the playground
when Visual first needs them. Switching to Visual SHALL select the node at the editor's cursor, and
switching to Source SHALL put the cursor at the start of the selected node and reveal it. When the
query fails, Visual SHALL say that the reading is unavailable, and Source SHALL keep working.

#### Scenario: Reading an example
- **WHEN** a visitor opens the default example and switches to Visual
- **THEN** the source pane SHALL show the example's reading, and the output pane SHALL be unchanged

#### Scenario: The selection crosses over
- **WHEN** a visitor selects an element in Visual and switches to Source
- **THEN** the editor's cursor SHALL be at the start of that element in the source

#### Scenario: Read on a phone
- **WHEN** a visitor switches to Visual at a viewport 375 pixels wide
- **THEN** the reading SHALL fit the pane's width without the page scrolling horizontally

### Requirement: Examples get a visitor started
The playground SHALL offer between 10 and 20 short examples, grouped by topic, that together cover
the language's main features, and MAY offer, under the topic "Larger examples", a few longer programs
of the kind a real app holds, such as a survey, a settings screen or an orders dashboard, for judging
how the Visual view reads a whole program. The larger examples SHALL NOT count toward the 10 to 20 and
need not fit without scrolling. They SHALL be offered from a single drop-down in the toolbar and SHALL
NOT be featured anywhere else on the page, because their purpose is to show what NX to write rather
than to be browsed. Each short example SHALL be small enough to read without scrolling on a laptop,
and each SHALL link to the documentation page for its topic.

#### Scenario: Topics
- **WHEN** a visitor opens the examples drop-down
- **THEN** it SHALL offer examples, grouped under their topic, for at least: basic values, records
  and defaults, unions, occurrences (`?`, `+`, `*`), sequences, `if`, `for` with a range, functions
  and function values, components, and text with interpolation

#### Scenario: The examples stay in the background
- **WHEN** a visitor opens the playground
- **THEN** the examples SHALL be reachable only through the toolbar's drop-down, which SHALL name
  the current example, or say the source is edited once it no longer matches one

#### Scenario: Choosing an example
- **WHEN** a visitor chooses an example from the drop-down
- **THEN** the source pane SHALL load it, the output pane SHALL show its value, and the address SHALL
  become `/play/<id>` with no fragment

#### Scenario: Leaving edits behind
- **WHEN** a visitor has edited the source and chooses another example
- **THEN** the edited source SHALL remain reachable through the browser's Back button, since its
  address held it

#### Scenario: From an example to its docs
- **WHEN** an example is loaded
- **THEN** the toolbar SHALL show a small link to the documentation page for its topic

#### Scenario: Larger examples
- **WHEN** a visitor opens the examples drop-down
- **THEN** the larger examples SHALL be grouped under "Larger examples", and the playground's tests
  SHALL count only the other examples toward the 10 to 20

## ADDED Requirements

### Requirement: The Visual view uses the program's viewers
The playground's Visual view SHALL ask for the source tree with `viewers`, naming no host viewer declaration
modules, so that viewer declarations in the visitor's source and in the standard libraries it imports shape the
reading. Viewer diagnostics SHALL be reported against the visitor's source like any other diagnostic.

#### Scenario: A viewer declaration in the visitor's source
- **WHEN** a visitor's source declares `type Choice = { value:string label:string }`, a viewer declaration
  `viewer Choice = <ElementViewer title=label id=value />`, and a value
  `let pick = <Choice value="a" label="Apples" />`, and the visitor switches to Visual
- **THEN** the reading of `pick` SHALL show `Apples` as its title and `a` as its id, without quotes

#### Scenario: A misspelled viewer declaration
- **WHEN** the visitor writes `viewer Choice = <ElementViewer title=lable />`
- **THEN** the source pane SHALL show a diagnostic on `lable`
