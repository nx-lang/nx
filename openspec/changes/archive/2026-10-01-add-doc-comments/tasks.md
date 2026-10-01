## 1. Doc comment token

- [x] 1.1 Add the `doc_comment` token to `extras` in `crates/nx-syntax/grammar.js` (design D1), run
  `pnpm generate` in `crates/nx-syntax`, and verify the regenerated `parser.c`, `grammar.json`, and
  `node-types.json` build with `cargo build -p nx-syntax`
- [x] 1.2 Add `DOC_COMMENT` to `SyntaxKind`, its token-kind match, `syntax_kind_from_str`, and
  `is_comment()`; verify with parser tests that `/// x` lexes as a doc comment, `//// x` and `// x`
  as line comments, `///` alone as a doc comment, and `/// x` inside text content not as a comment
- [x] 1.3 Map `doc_comment` to `@comment.documentation` in `crates/nx-syntax/queries/highlights.scm`
  and verify with `query_tests.rs`

## 2. Attachment pass

- [x] 2.1 Add `nx-syntax/src/doc_comments.rs` with the documentable-node collection and the
  leading-block attachment (design D2), exposed as the `doc_comments(&root)` function; verify
  unit tests for a component, a property, a `| case` line, an `emits` entry (outermost wins), and
  an `export` declaration
- [x] 2.2 Add trailing attachment: the outermost items that both start and end on the comment's
  line; verify unit tests for a property, a union case with a payload, and a one-line `type`
- [x] 2.3 Add the four attachment diagnostics from design D3, including the help text on
  `misaligned-doc-comment-continuation` and suppression of `dangling-doc-comment` when the file has
  parse errors; verify one test per scenario in the spec's "Doc comment attachment is checked"
  requirement
- [x] 2.4 Build the doc text (marker and one space stripped, lines joined) with its `line_starts`
  map; verify the empty-`///`-line scenario and that a text offset maps back to the right source
  offset
- [x] 2.5 Let a trailing doc comment continue on following `///` lines aligned to its column,
  report a misaligned continuation as `misaligned-doc-comment-continuation` (replacing
  `continued-trailing-doc-comment`), and treat a block after a blank line as an ordinary leading
  block; verify the "An aligned continuation extends a trailing doc comment", "A blank line
  separates a leading block from a trailing doc comment", and "A misaligned continuation is an
  error" scenarios, a partly aligned continuation, and non-ASCII text before the trailing comment

## 3. HIR

- [x] 3.1 Add the `pulldown-cmark` dependency (default features off) and the `Doc` / `DocLink`
  types, extracting links through the broken-link callback (design D4); verify unit tests for
  `[Name]`, `` [`Name`] ``, `[Type.member]`, brackets in a code span, `[text](url)`, and a label
  with a reference definition
- [x] 3.2 Add `doc: Option<Doc>` to `TypeAlias`, `RecordDef`, `UnionDef`, `UnionCaseDef`,
  `UnionCaseField`, `ValueDef`, `Function`, `Param`, `RecordField`, `Component`, and
  `ComponentEmit`, and fill it in `lower.rs` from the `DocComments` table; verify lowering tests
  for each documentable kind in the spec's list
- [x] 3.3 Carry docs onto synthesized and copied members: the inline action record of an `emits`
  entry, update-record fields, `EffectiveField` / `EffectiveEmit`, and the `Interface*` structs in
  `prepared.rs` and `nx-api` (`build_interface_item`, `record_field_to_interface_field`, the
  `from_interface_field` conversions); verify that an inherited field and an imported library
  field keep their docs
- [x] 3.4 Verify the spec's "Documentation does not change what a program means" requirement: a
  test evaluates a program with and without doc comments and gets the same result, and the prelude
  IR image changes only in its module fingerprint, which hashes the source text

## 4. Doc link resolution

- [x] 4.1 Resolve each `DocLink` in `nx-types` during checking (owner members first, then module
  scope including imports and aliases, then member paths), record `doc_links` on `ModuleArtifact`,
  and report the `unresolved-doc-link` warning (design D5); verify one test per scenario in the
  spec's "Doc links name declarations" requirement, including that the module still compiles with
  an unresolved link

## 5. Language service and hosts

- [x] 5.1 Append rendered documentation to hover content after the fenced fragment for
  declarations, references, component properties in tags, union cases, parameters, fields, and
  payload fields, rendering resolved links as code spans; verify one hover test per scenario in the
  editor-language-service "Hover shows a declaration's documentation" requirement, including a
  library reference and an undocumented declaration whose hover is unchanged
- [x] 5.2 Answer hover at a doc link inside a doc comment as a reference to its target, and report
  no result for an unresolved link; verify both scenarios
