## Purpose

Defines the NX viewer: the `<nx-viewer>` custom element, published in `@nx-lang/viewer`, that shows
a document's source tree as a reading view. Syntax becomes style and meaning stays visible: every
construct is shown or one click away, and every rendered piece maps back to its source text. The
element depends on no UI framework, so any host, the playground among them, can embed it.

## ADDED Requirements

### Requirement: One element shows a source tree in any host
The repository SHALL publish `@nx-lang/viewer`, which defines an `<nx-viewer>` custom element that
shows a `SourceTree`, as `@nx-lang/language-protocol` types it, together with the text of the
document the tree describes. The element SHALL depend on no UI framework, SHALL keep its markup and
styles in a shadow root, and SHALL follow the page's light or dark scheme. Where there is no DOM,
importing the package SHALL do nothing, and the package SHALL export a function that defines the
element in a registry the host chooses.

#### Scenario: A plain page
- **WHEN** a page with no framework imports the package, adds `<nx-viewer>` and sets its `tree` and
  `text`
- **THEN** the element SHALL show the document's reading view

#### Scenario: Host styles don't reach in
- **WHEN** the host page styles `div`, `span` or `button` elements globally
- **THEN** the element's rendering SHALL not change

#### Scenario: A stale tree
- **WHEN** the host sets the element's `stale` property while it computes a newer tree
- **THEN** the element SHALL mute its reading and mark it out of date, and selection and hover SHALL
  still work

### Requirement: The reading is lossless
The element SHALL render every node of the tree, either inline or in a details strip that one
action opens, and the rendering of each node SHALL carry the node's key. Import nodes SHALL be the
only nodes folded into a details strip, at the top of the document; comments SHALL read in place.
An `unparsed`
node SHALL be shown as its source text, marked as a region that could not be read. Every rendered
node SHALL offer its source: an action that shows the exact text of the node's range, sliced from
the document's text by the range's UTF-8 byte offsets.

#### Scenario: Coverage over the repository
- **WHEN** the viewer's coverage test renders the source tree of every `.nx` file in the repository
  with its details strips open
- **THEN** the key of every node SHALL appear in the rendering exactly once

#### Scenario: A forgotten role fails the test
- **WHEN** the renderer for a role is removed
- **THEN** the coverage test SHALL fail naming a file and the key of a node it did not render

#### Scenario: The source of a card
- **WHEN** a reader opens the source of the card for `<Choice value="other" label="Something else" />`
- **THEN** the element SHALL show exactly `<Choice value="other" label="Something else" />`

#### Scenario: Text after an emoji
- **WHEN** a reader opens the source of a literal that follows an emoji on its line
- **THEN** the element SHALL show exactly the literal's text

### Requirement: Elements read as cards
The element SHALL render an `element` node as a card headed by its kind's name split at its
capitals into sentence case, so `SingleChoice` reads "Single choice", and SHALL show the name as
written, marked as unresolved, when the node refers to no declaration. A declaration whose value is
an element SHALL show the declaration's name on the card. The card SHALL list the element's
attributes in the order its declaration lists their properties, inherited properties first, followed
by any attribute the declaration does not list, in source order. Attributes bound by content SHALL
list their items under the property's name, and nested elements SHALL be nested cards.

#### Scenario: A survey question
- **WHEN** the tree is that of `let roleQuestion = <SingleChoice id="role" label="Which describes your work best?" layout=chips allowsOther={true}>`
  with four `<Choice … />` children, and `SingleChoice` declares `id`, `label`, `help`, `required`
  and `requiredMessage` through its base before `allowsOther`, `layout` and `choices`
- **THEN** the card SHALL be headed "Single choice" and name `roleQuestion`
- **AND** its rows SHALL be `id`, `label`, `allowsOther` and `layout` in that order, followed by
  `choices` holding four nested "Choice" cards

#### Scenario: An unknown kind
- **WHEN** an element's kind refers to no declaration
- **THEN** its card SHALL be headed by the kind as written and marked as unresolved

### Requirement: Values have a style and lose their quotes
The element SHALL show a string literal without its quotes in a value style, a number in tabular
figures, a boolean as a check or a cross with its value as its accessible name, a `case` node as a
pill holding the case name, `{}` as an empty marker, and a `sequence` as a list of its items. A
sequence none of whose items is a block, such as an element or a comment, SHALL read on one line
with its items separated by commas. It SHALL show a `text` node as prose, rendering it as markdown
when its text type is `markdown`, and an `embed` inside text as a slot. A string and a name that
read alike SHALL remain distinguishable by style.

#### Scenario: A string and a case
- **WHEN** an element sets `label="Design"` and `layout=chips`
- **THEN** the label row SHALL show `Design` without quotes in the value style
- **AND** the layout row SHALL show a `chips` pill

#### Scenario: A list on one line
- **WHEN** an element sets `items={ "red" "green" "blue" }`
- **THEN** the items row SHALL read `red, green, blue`

#### Scenario: Markdown text
- **WHEN** an element has a body typed `markdown` holding a paragraph with emphasis
- **THEN** the element SHALL render the paragraph with the emphasis as prose

### Requirement: Omitted defaults show as ghosts
The element SHALL show a ghost row for each property that an element's declaration gives a
default and the element does not set: muted, marked as a default, and holding the default's source
text from the declaration table. Ghost rows SHALL be shown by default and hidden when the element's
`ghosts` property is false. A ghost row SHALL carry no key, since it has no node.

#### Scenario: A default that is not set
- **WHEN** a `SingleChoice` element does not set `required`, which its declaration defaults to `true`
- **THEN** its card SHALL show a ghost row `required` with the value `true` marked as a default

