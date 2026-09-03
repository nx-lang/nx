# Review: fix-component-signature-highlighting

## Scope
**Reviewed artifacts:** `proposal.md`, `design.md`, `specs/editor-syntax-highlighting/spec.md`,
`tasks.md`  
**Reviewed code:** `src/vscode/syntaxes/nx.tmLanguage.json`, `src/vscode/CHANGELOG.md`, and all
staged files under `src/vscode/test/grammar/`

## Findings

### ✅ Verified - RF1 A new declaration cannot recover from an open signature or nested record body
- **Severity:** High
- **Evidence:** The spec requires every top-level declaration to terminate an open union case,
  record body, or signature (`specs/editor-syntax-highlighting/spec.md:197`). The recovery lookahead
  on the component rule (`src/vscode/syntaxes/nx.tmLanguage.json:636`) cannot run while its nested
  `declaration-signature` is active, whose only end is `/ >`/`/>` (`:2185`). Record contexts likewise
  end only at `}` (`:575`, `:800`). Tokenizing an unterminated `component <Broken` or `type Broken =
  {` followed by `export external component <Next />` leaves `Next` inside the stale meta scope and
  scopes it as `entity.name.tag.nx`. The boundary matrix opens only union variants, and its record
  body is already closed (`src/vscode/test/grammar/declarations.test.ts:198`).
- **Recommendation:** Add declaration-start recovery to every nested context that can keep an outer
  declaration open, including declaration properties/signatures and record bodies, then add tests
  with genuinely missing `/>` and `}` terminators.
- **Fix:** Added declaration-start recovery to every context that can hold a declaration open —
  `declaration-signature`, `declaration-property`, `record-property`, the record and action bodies,
  the union-case and union-base brace rules, the nested `record-body` brace, and the `emits`/`state`
  groups. Regression tests cover a signature with no `/>` and a record body with no `}`, each
  followed by `export external component <Next />`; both fail against the old grammar.
- **Verification:** Reopened. The two added direct cases recover, but a nested child can still block
  every new boundary. `#values-braced-expression` still ends only at `}`
  (`src/vscode/syntaxes/nx.tmLanguage.json:908`). Tokenizing either `component <Broken` or
  `type Broken = {`, followed by `x: Element = {` and then
  `export external component <Next />`, leaves `Next` inside the stale braced/signature or record
  scopes and scopes it `entity.name.tag.nx`. Recovery must also unwind persistent child contexts
  reachable from a declaration property, with a regression covering an unterminated nested default.
- **Fix (2nd pass):** The persistent child contexts named in the reopen now carry the same
  declaration-start recovery. The lookahead had to take two shapes: declaration contexts anchor on
  `^\s*`, while expression contexts split three ways, because anchoring all four keywords at
  column 0 silently renested the indented `let` at `src/vscode/samples/tally-survey.nx:120`.
  Sixteen expression contexts carry it, `#values-braced-expression` among them.
- **Verification (2nd pass):** Verified against the reopen's own two reproductions. Tokenizing
  `component <Broken` / `x: Element = {` / `export external component <Next />`, and the same with
  `type Broken = {`, now scopes `Next` as `entity.name.type.nx` inside a fresh
  `meta.definition.component.nx`, with no stale braced, signature, or record scope surviving.
- **Verification:** Verified. Both nested-default reproductions still unwind to a fresh component,
  and `Next` is scoped `entity.name.type.nx` without stale braced, signature, or record scopes. The
  full declaration-recovery regression set also passes.

### ✅ Verified - RF2 The shared type annotation rule is neither shared everywhere nor able to match `TypeSuffix*`
- **Severity:** Medium
- **Evidence:** The change requires identical type scoping in signature, record, and value
  annotations, with `?` always a type modifier there
  (`specs/editor-syntax-highlighting/spec.md:62`). `value-definition` still uses its legacy type and
  operator patterns (`src/vscode/syntaxes/nx.tmLanguage.json:388`), so `let x: Catalog[]? = item`
  scopes `?` as `keyword.operator.conditional.nx`. The shared rules accept only zero or more `[]`
  followed by at most one `?` (`:2296`, `:2310`), while the language allows source-ordered
  `TypeSuffix*`, including `string?[]?` (`nx-grammar-spec.md:251`). Thus `T?[]` leaves `[]` unscoped.
  The test named "every annotation position" omits value definitions
  (`src/vscode/test/grammar/basic.test.ts:422`).
- **Recommendation:** Route value annotations, parameter annotations, and function return types
  through one shared rule that repeats either `?` or `[]` in source order, and cover each annotation
  position plus both suffix orders.
