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
For `setValue`, the target SHALL be a whole value written as a literal, a bare or qualified union
case, a name, an element, or a color constructor call the color-tooling capability recognizes, such
as `{c.oklch(0.6, 0.2, 250)}`: the value of an attribute, an item of a braced sequence, or the value of
a `let` without parameters. A name is a reference, or a member access whose innermost object is a
reference or `env`, such as `blue`, `standardPalette.blue600` or `env.color.text.muted`, with no
call, index or operator in it; a qualified case such as `Size.small` is a case, not a name. The
service SHALL replace only the target's text, keeping braces written around it except as this
requirement says below, and SHALL spell the value by its kind:

- a string as a double-quoted string literal, its quotes and backslashes escaped and every other
  character, a line break included, written as it is;
- a number as the text the request gives, when that text is an NX numeric literal, signed or not;
- a boolean as `true` or `false`;
- a case as its bare name, when the name is an identifier other than `true` and `false`; a case
  written qualified, as `{ Size.small }`, SHALL keep its qualifier.
- a name, `{ kind: "name", name }`, as written: one or more dot-separated identifiers, none of them
  `true`, `false` or a keyword, with at least two parts when the first is `env`. It SHALL be written
  in braces at an unbraced place, an attribute value or a `let` value, as `padding={env.space.md}`
  and `let accent: Color = {blue}`, since a bare name there reads as a case, and bare inside braces
  or as a sequence item.

A signed number SHALL be written only where the grammar takes one, an unbraced attribute value or
`let` value, and a case only at an unbraced place or as a qualified case; elsewhere, in a sequence
item or inside braces, where `-` reads as an operator and a bare name as a reference, the service
SHALL refuse the value as `invalidValue`.

When the target is a name or an element that is the whole value of an attribute or a `let` and the
new value is not a name, the service SHALL write the value as at an unbraced place, dropping braces
written around a name: `padding={env.space.md}` set to the number `12` SHALL read `padding=12`. A
literal or case target SHALL keep its braces, as before. A name that breaks the rule above SHALL be
refused as `invalidValue`.

A `name` value SHALL replace any target this requirement admits, a hex color literal, a color
constructor call and an element included, as its whole text, its content included, braced as an
unbraced place needs. A `color` value SHALL keep its current meaning on a hex literal, a color
constructor call or a color element: the color-tooling capability's edit of the written channels,
which keeps the written form and its braces, and which takes precedence over the brace-dropping rule
above. On a name target a `color` value SHALL be written as the color literal it carries, as at an
unbraced place. A `color` value on any other literal or on a case, or any value other than `color`
or `name` on a hex literal or a constructor call, SHALL be refused as `invalidValue`, as today. A
`color` value on a constructor call or a color element the color tooling cannot edit, as one whose
channels are computed or out of range or one in a wider space, SHALL be refused as `notEditable`,
as today, and SHALL NOT be written over it.

The service SHALL NOT check the value against the place's type, nor that a name resolves; a value of
the wrong type, or a name the module cannot see, SHALL be written and then reported by the type
checker as a hand edit would be.

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

#### Scenario: A name at an attribute
- **WHEN** a client sets the value of `padding=8` to the name `env.space.md`
- **THEN** the attribute SHALL read `padding={env.space.md}`

#### Scenario: A name replaces a name
- **WHEN** a client sets the value of `fill={env.color.accent}` to the name `standardPalette.red600`
- **THEN** the attribute SHALL read `fill={standardPalette.red600}`

#### Scenario: A literal replaces a name
- **WHEN** a client sets the value of `padding={env.space.md}` to the number `12`
- **THEN** the attribute SHALL read `padding=12`

#### Scenario: A name replaces an element
- **WHEN** a client sets the value of `fill=<Paint.color color=#2563eb />` to the name `blue`
- **THEN** the attribute SHALL read `fill={blue}`

#### Scenario: A name in a sequence and a `let`
- **WHEN** a client sets the second item of `let gaps: Spacing+ = { 4 8 }` to the name `env.space.md`,
  the value of `let gap: Spacing = 4` to the name `env.space.sm`, and the value of
  `let accent: Color = #2563eb` to the name `blue`
- **THEN** they SHALL read `let gaps: Spacing+ = { 4 env.space.md }`,
  `let gap: Spacing = {env.space.sm}` and `let accent: Color = {blue}`

#### Scenario: A literal replaces a name at a `let`
- **WHEN** a client sets the value of `let gap: Spacing = {env.space.sm}` to the number `4`
- **THEN** it SHALL read `let gap: Spacing = 4`

#### Scenario: A hex color bound to a token
- **WHEN** a client sets the value of `background=#ffffff` to the name `env.color.surface`
- **THEN** the attribute SHALL read `background={env.color.surface}`

#### Scenario: A color over a token
- **WHEN** a client sets the value of `background={env.color.surface}` to the color `#fafafa`
- **THEN** the attribute SHALL read `background=#fafafa`

#### Scenario: A color constructor bound to a token
- **WHEN** a client sets the value of `fill={c.oklch(0.6, 0.2, 250)}` to the name `env.color.accent`
- **THEN** the attribute SHALL read `fill={env.color.accent}`

#### Scenario: A color on a color element keeps the element
- **WHEN** a client sets the value of `tint=<Color.srgb r=0.2 g=0.4 b=0.8 />` to the color `#336699`
- **THEN** the attribute SHALL stay a `<Color.srgb … />` element, with only its channel fields
  changed as the color-tooling capability edits them

#### Scenario: A color on a computed color element is refused
- **WHEN** a client sets the value of `tint=<Color.srgb r={x} g=0 b=0 />` to the color `#ff0000`
- **THEN** the answer SHALL hold no edits and the refusal `notEditable`, and the element SHALL stay

#### Scenario: A color on a number is refused
- **WHEN** a client sets the value of `padding=8` to the color `#ffffff`
- **THEN** the answer SHALL hold no edits and the refusal `invalidValue`

#### Scenario: Names that cannot be written
- **WHEN** a client sets a value to the name `env.space.2xl`, to `env`, or to `if.x`
- **THEN** each answer SHALL hold no edits and the refusal `invalidValue`

#### Scenario: An expression is still refused
- **WHEN** a client sets the value of `count={total + 1}` to the name `limit`
- **THEN** the answer SHALL hold no edits and the refusal `notEditable`

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

