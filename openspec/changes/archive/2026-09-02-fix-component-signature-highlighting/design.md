## Context

See proposal.md — Why. Additional state that shapes the approach:

- `src/vscode/syntaxes/nx.tmLanguage.json` is a hand-maintained TextMate grammar with no build step.
  It is published three ways — inside the `nx-language` VSIX, as `@nx-lang/language/grammar` for
  Monaco/Shiki consumers, and by delegation from `nx.markdown.codeblock.tmLanguage.json` — so it
  must remain a standalone TextMate grammar. Semantic tokens from `nx-lsp` are not an option; the
  extension does not implement a semantic token provider today, and two of the three consumers have
  no language server at all.
- The grammar reuses one context, `start-tag`, for two syntactically similar but semantically
  different constructs: an element *reference* (`<Button x=1 />`, where `x` is an attribute) and an
  element-shaped *declaration signature* (`component <Text format: TextFormat = plain />`, where
  `format` is a property definition). The declaration rule `meta.definition.component.nx` was only
  ever reachable for a single-line, default-free signature, so in practice nearly every real
  declaration falls through to `start-tag`. Most of the reported defects are a consequence of that
  fall-through.
- `crates/nx-syntax/grammar.js` (tree-sitter) is the authoritative token inventory. Diffing its
  keyword literals against the TextMate grammar is what surfaced `external`, `emits`, `state`, and
  `content` as missing; that diff is worth keeping as a check rather than a one-time exercise.
- Existing grammar tests live in `src/vscode/test/grammar/` and assert scopes via
  `grammar.tokenizeLine(line, null)` — single lines, from a fresh rule stack. That shape structurally
  cannot catch either of the two worst defects here, both of which are multi-line and
  state-dependent.

## Goals / Non-Goals

**Goals:**

- One grammar context that owns element-shaped declaration signatures, distinct from the element
  reference context, so declaration and reference scoping can diverge where the language does.
- Declaration scoping that is a pure function of the declaration text — independent of layout and of
  what precedes it in the file.
- Test infrastructure that can express multi-line and whole-file assertions, since the requirements
  in `specs/editor-syntax-highlighting/spec.md` are largely about those.

**Non-Goals:**

- Rewriting or restructuring the element reference, expression, `if`/`for`, or markup-text contexts.
  Their existing tests must keep passing unchanged. The one exception is pattern *order* inside
  `attributes`, which is a defect rather than a restructuring — see "Let the attribute rule outrank
  the keyword rule" below.
- Changing the scope names already asserted by existing tests (`entity.name.tag.nx` for element
  references, `keyword.operator.conditional.nx` for the ternary `?`, `keyword.operator.type-modifier.nx`
  for `[]`, `variable.other.property.nx` for record fields). New scopes are introduced only where
  something is currently unscoped or provably mis-scoped.
- Adding a semantic token provider to `nx-lsp`.
- Updating `nx-grammar.md`, whose `ComponentDefinition` production omits `abstract` and `external`.
  That is a real documentation gap but a separate change; `nx-grammar-spec.md`, `grammar.js`, and
  `crates/nx-syntax/src/validation.rs` all agree the modifiers exist, so this change follows them.

## Decisions

### Give element-shaped declarations their own context instead of patching `start-tag`

The declaration path becomes a `declaration-signature` context, entered from both the component
definition rule and the element-shaped `let` function rule, and `start-tag` is left to element
references.

The two constructs disagree on almost every token: the leading name is a declaration in one and a
reference in the other; `name: Type` is a property definition in one and does not occur in the
other; `name=value` is an attribute in one and does not occur in the other; `extends`, `emits`, and
`content` appear only in declarations. Trying to serve both from `start-tag` is what produced
`support.type.text.nx` — a scope that exists to paint `:Type` in typed inline content and was
pressed into service for property annotations, which is where the colon-folding and the
backtracking lookahead both come from.

Alternative considered: keep `start-tag` shared and disambiguate with lookaheads at each pattern.
Rejected — the disambiguation would have to be repeated in every nested pattern, and the current
grammar is already at the limit of what is maintainable by hand.

### Anchor the signature context on `<` … `/>`, not on `(?==)`

`meta.definition.component.nx` currently ends at the first `=`, which for a defaulted property list
is the first default value. The new context begins at the signature's `<` and ends at its `/>`,
matching the `ComponentSignature` production. The declaration's modifiers and keyword are captured
before entering it, so the signature context does not need to re-derive them.

This is also what makes the "declaration scopes end at the declaration's terminator" requirement
achievable: with a real terminator, a runaway signature can only survive to the end of the file if
the `/>` is genuinely absent.

### Hoist the annotation lookahead out of the identifier

