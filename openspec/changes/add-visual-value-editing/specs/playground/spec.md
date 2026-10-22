## ADDED Requirements

### Requirement: Visual edits the visitor's source through the editor
The playground SHALL set the viewer's `editable` in Visual. For each `nx-edit`, it SHALL ask its
worker's language service for the `sourceEdit` of the visitor's source and apply the answer's edits
to the editor's model as one undoable step, so that Source shows them and undo and redo take them back
and forth in either view; with Visual focused, Ctrl or Cmd with Z SHALL undo and Ctrl+Y or Ctrl or Cmd
with Shift+Z SHALL redo. The NX pane's title bar SHALL show Undo and Redo buttons in both views, each
disabled while the editor's history holds nothing to take back or forth, with tooltips naming their
shortcuts on the visitor's platform; in Visual, Undo SHALL also be enabled while a field holds a
change not yet committed, as `nx-draft` says, or an edit Visual fired is not yet in the history. An undo or redo, from a button or a shortcut, SHALL take its step
after every edit Visual fired before it, so that pressing Undo with a field open, which commits the
field, takes back the field's edit. On a narrow screen, the title bar's Collapse all and Expand all
SHALL show as icons, so the view switch stays in view. After an edit from Visual the playground SHALL ask for the source tree at once,
without waiting for a pause, and the preview and output SHALL follow as they do after an edit in
Source. Edits SHALL be applied in the order the viewer fires them, each against the text the one before
left. A refused edit SHALL leave the source as it was. For `nx-open`, the playground SHALL switch to
Source with the cursor at the start of the node and reveal it.

#### Scenario: An edit in Visual shows in Source
- **WHEN** a visitor on the visualizer declarations example changes the `Latte` drink's name to
  `Flat white` in Visual and switches to Source
- **THEN** the source SHALL read `name="Flat white"` where it read `name="Latte"`

#### Scenario: Undo from Visual
- **WHEN** the visitor then switches back to Visual and presses Ctrl+Z
- **THEN** the source SHALL read `name="Latte"` again, and Visual SHALL read `Latte`

#### Scenario: Undo from the title bar
- **WHEN** the visitor makes an edit in Visual and presses the title bar's Undo button
- **THEN** the source SHALL read as before the edit, and Redo SHALL be enabled

#### Scenario: Undo before the first edit
- **WHEN** a visitor opens an example, types in a field in Visual and, before committing it, presses
  the title bar's Undo button
- **THEN** the source SHALL read as it did, and Visual SHALL read the value as before

#### Scenario: Opening an expression in Source
- **WHEN** a visitor double-clicks a braced expression in Visual
- **THEN** Source SHALL show, with the cursor at the start of that expression

## MODIFIED Requirements

### Requirement: Examples get a visitor started
The playground SHALL offer between 10 and 20 short examples, grouped by topic, that together cover
the language's main features, and MAY offer, under the topic "Larger examples", a few longer programs
of the kind a real app holds, such as a survey, a settings screen or an orders dashboard, for judging
how the Visual view reads a whole program, and, under the topic "Test cases", one example holding
one or two of every case the Visual view reads and edits, each with a comment saying what to try. The
larger examples and the test cases SHALL NOT count toward the 10 to 20 and need not fit without
scrolling. They SHALL be offered from a single drop-down in the toolbar and SHALL
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
  SHALL count toward the 10 to 20 only the examples under neither "Larger examples" nor "Test cases"

#### Scenario: Visual test cases
- **WHEN** a visitor opens the "Visual test cases" example and switches to Visual
- **THEN** it SHALL hold, with a comment for each saying what to try, a one-line element, a property
  sheet, an element as a property's value, a union case as an element, a group with no title to fold,
  an element whose visualizer merges with its base's and puts its id last, an icon Visual cannot draw,
  a function used as an element, text content around a braced value and over two lines, a doc on a
  type and on a property, and a value of every kind Visual edits: a string with escapes, `uitext`, an
  id formatted by a value visualizer, an `int`, an alias of one, a `float64`, an untyped number, a
  boolean written bare and in braces, a case written bare and qualified, a color formatted by a value
  visualizer and by a property visualizer, a value after an emoji and a sequence of numbers, together
  with a string Visual opens in Source, an expression, a `for` and an `if`, and properties that may
  and may not be left out
