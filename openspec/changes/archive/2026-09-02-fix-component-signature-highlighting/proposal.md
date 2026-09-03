## Why

The NX TextMate grammar (`src/vscode/syntaxes/nx.tmLanguage.json`) has never learned the `external`
modifier, and its component/function signature path was written for a single-line signature with no
default values. Real NX catalog files — `docs/drawnui-proposal/ui/ui.nx` is the motivating example —
are almost entirely `export external component <Name extends Base` declarations with multi-line,
defaulted, comment-annotated property lists, and the grammar mis-tokenizes essentially all of them.

The failures compound rather than degrade gracefully: a `//` comment containing `>` silently closes
the enclosing tag and the rest of the declaration is highlighted as an expression, and a multi-line
union declaration earlier in the file never terminates, so every subsequent declaration in the file
is nested inside a stale union-case scope. There is no test coverage for `external`, for multi-line
signatures, or for comments inside a property list, which is why none of this was caught.

## What Changes

Grammar corrections (all verified against the installed grammar with `vscode-textmate`):

- **Recognize `external`.** Add it to `keywords-core`, to the component definition `begin` pattern,
  and to the three "a new declaration starts here" lookaheads that currently terminate union and
  record bodies. Today `external` has no scope at all, and its absence from the lookaheads lets the
  multi-line union at `ui.nx:30` swallow the remaining ~160 lines of the file.
- **Make the component definition rule cover a real signature.** The rule at
  `nx.tmLanguage.json:535` accepts neither `external` nor `abstract`, and ends at `(?==)` — the
  first default value — so it cannot span a multi-line property list. Declarations therefore fall
  through to the generic XML `start-tag` rule and the component name is scoped
  `entity.name.tag.nx` (an element *reference*) instead of `entity.name.type.nx` (a *declaration*).
  Re-anchor the rule so it spans from the opening `<` to the signature's `/>`.
- **Recognize comments inside a signature.** `start-tag` has no `#comments` include. On
  `ui.nx:164`, `// >= 1; null` tokenizes as tag-self-closing `/`, tag-end `>`, assignment `=`,
  numeric `1`, and `constant.language.null.nx` — the comment closes the tag and every following
  line of the declaration, including the real `/>`, is mis-scoped.
- **Fix the type-annotation backtrack.** `":\\s*(?:(?!(?:raw|if)\\b)(?:[A-Za-z_][A-Za-z0-9_-]*)(?!\\s*=))?"`
  places `(?!\s*=)` after a greedy identifier, so the engine gives back one character to satisfy
  it: `TextFormat` scopes as `TextForma` + a stray `t`, and `float64 = 0.0` scopes as `float6`
  with `4 = 0.0` left entirely unscoped, so the numeric default loses its highlighting.
- **Separate the annotation colon from the type.** `": TextForma"` is currently a single token named
  `support.type.text.nx`; the colon and its whitespace should carry
  `punctuation.separator.type.annotation.nx`, as the `let` rule already does.
- **Scope property names, base types, and type suffixes in a signature.** Property names
  (`format`, `alt`), the `extends` base (`UiCommon`), and the `[]` sequence suffix currently get no
  scope inside a signature.
- **Add the missing keywords `emits`, `state`, and `content`.** None appear anywhere in the grammar.
  An `emits { ... }` group inside a signature is entirely unscoped; `state` scopes as
  `entity.name.qualifier.nx`; `content text: string` tokenizes as one unscoped run.
- **Fix the signature-closing `/>`.** In `component <SearchBox ... /> = {` the `/>` scopes as
  `keyword.operator.arithmetic.nx` + `keyword.operator.comparison.nx`.

Test coverage:

- Add a grammar test suite for element-shaped declarations covering every case above, plus a
  regression test that tokenizes `docs/drawnui-proposal/ui/ui.nx` end to end and asserts no stale
  union-case or tag scope survives to the last line.

## Capabilities

### New Capabilities
- `editor-syntax-highlighting`: The scopes the NX TextMate grammar assigns to NX source — element-shaped
  declarations (components and `let` functions), their property lists, modifiers, `emits`/`state`
  groups, comments inside a signature, and the requirement that a declaration's scopes never leak
  past its terminator.

### Modified Capabilities
<!-- None. `editor-assets` covers how grammar assets are packaged and published, not what the
     grammar highlights; its requirements are unchanged by this work. -->

## Impact

- `src/vscode/syntaxes/nx.tmLanguage.json` — `keywords-core`, `types`, the component definition
  rule, `start-tag`, `attributes`, and the union/record `end` lookaheads.
- `src/vscode/test/grammar/` — new test file for element-shaped declarations; existing suites should
  continue to pass unchanged.
- Downstream consumers of the grammar re-render differently: the `@nx-lang/language` npm package,
  the `nx-language` VSIX, and the markdown fenced-code-block grammar that delegates to `source.nx`.
- No compiler, parser, or LSP change. The tree-sitter grammar (`crates/nx-syntax/grammar.js`)
  already models `external`, `emits`, `state`, and `content` correctly and is the reference for
  what the TextMate grammar should recognize.