- [x] 5.3 Add `documentation: Option<String>` to `CompletionItem` and fill it for declaration,
  property, and case items; verify the three completion scenarios
- [x] 5.4 Map completion documentation to `MarkupContent` (Markdown) in `nx-lsp` `to_lsp_completion`;
  verify with an LSP test that a documented property's completion carries it
- [x] 5.5 Add `documentation?: string` to `CompletionItem` in `packages/language-protocol`, update
  the wasm and Node TypeScript declarations, and pass it to Monaco in `packages/monaco`; verify the
  package tests and a JSON round-trip test of a completion answer with documentation

## 6. Typegen

- [x] 6.1 Add `doc: Option<String>` to the typegen model types (design D7) and fill it from HIR in
  the `export_*` converters with doc links replaced by their labels; verify model tests
- [x] 6.2 Add the C# Markdown-to-XML helper (summary/remarks split, emphasis, code, links, lists,
  headings, escaped text for everything else); verify unit tests for each row of the mapping,
  XML escaping, and raw HTML kept as text
- [x] 6.3 Call the C# helper from every C# emit point for a documented declaration or member,
  keeping the fixed summary on a constant case's `Instance` field; verify the C# scenarios in
  the cli-code-generation spec and that the .NET test project builds with the generated code with
  no XML documentation warnings (CS1570)
- [x] 6.4 Emit `/** */` comments from the TypeScript writer, keeping the Markdown, escaping `*/` and
  word-initial `@` outside code, and skipping constant cases of a string-literal union; verify the
  TypeScript scenarios and that the generated output type-checks with `tsc`
- [x] 6.5 Verify that typegen snapshots for sources without doc comments are unchanged

## 7. Editor assets

- [x] 7.1 Add the `doc-comment` pattern to `src/vscode/syntaxes/nx.tmLanguage.json` before
  `line-comment`, and mirror it in `nx.markdown.codeblock.tmLanguage.json` if that grammar defines
  its own comments; verify the editor-syntax-highlighting scenarios in
  `src/vscode/test/grammar/comments.test.ts`
- [x] 7.2 Add the `onEnterRules` entry that continues a leading `///` block to
  `src/vscode/language-configuration.json` (design D8); verify by pressing Enter at the end of a
  leading and a trailing doc comment in the VS Code extension

- [x] 7.3 Scope inline Markdown inside `///` comments in the TextMate grammar: doc links, links with
  a destination, code spans, strong emphasis and emphasis; verify the editor-syntax-highlighting
  Markdown scenarios in `src/vscode/test/grammar/comments.test.ts`
- [x] 7.4 Add `nx_wasm_program_diagnostics` and `NxProgramArtifact.diagnostics()` to the wasm SDK
  (ABI version 5), return a compiled program's warnings from the playground's `evaluateSource`, mark
  them beside a value or a runtime error, and fail the example check on them; verify the sdk-wasm
  and playground scenarios

- [x] 7.5 Add `withNxDocCommentStyles` to `packages/monaco` and apply it to every theme it loads, and
  move the playground's source and output to `github-dark-default` / `github-light-default`; verify
  the monaco-language-integration and playground scenarios

- [x] 7.6 Complete doc link names: `nx_types::doc_link_candidates` from the resolver's own scopes, a
  `Comment` position context with the doc link written so far, and no completions elsewhere in a
  comment; suggest as the author types inside comments in the playground and, through
  `configurationDefaults`, in VS Code, with the editor's word-based suggestions off so plain text in
  a comment offers nothing; verify the editor-language-service "Doc link names are
  completed" scenarios and, in the playground, that typing `[Lo` offers `LoadState` and
  `[LoadState.f` offers `failed`
- [x] 7.7 Open the playground's completion details pane the first time a completion with something
  to show appears; verify in a browser that the first list shows the documentation and that a pane
  the visitor closes stays closed

## 8. Documentation and adoption

- [x] 8.1 Document doc comments in `nx-grammar.md` and `nx-grammar-spec.md` (the trivia sections),
  and add a comments page to the website language reference under `sites/website/src/content/docs`
  with leading, trailing, link, and error examples; verify `check-code-blocks.mjs` passes
- [x] 8.2 Convert the documentation-style `//` comments in `crates/nx-api/src/prelude.nx` and
  `docs/drawnui-proposal/**/*.nx` to `///`, and verify those sources report no attachment
  diagnostics and that hover on a prelude type shows its documentation
- [x] 8.3 Run `cargo test --workspace`, `cargo clippy --workspace`, and the JS package tests, and
  verify all pass
- [x] 8.4 Update the Comments reference page, `nx-grammar.md`, and `nx-grammar-spec.md` for aligned
  continuations, and rerun `check-code-blocks.mjs`
