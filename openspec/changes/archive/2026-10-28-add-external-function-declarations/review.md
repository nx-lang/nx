# Review: add-external-function-declarations

Reviewed 2026-10-28 03:06 EDT (America/New_York).

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, validation.md,
specs/external-functions/spec.md, specs/runtime-standard-functions/spec.md; main specs
openspec/specs/runtime-standard-functions/spec.md and openspec/specs/standard-libraries/spec.md for
post-archive consistency.
**Reviewed code:** the external-function part of the working tree and index, kept apart from the
archived add-math-standard-library work that is staged with it:
crates/nx-syntax (grammar.js, generated grammar.json/node-types.json/parser.c, queries/highlights.scm,
ast.rs, tests/external_functions.rs, validation.rs recovery messages), crates/nx-hir (lower.rs
`lower_external_function_body` / `lower_with_runtime_functions`, tests/external_functions.rs),
crates/nx-api (artifacts.rs standard-library module registry, std/math/math.nx,
standard_library_tests.rs), crates/nx-language-service/src/lib.rs (alias member hover, `external`
keyword completion, math hover/completion tests), src/vscode/syntaxes/nx.tmLanguage.json (which
this change does not modify), sites/website docs (reference/syntax/functions.md,
reference/syntax/modules.md, reference/libraries/math.md), nx-grammar.md,
specs/ir-conformance (math-library fixtures, README size-budget note, manifest).
**Validation run by reviewer:** `cargo test -q -p nx-syntax -p nx-hir -p nx-api -p nx-language-service`
(all pass); `cargo test -q -p nx-codegen -p nx-ir -p nx-ir-runtime -p nx-interpreter -p nx-value`
(all pass, including the IR corpus without `NX_UPDATE_CORPUS`); `pnpm test:grammar` in src/vscode
(287 passing); TypeScript runtime tests run against the existing `dist/` after confirming that a
fresh `tsc` build into a temp dir matches `dist/` byte for byte (all pass, including
`math: 184 reference cases`); `openspec validate add-external-function-declarations --strict`
(valid). Behavior probes used an out-of-repo crate (`/tmp/nxprobe/rs`) linked to the workspace
crates, plus a vscode-textmate tokenizer script (`/tmp/nxprobe/tm.cjs`).

## Findings

### ✅ Verified - RF1 The VS Code/website TextMate grammar does not recognize `external let` function declarations
- **Severity:** Medium
- **Evidence:** `value-definition` in src/vscode/syntaxes/nx.tmLanguage.json:441 begins with
  `^(\s*)((private|export)\s+)?(let)\s+(?!<)…` and the element form at :2413 likewise allows no
  `external` between the visibility and `let`. Tokenizing `export external let sin(x:float64):float64`
  gives only `storage.modifier.*` / `keyword.declaration.let.nx` and loose text: `sin` gets no
  `entity.name`, `x` is not a parameter, and there is no `meta.definition` scope. In
  crates/nx-api/src/std/math/math.nx:36, `atan2(` comes out as `" atan"` plus
  `constant.numeric.integer.nx` `"2"`. The `///` parameter docs inside the multiline signature are
  scoped, but the parameters are not. The website imports this grammar
  (sites/website/starlight.config.mjs:1), so the new `nx fragment` blocks in functions.md and math.md
  render wrongly too. Task 1.1 ("update highlighting") changed only tree-sitter highlights.scm, and
  `pnpm test:grammar` passes because no test covers this form.
- **Recommendation:** Allow an optional `external\s+` after the visibility in both `let` begin
  rules, for the identifier and element forms. Keep the `end` lookaheads consistent; they already
  mention `external … let`. Add declarations.test.ts cases for
  `export external let sin(x:float64):float64`, the element form `external let <Cos x:float64 />:float64`,
  and a multiline signature with `///` parameter docs. Each case should assert the function name,
  parameter and return-type scopes, and that the next declaration still scopes normally.

- **Fix:** Both TextMate declaration begin rules now accept external and scope its modifier. Three regression tests cover parenthesized and element declarations, return types, multiline parameter docs, numeric names, and subsequent declarations; all 290 grammar tests pass.
- **Verification:** Both begin rules (`value-definition`, `function-definition`) now take
  `(?:(external)\s+)?` with renumbered captures. A semantic JSON comparison against the index
  version shows that only those two rules changed. Re-running `pnpm test:grammar` gives 290 passing,
  including the three new external cases. Re-tokenizing crates/nx-api/src/std/math/math.nx gives
  `atan2` as one `entity.name.variable.nx` token, `y`/`x` as `variable.other.property.nx` across
  the multiline signature, scoped `///` parameter docs, and a scoped `:float64` result. Function
  names get the same scope as ordinary `let f(...)` declarations. The file was re-serialized with
  ASCII escapes in unrelated comments; that is tracked as RF7.

### ✅ Verified - RF2 Invalid external function syntax gives a generic error that swallows the rest of the file
- **Severity:** Medium
- **Evidence:** The grammar makes the external branch end at the return type and has no body slot
  (crates/nx-syntax/grammar.js:326-332). Probes:
  `external let cos(x:float64):float64 = { 0.0 }\nlet other() = { 1 }\nlet root() = { other() }`
  reports a single `syntax-error` "Syntax error" over 0:36–3:0. `other` and `root` are not lowered.
  `external let cos(x:float64)\n…` (no return type) reports "Syntax error" over 0:0–3:0 and lowers
  no items at all. The existing external-component convention accepts the body in the grammar
  (grammar.js:363-372) and diagnoses it precisely: `external component <X a:int /> = { … }` reports
  `invalid-component-definition` on just that declaration and keeps the following ones. The
  design's own risk ("Grammar changes can alter recovery") is not covered: the syntax tests in
  crates/nx-syntax/tests/external_functions.rs check only that `errors` is non-empty.
- **Recommendation:** Parse an optional `= body` and an optional return type on the external branch
  (or one shared branch with an optional `external` field). Then reject them in
  crates/nx-syntax/validation.rs with targeted messages, such as "an external function cannot have
  a body" and "an external function requires a result type", spanning only the offending
  declaration. Extend the syntax tests to check the message and the span, and check that a
  following declaration still parses and lowers.