#### Scenario: Ghosts hidden
- **WHEN** the host sets `ghosts` to false
- **THEN** no ghost row SHALL be shown

### Requirement: Logic reads as sentences
The element SHALL read an `operator` node by the viewer's operator list: `==` `!=` `<` `>` `<=` `>=`
as = ≠ < > ≤ ≥, `+` `-` `*` `/` and prefix `-` as + − × ÷ and −, `%` as "remainder of a ÷ b", `&&`
`||` `!` as "and", "or" and "not", `a ?? b` as "a, otherwise b", `x?` as "x is given", `a..b` as "a
up to b" and `a..=b` as "a through b". A `+` whose type is `string` SHALL read as a sentence with each
non-literal operand as a slot. An operator token the list does not name SHALL be shown as written.
The element SHALL keep the parentheses the source writes and add none. It SHALL read a `condition`
as a label "Only when" followed by its test above the guarded content, and its else branch under
"Otherwise"; a `match` as a table of its arms and their results; and a `loop` as "For each" followed
by its bindings and iterable, above its body.

#### Scenario: A guarded step
- **WHEN** a component body holds `if step == 3 && role != "engineer" { <Step … /> }`
- **THEN** the step's card SHALL be labelled "Only when step = 3 and role ≠ engineer", with
  `engineer` in the value style

#### Scenario: A sentence with a slot
- **WHEN** an attribute's value is `"Thanks, " + name + ". Where can we reach you?"`
- **THEN** it SHALL read "Thanks, name. Where can we reach you?" with `name` as a slot

#### Scenario: A fallback
- **WHEN** an attribute's value is `nickname ?? "Anonymous"`
- **THEN** it SHALL read "nickname, otherwise Anonymous"

### Requirement: Handlers read as events
The element SHALL read an attribute flagged `handler` as "When" followed by its name, without an
`on` prefix, split into lower-case words. When the handler's value is an element flagged
`stateUpdate`, it SHALL list one line per attribute of that element reading "set", the attribute's
name, "to" and its value. Any other handler value SHALL be read after an arrow as an ordinary value.

#### Scenario: A state update
- **WHEN** an element binds `onIntegerAnswered=<Update answered={answered + 1} teamSize={action.answer.value} />`
  inside a component body
- **THEN** it SHALL read "When integer answered" followed by "set answered to answered + 1" and "set
  teamSize to action.answer.value"

### Requirement: Declarations, references and comments read in place
The element SHALL render a `type`, `action` or `component` declaration as a table of its members,
each with its name, its type as NX spells it, its default when it has one and its doc comment, and a
union as the list of its cases. It SHALL render a function declaration with its parameters in the
same form above its body. It SHALL show a `reference` as a link naming its target, a `member` as its
object followed by the member name, and a `call` as its callee with its arguments. It SHALL show a
doc comment as prose above what it documents and a comment as a muted note where it stands. Line
comments that stand alone on consecutive lines SHALL read as one note; a comment that trails code on
its line SHALL not join them.

#### Scenario: A record type
- **WHEN** a document declares `type User = { id:string name:string? }` with a doc comment on `name`
- **THEN** the element SHALL show a table with the rows `id` of type `string` and `name` of type
  `string?` with its doc comment

#### Scenario: A comment over several lines
- **WHEN** a document has the comment lines `// The first line of a note` and
  `// and its second line.` one after the other, then a blank line and `// A note of its own.`
- **THEN** the element SHALL show the first two lines as one note and the third as a note of its own

### Requirement: Hover and peek explain what a node refers to
Hovering an attribute's name SHALL show, from the declaration table, the property's type, doc
comment and default and whether it is optional, and, when its type is a union, every case of the
union with the attribute's case marked. Hovering a reference SHALL show its target's name, kind and
doc comment. A reference whose target is a value the document declares SHALL offer to expand in
place, showing the nodes of the target's value range as they render where they are declared.

#### Scenario: The cases of a union
- **WHEN** a reader hovers the name of `layout=chips` and `ChoiceLayout` declares the cases `list`,
  `grid` and `chips`
- **THEN** the hover SHALL list the three cases with `chips` marked

#### Scenario: Expanding a question
- **WHEN** a reader expands the reference in `question={teamSizeQuestion}` and the document declares
  `let teamSizeQuestion = <Integer … />`
- **THEN** the `Integer` card SHALL be shown in place under the reference

### Requirement: A selection is shared by key
A click on a rendered node SHALL select the smallest node under the pointer, mark it, and dispatch an
`nx-select` event whose detail carries the node's key, role and range. Setting the element's
`selection` property to a key SHALL select that node, open any fold or details strip around it and
scroll it into view, without dispatching the event; a key the tree does not hold SHALL clear the
selection. The package SHALL export `nodeAtSpan(tree, start, end)`, returning the index of the node
whose range has exactly those start and end byte offsets, preferring an `element` node, or nothing
when no node has them.

#### Scenario: Selecting a choice
- **WHEN** a reader clicks the label of the third choice of `roleQuestion`
- **THEN** the element SHALL dispatch `nx-select` with the key of that `label` attribute's literal

#### Scenario: Selecting from the host
- **WHEN** the host sets `selection` to the key of an import inside the closed details strip
- **THEN** the strip SHALL open and the import SHALL be marked and scrolled into view

#### Scenario: From an origin to a node
- **WHEN** a host holds a value origin whose span is that of a `<SingleChoice … />` element
- **THEN** `nodeAtSpan` SHALL return the index of that element's node
