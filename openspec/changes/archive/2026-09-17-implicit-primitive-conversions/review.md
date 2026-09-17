# Review: implicit-primitive-conversions

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/implicit-primitive-conversions,
specs/primitive-type-names, specs/contextual-numeric-literals, specs/nx-ir-format,
specs/typescript-ir-runtime, specs/playground  
**Reviewed code:** the uncommitted working tree: `crates/nx-types` (ty.rs, semantics.rs, infer.rs,
check.rs), `crates/nx-hir` (ast/expr.rs, components.rs, lower.rs, scope.rs, lib.rs),
`crates/nx-interpreter` (eval/arithmetic.rs, eval/logical.rs, interpreter.rs, value.rs, tests),
`crates/nx-codegen` (builder.rs, emit.rs, ir.rs, ir_image.rs, ir_explain.rs, model.rs, runtime.rs,
tests), `runtime/typescript/src/index.ts` and tests, `specs/ir-conformance/conversions`, docs
(nx-ir-format.md, language-tour, reference/syntax), `sites/playground` (examples, examples.json,
types.ts, README, FINDINGS.md, scripts, instances.test.mjs), `specs/future.md`  
**Checks run:** `cargo test --workspace` (all green), `runtime/typescript` tests (green),
playground `check-examples`, `pnpm test` and `typecheck` (all green), probe programs through
`nxlang run` and `nxlang codegen --target javascript`

## Findings

### ✅ Verified - RF1 Generated JavaScript and the IR runtime print a computed `float32` differently from the interpreter
- **Severity:** Medium
- **Evidence:** With `type F = { v:float32 }` and `let g(w:float32) = { w * 3 + " px" }`,
  evaluating `g(<F v=2.3 />.v)` gives `6.8999996 px` in the interpreter. Generated JavaScript gives
  `6.8999998569488525 px`. Neither JS target rounds `float32` arithmetic, so `nxFloat32Text` /
  `float32Text` receive an unrounded `float64`. They then run the `toPrecision` search on `value`
  instead of on `Math.fround(value)`. The search can pick different digits, and when no precision
  round-trips it falls back to `String(value)`, which prints 17 digits
  (`crates/nx-codegen/src/runtime.rs:449-462`, `:841-854`;
  `runtime/typescript/src/index.ts:2966-2979`). A random sample of `float32` products differed in
  about 37% of cases. This breaks the *Backends agree* requirement for any `float32` that is not a
  stored value, and the conformance corpus only formats a stored `float32` (`tenth`), so the
  mismatch goes unnoticed.
- **Recommendation:** Search and fall back on `const target = Math.fround(value)`, calling
  `target.toPrecision(p)` and `String(target)`, in all three copies. Add a conformance program that
  concatenates a computed `float32` (for example `w * 3` with `w = 2.3`), plus a runtime test.
  Consider sharing a single helper between the generated runtime and `@nx-lang/ir-runtime` rather
  than keeping three copies.
- **Fix:** All three helpers (`crates/nx-codegen/src/runtime.rs` ×2,
  `runtime/typescript/src/index.ts`) now set `target = Math.fround(value)` first and run the
  finite/zero check, the `toPrecision` search and the fallback on `target`. Added the
  `float32Product` conformance program (`w * 3 + " px"` with `w = 2.3`, expected `6.8999996 px`), a
  generated-JavaScript agreement case in `crates/nx-codegen/src/tests.rs`, and runtime tests for
  `float32Text(Math.fround(2.3) * 3)`. Rebuilt `runtime/typescript/dist`. The three copies are still
  separate.
- **Verification:** Verified. All three helpers now search and fall back on `Math.fround(value)`. The repro `w * 3 + " px"` with `w = 2.3` prints `6.8999996 px` in both the interpreter and generated JavaScript. The `float32Product` conformance case passes in both the with-debug and stripped runs of the TS runtime suite, as does the float32 runtime test. Keeping three copies is a noted trade-off, not a defect.
### ✅ Verified - RF2 A body that is a single braced primitive is rejected at a `string` content property
- **Severity:** Medium
- **Evidence:** `check_content_binding` sends a one-piece body down the ordinary typed-binding path
  before the string-join branch (`crates/nx-types/src/infer.rs:3659-3673`). As a result,
  `<Label>{count}</Label>` with `count:int` reports "Content for 'Label' binds to 'text' expects
  string, found int", while `<Label>Total: {count}</Label>` is accepted. The requirement covers "a
  body [that] consists of text runs and braced values" and exempts only "a single text run". The
  docs also claim `{1.0}` reads `1` (`docs/src/content/docs/language-tour/textual-content.md:33`),
  but that body is rejected.
- **Recommendation:** When the property is `string`/`string?` and the single piece is a
  stringifiable non-string primitive, record it for a `ToText` (and treat it as a join of one
  piece). Add checker and interpreter tests for it. Alternatively, narrow the spec to multi-piece
  bodies and fix the docs example to `Width: {1.0}`.
- **Fix:** `check_content_binding` now treats a lone braced value at a `string`/`string?` property
  as a one-piece join when its type is a stringifiable non-string primitive. The per-piece check is
  factored into `check_string_content_piece`, so the piece is inferred only once.
  `apply_string_conversions` accepts a one-piece body. Added checker shape tests (`ToText(Ident,
  int)`, a lone braced string unchanged, `{1.0}` accepted, `{count}` with `count:int?` rejected),
  interpreter tests (`"3"`, `{1.0}` → `"1"`), a spec scenario, and a sentence in
  `textual-content.md`.
- **Verification:** Verified. `<Label>{count}</Label>` binds `"3"`, `{1.0}` at a `string?` property binds `"1"`, and `{42}` binds `"42"`. A lone braced string keeps its spaces. `int?` and record values are still rejected, and an `Element` property and an `object` property are unchanged. A second rewrite pass is harmless because the `ToText` wrapper is not itself recorded.
### ✅ Verified - RF3 A braced string literal at either end of a joined body is trimmed, unlike any other braced value
- **Severity:** Low
- **Evidence:** `apply_string_conversions` trims the first and last sequence element whenever it is
  a `Literal::String` (`crates/nx-hir/src/components.rs:797-806`). A braced literal is also a
  `Literal::String`, so `<Label>{"  pad "}{n}</Label>` binds `"pad 2"`, while the same text passed
  through a `string` parameter keeps its spaces. design.md Decision 7 says "the first *run*'s leading
  and last *run*'s trailing whitespace", but the spec says "piece". The two disagree, and the code
  follows neither exactly.
- **Recommendation:** Trim only text runs (for example, record which content pieces came from
  `TEXT_RUN` during lowering), and align the spec wording with the design.
- **Fix:** Lowering records which content pieces came from text in the new `Element::text_runs`, and
  `apply_string_conversions` trims the first/last piece only when it is a text run. The spec now
  says a text run that begins or ends the body is trimmed and a braced value never is, with a new
  scenario. Added an interpreter test (`{"  pad "}{count}{" end  "}` → `"  pad 2 end  "`), a design
  note, and a docs sentence.
- **Verification:** Verified. `Element::text_runs` limits trimming to text runs: `{"  pad "}{n}` binds `"  pad 2"`, while `   a {n} b   ` binds `"a 2 b"`. The spec and design now agree.
### ✅ Verified - RF4 Newlines and indentation between pieces of a multi-line body are kept verbatim
- **Severity:** Low
- **Evidence:** A body written as `Total:` / `{count}` / `items` on separate indented lines binds
  `"Total:\n  3\n  items"`. The recorded `WhitespaceRun` is the raw source gap
  (`crates/nx-hir/src/lower.rs` `ContentGaps::note_piece`). Meanwhile a single-run multi-line body
  binds `"Total\n"` (the old path), so the two layouts of the same text disagree. The spec scenario
  only covers a single indented line, so this behavior is unspecified.
