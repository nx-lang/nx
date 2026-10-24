# source-edits Specification

## Purpose
How the language service turns an edit to one value of a source tree, made in a reading such as
Visual, into the smallest text edits to the document, and when it refuses one.
## Requirements
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

### Requirement: A value is set in the spelling its place takes
For `setValue`, the target SHALL be a literal or a bare union case that is a whole value: the value of
an attribute, an item of a braced sequence, or the value of a `let` without parameters. The service
SHALL replace only the target's text, keeping braces written around it, and SHALL spell the value by
its kind:

- a string as a double-quoted string literal, its quotes and backslashes escaped and every other
  character, a line break included, written as it is;
- a number as the text the request gives, when that text is an NX numeric literal, signed or not;
- a boolean as `true` or `false`;
- a case as its bare name, when the name is an identifier other than `true` and `false`; a case
  written qualified, as `{ Size.small }`, SHALL keep its qualifier.

A signed number SHALL be written only where the grammar takes one, an unbraced attribute value or
`let` value, and a case only at an unbraced place or as a qualified case; elsewhere, in a sequence
item or inside braces, where `-` reads as an operator and a bare name as a reference, the service
SHALL refuse the value as `invalidValue`.

The service SHALL NOT check the value against the place's type; a value of the wrong type SHALL be
written and then reported by the type checker as a hand edit would be.

#### Scenario: A string with a quote
- **WHEN** a client sets the string value of `title="Hi"` to `Say "hi"`
- **THEN** the attribute SHALL read `title="Say \"hi\""`

#### Scenario: A braced boolean keeps its braces
- **WHEN** a client sets the boolean value of `iced={true}` to `false`
- **THEN** the attribute SHALL read `iced={false}`

#### Scenario: A case
- **WHEN** a client sets the case value of `size=medium` to `large`
- **THEN** the attribute SHALL read `size=large`

#### Scenario: A number kept as typed
- **WHEN** a client sets the number value of `padding=8` to the text `12.5`
- **THEN** the attribute SHALL read `padding=12.5`

#### Scenario: An item of a sequence
- **WHEN** a client sets the number value of the second item of `let primes:int+ = { 2 3 5 7 }` to
  `4`
- **THEN** the declaration SHALL read `let primes:int+ = { 2 4 5 7 }`

#### Scenario: A negative item of a sequence
- **WHEN** a client sets the number value of the second item of `let primes:int+ = { 2 3 5 7 }` to
  `-4`
- **THEN** the answer SHALL hold no edits and the refusal `invalidValue`

### Requirement: A line of text content is set as written
For `setText`, the target SHALL be a `text` node written on one line, outside a typed or raw text
body. Element content has no escapes, so the service SHALL write the new text as it is, and SHALL
refuse it as `invalidValue` unless the document then reads it back as one run of exactly that text,
with no new parse error: text holding `<` or `{`, starting with a space, or starting with `if`, `for`
or `else`, or the start of one of them, as a word (`for you`, `fo`, `e`) is refused. The run's
trailing whitespace, which separates it from a braced value or element after it, SHALL be kept, and
the new text SHALL replace only the words before it.

#### Scenario: Text with an ampersand
- **WHEN** a client sets the text of the run in `<Label>Hello</Label>` to `Fish & chips > all`
- **THEN** the element SHALL read `<Label>Fish & chips > all</Label>`

#### Scenario: A run before a braced value
- **WHEN** a client sets the text of the first run in `<L>Hello {1} world</L>` to `Hi`
- **THEN** the element SHALL read `<L>Hi {1} world</L>`

#### Scenario: Text that would start a tag
- **WHEN** a client sets the text of that run to `a < b`
- **THEN** the answer SHALL hold no edits and the refusal `invalidValue`

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

### Requirement: What cannot be done is refused, not failed
The service SHALL answer a request it cannot honor with no edits and a refusal that names why:
`unknownNode` when the document's source tree holds no node with the key; `notEditable` when the node
is not a target the edit takes, such as a literal inside an expression, a text run over several lines,
a value given to `remove`, anything but an element given to `addProperty`, or, given to `move` or
as an element to `remove`, anything but an element item of a sequence or of element content, an item of a condition's branch or a loop's body, an item of a list that holds text, or one of a list
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

### Requirement: The source edit is exposed to TypeScript hosts
The wasm and Node language snapshots SHALL answer the source edit query, and
`@nx-lang/language-core`'s service, the HTTP handler and the HTTP client SHALL expose it as
`sourceEdit`, with the same answer the Rust language service gives for the same request. The packages
that expose it SHALL document it as unstable until a later change commits to its shape.

#### Scenario: Parity between Rust and TypeScript
- **WHEN** the wasm SDK's language-service tests ask for source edits over their corpus through the
  TypeScript service and through the Rust service
- **THEN** the two answers SHALL be equal after JSON round-trip

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

### Requirement: An item moves within its list
For `move`, the target SHALL be an element that is an item of a sequence or of element content, not an item of a condition's branch or a loop's body, in a list that holds no text, and `to` SHALL be the
position the item takes in its list once moved, counted as the list's keys count its items. The
service SHALL move the item in the list's layout. In a list written an item to a line, where no item
shares a line with another, it SHALL move the item's whole lines, with the comments and doc comments
directly above it, to stand before the item that will follow it, or after the last item, keeping
the item's indentation. The blank lines between the item and the item after it, or the item before
it when it is the last, SHALL go with it, so a list that spaces its items keeps its spacing; other
blank lines SHALL stay where they are. In a list written on one line, it SHALL move the item within
the line, with the space that separated it from its neighbour. An item MAY move
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
doc comments directly above it, and the blank lines between it and the item after it, or the item
before it when it is the last. When more than blank lines stand in that gap, it SHALL remove only
the item's lines and, of two blank lines that would then stand together, one. In a list written on
one line, it SHALL remove the item and the space that separates it from the item
before it, or after it when it is the first. The service SHALL NOT check that the list may lose the
item.

#### Scenario: A step and its comment
- **WHEN** the comment `// Ask this first` stands on the line directly above step `B` of the steps
  `A`, `B` and `C`, each written over its own lines, and a client removes `B`
- **THEN** the content SHALL read `A` and `C` as written, and the comment and every line of `B`
  SHALL be gone

#### Scenario: Items a blank line apart
- **WHEN** the steps `A`, `B` and `C` stand a blank line apart and a client removes `A`
- **THEN** the content SHALL read `B`, a blank line, then `C`, with no blank line before `B`

#### Scenario: On one line
- **WHEN** a client removes the first item of `choices={ <Choice value="a" /> <Choice value="b" /> }`
- **THEN** the attribute SHALL read `choices={ <Choice value="b" /> }`

