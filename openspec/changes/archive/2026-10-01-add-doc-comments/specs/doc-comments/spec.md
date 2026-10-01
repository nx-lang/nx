## Purpose

Let NX authors document declarations and their members in the source, in a form the compiler
attaches to the declaration so editors and generated code can show it.

## ADDED Requirements

### Requirement: Doc comments are line comments marked with a third slash
A line comment that begins with exactly three slashes, `///`, SHALL be a doc comment. A line comment
that begins with four or more slashes SHALL be an ordinary comment, so that a banner such as
`//////////` documents nothing. Block comments, `/* */` and `<!-- -->`, SHALL never be doc comments.

A doc comment's text SHALL be the rest of its line after the marker, less one space immediately
after the marker if there is one. A doc comment SHALL NOT be recognized inside a string literal or
text content, where an ordinary comment is not recognized either.

#### Scenario: Three slashes begin a doc comment
- **WHEN** a file contains the line `/// Hint text.` directly above a declaration
- **THEN** the declaration's documentation SHALL be `Hint text.`

#### Scenario: Four slashes are an ordinary comment
- **WHEN** a file contains the line `//// Section` directly above a declaration
- **THEN** the declaration SHALL have no documentation
- **AND** no diagnostic SHALL be reported for the comment

#### Scenario: An empty doc line is a blank line in the text
- **WHEN** a declaration is preceded by the lines `/// Summary.`, `///`, and `/// Details.`
- **THEN** its documentation SHALL be `Summary.`, an empty line, and `Details.`, in that order

#### Scenario: A block comment is never documentation
- **WHEN** a declaration is directly preceded by `/** Summary. */`
- **THEN** the declaration SHALL have no documentation

### Requirement: A leading doc block documents the item on the next line
A leading doc comment is a `///` comment with nothing but whitespace before it on its line. Leading
doc comments on consecutive lines SHALL form one leading doc block, and the block's text SHALL be
their texts joined by line breaks, in source order.

A leading doc block SHALL document the documentable item whose first token is the first token on
the line immediately after the block's last line. When that token begins more than one documentable
item, the block SHALL document the outermost of them. The documentable items are:

- top-level declarations: type aliases, record types, union types, actions, values, functions,
  element-style functions, components, and external components
- record and action fields
- union cases, and the fields of a union case's payload
- component properties, `emits` entries, and `state` fields
- function and element-style function parameters

A type parameter, such as `T:type` in a record or `TItem:type` in a component, is not a
documentable item.

#### Scenario: A leading block documents a component
- **WHEN** a file contains `/// A text box.` on the line directly above `component <SearchBox`
- **THEN** the component `SearchBox` SHALL have the documentation `A text box.`

#### Scenario: A leading block documents a property
- **WHEN** a component's property list contains `/// Hint text.` on the line directly above
  `placeholder:string`
- **THEN** the property `placeholder` SHALL have the documentation `Hint text.`
- **AND** the component SHALL NOT have that documentation

#### Scenario: A leading block documents a union case
- **WHEN** a multi-line union declaration contains `/// No search has run yet.` on the line
  directly above `| idle`
- **THEN** the case `idle` SHALL have the documentation `No search has run yet.`

#### Scenario: The outermost item that starts on the line is documented
- **WHEN** an `emits` group contains `/// Fired on every keystroke.` on the line directly above
  `ValueChanged { value:string }`
- **THEN** the `emits` entry `ValueChanged` SHALL have that documentation
- **AND** its field `value` SHALL have no documentation

#### Scenario: A visibility modifier is part of the declaration
- **WHEN** `/// Shared theme.` is on the line directly above `export type Theme = string`
- **THEN** `Theme` SHALL have the documentation `Shared theme.`

### Requirement: A trailing doc comment documents the item on its own line
A trailing doc comment is a `///` comment with a token before it on its line. It SHALL document the
outermost documentable item that both starts and ends on that line.

A trailing doc comment SHALL continue onto the lines directly after it that hold only a `///`
comment starting at the same column as the trailing comment's `///`. Its text SHALL be the texts of
the trailing comment and its continuation lines, joined by line breaks in source order.