- **Recommendation:** Decide the rule for layout whitespace inside a body (for example, collapse a
  gap that contains a newline to one space, as JSX does) and state it in the spec, with a test. If
  keeping it verbatim is intended, say so in the spec and in `textual-content.md`.
- **Fix:** Line breaks are now layout. `apply_string_conversions` turns each whitespace stretch
  that contains a line break, whether in a gap or inside a text run, into one space
  (`collapse_line_breaks`), keeps whitespace within a line, and still trims the body's edges. A lone
  text run is now a one-piece join too, so the one-run and multi-run layouts agree. Only plain
  `TEXT_RUN`s are recorded in `text_runs`, so raw text and braced strings keep their text exactly.
  The spec has four new scenarios, and `design.md`, task 4.3, `textual-content.md` (with the
  comparison to JSX, HTML and XAML) and new tests in `tests/text_content.rs` were added or updated.
- **Verification:** Verified. Through `nxlang run`, `Total:` / `{count}` / `items` on three lines
  binds `"Total: 3 items"`, a lone multi-line run binds `"Total"` (was `"Total\n"`), `Two  spaces` +
  a wrapped line binds `"Two  spaces and a wrapped line"`, a lone `{count}` followed by a line break
  binds `"3"`, and a braced `{"  keep "}` on its own line keeps its spaces. The spec's four new
  scenarios each have a test in `tests/text_content.rs`, including raw text kept verbatim. Two
  leftovers are filed separately: design.md still says a single-run body keeps the old path (RF16),
  and a typed-text body loses its text runs (RF12).

### ✅ Verified - RF5 A value widened by a type join (`if`/`else`) keeps its source carrier in the interpreter
- **Severity:** Low
- **Evidence:** `let f(b:boolean, n:int, x:float64) = { if b { n } else { x } }` is inferred as
  `float64`, but `f(true, 3, 1.5)` evaluates to the integer `3`. It only becomes `3.0` once it
  crosses a declared site. Widening at run time happens only in `coerce_non_record_value`
  (`crates/nx-interpreter/src/interpreter.rs:3478`, `widened_to_site`). The requirement *A widened
  value takes the site's type at run time* aims at canonical output that reflects the static type,
  and an inferred return or branch join is a widening that analysis accepted.
- **Recommendation:** Either widen at inferred joins (branch results, list elements, inferred
  returns), or state in the spec that run-time widening happens only at sites with a declared type.
- **Fix:** Widened at joins. This is more than a formatting difference: `pick(true, 3, 1.5) / d`
  with `d:int` holding `2` evaluated to the integer `1` in the interpreter, while the checker typed
  it `float64` and generated JS gave `1.5`. The checker now records each `if`/`match` branch and
  list element whose numeric type is narrower than the join (`widened_joins`, including inside
  lists and nullable types). `nx_hir::apply_join_widenings` then wraps each one in a new
  `Expr::Widen`, the interpreter widens the value (item by item in a list), and codegen emits the
  branch unchanged. Inferred returns need nothing separate, since a return's type is its body's
  type. The generated JS, NX IR (`div`) and interpreter now all give `1.5`. Added a spec scenario, a
  *Joined branches* section in `reference/syntax/types.md`, checker tests for the rewrite's shape
  and the wrapper's type, and interpreter tests. The join also now lifts nullability
  (`nullable_join` in `semantics.rs`): it had typed any join with `null` as `object`, so
  `if a { n } else { if b { x } else { null } }` widened nothing. It is now `float64?` and returns
  `3.0`, and `if b { n } else { null }` is `int?`. Added a requirement for joins to the spec, design
  decision 8, and tasks 9.1–9.4.
- **Verification:** Verified. In the interpreter `pick(true, 3, 1.5)` is `3.0` and `/ d` gives
  `1.5`; match arms (`int32`/`int`/`float64`), condition-list arms, list elements (`{n x 1}` →
  `3.0 1.5 1.0`), an `if` inside a `for` body, and the nested `null` join all widen, and a join of
  one width is left alone. Generated JavaScript gives the same values for every probe it supports.
  A second analysis pass does not re-wrap, since a `Widen` already has the join's type. Gaps filed
  separately: no conformance-corpus or codegen test covers a join (RF15), and a declared nullable
  site does not accept what a join does (RF13).

### ✅ Verified - RF6 FINDINGS.md removes F14, which still reproduces
- **Severity:** Medium
- **Evidence:** The diff deletes the F14 entry and lists F14 among the findings that are gone
  (`sites/playground/docs/FINDINGS.md:6`). However, `component <Holder content kids:Element[]? />`
  with `<Holder><Leaf /></Holder>` still fails with "expected Element, got Leaf". Nothing in this
  change touches that path. The same header also lists F4 and F6 as gone, and neither was confirmed.
- **Recommendation:** Restore the F14 entry, and re-check F4 and F6 before listing them as gone.
- **Fix:** Restored F14 under *Not bugs* (it still reproduces: `expected Element, got Leaf`) and
  removed it from the header's list of findings that are gone. Rechecked the others. F6 is gone: the
  `snippet` corpus pins `"Type": "Column"` and the TS runtime tests evaluate against it. F13 is
  gone: `items:int[] = {}` evaluates. F4's premise, one combined module, no longer holds because the
  catalog is now an implicit import, and a sibling-module import of an external component with
  inherited props and defaults builds. The library-import form (NXE12/NXE13) is still listed as open
  in `docs/drawn-ui-proposal-nx-enhancements.md` and `specs/future.md`, so those were left alone.
- **Verification:** Verified. F14 is back under *Not bugs* (`FINDINGS.md:148`), and the header's list of findings that are gone no longer names it.
### ✅ Verified - RF7 `specs/future.md` still says numeric compatibility is not enforced by direction
- **Severity:** Low
- **Evidence:** `specs/future.md:153-162` says `int64 → int32` is implicitly allowed and proposes a
  directional "assignable" check. This change implements that check
  (`crates/nx-types/src/ty.rs` `widens_to`, `is_compatible_with`). Task 7.5 asks for
  `specs/future.md` to be updated.
- **Recommendation:** Delete that entry.
- **Fix:** Deleted the *Type compatibility is widening-only…* entry from `specs/future.md`.
- **Verification:** Verified. The directional-compatibility entry is gone from `specs/future.md`.
### ✅ Verified - RF8 The Accessibility readout keeps "—" placeholders where the spec says to leave the part out
- **Severity:** Low
- **Evidence:** `sites/playground/src/examples/nx/accessibility.nx:47` renders
  `"Nodes in the overlay: — · focused: — · last activated: " + last`. The playground scenario *A
  value NX cannot reach keeps its note* says the example "SHALL leave that part of the readout out".
- **Recommendation:** Render only the reachable part (`"Last activated: " + last`), or reword the
  scenario to allow a placeholder.
- **Fix:** The readout is now `"Last activated: " + last`. The comment above it says the node count
  and the focused node are left out because they are code-behind.
- **Verification:** Verified. `accessibility.nx:47` renders `"Last activated: " + last`, and the comment at lines 45-46 names the code-behind parts it leaves out.
### ✅ Verified - RF9 The Editor chat card counts whitespace-only messages, which the original skips
- **Severity:** Low
- **Evidence:** `sites/playground/src/examples/nx/editor.nx:110-111` checks only
  `action.text == ""`, but the original skips `!t.trim()` (`EditorPage.tsx:74`). The difference
  isn't noted.