- **Fix:** `value-definition`, its parenthesized parameter list, and `function-definition`'s
  return type now all route through the shared `#type-annotation` context instead of their own
  colon/type/`#types` trio, so `?` is a type modifier in all of them. `#type-annotation` now matches
  `((?:\[\]|\?)*)` — `TypeSuffix*` in source order — so `string?[]` and `Color[]?[]` scope every
  suffix. The "every annotation position" test now covers a value definition, a parameter, and a
  return type, plus both suffix orders; the ternary `?` assertion still passes.
- **Follow-up:** Routing the parameter list through `#type-annotation` alone still left the
  parameter *name* on `entity.name.qualifier.nx` and its default unscoped as an `RhsExpression`,
  since a paren parameter is a whole `PropertyDefinition`, not just an annotation. The list now
  includes `#declaration-property` (with `,` and `)` added to that rule's end lookahead), so name,
  type, and default are all scoped by the signature property rule. The spec's `RhsExpression`
  requirement now names a parenthesized parameter default as a sixth site, and the parity test
  covers it.
- **Verification:** Verified. Focused tokenization confirms value annotations, parenthesized
  parameter names/types/defaults, and function return types use the shared rule; `Catalog[]?`,
  `string?[]`, and `Color[]?[]` receive type-modifier scopes throughout, while the ternary `?`
  remains conditional. The expanded parity and annotation tests pass in the full suite.

### ✅ Verified - RF3 `content` properties in `emits` and `state` groups use weaker scoping
- **Severity:** Medium
- **Evidence:** Group properties must follow the same rules as signature properties
  (`specs/editor-syntax-highlighting/spec.md:171`). Both group bodies delegate to `#record-body`
  (`src/vscode/syntaxes/nx.tmLanguage.json:2139`, `:2169`), whereas only
  `#declaration-property` recognizes and scopes the optional `content` modifier (`:2061`). In
  `emits { Changed { content text: string } }` and `state { content child: Element }`, `content` is
  consequently scoped `entity.name.qualifier.nx`, not `storage.modifier.content.nx`.
- **Recommendation:** Share the full property-definition context between signatures and group
  bodies, or teach the shared record property rule the optional modifier, and add content-marked
  property cases for both groups.
- **Fix:** The `emits` action body and the `state` group now include `#declaration-property`
  ahead of `#record-body`, so a group property is scoped by the signature property rule and its
  optional `content` modifier is `storage.modifier.content.nx`. Tests added for both groups.
- **Verification:** Verified. In both `emits { Changed { content text: string } }` and
  `state { content child: Element }`, `content` is now `storage.modifier.content.nx`, the property
  name uses `variable.other.property.nx`, and the added regression tests pass.

### ✅ Verified - RF4 Qualified component and element-function names are not recognized as declarations
- **Severity:** Medium
- **Evidence:** `declaration-signature` accepts only one identifier before whitespace or the
  terminator (`src/vscode/syntaxes/nx.tmLanguage.json:2176`). The language defines the signature name
  as `ElementName`/`QualifiedMarkupName` (`nx-grammar-spec.md:280`, `:313`, `:657`). Valid inputs such
  as `component <Ns.Widget value: string />` and `let <Ns.Row item: string /> = ...` therefore never
  enter the signature context; their declared names and properties remain unscoped rather than
  receiving declaration scopes.
- **Recommendation:** Match and capture the full qualified markup name in the declaration-signature
  begin rule, preserving hyphen support after dots, and test both component and element-function
  declarations.
- **Fix:** `declaration-signature`'s begin now matches a `QualifiedMarkupName`
  (`([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_-]*)*)`), keeping hyphen support after the dot.
  `component <Ns.Widget value: string />` and `let <Ns.Row item: string /> = ...` now enter the
  signature context; a test asserts the name span is the whole `Ns.Widget` and that its properties
  are scoped.
- **Verification:** Verified. Component and element-function probes scope the complete qualified
  name as one `entity.name.type.nx` token, exclude `entity.name.tag.nx`, and retain declaration
  property scoping.

### ✅ Verified - RF5 A whitespace-separated `/ >` terminator closes the signature but not its component
- **Severity:** Medium
- **Evidence:** The signature end intentionally accepts whitespace between `/` and `>`
  (`src/vscode/syntaxes/nx.tmLanguage.json:2185`), but the parent component exits only through the
  exact lookbehind `(?<=/>)` (`:636`). For `component <C / > = { state { value: string } }`, the
  terminator gets punctuation scopes, but the body remains inside `meta.definition.component.nx`
  and `state` is left unscoped until another declaration appears.
- **Recommendation:** Make the parent lifetime derive from the signature's actual close rather than
  an exact textual lookbehind, and add a `/ >` component-body regression test.
- **Fix:** The component rule's end lookbehind is now `(?<=/\s*>)`, mirroring the signature's
  `(/)\s*(>)` rather than an exact `/>`; Oniguruma accepts the variable-length form. A
  `component <C / > = { state { … } }` test asserts `state` is scoped and that the component scope
  does not survive its signature.
- **Verification:** Verified. The `/ >` probe closes both contexts, scopes the following `state`
  group, and does not carry `meta.definition.component.nx` into the body; the focused regression
  and full suite pass.

### ✅ Verified - RF6 Core regression tests silently miss some of their intended targets
- **Severity:** Medium
- **Evidence:** The corpus range detector requires `component|let` and `<` on the same line
  (`src/vscode/test/grammar/corpus.test.ts:15`), so it skips the valid split declaration at
  `docs/drawnui-proposal/ui/ui.nx:182`; it also stops at the first textual `/>` (`:20`), which can be
  an element-valued property default rather than the signature terminator. Separately, the return
  type test computes the last `Element` but passes only the string `Element`
  (`src/vscode/test/grammar/declarations.test.ts:53`), and the helper always resolves the first
  occurrence (`src/vscode/test/grammar/helpers.ts:56`), so it checks the property type instead of the
  required function return type.
- **Recommendation:** Make token lookup occurrence/offset-aware, derive declaration ranges from
  tokenizer state or a delimiter-aware scan, and assert the expected number of corpus declarations
  so skipped declarations fail the test.
- **Fix:** `scopesForSubstring`, `scopesAt`, `scopesAtLine`, and `tokenTextAt` take an
  `occurrence` argument (1-based; negative counts from the end) via a new `occurrenceIndex` helper,
  and the return-type assertion uses `'Element', -1` — verified to resolve a different token than
  occurrence 1. The corpus test derives declaration ends from tokenizer state
  (`meta.definition.component.nx` / `meta.definition.function.nx`) instead of the first textual
  `/>`, and its start regex no longer requires `<` on the keyword line, which brings `ui.nx:182`
  into the sweep (11 declarations before, 12 now). It also asserts that every detected start is
  actually scoped as a declaration and that no range collapsed to its keyword line, so a skipped
  declaration fails rather than passing silently.
- **Verification:** Verified. The return-type assertion now selects the last `Element`; the corpus
  detector finds all 12 current component declarations, including the split declaration at line
  182, and declaration ends come from active tokenizer scopes rather than the first textual `/>`.
  The corpus regression passes.

### 🔴 Open - RF7 Manual editor verification is still incomplete
- **Severity:** Low
- **Evidence:** Task 6.5 remains unchecked (`tasks.md:128`), and `openspec instructions apply`
  reports 39 of 40 tasks complete. The requested visual checks for comment `null` and numeric `0.0`
  therefore have not been recorded as complete.
- **Recommendation:** Run the extension-host visual check and mark task 6.5 complete, or explicitly
  document why the manual check is being deferred.
- **Status:** Still not addressed, for the same reason as the last pass: task 6.5 launches the
  VS Code extension host and this environment has no GUI. The programmatic equivalents remain in
  place (`declarations.test.ts` pins comment `null` and numeric `0.0`), and
  `docs/scratch-highlighting.nx` §10 now also carries the control-form iterable and the unscoped
  split-binder layout, so the whole change is visually checkable in one file. Recommend the author
  run `pnpm --dir src/vscode run vscode:launch` and tick 6.5, or record the deferral in `tasks.md`.

## Questions
- None.

## Summary
- RF1–RF6 and RF8–RF10 are verified. RF7 remains open pending the GUI-only visual check.
- `pnpm --dir src/vscode test` passes all 239 tests, `pnpm --dir src/vscode run compile` passes,
  `NPM_PACKAGE_VERSION=0.0.0-review pnpm --dir src/vscode run package:language` passes,
  `openspec validate fix-component-signature-highlighting --strict` passes, and both staged and
  unstaged diff checks are clean.

## Fix pass (pre-verification notes)
- RF1–RF6 are fixed and await verification by the reviewer. RF7 (the extension-host visual check)
  is left open: it needs a GUI this environment does not have.
- Each of the six was reproduced against the pre-fix grammar before being changed. Reverting the
  grammar to its pre-fix state fails exactly the seven new or extended assertions and nothing else,
  so the new tests have teeth.
- `pnpm --dir src/vscode test` now passes 221 tests (215 before; six added).
- **Status:** Not addressed — task 6.5 requires launching the VS Code extension host, which this
  environment has no GUI for. The programmatic equivalent is covered: `declarations.test.ts` asserts
  `null` inside `// >= 1; null` is `comment.line.double-slash.nx` and not `constant.language.null.nx`,
  and that `0.0` in `letterSpacing: float64 = 0.0` is `constant.numeric.float.nx`. Recommend the
  author run `pnpm --dir src/vscode run vscode:launch` and tick 6.5, or record the deferral in
  `tasks.md`.

## New Findings Discovered During 2026-09-02 03:36 Review

### ✅ Verified - RF8 The loop-header rule drops scopes for legal multiline and compound iterables
- **Severity:** Medium
- **Evidence:** Both loop contexts use one `\G` match that requires the binders and `in` to occur in
  the same tokenized line and only captures a leading qualified identifier from the iterable
  (`src/vscode/syntaxes/nx.tmLanguage.json:1676`, `:1783`). NX treats whitespace and comments as
  extras and admits a full `ValueExpression` as the iterable (`crates/nx-syntax/grammar.js:11`,
  `:620`, `:723`). Consequently, in `for item,` followed by `index in items`, `item`, the comma,
  `index`, and `items` have only source/meta scopes; in `for item in (items)`, `items` is likewise
  unscoped; and in `for item in left + right`, only `left` receives
  `variable.other.readwrite.nx`. The regression at
  `src/vscode/test/grammar/control-blocks.test.ts:91` covers only single-line headers and a list of
  string literals, so all 229 tests still pass despite the gap. This falls short of the changelog
  claim that a loop's binding variables and iterable are scoped and the requirement at
  `specs/editor-syntax-highlighting/spec.md:199`.
- **Recommendation:** Model the header with a shared stateful context used by `elements-for` and
  `value-for`, so binder/comma/`in` scopes survive legal trivia and line breaks, then delegate the
  complete iterable to expression rules rather than capturing only its first identifier. Add
  multiline/comment-separated headers and parenthesized or binary iterables to the tokenizer tests.
- **Fix:** The single `\G` match is replaced by `#loop-header`, one context shared by `value-for`
  and `elements-for`, running from after `for` to the body brace. Binders and iterable names carry
  the same scope, so the region needs no state beyond `in`, and the iterable is delegated to the
  ordinary expression patterns rather than captured. `for item,` / `index in items`,
  `for item /* each */ in items`, `for item in (items)`, and `for item in left + right` all scope
  fully now.
- **Note:** The begin cannot be a bare `\G`. `for` also occurs in prose, and an unguarded header
  opened on `an easier way for neighbors to share tools`
  (`src/vscode/samples/tally-survey.nx:327`) and scoped every following word as a binding variable
  — caught by the whole-corpus token diff, not by the suite. The begin now requires text actually
  shaped like a header: `name in`, `name, name in`, `name,` at a line break, or a comment after the
  first binder. One legal layout stays unscoped as a result — a break between a sole binder and
  `in`, which is not distinguishable from prose — and a regression pins the prose case.
- **Tests:** Three added to `control-blocks.test.ts` (split header with a comment, compound
  iterable, prose guard). The first two fail against the pre-fix grammar; the third guards the new
  rule.
- **Verification:** Reopened. The added split-header, comment, parenthesized/binary-iterable, and
  prose regressions pass, but the fix is not complete. Its own note leaves the legal `for item`
  followed by `in items` layout unscoped: both `item` and `items` carry only source/meta scopes.
  More importantly, `#loop-header` does not actually delegate the full `ValueExpression`; it lists
  only comments, literals, names, and operators and ends before the first brace
  (`src/vscode/syntaxes/nx.tmLanguage.json:2409`). In the valid iterable
  `for item in if ready { items } { <Row /> }`, `if` is incorrectly scoped
  `variable.other.readwrite.nx` and the inner `items` has no token scope. This conflicts with the
  unrestricted `ValueExpression` iterable in `crates/nx-syntax/grammar.js:625` and `:728`, the
  review recommendation to delegate the complete iterable, and the general requirement at
  `specs/editor-syntax-highlighting/spec.md:199`. Use expression-aware iterable handling (and
  separate guarded/unguarded header entry where prose ambiguity requires it), then cover the
  sole-binder line break and a control-form iterable.
- **Fix (2nd pass):** `#loop-header` now delegates the iterable's control forms to `#value-if` and
  `#value-for`, listed ahead of its name rule. A child begin that starts earlier in the line beats
  the header's `(?=[{}])` end, so the conditional consumes its own braces first and the end then
  lands on the body brace. In `for item in if ready { items } else { fallback } { <Row /> }`, `if`
  and `else` are `keyword.control.conditional.nx`, both branches are scoped by the conditional's own
  patterns, and the loop body keeps `meta.control.loop.value.nx` instead of escaping at the
  branch's `}` — which is what it did before, a defect the reopen did not name.
- **Status (sole-binder line break):** Not fixed; recorded as an accepted limitation rather than
  left silent. It is not a rule-ordering problem: a TextMate `begin` sees one line, and prose
  reaches `#value-for` mid-sentence — `docs`-free evidence is in the corpus itself, where
  `src/vscode/samples/tally-survey.nx:327` puts a whole paragraph inside
  `meta.control.loop.value.nx` even before this change. So `for item` at a line end is
  indistinguishable from a wrapped sentence ending in `for item`, and relaxing the guard was tried:
  an unguarded `\G` header re-scopes ~60 prose words in that sample as binding variables. The loop
  itself is still recognized in that layout; only the sole binder goes unscoped. Documented in
  `design.md`, in the grammar rule's own comment, in `CHANGELOG.md`, and pinned by a regression that
  also asserts the loop body still carries the loop scope. Reopening this would mean fixing prose
  tokenization in text blocks, which is a pre-existing defect outside this change.
- **Tests:** Two added to `control-blocks.test.ts` — the control-form iterable (fails against the
  pre-fix grammar with `if` scoped as a name) and the documented split-binder limitation. A new
  spec scenario, "A control form as the iterable", covers the fix.
- **Verification (2nd pass):** Reopened. The control-form half is fixed: independent value-loop and
  elements-loop probes scope `if`/`else` as conditional keywords, keep branch names in the
  conditional context, and keep the outer body in its loop meta scope. The remaining legal
  sole-binder split still prevents the finding from being verified. In `for item` followed by
  `in items {`, both `item` **and** `items` have only source/meta scopes; the new regression asserts
  only that `item` lacks `variable.other.readwrite.nx` and does not check `items`. This also makes
  the design and changelog statement that only the binder remains unscoped inaccurate. Recording
  the behavior as an accepted TextMate limitation does not satisfy the normative requirement that
  the grammar SHALL scope a loop's binding variables and iterable
  (`specs/editor-syntax-highlighting/spec.md:199`). Either disambiguate loop entry from prose (for
  example through context-specific text handling) or explicitly revise the requirement if this
  limitation is intended to be accepted.
- **Verification (3rd pass):** Independent re-check; status unchanged at Open, and both points of
  the 2nd-pass verification are confirmed. The control-form half is correct and complete on both
  loop paths — probes covering a value-position loop, a top-level `elements-for`, an `if … else`
  iterable, an `if … is { … }` match iterable, a nested `for` iterable, and an unterminated `if`
  iterable followed by a new declaration all scope as intended, and the pre-existing single-line,
  comma, comment, parenthesized, binary, and list-literal headers are unchanged (whole-corpus token
  diff over 95 tracked `.nx` files: zero lines). The `items` observation is confirmed: in the split
  layout the iterable is unscoped too, so the `design.md` and `CHANGELOG.md` wording that only the
  binder goes unscoped is wrong and the new regression under-asserts.
- **Verification (3rd pass), on the deferral itself:** The recorded justification tests the wrong
  relaxation and should not be relied on. Two experiments:
  1. The narrow relaxation — adding only a `name` at end-of-line alternative to the existing guard,
     not an unguarded `\G` — produces a **zero-line whole-corpus diff** and fixes the layout. It is
     still wrong: on a hand-built rewrap of the same sentence, where a line ends in
     `… an easier way for neighbors`, it scopes 16 prose words across two paragraphs as binding
     variables. So the corpus check cannot validate this class of change, and "the guard must stay
     as it is" does not follow from the unguarded-`\G` experiment.
  2. A bounded variant does work. A second context entered only on a sole binder at end of line,
     whose `end` also pops at the start of any line that does not begin with `in`, scopes `item`,
     `in`, `items`, and the body brace in the legal layout, bounds the worst case in prose to the
     single word after `for`, and leaves the corpus untouched apart from the intended scratch-file
     line. Note the end must be written `(?=^(?![ \t]*in(?![A-Za-z0-9_-])))`; the natural
     `(?=^[ \t]*(?!in…))` backtracks the whitespace run to zero width and pops immediately — the
     same greedy-quantifier-then-negative-lookahead shape task 6.1 exists to catch, and it silently
     costs the iterable's scope.
  The choice is therefore not "the guard or a prose disaster". Either adopt a bounded entry of this
  kind and drop the limitation, or keep the limitation and make it accurate everywhere: correct
  `design.md`, the rule comment, and `CHANGELOG.md` to say the whole header goes unscoped, extend
  the regression to assert `items` as well, and narrow the absolute requirement at
  `specs/editor-syntax-highlighting/spec.md:199` with a scenario that records the exception, since
  no scenario currently admits it.
- **Fix (3rd pass):** Took the first option — the limitation is gone rather than documented. A
  second entry, `#loop-header-split-binder`, opens on a sole binder at end of line, the shape the
  main guard cannot verify, and pays for the guess in its `end` instead of its `begin`: the header
  also pops at the start of any line that does not begin with `in`. In the legal layout the next
  line does begin with `in`, so `item`, the `in`, and `items` are all scoped; in prose that wraps
  after `for neighbors`, that one word is mis-scoped and the header closes before the next line.
  Both entries share `#loop-header-parts`, so the two headers cannot drift. The end is written
  `(?=^(?![ \t]*in(?![A-Za-z0-9_-])))` — the naive `(?=^[ \t]*(?!in…))` backtracks to zero width
  and pops before the iterable is scoped, which a mutation test now pins.
- **Fix (3rd pass), documentation:** The inaccurate claims are corrected, not just softened.
  `design.md` now describes the split entry, the end-pattern trap, and why a whole-corpus diff
  cannot validate a change here; `CHANGELOG.md` states the layout is scoped; the grammar comments
  are rewritten; the scratch file's §10 example is now a positive case. A new spec scenario, "A
  sole binder split from its `in`", covers both the scoped layout and the bounded prose cost, so
  the requirement at `spec.md:199` no longer has an unrecorded exception.
- **Tests:** The limitation test is replaced by two: the split layout now asserts `item`, `in`, and
  `items` (the under-assertion the 2nd-pass verification caught), and a prose regression asserts
  that no word after the first is scoped. Three mutations were checked and each fails exactly one
  test: dropping the rule, dropping the pop from its end, and using the naive end shape.
- **Verification:** Reopened on the 4th pass. The direct `for item` / `in items` layout is fixed in both
  value and elements loops: `item` and `items` are `variable.other.readwrite.nx`, `in` is
  `keyword.control.loop.nx`, and the body retains the correct loop meta scope. The bounded entry is
  still incomplete for legal trivia, however. With a blank line, `//` line comment, or block-comment
  line between `for item` and `in items`, `#loop-header-split-binder` deliberately pops before the
  `in` line and `items` again has only source/meta scopes. This conflicts with the general SHALL at
  `spec.md:199` and the scenario requiring the same scopes when a comment separates the binder from
  `in` (`spec.md:216`). The new prose guarantee is also too broad for the implementation: if prose
  wraps as `… for neighbors` followed by `in towns and cities`, the second line is indistinguishable
  from a header and scopes `towns` and `and` as `variable.other.readwrite.nx`, despite the scenario
  saying no word on following lines may receive that scope. The regression covers only a continuation
  beginning with `to`, so neither edge is detected. Preserve trivia while awaiting `in`, and either
  separate markup prose from code structurally or narrow the prose requirement and documentation to
  the ambiguity the grammar can actually bound.
- **Fix (4th pass):** Both points accepted; both reproduced against the grammar before changing it.
  Trivia is now preserved: `#loop-header-split-binder` pops at the start of the first line that does
  not *continue* the header — not blank, not a comment, and not beginning with `in` — instead of at
  the first line that does not begin with `in`. `for item` followed by a blank line, a `//` line, or
  a `/* … */` line and then `in items` now scopes the iterable, which settles the conflict with
  `spec.md:216`. A block comment spanning lines needed no alternative of its own: its own context is
  on the stack, so this end is not tested while it is open, and that case was checked rather than
  assumed.
- **Fix (4th pass), the prose guarantee:** The over-broad promise is narrowed to what the grammar
  bounds, and the residual is pinned instead of claimed away. `… for neighbors` / `in towns and
  cities` is a header in every visible respect and is scoped as one; what the implementation does
  guarantee is that the misreading stops at the end of that line rather than running to the next
  brace. The scenario is split in two accordingly: one for the non-`in` continuation, where only the
  word after `for` is scoped, and one stating where the `in` continuation stops. `design.md`, the
  rule comment, and `CHANGELOG.md` say the same thing.
- **Tests (4th pass):** Two added — blank, `//`, and multi-line block-comment trivia inside a split
  header, and the `in`-continuation residual asserting the following line keeps its prose scopes.
  Three mutations each fail only the tests they should: dropping the trivia alternatives fails the
  trivia test, dropping the pop fails both prose tests, and the backtracking-prone
  `(?=^[ \t]*(?!…))` shape fails three. Suite is 239 tests; the whole-corpus token diff is limited
  to the intended scratch-file lines.
- **Verification:** Verified. Independent probes confirm the direct sole-binder split and mixed
  blank, line-comment, single-line block-comment, and multi-line block-comment trivia retain the
  binder, `in`, iterable, and body scopes in both value and elements loops. A control-form iterable
  after that trivia also retains its conditional and enclosing loop scopes. For false prose opens,
  a non-`in` continuation receives no header-name scopes, while an indistinguishable `in`-initial
  continuation is scoped only through that line, matching the narrowed spec and design. The 239-test
  suite and package validation pass.

### ✅ Verified - RF9 Property condition-list names can still be stolen by declaration keywords
- **Severity:** Medium
- **Evidence:** The new `#literals` include correctly handles the outer simple-condition path, but
  the nested brace of `property-list-if` includes all of `#attributes`
  (`src/vscode/syntaxes/nx.tmLanguage.json:2071`). That context falls through to `#keywords-core`
  for a name not followed by assignment (`:1439`), before `#qualifiers` can scope the expression.
  Tokenizing `<Notice if { state => tone="danger" } />` therefore scopes `state` as
  `keyword.declaration.state.nx`, even though the spec says names that merely share keyword spelling
  keep their positional scope because only `true`, `false`, and `null` are reserved
  (`specs/editor-syntax-highlighting/spec.md:199`). The test at
  `src/vscode/test/grammar/control-blocks.test.ts:109` checks `state` only in the outer
  `if state is { ... }` path and does not exercise a condition-list arm.
- **Recommendation:** Reuse only the assignment-specific attribute patterns inside the nested
  property block, without the declaration-keyword fallback, and include `#literals` explicitly
  ahead of the expression-name rule. Add a condition-list regression that keeps `state` non-keyword
  while still scoping `type = "x"` as an attribute and `true` as a literal.
- **Fix:** `#attributes` is split into `#attribute-spread` and `#attribute-assignment`, and the
  nested brace of `property-list-if` now includes `#attributes-condition` — the same patterns with
  `#literals` in place of the `#keywords-core` fallback. `state` in
  `if { state => tone="danger" }` is `entity.name.qualifier.nx` again, while `tone` and `type` stay
  `entity.other.attribute-name.nx` and `true` stays `constant.language.boolean.nx`.
- **Also fixed:** Splitting the rule out exposed a second defect on the same path, present before
  this change too. An attribute ended only at a line break, `/`, or `>`, so it swallowed the `}`
  closing its arm and everything after tokenized one context too deep. In
  `if compact { density="tight" } else { density="normal" }` (`examples/nx/component.nx:43`, valid
  NX) the second `density` was `entity.name.qualifier.nx` and the element's `/>` on the next line
  was scoped as division and greater-than. `}` now ends an attribute; a braced or quoted value is
  unaffected, since its own child context consumes its `}` first.
- **Tests:** Two added to `control-blocks.test.ts`; both fail against the pre-fix grammar.
- **Verification:** Verified. In a condition-list arm, `state` now receives the positional
  qualifier scope rather than `keyword.declaration.state.nx`, while `tone`/`type` remain attributes
  and `true` remains a boolean literal. The adjacent arm-boundary fix also releases both attributes
  correctly and leaves the following `/>` scoped as tag punctuation.

### Review-pass summary
- Two new medium-severity findings were opened for the unstaged control-form grammar changes.
- `pnpm --dir src/vscode test` passes all 229 tests, `pnpm --dir src/vscode run compile` passes,
  `openspec validate fix-component-signature-highlighting --strict` passes, and staged and unstaged
  diff checks are clean.

## Fix pass (2026-09-02)

- RF1, RF8, and RF9 are fixed and await verification. RF7 remains open: it is a GUI-only check.
- RF1 needed no new work — the reopen described a state two later passes had already changed. Its
  two reproductions were run against the current grammar and both recover.
- Every change was checked with a whole-corpus token-stream diff over all 95 tracked `.nx` files
  plus the scratch file, comparing before and after. That is what caught the prose-`for` regression
  in RF8, which the green suite did not. The final diff is four files, all intended: list-literal
  commas now scoped in `examples/nx/complex.nx` and the scratch file, and the arm fix in
  `examples/nx/component.nx`.
- `pnpm --dir src/vscode test` passes 234 tests (229 before; five added), `pnpm --dir src/vscode
  run compile` passes, and `openspec validate fix-component-signature-highlighting --strict`
  passes.
- `docs/scratch-highlighting.nx` §10 gained the split header, the compound iterable, and the
  condition arms, so the three fixes are visually checkable alongside the rest.

## Fix pass (2026-09-02, second)

- RF8 is fixed for the control-form iterable and awaits verification. Its sole-binder-line-break
  half is recorded as an accepted limitation, with the corpus evidence for why the guard has to
  stay; see the Status note under RF8.
- RF7 remains open. It is a GUI-only check this environment cannot run.
- The change was checked with the same whole-corpus token-stream diff over all 95 tracked `.nx`
  files plus the scratch file: the grammar edit alone produces a zero-line diff, and the only file
  that differs afterwards is `docs/scratch-highlighting.nx`, which gained the two new §10 examples.
- `pnpm --dir src/vscode test` passes 236 tests (234 before; two added), `pnpm --dir src/vscode run
  compile` passes, and `openspec validate fix-component-signature-highlighting --strict` passes.
- Reverting just the two new `#loop-header` includes fails exactly the new control-form test and
  nothing else.

## Verification pass (2026-09-02, third)

- No finding was in the `🟡 Fixed` state at the start of this pass: a concurrent verification had
  already processed RF8's second fix and reopened it. This pass re-verified that work independently
  rather than re-running the same assertions, and added measured evidence for the half that is
  still open.
- Verified as correct and complete: the control-form iterable delegation in `#loop-header`.
- Still open: RF8 (sole-binder split header, plus the inaccurate documentation and under-asserting
  regression that go with it) and RF7 (the GUI-only extension-host check, task 6.5).
- `pnpm --dir src/vscode test` passes 236 tests, and the whole-corpus token diff is clean.

## Fix pass (2026-09-02, third)

- RF8 is fixed and awaits verification. Both halves of the reopen are now addressed: the
  control-form iterable in the previous pass, and the sole-binder split header in this one.
- The deferral recorded in the previous pass is withdrawn. The verification experiments showed the
  choice was never "the guard or a prose disaster": bounding the cost in the `end` fixes the layout
  and caps a false open at one word.
- Whole-corpus token diff over 95 tracked `.nx` files plus the scratch file: the only differences
  are in `docs/scratch-highlighting.nx`, and they are the intended ones — the rewritten §10 comment
  and the `straggler`/`items` tokens that are now scoped.
- `pnpm --dir src/vscode test` passes 237 tests (236 before; one net new after replacing the
  limitation test), `pnpm --dir src/vscode run compile` passes, and
  `openspec validate fix-component-signature-highlighting --strict` passes.
- RF7 remains open; it is still the GUI-only check.

## Fix pass (2026-09-02, fourth)

- RF8 is fixed again and awaits verification. The 4th-pass verification was right on both counts:
  trivia between the binder and `in` was a genuine spec conflict, and the prose scenario I wrote
  promised more than the grammar can deliver.
- The trivia gap is closed in the rule; the prose promise is narrowed to the bound that actually
  holds, with the residual ambiguity written down and pinned by a regression rather than left for
  the next reviewer to rediscover.
- RF7 remains open. It is still the GUI-only extension-host check.
- `pnpm --dir src/vscode test` passes 239 tests, `pnpm --dir src/vscode run compile` passes,
  `openspec validate fix-component-signature-highlighting --strict` passes, and the whole-corpus
  token diff over 95 tracked `.nx` files plus the scratch file shows only the intended
  scratch-file changes.

## New Findings Discovered During 2026-09-02 17:40 Verification

### ✅ Verified - RF10 Changelog overstates the split-header prose bound
- **Severity:** Low
- **Evidence:** The updated spec and design correctly record the unavoidable case where prose wraps
  as `… for neighbors` followed by `in towns and cities`: the second line is indistinguishable from
  a loop header and its names receive `variable.other.readwrite.nx`, with the split-header context
  ending before the following line. The changelog instead says without qualification that prose
  wrapping after `for <word>` “mis-scopes that one word and stops there”
  (`src/vscode/CHANGELOG.md:79`). A direct tokenizer probe scopes both `towns` and `and` as loop
  names, so the downstream-facing description contradicts the documented and tested behavior.
- **Recommendation:** Amend the changelog bullet to mention that an `in`-initial continuation is
  necessarily treated as a header and that its header-name scoping stops after that continuation
  line.
- **Fix:** Confirmed first — `control-blocks.test.ts:258` already asserts that `towns` is scoped
  `variable.other.readwrite.nx` in `… for neighbors` / `in towns and cities`, so the changelog was
  the only artifact out of step. `src/vscode/CHANGELOG.md:78-82` now states both cases: prose
  wrapping after `for <word>` mis-scopes that one word, an `in`-initial continuation line is
  indistinguishable from a header and has its names scoped too, and in both cases the misreading
  ends at the first line that does not continue the header rather than running to the next brace.
  This matches `spec.md:232-236` and the design note. 239 tests pass and
  `openspec validate --strict` passes.
- **Verification:** Verified. The changelog explicitly describes both prose-continuation shapes and
  their actual stopping point, matching the specification, design, tokenizer fixture, and current
  grammar behavior. The VS Code extension's 239 tests and compile step pass, strict OpenSpec
  validation passes, and both staged and unstaged diff checks are clean.

## Fix pass (2026-09-02, fifth)

- RF10 is fixed and awaits verification. The finding was correct: the spec, design, and regression
  test all recorded the `in`-continuation residual, but the changelog bullet — the artifact
  downstream theme and Monaco consumers actually read — still claimed the misreading was limited to
  one word. It now describes both continuation shapes and the bound that holds for each.
- No grammar change was needed or made; this pass touched documentation only.
- RF7 remains open. It is still the GUI-only extension-host check.
- `pnpm --dir src/vscode test` passes 239 tests and
  `openspec validate fix-component-signature-highlighting --strict` passes.
