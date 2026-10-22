## ADDED Requirements

### Requirement: The language service answers a source edit for a node
The language service SHALL answer a `sourceEdit` query, which carries the documents, the URI of the
document to edit, the key of a node in that document's source tree, and one edit: `setValue` with a
typed value, `setText` with a line of text, or `remove`. It SHALL resolve the key against the
document as the request carries it, by the keys its source tree gives. Its answer SHALL be the text
edits that make the change, each a range in the protocol's form and its new text. Every range SHALL be
in the document as sent, and no two SHALL overlap, so a client applies them together as one change.
The edits SHALL touch nothing outside the target and the whitespace (and a trailing line comment, for
`remove`) that separates it from its neighbours, and the edited document SHALL parse wherever the
original did. An edit that would leave the text as it is SHALL
answer no edits.

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
For `remove`, the target SHALL be an attribute. The service SHALL remove the attribute and the
whitespace that separates it from the attribute or tag before it, and a `//` comment that follows it
on its line; when the attribute stands alone on its line, the service SHALL remove that whole line. A
comment on the line above SHALL stay. The service SHALL NOT check that the property may
be left out.

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
is not a target the edit takes, such as a literal inside an expression, a text run over several lines
or a value given to `remove`; and `invalidValue` when the value cannot be spelled, such as a number
that is not a numeric literal, a case name that is not an identifier, or text that is empty or holds a
line break. A refusal SHALL NOT be reported as a transport or service error.

#### Scenario: An operand
- **WHEN** a client sets the value of the literal `1` in `count={total + 1}`
- **THEN** the answer SHALL hold no edits and the refusal `notEditable`

#### Scenario: A key from another document
- **WHEN** a client sends a key that the document's source tree does not hold
- **THEN** the answer SHALL hold no edits and the refusal `unknownNode`

#### Scenario: Not a number
- **WHEN** a client sets a number value with the text `12px`
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
