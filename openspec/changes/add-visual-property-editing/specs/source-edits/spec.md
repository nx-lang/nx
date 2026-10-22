## MODIFIED Requirements

### Requirement: The language service answers a source edit for a node
The language service SHALL answer a `sourceEdit` query, which carries the documents, the URI of the
document to edit, the key of a node in that document's source tree, and one edit: `setValue` with a
typed value, `setText` with a line of text, `remove`, or `addProperty` with a property's name and a
typed value. It SHALL resolve the key against the document as the request carries it, by the keys its
source tree gives. Its answer SHALL be the text edits that make the change, each a range in the
protocol's form and its new text. Every range SHALL be in the document as sent, and no two SHALL
overlap, so a client applies them together as one change. The edits SHALL touch nothing outside the
target and the whitespace (and a trailing line comment, for `remove`) that separates it from its
neighbours, and the edited document SHALL parse wherever the original did. An edit that would leave
the text as it is SHALL answer no edits.

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
a value given to `remove`, or anything but an element given to `addProperty`; and `invalidValue` when
the value cannot be spelled, such as a number that is not a numeric literal, a case name that is not
an identifier, text that is empty or holds a line break, or a property `addProperty` cannot add. A refusal SHALL NOT be reported as a transport
or service error.

#### Scenario: An operand
- **WHEN** a client sets the value of the literal `1` in `count={total + 1}`
- **THEN** the answer SHALL hold no edits and the refusal `notEditable`

#### Scenario: A key from another document
- **WHEN** a client sends a key that the document's source tree does not hold
- **THEN** the answer SHALL hold no edits and the refusal `unknownNode`

#### Scenario: Not a number
- **WHEN** a client sets a number value with the text `12px`
- **THEN** the answer SHALL hold no edits and the refusal `invalidValue`

## ADDED Requirements

### Requirement: A property is added in its element's layout
For `addProperty`, the target SHALL be an element, and the service SHALL write the attribute
`name=value`, the value spelled as `setValue` spells it at an attribute, after the element's last
attribute and in the element's layout: on the tag's line, after a space, when the element writes its
attributes on that line or has none; on a line of its own at the indentation of the element's other
attributes when it writes them a line each. An element of a union's case SHALL take the union's
shared properties and its case's. The service SHALL refuse as `invalidValue` a property the element
already writes, directly or in any branch of a property-list condition or match, one its declaration
does not have, and a content property. It SHALL NOT check the value against the property's type.

#### Scenario: On one line
- **WHEN** a client adds `iced` with the boolean `true` to `<Drink code="LAT" name="Latte" />`
- **THEN** the document SHALL read `<Drink code="LAT" name="Latte" iced=true />`

#### Scenario: A property per line
- **WHEN** a client adds `price` with the number `5.25` to an element written with `code="CBR"` and
  `name="Cold brew"` each on its own line, indented four spaces
- **THEN** the document SHALL hold `price=5.25` on a new line after `name="Cold brew"`, indented four
  spaces, and nothing else SHALL change

#### Scenario: A property already written
- **WHEN** a client adds `name` to `<Drink code="LAT" name="Latte" />`
- **THEN** the answer SHALL hold no edits and the refusal `invalidValue`

#### Scenario: A property written under a condition
- **WHEN** a client adds `iced` to `<Drink code="A" if flag { iced=true } />`
- **THEN** the answer SHALL hold no edits and the refusal `invalidValue`

#### Scenario: A union case's element
- **WHEN** a client adds `pumps` with the number `2` to `<Extra.syrup flavor="mocha" />`, where the
  case `syrup` declares `flavor` and `pumps`
- **THEN** the document SHALL read `<Extra.syrup flavor="mocha" pumps=2 />`
