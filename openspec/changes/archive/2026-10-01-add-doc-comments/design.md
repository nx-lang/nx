## Context

See proposal.md for motivation and `specs/doc-comments/spec.md` for the language rules.

- Comments are tree-sitter `extras` (`crates/nx-syntax/grammar.js:11-16`). Tree-sitter hangs an extra
  on whatever node is open where it appears, so a comment above a property may be a child of
  `component_signature`, `record_definition`, `state_group`, or `emit_definition`, and a comment
  above a top-level item is usually a child of `module_definition`. `SyntaxNode::children()` and the
  sibling accessors already skip comments; `children_with_tokens()` keeps them.
- Every documentable HIR type already stores the span of its whole syntax node: `TypeAlias`,
  `RecordDef` (records and actions), `UnionDef`, `UnionCaseDef`, `UnionCaseField`, `ValueDef`,
  `Function`, `Param`, `RecordField` (record fields, component props, and state), `Component`, and
  `ComponentEmit`. Cross-module copies live in `prepared.rs` (`InterfaceField`, `InterfaceParam`,
  `InterfaceUnionCase`, `InterfaceItem`) and are built by `nx-api` (`build_interface_item`,
  `record_field_to_interface_field`).
- Hover already reaches library and prelude declarations through each library's in-memory
  `ModuleArtifact.lowered_module`. The serialized NX IR is runtime-only and is not read back for
  editor information, so docs do not need to be in it.
- Lowering diagnostics (`LoweringDiagnostic`) have no code or severity. Syntax validation
  (`nx-syntax/src/validation.rs`) and checking (`nx-types`) produce coded `Diagnostic`s, and a
  warning from either does not block lowering or evaluation.
- There is no NX source formatter, and no signature-help or definition query.

## Goals / Non-Goals

**Goals:**
- One attachment pass, owned by `nx-syntax`, that every later stage reads instead of rediscovering
  comments.
- Doc text flows through HIR to hover, completion, and typegen with no stage re-reading source.
- Docs never affect evaluation, types, or NX IR.

**Non-Goals:**
- `//!` module documentation. Nothing can show it yet: hover has no import positions and typegen
  generates no per-module declaration. It is left for a follow-up that adds a place to show it.
- Checking or running code examples in documentation (doctests).
- Aligning trailing doc comments; there is no formatter.
- Clickable doc links, go-to-definition from a link, and signature help. None of those queries exist.
- Hover for component `state` fields and `emits` entries, which have no hover today. (A doc link to
  an `emits` entry does hover, as the action the entry names, which has one.) Their docs are
  still attached and reach typegen.
- A deprecation marker. That belongs in an attribute, not in doc text.
- Rendering doc Markdown in the playground value view (`packages/value-view/src/markdown.ts`).

## Decisions

### D1. Doc comments stay trivia, with their own token

Add a `doc_comment` token to `extras` next to `line_comment`:

```js
doc_comment: $ => token(seq('///', optional(seq(/[^\/\n]/, /.*/)))),
```

For `/// text` both tokens match the same length, and tree-sitter breaks the tie in favor of the rule
declared first, so `doc_comment` is declared before `line_comment`. For `//// text` `doc_comment` can
only match `///`, so the longer `line_comment` wins, which is what keeps four slashes ordinary. The
character after `///` may be `\r`: tree-sitter's `.` matches `\r`, so if `doc_comment` excluded it, an
empty `///` line in a CRLF file would lose to `line_comment`'s longer `///\r`. The
token must not have a precedence: tree-sitter ranks lexical precedence above match length, so
`prec(1)` would lex `////` as `///` followed by an error. `SyntaxKind` gets `DOC_COMMENT`, and
`is_comment()` includes it, so every existing comment-skipping path is unaffected.

*Alternative:* make doc comments grammar children of each documentable rule. That would touch
every property, case, and declaration rule, add parser conflicts, and still could not express
the trailing form, which comes after the item's last token. Rejected.

### D2. Attachment is a positional pass over the whole tree

A new module, `nx-syntax/src/doc_comments.rs`, runs after parsing:

1. Walk the tree once, collecting every `DOC_COMMENT` token and every documentable node with its
   start line, end line, and start offset. The documentable node kinds are the ones the
   `doc-comments` spec lists; a `union_case` node already includes its leading `|`, so "first
   token on the line" needs no special case for unions.
2. Classify each doc comment as leading or trailing by whether a non-trivia token precedes it on
   its line, and record its column (in characters from the line start). Group consecutive leading
   lines into blocks.
3. A leading block that starts on the line after a trailing doc comment is first read as that
   comment's continuation: its lines whose `///` is at the trailing comment's column join the
   trailing comment, from the top, until the first line that is not. That line and the rest of the
   block are a misaligned continuation and document nothing. A blank line in between ends the
   trailing comment, so a block after one is an ordinary leading block.
