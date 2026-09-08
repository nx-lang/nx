# resolve-editor-positions — verification

## The measurement table, re-run

Measured against a rebuilt `target/debug/nx-lsp --stdio` over the protocol, driving `initialize`,
`textDocument/didOpen`, and then `textDocument/completion` or `textDocument/hover` — the same way
an editor reaches the server. Fixtures are the ones `proposal.md` measured, with
`type Mode = light | dark` and `let <Panel mode:Mode="light" title:string caption:string />`.

| Request | Position | Result before | Result now |
| --- | --- | --- | --- |
| Property-name completion | `<Panel ⟨cursor⟩ />` on one line | `mode`, `title`, `caption` | `mode`, `title`, `caption` |
| Property-name completion | same tag written across lines | `import`, `from`, `as`, `export`, … | `mode`, `title`, `caption` |
| Property-value completion | `<Panel mode=⟨cursor⟩ />` on one line | `light`, `dark` | `light`, `dark` |
| Property-value completion | same tag written across lines | `import`, `from`, `as`, `export`, … | `light`, `dark` |
| Hover | declaration name `Panel` | ``component `Panel` `` — kind only | ``component `Panel` `` + `component <Panel mode:Mode title:string caption:string />` |
| Hover | tag reference `<Panel` | `null` | ``component `Panel` `` + `component <Panel mode:Mode title:string caption:string />` |
| Hover | parameter `count` in a function body | `null` | `` `int` `` |

Every row that read `null` or the declaration-keyword list now returns the specified result, and
both rows that already worked are unchanged.

## Deviations from the artifacts

Three things the plan assumed did not hold. Each is recorded against its task in `tasks.md`.

- **`SyntaxTree::node_at` could not be used (task 4.2).** It asks tree-sitter for the smallest
  descendant spanning the empty range at the offset, which refuses to enter a node the offset merely
  touches and refuses to enter a zero-width node at all. Both are where an editor asks: in
  `<Panel\n  mode=|\n/>` the cursor is at the end of `property_list` and inside a zero-width
  `rhs_expression`, and `node_at` answers `element` for it. `positions::ancestor_chain` descends with
  boundaries included instead, over the same public `SyntaxNode` API. `nx-syntax` is unchanged.

- **Task 2.3's premise was wrong.** "Type completions are offered in a multi-line signature" already
  passed: `is_type_position` sees the annotation's `:` on the cursor's own line even when the
  signature spans lines. The test was kept as a regression guard. The layout dependence the
  requirement actually names was pinned by a test that *did* fail —
  `a_colon_inside_a_property_value_does_not_make_a_type_position`, where a colon inside a property
  value made a property-name position look like a type position.

- **Reusing `detail` alone could not satisfy the component-signature scenario (task 5.3).** `detail`
  was `component <Panel />` and carries no properties, but the scenario requires the properties and
  their declared types. The one existing formatter was enriched (`markup_signature`) rather than a
  second one added, so completion detail and hover still agree by construction.

## Findings not fixed here

- **A parameter interpolated into a markup body has no inferred type.**
  `let <Panel width:int /> = <div>{width}</div>` lowers the `width` reference with a real span, but
  the type environment has no entry for it; the same reference in a function body does. Hover is
  conservative there, which the specified contract permits. Closing it means changing an analysis
  crate, which this change's Non-Goals forbid. Worth its own proposal.

- **Hover on a literal is still out of reach**, as design D6 predicted. Literals now reach the type
  environment with a type, but `expr_span` still reports `0..0` for them, so they cannot be found by
  offset.
  — **Fixed after the verification pass.** D7 had already moved the position-to-expression query
  into `nx-hir`, which left recording the spans a change to one crate and seven call sites. Lowering
  now records the span of every literal, folding `-` rewrites the operand in place instead of
  leaving a narrower unnegated literal to shadow it, and the resolver bounds the lookup by the whole
  literal rather than the token under the cursor. Hover reports `string`, `int`, `float64`,
  `boolean`, and `int` for `-42` at both the sign and the digits. Recorded as design D8 and tasks
  10.1–10.4, with a scenario on the MODIFIED hover requirement.

- The **first** limitation above stays open and is the one still worth its own proposal. Its cause is
  narrower than "an analysis crate cannot change": body content is passed to `infer_expr` only where
  the element's tag declares a content property (`check_element_bindings`,
  `crates/nx-types/src/infer.rs:1883`), and a built-in tag such as `div` matches no function,
  component, record, or union branch of `infer_element_expression`, so neither its content nor its
  property values are ever inferred. Closing it means inferring expressions the checker does not
  visit today, which can surface diagnostics on code that reports none now — a change to what NX
  checks, with its own spec deltas, not a side effect of an editor change.

---

# Review: resolve-editor-positions

## New Findings Discovered During 2026-09-05 13:36 Review

## Scope
**Reviewed artifacts:** `proposal.md`, `design.md`, `specs/editor-language-service/spec.md`,
`tasks.md`, the existing `review.md` verification note.
**Reviewed code:** `crates/nx-language-service/src/positions.rs` (new),
`crates/nx-language-service/src/lib.rs` (working-tree diff),
`crates/nx-language-service/Cargo.toml`, `crates/nx-lsp/src/lib.rs`.
**Checks run:** `cargo test --workspace` (all green), `cargo clippy -p nx-language-service
--all-targets` (three warnings, all on lines this change did not touch),
`openspec validate resolve-editor-positions --strict` (valid), plus throwaway integration probes
against the public `WorkspaceSnapshot` API to exercise positions the test suite does not cover
(removed afterwards; the working tree is unchanged by this review).

Task claims verified: no analysis crate is modified (`git status` shows only `nx-language-service`
and `nx-lsp`), `line_prefix` / `is_identifier_start` / `EditorRange::contains_byte` are gone with no
callers left, and `server_capabilities()` is untouched.

## Findings

### ✅ Verified - RF1 The property name the cursor is inside counts as already supplied, so re-editing a property name drops it from the completion list
- **Severity:** Medium
- **Evidence:** `supplied_properties` / `collect_supplied`
  (`crates/nx-language-service/src/positions.rs:356-380`) collect every property name in the opening
  tag, including the one the cursor sits in, and `element_context` passes that set through for the
  cursor-on-a-property-name branch (`positions.rs:188`). With
  `type Mode = light | dark` / `let <Panel mode:Mode title:string caption:string /> = <div />`, a
  completion request at `<Panel mo⟨cursor⟩de="light" />` returns `["title", "caption"]` — the name
  being edited is the one name not offered. The old line-scanning `supplied_properties` read only
  the text *before* the cursor, so ` mo` was not followed by `=` and `mode` was still offered; this
  is therefore a single-line regression, and the design's second Risk names single-line parity as
  the safety property. `assert_same_completions` cannot catch it because both spellings regress
  identically.
- **Recommendation:** In `element_context`, drop the property node the cursor is inside from
  `supplied` before building `PositionContext::PropertyName` (the `property_value` node is already
  in hand at `positions.rs:180`), and add a test for `<Panel mo|de="light" />` in both spellings.
- **Fix:** `element_context` now removes the cursor's own `property_value` name from `supplied`
  before building the `PropertyName` context (`positions.rs`). New test
  `the_property_name_under_the_cursor_is_still_offered` asserts `mode` is offered at
  `<Panel mo⟨cursor⟩de="light" />` in both the single-line and the multi-line spelling; without the
  fix it returns `["title", "caption"]`.
- **Verification:** Confirmed fixed. `<Panel mo⟨cursor⟩de="light" />` now offers
  `["mode", "title", "caption"]`, and the multi-line spelling with `title` supplied on another line
  offers `["mode", "caption"]` — the edited name is back, and a genuinely supplied one is still
  withheld. `element_context` removes only the cursor's own `property_value` name from `supplied`,
  so the empty-slot branch is untouched. The new test covers both spellings.