- **Recommendation:** Note in the example's header that NX has no `trim`, so a blank message is
  counted. This belongs with F29's string gaps.
- **Fix:** Added a note at the submit handler in `editor.nx` saying NX has no trim, so only an empty
  message is skipped. F29's string-length bullet in FINDINGS.md now also covers trim.
- **Verification:** Verified. `editor.nx:108` notes that NX has no trim, and F29 in `FINDINGS.md:104` now covers trim.
### ✅ Verified - RF10 Task text and small leftovers do not match what was done
- **Severity:** Low
- **Evidence:**
  - Tasks 7.5 and 8.9 say to "mark F22 resolved" and "update the F22 entry", but the entry was
    deleted, following the file's own "removed when fixed" rule.
  - The doc comment on `InferenceContext::converted_literals` still reads "the integer literals that
    took a floating-point type" (`crates/nx-types/src/infer.rs:4014`).
  - `sites/playground/README.md:170,172` have ~135-character lines inside a wrapped paragraph, and
    `sites/playground/scripts/expand-components.mjs:5-7` is re-wrapped raggedly.
- **Recommendation:** Reword tasks 7.5 and 8.9 to "remove F22 (fixed)", update the doc comment to
  cover every numeric width, and re-wrap the two paragraphs.
- **Fix:** Tasks 7.5 and 8.9 now say to remove F22 (fixed). The `converted_literals()` doc comment
  now covers every numeric literal and width. Re-wrapped the *All twenty DrawnUI demo pages*
  paragraph in `sites/playground/README.md` and the header comment of `expand-components.mjs` at 100
  columns.
- **Verification:** Verified. Tasks 7.5 and 8.9 say to remove F22, the `converted_literals()` doc comment covers every width, and the named README paragraph and the `expand-components.mjs` header are wrapped at 100 columns. The README lines that still exceed 100 columns are table rows or lie outside the paragraph named in this finding.
## New Findings Discovered During 2026-09-17 03:04 Review

**Scope of this pass:** every staged and unstaged file of the change (`git diff HEAD`), including
the RF4/RF5 work: `nx-types` (ty.rs, semantics.rs, infer.rs, check.rs, env.rs), `nx-hir`
(expr.rs, components.rs, lower.rs, lib.rs, scope.rs), `nx-interpreter`, `nx-codegen`,
`runtime/typescript`, the `conversions` corpus, docs, and the untracked
`crates/nx-types/tests/nullable_joins.rs`. The `indexed_loops` / `for_loop_names` work in
`infer.rs` and `env.rs` belongs to another change and was not reviewed.  
**Checks run:** `cargo test --workspace` (green), `runtime/typescript` `pnpm test` (green), probe
programs through `nxlang run` and `nxlang codegen --target javascript`, and a throwaway
interpreter test for host-supplied values (deleted afterwards).

### ✅ Verified - RF11 A host-supplied JSON number no longer binds at an `int32` or `float32` entry point
- **Severity:** Medium
- **Evidence:** The interpreter's binding check goes through the same one-directional relation
  (`value_matches_expected_type` → `type_satisfies_expected`,
  `crates/nx-interpreter/src/interpreter.rs:3534`). A JSON number always deserializes to
  `NxValue::Int` or `NxValue::Float` (`crates/nx-value/src/lib.rs:171-190`; serde_json never calls
  `visit_i32`/`visit_f32`), and `from_nx_value` maps those to `Value::Int`/`Value::Float`. With
  `let f(n:int32) = { n + 1 }` and `let g(w:float32) = { w * 2 }`, calling `f` with `Value::Int(7)`
  now fails with "expected int32, got int", and `g` with `Value::Float(1.5)` fails with "expected
  float32, got float64". Both were accepted before this change. Any host that passes props or
  arguments as JSON to a component or function declaring `int32`/`float32` breaks at run time, and
  the host has no way to spell the narrower width in JSON. The tests were adapted by passing
  `Value::Int32(7)` (`tests/floats.rs`), so nothing covers the host path.
- **Recommendation:** Treat a value that arrives from outside the checked program differently from
  one the checker typed: at the host boundary (or in `coerce_non_record_value`), convert an `Int`
  that fits `i32` to `Int32` at an `int32` site and a `Float` to `Float32` at a `float32` site,
  range-checked, with a run-time error when it does not fit. The static rule is unaffected, since
  the checker already guarantees no checked expression narrows. Add an `nx-api` test that
  evaluates an `int32`/`float32` entry point from JSON input, and say in the spec that a host value
  takes the declared width.
- **Fix:** The interpreter now gives a host-supplied number the width of its site
  (`Interpreter::host_number_at_site`, called from `coerce_non_record_value`). At an `int32` site an
  `Int` becomes `Int32` after a range check. At a `float32` site an `Int` becomes `Float32` after an
  exactness check, and a `Float` is rounded, on the same terms as a literal. A value that does not
  fit fails with a type mismatch ("out of range for int32", "not exact as a float32"). This covers
  props, state and dispatch batches (`ValueOrigin::Host`) and entry-call arguments. Entry-call
  arguments use a new `ValueOrigin::EntryArgument`, which narrows numbers but still passes records
  through, as the existing `simple_functions.rs` tests require. Values from checked NX are
  unchanged. Added two `nx-api` tests that evaluate a component from JSON props and state (`7` →
  `int32`, `0.1` and `2` → `float32`, `3000000000` refused), a `tests/floats.rs` test for entry
  arguments (including a `float32?[]` list and the two refusals), a spec requirement *A number the
  host supplies takes the width of its site* with two scenarios, and task 10.1.
- **Verification:** Verified. Through a temporary interpreter test, `Value::Int(7)` at an `int32`
  parameter becomes `Int32(7)`, `i32::MIN` is accepted, `2147483648` is refused with "out of range
  for int32", `16777217` at `float32` is refused as not exact, and a `float64` at `int32` is still
  refused. The two `nx-api` tests pass, and they cover the embedding path: host props and state go
  through `ValueOrigin::Host`, which rebuilds records, so nested fields are narrowed too. One
  residual gap, on the Rust-only `execute_function` API: a record passed as an entry argument is
  passed through unchanged (by design), so its `int` field stays `Int`, and
  `let g(p:P): int32 = { p.n }` fails with "expected int32, got int". No host binding calls
  `execute_function` with arguments, so this is not reopened.

