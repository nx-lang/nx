## ADDED Requirements

### Requirement: Generated declarations carry NX documentation
A generated type or member that `typegen` derives from a documented NX declaration or member SHALL
carry that documentation. This covers record and action types and their properties, union roots,
union case types and their properties, the members of an enum generated for a constant union, type
aliases, external component state contracts and their properties, and the properties of update
companions derived from documented fields or state. A generated declaration with no NX counterpart,
or whose counterpart has no documentation, SHALL carry the documentation it carries today, if any.

In C#, the documentation SHALL be XML documentation comments, with the Markdown converted to the
XML documentation elements that IDE tooltips render. The summary paragraph SHALL be the
`<summary>`, and any remaining blocks SHALL be the `<remarks>`, each paragraph wrapped in `<para>`
when there is more than one block. Within them:

- strong emphasis SHALL become `<b>`, and emphasis SHALL become `<i>`
- a code span SHALL become `<c>`, and a fenced or indented code block SHALL become `<code>`
- a link with a destination, `[text](url)`, SHALL become `<a href="url">text</a>`
- a doc link SHALL become its label in `<c>`
- a bulleted list SHALL become `<list type="bullet">`, and a numbered list SHALL become
  `<list type="number">`, with each item as `<item><description>…</description></item>`
- a heading SHALL become a `<para>` holding the heading text in `<b>`
- any other Markdown, such as a table, an image, or raw HTML, SHALL be kept as its written text

`&`, `<`, and `>` in the text SHALL be escaped, so no Markdown content can produce an XML element
the mapping above does not.

In TypeScript, the documentation SHALL be a `/** */` comment holding the Markdown text, one ` * `
prefixed line per source line, with each doc link replaced by its label in a code span. The
Markdown SHALL otherwise be kept as written, since TypeScript tools render it. A `*/` in the text
SHALL be written so that it does not end the comment, and an `@` that begins a word outside a code
span or code block SHALL be escaped as `\@`, so that no documentation text is read as a JSDoc tag.
A constant case of a union generated as a string-literal union has no declaration of its own in
TypeScript, so its documentation SHALL NOT be emitted there.

#### Scenario: A documented C# record property carries a summary
- **WHEN** an exported record field is documented `Hint text.`
- **THEN** the generated C# property SHALL be preceded by `/// <summary>Hint text.</summary>`

#### Scenario: C# documentation splits summary and remarks
- **WHEN** an exported record's documentation is `A contact.`, an empty line, and `Shown in lists.`
- **THEN** the generated C# type's `<summary>` SHALL hold `A contact.`
- **AND** its `<remarks>` SHALL hold `Shown in lists.`

#### Scenario: C# documentation escapes XML
- **WHEN** an exported declaration's documentation contains `a < b & c`
- **THEN** the generated C# documentation SHALL contain `a &lt; b &amp; c`

#### Scenario: C# documentation converts Markdown emphasis and links
- **WHEN** an exported declaration's documentation is ``Use **only** with `Grid`; see [the guide](https://nxlang.org/guide).``
- **THEN** the generated C# `<summary>` SHALL contain `<b>only</b>`, `<c>Grid</c>`, and
  `<a href="https://nxlang.org/guide">the guide</a>`
- **AND** it SHALL NOT contain `**`

#### Scenario: C# documentation converts a bulleted list
- **WHEN** an exported declaration's documentation is `Modes.`, an empty line, `- fill`, and
  `- cover`
- **THEN** the generated C# `<remarks>` SHALL contain a `<list type="bullet">` with the items `fill`
  and `cover`

#### Scenario: C# documentation keeps unsupported Markdown as text
- **WHEN** an exported declaration's documentation contains the raw HTML `<br>`
- **THEN** the generated C# documentation SHALL contain `&lt;br&gt;`

#### Scenario: A documented constant case documents its enum member
- **WHEN** an exported constant union's case `dark` is documented `Light text on dark.`
- **THEN** the generated C# enum member `Dark` SHALL carry that summary
- **AND** the generated TypeScript string-literal union SHALL NOT carry it, because TypeScript tools
  read no documentation from a member of a literal union

#### Scenario: A documented TypeScript property carries a JSDoc comment
- **WHEN** an exported record field is documented `Hint text.`
- **THEN** the generated TypeScript property SHALL be preceded by a `/** */` comment containing
  `Hint text.`

#### Scenario: TypeScript documentation keeps Markdown
- **WHEN** an exported declaration's documentation contains `Use **only** with [Grid].`
- **THEN** the generated TypeScript comment SHALL contain ``Use **only** with `Grid`.``

#### Scenario: An at sign in documentation is not a JSDoc tag
- **WHEN** an exported declaration's documentation contains a line beginning `@nx/prelude provides`
- **THEN** the generated TypeScript comment SHALL contain `\@nx/prelude provides`
- **AND** an `@` inside a code span SHALL be written unescaped

#### Scenario: A comment terminator in documentation does not end the TypeScript comment
- **WHEN** an exported declaration's documentation contains `*/`
- **THEN** the generated TypeScript SHALL still compile, with the documentation comment ending where
  the generator ends it

#### Scenario: An undocumented declaration generates as before
- **WHEN** an exported declaration and its members carry no documentation
- **THEN** the generated C# and TypeScript for it SHALL be unchanged by this capability
