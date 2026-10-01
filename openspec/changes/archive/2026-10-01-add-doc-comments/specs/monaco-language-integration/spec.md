## ADDED Requirements

### Requirement: Doc comment Markdown is styled as documentation
Every theme the integration loads SHALL style the Markdown in NX doc comments the way documentation
reads in other editors rather than the way a Markdown file does: strong emphasis and emphasis SHALL
keep the theme's comment color and change only the font, a code span SHALL take the theme's
code-span color, and a doc link SHALL take that color with an underline. The theme's code-span
color is its color for code spans, or failing that its color for one language's Markdown code
spans, such as `markup.inline.raw.markdown`. In a theme with no code-span color, a doc link SHALL
keep the comment color, underlined. The integration SHALL export the same transformation for a host
that defines its themes itself.

#### Scenario: Emphasis keeps the comment color
- **WHEN** a theme with a comment color is loaded and a doc comment contains `**b**` and `*i*`
- **THEN** `**b**` SHALL be shown bold and `*i*` italic, both in the comment color

#### Scenario: A theme that colors only Markdown's code spans
- **WHEN** a theme whose only code-span rule is for `markup.inline.raw.markdown` is loaded and a doc
  comment contains `[Link]`
- **THEN** `Link` SHALL be shown underlined in that rule's color

#### Scenario: A doc link takes the code color
- **WHEN** a theme with a code-span color is loaded and a doc comment contains `[Link]`
- **THEN** `Link` SHALL be shown underlined in the code-span color
