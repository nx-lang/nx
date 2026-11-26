## ADDED Requirements

### Requirement: A property offers the named values of its type
When the host has set `editable` and the tree carries suggestions, a property in an inspector SHALL
offer its named values: the tree's suggestions whose key is the property's `suggestionKey`, leaving
out each one whose `scope` is not the top-level declaration the property is written in and each one
marked `self` with that declaration. A property with none SHALL offer nothing and read as it does
without this requirement. A `let` value whose declaration carries a `suggestionKey` SHALL offer them
the same way.

A property that offers named values SHALL show a quiet named-values button after its value and
before its `⋯`, written or unset, while the property is under the pointer, selected or open in an
editor, drawn with the host's `named-values` icon or the viewer's own when the host draws none. The
button SHALL NOT be a Tab stop. A click on it, and Ctrl+Space or Alt+Escape (Option+Esc on a Mac,
where the system may take Ctrl+Space) with the property selected or its field open, SHALL open the
named-values popover; a click on it while the popover is open SHALL close
it.

The popover SHALL open over the reading, placed by the value, and move nothing. It SHALL hold a
search field over a list of the named values by group, each group under its label, in the tree's
order. Each row SHALL read the name, without its leading `env.` under the Environment group and with
its import alias quieter elsewhere, and, in a second column, its preview: a swatch with a faint
border, and the hex when it has one, for a color, and the text of a number, string, boolean or case. When the property has a default, a `Default` row and
the default SHALL lead the list, set off by a rule, as the choices list's does. The row whose name is
the property's current value SHALL be marked, active, and scrolled to the middle of the list when the
popover opens. Typing SHALL keep the rows whose name or doc holds every word typed, ignoring case and taking `.` as a space, and the groups that still hold rows. Up and Down
SHALL move through the rows, from the marked row when none is active yet, Enter SHALL choose the
active row, and Escape or a click elsewhere SHALL close the popover and keep the selection. The list
SHALL scroll inside the popover, its group labels staying at its top as it scrolls, and never scroll
the reading. A row's hover card, and a line above the foot for the active row, SHALL give its type, its full
preview text, and its doc. The popover's foot SHALL read, quieter, `Values at their defaults, before
any provide or env. override`. The search field SHALL be a combobox over a listbox, with the active
row announced.

#### Scenario: Spacing for a padding
- **WHEN** a Card typed with `padding: Spacing = 16` writes `padding=8` in a document that imports
  `@nx/design`, and the reader opens the named-values popover of `Padding`
- **THEN** the popover SHALL read `Default 16`, then under `Environment` `space.none` through
  `space.xxxl`, each with its number, and no `radius` row

#### Scenario: Colors with their swatches
- **WHEN** a document imports `@nx/graphics` and `@nx/design`, a Circle writes
  `fill=<Paint.color color=#2563eb/>`, whose case field `color` is typed `Color`, and the reader
  opens the popover of that `Color` in Circle's inspector
- **THEN** the rows under `Environment` SHALL include `color.accent` with a swatch, and the rows
  under `@nx/design` SHALL include `standardPalette.blue600` with a swatch

#### Scenario: Filtering
- **WHEN** the reader types `text mu` in the popover of a `Color`
- **THEN** the list SHALL hold `color.text.muted` and no row whose name and doc lack either word

#### Scenario: Searching descriptions
- **WHEN** the reader types `ring` in the popover of a `Color`
- **THEN** the list SHALL hold `color.focus`, whose doc reads "The focus ring."

#### Scenario: A property without named values
- **WHEN** a property is typed `string`
- **THEN** it SHALL show no named-values button, and Ctrl+Space SHALL open nothing

#### Scenario: A component's own property is offered inside it
- **WHEN** `component <Badge tint: Color /> = { <Dot fill={env.color.accent} /> }` and the reader
  opens the popover of Dot's `Fill`
- **THEN** the list SHALL hold `tint` under `Badge`, and a Dot outside `Badge` SHALL NOT offer `tint`

#### Scenario: Not editable
- **WHEN** the host has not set `editable`
- **THEN** no named-values button SHALL show

### Requirement: Choosing a named value writes its name
Choosing a named value SHALL fire `nx-edit` setting the property's value to the name, as a `name`
value of the source-edits capability, when the property is written, and adding the property with
that value when it is unset; choosing the row of the current value SHALL fire nothing. Choosing the
`Default` row SHALL leave the attribute out, as the row's Reset to default does, or, on an unset
property, add nothing. The field SHALL then read the name as a token, muted as stale, until the host
sets a newer tree.