#### Scenario: A trailing doc comment documents a property
- **WHEN** a component's property list contains the line
  `placeholder:string      /// Hint text shown while the box is empty.`
- **THEN** the property `placeholder` SHALL have the documentation
  `Hint text shown while the box is empty.`

#### Scenario: A trailing doc comment documents a union case with a payload
- **WHEN** a multi-line union declaration contains the line
  `| failed { message:string }   /// The last search failed.`
- **THEN** the case `failed` SHALL have the documentation `The last search failed.`
- **AND** the field `message` SHALL have no documentation

#### Scenario: An aligned continuation extends a trailing doc comment
- **WHEN** a property list contains `placeholder:string   /// Hint text shown while`, then a line
  holding only `/// the box is empty.` whose `///` starts at the same column as the one above, then
  `tone:string`
- **THEN** the property `placeholder` SHALL have the documentation `Hint text shown while`, a line
  break, and `the box is empty.`
- **AND** `tone` SHALL have no documentation
- **AND** no diagnostic SHALL be reported

#### Scenario: A blank line separates a leading block from a trailing doc comment
- **WHEN** a property list contains `label:string   /// The label.`, then a blank line, then
  `/// Which way it faces.` directly above `facing:string`
- **THEN** `label` SHALL have the documentation `The label.`
- **AND** `facing` SHALL have the documentation `Which way it faces.`

#### Scenario: A trailing doc comment documents a one-line declaration
- **WHEN** a file contains the line `type Size = int   /// Size in device pixels.`
- **THEN** `Size` SHALL have the documentation `Size in device pixels.`

### Requirement: Doc comment attachment is checked
Each of the following SHALL be reported as a static error at the doc comment, and the doc comment
SHALL document nothing:

- a leading doc block whose next line does not start with a documentable item, including a block
  followed by a blank line, by an ordinary comment, or by the end of the file
- a trailing doc comment whose line has no documentable item that both starts and ends on it
- a trailing doc comment whose line has more than one outermost documentable item that starts and
  ends on it
- a `///` line directly after a trailing doc comment or its continuation, holding only that
  comment, whose `///` does not start at the trailing comment's column; it and every `///` line
  directly after it document nothing
- a leading doc block and a trailing doc comment that both document the same item; the error SHALL
  be reported at the trailing doc comment

A diagnostic for a misaligned continuation SHALL say that a trailing doc comment continues only on
lines aligned with it, and that documentation for the next item needs a blank line before it.

#### Scenario: A doc block separated from its item by a blank line is an error
- **WHEN** a file contains `/// Theme.`, then a blank line, then `type Theme = string`
- **THEN** a static error SHALL be reported at the doc comment
- **AND** `Theme` SHALL have no documentation

#### Scenario: A doc comment at the end of a file is an error
- **WHEN** the last line of a file is `/// Nothing follows.`
- **THEN** a static error SHALL be reported at the doc comment

#### Scenario: A doc comment above an expression is an error
- **WHEN** a component body contains `/// The input.` on the line directly above
  `<TextInput value={query} />`
- **THEN** a static error SHALL be reported at the doc comment

#### Scenario: A doc comment on a type parameter is an error
- **WHEN** a record type contains `/// The element type.` on the line directly above `T:type`
- **THEN** a static error SHALL be reported at the doc comment

#### Scenario: A trailing doc comment on a closing brace is an error
- **WHEN** the line `}   /// End of the record.` closes a record type begun on an earlier line
- **THEN** a static error SHALL be reported at the doc comment

#### Scenario: A trailing doc comment on a line with two properties is an error
- **WHEN** a component's property list contains the line `tone:string density:string   /// Style.`
- **THEN** a static error SHALL be reported at the doc comment
- **AND** neither `tone` nor `density` SHALL have documentation

#### Scenario: A misaligned continuation is an error
- **WHEN** a property list contains `label:string   /// The label.`, then `/// Which way it faces.`
  indented like the properties, directly above `facing:string`
- **THEN** a static error SHALL be reported at `/// Which way it faces.`
- **AND** the diagnostic SHALL say that a continuation must align with the trailing doc comment and
  that documentation for the next item needs a blank line before it
