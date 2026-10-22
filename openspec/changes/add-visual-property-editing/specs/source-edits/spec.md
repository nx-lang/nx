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

### Requirement: A value is set in the spelling its place takes
For `setValue`, the target SHALL be a literal, a bare union case, or a reference to a named value
that is a whole value: the value of an attribute, an item of a braced sequence, or the value of a
`let` without parameters. The service SHALL replace only the target's text, keeping braces written
around it unless the new value needs other ones, and SHALL spell the value by its kind:

- a string as a double-quoted string literal, its quotes and backslashes escaped and every other
  character, a line break included, written as it is;
- a number as the text the request gives, when that text is an NX numeric literal, signed or not;
- a boolean as `true` or `false`;
- a case as its bare name, when the name is an identifier other than `true` and `false`; a case
  written qualified, as `{ Size.small }`, SHALL keep its qualifier;
- a reference as its path, identifiers joined by dots, in braces as an attribute's value
  (`color={theme.accent}`) and bare as a sequence item or a `let`'s value.

A signed number SHALL be written only where the grammar takes one, an unbraced attribute value or
`let` value, and a case only at an unbraced place or as a qualified case; elsewhere, in a sequence
item or inside braces, where `-` reads as an operator and a bare name as a reference, the service
SHALL refuse the value as `invalidValue`. A literal or case that replaces a braced reference SHALL be
written in the place's usual spelling, its braces dropped where the literal reads without them.

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

#### Scenario: A reference
- **WHEN** a client sets `color="#5a3a22"` to the reference `theme.accent`
- **THEN** the document SHALL read `color={theme.accent}`

#### Scenario: A literal over a reference
- **WHEN** a client sets `color={theme.accent}` to the string `#5a3a22`
- **THEN** the document SHALL read `color="#5a3a22"`

### Requirement: What cannot be done is refused, not failed
The service SHALL answer a request it cannot honor with no edits and a refusal that names why:
`unknownNode` when the document's source tree holds no node with the key; `notEditable` when the node
is not a target the edit takes, such as a literal inside an expression, a text run over several lines,
a value given to `remove`, or anything but an element given to `addProperty`; and `invalidValue` when
the value cannot be spelled, such as a number that is not a numeric literal, a case name that is not
an identifier, text that is empty or holds a line break, a reference path that is not identifiers
joined by dots, or a property `addProperty` cannot add. A refusal SHALL NOT be reported as a transport
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

### Requirement: The source edit is exposed to TypeScript hosts
The wasm and Node language snapshots SHALL answer the source edit and named values queries, and
`@nx-lang/language-core`'s service, the HTTP handler and the HTTP client SHALL expose them as
`sourceEdit` and `namedValues`, with the same answers the Rust language service gives for the same
requests. The packages that expose them SHALL document them as unstable until a later change commits
to their shape.

#### Scenario: Parity between Rust and TypeScript
- **WHEN** the wasm SDK's language-service tests ask for source edits and named values over their
  corpus through the TypeScript service and through the Rust service
- **THEN** the two answers SHALL be equal after JSON round-trip

## ADDED Requirements

### Requirement: A property is added in its element's layout
For `addProperty`, the target SHALL be an element, and the service SHALL write the attribute
`name=value`, the value spelled as `setValue` spells it at an attribute, after the element's last
attribute and in the element's layout: on the tag's line, after a space, when the element writes its
attributes on that line or has none; on a line of its own at the indentation of the element's other
attributes when it writes them a line each. The service SHALL refuse as `invalidValue` a property the
element already writes, one its declaration does not have, and a content property. It SHALL NOT check
the value against the property's type.

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

#### Scenario: A named value
- **WHEN** a client adds `color` with the reference `accent` to `<Drink code="LAT" name="Latte" />`
- **THEN** the document SHALL read `<Drink code="LAT" name="Latte" color={accent} />`

### Requirement: The language service answers the named values a property can take
The language service SHALL answer a `namedValues` query, which carries the documents, the URI of a
document, the key of an element in that document's source tree and the name of one of the element's
properties, written or not. Its answer SHALL list the named values in the document's scope whose type
the type checker accepts for the property: each `let` without parameters declared in the document or
visible through its imports, and each member of such a `let` whose value is a record, one level deep.
Each item SHALL carry its reference path as it would be written (`accent`, `theme.accent`), its
declared or inferred type spelled in NX, its doc comment when it has one, and its value's source text
when that is a literal. Items SHALL be ordered by the document's own values first, then each import's,
each in declaration order. A key that names no element, or a property the element does not have,
SHALL answer an empty list.

#### Scenario: A color token
- **WHEN** a document declares `type Color = string`, `let accent:Color = "#c8a27a"` and `let count = 3`,
  and a client asks for the named values of `color?:Color` on one of its `Drink` elements
- **THEN** the answer SHALL hold `accent`, typed `Color`, with the value `"#c8a27a"`, and SHALL NOT hold
  `count`

#### Scenario: A theme's members
- **WHEN** a document declares `let theme = <Theme accent="#c8a27a" muted="#888888" radius=4 />`, where
  `Theme`'s `accent` and `muted` are typed `Color`, and a client asks for the named values of a `Color`
  property
- **THEN** the answer SHALL hold `theme.accent` and `theme.muted`, and SHALL NOT hold `theme.radius`

#### Scenario: Not an element
- **WHEN** a client asks for the named values of a property with the key of a string literal
- **THEN** the answer SHALL be an empty list
