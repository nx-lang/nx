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
- **AND** it SHALL hold an element with unwritten properties to add, among them one whose type has no
  editor, a value written as a reference, which opens Source, a number with labeled choices and a
  range, a number with a range and no editor, a string with choices, a case hinted as segmented,
  a string hinted multi-line with a placeholder, and a string written over several lines
- **AND** it SHALL hold lists to reorder: content of four or more steps, among them a group to fold, one
  with a comment directly above it and an `if` block holding a step, a list property written an item
  to a line, one written on one line, and a list of one item, which has no grip