Replace `":\\s*(?:(?!(?:raw|if)\\b)(?:[A-Za-z_][A-Za-z0-9_-]*)(?!\\s*=))?"` with separate patterns
for the colon and the type, and assert the identifier boundary before the negative lookahead:
`[A-Za-z_][A-Za-z0-9_-]*(?![A-Za-z0-9_-])`. Without the boundary assertion the engine satisfies
`(?!\s*=)` by giving back one character, which is the entire cause of the `TextForma` + `t` and
`float6` + `4 = 0.0` splits.

The `(?!\s*=)` guard existed to stop an attribute name from being mistaken for a type. In the
declaration context that guard is unnecessary — a property definition always has a type after its
colon — so it can be dropped there rather than repaired. It stays, corrected, wherever the typed
inline content path still needs it.

### Add the missing keywords from the tree-sitter inventory, and keep the two in sync by test

`external`, `emits`, `state`, and `content` get scopes (`storage.modifier.external.nx`,
`keyword.declaration.emits.nx`, `keyword.declaration.state.nx`, `storage.modifier.content.nx`),
chosen to parallel the existing `storage.modifier.abstract.nx` and `keyword.declaration.*` families.

Rather than trusting that the next keyword addition will be remembered, add a test that extracts the
keyword literals from `crates/nx-syntax/grammar.js` and asserts each one is matched somewhere in the
TextMate grammar. This is the check that would have caught `external` when it was introduced.

Alternative considered: generating `nx.tmLanguage.json` from the tree-sitter grammar. Rejected as
disproportionate — the two grammars have genuinely different jobs, and a lint-style test gets the
regression protection at a fraction of the cost.

### Scope primitives as `support.type`, not `storage.type`

`string`, `int`, `float64` and the rest move from `storage.type.primitive.nx` to
`support.type.primitive.nx`, matching what TypeScript's grammar does (verified: TypeScript scopes
`string` as `support.type.primitive.ts` and a user-defined type as `entity.name.type.ts`).

`storage.type` is for a type that is itself the declarator — C's `int x = 5`, where the type keyword
occupies the declaration slot and is therefore coloured with the keywords. NX never uses primitives
that way: they appear only in annotation position after a colon, with `let`/`type`/`component` as
the declarator. The C rationale does not transfer; TypeScript's does, because the syntax is the same
shape.

The reader-facing effect is that `label: string` and `format: TextFormat` — the same kind of thing
in the same position — stop being coloured as if one were a keyword, and the keyword colour goes
back to meaning control flow and declarations. The distinction is not lost: `support.type.primitive.nx`
and `entity.name.type.nx` remain separate scopes, so a theme that wants to tell built-in from
user-defined still can; most simply choose not to. The grammar already uses this family for
`support.type.text.nx`, so the naming is consistent with what is there.

No backward-compatible alias is kept for the old scope name.

### Keep the three declaration-start lookaheads duplicated, but pin them with tests

`(?=^\s*(?:(?:private|export)\s+)?(?:(?:abstract)\s+)?(?:type|action|component|let)\b)` appears at
three places in the grammar and must gain `external`. TextMate JSON has no way to share a regex
fragment, and introducing a build step to get one is out of proportion to the problem. Instead, each
of the three sites gets a test that an open construct of that kind is terminated by each declaration
form, so a future divergence fails loudly rather than silently swallowing the rest of a file.

### Let the attribute rule outrank the keyword rule

`attributes` listed `#keywords-core` before the `name = value` rule. TextMate breaks a same-position
tie by list order, so in `<Question type = "multiple" />` the keyword rule claimed `type` as
`keyword.declaration.type.nx`; the attribute rule's `(?=name\s*=)` lookahead then no longer matched,
so the `= "multiple"` was left unscoped as well. Moving `#keywords-core` after the attribute rule
fixes both halves at once.

This is safe because the attribute rule only claims a name that is followed by `=`, which is exactly
the position where a keyword cannot be a keyword. A keyword anywhere else in a start tag still falls
through to `#keywords-core`. Verified against every `.nx` file in the repository: the only tokens
that changed are the keyword-named attributes themselves and the values that had been left unscoped
behind them.

### Scope a control form's own names, but leave the qualifier catch-all alone

`#qualifiers` ends in a bare-identifier catch-all scoped `entity.name.qualifier.nx`. That is the
wrong scope for a variable reference — `ready` in `if ready { … }` is not a module qualifier — but
it cannot simply be rescoped, because the same catch-all is what markup prose falls through to:
`Social media` inside `<Option>Social media</Option>` is scoped `entity.name.qualifier.nx` too
(`src/vscode/samples/survey.nx:21`). No theme styles that scope, so prose renders as plain text by
accident. Repointing the catch-all at a variable scope would colour prose as variables, which is
worse than the defect. Separating the two needs the markup-text contexts reworked, which is a
non-goal here.