4. For a leading block, take the first non-trivia token on the next line and pick the outermost
   documentable node that starts there. For a trailing comment, take the documentable nodes that
   start and end on its line and keep the outermost ones; exactly one must remain.
5. Apply the checks from the spec's "Doc comment attachment is checked" requirement.

The pass is a pure function of the root node, `nx_syntax::doc_comments(&root) -> DocComments`,
returning a table keyed by the documented node's start offset. Syntax validation runs it to report
the attachment diagnostics, and `nx_hir::lower` runs it on the root it is given and looks up
`docs.get(node.span().start())` for each node it lowers. `lower` takes only a root node and has
dozens of callers, so recomputing the table there (one tree walk) is simpler than threading it
from `ParseResult` through every one of them.

Working from positions rather than from the tree's parent/child structure is deliberate: it makes
the result independent of which node tree-sitter happened to hang a comment on.

A visibility modifier's position is whatever the declaration node covers; if `export` turns out
to be a sibling of the declaration node rather than part of it, the pass treats the modifier's
start as the declaration's start.

*Alternative:* attach during HIR lowering. Every lowering function would need the same line
arithmetic, and the syntax-level diagnostics would move out of syntax validation. Rejected.

### D3. The attachment diagnostics

Syntax-validation errors, one code each:

| Code | Case |
|---|---|
| `dangling-doc-comment` | A leading block not followed by a documentable item, or a trailing comment with none on its line |
| `ambiguous-trailing-doc-comment` | Two or more outermost items on a trailing comment's line |
| `misaligned-doc-comment-continuation` | A `///` line directly under a trailing doc comment (or its continuation) whose `///` is not at the trailing comment's column. The help text says to align it to continue the comment, or to put a blank line before it to document the next item. |
| `duplicate-doc-comment` | An item documented both ways |

When the file has parse errors, `dangling-doc-comment` is suppressed. An error-recovery node is not
a documentable item, so a half-typed declaration would otherwise produce a second, misleading
diagnostic.

### D4. `Doc` in HIR, with links parsed once

`nx-hir` gets `Doc`, one shared `Arc` around `{ text, links, line_starts }`. Every copy of a
declaration carries its doc, and a single pointer keeps `Option<Doc>` to eight bytes in each:

- `DocLink` holds the label, its identifier path, and its source span.
- `line_starts` maps an offset in the text back to the source, so a link's span is a real source
  range.

Each documentable HIR type above gets `doc: Option<Doc>`. So do their effective copies
(`EffectiveField`, `EffectiveEmit`) and the interface copies in `prepared.rs`, so an inherited or
imported member keeps its documentation. Some items are synthesized from a documented member:

- The inline action `RecordDef` made for an `emits` entry takes the entry's doc.
- An update record's fields take the doc of the field or state entry they mirror.

Links are extracted with `pulldown-cmark` (a new dependency, with default features off). Its
broken-link callback reports exactly the spec's doc links: shortcut references with no matching
definition. It never looks inside code spans or code blocks, and ignores links with an explicit
destination. Rustdoc uses the same crate for the same job. Hand-parsing CommonMark's code-span and
reference-definition rules is where a home-grown extractor would go wrong.

### D5. Doc links resolve in `nx-types`

`nx-types` already resolves names at module scope, including imports and aliases, and already emits
coded warnings. Checking a module resolves each `DocLink`:

- The first identifier is looked up among the owner declaration's members, then at module scope.
- Each further identifier names a member of the previous part's target.

The results go into `ModuleArtifact` as `doc_links: Vec<ResolvedDocLink { span, target }>`, where
`target` is the declaration's visible name plus an optional member (a parameter, field, property,
state field, emit, case, or payload field): the terms the language service already answers hover
in. An inline `emits` entry's documentation looks among its own payload fields before the
component's members. An
unresolved link is the warning `unresolved-doc-link`. The language service reads `doc_links` for
two things: to render a resolved link as a code span, and to answer hover on a link by treating it
as a reference to `target`. A library's links were resolved when the library was checked, so its
docs render the same way in a consumer's hover.

### D6. Hover, completion, and the protocol

- **Hover:** the documentation goes after the fenced fragment, separated by a blank line and before
  any existing section such as "Inherited from". There is no horizontal rule; that matches the
  TypeScript server's hovers.
- **Completion:** `CompletionItem` gets `documentation: Option<String>` (Markdown, rendered like
  hover).
- **Hosts:** it is mapped to `Documentation::MarkupContent` in `nx-lsp`, added as `documentation?:
  string` to `packages/language-protocol`, and passed to Monaco as `{ value }`. The wasm and Node
  hosts serialize the Rust type, so they need only their TypeScript declarations updated.

### D7. Typegen reads docs from HIR

