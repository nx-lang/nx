## 1. Test infrastructure

- [x] 1.1 Extract the grammar loader and `scopesForSubstring` helper from
  `src/vscode/test/grammar/basic.test.ts` into a shared `src/vscode/test/grammar/helpers.ts`, and
  verify `pnpm --dir src/vscode test` still passes with every existing suite importing it
- [x] 1.2 Add a `tokenizeLines(lines)` helper that carries the rule stack across lines and returns
  per-line tokens, plus `scopesAt(result, line, substring)`; verify with a throwaway assertion that a
  two-line record definition reports the second line's scopes correctly
- [x] 1.3 Add negative-assertion support (`expectScopes(...).toNotInclude(...)` or equivalent) so the
  spec's "SHALL NOT be scoped" scenarios are expressible; verify by asserting that `Button` in
  `<Button x=1 />` is not scoped `entity.name.type.nx`

## 2. Failing tests for the reported defects

Write these before touching the grammar; each must fail against the current grammar for the reason
stated in `proposal.md`.

- [x] 2.1 Add `src/vscode/test/grammar/declarations.test.ts` covering the "Element-shaped
  declarations are scoped as declarations" scenarios — `export external component`,
  `export abstract external component`, the element-shaped `let` function, and the element-reference
  counter-case; verify all four fail today on the `entity.name.type.nx` / `storage.modifier.external.nx`
  assertions
- [x] 2.2 Add tests for the "Declaration property lists are scoped independent of layout" scenarios —
  `format: TextFormat = plain`, `letterSpacing: float64 = 0.0`, `color: Color?` / `items: string[]?`,
  and `content text: string` — using the multi-line helper; verify the `TextFormat`-splitting and
  `float64`/`0.0` assertions fail today
- [x] 2.3 Add tests for the "Comments inside a declaration signature" scenarios, including the
  `// >= 1; null` case and the assertion that the property line following it is still scoped as a
  declaration property; verify they fail today
- [x] 2.4 Add tests for the "Emits and state groups are scoped" scenarios; verify they fail today
- [x] 2.5 Add tests for the "Declaration scopes end at the declaration's terminator" scenarios,
  including a multi-line union followed by `export external component`; verify they fail today
- [x] 2.6 Add the position-independence regression: tokenize each component declaration in
  `docs/drawnui-proposal/ui/ui.nx` standalone and again in situ, assert the two token streams are
  identical, and skip cleanly if the file is absent; verify it fails today

## 3. Grammar: keywords and declaration boundaries

- [x] 3.1 Add `external` to `keywords-core` as `storage.modifier.external.nx`; verify the 2.1
  `external` assertions pass
- [x] 3.2 Add `external` to the three declaration-start lookaheads (`nx.tmLanguage.json:353`, `:410`,
  `:611`), keeping the three regexes character-identical; verify the 2.5 union-termination test passes
- [x] 3.3 Add a test asserting each of the three lookaheads terminates an open construct for every
  declaration form (`type`, `action`, `component`, `let`, each with and without
  `private`/`export`/`abstract`/`external`); verify it passes
- [x] 3.4 Add `emits`, `state`, and `content` as `keyword.declaration.emits.nx`,
  `keyword.declaration.state.nx`, and `storage.modifier.content.nx`; verify the 2.4 keyword
  assertions pass
- [x] 3.5 Add a keyword-inventory test that extracts keyword literals from
  `crates/nx-syntax/grammar.js` and asserts each is matched by the TextMate grammar; verify it
  passes and that removing `external` from `keywords-core` makes it fail

## 4. Grammar: the declaration signature context

- [x] 4.1 Add a `declaration-signature` context that begins at the signature's `<Name` and ends at
  its `/>`, scoping the name `entity.name.type.nx` and the terminator as tag punctuation; verify the
  2.1 name-scope and 2.5 terminator tests pass
- [x] 4.2 Rewrite the component definition rule (`nx.tmLanguage.json:535`) to accept
  `[private|export] [abstract] [external] component`, capture the modifiers, and delegate to
  `declaration-signature` instead of ending at `(?==)`; verify the full 2.1 suite passes
- [x] 4.3 Route the element-shaped `let` function declaration through `declaration-signature` as
  well; verify the 2.1 element-function scenario passes
- [x] 4.4 Scope the `extends` base name as `entity.name.type.nx` inside `declaration-signature`;
  verify the `UiCommon` assertions in 2.1 pass
- [x] 4.5 Confirm `start-tag` no longer handles declarations and that
  `basic.test.ts`/`text-elements.test.ts` element-reference tests still pass unchanged

