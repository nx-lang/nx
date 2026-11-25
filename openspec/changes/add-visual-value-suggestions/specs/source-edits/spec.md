## MODIFIED Requirements

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
unbraced place. A `color` value on any other literal or on a case, a constructor call it cannot
edit, or any value other than `color` or `name` on a hex literal or a constructor call, SHALL be
refused as `invalidValue`, as today.

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

#### Scenario: Names that cannot be written
- **WHEN** a client sets a value to the name `env.space.2xl`, to `env`, or to `if.x`
- **THEN** each answer SHALL hold no edits and the refusal `invalidValue`

#### Scenario: An expression is still refused
- **WHEN** a client sets the value of `count={total + 1}` to the name `limit`
- **THEN** the answer SHALL hold no edits and the refusal `notEditable`