### ✅ Verified - RF2 Hover inside a string property value reports the enclosing element's type
- **Severity:** Medium
- **Evidence:** `inferred_type_hover` (`crates/nx-language-service/src/lib.rs:393-416`) ignores the
  resolved context's span and scans the whole expression arena for the smallest recorded span
  containing the raw offset. A cursor inside `"hello"` in `<Panel title="he⟨cursor⟩llo" />`
  resolves to `Expression` (the string token), the string's own literal expression has the `0..0`
  span design D6 records, and the nearest surviving span is the `<Panel …/>` element expression —
  so hover answers `` `div` ``, the component's return type, for a cursor in an unrelated string.
  That is the "fabricated result" the conservative contract and design D5 rule out, and no test
  covers a hover inside a quoted value.
- **Recommendation:** Require the chosen expression's span to be contained in the resolved
  context's span rather than merely to contain the offset. That keeps the parameter-reference case
  (the `Ident` expression's span is the identifier node's own) and makes the string case return no
  hover, which is the specified answer.
- **Fix:** `inferred_type_hover` now takes the resolved context's span as a `within` bound and skips
  any expression whose span is not contained in it, in addition to the existing offset test. New
  test `hover_inside_a_string_property_value_returns_no_result` covers the quoted-value case;
  without the fix it reports the enclosing element's type.
- **Verification:** Confirmed fixed. Hover inside `title="he⟨cursor⟩llo"` returns no result,
  while the parameter case still reports `` `int` `` and a braced value `mode={m⟨cursor⟩}` still
  reports `` `string` ``. The bound is enforced inside `LoweredModule::innermost_expr_at` as
  `within.contains_range(span)`, so it cannot be bypassed by a caller. One deliberate consequence:
  hover on an operator token (`count ⟨cursor⟩+ 1`) now reports nothing where it previously reported
  the binary expression's type — the conservative answer, consistent with the contract.
- **Amended by the literal-span work (D8).** The defect this finding named — the *enclosing element's*
  type answering for a cursor in a string — stays fixed, and the bound in
  `LoweredModule::innermost_expr_at` is what keeps it fixed. What changed is the right answer at that
  position: the string literal now carries a span, so hover reports `` `string` `` where it reported
  nothing. The test was rewritten to assert both halves and renamed
  `hover_inside_a_string_property_value_reports_the_string_not_its_element`; asserting no result
  would now pin the D6 limitation rather than the requirement.


### ✅ Verified - RF3 Name lookup preempts the inferred type, so a shadowed local hovers as the top-level declaration
- **Severity:** Medium
- **Evidence:** `hover_contents` (`lib.rs:372-377`) resolves a `Reference` by name through
  `scope.visible` first and only falls back to the type environment. With
  `let count = "top"` followed by `let add(count:int) = { c⟨cursor⟩ount + 1 }`, hover on the
  parameter reference returns ``value `count`\n\nvalue`` — the top-level value, not the parameter's
  `int`, which is exactly the scenario "Hover over an expression reports its inferred type" names.
  The same ordering means a reference to any declaration whose `detail` carries no type (a value
  with no annotation, a union, a record) reports no type at all even though the type environment
  has one.
- **Recommendation:** For a `Reference`, look the resolved expression up in the type environment
  first and use the declaration only when the reference has no expression entry — or report both,
  kind and signature plus inferred type. Add a shadowing test.
- **Fix:** `hover_contents` resolves a `Reference` through the type environment first and falls back
  to `scope.visible` only where the reference reaches no typed expression. The spec's "Hover over a
  reference reports the referenced declaration" scenario is about a component tag, which resolves to
  `ComponentTag` and is unaffected. New test
  `hover_on_a_shadowing_parameter_reports_the_parameter_not_the_top_level_name` uses the review's own
  fixture; without the fix it reports the top-level value.
- **Verification:** Confirmed fixed. The review's own fixture reports `` `int` ``, and a
  reference to a top-level `let count = 5` now reports `` `int` `` instead of a bare `value` line.
  The declaration fallback is still reachable where a name has no typed expression: hover on `Img`
  in `import { Img } from "./widgets.nx"` still returns the component with its signature. Tag
  references go through `ComponentTag` and are unaffected, so the spec scenario still holds.


### ✅ Verified - RF4 Hover is silent at type annotations and property names, which the modified requirement lists as hoverable
- **Severity:** Low
- **Evidence:** The MODIFIED hover requirement covers "a declaration, reference, type annotation,
  component tag, property, or expression with known metadata"
  (`specs/editor-language-service/spec.md`). `hover_contents` (`lib.rs:379-386`) returns `None` for
  `TypeAnnotation`, `PropertyName`, and `PropertyValue` unconditionally. Hovering `Mode` in
  `let <Card m:M⟨cursor⟩ode /> = <div />` returns nothing, though `Mode` is a visible declaration
  with the metadata the requirement asks for; the same holds for `<Panel ti⟨cursor⟩tle="x" />`.
  The comment there says these contexts "carry no metadata of their own", which is true of an empty
  slot but not of a written type name or a declared property.
- **Recommendation:** Either report the declaration for a type-annotation position that names a
  visible type, and the property's declared type for a property-name position, or state the
  narrowing in the spec so requirement and implementation agree. No scenario currently pins either
  position, so this is invisible to the suite.
- **Fix:** widened, on the author's call — the requirement already licensed these positions, so
  narrowing it would have been writing the gap down rather than closing it. Recorded as tasks 8.1–8.5.
  The resolved context now carries the name at the position (`property` on `PropertyName`, `name` on
  `TypeAnnotation`), absent where the slot is empty. A property name reports the type its component
  declares; a type annotation reports the declaration the name resolves to, or the primitive or
  built-in classification where NX rather than the workspace defines it. An empty property slot and
  an empty annotation still report nothing, which is now a pinned scenario rather than a silence.
  Five tests, and three scenarios added to the MODIFIED hover requirement.
- **Note:** the review was right that the `Declaration` record could not answer this as it stood, but
  the missing piece was the type's spelling, not a lookup. `properties: Vec<String>` and
  `property_types: Vec<(String, String)>` were parallel vectors, and the latter held `base_type_name`,
  which spells `string[]` as `string` — correct for resolving a union into its declaring namespace,
  wrong to show a reader. Both are now one `Vec<PropertyDeclaration>` carrying the name, the base
  type, and the type as written. Property completions show the declared type as their detail as a
  result, so hover and completion read one recorded fact instead of two spellings that could drift.
- **Verification:** Confirmed fixed, and the widening is the better of the two options the
  finding offered. Measured: a property name reports ``property `title` `` / `string`; an unknown
  property, an empty slot, and an empty annotation each report nothing; a type annotation naming a
  declaration reports ``union `Mode` ``; a primitive reports ``primitive type `string` ``. Five
  tests and three scenarios back this, and `openspec validate --strict` passes. The
  `PropertyDeclaration` collapse the note describes is real and visible: property completions now
  carry the declared type as detail (`mode → Mode`, `title → string`) with labels unchanged.


### ✅ Verified - RF5 `declaration_hover` repeats the symbol kind as the signature for values, unions, records, and actions
- **Severity:** Low
- **Evidence:** `declaration_hover` (`lib.rs:1015`) always appends `declaration.detail`, but
  `detail` is the bare kind word for a union (`lib.rs:1583`), a record, and an action, and for an
  unannotated value (`lib.rs:1533`). Hover on `type M⟨cursor⟩ode = light | dark` renders
  ``union `Mode`\n\nunion``; on `let co⟨cursor⟩unt = 5`, ``value `count`\n\nvalue``. The scenario
  asks for "the symbol kind and available signature or type information" — the second line here is
  neither.
