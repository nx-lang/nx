## ADDED Requirements

### Requirement: An item moves within its list
For `move`, the target SHALL be an element that is an item of a sequence or of element content, not
inside a condition's branch or a loop's body, in a list that holds no text, and `to` SHALL be the
position the item takes in its list once moved, counted as the list's keys count its items. The
service SHALL move the item in the list's layout. In a list written an item to a line, where no item
shares a line with another, it SHALL move the item's whole lines, with the comments and doc comments
directly above it, to stand before the item that will follow it, or after the last item, keeping
the item's indentation; blank lines between other items SHALL stay where they are. In a list written
on one line, it SHALL move the item within the line, one space from its neighbours. An item MAY move
past a condition or a loop that is an item of the list. A `to` equal to the item's position SHALL
answer no edits.

#### Scenario: A step moves down
- **WHEN** a client moves `steps[0]` of a `QuestionFlow` whose content holds the steps `A`, `B` and
  `C`, each written over its own lines, to `1`
- **THEN** the content SHALL read `B`, `A`, `C`, each with its lines and indentation as written,
  and nothing else in the document SHALL change

#### Scenario: Its comment goes with it
- **WHEN** the comment `// Ask this first` stands on the line directly above step `C` and a client
  moves `C` to `0`
- **THEN** the comment SHALL stand directly above `C`, before `A`

#### Scenario: On one line
- **WHEN** a client moves the first item of `choices={ <Choice value="a" /> <Choice value="b" /> }`
  to `1`
- **THEN** the attribute SHALL read `choices={ <Choice value="b" /> <Choice value="a" /> }`

#### Scenario: Past a condition
- **WHEN** an element's content holds `<A />`, then `if wide { <B /> }`, then `<C />`, each on its
  own lines, and a client moves `<C />` to `1`
- **THEN** the content SHALL read `<A />`, `<C />`, then the `if` block

#### Scenario: To where it is
- **WHEN** a client moves `steps[1]` to `1`
- **THEN** the answer SHALL hold no edits and no refusal

### Requirement: An item is removed with its lines
The service SHALL also take for `remove` an item that `move` takes, whatever the list's length. In
a list written an item to a line, it SHALL remove the item's whole lines with the comments and
doc comments directly above it, and, where two blank lines would then stand together, one of them;
in a list written on one line, it SHALL remove the item and the space that separates it from the item
before it, or after it when it is the first. The service SHALL NOT check that the list may lose the
item.

#### Scenario: A step and its comment
- **WHEN** the comment `// Ask this first` stands on the line directly above step `B` of the steps
  `A`, `B` and `C`, each written over its own lines, and a client removes `B`
- **THEN** the content SHALL read `A` and `C` as written, and the comment and every line of `B`
  SHALL be gone

#### Scenario: On one line
- **WHEN** a client removes the first item of `choices={ <Choice value="a" /> <Choice value="b" /> }`
- **THEN** the attribute SHALL read `choices={ <Choice value="b" /> }`

## MODIFIED Requirements

### Requirement: A property is removed with its separating space
For `remove`, the target SHALL be an attribute, or an item as "An item is removed with its lines"
says. The service SHALL remove the attribute and the whitespace that separates it from the attribute
or tag before it, and a `//` comment that follows it on its line; when the attribute stands alone on
its line, the service SHALL remove that whole line. A comment on the line above SHALL stay. The
service SHALL NOT check that the property may be left out.

#### Scenario: On one line
- **WHEN** a client removes `size=large` from `<Drink code="CBR" size=large iced={true} />`
- **THEN** the element SHALL read `<Drink code="CBR" iced={true} />`

#### Scenario: On its own line
- **WHEN** a client removes an attribute that stands alone on its line in an element written one
  attribute per line
- **THEN** that line SHALL be gone and every other line SHALL be unchanged

#### Scenario: With a comment after it
- **WHEN** a client removes `a=1` from a line that reads `  a=1 // note`
- **THEN** the whole line, comment included, SHALL be gone

### Requirement: The language service answers a source edit for a node
The language service SHALL answer a `sourceEdit` query, which carries the documents, the URI of the
document to edit, the key of a node in that document's source tree, and one edit: `setValue` with a
typed value, `setText` with a line of text, `remove`, `addProperty` with a property's name and a
typed value, or `move` with a position. It SHALL resolve the key against the document as the request
carries it, by the keys its source tree gives. Its answer SHALL be the text edits that make the
change, each a range in the protocol's form and its new text. Every range SHALL be in the document as
sent, and no two SHALL overlap, so a client applies them together as one change. The edits SHALL
touch nothing outside the target and the whitespace (and a trailing line comment, for `remove` of
an attribute; the comments above an item, for `remove` of an item and for `move`, and the place it
goes, for `move`) that separates it from its neighbours,
and the edited document SHALL parse wherever the original did. An edit that would leave the text as
it is SHALL answer no edits.

#### Scenario: Applying the answer
- **WHEN** a client asks to set the value of `name` in `<Drink code="LAT" name="Latte" />` to the
  string `Flat white` and applies the answer's edits to the document
- **THEN** the document SHALL read `<Drink code="LAT" name="Flat white" />`, and nothing else in it
  SHALL change

#### Scenario: No change
- **WHEN** a client asks to set `size=large` to the case `large`
- **THEN** the answer SHALL hold no edits and no refusal

### Requirement: What cannot be done is refused, not failed
The service SHALL answer a request it cannot honor with no edits and a refusal that names why:
`unknownNode` when the document's source tree holds no node with the key; `notEditable` when the node
is not a target the edit takes, such as a literal inside an expression, a text run over several lines,
a value given to `remove`, anything but an element given to `addProperty`, or, given to `move` or
as an element to `remove`, anything but an element item of a sequence or of element content, an item
inside a condition's branch or a loop's body, an item of a list that holds text, or one of a list
written neither an item to a line nor on one line; and `invalidValue` when the value cannot be spelled, such as a number
that is not a numeric literal, a case name that is not an identifier, text that is empty or holds a
line break, a property `addProperty` cannot add, or a `move` position outside the list. A refusal
SHALL NOT be reported as a transport or service error.

#### Scenario: An operand
- **WHEN** a client sets the value of the literal `1` in `count={total + 1}`
- **THEN** the answer SHALL hold no edits and the refusal `notEditable`

#### Scenario: A key from another document
- **WHEN** a client sends a key that the document's source tree does not hold
- **THEN** the answer SHALL hold no edits and the refusal `unknownNode`

#### Scenario: Not a number
- **WHEN** a client sets a number value with the text `12px`
- **THEN** the answer SHALL hold no edits and the refusal `invalidValue`

#### Scenario: An item an `if` makes
- **WHEN** a client moves the `<B />` inside `if wide { <B /> }`
- **THEN** the answer SHALL hold no edits and the refusal `notEditable`

#### Scenario: Past the end
- **WHEN** a client moves an item of a list of three to `3`
- **THEN** the answer SHALL hold no edits and the refusal `invalidValue`