### ✅ Verified - RF12 A typed-text body at a `string` content property silently drops its text
- **Severity:** Medium
- **Evidence:** `<Label:md>a @{n} b</Label>` with `content text:string` and `n = 2` binds `"2"`.
  Lowering handles only `TEXT_RUN` and `RAW_TEXT_RUN` (`crates/nx-hir/src/lower.rs`
  `lower_element_content`); an `EMBED_TEXT_RUN` is not a content piece at all, so the body is the
  lone piece `n`. Before this change that was a type error (`expects string, found int`), which at
  least told the author. The new lone-braced-number rule (RF2's fix) now accepts it and the text
  "a" and "b" disappears without a diagnostic. The same body with no `@{}` still fails at run time
  ("expected string, got object?"), so typed text at a `string` property does not work either way.
- **Recommendation:** Lower `EMBED_TEXT_RUN` as a text piece (recording it in `text_runs` and the
  gaps) so a typed-text body joins like a plain one, or reject a typed-text body at a `string`
  content property during analysis until it is supported. Add a test for either outcome.
- **Fix:** Lowering now treats an `EMBED_TEXT_RUN` as text, like a `TEXT_RUN`, and records
  it in `text_runs` (`lower_element_content`), so a typed text body joins like a plain one.
  `<Label:md>a @{n} b</Label>` now binds `"a 2 b"`. This also stops a typed text body under any
  tag from silently losing its text (`<p:md>hello @{n}</p>` used to lower to just `n`). The text
  type itself is still not represented in HIR, as before. Added
  `a_typed_text_body_joins_like_a_plain_one` (multi-line with `@{}`, and text only), a spec
  sentence and scenario, a design note, a sentence in `textual-content.md`, and task 10.2.
- **Verification:** Verified. `<Label:md>a @{n} b</Label>` binds `"a 2 b"`, a multi-line typed body
  joins with its line breaks collapsed, a text-only typed body binds `"only text"` (it used to fail
  at run time), and `<p:md>hello @{n} there</p>` keeps its text. The escape `\@` now reaches the
  string verbatim, which is filed as RF21.

### ✅ Verified - RF13 `int?` is rejected at a `float64?` site, and `int?[]` at `float64?[]`, though a join accepts the same widening
- **Severity:** Medium
- **Evidence:** `let a:int? = {3}` then `let b:float64? = {a}` reports "expects float64?, found
  int?", and so does `let f(n:int?): float64? = { n }`. `Type::is_compatible_with`
  (`crates/nx-types/src/ty.rs:497-503`) only tries `self` against the inner type of a nullable
  `other`, so `Nullable(int)` is compared with `float64` and fails; there is no
  `Nullable`/`Nullable` case. Arrays are covariant (`int[]` binds at `float64[]`), and the join
  requirement types `int?` with `float64` as `float64?` and says a branch widens "on the same terms
  as a value bound at a declared site, including ... inside a nullable type". So
  `if b { n } else { x }` with `n:int?` is accepted and widened, while binding the same `n` at a
  declared `float64?` is refused. The conversion is total and exact (`null` stays `null`), so the
  rule admits it. The interpreter's site coercion already takes nullable types apart.
- **Recommendation:** Add a `(Nullable(a), Nullable(b)) => a.is_compatible_with(b)` case to
  `is_compatible_with` (and the matching case in `type_satisfies_expected` if it does not
  delegate), with checker tests for `int?` at `float64?`, `int?[]` at `float64?[]`, and the
  still-rejected `int64?` at `float64?`, plus an interpreter test that the value arrives as `3.0`.
  Add a spec scenario under *Numeric types widen in one direction*.
- **Fix:** Added a `(Nullable(a), Nullable(b))` case to both `Type::is_compatible_with` and
  `InferenceContext::type_satisfies_expected`. `int?` now binds at `float64?`, and `int?[]` at
  `float64?[]`. `int64?` at `float64?` is still rejected, and the diagnostic names both types. The
  interpreter already took nullable and list types apart, so nothing changed at run time. Added
  checker cases (`let b:float64? = {a}`, a nullable return type, a list of nullables, and the
  `int64?` rejection), an interpreter test (`3` → `3.0`, `null` → `null`, `[3, null]` →
  `[3.0, null]`), a spec scenario *A nullable value widens at a nullable site*, and task 10.3.
- **Verification:** Verified. `let b:float64? = {a}` with `a:int?` gives `3.0`, a `null` stays
  `null`, `int?[]` at `float64?[]` gives `{1.0 null}`, and `let f(n:int?): float64? = { n }` gives
  `4.0`. `int64?` at `float64?`, `int64?` at `int?` and `int32?` at `string?` are still rejected, and
  each diagnostic names both types.

### ✅ Verified - RF14 A computed `float32` differs between the interpreter and the JavaScript backends once it is widened or compared
- **Severity:** Low
- **Evidence:** RF1 fixed the text form only. With `let w:float32 = 2.3` and
  `let scaled(v:float32): float64 = { v * 3 }`, the interpreter returns `6.899999618530273` (the
  `float32` product, widened) and generated JavaScript returns `6.8999998569488525` (the unrounded
  `float64` product). `v * 3 == 6.899999618530273` is `true` in the interpreter and `false` in
  JavaScript. The `nx-ir-format` spec says a widening "is unobservable" on a runtime that carries
  every number as a `number`; that holds for a stored `float32` but not for `float32` arithmetic,
  which this change makes reachable by letting literals take `float32` (`w * 1.5` stays `float32`).
- **Recommendation:** Round `float32`-typed arithmetic results in the JS targets (`Math.fround`
  around a binary node whose checked type is `float32`, in `emit.rs` and the IR runtime via the
  static type), and add a corpus case. If that is out of scope, record it as a known divergence in
  design.md and soften the "unobservable" sentence in the spec.
- **Status:** Not fixed. Rounding `float32` arithmetic in generated JavaScript is simple
  (`emit.rs` knows each expression's type), but the IR runtime has no static types. It would need a
  new operator or node, such as a `float32` variant of the arithmetic operators, which is an IR
  schema decision. Fixing only one JavaScript target would leave the two disagreeing. Recommend
  either adding a `fround`-style IR node the emitter wraps around each `float32`-typed arithmetic
  result (schema 4 is still unreleased, so now is the cheap time), plus `Math.fround` in `emit.rs`
  and a corpus case, or recording the divergence in design.md and softening the "unobservable"
  sentence in the `nx-ir-format` spec.
- **Fix:** Added the IR binary operators `fadd32`, `fsub32`, `fmul32` and `fdiv32` (`16`–`19`,
  still schema 4). `binary_operator` in `ir.rs` picks them when the expression's checked type is
  `float32`; `mod` stays, since a `float32` remainder is exact. The TS runtime evaluates them as the
  `float64` operation under `Math.fround`, and `emit.rs` wraps `float32` `+ - * /` in
  `Math.fround`. Because the fix relies on every held `float32` being rounded, both JS targets now
  round a host number at a `float32` site (TS runtime `normalizePrimitiveValue`; generated
  JavaScript's new `nxFloat32Schema`) and refuse an integer a `float32` does not hold exactly. The
  TS runtime also checks `int32` sites, matching RF11. Coverage: `float32Widened`
  (`6.899999618530273`), `float32Compared` (`true`) and `float32StepsOf` (all four operators) in
  the `conversions` corpus, `ir_tests.rs` `float32_arithmetic_is_the_float32_operator`, four
  generated-JavaScript agreement cases, a generated float32-prop test, and two TS runtime tests.
  Documented in `docs/nx-ir-format.md`, the `nx-ir-format` spec delta (new requirement) and
  design.md Decision 9. `int32` overflow (the interpreter wraps, JavaScript does not) is left to
  the deferred range-enforcement change.
- **Verification:** Verified for `float32` arithmetic outside a join. The interpreter and generated
  JavaScript agree on a widened product (`6.899999618530273`), a comparison (`true`), a chain of all
  four operators, negation, and the text form. `ir explain` shows `fmul32`, and the corpus cases
  pass on the TS runtime (with-debug and stripped). A `float32` operation *inside a widened join
  branch* still diverges (`if b { v * 3 } else { x }` emits `mul`), because of how the builder
  types a `Widen`. That is filed as RF19.

### ✅ Verified - RF15 The join scenario's "on every backend" clause has no cross-backend test
- **Severity:** Low
- **Evidence:** The spec scenario *A narrower branch of a join evaluates at the join's type* says
  the division "SHALL yield `1.5` on every backend", and RF5's fix note says the NX IR now gives
  `1.5`. `specs/ir-conformance/conversions/main.nx` has no `if`/match/list join, and nothing in
  `crates/nx-codegen/src` (`tests.rs`, `ir_tests.rs`) mentions `Widen` or a joined branch, so the
  builder's `Expr::Widen` arm (`builder.rs:1232`, which re-types the branch) and the `div`-not-
  `idiv` choice after a join are untested. There is also no interpreter test for a widened match
  arm; only `if`, lists and nullable joins are covered in `tests/floats.rs`. Manual probes pass.
- **Recommendation:** Add `pick(true, 3, 1.5) / 2`, a match-arm join and a list join to the
  `conversions` corpus (checked by the interpreter, the IR runtime and generated JavaScript), an
  `ir_tests.rs` case asserting `div`, and a match-arm case in `tests/floats.rs`.
- **Fix:** Added `joinedIf` (`pick(true, count, 1.5) / 2`), `joinedMatch` (a match arm
  `/ 2`) and `joinedList` (`count 1.5`) to the `conversions` corpus. The interpreter and the TS
  runtime (with-debug and stripped) now check them, giving `1.5`, `1.5` and `[3.0, 1.5]`, and the
  manifest now lists `conversions` under `if` and `ifIs`. Added the `ir_tests.rs` case
  `a_division_of_a_widened_join_is_the_floating_point_operator`, which asserts `div` for an `if`
  join and a match join, with no `idiv` and no `text<`. Added an `if`-join case to the
  generated-JavaScript agreement test. Generated JavaScript does not support match expressions yet,
  and it prints a list's `3.0` as `3`, so those two join forms are covered by the corpus instead.
  Added the `tests/floats.rs` test `test_a_narrower_match_arm_evaluates_as_the_join`
  (`int32`/`int`/`float64` arms) and task 10.4.
- **Verification:** Verified. `joinedIf`, `joinedMatch` and `joinedList` are in the corpus with
  `1.5`, `1.5` and `[3.0, 1.5]`, the manifest lists `conversions` under `if` and `ifIs`, and
  `a_division_of_a_widened_join_is_the_floating_point_operator` asserts `div` for both joins. These
  cover a division *of* a join. A division *inside* a widened branch is wrong in the IR (RF19), and
  no test covers that case.

### ✅ Verified - RF16 design.md and a test name still describe the single-run path RF4 removed
- **Severity:** Low
- **Evidence:** RF4's fix made a lone text run a one-piece join, so `<Label>` + newline + `Total` +
  newline now binds `"Total"` where it bound `"Total\n"`. design.md still says the opposite in two
  places: Decision 7 ends "A single-run body keeps today's path" (`design.md:147`), and the risk
  entry reads "The spec keeps the single-run path as today; only multi-piece bodies trim"
  (`design.md:193-194`). The Implementation Notes below them say the reverse. The test
  `a_single_run_body_binds_as_it_always_has` (`tests/text_content.rs:93`) only uses a body with no
  surrounding whitespace, so its name claims more than it checks. This is a behavior change for
  existing programs with a multi-line single-run `string` body, and proposal.md does not list it.
- **Recommendation:** Update Decision 7 and the risk entry to say a single-run body is trimmed and
  its line breaks collapsed, mention the change in proposal.md's *What Changes*, and rename the
  test (the multi-line single-run case already has its own test).
- **Fix:** Decision 7 now says a single-run body is a one-piece join, trimmed and with its
  line breaks collapsed. The risk entry now accepts the behavior change and gives the `"Total\n"` →
  `"Total"` example. proposal.md's *What Changes* describes line-break layout, trimming, and the
  single-run behavior change. The test is renamed to
  `a_body_of_one_text_run_binds_as_that_text`.
- **Verification:** Verified. Decision 7 (`design.md:163`) and the risk entry (`design.md:236-238`)
  now describe the one-piece join and give the `"Total\n"` → `"Total"` example,
  `proposal.md:48` lists the behavior change, and the test is renamed
  (`tests/text_content.rs:94`).

### ✅ Verified - RF17 Constant arithmetic cannot reach a `float32` or `int32` site, and an out-of-range literal operand is an error where promotion would do
- **Severity:** Low
- **Evidence:** `<B v={1.5 * 2} />` with `v:float32` reports "expects float32, found float64": a
  literal takes a width only from a declared site or from a non-literal sibling operand, so an
  all-literal expression is always `int`/`float64` and then cannot narrow. Without the explicit
  conversions this change defers, the only fix for the author is to fold the arithmetic by hand.
  Separately, `n32 > 3000000000` with `n32:int32` is rejected ("3000000000 is out of range for
  int32") although the comparison is well defined at `int`; the literal is forced to the other
  operand's width before promotion is tried (`infer.rs` `convert_literal_operand`). Both follow
  the letter of the spec ("on the same terms"), so this is a spec gap more than a code bug.
- **Recommendation:** Pass the site's expected numeric type down into a binary operator whose
  operands are all literals (or document the limit in `reference/syntax/types.md`). For a literal
  operand that does not fit the other operand's type, fall back to ordinary promotion instead of
  reporting, or state in the spec that it is an error.
- **Status:** Not fixed. Both parts are open language-design choices (see the question
  below), not clear defects. One is whether an all-literal expression should take its site's
  numeric type. The other is whether an out-of-range literal operand should fall back to
  promotion. Recommend deciding the second first. Falling back to promotion for a literal that
  does not fit is the more permissive choice, and it matches how the operation is well defined at
  `int`. Then either pass the expected type into all-literal binary operators, or document the
  limit in `reference/syntax/types.md`.
- **Fix:** Adopted the Go/Ada constant rule. An arithmetic expression whose operands are all numeric
  literals is folded (`fold_constant` in `infer.rs`) at its literals' own types, so `7 / 2` is `3`
  at every site, and the result is then converted as a written literal would be:
  `<B v={1.5 * 2} />` binds at `float32`, `n32 * (2 + 3)` is `int32`, and `1000000 * 3000` at
  `int32` is out of range. `nx_hir::apply_constant_folds` replaces the expression with the folded
  literal. Division by zero and `int` overflow in a folded constant are compile-time errors. The
  out-of-range literal operand (`n32 > 3000000000`) stays an error, as in Go, Rust and Swift, and
  the spec and docs now say so. Added the spec text and three scenarios, design.md notes under
  Decision 6, the "Constant expressions" section in `reference/syntax/types.md`, a sentence in the
  language tour, five `tests/floats.rs` tests and an `ir_tests.rs` case.
- **Verification:** Verified. `<B f={1.5 * 2} />` binds `float32` `3.0`, `i={n32 * (2 + 3)}` binds
  `25`, `d={7 / 2}` binds `3.0`, `f={1 + 0.1}` rounds to a `float32`, and `-(3 - 5)` folds.
  `1000000 * 3000` at `int32` is out of range, `16777217 + 0` at `float32` is not exact, and at a
  declared site `1 / 0` and `i64::MAX + 1` are compile-time errors. With no site, `1 / 0` is still a
  run-time error and `i64::MAX + 1` still wraps, which matches the spec's "when a site gives it a
  type other than the one it infers". The `n32 > 3000000000` error is now specified.

### ✅ Verified - RF18 The tests task 9.3 names are untracked
- **Severity:** Low
- **Evidence:** `crates/nx-types/tests/nullable_joins.rs` is `??` in `git status`, while the rest of
  the change is staged. Task 9.3 cites it as the coverage for the nullable joins. A commit of the
  staged tree would drop it.
- **Recommendation:** `git add` the file with the rest of the change. (`for_loop_names.rs` is also
  untracked but belongs to another change.)
- **Fix:** Ran `git add crates/nx-types/tests/nullable_joins.rs`. `for_loop_names.rs` was
  left untracked, since it belongs to another change.
- **Verification:** Verified. `git status` shows `A  crates/nx-types/tests/nullable_joins.rs`.

## New Findings Discovered During 2026-09-17 13:19 Verification

### ✅ Verified - RF19 An operator inside a widened join branch is emitted at the join's type, so IR and JavaScript compute a different value
- **Severity:** High
- **Evidence:** The builder's `Expr::Widen` arm builds the branch and then overwrites its type with
  the join's (`widened.ty = ty`, `crates/nx-codegen/src/builder.rs`, the arm that begins "A widening
  changes nothing a generated runtime can observe"). `binary_operator` in `ir.rs` and the
  `Math.fround` wrapping in `emit.rs` then read that overwritten type, so the branch's outermost
  operator is chosen for the join and not for its own operands:
  - `let half(b:boolean, n:int, x:float64) = { if b { n / 2 } else { x } }` explains as
    `(n div 2)`, not `idiv`. `half(true, 7, 0.5)` is `3` in the interpreter (integer division, then
    widened) and `3.5` in the NX IR and in generated JavaScript.
  - `let pickf(b:boolean, v:float32, x:float64) = { if b { v * 3 } else { x } }` explains as
    `(v mul 3.0)`, not `fmul32`, and generated JavaScript omits `Math.fround`. With `v = 2.3`, the
    interpreter gives `6.899999618530273` and JavaScript gives `6.8999998569488525`, the RF14
    divergence again.
  The same applies to match arms and list elements, which share the arm. Only the outermost
  operator is affected: in `n % 4 - 7 / 2` the inner `imod`/`idiv` are still correct. The widening
  is not "unobservable" here, because it changes which operator is emitted. The existing tests only
  check a division applied to a join's result.
- **Recommendation:** Keep the branch's own type on its expression. Either leave `widened.ty`
  alone, or add a transparent wrapper kind whose type is the join's and whose inner expression
  keeps its own type, and have the emitters look through it. Add `ir_tests.rs` cases asserting
  `idiv` and `fmul32` inside a widened `if` branch, a match arm and a list element, plus
  `conversions` corpus programs (`half(true, 7, 0.5)` → `3.0`, and the `float32` product inside a
  join).
- **Fix:** The builder's `Widen` arm now returns the branch as built, with its own type, so
  `binary_operator` and the `Math.fround` wrapping read the branch's type. The IR now explains the
  first two examples as `(n idiv 2)` and `(v fmul32 3.0)`. Added
  `an_operator_in_a_widened_branch_is_chosen_by_the_branch_type` (`if`, match arm, list element),
  four `conversions` corpus programs (`joinedIntegerDivision`, `joinedArmDivision`,
  `joinedListDivision` → `3.0`/`[3.0, 0.5]`, `joinedFloat32Product` → `6.899999618530273`), and
  two generated-JavaScript agreement cases. The spec's join requirement gained a scenario, and
  design.md Decision 8 explains the fix.
- **Verification:** Verified. `ir explain` now shows `(n idiv 2)` and `(v fmul32 3.0)` inside the
  branches, and `(if true then (n idiv 2) else x)` inside a list literal. `half(true, 7, 0.5)` is
  `3` in the interpreter, generated JavaScript and the IR alike, and the `float32` product is
  `6.899999618530273` in all three. A widened join inside a string `+` still renders through the
  checker's `text<float64>`, and a list join gives `[3, 0.5]` in JavaScript. The four corpus
  programs and their expected results are in place, and both test suites are green.

### ✅ Verified - RF20 Generated JavaScript does not truncate integer division
- **Severity:** Low
- **Evidence:** `let a(n:int) = { n / 2 }` emits `return (n / 2);`, so `a(7)` is `3.5` in generated
  JavaScript, while the interpreter and the NX IR (`idiv`) give `3`. `let c() = { 7 / 2 }` is
  also `3.5`. `binop_text` maps `BinOp::Div` to `/` regardless of type (`emit.rs`), as it did at
  HEAD, so this predates the change. It still contradicts this change's scenario *Integer division
  stays integer* (`f(7, 2)` SHALL produce `3`) for one of the three backends the spec requires to
  agree. The corpus checks `integerDivisionOf` only on the interpreter and the IR runtime.
- **Recommendation:** Emit `Math.trunc(a / b)` (with the IR runtime's divide-by-zero behavior) for
  an integer-typed `/`, add `integerDivisionOf` to the generated-JavaScript agreement test, or
  record the gap as a known limitation of the JavaScript target in design.md and specs/future.md.
- **Fix:** An integer-typed `/` is emitted as `nxIntDiv(a, b)`, a new helper in both runtime
  preludes. It truncates toward zero, normalizes `-0`, and throws "Division by zero" as the
  interpreter does. `a(7)` is now `3` and `a(-7)` is `-3`. Added generated-JavaScript agreement
  cases for `n / 2` and `integerDivision(7, 2)`. design.md Decision 3 records it. A remainder
  by zero is still `NaN` in generated JavaScript, as is a float division by zero, which is
  `Infinity`; both predate this change.
- **Verification:** Verified. `nxIntDiv` truncates toward zero and normalizes `-0`: `a(7)` is `3`,
  `a(-7)` is `-3`, `b32(-9, 2)` is `-4` and `m64(9, -2)` is `-4`, all matching the interpreter. A
  division by zero throws "Division by zero", as the interpreter does. A mixed `int`/`float64`
  division still uses `/` and gives `3.5`. The remaining divergences the fix note names (remainder
  by zero, float division by zero) are filed as RF22 so they are not lost.

### ✅ Verified - RF21 Escape sequences reach a joined `string` body verbatim
- **Severity:** Low
- **Evidence:** RF12's fix makes typed-text runs part of the join, and the docs say to "Escape `@`
  as `\@` when you need a literal at-sign" (`language-tour/textual-content.md`). However,
  `<Label:md>at \@ sign @{n}</Label>` binds `"at \@ sign 2"`, with the backslash kept, in both the
  interpreter and generated JavaScript. Lowering copies a run's source text, including its
  `escaped_at` / `escaped_lbrace` / `escaped_rbrace` tokens and entities (`x &lt; y` binds
  `"x &lt; y"`), and this change is what turns that text into a user-visible string. (Separately,
  `\{` in a plain `<Label>` body does not parse at all, but that is a grammar issue outside this
  change.)
- **Recommendation:** Decode the escapes, and the entities if the open question below is answered
  yes, when lowering a text run into a content string, and add a test. At minimum, `\@` should
  yield `@` as the docs promise.
- **Fix:** `text_run_value` in `lower.rs` replaces each `escaped_at`, `escaped_lbrace` and
  `escaped_rbrace` child of a run with its character, so every backend binds
  `"at @ sign {x} 2"`. Entities are still kept as written, pending the open question below.
  Added `an_escape_in_a_typed_text_body_binds_as_the_character_it_escapes` and a spec scenario.
- **Verification:** Verified. `<Label:md>at \@ sign \{x\} @{n}</Label>` binds `"at @ sign {x} 2"`
  in both the interpreter and generated JavaScript, and a lone `\@` binds `"@"`. Raw text still
  keeps its backslashes (`<Label:raw>` binds `raw \@ \{ kept`), and a backslash that escapes
  nothing is kept, which is right since those three are the only escapes. Entities are still
  verbatim, which the open question covers. `\{` in a plain (untyped) body still does not parse,
  the pre-existing grammar issue this finding excluded.

## New Findings Discovered During 2026-09-17 13:48 Verification

### ✅ Verified - RF22 Generated JavaScript returns NaN or Infinity where the other two backends fail on a division by zero
- **Severity:** Low
- **Evidence:** RF20's fix routes an integer-typed `/` through `nxIntDiv`, which throws "Division by
  zero" as the interpreter does, but its siblings were left alone. `let r(n:int, m:int) = { n % m }`
  with `m` zero is a run-time error in the interpreter and an `nx-ir-division-by-zero` failure in
  the IR runtime (`imod` in `runtime/typescript/src/index.ts`), while generated JavaScript returns
  `NaN`. `let d(x:float64, y:float64) = { x / y }` with `y` zero is likewise an error in the
  interpreter and the IR runtime (`mod`/`div` both check the divisor) but `Infinity` in generated
  JavaScript. So a program that divides by zero fails on two backends and silently produces a
  non-number on the third. This predates the change and the fix note for RF20 names it; it is filed
  so it is not lost.
- **Recommendation:** Give `%` and the floating-point `/` the same treatment as `nxIntDiv`, with
  helpers that throw "Division by zero", and add a generated-JavaScript agreement case for each.
  Otherwise record the divergence in design.md and `specs/future.md` as a known limitation of the
  JavaScript target.
- **Fix:** Every `/` and `%` now goes through a helper. `nxIntDiv` truncates, `nxDiv` divides and
  `nxMod` takes a remainder, and each raises "Division by zero" on a zero divisor, the message the
  interpreter uses. A `float32` quotient is `Math.fround(nxDiv(...))`, so the RF14 rounding is kept.
  Added `generated_javascript_fails_on_a_division_by_zero_like_the_interpreter` (`int` division,
  `int` remainder, `float64` division and `float32` division) and three agreement cases for a
  remainder and a `float32` quotient. design.md Decision 3 records it.
- **Verification:** Verified. In generated JavaScript, `int` division, `int` remainder, `float64`
  division and remainder, `float32` division and remainder, and a mixed `int`/`float64` division all
  throw "Division by zero" on a zero divisor, and the interpreter fails the same way on all seven.
  The non-zero results still agree exactly, `float32` rounding included (`2.3 / 3` is `0.76666665`
  and `2.3 % 2` is `0.29999995` in both). The IR runtime's `fdiv32` checks its divisor too.

## New Findings Discovered During 2026-09-17 13:57 Verification

### ✅ Verified - RF23 A typed text body collapses its line breaks, so a Markdown body loses its structure
- **Severity:** Medium
- **Evidence:** RF12's fix lowers an `EMBED_TEXT_RUN` as text, which puts a typed body through the
  plain-body layout rule: every whitespace stretch containing a line break becomes one space. A
  `<Note:markdown>` body of a heading, a paragraph and a two-item list binds
  `"# Title First paragraph. - item one - item two"` — one line, with the blank lines and list
  structure gone, so a Markdown renderer sees a single paragraph. `specs/future.md:557` predicts
  exactly this ("The plain-body rule that line breaks read as one space would break Markdown, so a
  typed body should most likely keep its text exactly, apart from removing the common
  indentation"), but the rule was applied to typed bodies anyway. Before the change the text was
  dropped entirely, so this is not a regression from `main`, but `:markdown` is a documented
  feature of the language tour and the text it now produces is wrong for it.
- **Recommendation:** Keep a typed body's text as written, minus the common indentation, the way
  the `specs/future.md` entry proposes, and join only the `@{}` values into it. Failing that, state
  in the spec and in `textual-content.md` that a typed body is laid out like a plain one and that a
  text processor sees one line, so the limitation is at least recorded. Add a test with a
  multi-paragraph Markdown body either way.
- **Fix:** Lowering records the tag's text type on `Element` (`text_type`), and the join gives a
  typed body its own layout rule (`dedent_typed_body` in `components.rs`): the indentation its
  lines share comes off, as do the line break after the open tag and a whitespace-only line before
  the close tag, and every other line break is kept. The RF23 body now binds
  `"# Title\n\nFirst paragraph.\n\n- item one\n- item two"`, and a line indented past the shared
  indentation keeps the rest, so a nested list survives. A plain body is unchanged. The spec's
  typed-body requirement and scenario, the language tour and design.md Decision 7 say so; tests:
  `a_typed_text_body_keeps_its_line_breaks_and_loses_its_indentation`,
  `a_plain_body_still_reads_its_line_breaks_as_layout`, and a `markdownNote` corpus case that pins
  the same text on every backend.
- **Verification:** Verified. The RF23 body now binds
  `"# Title\n\nFirst paragraph.\n\n- item one\n- item two"`, and a body with a nested list and an
  indented code block keeps both (`"- item one\n  - nested\n- item two\n\n    code block"`), so only
  the shared indentation comes off. A tab-indented body works the same. A plain body still reads its
  line breaks as layout (`"Total: 2 items"`), a one-line typed body binds `"a 2 b"`, raw text is
  untouched, and a typed body under an undeclared tag (`<p:md>`) is unchanged. Generated JavaScript
  produces the same string as the interpreter, and the `markdownNote` corpus case pins it for the IR
  runtime. The requirement and its two scenarios match the behavior. One detail the spec leaves
  implicit: a one-line typed body keeps its leading and trailing spaces (`"   lead and trail   "`),
  where a plain body trims them. That follows from "SHALL NOT lose its text" but is worth a sentence
  if it was not deliberate.

### ✅ Verified - RF24 The `specs/future.md` entry on typed text bodies describes behavior the change already fixed
- **Severity:** Low
- **Evidence:** `specs/future.md:557-576`, added by this change, says lowering "drops everything
  except the `@{...}` values", that `lower_element_content` "has no arm for `EMBED_TEXT_RUN`", that
  `**Bold** and _italic_ @{1} x` binds `"1"`, and that "no test covers a typed body today". RF12
  and RF21 fixed all of that: the same body now binds `"**Bold** and _italic_ 1 x"`, lowering has
  the arm, the escapes are decoded, and `a_typed_text_body_joins_like_a_plain_one` covers it. The
  entry's first bullet ("Lower `EMBED_TEXT_RUN` as text, decoding `\@`, `\{` and `\}`") is done.
  What is still true is narrower: the text type is not recorded on `Element`, and what a text
  processor should receive is undecided. Separately, this change writes future-work entries into
  two different files — `specs/future.md` (escapes, typed bodies) and the root `future.md`
  (entities, `int32` overflow) — which makes the open questions harder to find.
- **Recommendation:** Rewrite the entry to describe what remains (the unrecorded text type, the
  layout question from RF23, what a host processor receives) and drop the parts that are fixed.
  Decide which of the two future-work files this change's entries belong in, and put them all
  there.
- **Fix:** The entry is rewritten as "Typed Text Bodies: What A Text Type Means", covering what is
  still open — what a text type gives a host, whether an unknown one is an error, and the entity
  question — and the fixed parts are gone. The root `future.md` is removed and its three entries
  (logical operands, `int32` overflow, entities) now live in `specs/future.md`, which every other
  change already uses, so there is one future-work file.
- **Verification:** Verified. The entry is now "Typed Text Bodies: What A Text Type Means"
  (`specs/future.md:557`) and describes only what is open: what a text type gives a host, whether an
  unknown one is an error, and the entity interaction. The root `future.md` is deleted (`D` in
  `git status`), its one committed entry moved verbatim, and the two entries this change added moved
  with it, so all three are in `specs/future.md`. No reference to the old path remains (the
  `nx-planning-future.md` mentions are a different file).

## New Findings Discovered During 2026-09-17 14:20 Verification

### ✅ Verified - RF25 Re-wrapped paragraphs in the change's own documents run past the column the rest of the file keeps
- **Severity:** Low
- **Evidence:** The fix passes edited paragraphs in place without re-wrapping them. `design.md` has
  six lines over 100 columns, up to 159 (the Decision 3 sentence on `Infinity` or `NaN`, the
  constant-folding sentence on `1 / 0`, the Decision 8 sentence on a `div` branch, and the
  *Line breaks are layout* note), and `specs/implicit-primitive-conversions/spec.md:271` is 112
  columns where the sentence about the open tag and the close tag was inserted mid-paragraph. Every
  surrounding paragraph in these files wraps near 100, and RF10 already fixed this class of slip
  once in the README and a script header.
- **Recommendation:** Re-wrap the six `design.md` paragraphs and the one spec paragraph at the
  column the rest of each file uses. The two other over-long spec lines are single `WHEN` lines
  that are mostly inline code, and are best left alone.
- **Fix:** Re-wrapped the four `design.md` paragraphs at 100 columns (Decision 3's division
  helpers, Decision 6's constant folding, Decision 8's widened branch, and the two layout bullets,
  which are now one bullet each again) and the spec's text-body requirement paragraph. The two
  remaining over-long `design.md` lines are the Decision 3 and Decision 7 headings, which predate
  the fix passes and cannot be wrapped. The two `WHEN` lines are left alone as recommended.
- **Verification:** Verified. No prose paragraph in `design.md` or the spec deltas exceeds 100
  columns now, and the re-wrapped text reads correctly with nothing lost or run together: Decision 3
  keeps both the `nxIntDiv` sentence and the `nxDiv`/`nxMod` one, Decision 8 keeps the explanation
  that the branch keeps its own type, the *Line breaks are layout* and *A typed body keeps its line
  breaks* notes are one bullet each, and the text-body requirement still carries the typed-body and
  escape sentences. What is left over 100 columns is the two `### 3.`/`### 7.` headings (which
  cannot wrap), the two `WHEN` lines this finding excluded, and four pre-existing lines in
  `proposal.md`, `tasks.md` and the `contextual-numeric-literals` delta that no fix pass touched.

## Summary
- The core design is implemented as specified and well tested: a single widening lattice, the
  checker owning the `+` decision, `Concat`/`ToText` in HIR, the `text` IR node and schema 4. All
  Rust, TS runtime and playground suites pass, and the old `BinOp::Concat` is fully gone.
- RF1 is a real cross-backend text divergence with a small fix. RF2 is a spec/docs gap that authors
  will hit quickly. RF6 removes a finding that is still live. The rest are minor consistency and
  wording issues.
- 2026-09-17 pass: RF4 and RF5 verified; every earlier finding is now verified. Eight new findings
  (RF11–RF18). RF11 (host JSON numbers refused at `int32`/`float32` entry points), RF12 (typed-text
  body silently loses its text) and RF13 (`int?` refused at `float64?`) are the ones to fix before
  archiving; the rest are test, docs and spec-gap items.
- (2026-09-17 fix pass) RF11, RF12, RF13, RF15, RF16 and RF18 are fixed. RF14 (IR `float32`
  arithmetic rounding) and RF17 (literal arithmetic at narrow sites) are left open as design
  decisions. `cargo test --workspace` and the `runtime/typescript` suite are green.
- (2026-09-17, second pass) RF14 is fixed with dedicated `float32` IR operators, and RF17 by
  folding constant expressions before they take a site's type.
- (2026-09-17 13:19 verification) RF11–RF18 verified. Three new findings: RF19 (High), where a
  widened join branch is emitted at the join's type, so `idiv` becomes `div` and `fmul32` becomes
  `mul` in the IR and JavaScript; RF20 (Low, predates this change), where generated JavaScript
  does not truncate integer division; and RF21 (Low), where escape sequences reach joined text
  verbatim. `cargo test --workspace` and the `runtime/typescript` suite are green.
- (2026-09-17, third fix pass) RF19, RF20 and RF21 are fixed. `cargo test --workspace`, the
  `runtime/typescript` suite (225 ok) and `openspec validate` pass.
- (2026-09-17) All three questions are settled, so the Questions section is removed:
  - Integers beyond ±2^53: the spec's canonical-text requirement now limits the backends-agree
    guarantee to `int`'s specified range, and defers a larger `int64` to the `int64` carrier
    change in `primitive-type-names`.
  - `float32` in `nxlang run`: `format.rs` now prints a `float32`'s own shortest digits
    (`<F v=0.1 />`), with a test.
  - Entity decoding: recorded in `specs/future.md` ("Entities in text content are kept as
    written").
- (2026-09-17 13:48 verification) RF19, RF20 and RF21 verified: the IR and generated JavaScript now
  choose an operator by the branch's own type, integer division truncates in JavaScript, and text
  escapes are decoded. One new finding, RF22 (Low, predates the change): generated JavaScript
  returns `NaN` or `Infinity` for a remainder or float division by zero where the interpreter and
  the IR runtime fail. `cargo test --workspace` (exit 0) and the `runtime/typescript` suite
  (225 ok) are green.
- (2026-09-17, fourth fix pass) RF22 is fixed: generated JavaScript raises a run-time error on a
  zero divisor for `/` and `%` at every numeric type. `cargo test --workspace` and the
  `runtime/typescript` suite (225 ok) are green, and `openspec validate` passes.
- (2026-09-17 13:57 verification) RF22 verified: every `/` and `%` in generated JavaScript now fails
  on a zero divisor as the interpreter and the IR runtime do. Spot-checked the three settled
  questions: `nxlang run` prints `v=0.1` for a `float32`, the canonical-text requirement limits the
  backends-agree guarantee to `int`'s range, and the entity decision is recorded (in the root
  `specs/future.md`). Two new findings from that check: RF23 (Medium), a typed Markdown body is collapsed
  to one line, and RF24 (Low), the `specs/future.md` entry on typed bodies describes behavior RF12
  and RF21 already fixed. `cargo test --workspace` (exit 0), the `runtime/typescript` suite
  (225 ok) and `openspec validate` pass.
- (2026-09-17, fifth fix pass) RF23 and RF24 are fixed: a typed body keeps its line breaks and
  loses the indentation its lines share, and the two future-work files are merged into
  `specs/future.md`. `cargo test --workspace`, the `runtime/typescript` suite and `openspec
  validate` pass.
- (2026-09-17 14:20 verification) RF23 and RF24 verified: a typed body keeps its line breaks and
  loses only its shared indentation, on every backend, and the future-work entries are rewritten and
  consolidated into `specs/future.md`. One new finding, RF25 (Low): six `design.md` paragraphs and
  one spec paragraph were left over-wrapped by the in-place edits. `cargo test --workspace`
  (exit 0), the `runtime/typescript` suite (227 ok) and `openspec validate` pass. Every finding
  except RF25 is now verified.
- (2026-09-17, sixth fix pass) RF25 is fixed: the paragraphs the fix passes edited in place are
  re-wrapped at the column their files use.
- (2026-09-17 14:36 verification) RF25 verified. Every finding in this report, RF1 through RF25, is
  now verified, and the working tree is staged. `cargo test --workspace` (exit 0), the
  `runtime/typescript` suite (227 ok) and `openspec validate` pass. One thing to note before
  committing: `crates/nx-types/tests/for_loop_names.rs` is now staged, which RF18 had deliberately
  left untracked as another change's file. Its `infer.rs` and `env.rs` code is staged too, so the
  tree is at least self-consistent, but the loop-name diagnostic work should be split out or
  acknowledged as riding along.
