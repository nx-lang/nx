# Review: add-visual-property-editing

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/{language-protocol, playground, source-edits,
source-tree, viewer, visualizer-declarations, visualizer-library}/spec.md  
**Reviewed code:** `git diff a6259e1..HEAD` (commits `1815ec3`, `dbf4a4b`): `crates/nx-language-service`
(`source_edit.rs`, `named_values.rs`, `source_tree.rs`, `visualizers.rs` and their tests), `crates/nx-api`
(`visualizer.nx`, `program_visualizers.rs`, visualizer declaration tests), `crates/nx-types/src/infer.rs`
(hint diagnostics), `crates/nx-hir/src/lower.rs`, `bindings/wasm` and `bindings/node` (ABI 10),
`packages/language-{protocol,core,http,client}`, monaco test fakes, `packages/viewer/src/{edit,editors,index,styles,card}.ts`
and `test/editing.test.ts` with `test/fixtures/sources.json`, `sites/playground/src/{App.tsx,reading/ReadingView.tsx,worker/session.ts,language/worker.ts,examples/nx/visual-test-cases.nx}`,
the viewer and protocol READMEs, the website reference pages and `src/vscode/CHANGELOG.md`.  

Checks run: `cargo test -p nx-language-service` (272 passed); `packages/viewer` build and
`node --test 'dist/test/**/*.test.js'` (142 passed); `sites/playground` `src/reading/*.test.mjs` (9 passed).
Suspected bugs were confirmed with throwaway scripts (kept outside the repository) that drive the wasm
language snapshot and the built viewer under jsdom.

## Findings

