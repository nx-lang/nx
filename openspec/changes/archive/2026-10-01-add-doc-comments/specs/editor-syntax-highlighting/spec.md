## ADDED Requirements

### Requirement: Doc comments are scoped as documentation comments
The grammar SHALL scope a `///` doc comment, whether it stands on its own line or trails code, as
`comment.line.documentation.nx`, from `///` to the end of the line, with the `///` itself also
scoped `punctuation.definition.comment.nx`. A line
comment beginning with four or more slashes SHALL remain scoped `comment.line.double-slash.nx`. A
doc comment SHALL be recognized everywhere an ordinary line comment is, and nowhere else, and it
SHALL NOT terminate the enclosing declaration regardless of the characters it contains.

#### Scenario: A leading doc comment is a documentation comment
- **WHEN** a file contains the line `/// A text box.`
- **THEN** all of `/// A text box.` SHALL be scoped `comment.line.documentation.nx`

#### Scenario: A trailing doc comment on a property is a documentation comment
- **WHEN** a declaration signature contains `placeholder:string   /// Hint text; shown when empty.`
- **THEN** `/// Hint text; shown when empty.` SHALL be scoped `comment.line.documentation.nx`
- **AND** a property line following it SHALL still be scoped as a declaration property

#### Scenario: Four slashes stay an ordinary comment
- **WHEN** a file contains the line `//// Section`
- **THEN** it SHALL be scoped `comment.line.double-slash.nx`

#### Scenario: Three slashes in text content are not a comment
- **WHEN** text content contains `a /// b`
- **THEN** `///` SHALL NOT be scoped as a comment

### Requirement: Doc comment Markdown is scoped
Within a doc comment, and only there, the grammar SHALL scope inline Markdown one line at a time,
leaving every marker character in the text:

- a doc link, `[Name]`, `[Type.member]`, or either in backticks, not followed by `(` or `[`, with
  its label scoped `markup.underline.link.reference.nx`
- a link with a destination, `[text](url)`, with its text scoped `markup.underline.link.text.nx` and
  its destination `markup.underline.link.nx`
- a code span as `markup.inline.raw.nx`, with nothing inside it scoped as a link or emphasis
- strong emphasis, `**text**` or `__text__`, as `markup.bold.nx`, and emphasis, `*text*` or
  `_text_`, as `markup.italic.nx`

A bracketed phrase that is not an identifier path, an asterisk between spaces, and an underscore
inside a word SHALL NOT be scoped as a link or emphasis. None of these scopes SHALL be a `string` scope: editors class text by
its scope, and the text of a doc link must stay comment text, where an editor suggests doc link
names as they are typed, rather than string text, where it suggests nothing.

#### Scenario: A doc link is scoped as a link
- **WHEN** a file contains the line `/// See [renderNotice] and [LoadState.idle].`
- **THEN** `renderNotice` and `LoadState.idle` SHALL be scoped `markup.underline.link.reference.nx`

#### Scenario: Brackets in a code span are not a link
- **WHEN** a file contains the line ``/// Read `items[index]` first.``
- **THEN** `` `items[index]` `` SHALL be scoped `markup.inline.raw.nx`
- **AND** `index` SHALL NOT be scoped `markup.underline.link.reference.nx`

#### Scenario: Emphasis keeps its markers
- **WHEN** a file contains the line `/// Use **only** once.`
- **THEN** `**only**`, asterisks included, SHALL be scoped `markup.bold.nx`

#### Scenario: Markdown in an ordinary comment is not scoped
- **WHEN** a file contains the line `// Use **only** [Grid].`
- **THEN** neither `only` nor `Grid` SHALL be scoped as emphasis or a link
