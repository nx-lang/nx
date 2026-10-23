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

## MODIFIED Requirements

### Requirement: The language service answers a source edit for a node
The language service SHALL answer a `sourceEdit` query, which carries the documents, the URI of the
document to edit, the key of a node in that document's source tree, and one edit: `setValue` with a
typed value, `setText` with a line of text, `remove`, `addProperty` with a property's name and a
typed value, or `move` with a position. It SHALL resolve the key against the document as the request
carries it, by the keys its source tree gives. Its answer SHALL be the text edits that make the
change, each a range in the protocol's form and its new text. Every range SHALL be in the document as
sent, and no two SHALL overlap, so a client applies them together as one change. The edits SHALL
touch nothing outside the target and the whitespace (and a trailing line comment, for `remove`; the
comments above the item and the place it goes, for `move`) that separates it from its neighbours,
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
a value given to `remove`, anything but an element given to `addProperty`, or, given to `move`,
anything but an element item of a sequence or of element content, an item inside a condition's
branch or a loop's body, an item of a list that holds text, or one of a list written neither an item
to a line nor on one line; and `invalidValue` when the value cannot be spelled, such as a number
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