A click on a token value SHALL open the named-values popover of its property when the property
offers named values, and SHALL otherwise select the property as a click on a value it cannot edit
does. Replace with value in a token's menu SHALL open the property's own editor (its number or text
field, its color's parts or popover, or its case popover) holding the token's preview, as if the
value were written so; committing it SHALL fire `nx-edit` setting that value over the token, and
cancelling SHALL leave the token. A token without a preview, or with one the property's editor does
not take, SHALL NOT offer Replace with value; for a color, the color-tooling capability decides,
offering it only for a color inside sRGB.

#### Scenario: Binding a padding to a token
- **WHEN** the reader opens the popover of `padding=8` and chooses `env.space.md`
- **THEN** the element SHALL fire `nx-edit` setting the name `env.space.md`, and the field SHALL
  read the token `{env.space.md}`

#### Scenario: Adding an unset property as a token
- **WHEN** a Card does not write `background`, typed `Color`, and the reader shows its unset
  properties, opens the popover of `Background` and chooses `env.color.surface`
- **THEN** the element SHALL fire `nx-edit` adding `background` with the name `env.color.surface`

#### Scenario: Swapping one token for another
- **WHEN** a Card writes `background={env.color.surface}` and the reader clicks the token
- **THEN** the popover SHALL open with `env.color.surface` marked, and choosing
  `env.color.surfaceMuted` SHALL fire `nx-edit` setting that name

#### Scenario: A wide color token keeps its name
- **WHEN** a Card writes `fill={standardPalette.green500}`, whose preview is outside sRGB, and the reader opens the token's menu
- **THEN** the menu SHALL offer Go to where it applies and Show in Source, and no Replace with value

#### Scenario: Back to a plain number
- **WHEN** a Card writes `padding={env.space.md}`, whose preview is `12`, and the reader chooses
  Replace with value in the token's menu, types `14` and presses Enter
- **THEN** the element SHALL fire `nx-edit` setting the number `14`

## MODIFIED Requirements

### Requirement: References and nested values read by what they name
A value source writes in braces SHALL keep them, in the quiet color of an `if`'s braces and with the
spaces inside them dropped, so that it reads as computed; a literal SHALL read without them. A
property whose value is a name, as the source-edits capability defines it (a reference to a
top-level `let`, a parameter or a state field, a member chain on one, or an `env.` read such as
`{env.color.accent}`), SHALL read as a token with that name in its braces; when
the `let` is written as a literal color, or as an element whose visualizer formats one property as a
color, or when the tree's suggestion with that name has a color preview, the token SHALL show that
color's swatch before its braces. The token's menu SHALL offer Go to and the name, as Go to `blue`,
when the name starts from a declaration of this document, selecting the value it names; Replace
with value, as "Choosing a named value writes its name" says, when the host has set `editable`; and
Show in Source. A property whose value is an element SHALL read as a field with the element's icon,
kind and the value it is named by, and SHALL unfold into a nested group of that element's fields, edited the same
way. A short element value SHALL instead read its set properties on its field's line, each its label
and its value, edited and scrubbed there as an inspector's fields are, wrapping under the first where
the line runs out: one that holds only properties, one to four of them, each one simple value on one
line (a number, a boolean, a color, a bare case, a reference, or a string of at most 24 characters).
Its visualizer's `inline` SHALL decide instead when it is set: `true` reads any such value of one-line
properties on its line, `false` never does. A sole property, or the title, SHALL read without its
label. On its line its kind SHALL read only what the property's type leaves open: nothing for the
record the type names, and the case alone for a case of the union it names. Its chevron SHALL still
unfold it into the nested group, its kind in full, where the properties it does not set are. An action value, a component's `State.Update` included, SHALL show the action icon. Any other expression SHALL read as written, code-styled, and open in Source.

#### Scenario: A shared paint
- **WHEN** Star writes `fill={blue}` and `let blue: Paint = <Paint.color color=#2563eb/>`
- **THEN** Star's `Fill` field SHALL read a `#2563eb` swatch and the token `{blue}`, and its `⋯`
  menu's Go to `blue` SHALL select `blue`'s value

#### Scenario: An expression keeps its braces
- **WHEN** a Box writes `w={1 + 2}` and `label="plain"`
- **THEN** its `W` field SHALL read `{1 + 2}`, its braces quiet and its tokens in their code colors,
  and its `Label` field SHALL read `plain`

#### Scenario: A short element value on its field's line
- **WHEN** Drawing writes `viewBox=<Rect x=0 y=0 width=400 height=240/>` and
  `background=<Paint.color color=#ffffff/>`
- **THEN** View box SHALL read `X 0  Y 0  Width 400  Height 240`, with no `Rect`, and Background
  SHALL read `color` and the color, and dragging the `Width` label SHALL scrub `width` in that Rect

#### Scenario: A nested element value
- **WHEN** Drawing writes `viewBox=<Rect x=0 y=0 width=400 height=240/>` and the reader unfolds it
- **THEN** a nested group SHALL show the Rect's fields, and changing its width SHALL edit `width` in
  that Rect

#### Scenario: An environment token
- **WHEN** a Card writes `background={env.color.surface}`, and the tree carries the suggestion
  `env.color.surface` with a color preview
- **THEN** Card's `Background` field SHALL read that color's swatch and the token
  `{env.color.surface}`