The typegen model types (`ExportedRecordField`, `ExportedRecord`, `ExportedUnion`,
`ExportedUnionCase`, `ExportedAlias`, `ExportedExternalState`, `ExportedUpdate`) get
`doc: Option<String>`, holding the Markdown with each doc link already replaced by its label
in a code span. Neither writer then needs to know about doc links: C# turns the code span into
`<c>`, and TypeScript keeps it.

- **C#:** a `csharp_doc_lines(&str) -> Vec<String>` helper walks the `pulldown-cmark` event stream
  and applies the spec's Markdown-to-XML mapping. Each supported start/end event pair becomes an
  element: strong and emphasis, code, link, list and item, and heading. Every other event is
  written as escaped text. Every writer that emits a documented declaration calls the helper before
  the attributes. A constant case's class carries the case's documentation; the fixed summary on
  its `Instance` field documents that field and is unchanged.

  The target elements are the ones Visual Studio and Rider tooltips render. XML documentation has
  no standard bold or italic, so `<b>`, `<i>`, and `<a href>` are the de facto set, and DocFX
  accepts them as well. *Alternative:* keep the Markdown as literal text. That would show
  asterisks in IntelliSense for every emphasized word. Rejected.

  A doc link could instead become `<see cref="…"/>` when its target is a type `typegen` also
  generates, which makes it clickable in the IDE. That needs a map from NX names to generated C#
  names across modules and libraries, so it is left for later; `<c>` is right in the meantime.
- **TypeScript:** writes `/**`, one ` * `-prefixed line per source line, and ` */`. The Markdown is
  kept as written, because TSDoc and JSDoc bodies are Markdown and TypeScript tools render them.
  Two escapes apply:
  - `*/` in the text is written as `*\/`.
  - An `@` that begins a word, outside a code span or block, is written as `\@`. JSDoc reads
    `@word` as a tag and would otherwise drop the text that follows it from the description.

  The code-span check for `@` reuses the same `pulldown-cmark` event stream's offsets.

### D8. Editor assets

- **TextMate:** `nx.tmLanguage.json` gets a `doc-comment` pattern, `(///)(?!/).*$`, scoped
  `comment.line.documentation.nx` and listed before `line-comment` in the `comments` repository.
  Every context that includes `#comments` then picks it up.
- **Tree-sitter:** `highlights.scm` maps `doc_comment` to `@comment.documentation`.
- **Enter key:** `language-configuration.json` gains an `onEnterRules` entry with
  `beforeText: ^\s*///(?!/)` that continues a *leading* block with `/// `. It does not continue a
  trailing doc comment: a continuation must start at the trailing comment's column, which an
  `onEnterRules` entry cannot compute, and `/// ` at the line's indentation would be the
  `misaligned-doc-comment-continuation` error.
  `packages/monaco` converts the same rule for the playground.

## Risks / Trade-offs

- **Stricter than most languages.** A blank line between a doc block and its item is an error.
  Rust and Kotlin allow it. → The error message says exactly what to change, and the rule keeps a
  doc comment from silently attaching to something far away.
- **Alignment is significant.** A continuation line one column off is an error rather than a
  leading block for the next item. → That is the point: the column says which of the two the author
  meant, the error names both fixes, and a blank line always makes the intent unambiguous. It also
  keeps multi-line trailing docs aligned, the style they are written in anyway.
- **Line-based rules in a free-form language.** An author who puts several properties on one line
  cannot document them individually. → The leading form still works on a multi-line layout, and
  the ambiguity is an error rather than a guess.
- **Lossy Markdown-to-XML conversion for C#.** Tables, images, raw HTML, and nested block
  structure appear as literal text. → The mapping covers what member docs actually use (emphasis,
  code, links, lists). The spec states it, so its output can be tested.
- **`pulldown-cmark` grows the wasm bundle.** → Default features are off and only the parser is
  used. Measured on `nx.wasm` (wasm-release): 2,759,890 bytes (929,797 gzipped) before this change,
  3,075,085 (1,041,938) after. Of the 112 KB gzipped increase, about 83 KB is `pulldown-cmark`,
  which is used in the wasm only to find doc links; the rest is the attachment pass, link
  resolution, and the language-service work. A hand-written extractor for code spans, code blocks,
  and reference definitions would recover most of the 83 KB, at the cost D4 describes. Accepted
  for now: `pulldown-cmark` stays, and because only `doc_links()` uses it in the wasm, a hand-written
  extractor can replace it there later without touching callers if a size budget calls for one.
- **Doc text in HIR invalidates incremental queries on doc edits.** → An edit to a doc changes the
  source text, and that fingerprint already invalidates. No new cost.

## Migration Plan

Nothing is deployed separately. After the language work lands, convert the documentation-style
`//` comments in `crates/nx-api/src/prelude.nx` and in `docs/drawnui-proposal/**/*.nx` to `///`.
This exercises both forms on real sources and gives hover something to show for prelude and
catalog types. Snapshot tests that include those sources will need their snapshots updated. No
existing `.nx` source contains `///`.