What is fixable without touching the catch-all is anything positional:

- A `for` header is `#loop-header`, a context shared by `value-for` and `elements-for`, so the
  binders and the iterable get `variable.other.readwrite.nx`. They previously had no token scope at
  all.
- `if true { … }` in a property list scoped `true` as a qualifier. A property list holds no prose,
  so a literal rule can outrank the catch-all there.

The literal rule is `#literals` — `true`, `false`, `null` — split out of `#keywords-core`, which
`keywords-core` now includes. Including all of `keywords-core` was tried first and was wrong:
`state` is a legal parameter name (`examples/nx/component.nx:39`) and `type` a legal attribute name,
so the declaration keywords cannot be recognised outside declaration positions. Only the three
reserved literals can.

The same mistake survived one level down. A property-list condition arm included all of
`#attributes`, whose last pattern is that same `#keywords-core` fallback, so `state` in
`if { state => tone="danger" }` was scoped `keyword.declaration.state.nx`. `#attributes` is now
split into `#attribute-spread` and `#attribute-assignment`, and an arm includes
`#attributes-condition` — the same patterns with `#literals` in place of the keyword fallback.

Splitting the rule out surfaced a second defect in the same path. An attribute ended only at a line
break, `/`, or `>`, so it swallowed the `}` that closed its arm and everything after it tokenized
one context too deep: in `if compact { density="tight" } else { density="normal" }`
(`examples/nx/component.nx:43`) the second `density` was a qualifier and the element's `/>` was
scoped as division and greater-than. `}` now ends an attribute too. A braced or quoted value is
safe, because its own child context consumes its `}` before the attribute's end is tried.

#### The loop header is a context, not a match

The header was first written as one `\G`-anchored match. That works only when the whole header
fits on one tokenized line with nothing between its parts, and NX treats whitespace and comments as
extras: `for item,` / `index in items` and `for item /* each */ in items` both lost every scope in
the header. A match also captures only what it names, so only the first identifier of the iterable
was scoped — `for item in left + right` left `right` bare.

As a begin/end context ending at the body brace, the header keeps its scopes across a line break
and delegates the iterable to the ordinary expression patterns, whatever its shape. Binders and
iterable names carry the same scope, so the region needs no state beyond `in` itself.

The begin cannot be a bare `\G`, for two reasons. A zero-width begin risks a push/pop that never
advances, and — the one that actually bit — `for` occurs in running prose. `An easier way for
neighbors to share tools` (`src/vscode/samples/tally-survey.nx:327`) opened the header and scoped
every following word as a binding variable. The begin therefore requires text actually shaped like
a header: `name in`, `name, name in`, `name,` at a line break, or a comment after the first binder.
That guard also guarantees the begin consumes the whitespace after `for`, so it always advances. It
does not admit a break between the sole binder and `in` — `for item` alone at end of line is
indistinguishable from prose, so that layout needs its own entry.

`#loop-header-split-binder` is that entry, and it is the interesting one. It opens on a sole binder
at end of line — the shape the guard cannot verify — and pays for the guess in its `end` rather
than at its `begin`: the header pops at the start of the first line that does not continue it — one
that is not blank, not a comment, and does not begin with `in`. In the legal layout the next line
does begin with `in`, so the binder, the `in`, and the iterable are all scoped, and NX's trivia
between them is preserved. A block comment spanning lines needs no alternative of its own, since its
own context is on the stack and this end is not tested while it is open. In prose that wraps after
`for neighbors`, exactly one word is mis-scoped before the header pops, instead of every word up to
the next brace. Both halves share `#loop-header-parts`.

One case stays ambiguous by construction: prose that wraps as `… for neighbors` / `in towns and
cities` is a header in every visible respect, and is scoped as one. The pop is what makes that
survivable — the misreading ends with that line instead of running to the next brace — so the
guarantee worth stating, and the one the spec states, is about where it stops rather than that it
never happens.

Two things make this easy to get wrong. The end must be written
`(?=^(?![ \t]*in(?![A-Za-z0-9_-])))`, not `(?=^[ \t]*(?!in…))`: the latter lets `[ \t]*` backtrack
to zero width, tests the lookahead against the leading space, and pops before the iterable is
scoped — the greedy-quantifier-then-negative-lookahead shape task 6.1 exists to catch. And a
whole-corpus token diff cannot validate a change here: a relaxation that merely widens the guard to
accept a sole binder at end of line leaves the corpus byte-identical while scoping whole paragraphs
of a rewrapped sample as binding variables. The prose cost has to be measured on prose written for
the purpose, which is what the regression beside the split-header test does.

