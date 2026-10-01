## Why

NX has no way to document a declaration. Every comment is trivia the lexer discards, so an author's
explanation of a component, a property, or a union case never reaches the people who use it: not
hover, not completion, not the C# and TypeScript types `typegen` generates. Authors already write
this documentation anyway — the DrawnUI catalogs in `docs/drawnui-proposal` describe components in
leading `//` blocks and properties in trailing `//` comments — and all of it is invisible to tools.

## What Changes

- Add doc comments to the language. A `///` line comment on its own line documents the declaration
  that follows it; a `///` comment after code on the same line documents the item written on that
  line. Exactly three slashes: `////` and longer remain ordinary comments. Module-level docs (`//!`)
  are left for a follow-up, since no tool has a place to show them yet.
- Doc comment text is CommonMark. Its first paragraph is the summary.
- Every documentable thing can carry its own doc: top-level declarations, record fields, union
  cases and their payload fields, component props, `emits` entries, `state` fields, and function
  parameters. There is no `@param`-style tag vocabulary; a member is documented where it is declared.
- A trailing doc comment continues onto following `///` lines aligned with it, so a member's
  documentation can run to several lines without leaving the member's line.
- Attachment is checked. A doc comment that documents nothing, a trailing doc comment whose line
  starts no item or starts more than one, a `///` line directly under a trailing doc comment but
  not aligned with it, and an item documented both ways are each a static error. A leading block
  for the next item after a trailing doc comment therefore needs a blank line before it.
- `[Name]` and `[Type.member]` in doc text are links to declarations, resolved like any name in the
  module. A link that resolves to nothing is a warning.
- The language service shows documentation in hover and in completion items, and hovering a link
  inside a doc comment reports its target. Typing a link in a doc comment completes the names it
  can resolve to, and nothing else in a comment offers completions.
- `typegen` carries docs into generated code: `/// <summary>` XML documentation in C# and `/** */`
  comments in TypeScript.
- Editor grammars scope doc comments as documentation comments, and the inline Markdown in them:
  doc links, code spans, and emphasis, with the Markdown characters left visible.
- The Monaco integration styles doc comment Markdown as documentation: emphasis keeps the comment
  color and changes only the font, and a doc link takes the code color. The playground moves to
  GitHub's current themes, whose comments are readable.
- The wasm SDK reports the warnings of a program that builds, and the playground marks them, so an
  unresolved doc link is visible there too. Until now the playground showed diagnostics only when
  a program failed to compile.
- Completion items gain an optional markdown documentation field in the language protocol, the LSP
  adapter, and Monaco.

## Capabilities

### New Capabilities

- `doc-comments`: the `///` form, how a doc comment attaches to a declaration (leading and
  trailing), the attachment diagnostics, Markdown content and summary, and doc links.

### Modified Capabilities

- `editor-language-service`: hover and completion items include a declaration's documentation; hover
  on a doc link reports the linked declaration.
- `cli-code-generation`: generated C# and TypeScript declarations carry the NX documentation of the
  declaration and member they come from.
- `editor-syntax-highlighting`: `///` comments are scoped as documentation comments, with their
  inline Markdown scoped within them.
- `language-protocol`: a completion item may carry documentation as markdown.
- `sdk-wasm`: a built program artifact reports its warnings.
- `playground`: the warnings of source that compiles are marked, and the source and output use
  GitHub's current themes.
- `monaco-language-integration`: themes style doc comment Markdown as documentation.

## Impact

- `crates/nx-syntax`: grammar tokens for doc comments, regenerated parser, `SyntaxKind` entries, and
  the attachment pass with its diagnostics.
- `crates/nx-hir`: a `doc` field on every documentable declaration and member, filled during
  lowering, and carried through the library interface structures in `prepared.rs` and `nx-api`.
- `crates/nx-types`: doc link resolution and its warning.
- `crates/nx-language-service` (hover, completions) and the LSP/HTTP/wasm adapters that pass
  completion documentation through.
- `crates/nx-cli` typegen model and the C# and TypeScript writers.
- VS Code / Monaco TextMate grammar.
- `bindings/wasm` (a new `nx_wasm_program_diagnostics` export, ABI version 5, and
  `NxProgramArtifact.diagnostics()`) and `sites/playground` (marking those warnings, and failing the
  example check on them).
- Language documentation (`nx-grammar.md`, `nx-grammar-spec.md`, the website's language reference).
- **BREAKING** (in principle): `///` is an ordinary comment today, so a source that writes one
  where it documents nothing now gets an error. No `.nx` file in the repository does.