- **Fix:** The external grammar accepts missing result annotations and optional authored bodies for recovery. Syntax validation emits invalid-external-function-definition on the relevant declaration/body. Parser and HIR regression tests prove later declarations survive and diagnostics do not span them.
- **Verification:** crates/nx-syntax/grammar.js:326-333 makes the result type and `= body` optional
  on the external branch only. `validate_external_function_definitions`
  (crates/nx-syntax/src/validation.rs) reports `invalid-external-function-definition`: on the
  declaration for a missing result type, and on just the body for an authored body. Regenerating
  the parser from grammar.js in a temp dir with the pinned tree-sitter-cli 0.25.10 reproduces
  grammar.json, node-types.json and parser.c byte for byte. Editor-diagnostics probes now report:
  body → 0:38–0:45 "cannot have a body"; no result → 0:0–0:27 "requires a result type"; both →
  both diagnostics; element form with a body → body span only. None of them produces a
  `syntax-error`, and the following `other`/`root` declarations survive (hover on `other()` resolves
  `let other()`). Ordinary bodyless functions still fail, `let f(external:int)` still parses as an
  identifier, and the new parser and HIR tests check the message, the span bound and the retained
  declarations. A residual recovery gap for the non-function shape `external let value:float64` is
  tracked as RF8.

### ✅ Verified - RF3 After archive, the main standard-libraries spec will still forbid user-visible external-function syntax
- **Severity:** Medium
- **Evidence:** openspec/specs/standard-libraries/spec.md:51-54, the scenario "Runtime-provided math
  declarations are ordinary imported functions", requires math to expose its functions "without
  user-visible external-function syntax". This change adds public, documented `external let` syntax
  (sites/website/src/content/docs/reference/syntax/functions.md "External functions"), and math.nx
  is authored with it. The change's deltas modify only runtime-standard-functions, where the
  matching "No public external-function syntax" clause is correctly removed. Nothing touches
  standard-libraries, so archiving leaves contradictory main specs. openspec/specs/math-library has
  no similar wording.
- **Recommendation:** Add a `specs/standard-libraries/spec.md` delta that MODIFIES that requirement.
  Keep the scenario heading and edit only its body. For example: importers see ordinary typed
  declarations, the declarations are authored as `external let` in compiler-owned source, and user
  code cannot register or replace their implementations. Re-run strict validation.

- **Fix:** Added the standard-libraries capability and a MODIFIED delta preserving the existing requirement/scenarios while replacing the no-external-syntax clause with documented compiler-owned external signatures.
- **Verification:** specs/standard-libraries/spec.md MODIFIES "Standard libraries are carried by
  the compiler and imported by name". A line diff against openspec/specs/standard-libraries/spec.md
  shows the whole requirement and all seven scenario headings unchanged except the math scenario's
  THEN clause. That clause drops "without user-visible external-function syntax" and adds that the
  declarations are documented `external let` signatures in compiler-owned NX source; the
  no-register/replace clause is kept. The proposal lists the modified capability,
  `openspec instructions apply` picks up the third spec file, and
  `openspec validate add-external-function-declarations --strict` passes.

### ✅ Verified - RF4 `clamp`'s documentation links `[min]` and `[max]` resolve to its own parameters, not the min/max functions
- **Severity:** Low
- **Evidence:** crates/nx-api/src/std/math/math.nx:65 reads "using [min] and [max] signed-zero
  rules", which means the `min`/`max` functions and their signed-zero preferences. `clamp`'s
  parameters are also named `min` and `max`, and doc-link resolution checks the documented
  declaration's own members first (crates/nx-types/src/doc_links.rs:296-300, `Scope::Function`
  matching parameters at :328-334). A user-module copy of the same shape resolves `[min]` to
  `(parameter) min: float64`. Hover and navigation therefore point at the parameter, and the
  sentence reads as circular.