The iterable is a full `ValueExpression`, so it may be a control form. Listing only names, literals,
and operators in the header made `for item in if ready { items } { … }` scope `if` as a binding
variable, leave the branch unscoped, and — worse — end the loop at the branch's `}`, so the body
escaped the loop context entirely. The header therefore delegates to `#value-if` and `#value-for`
ahead of its name rule. Order does the work: a child begin that starts earlier in the line beats the
`(?=[{}])` end, so the conditional consumes its own braces first and the end then lands on the
body brace, which is where it belongs.

### Recover at the next declaration, with two lookahead shapes

A declaration whose signature or body is never terminated must not swallow the rest of the file, so
every context that can outlive its line carries a declaration-start lookahead in its `end`. The
lookahead has to tell a new declaration from three things that look like one:

- **An attribute or property named after a keyword.** `type = "multiple"`
  (`src/vscode/samples/tally-survey.nx:11`) is a legal attribute. Every recovery lookahead therefore
  ends `(?!\s*[:=])`: an attribute name is always followed by `=` or `:`, and a declaration keyword
  never is.
- **An indented nested binding.** `let myOptions = { … }` inside a braced expression
  (`src/vscode/samples/tally-survey.nx:120`) is a binding, not a new declaration. Only that one
  form is ambiguous, so only that one form is restricted. Inside an *expression* context — a braced
  or embed expression, an `if`/`for` form, a start tag, a right-hand side — the lookahead splits
  three ways: `type`, `action`, and `component` recover at any indentation, because they have no
  production outside the module top level (`crates/nx-syntax/grammar.js`, `source_file`); a `let`
  carrying a visibility, `abstract`, or `external` modifier recovers at any indentation, because a
  binding never carries one; and a bare `let` recovers only at column 0. Declaration contexts — a
  signature, a property definition, a record or action body, a union case, an `emits`/`state`
  group — keep a plain `^\s*` for all four keywords, since nothing can nest there.
- **Literal text.** The three text-content contexts get no recovery at all; `//` and `type` are
  literal text there, per `nx-grammar-spec.md`.

Alternative considered: a single lookahead everywhere. Rejected — anchoring all of them at `^\s*`
breaks the nested bindings that exist in the sample corpus today, and anchoring all at `^` stops an
indented top-level declaration from terminating a preceding construct, which the "Declaration
scoping is independent of file position" requirement forbids. Splitting by declaration form costs
one more alternative in one lookahead and gives up nothing: the only case that stays column-0 is the
one the language genuinely makes ambiguous.

### Test at three levels

1. **Line-level scope assertions**, as today, for the single-line cases.
2. **Multi-line assertions** via a helper that carries the rule stack across lines — the existing
   suites always pass `null`, which is why every multi-line defect here went unnoticed. The helper
   also needs a negative form (`assert scope NOT present`), because several requirements are stated
   negatively: a comment's `>` must not be tag punctuation, a type's last character must not be an
   attribute name.
3. **Corpus regression** over `docs/drawnui-proposal/ui/ui.nx`, tokenized end to end, asserting the
   position-independence requirement directly: each component declaration is tokenized standalone and
   in situ, and the two token streams must agree. This is a stronger and more durable assertion than
   listing the scopes expected at particular lines, and it is exactly the invariant the runaway union
   violated.

## Risks / Trade-offs

- **Splitting `start-tag` regresses element reference highlighting** → The element reference tests in
  `basic.test.ts` and `text-elements.test.ts` are the guard; they must pass unchanged, and the
  refactor should move patterns out of `start-tag` rather than rewriting them.
- **New scopes render as unstyled in some themes.** `storage.modifier.external.nx` and
  `keyword.declaration.emits.nx` fall back to `storage.modifier` and `keyword` in any reasonable
  theme, but a theme keyed on exact scope strings would show them uncolored → Mitigated by choosing
  names inside families the grammar already uses, so the fallback is always to a well-styled parent.
- **Downstream consumers see different colors after upgrading.** The VSIX, the npm assets package,
  and the markdown code-block grammar all change together → This is the intended outcome, but it is a
  visible change to published assets and belongs in the extension changelog.
- **TextMate backtracking has more surprises than the one found.** The annotation lookahead is
  unlikely to be the only greedy-quantifier-plus-lookahead in the grammar → Audit the grammar for
  the same shape (`[A-Za-z0-9_-]*` immediately followed by a negative lookahead) while making this
  change, and fix any others found the same way.
- **The corpus test couples the grammar suite to a proposal document.** If
  `docs/drawnui-proposal/ui/ui.nx` is deleted or restructured, the test breaks for an unrelated
  reason → Keep the position-independence assertion generic (compare standalone vs in-situ token
  streams) so it can be repointed at any `.nx` file, and skip cleanly if the file is absent.