## 5. Grammar: property lists inside a signature

- [x] 5.1 Add a property-definition pattern to `declaration-signature` scoping the name
  `variable.other.property.nx` and the colon `punctuation.separator.type.annotation.nx`; verify the
  2.2 name/colon assertions pass
- [x] 5.2 Scope the type after the colon, splitting primitive (`support.type.primitive.nx`) from
  user-defined (`entity.name.type.nx`), matching the complete identifier with an
  `(?![A-Za-z0-9_-])` boundary before any lookahead; verify the `TextFormat` and `float64`
  assertions in 2.2 pass
- [x] 5.3 Scope the `?` and `[]` type suffixes as `keyword.operator.type-modifier.nx` inside a
  signature; verify the 2.2 suffix scenario passes and the existing ternary-`?` test still passes
- [x] 5.4 Scope default values in a signature (numeric, string, contextual union case, element);
  verify `0.0` scopes `constant.numeric.float.nx` and `plain` scopes `variable.other.enummember.nx`
- [x] 5.5 Scope the `content` modifier ahead of the property name; verify the 2.2 content scenario
  passes
- [x] 5.6 Include `#comments` in `declaration-signature` at a priority above the tag punctuation
  patterns; verify the whole 2.3 suite passes
- [x] 5.7 Add `emits` and `state` group patterns that scope the group keyword, the emitted action
  name, an optional `extends` base, and the group's properties by the same property rules; verify
  the 2.4 suite passes

## 7. Comments and type suffixes outside a signature

Found while reviewing rendered output against the change; same defects, adjacent contexts.

- [x] 7.1 Move `#comments` ahead of `#operators`/`#qualifiers`/`#keywords-core` in `record-property`
  and `value-definition`, so a trailing comment is not shredded into arithmetic, literals, and
  qualifiers; verify with a test per context and confirm each fails against the old ordering
- [x] 7.2 Leave the three text-content contexts alone — `//` is literal text there per
  `nx-grammar-spec.md` — and cover that with a test
- [x] 7.3 Add a structural test asserting `#comments` precedes any rule that also matches at `//`
  in every non-text context, so the ordering cannot regress
- [x] 7.4 Extract the type-with-suffixes rule into a shared `type-annotation` context and use it
  from both `record-property` and `declaration-property`, so `?` and `[]` are scoped alike in a
  record, a signature, and a value definition; verify the ternary `?` is unaffected

- [x] 7.5 Rename `storage.type.primitive.nx` to `support.type.primitive.nx` across the grammar,
  tests, and spec, so primitives sit in the same scope family as user-defined types the way
  TypeScript does; no alias for the old name

- [x] 7.6 Add a `#rhs-expression` rule for the `RhsExpression` production and route every site that
  admits one through it — value definition, function definition, record property default, signature
  property default, attribute value. A record default previously fell through to
  `entity.name.qualifier.nx` and a value definition's right-hand side was unscoped entirely
- [x] 7.7 Consume a paren parameter list as a unit in `value-definition` so a parameter default's
  `=` is not mistaken for the definition's `=`
- [x] 7.8 Confirm against `nx-grammar.md` that a `ContextualName` is never a qualified name, and
  test that a dotted name in a value position is not scoped as a qualified union case

## 6. Audit and verification

- [x] 6.1 Grep the grammar for the greedy-quantifier-then-negative-lookahead shape
  (`[A-Za-z0-9_-]*` immediately followed by `(?!`), fix any other instance the same way as 5.2, and
  add a test for each one found
- [x] 6.2 Re-run the tokenizer over `docs/drawnui-proposal/ui/ui.nx` and `src/vscode/samples/` and
  confirm no token outside a comment or string is left with only the `source.nx` scope
- [x] 6.3 Run `pnpm --dir src/vscode test` and confirm every suite passes, including all pre-existing
  tests unchanged
- [x] 6.4 Run `pnpm --dir src/vscode run compile` and `pnpm --dir src/vscode run package:language`
  and confirm the editor-assets package still builds with the grammar tests green
- [ ] 6.5 Load the extension with `pnpm --dir src/vscode run vscode:launch`, open
  `docs/drawnui-proposal/ui/ui.nx`, and confirm visually that `null` inside the line-164 comment is
  no longer colored as a keyword and that `0.0` on line 163 is colored as a number
- [x] 6.6 Add a `src/vscode/CHANGELOG.md` entry describing the corrected and newly added scopes for
  downstream theme and Monaco/Shiki consumers