### 🟡 Fixed - RF1 Visual edits any reference, so a parameter or a loop variable is replaced by a literal
- **Severity:** High
- **Evidence:** `packages/viewer/src/edit.ts:207-218` makes every whole-value `reference` or `member` node
  whose path is a dotted name an edit target. It does not check that the name is a named value (a
  parameterless `let`). In the playground's own "Visual test cases" example, `editTargets` returns
  `Special.body.name.value`, the `{name}` parameter in `let <Special name:uitext /> = <Drink ... name={name} />`,
  as a `text` target. It also returns `root.body.items[5][0].then[0].size.value`, the `{size}` loop variable
  in `for size in sizes { ... size={size} ... }`, as a `case` target. A click opens an editor on either one.
  Committing writes a literal over the binding (`name="x"`, `size=large`), which silently changes what the
  component or the loop means. In phase 1 these opened Source. The viewer spec ("A reference to a named value
  edits with its type's editor") and the design limit this to named values. The tree already tells them
  apart: a reference to a `let` carries `declaration` pointing at a `kind: "value"` entry (`banner.value.tint.value`
  → declaration 26 `accent`, and a member's `.object` child → `theme`). The parameter and loop-variable
  references carry no `declaration`.
- **Recommendation:** In `targetOf`, accept a reference only when its root reference node (the node itself,
  or a `member`'s innermost `object` child) has a `declaration` whose kind is `value`. Anything else falls
  through to `nx-open`. Add viewer tests over the test-cases fixture: `{name}` in `Special` and `{size}` in
  the `for` fire `nx-open` and are not `.editable-value`. **Confidence in fix: high.**
- **Fix:** In `edit.ts`, `namesValue` accepts a reference only when its name, or a member's innermost object, has a `declaration` of kind `value`; anything else falls through to `nx-open`. New `bindings` fixture and tests: `{name}` in `Special` and `{size}` in a `for` are not targets and open Source, while `{accent}` and `palette.accent` still edit. Viewer spec scenario "A parameter is not a named value" and README row added.

### 🟡 Fixed - RF2 An empty range (`min` above `max`) reaches the viewer and makes every number unenterable
- **Severity:** Medium
- **Evidence:** `editor-hint-range` is only a warning (`crates/nx-types/src/infer.rs:1792`).
  `editor_hints` (`crates/nx-language-service/src/source_tree.rs:2381-2403`) still passes both bounds to the
  tree: `visualizer Spacing = <ValueVisualizer min=10 max=0 />` gives `{"min":10,"max":0}` on `Card.pad`.
  The viewer then:
  - opens a slider (`isSlider`, `editors.ts:155`) with `min > max`;
  - has `rangeProblem` (`editors.ts:139`) refuse every entry. Measured: `5` gives "At least 10", `10` gives
    "At most 0", and `20` gives "At most 0".

  So a warning-level hint stops Visual from editing the property at all. `clamp` already guards `low > high`,
  but `rangeProblem` and `isSlider` do not.
- **Recommendation:** Treat an empty range like a mismatched hint: when both `min` and `max` resolve and
  `min > max`, leave them out of `SourceEditorHints`, with a source-tree test. Also guard `rangeProblem` and
  `isSlider` with `min <= max`. **Confidence in fix: high.**
- **Fix:** `editor_hints` now leaves out both `min` and `max` when `min > max` (source-tree test `an_empty_range_is_left_out`, spec scenario added). `rangeProblem` and `isSlider` also treat an empty range as no range (viewer test).

### 🟡 Fixed - RF3 `namedValues` ignores an optional property's `?`, so optional members and `T?` lets are never offered
- **Severity:** Medium
- **Evidence:** `crates/nx-language-service/src/named_values.rs:102` resolves `expected` from the
  property's written type. For `tint?:Color` that is just `Color`, because the `?` is the property's flag and
  not part of its type. An optional member is then widened to `Color?` (`:186-191`) and fails against `Color`.
  The comment there says this is meant to happen "only [at] a site that admits" absence, but no site ever
  admits it. Probed through wasm: for `type Card = { tint?:Color ... }`, with `type Theme = { accent:Color muted?:Color }`,
  `let theme = <Theme accent=... muted="#222222" />` and `let maybe:Color? = ...`, the answer for `tint` is
  only `theme.accent`. Yet `<Card tint={theme.muted} />` and `<Card tint={maybe} />` type-check with no
  diagnostic. The spec asks for the values "whose type the type checker accepts for the property".
- **Recommendation:** When `declared.flags` contains `Optional`, use
  `expected = nx_types::read_type(&expected, true)`, as the checker reads `p?:T`. Add a test where an
  optional property is offered an optional member and a `T?` let, and a required one is offered neither.
  **Confidence in fix: high.**
- **Fix:** `named_values_in` reads the expected type with `nx_types::read_type(&expected, optional)` for an optional property. Test `an_optional_property_takes_optional_members_and_values` covers an optional member and a `T?` let offered to `tint?:Color`, and neither offered to a required property.

### 🟡 Fixed - RF4 A case or a negative number over a braced reference in a `let` is refused, against the spec
- **Severity:** Low
- **Evidence:** `source_edit.rs:248` drops a braced reference's braces only when its owner is an
  `Attribute`. At a `let` value the braces stay, so `unbraced` (`:255`) is false and a case or signed number
  is refused. Probed: `let x:S = {s0}` set to case `large` gives `invalidValue`, and `let x:int = {total}` set
  to `-4` gives `invalidValue`. The spec says "A literal or case that replaces a braced reference SHALL be
  written in the place's usual spelling, its braces dropped where the literal reads without them", and a
  `let`'s value reads `large` or `-4` unbraced. The viewer offers both edits: `edit.ts:212` sets
  `signed` for a `declaration` owner, and a `Size` reference opens the case popover. So the reader sees a
  pending value that the next tree reverts.
- **Recommendation:** Accept `Some(SourceRole::Declaration)` as well as `Attribute` at `source_edit.rs:248`,
  and add both probes as tests. **Confidence in fix: high.**
- **Fix:** `set_value` drops a braced reference's braces at a `let` value as at an attribute. Test `a_literal_or_case_over_a_braced_reference_in_a_let_drops_the_braces` covers a case and `-4`.

### 🟡 Fixed - RF5 The viewer's type sets miss `int32` and `int64`, and accept `bool` and `float`, which NX does not have
- **Severity:** Low
- **Evidence:** `edit.ts:61` sets `NUMBER_TYPES = {int, float, float32, float64}`, and `edit.ts:353` sets
  `BOOLEAN_TYPES = {boolean, bool}`. NX's primitives are `string int int32 int64 float32 float64 boolean object`
  (`nx_syntax::PRIMITIVE_TYPE_NAMES`). For `type Big = { a:int64 = 1 b?:int32 ... }`, `a=5` is no edit target,
  and `b` lists with no editor, so its dimmed row opens Source. The service treats `int32` and `int64` as
  numbers for hints (`source_tree.rs` `hint_value_kind`, `infer.rs` `hint_value_kind`), so their slider and
  range hints are resolved but never used. The `adding` fixture declares `iced:bool = false`, an unknown type
  in NX, and its tests pass only because the viewer accepts `bool`. The literal case predates this change, but
  unwritten properties, references and hints now depend on these sets.
- **Recommendation:** Use `{int, int32, int64, float32, float64}` and `{boolean}`, change the fixture to
  `iced:boolean`, and add an `int64` case to the unwritten-properties test. **Confidence in fix: high.**
- **Fix:** `NUMBER_TYPES` is `{int, int32, int64, float32, float64}` and `BOOLEAN_TYPES` is `{boolean}`. All viewer fixtures now write `boolean`. The `bindings` fixture has `big:int64` and `few?:int32`, with a test of their targets and unwritten editors.

### 🟡 Fixed - RF6 Clicking a row's `⋯` again does not close its menu
- **Severity:** Low
- **Evidence:** `#onDocumentPointerDown` (`index.ts:369-373`, a capture listener on the document) closes the
  menu on any press outside `#menu`, including a press on the row's own `.row-menu` mark. The click that
  follows then finds `#menuKey` unset and opens the menu again, so the toggle branch at `index.ts:2951` never
  runs in a browser. Reproduced under jsdom with pointerdown → pointerup → click on the mark twice: the
  menu is still open after the second click. The tests only call `.click()`, which skips pointerdown. Also,
  the mark's `⋯` text is part of the row's text, so the row's spoken words, built by the text walk near
  `index.ts:180`, end in "⋯".
- **Recommendation:** In the pointerdown handler, leave the menu alone when the composed path includes the
  open row's `.row-menu`, or remember the press as `#closedOnPress` does for popovers. Add a test that sends
  pointerdown before each click. Skip `.row-menu` in the words walk. **Confidence in fix: high.**
- **Fix:** The document pointerdown handler leaves the menu open when the press is on the open row's own `.row-menu`, so the click that follows closes it. `spoken` skips `.row-menu`. A test sends pointerdown, pointerup and click twice and checks the announcement; it fails without either change.

### 🟡 Fixed - RF7 A property written only inside a property-list `if` counts as unwritten, and adding it is overridden
- **Severity:** Low
- **Evidence:** Attributes under a property-list condition hang under a `condition` or `match` node, not
  the element. `unwrittenProperties` (`edit.ts`, direct `attribute` children) and `add_property`
  (`source_edit.rs:509-517`, `candidate.parent == Some(at)`) both count only direct attributes.
  `<Drink code="A" name="n" if flag { iced=true } />` therefore lists `iced` under `+ N more`. Adding it gives
  `... name="n" iced=true if flag { iced=true } />`, where the conditional still wins when `flag` holds, and no
  diagnostic is reported.
- **Recommendation:** Treat a property written in any branch of a property-list condition or match as
  written. Leave it out of `+ N more`, and refuse it in `add_property` as `invalidValue` ("a property the
  element already writes"). Add a test for each. **Confidence in fix: medium.** It is a product call whether
  the property should instead be offered as the unconditional value.
- **Fix:** Product call taken: a property written in any branch of a property-list condition or match counts as written. `attributeElement` (viewer `tree.ts`) and `attribute_element` (Rust) find an attribute's element through conditions. `unwrittenProperties` leaves such a property out, and `add_property` refuses it as `invalidValue` while still placing new attributes after the element's own ones. The corpus round-trip test uses the same rule. Tests in both layers, and source-edits and viewer spec wording plus a scenario.

### 🟡 Fixed - RF8 A union case's element never offers the properties it does not write
- **Severity:** Low
- **Evidence:** `unwrittenProperties` returns nothing for a declaration of kind `union` (`edit.ts:448`), and
  `add_property` finds no `properties` on the union's declaration, so it answers `invalidValue`.
  `let promo = <Extra.syrup flavor="mocha" />` in the test-cases example never shows `+ 1 more` for
  `pumps:Count = 1`. The viewer requirement "A selected element offers the properties it does not write"
  makes no exception for case elements, and the tree has the case's properties (`cases[].properties`).
- **Recommendation:** Either support case elements (read the case's properties in both
  `unwrittenProperties` and `add_property`), or narrow the spec to record, component and element-function
  elements and say so in the README. **Confidence in fix: medium**, because which way to go is a product
  call.
- **Fix:** Product call taken: case elements are supported. `elementProperties` (viewer) and `element_properties` (Rust) give the union's shared properties and then the case's. They are used by `unwrittenProperties`, `declaredType`, `removalOf`, the property card and `add_property`. Tests: Rust `adding_to_a_union_cases_element`, and the viewer's `+ 1 more` on `<Extra.syrup>` adding `pumps`. Spec wording and a scenario added. `namedValues` still answers nothing for a case element's property, which leaves its editors reading as with no host function.

### 🟡 Fixed - RF9 A mismatched property hint hides the alias's valid hint of the same name
- **Severity:** Low
- **Evidence:** In `element_visualizer`'s `editor` closure (`source_tree.rs`), the property's own hints are
  merged with the alias's, keyed by name, before `editor_hints` drops the ones that do not fit. So
  `<PropertyVisualizer property=padding editor=dropdown />` on a `Spacing` whose `ValueVisualizer` says
  `editor=field` resolves to no `kind`: `dropdown` is dropped as a mismatch and the alias's `field` was already
  shadowed. With the alias's `min` and `max`, the viewer then shows a slider where the library asked for a
  field. The spec says a mismatched hint "SHALL be left out", which should leave the alias's hint in force.
- **Recommendation:** Filter each source's hints by fit (`editor_hints`' rules) before merging, so only
  fitting property hints shadow the alias's. Add a test with a mismatched `editor` over an alias's `editor`.
  **Confidence in fix: high.**
- **Fix:** Each source's hints are filtered by `hint_fits` before merging, so only fitting property hints shadow the alias's. Test `a_mismatched_property_hint_leaves_the_aliass_in_force`, with spec wording added.

### 🟡 Fixed - RF10 Units are drawn after the reading is measured, so a line that fit can overflow
- **Severity:** Low
- **Evidence:** In `#render`, the width-fitting passes (`index.ts:852-866`, `#wrapped()`) measure lines
  without units. `#markUnits(targets)` then appends ` px` after each hinted number (`index.ts:874`). A
  one-line element that only just fit, such as `<Card padding=8 gap=4 ... />`, can overflow once its units
  are added, and it gets no rows. This cannot be checked under jsdom.
- **Recommendation:** Add the unit marks inside `draw` (or in `renderSource`), before measuring, so the
  fitting passes see them. **Confidence in fix: medium.**
- **Fix:** `#markUnits` now runs inside `draw`, after each `renderSource` and before the fitting passes measure. It finds its spans in the freshly drawn body.

### 🟡 Fixed - RF11 Risky paths have no tests
- **Severity:** Low
- **Evidence and Recommendation:** Add tests for:
  - (a) Task 3.2's "Nothing moves": no test sets a newer tree after an `addProperty`. "keeps open across a
    newer tree" sets the same tree again.
  - (b) A reference whose type has no editor firing `nx-open`, and references that are not named values (RF1).
  - (c) Named values listed above a case popover's cases, including the segmented row falling back to the
    list (`#showNamed` removes `segmented`).
  - (d) The playground: no test covers the worker answering `namedValues` (`worker/session.ts`) or the
    "Picking a named value" scenario through `reading/edits.ts`.
  - (e) Rust `addProperty` on an element whose last attribute's value spans lines, and on one with a
    property-list condition (RF7).
  - (f) An empty range (RF2).

  **Confidence in fix: high.**
- **Fix:** Added tests:
  - (a) "moves nothing when the newer tree writes the added property" sets a real newer tree after `addProperty`.
  - (b) References that are not named values (the RF1 tests).
  - (c) Named values in a segmented case popover, falling back to a list and writing a reference.
  - (d) The playground worker answering `namedValues` for the test-cases example, and picking `theme.muted` writing `tint={theme.muted}` through the edit queue.
  - (e) Rust `adding_after_a_value_over_several_lines` (on the tag's line and a line each) and the property-list condition case (RF7).
  - (f) The empty range (RF2).

### 🟡 Fixed - RF12 A new test writes a one-line union with a leading bar, against AGENTS.md
- **Severity:** Low
- **Evidence:** `crates/nx-api/src/visualizer_declarations_tests.rs:215` writes
  `type Extra = | none | syrup { flavor:string }`. AGENTS.md says to omit the leading `|` when the cases are
  on one line. The test-cases example has the same spelling (`visual-test-cases.nx:23`), but that predates
  this change.
- **Recommendation:** Write `type Extra = none | syrup { flavor:string }` in the test and, while there, in
  the example. **Confidence in fix: high.**
- **Fix:** Both spellings are now `type Extra = none | syrup { ... }`.

## Questions
- RF7 and RF8 were product calls. The fix took the reviewer's first option for each: a conditional write
  counts as written, and case elements are supported. Should a property written only under a condition count as written? Should
  union-case elements list their unwritten properties, or should the spec exclude them?
- `namedValues` orders imports by module identity, not by import order (`named_values.rs` sort key). Is
  that what "then each import's" means?
- `namedValues` leaves out only the `let` the element is in. A pair of lets can still be made to refer to
  each other (`a.tint={b.tint}` and `b.tint={a.tint}`). Is that acceptable for now?

## Summary
- The change is largely complete and well tested at the unit level, and the Rust, TypeScript and playground
  layers are wired consistently, with ABI 10 in both places.
- The main problem is RF1. The viewer's new reference editing reaches parameters and loop variables, so a
  click and a typed value can rewrite a component's or a loop's binding into a literal.
- RF2 and RF3 are smaller correctness gaps in hint ranges and in named values for optional properties.
- The rest are edge cases, a convention nit and missing tests.