- **Recommendation:** Omit the detail line when it adds nothing beyond the kind, or enrich those
  details the way `markup_signature` enriched the component case (union cases, record fields, the
  value's inferred type).
- **Fix:** the first of the two, as the narrower change — `declaration_hover` writes the second line
  only where `detail` says something the kind does not. Hover on `type Mode = light | dark` is now
  ``union `Mode` `` alone. New test `hover_on_a_union_declaration_does_not_repeat_its_kind`.
  Enriching the union, record, and value details the way `markup_signature` enriched the component
  case is a legible follow-up, not needed for the second line to stop being noise.
- **Verification:** Confirmed fixed. `type Mode = light | dark` hovers as ``union `Mode` ``
  and `let count = 5` as ``value `count` ``, both with no second line; action and record
  declarations behave the same. Function and component hovers keep their signature line, so the
  suppression is keyed on the detail actually repeating the kind rather than on the kind itself.


### ✅ Verified - RF6 Every `ModuleArtifact` is deep-cloned into the snapshot, and the insert is written twice
- **Severity:** Low
- **Evidence:** `build_workspace_declarations` (`lib.rs:678` and `lib.rs:699`) inserts
  `module.clone()` while iterating `&modules`, a local `Vec` dropped at the end of the function.
  `ModuleArtifact` is `#[derive(Clone)]` over `TypeEnvironment`, `Vec<Diagnostic>`, and
  `Vec<PreparedBinding>` (`crates/nx-types/src/check.rs:20-38`), so this deep-copies the whole type
  environment of every module on every snapshot build purely to retain what was about to be
  discarded. The identical insert also appears in both the early-`continue` branch and the tail of
  the loop.
- **Recommendation:** Iterate `modules.into_iter()` and move each artifact in, and hoist the single
  insert to the top of the loop body so the `lowered_module` early-continue no longer needs its own
  copy of it.
- **Fix:** `build_workspace_declarations` iterates `modules` by value and moves each artifact into
  `declarations.artifacts` once, above the `lowered_module` check, so neither branch clones. The loop
  keeps its handle on the lowered module by cloning the `Arc`, which is a refcount rather than a copy
  of the type environment.
- **Verification:** Confirmed fixed. The loop now iterates `modules` by value and moves each
  artifact into `declarations.artifacts` once, above the `lowered_module` check, holding only an
  `Arc` clone of the lowered module across the move. No `module.clone()` remains, and the
  non-lowering branch still records its artifact.


### ✅ Verified - RF7 Hover scans the entire expression arena on every request
- **Severity:** Low
- **Evidence:** `inferred_type_hover` (`lib.rs:401-412`) walks `0..module.expr_count()` for each
  hover. It is correct and small today, but it is linear in module size on a request an editor
  fires on every cursor rest, and the design's cost argument (D3) accounted only for the extra
  parse.
- **Recommendation:** Bound the scan with the containment filter RF2 proposes, or descend the
  arena by span rather than enumerating it.
- **Fix:** the Non-Goal was amended rather than worked around, on the author's call. The query moved
  to `nx-hir` as `LoweredModule::innermost_expr_at(within, offset)`, with an `exprs()` iterator
  replacing the raw-index reconstruction; `nx-language-service` no longer depends on `la-arena`.
  Recorded as design D7 and tasks 9.1–9.3, with four cases tested in `nx-hir` itself.
- **Note on the remedy.** The scan is still linear, deliberately. Re-reading the finding, the cost was
  the smaller half of it: the walk was `0..expr_count()` rebuilding each `ExprId` with
  `ExprId::from_raw(RawIdx::from(index))`, which is a reach around `LoweredModule` — the arena and the
  span map are private, so that was the only way to ask the question from outside. Indexing it in
  place would have optimized the wrong thing and left the encapsulation break. An interval query that
  is genuinely sublinear in the worst case needs an interval tree, since lowered spans neither nest
  reliably nor are all present, and that is not worth its invalidation surface for hundreds of
  expressions once per cursor rest. What the move buys is that the decision is now `nx-hir`'s alone:
  an index goes behind that signature and no caller changes.
- **Note on what else this unblocks.** Both limitations under "Findings not fixed" above — a literal
  carrying no usable span, and a markup-body interpolation reaching the type environment with no entry
  — are now fixable entirely inside the analysis crates, because the language service no longer knows
  how an expression is found.
- **Verification:** Confirmed fixed, by a wider remedy than the finding proposed and a sound
  one. `LoweredModule::innermost_expr_at` and `exprs()` are in `nx-hir` with four cases tested
  there, and `la-arena`, `RawIdx`, and `expr_count` are all gone from `nx-language-service` —
  `Cargo.toml` and `lib.rs` both. The scan stays linear on purpose, which the recommendation
  allowed. The Non-Goal amendment is recorded where it belongs (design D7, tasks 9.1–9.3), and
  `nx-hir`'s public surface gained only two read-only queries with no existing signature changed,
  so the constraint the Non-Goal was protecting still holds.


### ✅ Verified - RF8 No test covers hover on a reference to a declaration in another document
- **Severity:** Low
- **Evidence:** The scenario "Hover over a reference reports the referenced declaration" specifies a
  component "declared elsewhere in the workspace snapshot and visible to the document", but every
  hover test in `crates/nx-language-service/src/lib.rs` builds a single-document snapshot via
  `hover_at`; the multi-document `snapshot_of` helper is used only by completion tests. The
  behavior does work — a probe against an imported and a wildcard-aliased tag both returned
  ``component `Img` `` with its signature — so this is a coverage gap, not a defect.
- **Recommendation:** Add a hover test over `snapshot_of` for a selectively imported tag and one for
  a `ui.Img` wildcard alias, mirroring the existing completion tests.
- **Fix:** added a `hover_in(documents, uri, marked)` helper over `snapshot_of` and the two tests the
  recommendation names — `hover_over_a_selectively_imported_tag_reports_the_declaration` and
  `hover_over_a_wildcard_aliased_tag_reports_the_declaration` — against the same two-document fixture
  the completion tests use. Both pass, confirming this was a coverage gap rather than a defect.
- **Verification:** Confirmed fixed. `hover_in` builds a multi-document snapshot, and both
  named tests assert the declaration and its `fit` property through a selective import and through
  a `ui.Img` wildcard alias. Both pass, matching the probe result recorded in the finding.


### ✅ Verified - RF9 `hover` and `completions` repeat the same resolve preamble
- **Severity:** Low
- **Evidence:** `lib.rs:320-334` and the head of `completions` both look the document up, build a
  `LineIndex`, convert the position to an offset, parse the source, call `positions::resolve`, and
  build the document scope — with the two differing only in how a failed parse is handled (`hover`
  returns `Ok(None)`, `completions` treats it as `Unresolved`). The design's first Goal is that the
  two "cannot disagree about what the cursor is on"; two hand-written copies of the preamble is
  where that would drift.
- **Recommendation:** Extract one `fn resolved_position(&self, uri) -> Option<(…index, offset,
  context, scope)>` and have both entry points consume it.
- **Fix:** extracted `WorkspaceSnapshot::resolve_position`, returning a `ResolvedPosition` — the
  document, its `LineIndex`, the byte offset, the resolved context, and the document scope. Both
  entry points consume it and neither parses or resolves on its own. The one behavioral difference
  goes with it: a document that fails to parse now resolves to `Unresolved` for both, which hover
  already answers with no result.
- **Verification:** Confirmed fixed. `resolve_position` returns a `ResolvedPosition` carrying
  the document, index, offset, context, and scope; neither `hover` nor `completions` parses or
  resolves on its own any more. The behavioral unification is benign — a document that fails to
  parse resolves to `Unresolved`, which hover answers with no result exactly as the old early
  return did. Report note: this fix bullet had been appended after `## Summary`; it is moved under
  the finding here.


## Questions
- RF4 is the only finding that needs a decision rather than a fix: should hover answer at type
  annotations and property names, or should the spec's hover requirement be narrowed to the
  positions the implementation deliberately treats as completion-only?

## Summary
- The change does what it set out to do. Context resolution moved off line text onto the syntax
  tree, the multi-line failures in the `proposal.md` table are fixed, the line-scanning helpers are
  gone with no fallback path left behind, no analysis crate was touched, and the whole workspace
  suite passes.
- Three findings are worth fixing before archiving: RF1 is a single-line regression the parity
  helper structurally cannot see, and RF2 and RF3 are cases where hover answers with the wrong
  thing rather than with nothing — the one failure mode design D5 was written to prevent.
- The rest are polish: a spec/implementation gap at type-annotation and property positions (RF4),
  redundant hover text (RF5), an avoidable deep clone (RF6), a linear scan (RF7), a coverage gap on
  a specified cross-document scenario (RF8), and duplicated preamble between the two entry points
  (RF9).

## Fix pass — 2026-09-05

All nine findings are fixed. RF1, RF2, RF3, and RF5 each gained a test that fails against the pre-fix
code, so the two
wrong-answer cases the review called out — a cursor in a string answered with the element's type,
and a shadowed local answered as the top-level declaration — are now pinned rather than trusted.

Two findings were raised as open questions and answered by taking the wider option in both cases.
RF4 was answered by widening hover rather than by narrowing the requirement. It is the one finding
that changed the specification: three scenarios were added to the MODIFIED hover
requirement, and `tasks.md` gained a section 8 for the work. It is also the one finding whose fix
reached past the review's own diagnosis — the obstacle was that the `Declaration` record spelled a
property's type only in the stripped form a namespace lookup wants, which collapsing its two
parallel property vectors into one `PropertyDeclaration` fixed for hover and completion together.

RF7 was answered by amending the Non-Goal on analysis crates rather than by deferring the finding.
`nx-hir` gained one read-only query and the iterator it is built on; no lowering, no analysis result,
and no existing signature changed. Recorded as design D7 and tasks 9.1–9.3. The scan it replaced is
still linear on purpose — the finding's real cost was that the question was being asked from outside
the type that owns the answer.

`cargo test --workspace` passes, with 61 tests in `nx-language-service` against the 50 the review
read, and two more in `nx-hir` for the query D7 moved there. `git diff --stat` over the analysis
crates is no longer empty, which is the amended Non-Goal and not a slip: it is `nx-hir` only, and it
is additive. `cargo clippy -p nx-language-service --all-targets` still reports the same three warnings on
lines this change did not touch, and none in the code the fixes added.
Fixed findings are marked 🟡 Fixed rather than ✅ Verified: verification belongs to the
reviewer that raised them.

## Verification pass — 2026-09-05 14:12

All nine findings verified as fixed; none reopened, and no new findings. Each fix was checked
against the code and then measured through the public `WorkspaceSnapshot` API with throwaway
integration probes (removed afterwards; the working tree is unchanged by this pass).

Suite and tooling: `cargo test --workspace` green, with `nx-language-service` at 61 tests against
the 50 the review read and `nx-hir` at 115 against 113. `cargo clippy -p nx-language-service -p
nx-hir --all-targets` adds no warning in the code these fixes wrote. `openspec validate
resolve-editor-positions --strict` passes with the three new hover scenarios.

The amended Non-Goal is the one thing a reader should not skim past: `crates/nx-hir` is now in the
diff, which task 3.3 and the original design forbade. It is additive and read-only —
`LoweredModule::innermost_expr_at` and `exprs()`, no lowering and no existing signature touched —
and design D7 records the reasoning, so the change is deliberate and traceable rather than a slip.

## Literal spans — 2026-09-05

Closes the second of the two limitations under "Findings not fixed", after the user asked whether
either belonged in this change. Design D6 is reversed by D8 and tasks 10.1–10.4 record the work;
the MODIFIED hover requirement gained "Hover over a literal reports its type".

The first limitation stays out, and its entry above now records why in terms of the code rather than
of the Non-Goal: it needs the checker to visit expressions it does not visit today, which can
surface diagnostics where there are none now. That is its own change.

`cargo test --workspace` passes at 1490 tests. `nx-hir` gained two tests for the spans and for
in-place folding; `nx-language-service` gained one covering all five literal forms and both ends of
`-42`. The only existing test that had to change is the one this pass renamed, recorded under RF2.

## New Findings Discovered During 2026-09-05 16:39 Review

### Scope

**Reviewed artifacts:** `proposal.md`, `design.md` (D6/D7/D8 and the amended Non-Goal),
`specs/editor-language-service/spec.md`, `tasks.md` (section 10), the existing `review.md`.

**Reviewed code:** the currently unstaged edits only — `crates/nx-hir/src/lower.rs`
(`literal_expr`, in-place `fold_negated_literal`, the seven literal sites, the text-run site, two new
tests), `crates/nx-language-service/src/positions.rs` (`is_literal` and the literal widening in
`resolve`), `crates/nx-language-service/src/lib.rs` (three test changes), and `specs/future.md` (the
three new sections). The staged work behind them was read for context but re-reviewed only where an
unstaged edit changed its behavior.

**How it was checked:** `cargo test --workspace` is green (51 suites, no failures);
`nx-language-service` is at 63 tests and `nx-hir` at 117. Behavior was measured through the public
`WorkspaceSnapshot::hover` API with a throwaway integration test, run once against the working tree
and once against the tree with the unstaged patch reverted, so each finding below is stated as
"introduced here" or "pre-existing" on evidence rather than on reading. The probe was removed; the
working tree is unchanged by this pass.

### Findings

### ✅ Verified - RF10 A negated literal is unanswerable wherever `-` is spelled as a prefix-unary expression, so `-42` hovers and `{-42}` does not

- **Severity:** Medium
- **Evidence:** `fold_negated_literal` folds `-` into the literal in all three spellings — its own
  doc comment says "unbraced, braced, and inside a larger expression" — and records the span of the
  whole written form (`crates/nx-hir/src/lower.rs:234-248`). The position resolver only widens to a
  node whose *kind* is a literal (`crates/nx-language-service/src/positions.rs:100-104`, `is_literal`
  at `:465`). `SIGNED_NUMERIC_LITERAL` is such a kind, so the unbraced `-42` works; a braced or
  nested `-42` parses as a prefix-unary expression over `INT_LITERAL`, so the bound is `42` while the
  folded literal spans `-42`, and `within.contains_range(span)`
  (`crates/nx-hir/src/lib.rs:948`) rejects it. Measured:

  ```text
  "let value = -4@2"        -> `int`      "let value = {-4@2}"     -> None
  "let f(x:int) = {x + 1@0}" -> `int`     "let f(x:int) = {x + -1@0}" -> None
  "let value = {1.@5 * 2}"  -> `float64`  "let value = {-1.@5}"    -> None
  ```

  The spec scenario is "Hover over a literal reports its type … the result SHALL be the same wherever
  in the literal the position falls" (`specs/editor-language-service/spec.md:87-90`), and design D8
  claims the three spellings share one lowered representation. Not a regression — these positions
  answered nothing before too — but half of what task 10.3 set out to deliver. The test loop hides it
  by exercising only the unbraced form.
- **Recommendation:** In `resolve`, treat a unary-minus expression whose operand is a literal as the
  literal: after finding the literal node, keep walking outward while the parent is a
  `PREFIX_UNARY_EXPRESSION`/`UNARY_EXPRESSION` whose only non-`MINUS` child is that node, and use its
  span. Then add `{-42}` and `{x + -10}` to `hover_over_a_literal_reports_its_type` — the fixtures
  that would have caught this.
- **Fix:** `positions::resolve` no longer keys the widening on the node's kind. `literal_span` takes
  the outermost chain node that spells the literal and nothing besides — `spans_a_literal`, which
  accepts a literal kind, a prefix `-` over one (`negates_a_literal`, recursive because the fold
  is), or a wrapper node spanning exactly what it wraps, which is how `value_expression` and
  `value_list_item_expression` are told from a node that adds something of its own. Four fixtures
  added to `hover_over_a_literal_reports_its_type`: `{-4⟨cursor⟩2}`, `{-⟨cursor⟩42}`,
  `{-1.⟨cursor⟩5}`, and `{x + -1⟨cursor⟩0}` with `{x + ⟨cursor⟩-10}` beside it. All report the
  literal's type; without the fix each reports nothing.
- **Not covered:** `{⟨cursor⟩-42}`, the offset between `{` and the sign. It resolves to the brace,
  not to the literal, because `child_at` gives a boundary to the node that ends there before the one
  that starts there — the tie-break other positions depend on. The unbraced `⟨cursor⟩-42` answers
  because whitespace is no node and nothing competes for the offset.
- **Verification:** Correct, and wider than the fix note claims. Measured through the public
  `WorkspaceSnapshot::hover` API: `{-4⟨cursor⟩2}`, `{-⟨cursor⟩42}`, `{-1.⟨cursor⟩5}`,
  `{x + -1⟨cursor⟩0}`, and `{x + ⟨cursor⟩-10}` all report the literal's type where each reported
  `None` before, and the unbraced spellings still do. `{- -4⟨cursor⟩2}` reports `` `int` `` too,
  so the recursion in `negates_a_literal` is exercised by behavior and not only by reading.
  `spans_a_literal` identifying a wrapper by "spans exactly what it wraps" rather than by naming
  `value_expression`/`value_list_item_expression` is the more durable of the two shapes, and the
  grammar bears out the reason the recommended walk-outward would not have worked
  (`grammar.js:443-460` puts `value_list_item_expression` between the operator and its operand).
  The uncovered `{⟨cursor⟩-42}` reproduces and is honestly recorded with its cause.

### ✅ Verified - RF11 Hover on `null` reports `` `T0?` ``, leaking an inference variable into editor text

- **Severity:** Medium
- **Evidence:** Introduced by these edits: with the unstaged patch reverted,
  `let value = nu@ll` hovers as `None`; with it applied it hovers as `` `T0?` ``, and
  `let v:string? = nu@ll` does too. `inferred_type_hover` declines only two of the four types that
  mean "inference has nothing here" — `if matches!(ty, nx_types::Type::Unknown | nx_types::Type::Error)`
  (`crates/nx-language-service/src/lib.rs:448`) — while `Type::Variable(TypeId)` renders as `T0`
  (`crates/nx-types/src/ty.rs:186`). `T0?` names nothing a reader can act on, which is the fabricated
  hover design D5 and the conservative contract rule out. The same gate lets the pre-existing
  `Type::ContextualName` through: `let add(count:int) = cou@nt` hovers as `` `count` ``, which reads
  as a type named `count` but is a bare name still awaiting resolution (`ty.rs:188-192`); that one
  predates this change, but it is the same one-line fix.
  `null` is also the one `Literal` variant the new test loop omits — it covers string, int, float,
  boolean, and both ends of `-42` — which is why this reached the working tree.
- **Recommendation:** Widen the guard to decline a type that still contains an unsolved variable or a
  `ContextualName`, not just `Unknown`/`Error`, and add `null` to
  `hover_over_a_literal_reports_its_type`. If `null` should answer rather than decline, the answer a
  reader wants is `null` or `T?`, decided in the renderer — not the raw variable id.
- **Fix:** declining, which is what the conservative contract implies; the renderer question is
  recorded in `specs/future.md` instead. `is_unresolved_type` replaces the two-variant `matches!` and
  recurses through `Array`, `Nullable`, and `Function`, so `T0?` is declined for the same reason
  `T0` is. `hover_over_a_null_literal_reports_no_type` covers `let value = null` and
  `let value:string? = null`; without the fix both report `` `T0?` ``. The pre-existing
  `ContextualName` leak goes with it: `let add(count:int) = cou⟨cursor⟩nt` reported `` `count` ``
  and now reports nothing. No spec change — the literal scenario is conditioned on a literal
  "written in a position NX static analysis typed", which `null` is not.
- **Verification:** Confirmed. `let value = nu⟨cursor⟩ll` and `let value:string? = nu⟨cursor⟩ll`
  both report `None` where each reported `` `T0?` ``, and the pre-existing `ContextualName` leak
  closes with them: `let add(count:int) = cou⟨cursor⟩nt` reported `` `count` `` and now reports
  nothing. `is_unresolved_type` (`crates/nx-language-service/src/lib.rs:1405-1420`) recurses through
  `Array`, `Nullable`, and `Function` and is exhaustive over `Type` with no wildcard arm, so a new
  variant cannot be silently admitted. Declining rather than inventing a spelling is the right call
  against D5, and the alternative is recorded rather than lost. No regression: the five literal
  forms that did answer still answer.

### ✅ Verified - RF12 The new type-annotation test asserts only that the hover contains the word "type", so it cannot fail on a wrong type

- **Severity:** Low
- **Evidence:** `hover_over_a_type_annotation_reports_it_in_every_declaration_that_can_carry_one`
  (`crates/nx-language-service/src/lib.rs:2894-2911`) checks `hover.contents.contains("type")` across
  five fixtures. Every hover this code path can produce satisfies that: `primitive type \`int\``,
  `primitive type \`string\``, and `type \`R\`` all contain it. The test would still pass if every
  annotation reported the same primitive, or if `a:int` in a record reported `string`. The
  neighbouring per-kind tests (`hover_over_a_primitive_type_annotation_reports_the_primitive`,
  `:2881`) assert exact contents, so the weaker form is the outlier.
- **Recommendation:** Pair each fixture with its expected content — `("action Go = {\n  query:str⟨cursor⟩ing\n}\n", "primitive type \`string\`")` and so on — and assert equality, as the
  literal loop does.
- **Fix:** each of the five fixtures is now paired with its expected content and asserted with
  `assert_eq!`, as the recommendation names — `primitive type` spellings across an action field, a
  record field, a component property, a parameter, and a value.
- **Verification:** Confirmed. All five fixtures are now `(fixture, expected)` pairs asserted with
  `assert_eq!` against exact contents — `primitive type \`string\`` / `primitive type \`int\`` across
  an action field, a record field, a component property, a parameter, and a value. The test can now
  fail on a wrong type, which was the whole of the finding.

### ✅ Verified - RF13 A doc comment now documents the test below it rather than the one it belongs to

- **Severity:** Low
- **Evidence:** `crates/nx-language-service/src/lib.rs:2891`. "An annotation with nothing written in
  it names no type." was written for `hover_over_an_empty_type_annotation_returns_no_result`; the new
  test was inserted between it and that test, so the comment now heads
  `hover_over_a_type_annotation_reports_it_in_every_declaration_that_can_carry_one` — where it
  contradicts what the test asserts — and `hover_over_an_empty_type_annotation_returns_no_result`
  (`:2914`) is left undocumented.
- **Recommendation:** Move the line back above the empty-annotation test.
- **Fix:** moved. `hover_over_an_empty_type_annotation_returns_no_result` carries it again, and the
  every-declaration test keeps only the line written for it.
- **Verification:** Confirmed. "An annotation with nothing written in it names no type." sits at
  `crates/nx-language-service/src/lib.rs:2968`, immediately above
  `hover_over_an_empty_type_annotation_returns_no_result`, and the every-declaration test carries
  only the line written for it.

### ✅ Verified - RF14 `specs/future.md` claims three hover positions still report nothing; at least two more do

- **Severity:** Low
- **Evidence:** "Editor Hover: The Positions Still Unanswered" enumerates a record/action field name,
  bare-kind declaration details, and operator positions. Measured against the working tree, list
  elements report nothing for a literal *and* for a reference — `let value = [1@, 2]` and
  `let a = 1\nlet value = [a@, 2]` both hover as `None`, so it is not the built-in-content cause the
  section above it describes — and a braced negated literal reports nothing (RF10). A future reader
  taking the list as complete would conclude those are covered.
- **Recommendation:** Add the list-element case (with a note that its cause is unknown, since a
  reference fails there too) and either fix RF10 or record it, and drop the word "Three".
- **Fix:** "Three positions" is gone, and the section now says the list was measured rather than
  reasoned from the code. Three entries added: the list-element case with the reference measurement
  that rules out both causes named above it, `null` with the renderer question RF11 left open, and a
  unit literal, which lowers to no expression at all. The braced negated literal is not among them —
  RF10 fixed it.
- **Verification:** **Reopened.** The undercount is real and "Three positions" is correctly gone,
  but two of the three entries the fix added are wrong, and both are wrong the same way: they were
  measured from fixtures that are not valid NX, so what was measured is the syntax-error path, not
  the position. NX has no `[a, b]` list syntax — `let value = [1, 2]` produces a `syntax-error`
  diagnostic spanning the whole declaration, as does `let value:int[] = [1, 2]` (`[` `]` at
  `grammar.js:233` is a *type* suffix). A list is written `{a b}`, and hover answers inside one:
  `{1⟨cursor⟩ 2}` and `{1 2⟨cursor⟩}` report `` `int` ``, `{"a⟨cursor⟩b" "c"}` reports
  `` `string` ``, `{1 -4⟨cursor⟩2}` reports `` `int` ``, and a reference answers too —
  `let ab = 1` / `let value = {a⟨cursor⟩b 2}` reports ``value `ab` ``. So "Anything inside a list"
  is not an unanswered position at all, and the reference measurement offered as proof that the
  cause is neither of the two above it was measuring a parse failure. The unit-literal entry has
  the same defect: `let value = (⟨cursor⟩)` is a syntax error, and `unit_literal` is valid only as a
  list/value-list item (`grammar.js:451,464`). Its conclusion survives — `{(⟨cursor⟩) 2}` parses
  clean and still reports nothing — but its stated cause was never measured. The three original
  entries all reproduce (record field name `None`, `union \`Mode\`` with no detail line, operator
  `None`), as does the `null` entry. This matters more than a normal doc slip because the section
  asserts its own rigor — "Measured against the tree that change left behind, not reasoned from the
  code" — and `specs/future.md` outlives the change; a future reader would go hunting for a
  type-environment bug in lists that does not exist.
- **Recommendation:** Delete the list entry. Rewrite the unit-literal entry against `{()}` or drop
  it. Then re-measure whatever else the section asserts against fixtures checked to produce zero
  diagnostics first — `hover_inside_a_declaration_with_a_syntax_error_returns_no_result` already
  pins "no hover inside a syntax error" as intended behavior, which is what both bad fixtures were
  actually observing.
- **Fix (second pass):** every claim in the reopening reproduces, and the section is rewritten on
  that basis. `[1, 2]`, `let value:int[] = [1, 2]`, and `let value = ()` each emit one
  `syntax-error` diagnostic; `{1⟨cursor⟩ 2}`, `{1 2⟨cursor⟩}`, `{"a⟨cursor⟩b" "c"}`, and
  `{1 -4⟨cursor⟩2}` parse clean and report their item's type, and `let ab = 1` /
  `{a⟨cursor⟩b 2}` reports ``value `ab` ``. So the list entry is deleted, and
  `hover_inside_a_list_reports_the_item_it_is_on` pins those five so the wrong conclusion cannot be
  re-derived. The unit-literal entry is rewritten against `{(⟨cursor⟩) 2}`, which parses clean and
  still reports nothing while the `2` beside it reports `int` — the conclusion survives on a
  fixture that measures it. The three original entries and the `null` entry were re-measured on
  clean fixtures and all four hold. The section's rigor claim now says what was actually done: each
  position measured on a fixture checked to produce no diagnostic first, and why that matters.
- **Verification (second pass):** Confirmed, and the section is now stronger than it was before
  the entry was ever added. Re-measured every surviving claim on fixtures I checked emit zero
  `syntax-error` diagnostics first: record field name `None`, `union \`Mode\`` with no detail line,
  operator `None`, `null` `None`, and the unit literal `{(⟨cursor⟩) 2}` `None` with the `2` beside
  it reporting `` `int` `` — so the contrast the rewritten entry rests on is real, and the entry now
  measures the gap it describes rather than a parse failure. The list entry is gone, and
  `hover_inside_a_list_reports_the_item_it_is_on` pins all five disproving measurements
  (`{1⟨cursor⟩ 2}`, `{1 2⟨cursor⟩}`, `{"a⟨cursor⟩b" "c"}`, `{1 -4⟨cursor⟩2}`, and the reference
  `{a⟨cursor⟩b 2}` → ``value `ab` ``), which is the part that matters: the claim cannot be
  re-derived from a bad fixture without a test going red. The rigor sentence now states the method
  and why it matters instead of asserting a result.

### ✅ Verified - RF15 The change's own files are not rustfmt-clean

- **Severity:** Low
- **Evidence:** `cargo fmt --check` reports diffs in this change's code at
  `crates/nx-hir/src/lower.rs:2111`, `crates/nx-hir/src/lib.rs:1065`, and
  `crates/nx-language-service/src/lib.rs:6`, `:2849`, `:2900`. The repo has pre-existing drift
  elsewhere (`nx-codegen/src/builder.rs`, `nx-syntax/tests/parser_tests.rs`,
  `nx-types/tests/contextual_literals.rs`), so this is not a gate the change broke — but the five
  spots above are lines it wrote.
- **Recommendation:** `cargo fmt` the three files this change touches, leaving the unrelated drift
  alone.
- **Fix:** `cargo fmt --check` now reports no diff in any file this change wrote. The two `nx-hir`
  spots were applied by hand rather than by running rustfmt over the crate root, which would have
  swept in `nx-hir/src/scope.rs` — pre-existing drift this change did not touch. The drift in
  `nx-codegen`, `nx-syntax`, and `nx-types` is left alone.
- **Verification:** Confirmed. `cargo fmt --check -p nx-hir -p nx-language-service` reports one
  diff, `crates/nx-hir/src/scope.rs:328`, and `git diff HEAD -- crates/nx-hir/src/scope.rs` is
  empty — pre-existing drift this change did not write. No file this change touched is unformatted.

### ✅ Verified - RF16 `is_literal` lists kinds the parser never produces, and the text-run literal is the one site that bypasses `literal_expr`

- **Severity:** Low
- **Evidence:** Two small consistency gaps in otherwise careful work.
  `is_literal` (`crates/nx-language-service/src/positions.rs:465-479`) includes `NUMBER_LITERAL` and
  `BOOLEAN_LITERAL`, for which `grammar.js` has no rule — only `number_literal`'s and
  `boolean_literal`'s absent spellings, against the real `int_literal`, `real_literal`, `hex_literal`,
  `bool_literal` — and `UNIT_LITERAL`, which lowers to no literal expression (`let value = (@)` hovers
  as `None`). Separately, task 10.1's stated invariant is that routing through `literal_expr` means "a
  new literal kind cannot quietly arrive without a span", but the text-run site
  (`crates/nx-hir/src/lower.rs:1715-1721`) allocates and sets the span by hand and sets no type,
  so the helper does not in fact guard every literal.
- **Recommendation:** Trim `is_literal` to the kinds the grammar produces. For the text run, either
  route it through `literal_expr(.., TypeTag::String, ..)` — text runs are never operands of the
  fold, so the lowering-local type is inert — or add a line saying why it stays outside the helper.
- **Fix:** both. `NUMBER_LITERAL` and `BOOLEAN_LITERAL` are gone, and so is `UNIT_LITERAL` — the
  grammar does produce it, but `()` lowers to no literal expression, so widening to it would bound a
  lookup that finds nothing; the doc comment now says so. The text run goes through `literal_expr`,
  which answers the open question raised with it: the comment there records that nothing reads the
  span yet because no offset resolves into element content, and points at the `specs/future.md`
  entry.
- **Verification:** Confirmed on both halves, with one caveat carried to RF17. `grammar.js` has no
  `number_literal` or `boolean_literal` rule (`grep -c` returns 0) and both kinds are gone from
  `is_literal`, which now lists exactly the eight the grammar produces. The text run routes through
  `literal_expr` (`crates/nx-hir/src/lower.rs:1723`) with the comment that answers the open
  question. The caveat is the doc-comment justification for dropping `UNIT_LITERAL`: the outcome is
  right — `UNIT_LITERAL` appears nowhere in `lower.rs`, and `{(⟨cursor⟩) 2}`, a spelling that does
  parse, reports nothing whether or not the resolver widens to it — but the fixture the fix note
  measured it from, `let value = (⟨cursor⟩)`, is a syntax error, so the *stated* evidence does not
  support the *correct* conclusion. Filed as part of RF17 because the same fixture reached
  `specs/future.md`.

### Questions

- RF11: should `null` hover as `null`, or decline until the checker solves the variable? Declining is
  what the conservative contract implies; either is defensible, but `T0?` is not.
- The text-run span (RF16) is recorded but unreachable today — `is_expression_container` does not
  admit markup content, so no offset resolves there. Is it deliberate groundwork for the
  `specs/future.md` item on unchecked element content, or a leftover? A one-line comment would
  settle it.
- `cargo clippy -p nx-hir --all-targets` does not compile: `approx_constant` is denied and
  `crates/nx-hir/src/ast/expr.rs:382-383` uses `3.14`. Pre-existing and untouched here, but it means
  the earlier passes' clippy runs cannot have covered `nx-hir`'s test targets.

### Summary

The literal-span work is sound where it is exercised: folding in place is the right call and the
`nx-hir` test pins it, spans cover exactly what was written, and the position resolver widening to the
whole literal is the correct shape of fix. Two findings are worth acting on before archiving. RF10 is
the substantive one — the fix reaches only the unbraced spelling of a negative literal, so `{-42}` and
`{x + -10}` still answer nothing while the design claims all three spellings are one representation.
RF11 is the one that made behavior worse than before this pass: giving `null` a span routed it to a
hover that prints an unsolved inference variable, `T0?`, at a position that previously declined.
Both are pinned by the same gap in the new test loop, which covers five literal forms and omits the
sixth.

The rest is polish: a test that cannot fail on a wrong answer (RF12), a doc comment attached to the
wrong test (RF13), a `future.md` inventory that undercounts (RF14), rustfmt drift in the new lines
(RF15), and two small consistency gaps (RF16). Nothing here questions the design; `cargo test
--workspace` is green throughout.

## Fix pass — 2026-09-05 (RF10–RF16)

All seven findings fixed. Recorded as tasks 11.1–11.5; no spec change, and both open questions the
review raised are answered.

**RF11's question — should `null` hover as `null`, or decline?** Decline. The renderer would have to
invent a spelling for an unsolved variable, which is the fabricated hover D5 rules out; the
alternative is recorded in `specs/future.md` for whoever wants `null` to answer for a reason rather
than by accident. The fix is wider than `null`: `is_unresolved_type` recurses, so an unsolved
variable is declined wherever in a type it sits, and the pre-existing `ContextualName` leak
(`let add(count:int) = cou⟨cursor⟩nt` reporting `` `count` ``) closes with it.

**RF16's question — is the text-run span groundwork or leftover?** Neither, now that it is written
down. The site routes through `literal_expr` like every other literal, and a comment says the span is
unread today because no offset resolves into element content, pointing at the `specs/future.md` entry
that would change that.

RF10 needed a different fix from the one recommended. Walking outward from the literal through
`PREFIX_UNARY_EXPRESSION` parents does not reach the sign: the grammar puts `value_expression` and
`value_list_item_expression` between an operator and its operand, so the unary node is never the
literal's parent. `spans_a_literal` accepts a wrapper by the property that identifies one — it spans
exactly what it wraps — rather than by naming the kinds, so a new wrapper does not silently break it.
One position the finding implied is still unanswered and is recorded under it: `{⟨cursor⟩-42}`, where
the offset belongs to the brace under the boundary tie-break other positions depend on.

`cargo test --workspace` passes with no failures; `nx-language-service` is at 64 tests against the 63
the review read, and `nx-hir` at 117. `cargo fmt --check` reports no diff in any file this change
wrote — the pre-existing drift in `nx-codegen`, `nx-hir/src/scope.rs`, `nx-syntax`, and `nx-types` is
untouched. `openspec validate resolve-editor-positions --strict` passes.

Findings are marked 🟡 Fixed rather than ✅ Verified: verification belongs to the reviewer that
raised them.

## New Findings Discovered During 2026-09-05 18:24 Verification

### ✅ Verified - RF17 Nothing checks that a hover fixture is valid NX, and invalid fixtures have now produced three wrong conclusions in this change

- **Severity:** Medium
- **Evidence:** `hover_at` and `hover_in` (`crates/nx-language-service/src/lib.rs:1994`, `:2724`)
  build a snapshot and ask for hover without ever looking at the document's diagnostics. Hover on a
  declaration containing a syntax error correctly reports nothing — that is pinned deliberately by
  `hover_inside_a_declaration_with_a_syntax_error_returns_no_result` (`:3007`). The two behaviors are
  indistinguishable from a `None`, so a fixture with a typo in it reads as a discovered gap.
  It has produced a wrong conclusion three times in this change's review cycle:
  `record R { a:int }` (invalid; the spelling is `type R = { … }`) was nearly filed as a
  missing-record-hover finding; `action Go { … }` (invalid; the spelling is `action Go = { … }`)
  produced a speculative widening of `type_annotation_context` that was written, tested, and
  reverted; and `[1, 2]` and `()` (both syntax errors) reached `specs/future.md` as two permanent
  entries describing gaps that do not exist (RF14). Only the third survived to a committed artifact,
  and only because nothing in the loop between fixture and conclusion asks whether the fixture
  parses. The cost is asymmetric: a fixture that should hover and does not is caught by the
  assertion, while a fixture that should hover and cannot parse looks exactly like the finding the
  author went looking for.
- **Recommendation:** Have `hover_at`/`hover_in` assert the fixture produces no `syntax-error`
  diagnostic before asking for hover, and give the one test that wants a syntax error an explicit
  helper (`hover_at_unparsed`, or an argument) so the exception is stated rather than implied. That
  turns every future "this position reports nothing" observation into either a real finding or a
  failed fixture, which is the distinction this change kept losing. The same guard applied to
  `labels_at` costs nothing and closes the same hole on the completion side.
- **Fix:** `assert_fixture_parses` fails on any `syntax-error` diagnostic, and `hover_at`,
  `hover_in`, and `labels_at` all call it. The exception is stated at each site rather than implied:
  `hover_at_incomplete` and `labels_at_incomplete` carry the fixtures that deliberately do not
  parse. There turned out to be five, not one, and naming them is worth as much as the guard —
  `let broken( = { missing }` is the syntax-error case the recommendation named, and the other four
  are the half-typed editor states this change exists for (`mode=` with no value in two spellings,
  `mode:` with no type in a signature, `let value:` with no type). Nothing else in the suite was
  running on an invalid fixture. `malformed_documents_answer_position_queries_without_panicking`
  calls the snapshot API directly and is unaffected.
- **Verification:** Confirmed empirically rather than by reading. I re-injected the exact fixture
  that produced the wrong conclusion — `("let value = [1⟨cursor⟩, 2]\n", "`int`")` into
  `hover_over_a_literal_reports_its_type` — and the suite fails with
  `fixture is not valid NX: "let value = [1, 2]\n" — ["Syntax error"]`, naming the cause at the
  fixture instead of presenting a `None` that reads as a finding. That is the whole of what the
  finding asked for. The injection was reverted; the working tree is unchanged.
  `assert_fixture_parses` filters on `code == Some("syntax-error")`, so it does not fire on type
  errors — correct, since a fixture may legitimately carry one. `hover_at`, `hover_in`, and
  `labels_at` all call it. The five exceptions are all genuine and each is named at its call site:
  `mode=` with no value in two spellings, `mode:` with no type in a multi-line signature,
  `let value:` with no type, and the deliberate `let broken( = { missing }`. Finding five where the
  finding assumed one is the better outcome — four of them are the half-typed editor states this
  change exists to answer, so they document intent rather than debt.

### Questions

- None. Both questions the 16:39 review raised were answered in the fix pass and the answers hold up:
  declining `null` follows from D5, and the text-run comment settles the groundwork-or-leftover
  question.

### Summary

Six of the seven findings are verified and one is reopened. RF10 and RF11 — the two substantive ones
— are fixed correctly and more thoroughly than their notes claim: every spelling of a negated
literal now answers including the doubly-negated `{- -42}`, the unresolved-type guard is exhaustive
over `Type` with no wildcard arm, and both fixes are pinned by fixtures that fail without them. RF12,
RF13, RF15, and RF16 are confirmed as described.

RF14 is reopened, and the reason generalizes into RF17. Its fix added two entries to
`specs/future.md` describing hover gaps that do not exist: `[1, 2]` is not NX list syntax, and hover
answers correctly inside the real spelling `{a b}` for literals and references alike. The conclusion
was drawn from a syntax error, which hover declines by design. That is the third time an invalid
fixture has produced a wrong conclusion in this change, and the first time one reached a document
that outlives it — `specs/future.md` is where a future maintainer would start, and it currently
points at a type-environment bug in lists that is not there. RF17 proposes the one-line guard that
would have caught all three.

`cargo test --workspace` is green at 1492 with no failures, `cargo fmt --check` reports no diff in
any file this change wrote, and `openspec validate resolve-editor-positions --strict` passes. The
throwaway probe used for these measurements was removed; the working tree is unchanged by this pass.


## Fix pass — 2026-09-05 (RF14 reopened, RF17)

Both fixed. Recorded as tasks 11.6–11.7.

The reopening is correct in every particular, and I verified each claim rather than taking it: `[1,
2]`, `let value:int[] = [1, 2]`, and `let value = ()` each emit one `syntax-error` diagnostic, while
`{1 2}`, `{"ab" "c"}`, `{1 -42}`, and a reference in `{ab 2}` all parse clean and hover correctly.
The list entry in `specs/future.md` described a type-environment bug that does not exist; it is
deleted, and `hover_inside_a_list_reports_the_item_it_is_on` now pins the five measurements that
disprove it, so the claim cannot be re-derived from a bad fixture. The unit-literal entry is
re-anchored on `{(⟨cursor⟩) 2}` — clean parse, still no hover, with the `2` beside it reporting `int`
as the contrast that makes it a real gap. The three original entries and the `null` entry were
re-measured on clean fixtures and all hold.

RF17's guard is the fix that keeps this from recurring, and applying it found the shape of the
problem was slightly different from the finding's: five fixtures in the suite do not parse, not one.
Four are deliberate half-typed editor states — `mode=` with no value, `mode:` with no type — which
are the positions this change exists to answer, so they are not defects to fix but exceptions to
name. `hover_at_incomplete` and `labels_at_incomplete` name them at each call site.

`cargo test --workspace` passes with no failures, `nx-language-service` now at 65 tests.
`cargo fmt --check` reports no diff in any file this change wrote.
`openspec validate resolve-editor-positions --strict` passes.

Both findings are marked 🟡 Fixed rather than ✅ Verified: verification belongs to the reviewer that
raised them.

## New Findings Discovered During 2026-09-05 19:41 Verification

### ✅ Verified - RF18 `assert_same_completions` is the one fixture entry point where the parse exception is silent rather than named

- **Severity:** Low
- **Evidence:** RF17's fix established the rule that a fixture either parses or says at its call site
  why it does not — `hover_at_incomplete` and `labels_at_incomplete` exist to make the exception
  visible. `assert_same_completions` (`crates/nx-language-service/src/lib.rs:2057`) builds two
  snapshots of its own and calls neither the guard nor an `_incomplete` helper, so it is exempt by
  omission. The exemption is substantively correct — all four call sites (`:2686`, `:2696`, `:3114`,
  `:3121`) pass half-typed fixtures (`<Panel ⟨cursor⟩/>`, `mode=⟨cursor⟩`, `fit=⟨cursor⟩`), which are
  the editor states the parity spec is about — so nothing is wrong today. What is missing is the
  statement: a reader comparing the three guarded helpers against this one cannot tell whether it was
  considered and exempted or simply missed, and a fixture added here later gets neither the guard nor
  the naming convention that would explain its absence.
- **Recommendation:** A line in the existing doc comment saying the fixtures are half-typed by
  construction — the parity guarantee is about layout, and both spellings are mid-edit — so the
  exemption reads as a decision. No behavior change; this is the last place the RF17 convention is
  not visible.
- **Fix:** the doc comment now states the exemption and its reason — a parity fixture is half-typed
  on both sides, because the positions where layout could change the answer are the ones mid-edit —
  and points at `hover_at_incomplete`/`labels_at_incomplete` as the same convention at call-site
  scope. No behavior change.
- **Verification:** Confirmed. `cargo test --workspace` green at 1493, `cargo fmt --check` clean on
  the file. Every entry point that builds a fixture snapshot now either calls `assert_fixture_parses`
  or says in words why it does not.

### Questions

- None.

### Summary

Both findings verified; one new low-severity finding.

RF14's second pass is correct and ends better than the entry it replaces. Every surviving claim in
"The Positions Still Unanswered" re-measures on fixtures I confirmed emit zero syntax diagnostics,
the unit-literal entry now rests on a contrast that actually holds (`{()  2}` — the `()` silent, the
`2` reporting `int`), and the deleted list claim is pinned against re-derivation by
`hover_inside_a_list_reports_the_item_it_is_on`. Pinning the disproof in a test rather than only
deleting the prose is what makes this durable: the wrong conclusion now costs a red test to restate.

RF17 I verified by re-injecting the original bad fixture rather than by reading the guard. The suite
fails with `fixture is not valid NX: "let value = [1, 2]\n" — ["Syntax error"]`, which is the
diagnosis that was missing every one of the three times an invalid fixture produced a wrong
conclusion here. Applying the guard also turned up five deliberate exceptions where the finding
assumed one, and four of them are half-typed editor states — naming them documents what this change
is for, so the wider result is the better one.

RF18 is what is left: `assert_same_completions` is exempt from the guard by omission rather than by
statement. Correct today, unstated for the next person.

`cargo test --workspace` is green at 1493 with no failures. `cargo fmt --check` on this change's two
crates reports one diff, `crates/nx-hir/src/scope.rs:328`, which `git diff HEAD` confirms this change
did not write. `openspec validate resolve-editor-positions --strict` passes. The probe used for these
measurements was removed and the injected fixture reverted; the working tree is unchanged by this
pass.

## Deferral audit — 2026-09-05 20:12

Archiving check: every open item this change discovered must live in `specs/future.md` or a future
change, not in these artifacts. Four were recorded only here and have been moved out.

- **`{⟨cursor⟩-42}`** (RF10, "Not covered") — the offset between `{` and a sign. Added to
  "Editor Hover: The Positions Still Unanswered" with the boundary tie-break that causes it and the
  warning that changing it is not local.
- **Go-to-definition and rename** — an explicit design Non-Goal, recorded nowhere else. Now
  `specs/future.md`, "Editor Navigation: Go-To-Definition And Rename", including what
  `PositionContext` already provides and that rename's reverse index is the larger half.
- **`cargo clippy -p nx-hir --all-targets` does not compile** (`approx_constant` at
  `crates/nx-hir/src/ast/expr.rs:382`) — pre-existing, found here. Now `specs/future.md`, "Lint And
  Format Gates Do Not Cover The Whole Workspace", with the point that matters: `nx-hir`'s test
  targets have never been linted.
- **Pre-existing rustfmt drift** in four files — same section. It is what stops `cargo fmt --check`
  from being usable as a CI gate.

Already recorded outside before this audit and re-confirmed: built-in element content and property
values never type-checked (covers the markup-body parameter and the text-run span), the five
unanswered hover positions, and the per-request cost decisions including the linear
`innermost_expr_at`. All findings RF1–RF18 are `✅ Verified`; nothing in this change is left open.
