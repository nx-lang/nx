## MODIFIED Requirements

### Requirement: The reading is lossless
The element SHALL render every node of the tree, either inline or in a details strip that one
action opens, and the rendering of each node SHALL carry the node's key. Import nodes SHALL be folded
into a details strip at the top of the document, and an element's attributes SHALL be folded into a
details strip at the foot of its card only where a viewer places them there; no other node SHALL be
folded, and comments SHALL read in place. A viewer SHALL never hide a node.
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

#### Scenario: A property placed in details folds but stays
- **WHEN** the tree carries viewers that place `id` in details, and a card's element sets `id="role"`
- **THEN** the card's details strip SHALL hold the `id` row with the key of its attribute node
- **AND** the coverage test SHALL still find that key exactly once

### Requirement: Elements read as cards
The element SHALL render an `element` node as a card headed by the `name` of its kind's `viewer`
value when the declaration entry carries one, and otherwise by its kind's name split at its
capitals into sentence case, so `SingleChoice` reads "Single choice", and SHALL show the name as
written, marked as unresolved, when the node refers to no declaration. A declaration whose value is
an element SHALL show the declaration's name on the card. The card SHALL list the element's
attributes in the order of the `properties` of its kind's `viewer` value when it carries one, and
otherwise in the order its declaration lists their properties, inherited properties first, followed
in both cases by any attribute the declaration does not list, in source order. Attributes bound by content SHALL
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

#### Scenario: A kind named by its viewer
- **WHEN** the tree carries a `viewer` value for `SingleChoice` with the name `Pick one`
- **THEN** a `SingleChoice` card SHALL be headed "Pick one"

## ADDED Requirements

### Requirement: Viewers shape a card
When an element's kind carries a `viewer` value, the card SHALL show the attribute bound to the
viewer declaration's `title` as the card's title, the one bound to `subtitle` under it, the one bound to `badge`
as a badge beside the heading, and the one bound to `image` as a picture, each still carrying its
node's key and offering its source, and SHALL NOT show those attributes again as rows. A row SHALL be
labelled with its property's `label` when the viewer sets one and with the property's name otherwise.
A boolean property with `whenTrue` or `whenFalse` SHALL read as that phrase, in place of its row,
when the attribute or its ghost has that value, and as an ordinary row when the matching phrase is
not set. A property placed in `details` SHALL be shown, row and ghost alike, in a details strip at
the foot of the card that one action opens. A `row` layout SHALL show the card as one line holding
its heading, title, subtitle, badge and image, with every other row in its details strip, and an
`inline` layout SHALL show the element within the line that holds it, as its heading and title, with
every other row in its details strip. A ghost row SHALL follow the same viewer as the property it
stands for.

#### Scenario: A question with a title and a phrase
- **WHEN** the tree carries a viewer for `Question` with `title=label`, a property viewer for `required` with
  `whenTrue="Required"` and `whenFalse="Optional"`, and one for `id` placed in details, and a
  `SingleChoice` element sets `id="role"` and `label="Which describes your work best?"` and leaves
  `required` to its default `true`
- **THEN** the card's title SHALL read "Which describes your work best?" with the key of the
  `label` attribute node
- **AND** the card SHALL show the ghost phrase "Required" and no `required` row
- **AND** the `id` row SHALL be in the card's details strip

#### Scenario: Choices as rows
- **WHEN** the tree carries a viewer for `Choice` with `layout=row`, `title=label`, `subtitle=description` and
  `badge=value`
- **THEN** each `Choice` element SHALL read as one line with its label, its description and its
  value as a badge

### Requirement: Viewers label cases and format values
A `case` node of a union whose entry carries a `viewer` value SHALL show its case's `label` in its
pill when the viewer sets one, with the case name and the `example` in its hover. A string literal
bound to a property with a resolved format SHALL be shown by that format, still without its quotes
and still offering its source: `url` and `email` as links, `phone` as a link that dials, `image` as
a small picture with the URL beside it when the URL is `https:` or `http:` and the element's
`images` property is not false, and as a link otherwise, `color` as a swatch beside the text, `markdown` as prose, and
`text` as a plain value. A format SHALL NOT change a value that is not a string literal, such as an
embed or a reference.

#### Scenario: A labelled case
- **WHEN** the tree carries a viewer for `ChoiceLayout` with the label "chips" for `chips` and "a list" for `list`,
  and an element sets `layout=list`
- **THEN** the layout row SHALL show a pill reading "a list"

#### Scenario: An image URL
- **WHEN** a `Choice` element sets `imageUrl="https://example.com/a.png"` and `imageUrl`'s resolved
  format is `image`
- **THEN** the row SHALL show a picture loaded from that URL and the URL beside it

#### Scenario: Images turned off
- **WHEN** the host sets the element's `images` property to false
- **THEN** the same row SHALL show the URL as a link and load nothing

### Requirement: A viewer declaration reads as a declaration
The element SHALL render a `viewer` declaration as a declaration headed "Viewer for" its target,
with its value read as any element is, so a reader sees how a kind's viewer is declared in the same terms as
the rest of the document.

#### Scenario: A viewer declaration in the document
- **WHEN** a document holds `viewer Choice = <ElementViewer title=label />`
- **THEN** the element SHALL show a declaration headed "Viewer for Choice" holding an "Element
  viewer" card with a `title` row reading `label`