- **Recommendation:** Reword so the link cannot bind to a parameter. Write the rule out in words
  ("comparisons of signed zeros prefer negative zero at the lower bound and positive zero at the
  upper bound"), or name the functions without links. Add a resolved-link assertion for the math
  module, or a check that its doc links resolve to the intended declarations.

- **Fix:** Reworded clamp with a code-form expression and no ambiguous min/max links. A math-module test verifies every remaining resolved doc link targets a public function rather than a parameter. Refreshed math source/version fixtures.
- **Verification:** `clamp` now documents `` `[min, max]` `` and `` `min(max(value, min), max)` `` in
  code spans, with "The min and max functions determine the signed-zero preferences" in prose. The
  only remaining links in math.nx are `[roundTiesToEven]`, `[round]`, `[radiansToDegrees]` and
  `[degreesToRadians]`, and none of those names is a parameter of the declaration it documents.
  `math_doc_links_target_public_functions_without_shadowed_parameter_links` requires a non-empty
  link set whose targets all have `member: None` and name a `MathOp`, which would catch a
  reintroduced parameter link. That matches `minimum(maximum(x, lo), hi)` in
  crates/nx-value/src/math.rs. The IR corpus passes with the refreshed fixtures.

### ✅ Verified - RF5 Registry-binding diagnostics are not source-oriented enough and expose the internal version descriptor
- **Severity:** Low
- **Evidence:** In crates/nx-hir/src/lower.rs:3271-3280, a missing registered declaration is reported
  at `root.span()`, the whole file (probe: span 0..38 for a two-line module). In
  lower.rs:2657-2665, a mismatch reports
  "does not match its registered runtime signature: <module>::cos(x:float64):float64=1:v1". That
  embeds the hashing descriptor, with the placeholder `<module>` and the internal `=<op>:v1`
  suffix. It also does not say what differs (parameter name, type, optional/default/content flag,
  calling form, export visibility or result type). The spec's "Registry mismatch" scenario asks
  for a source-oriented diagnostic. These only fire for compiler-owned sources, so the effect is
  limited to maintainers, and HIR tests assert only that some diagnostic exists.
- **Recommendation:** Report the expected signature in NX form, such as
  `export external let cos(x:float64):float64`, and name the first differing aspect. Put the span
  on the differing parameter or return type where possible. For a missing declaration, use an
  empty span at the start of the file, or attach it to a same-named ordinary declaration when one
  exists. Assert the message text in crates/nx-hir/tests/external_functions.rs.

- **Fix:** Binding diagnostics identify the first mismatch, show the expected NX signature, and highlight the relevant parameter/return annotation. Missing declarations use an empty start-of-file span or a same-named ordinary declaration. Added exact message/span checks.
- **Verification:** Probes through `lower_with_runtime_functions` give a reason plus
  ``expected `export external let …(…):float64` `` with no `<module>` and no `:v1`. Reasons and
  spans by case: calling form, visibility and arity → declaration span; wrong second parameter name
  (`atan2(y, z)`) → `z:float64`; content parameter → `content x:float64`; `x:int` / `x?` / default →
  the parameter; `:int` result → `int`; missing result → declaration. A missing declaration is the
  empty span 0..0. A same-named ordinary declaration is that declaration's span. A duplicate is the
  second declaration. The new HIR test asserts exact text and spans. Two non-blocking nits:
  (1) a mismatched declaration also gets "Registered runtime function '…' requires a matching
  external declaration" at the same declaration, so maintainers see two errors for one mistake;
  (2) the arity message says "expected 1 parameters". Neither leaks the descriptor or misplaces a
  span, and both are maintainer-only.

### ✅ Verified - RF6 The formal grammar reference was not updated for external function declarations
- **Severity:** Low
- **Evidence:** reference/syntax/functions.md:542 links to nx-grammar.md#functions as the grammar.
  There, `ElementFunctionDefinition` and `ParenFunctionDefinition` (nx-grammar.md:244-248) still
  require `"=" RhsExpression` and have no `external` form or required result type.
  nx-grammar-spec.md:407-418 and :817-818 likewise describe `body` as always present. Task 3.1
  says external declarations were documented in the NX references.
- **Recommendation:** Add an `ExternalFunctionDefinition` production
  (`[VisibilityModifier] "external" "let" (signature) ":" TypeDeclaration`) to nx-grammar.md. Update
  the nx-grammar-spec.md AST description, which now has an optional body and an external flag.
  The missing `external` on `ComponentDefinition` (nx-grammar.md:271) is an older gap that can be
  fixed in the same edit.

- **Fix:** Updated nx-grammar.md with the external production and component modifiers; nx-grammar-spec.md documents external tokens/productions, optional AST body/external flags, and recovery validation.
- **Verification:** Incomplete. nx-grammar.md is correct: `FunctionDefinition ::= … |
  ExternalFunctionDefinition`, a `FunctionSignature` production, the restriction prose, and the
  `abstract`/`external` component modifiers with an optional body. nx-grammar-spec.md adds the
  `EXTERNAL` keyword, an `ExternalFunctionDefinition` production and the AST fields
  (`isExternal`, `body?`). However, its `FunctionDefinition` sum (nx-grammar-spec.md:366-368) still
  lists only `ElementFunctionDefinition` and `ParenFunctionDefinition`. So in that document the new
  production is unreachable from `ModuleMember`, and the formal derivation still excludes external
  functions. Fix: add
  `- FunctionDefinition → ExternalFunctionDefinition (ElementFunctionDefinitionSyntax | ParenFunctionDefinitionSyntax with isExternal)`,
  or an equivalent line.

## New Findings Discovered During 2026-10-28 03:19 Verification

- **Fix (round 2):** Second pass: added ExternalFunctionDefinition to the FunctionDefinition alternatives in nx-grammar-spec.md, making the new production reachable from ModuleMember.
- **Verification (round 2, RF6):** nx-grammar-spec.md:366-369 now lists
  `FunctionDefinition → ExternalFunctionDefinition (ElementFunctionDefinitionSyntax |
  ParenFunctionDefinitionSyntax with isExternal)`, so the production is reachable from
  `ModuleMember → FunctionDefinition`. It also gains a recovery note for external value shapes,
  which matches RF8's grammar. nx-grammar.md is unchanged from round 1 and still correct. Verified.
  (This round-2 fix note sits under the 03:19 "New Findings" header but belongs to RF6 above. It
  was left in place to keep the history intact.)

### ✅ Verified - RF7 The TextMate grammar file was re-serialized, escaping non-ASCII characters in unrelated comments
- **Severity:** Low
- **Evidence:** The unstaged src/vscode/syntaxes/nx.tmLanguage.json diff rewrites five unrelated
  `comment` strings (`rhs-expression`, `loop-header`, `loop-header-split-binder`,
  `hover-annotation` and its first pattern), turning literal `—`/`…` into `\u2014`/`\u2026`. The
  file now has no literal `—` left. The JSON is semantically identical (a parsed comparison shows
  only the two intended rule changes), but the churn makes the diff hard to read and mixes styles
  for whoever edits the grammar next.
- **Recommendation:** Restore the original UTF-8 characters in those comments so the diff contains
  only the two begin rules and their captures. If the file is edited by script, write it with
  `ensure_ascii=False` or the equivalent.

- **Fix (round 2):** Restored literal UTF-8 comments using ensure_ascii=False; the JSON diff now changes only the two intended declaration rules and captures.
- **Verification (round 2):** `git diff -- src/vscode/syntaxes/nx.tmLanguage.json` (and against
  HEAD) is now 9 insertions and 3 deletions, confined to the `value-definition` and
  `function-definition` begin regexes and their `beginCaptures`. Capture keys are in numeric order
  (3 visibility, 4 external, 5 let, 6 name). No `\u2014`/`\u2026` escape sequences remain. `pnpm test:grammar`
  gives 290 passing. The two rules carry the same RF1 semantics, which round 1 confirmed by
  re-tokenizing math.nx. Verified. The reviewer also restored this finding's Evidence wording,
  where the escape sequences had been rendered as literal characters.

### ✅ Verified - RF8 `external let value:float64` (an external value shape) still swallows the rest of the file
- **Severity:** Low
- **Evidence:** RF2's fix covers external functions, but a plausible external constant,
  `external let value:float64\nlet other() = { 1 }\nlet root() = { other() }`, still yields one
  `syntax-error` "Syntax error" over 0:0–3:0. That drops every later declaration. The comparable
  ordinary mistake `let value:float64` (no initializer) is reported at 0:0–0:17 and keeps the
  following declarations. crates/nx-syntax/tests/external_functions.rs only asserts that this
  input is rejected.
- **Recommendation:** Give the external branch a recoverable shape for a non-function name, for
  example an `external let identifier [: type] [= value]` alternative that validation rejects with
  "external declarations must be functions" on that declaration only. Alternatively, document it as
  accepted recovery behavior. Add a recovery test like RF2's that checks the span bound and the
  surviving later declarations.

- **Fix (round 2):** Added an external identifier-only recovery alternative. Validation rejects it as External declarations must be functions without consuming later declarations. New tests cover typed/untyped/initialized invalid values, precise diagnostic bounds and surviving CST/HIR functions.
- **Verification (round 2):** The external branch in grammar.js is now
  `choice($._function_signature, field('name', $.identifier))`, with optional result and body.
  Regenerating the parser in a temp dir reproduces grammar.json, node-types.json and parser.c byte
  for byte. Editor-diagnostics probes followed by `let other()`/`let root()`: `external let
  value:float64`, `… = 0.0`, `external let value`, `export external let value = 1`,
  `external let value = { (1) }` and `external let value:<function x:int />:int` each give exactly
  one `invalid-external-function-definition` "External declarations must be functions", confined
  to that line, and no `syntax-error`. `other`/`root` are kept, with hover and completion working
  after them. The `(`/`<` check correctly ignores parentheses inside bodies and function types.
  Well-formed parenthesized and element externals and `let f(external:int)` are unaffected. In a
  trusted module, `export external let cos:float64` fails with a parse error plus a binding
  mismatch, so no value shape can bind. Verified. Remaining nit, not blocking: a rejected value
  shape lowers as a zero-parameter function, so hover shows `let value(): float64` and a redundant
  "No runtime implementation is registered…" follows. Both appear only alongside the primary
  error. A related half-typed-declaration issue is tracked as RF9.

## New Findings Discovered During 2026-10-28 03:24 Verification

### ✅ Verified - RF9 Half-typed external declarations get targeted diagnostics attributed to the next declaration
- **Severity:** Low
- **Evidence:** When an external declaration has a syntax error, tree-sitter recovery folds the
  next declaration into it (as it does for ordinary functions). `validate_external_function_definitions`
  (crates/nx-syntax/src/validation.rs) then runs on that merged node. Editor-diagnostics probes,
  each followed by `\nlet other() = { 1 }\nlet root() = { other() }`:
  - `external let cos(`: besides `syntax-error` "Invalid function definition", this adds
    "An external function requires a result type" over 0:0–1:19, "An external function cannot have
    a body" at 1:14–1:19 (that is `other`'s `{ 1 }`), and "No runtime implementation is
    registered for external function 'cos'".
  - `external let ` (keyword typed, name not yet): "cannot have a body" again lands on `other`'s
    body, and the lowering error names **`other`** as the external function.
  - The ordinary equivalent `let cos(` reports only the syntax error plus the consequential
    undefined `other`.
  These are realistic while typing in the editor. The extra errors point at the wrong declaration
  and suggest edits ("remove the authored body") that would delete working code. Recovery tests
  cover only well-formed invalid shapes, not declarations that contain ERROR/MISSING nodes.
- **Recommendation:** In `validate_external_function_definitions`, skip any external
  `function_definition` that `has_error()` / contains ERROR or MISSING nodes, and let the syntax
  error stand alone. Optionally also skip lowering's unregistered-implementation diagnostic for
  such nodes. Add tests for `external let cos(` and `external let ` followed by a declaration with
  a body: they should assert that no `invalid-external-function-definition` lands outside the
  first line and that no diagnostic names `other`.

- **Fix (round 3):** External-specific syntax checks skip nodes with ERROR/MISSING descendants. Lowering skips malformed external recovery nodes and invalid identifier-only value shapes, preventing misleading implementation diagnostics and fabricated zero-parameter function hovers. Added parser, HIR and editor regressions for three half-typed declarations and invalid value shapes.
- **Verification (round 3):** `validate_external_function_definitions`
  (crates/nx-syntax/src/validation.rs:1238-1242) now skips external nodes where `has_error()` is
  true. `lower_module` (crates/nx-hir/src/lower.rs:3188-3199) skips external nodes that contain an
  error or have no `(`/`<` parameter list. Editor-diagnostics probes, each followed by
  `let other() = { 1 }` / `let root() = { other() }`: `external let cos(` and `external let `
  now report only the syntax error plus the consequential undefined `other`, the same output as
  ordinary `let cos(`. `export external let cos(x:` reports only syntax errors. No "cannot have a
  body", no "No runtime implementation", and no diagnostic naming `other` as an external.
  Well-formed invalid shapes are unchanged: a missing result type, an authored body and an
  external value still each give their targeted diagnostic, and the value shape no longer
  fabricates a zero-parameter function (hover returns nothing), which closes RF8's nit. Using a
  rejected value gives `Undefined identifier 'value'`, the same as ordinary `let value:float64`.
  Valid trusted bindings still pass. Trusted value or half-typed shapes still fail through their
  parse errors plus the empty-span missing-declaration diagnostic. `cargo test` for nx-syntax,
  nx-hir, nx-api, nx-language-service and nx-types has no failing suites, including the 5 + 7
  external_functions tests and
  `incomplete_external_syntax_has_no_misleading_runtime_or_body_diagnostics`. `cargo fmt --check`,
  `git diff --check` and strict validation pass, and the generated parser still matches grammar.js
  (no grammar change this round). Verified. Non-blocking observation: an otherwise well-formed
  external whose authored body itself contains a syntax error
  (`external let cos(x:float64):float64 = { 1 + }`) is now dropped entirely. Its call sites gain
  `Undefined identifier 'cos'` and the "cannot have a body" hint is lost, while the ordinary
  equivalent stays defined. This needs an already-invalid body that is also malformed, and the
  syntax error still points at it. If wanted, restrict the skip to errors outside the `body` field.

## Questions
- A parameter that shadows an import alias does not shadow it in qualified access. With
  `import "@nx/math" as math`, `type M = { pi:string }` and `let f(math:M):string = { math.pi }`,
  the type checker reports "expects string, found float64". The new alias-member hover
  (crates/nx-language-service/src/lib.rs:754-779) matches that, showing `@nx/math` `pi` on
  `math.pi`. The cause is that `UndefinedIdentifierChecker` skips the base of a member path that
  resolves to a prepared binding (crates/nx-hir/src/scope.rs:563-577), so `is_local_reference` is
  false. The behavior predates this change and hover is consistent with the compiler, but the
  comment "real/local receivers retain member hover below" is not true for this case. Is
  alias-over-parameter resolution intended? If not, fix it in scope resolution in a separate change
  and add a hover test that pins the result.
- Completion documentation for a name that a parameter shadows (for example `pi` with
  `let root(pi:string)`) shows the shadowed declaration's docs. This also happens for same-module
  declarations, so it is older behavior, noted only for awareness.

## Summary
- The core of the change is sound. `external let` parses as a bodyless signature. Ordinary and
  host-library lowering rejects unregistered declarations. Trusted binding checks names, order,
  float64 types, flags, calling form, export visibility, result type, missing and duplicate
  declarations. Docs and declaration spans come from math.nx while body and argument spans stay
  synthetic (empty), and the IR debug text shows declaration origins only. Hover and completion
  carry source docs for wildcard, aliased, selective and qualified-selective imports. All Rust,
  TypeScript, grammar and conformance suites the reviewer ran pass.
- The open issues are editor and diagnostic quality and spec consistency: the VS Code/website
  TextMate grammar (RF1), recovery and messages for invalid external syntax (RF2), and a main-spec
  contradiction that archiving would leave behind (RF3). There are also three Low documentation
  and diagnostic polish items (RF4–RF6).
- After verification (2026-10-28 03:19 EDT): RF1–RF5 are verified. RF6 needs one more line in
  nx-grammar-spec.md. RF7 and RF8 are Low cleanup and recovery items. Nothing at Medium or above
  remains open.

## Implementation response

RF1–RF6 are fixed pending independent Claude verification. The full nine-crate Rust regression
run, full TypeScript suite, 290 TextMate grammar tests, documentation examples, regenerated
math fixtures and strict OpenSpec/diff/format checks pass. The workspace also checks successfully.

The two questions concern existing compiler/completion shadowing semantics, outside this change's
binding and documentation contract. They remain recorded for separate scope-resolution work.
The alias-hover comment was corrected to describe following compiler resolution rather than promise
local shadowing behavior that the compiler does not currently provide. No resolution semantics
were changed in this fix round.

## Verification (2026-10-28 03:19 EDT, America/New_York)

- **Results:** RF1–RF5 verified. RF6 reopened (nx-grammar-spec.md `FunctionDefinition`
  alternatives omit `ExternalFunctionDefinition`). New Low findings RF7 (TextMate JSON escape churn)
  and RF8 (external value-shape recovery).
- **Shadowing questions:** Accepted as pre-existing scope-resolution behavior outside this change.
  The corrected alias-hover comment (crates/nx-language-service/src/lib.rs:754-755) now describes
  following compiler resolution, and no resolution semantics changed.
- **Reviewer-run checks:** `cargo test -q -p nx-syntax -p nx-hir -p nx-api -p nx-language-service -p nx-types`
  and `cargo test -q -p nx-codegen -p nx-ir -p nx-ir-runtime -p nx-interpreter -p nx-value` (no
  failing suites, IR corpus without update mode). `pnpm test:grammar` gives 290 passing. A fresh
  `tsc` build into a temp dir matches `dist/` exactly, and the TS runtime, corpus, emitted-IR, math
  (184 cases) and bench tests pass against it. `check-code-blocks.mjs` for functions.md and
  math.md: 23 blocks, 0 failures. `git diff --check` is clean. Strict OpenSpec validation passes.
  The parser regenerated in a temp dir is byte-identical to the committed generated files. Probes
  ran out of repo (`/tmp/nxprobe`). The reviewer changed no implementation files.

## Implementation response, round 2

RF6, RF7 and RF8 are fixed pending Claude re-verification; RF1–RF5 remain verified. All ten focused
external-function parser/lowering tests pass, including three external-value recovery variants.
The formal grammar alternative and the minimal UTF-8 TextMate diff have been checked. Affected
Rust and TextMate regression suites run again for this round. No shadowing semantics changed.

## Verification, round 2 (2026-10-28 03:24 EDT, America/New_York)

- **Verified this round:** RF6, RF7 and RF8. RF1–RF5 stay verified.
- **Reopened:** none.
- **New:** RF9 (Low): half-typed external declarations get validation and lowering diagnostics
  attributed to the next declaration.
- **Outstanding:** 1 open finding (RF9, Low). Nothing at Medium or above is open.
- **Out of scope:** The two shadowing observations under Questions (a parameter does not shadow an
  import alias in qualified access, and completion docs for shadowed names) describe
  pre-existing scope-resolution and completion behavior. They are not findings for this change.
  This round changed no resolution semantics.
- **Reviewer-run checks:** `cargo test -q -p nx-syntax -p nx-hir -p nx-api -p nx-language-service -p nx-types`
  and `cargo test -q -p nx-codegen -p nx-ir -p nx-ir-runtime -p nx-interpreter -p nx-value` (no
  failing suites, IR corpus without update mode). `pnpm test:grammar` gives 290 passing. Strict
  OpenSpec validation passes and `git diff --check` is clean. The parser regenerated from grammar.js
  in a temp dir is byte-identical to the committed generated files. Probes ran out of repo
  (`/tmp/nxprobe`). The reviewer changed no implementation files. Besides the status and
  verification notes, the reviewer restored RF7's Evidence sentence, where `—`/`…` had
  been rendered as literal characters.

## Implementation response, round 3

RF1–RF8 remain verified. RF9 is fixed pending independent verification. All twelve focused
external parser/lowering tests and the editor diagnostic regression pass; affected Rust regressions,
formatting, diff checks and strict OpenSpec validation run again. This round changes validation
and HIR recovery only, with no grammar or runtime numerical changes.

## Verification, round 3 (2026-10-28 03:31 EDT, America/New_York)

- **Verified this round:** RF9. RF1–RF8 stay verified.
- **Reopened:** none. **New:** none (the malformed-body-in-external cascade noted under RF9 is
  recorded as non-blocking).
- **Cumulative:** 9 findings, RF1–RF9, all ✅ Verified. 0 open.
- **Out of scope:** The shadowing observations under Questions are unchanged and stay separate,
  pre-existing scope-resolution and completion behavior.
- **Reviewer-run checks (focused; no grammar or numerical runtime change this round):**
  `cargo test -q -p nx-syntax -p nx-hir -p nx-api -p nx-language-service -p nx-types` (no failing
  suites), `cargo fmt --all -- --check`, `git diff --check`, strict OpenSpec validation, a parser
  regeneration identity check, and out-of-repo probes in `/tmp/nxprobe`. The reviewer changed no
  implementation files.

## New Findings Discovered During 2026-10-28 11:33 Review

Fresh independent review (2026-10-28 11:33 EDT, America/New_York) of the syntax revision that drops
`let` from external function declarations (`export external sin(x:float64):float64`,
`external <Cos x:float64 />:float64`). RF1–RF9 above are historical and describe the earlier
`external let` syntax. Their old-syntax evidence and validation records are not treated as live
stale documentation.

### Scope
**Reviewed artifacts:** proposal.md, design.md (including "Syntax revision"), tasks.md (4.1 done,
4.2 open), validation.md ("Bare external syntax revision"), specs/external-functions,
specs/runtime-standard-functions and specs/standard-libraries deltas, plus the working-tree main specs
they modify.
**Reviewed code:** the unstaged revision diff on top of the staged implementation:
crates/nx-syntax (grammar.js external branch, regenerated grammar.json/parser.c, validation.rs
external checks and error classification, tests/external_functions.rs), crates/nx-hir (lower.rs
expected-signature text, tests/external_functions.rs), crates/nx-api (std/math/math.nx, all 22
declarations; standard_library_tests.rs), crates/nx-language-service (keyword list, hover.rs
signature rendering, typing-recovery test), src/vscode/syntaxes/nx.tmLanguage.json and
test/grammar/declarations.test.ts, nx-grammar.md, nx-grammar-spec.md, website functions.md and
math.md, and the specs/ir-conformance/math-library fixtures. The archived math implementation and the
unrelated DrawnUI/graphics proposal work in the same worktree are out of scope.
**Validation run by reviewer:** `cargo test -q -p nx-syntax -p nx-hir -p nx-api -p nx-language-service -p nx-types`
(40 suites, no failures). `cargo test -q -p nx-codegen -p nx-ir -p nx-ir-runtime -p nx-interpreter -p nx-value`
(38 suites, no failures, including the IR corpus without update mode). `pnpm test:grammar` in
src/vscode gives 290 passing. A fresh `tsc` into a temp dir matches `runtime/typescript/dist`
exactly. Against that dist, `corpus.test.mjs`, `emitted-ir.test.mjs` and `math.test.mjs` pass
(`math: 184 reference cases`). Regenerating the parser from grammar.js in a temp dir with
tree-sitter-cli 0.25.10 reproduces grammar.json, node-types.json and parser.c byte for byte.
`check-code-blocks.mjs` for functions.md and math.md gives 23 blocks, 0 failures. Strict OpenSpec
validation, `git diff --check` (staged and unstaged) and `cargo fmt --all -- --check` pass. No
`external let` remains in live source, docs or specs outside the deliberate rejection test and
the change's design/tasks. Math-free IR fixtures are unchanged. In the math fixtures only spans,
fingerprints and versions moved: declaration origins are now 4 columns shorter and bodies stay
synthetic. Behavior probes ran out of repo (`/tmp/nxrev10`: a Rust crate using the editor
diagnostics, CST and hover APIs, and a vscode-textmate tokenizer comparing the HEAD and worktree
grammars). The reviewer changed no implementation files.

Checked with no finding: `external component <X />` still parses as a component, because the
`component` keyword wins after `external`. `let external = 1`, `let f(external:int)`, a record field
`external:int` and a trailing root element after an external declaration are unaffected. Keyword
names (`external type(…)`) are accepted the same way as the existing `let type(…)`. Half-typed
external declarations (`external cos(`, `external <Cos`, `external sin(x:float64):`) recover the
same way as their ordinary `let` equivalents. A `///` comment between an external and the next
declaration attaches correctly. Trusted binding, docs and source origins and math hover/completion
docs pass their tests and probes. The binding diagnostic now prints
`export external cos(x:float64):float64`.

### ✅ Verified - RF10 The superseded `external let` spelling is rejected with generic or misleading diagnostics, and the element form swallows the rest of the file
- **Severity:** Low
- **Evidence:** After `external`, `let` is not a valid keyword, so tree-sitter lexes it as an
  identifier. The new grammar (crates/nx-syntax/grammar.js:328-331) then has
  `field('name', $.identifier)` available for it. Editor-diagnostics probes, each followed by
  `let other() = { 1 }` / `let root() = { other() }`:
  - `export external let sin(x:float64):float64` reports one `syntax-error`, "Invalid function
    definition", on `let` (0:16–0:19). Its help is the generic
    "Expected: let name(params) = { value }" (crates/nx-syntax/src/validation.rs:1813-1817), which
    tells the author to write `let` and add a body, the opposite of the external rules. Because the
    node has an ERROR child, validation and lowering skip it, so `sin` disappears and its callers
    get `Undefined identifier`.
  - `external let <Cos x:float64 />:float64` parses as an external value named `let`. It reports
    "External declarations must be functions" at 0:0–0:12, which is misleading because the author
    did write a function. It also reports a `syntax-error` "Syntax error" over 0:13–2:24, which
    swallows `other` and `root`. RF2 and RF8 fixed exactly this kind of whole-file loss.
  - `external let` on a line of its own gives no syntax error at all, only the value-shape
    diagnostic for an "external value named `let`".

  `superseded_external_let_spelling_is_rejected`
  (crates/nx-syntax/tests/external_functions.rs:135-146) asserts only `!parsed.errors.is_empty()`.
  That is the same weak check RF2 called out, so it does not pin the message, the span or the
  later declarations.
- **Recommendation:** Give the old spelling a recoverable shape. For example, add
  `optional(field('obsolete_let', 'let'))` right after `field('external', 'external')` in
  grammar.js and regenerate the parser. Then make `validate_external_function_definitions` report
  `invalid-external-function-definition` on just the `let` token: "External functions are declared
  without `let`", with help such as "write `external name(params):Result`". Validate and lower the
  rest of the declaration as usual. Extend the rejection test to cover both forms, asserting the
  message, a span confined to `let`, no `syntax-error` swallowing later lines, and surviving
  `other`/`root` declarations. Optionally add an "Invalid external syntax" scenario for the
  superseded spelling to specs/external-functions, since design.md states the rejection.

- **Fix (bare external revision):** Added same-line obsolete-prefix recovery and a targeted diagnostic on only let, including standalone obsolete syntax. Tests verify both full declaration forms, message/code/span, no whole-file syntax error, and retained other/root declarations; ordinary external names beginning with let and the half-typed next-line cases are also checked.
- **Verification:** grammar.js adds `obsolete_external_keyword`, a single token
  `external[ \t]+let[ \t]+`, as an alternative for the `external` field. Validation reports
  `invalid-external-function-definition` "External functions are declared without `let`" on just
  the `let` bytes, and for a standalone `external let` it reports the same diagnostic on a
  same-line `let` name. Parser regeneration in a temp dir with tree-sitter-cli 0.25.10 is byte for
  byte identical. Editor-diagnostics probes comparing the previous (index) parser with the
  worktree, each followed by `let other()`/`let root()`:
  - `export external let sin(…)`: before, a generic "Invalid function definition" and the
    declaration dropped. Now: the targeted diagnostic at 0:16–0:19 only, `sin` still lowers, and
    the expected unregistered-implementation error appears.
  - `external let <Cos x:float64 />:float64`: before, a misleading value-shape error plus a
    0:13–2:24 whole-file "Syntax error". Now: the targeted diagnostic at 0:9–0:12, with
    `other`/`root` kept and no `syntax-error`.
  - Standalone `external let`, `external let value:float64`, `external\tlet\tcos(…) = { 1.0 }`
    (obsolete diagnostic plus the existing body diagnostic), a documented obsolete declaration and
    an obsolete declaration after `external component <B />` all give targeted diagnostics only.
  - `external letter(…)`, `external external(…)`, `external let(…)` (a function named `let`, like
    `let let(…)`) and `let x = { external let }` are unaffected.
  - Half-typed `external let cos(` improved from a whole-file error to a bounded one. `external let `
    followed by a newline and a `let` declaration still loses that declaration, which is the same
    result as the existing `external ` half-typed case. `external` followed by a newline never
    forms the token, so a following-line `let` stays a separate declaration.

  The rejection test now asserts the code, message, a span of exactly `let`, no `syntax-error`,
  and that `other`/`root` survive for all three forms. Verified. Non-blocking notes: (1)
  `syntax_kind_from_str` has no entry for `obsolete_external_keyword`, so the token maps to
  `SyntaxKind::ERROR` and `SyntaxNode::is_error()` is true for it. No probe or test showed an effect
  (raw `has_error()` drives validation and lowering), but an explicit kind would be sturdier. (2)
  Across 12 ordinary-recovery probes, the only change from the regenerated parser is the
  end-of-file `let x = ` case, matching the refreshed snapshot. Before: "Expected identifier here"
  at 0:7 plus a bogus contextual-name diagnostic. Now: one "Syntax error" over 0:0–0:8. It is
  still a syntax error, and the other 11 cases produce identical diagnostics.

### ✅ Verified - RF11 The TextMate grammar now ends declaration contexts at any line that starts with the identifier `external`
- **Severity:** Medium
- **Evidence:** The revision adds a bare `external` to about 35 `end` lookaheads in
  src/vscode/syntaxes/nx.tmLanguage.json, as `(?:type|action|component|let|external|visualizer)\b(?!\s*[:=])`.
  The value-definition begin rule (:441) also now matches `^\s*external\s+<identifier>`. But
  `external` is still an ordinary identifier, and the tree-sitter parser accepts
  `let f(external:int)` and fields named `external`. Comparing the HEAD and worktree grammars
  with vscode-textmate shows these regressions:
  - Record `type Link = {` / `href:string` / `external?:boolean` / `label:string` / `}`: at HEAD,
    `external` is `variable.other.property.nx` inside `meta.definition.type.record.nx`. In the
    worktree, `external` becomes `storage.modifier.external.nx`, the record scope ends, `?:` and
    `label:` lose their scopes, and `}` is unscoped. The `?` defeats the `(?!\s*[:=])` guard.
  - In a multiline component signature with the same `external?:boolean` property, the closing
    `/>` is scoped as `keyword.operator.arithmetic.nx` / `keyword.operator.comparison.nx`.
  - `let f(external:int):int = {` / `external` / `}`: at HEAD the identifier stays inside
    `meta.values-braced-expression.nx`. In the worktree the braced expression and the definition
    end, and `}` is unscoped.
  - In element text content, a prose line `external links open in a new tab` now opens
    `meta.definition.value.nx`, scoping `links` as `entity.name.variable.nx` and each later word as
    `entity.name.qualifier.nx`. HEAD already mis-scoped this line, but differently.

  The website uses this grammar (RF1), so docs code blocks are affected too. The new
  declarations.test.ts cases check only well-formed external declarations. There is no
  "`external` as a property/identifier" test like the existing "keeps visualizer as a property
  name on its own line of a record".
- **Recommendation:** Only treat `external` as a declaration start when a declaration shape follows.
  In the end lookaheads, replace the bare `external` alternative with
  `external\s+(?:<|[A-Za-z_][A-Za-z0-9_]*\s*\()`, keeping the existing `external\s+component`
  prefix. In the value-definition begin rule, require `(?=\s*\()` after the name in the
  `external` branch. Add grammar tests for an optional `external?:boolean` property on its own line
  in a record and in a multiline component signature, a line-leading `external` identifier inside a
  braced body, and a prose line that starts with "external". Each should assert that the
  surrounding scope continues.

- **Fix (bare external revision):** External declaration starts in TextMate now require < or name(, and the identifier-style begin rule requires a parameter-list lookahead. Four regression tests cover optional external properties in records/components, a body identifier, prose, and recovery at a real declaration. All294 grammar tests pass.
- **Verification:** Every external alternative in the declaration-boundary lookaheads is now
  `external(?=\s+(?:<|[A-Za-z_][A-Za-z0-9_]*\s*\())` (35 sites). The identifier begin rule (:441)
  requires `(?=\s+name\s*\()`, and the element begin rule (:2416) requires `\s+(?=<)`. Re-running
  the original vscode-textmate probes:
  - A record or multiline component signature with `external?:boolean` keeps
    `variable.other.property.nx` and its enclosing scope, the following `label:string` is scoped,
    and `}` / `/>` close correctly.
  - A line-leading `external` in a function body stays inside `meta.values-braced-expression.nx`,
    and its `}` is scoped.
  - The prose line `external links open in a new tab` no longer opens a declaration. It scopes
    exactly as at HEAD.

  The four new tests cover these cases plus recovery at a real
  `export external sin(…)` after an unterminated record. `pnpm test:grammar` gives 294 passing.
  Verified. A residual edge that matches HEAD's pre-existing behavior, not a regression: a
  line-leading `external <…` or `external Word (` in prose or an expression still reads as a
  declaration start.

### ✅ Verified - RF12 Hover and completion signatures show external functions as `let` declarations
- **Severity:** Low
- **Evidence:** `function_signature` (crates/nx-language-service/src/hover.rs:132-168) always
  prints `let`. Hovering imported `atan2` shows
  ``(standard library @nx/math)`` / `let atan2(y:float64, x:float64): float64`. A same-module
  `external <Cos x:float64 />:float64` hovers as `let <Cos x:float64 />: float64`. After this
  revision, design.md says `external` is the declaration keyword for runtime-provided functions
  and `let` declares authored functions. A bodyless `let` signature is also a syntax error. So the
  editor shows a declaration form that the math source does not use and that would not compile if
  copied. `hover_describes_standard_math_signature` checks only for `sin`, `float64` and
  `@nx/math`.
- **Recommendation:** Render external functions with the `external` keyword, for example
  `external atan2(y:float64, x:float64): float64`. Use information lowering already has, such as
  the external syntax flag or a trusted runtime body. Assert the rendered keyword in the math hover
  test. If `let` is meant to be a generic "callable" display form, record that decision in
  design.md instead.

- **Fix (bare external revision):** HIR functions now retain an explicit is_external source marker. Hover and completion use it to render external for registered and unregistered declarations, including the element form; ordinary declarations retain let. Updated direct-HIR fixtures default the marker to false and added editor/HIR assertions.
- **Verification:** `nx_hir::Function` gains `is_external`, which lowering sets from the
  `external` syntax field (crates/nx-hir/src/lower.rs:2610) and the synthesized root sets to
  false. That makes it independent of whether a runtime body is registered. `function_signature`
  (crates/nx-language-service/src/hover.rs) picks `external`/`let` for both forms, and
  completion detail uses `external …` in place of `function …` for parenthesized functions and
  prefixes `external ` for element ones. Hover probes: imported `atan2` →
  `external atan2(y:float64, x:float64): float64`; unregistered same-module
  `external <Cos x:float64 />` → `external <Cos x:float64 />: float64`; `let <Panel …/>` and
  `export let <Panel …/>` still render `let`. The new
  `external_function_hover_and_completion_use_the_source_keyword` test covers imported,
  unregistered parenthesized and element externals, an ordinary `let`, and the `sin`
  completion detail. The HIR test asserts `is_external`.
  `cargo check --workspace --all-targets` passes, so no other `Function` constructor was
  missed. Verified.

### Questions
- None new. The earlier shadowing observations stay out of scope, as recorded above.

### Summary
- The revision is otherwise consistent. The grammar, regenerated parser, validation messages,
  binding diagnostic text, all 22 math declarations, refreshed math fixtures, formal grammar
  documents, website reference pages, active specs and TextMate begin rules all use
  `external name(…):Result`. External components keep their syntax. Trusted binding, docs and
  source origins, and recovery for well-formed invalid shapes still work. Every suite the reviewer
  ran passes.
- Opened 3 findings: RF11 (Medium) is a TextMate regression for valid code that uses `external` as
  an identifier. RF10 (Low) is poor recovery and messaging for the superseded `external let`
  spelling. RF12 (Low) is hover and completion signatures still showing `let`.

## Implementation response: bare external revision

RF10–RF12 are fixed pending independent verification. Historical RF1–RF9 remain verified for their
review scope. Updated parser recovery emits a specific obsolete-keyword diagnostic; syntax and
editor tests preserve ordinary external identifiers/properties and imported docs. The generic
unfinished-initializer diagnostic snapshot was refreshed for the regenerated parser's recovery
representation (still a syntax error). Updated runtime/docs checks and verification are recorded
in validation.md and the next reviewer section.

## Verification, bare external revision (2026-10-28 12:05 EDT, America/New_York)

- **Verified this round:** RF10, RF11 and RF12. RF1–RF9 stay verified as historical findings for
  the earlier `external let` syntax, and their old-syntax evidence is unchanged.
- **Reopened:** none. **New:** none. Two non-blocking notes are recorded under RF10: the obsolete
  keyword token maps to `SyntaxKind::ERROR`, and the end-of-file `let x = ` diagnostic is now a
  generic syntax error.
- **Cumulative:** 12 findings, RF1–RF12, all ✅ Verified. 0 open.
- **Reviewer-run checks:**
  - Rust: `cargo test -q` for nx-syntax, nx-hir, nx-api, nx-language-service, nx-types,
    nx-interpreter, nx-codegen, nx-ir, nx-ir-runtime and nx-value gives 84 suites, no failures,
    with the IR corpus run without update mode. `cargo check --workspace --all-targets` passes.
  - Parser: regenerating from grammar.js in a temp dir (tree-sitter-cli 0.25.10) is byte for byte
    identical to the committed files.
  - Grammar and runtime: `pnpm test:grammar` gives 294 passing. The TypeScript corpus and math
    tests pass (184 cases).
  - Docs and hygiene: `check-code-blocks.mjs` for functions.md and math.md gives 23 blocks, 0
    failures. Strict OpenSpec validation, `git diff --check` (staged and unstaged) and
    `cargo fmt --all -- --check` pass.
  - Probes ran out of repo in `/tmp/nxrev10`. They compared editor diagnostics between the previous
    (index) parser and the worktree on 27 inputs, ran hover probes, and re-ran the vscode-textmate
    tokenizer cases.

  The reviewer changed only review.md: no implementation files, staging, commits or archiving.