- **AND** `label` SHALL have the documentation `The label.`
- **AND** `facing` SHALL have no documentation

#### Scenario: An item documented both ways is an error
- **WHEN** `/// Hint.` is on the line directly above `placeholder:string   /// Placeholder.`
- **THEN** a static error SHALL be reported at `/// Placeholder.`

### Requirement: Documentation is Markdown with a summary paragraph
Documentation text SHALL be interpreted as CommonMark. NX SHALL NOT define a tag vocabulary such as
`@param` or `@returns`; a member is documented by a doc comment on the member.

The summary of a declaration's documentation SHALL be its first Markdown block. For ordinary text
that is its first paragraph, the text up to the first blank line; a block that can interrupt a
paragraph, such as a list, ends the summary without one. Tools that show a short form of
documentation SHALL show the summary.

#### Scenario: The first paragraph is the summary
- **WHEN** a declaration's documentation is `Summary line.`, an empty line, and `More detail.`
- **THEN** its summary SHALL be `Summary line.`

#### Scenario: A list ends the summary
- **WHEN** a declaration's documentation is `Modes:` followed directly by the lines `- fill` and
  `- cover`
- **THEN** its summary SHALL be `Modes:`

### Requirement: Doc links name declarations
A shortcut reference link in documentation — `[Name]` or `` [`Name`] `` not followed by `(` or `[`,
with no matching link reference definition in the text — whose label is a name or a
dot-separated path of names SHALL be a doc link, where a name is an identifier that may also hold
`-` after its first character, as a markup-style member name such as `aria-label` does. Brackets inside a code span or code block SHALL
NOT form a doc link, and a link with an explicit destination, `[text](url)`, SHALL remain an
ordinary Markdown link.

A doc link SHALL resolve as follows: its first identifier is looked up first among the members of
the declaration the documentation belongs to or belongs within, and then as a name visible at the
top level of the module, including imported names and their aliases. Each further identifier SHALL
name a member of what the previous part resolved to: a field of a record or action, a case of a
union, a field of a case's payload, or a property of a component. A doc link that does not resolve
SHALL be reported as a warning at the link, and SHALL NOT prevent compilation.

#### Scenario: A link to a top-level declaration resolves
- **WHEN** a declaration's documentation contains `See [renderNotice].` and the module declares
  `renderNotice`
- **THEN** the link SHALL resolve to that declaration
- **AND** no diagnostic SHALL be reported

#### Scenario: A link to a union case resolves
- **WHEN** documentation contains `[LoadState.idle]` and `LoadState` is a union with a case `idle`
- **THEN** the link SHALL resolve to the case `idle`

#### Scenario: A property's doc links to a sibling property
- **WHEN** the documentation of a component property contains `Ignored when [value] is set.` and
  the same component declares a property `value`
- **THEN** the link SHALL resolve to that property

#### Scenario: A link names a markup-style member
- **WHEN** a component declares `aria-label:string` and documentation within it contains
  `[aria-label]`
- **THEN** the link SHALL resolve to that property

#### Scenario: An unresolved doc link is a warning
- **WHEN** documentation contains `[Missing]` and no visible name `Missing` exists
- **THEN** a warning SHALL be reported at `[Missing]`
- **AND** the module SHALL still compile

#### Scenario: Brackets in code are not doc links
- **WHEN** documentation contains `` `items[index]` ``
- **THEN** no doc link SHALL be formed and no diagnostic SHALL be reported

#### Scenario: An explicit link is ordinary Markdown
- **WHEN** documentation contains `[the spec](https://nxlang.org/spec)`
- **THEN** no doc link SHALL be formed and no diagnostic SHALL be reported

### Requirement: Documentation does not change what a program means
Adding, removing, or editing a doc comment SHALL NOT change a program's types, values, rendered
output, or generated NX IR executable content. The only effects of a doc comment SHALL be the
documentation it attaches, the diagnostics this capability defines, and what tools show.

#### Scenario: Documenting a declaration does not change its evaluation
- **WHEN** a program is evaluated before and after a doc comment is added to one of its declarations
- **THEN** both evaluations SHALL produce the same result
