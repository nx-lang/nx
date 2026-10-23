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

### ✅ Verified - RF1 Visual edits any reference, so a parameter or a loop variable is replaced by a literal
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
- **Verification:** Confirmed. `namesValue` accepts a reference only when its name, or a member's innermost object, has a `declaration` of kind `value`. Probed through the wasm tree: an imported `let` (`name={brand}`) and an imported `let`'s member (`tint={theme.accent}`) still edit, and a component parameter's member (`tint={t.accent}`) does not. The `bindings` tests check both the targets and `nx-open` on a double-click. **Confidence: high.**

### ✅ Verified - RF2 An empty range (`min` above `max`) reaches the viewer and makes every number unenterable
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
- **Verification:** Confirmed. `editor_hints` leaves out both bounds when `min > max` (`an_empty_range_is_left_out`, which also checks that `step` stays), and `rangeProblem` and `isSlider` treat an empty range as no range (viewer test). One gap remains, listed under Questions: when merging makes the range empty (a property's `min=100` over an alias's `max=64`), the range is now left out with no warning, because analysis only checks the hints each visualizer element writes itself. **Confidence: high.**

### ✅ Verified - RF3 `namedValues` ignores an optional property's `?`, so optional members and `T?` lets are never offered
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
- **Verification:** Confirmed. `read_type(&expected, optional)` reads `p?:T` the way the checker does. The new test offers an optional member and a `T?` let to `tint?:Color`, and offers neither to a required property. A side effect is listed under Questions. **Confidence: high.**

### ✅ Verified - RF4 A case or a negative number over a braced reference in a `let` is refused, against the spec
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
- **Verification:** Confirmed. `set_value` now drops the braces at a `Declaration` owner as well as at an `Attribute` owner. The test covers a case (`let x:S = large`) and `-4`. A reference that is a sequence item keeps a `Sequence` owner, so it still refuses a sign or a bare case. **Confidence: high.**

### ✅ Verified - RF5 The viewer's type sets miss `int32` and `int64`, and accept `bool` and `float`, which NX does not have
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
- **Verification:** Confirmed. Both sets now match `PRIMITIVE_TYPE_NAMES`. No `bool` is left in the fixtures. The `bindings` test checks that `big:int64` is a number target and that `few?:int32` lists with a number editor. **Confidence: high.**

### ✅ Verified - RF6 Clicking a row's `⋯` again does not close its menu
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
- **Verification:** Confirmed. A press on the open row's own `.row-menu` no longer closes the menu, so the click toggles it shut. A press on another row's mark still closes the menu, and that click opens the other row's. `spoken` skips the mark. The new test sends pointerdown, pointerup and click twice, then checks the label and the live region. **Confidence: high.**

### ✅ Verified - RF7 A property written only inside a property-list `if` counts as unwritten, and adding it is overridden
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
- **Verification:** Confirmed, and consistent across layers. `attribute_element` (Rust) and `attributeElement` (viewer) climb through `condition` and `match` nodes. Probed through wasm with `if flag { iced=true } else { tint={shade} }` and `if mode is { light => iced=true dark => shots=2 }`: `unwrittenProperties` leaves those properties out, and `addProperty` refuses them as `invalidValue`. A new property still goes after the element's own attributes, on the tag's line or on a line of its own. An element whose only attributes are conditional takes the new one on its tag's line. The source-edits and viewer specs, the README and the corpus round-trip test all use the same rule. The fix also routed `declaredType`, `hintsOf` and `propertyOf` through `attributeElement`, but not the format, hover-card and removal lookups (RF13). **Confidence: high.**

### ✅ Verified - RF8 A union case's element never offers the properties it does not write
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
- **Verification:** Only partly fixed. `addProperty`, `unwrittenProperties`, `declaredType`, `removalOf` and the property card now take the union's shared properties and then the case's, the same way in Rust and the viewer, and the source-edits and viewer specs say so. Two places still ignore the case's properties:
  - `namedValues` (`named_values.rs:74-93`) still looks only in `declaration.properties`, and only searches records, components and functions for the declaration's origin. Probed through wasm: for `<Extra.syrup flavor="x" />` with `let mocha = "mocha"` in scope, it answers `[]` for `flavor`, while a record's `name:string` gets `mocha`. The source-edits spec says a property the element does not have answers an empty list. Under RF8 the element now has `flavor`, and the viewer requirement "Editors offer the named values the host gives" makes no exception for case elements.
  - `list_property_types` (`source_tree.rs:1994-2013`) lists property types only for records, components and element functions, as the source-tree requirement "The types of unwritten properties are listed" says. So a case's property typed by an alias or a union that no node refers to is not in the declaration table. Probed with an imported `type Extra = none | syrup { flavor:string strength:Strength = mild shots:Shots = 1 }` used only as `<Extra.syrup flavor="mocha" />`: the dimmed rows `strength` and `shots` get no editor and open Source. The same properties on a record get a case popover and a number field.

  **Recommendation:** in `named_values_in`, find the property with `element_properties` and resolve a case property's written type from the union's case. In `list_property_types`, also list the types of a union's shared and case properties, and extend the source-tree requirement to cover case elements. Add a test for each. **Confidence in fix: high.**
- **Fix (second round):** `named_values_in` finds the property with `element_properties`, matches a union's origin, and resolves a case property's written type from the element's case (test `a_union_cases_element_takes_named_values`). `list_property_types` also lists the types of a union's base and case properties (test `a_union_cases_property_types_are_listed`, with an imported `Extra` whose `strength:Strength` and `shots:Shots` nothing else names). The source-tree requirement now names union cases.
- **Verification (second round):** Confirmed. `named_values_in` finds the property with `element_properties`, so
  it sees the union's shared properties and then the case's, and it reads a case property's written type from
  the union's own module. `list_property_types` treats a union as an element, so its base's property types
  and every case's property types are listed, one level, as for a record. Probed through wasm with an
  imported `type Extra extends ExtraBase = | none | syrup { flavor:string strength:Strength = mild shots:Shots = 1 }`
  used only as `<Extra.syrup flavor="mocha" />`: the dimmed rows are `note` (text), `level` (number),
  `strength` (case) and `shots` (number). `namedValues` answers `lucky` for `shots` and for the base's
  `level`, `bold2` for `strength`, and the string values for `flavor`. Both new tests fail without the fix.
  **Confidence: high.**

### ✅ Verified - RF9 A mismatched property hint hides the alias's valid hint of the same name
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
- **Verification:** Confirmed. `hint_fits` filters the property's hints before the alias's are merged in, and `editor_hints` filters again when it reads them. The test has a mismatched `editor=dropdown` over an alias's `editor=field min=0 max=10`, and checks that the result is `field` with max 10. **Confidence: high.**

### ✅ Verified - RF10 Units are drawn after the reading is measured, so a line that fit can overflow
- **Severity:** Low
- **Evidence:** In `#render`, the width-fitting passes (`index.ts:852-866`, `#wrapped()`) measure lines
  without units. `#markUnits(targets)` then appends ` px` after each hinted number (`index.ts:874`). A
  one-line element that only just fit, such as `<Card padding=8 gap=4 ... />`, can overflow once its units
  are added, and it gets no rows. This cannot be checked under jsdom.
- **Recommendation:** Add the unit marks inside `draw` (or in `renderSource`), before measuring, so the
  fitting passes see them. **Confidence in fix: medium.**
- **Fix:** `#markUnits` now runs inside `draw`, after each `renderSource` and before the fitting passes measure. It finds its spans in the freshly drawn body.
- **Verification:** Confirmed by reading the code. `#markUnits` runs inside `draw` after every `renderSource`, so the `#wrapped()` passes measure lines with their units. It reads spans from the freshly drawn body and keeps the last one for each key, as `#rendered` does. No other path redraws the body. jsdom cannot measure widths, so this has no test, but the existing unit tests pass. **Confidence: medium-high.**

### ✅ Verified - RF11 Risky paths have no tests
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
- **Verification:** Confirmed. Each of (a) to (f) has a test that exercises the path: (a) sets a real newer tree that writes `iced` and checks `+ 3 more` and the selection; (d) runs the worker's `namedValues` on the test-cases example and applies the reference through the edit queue. All pass: `cargo test -p nx-language-service` 279, viewer 151, playground `edits.test.mjs` 6. **Confidence: high.**

### ✅ Verified - RF12 A new test writes a one-line union with a leading bar, against AGENTS.md
- **Severity:** Low
- **Evidence:** `crates/nx-api/src/visualizer_declarations_tests.rs:215` writes
  `type Extra = | none | syrup { flavor:string }`. AGENTS.md says to omit the leading `|` when the cases are
  on one line. The test-cases example has the same spelling (`visual-test-cases.nx:23`), but that predates
  this change.
- **Recommendation:** Write `type Extra = none | syrup { flavor:string }` in the test and, while there, in
  the example. **Confidence in fix: high.**
- **Fix:** Both spellings are now `type Extra = none | syrup { ... }`.
- **Verification:** Confirmed. Both spellings are fixed. No other line added since `a6259e1` writes a one-line union with a leading bar. **Confidence: high.**

## New Findings Discovered During 2026-10-22 08:20 Verification

### ✅ Verified - RF13 A value under a property-list condition takes its property's hints, but not its format or hover card
- **Severity:** Low
- **Evidence:** The RF7 fix sends `declaredType`, `hintsOf` and `propertyOf` (`packages/viewer/src/edit.ts`)
  through `attributeElement`. So a value written under `if flag { ... }` now gets its property's type,
  its `PropertyVisualizer` hints and its named values. Three lookups still read only the attribute's
  direct parent, which there is the `condition` node: `formattedNodes` (`source.ts:1421-1440`),
  `propertyCard` (`card.ts:132-138`) and `removalOf` (`edit.ts:168-183`). Probed with
  `<PropertyVisualizer property=tint format=color />` and `<PropertyVisualizer property=size unit="px" min=0 max=9 />`
  on `<Drink code="B" if flag { tint="#ffffff" size=2 } />`:
  - `size` takes its hints (`{min:0,max:9,unit:"px"}`).
  - `tint` edits as `text`, though `<Drink tint="#ffffff" />` edits it as `color`.
  - The conditional `tint` attribute has no hover card.

  Before the fix such a value took none of these, so it was consistent. Now it takes half. Without
  Reset or Remove, a conditional row stays safe, so `removalOf` can stay as it is.
- **Recommendation:** Use `attributeElement` in `formattedNodes`, and in `propertyCard` to find the
  element it passes to `elementProperties`. Add a test over the
  `bindings` fixture's `sometimes` element with a `format=color` property. **Confidence in fix: high.**
- **Fix:** `formattedNodes`, `slotType` and the uitext marking in `source.ts`, and `propertyCard` in `card.ts`, now find the element through `attributeElement` and its properties through `elementProperties`. `removalOf` stays as it is, as recommended. Test "reads a value under a condition with its property's format and card" in the `bindings` fixture, with a `Tag` whose `tint` has `format=color`.
- **Verification:** Confirmed. `formattedNodes`, `uiTextNodes`, the slot type in `renderSource` and
  `propertyCard` all find the element through `attributeElement` and its properties through
  `elementProperties`. The other direct-parent reads left in the viewer (the scrub mark on an attribute's
  name, `#labelOf`, `removalOf`, the selection after a removal) need no change. Probed with the built viewer
  on values under `if flag { ... }`: a `PropertyVisualizer` `format=color` and a `Swatch` alias both give a
  `color` target and a format; `uitext` strings are marked; the hover card reads `(property) tint?: string`,
  also on a union case's element (`(property) tint?: Swatch`) and in an `else` branch. The new test fails
  without the fix. **Confidence: high.**

## New Findings Discovered During 2026-10-22 08:33 Verification

### ✅ Verified - RF14 `namedValues` leaves out an optional member that a non-literal record value writes
- **Severity:** Low
- **Evidence:** The new check in `named_values_in` (`named_values.rs:215-219`) skips an optional member when
  `member_value` finds no expression for it. `member_value` only reads an element or record literal. For any
  other value, such as a reference or a call, it answers `None`, so every optional member of that value is
  treated as never written. Probed through wasm, with `type Theme = { accent:string muted?:string }`:
  - `let base = <Theme accent="#a" muted="#b" />` offers `base.muted` to an optional `string` property.
  - `let alias = {base}` and `let viaFn = {mk("#1")}`, where `mk` returns `<Theme ... muted="#2" />`, offer
    only `alias.accent` and `viaFn.accent`.

  `nxlang run` on `<Card tint={alias.muted} />` and `<Card tint={viaFn.muted} />` gives `tint="#b"` and
  `tint="#2"`, so both members are always present. Before `c2fece1` they were offered. The spec leaves out
  only "an optional member the record neither writes nor defaults", and for these values nothing shows that.
  The `field.default.is_none()` test is also always true: the parser rejects `?` with a default.
- **Recommendation:** Leave a member out only when the value is an element or record literal that writes no
  entry for it, and count a write under a property-list condition or match (`Element::property_entries`) as
  a write, as RF7 does. Keep offering the optional members of any other value. Drop the
  `field.default.is_none()` test. Add `alias.muted` to `an_optional_property_takes_optional_members_and_values`.
  Also add the rule to the `namedValues` paragraph in `packages/language-protocol/README.md`, which repeats
  the spec's wording. **Confidence in fix: high.**
- **Fix:** `never_writes` leaves out a member only when the value is an element or record literal that writes no entry for it in any branch (`Element::property_entries`). A reference, a call or any other value keeps its optional members. The `field.default` test is gone. `an_optional_property_takes_optional_members_and_values` now covers `alias = {theme}` (offers `alias.muted`) and `sometimes = <Theme accent=... if flag { muted=... } />` (offers `sometimes.muted`). The rule is in the source-edits spec and the language-protocol README.

  The reviewer also noted that a declared record with a property-list condition lost its conditional properties when lowered: `<Theme accent="c" if flag { muted="d" } />` evaluated without `muted`, which the `sometimes` case also needed. `lower.rs` now keeps such an element an element, binding its entries as written, instead of a record literal of its direct properties. A regression test, `test_property_fragment_on_a_declared_record_is_kept`, in `nx-interpreter`'s conditionals tests fails without the change.
- **Verification:** Confirmed. `never_writes` leaves out a member only for an element or record literal that
  writes it in no branch, and keeps the members of any other value. Probed through wasm with a
  `Theme` imported and local: `alias.muted` and a call's `.muted` (when the function declares its return
  type) are offered; `muted` is left out of `<Theme accent=... if flag { size=3 } />` and of an imported
  `let` that never writes it; it is offered when only a match arm or an `else` writes it, and for a `let`
  whose value is an `if` of two records. Both test changes are meaningful: with the old `named_values.rs`
  the test misses `alias.muted` and `sometimes.muted`, and with the old `lower.rs` it misses
  `sometimes.muted` and the new interpreter test fails (`muted` is `None`).

  The lowering change keeps the conditional properties in the interpreter, and nothing that matches
  `Expr::RecordLiteral` breaks: such a record now takes the path an imported record or a component
  already took. Checked with `nxlang run` and probes: missing required, unknown, wrongly typed and
  duplicated properties are reported, inside and outside branches; defaults fill in; a generic record's
  `T=int` is still consumed; abstract records are refused; `.Update`, `Comp.Props`, `State.Update` in a
  handler and a same-module action with a condition all evaluate and dispatch with their conditional
  fields; field order and JSON output are unchanged. Hover and the source tree read the same as for a
  record without a condition. Executable codegen now refuses such a record (RF15), and a condition makes
  the element checker repeat a direct property's type error (RF16). `cargo test --workspace` (3395
  passed), the viewer (152 passed) and the wasm SDK's tests (all but the known `recursion-stack`) pass.
  **Confidence: high.**

## New Findings Discovered During 2026-10-22 08:50 Verification

### ✅ Verified - RF15 NX IR and TypeScript codegen now refuse a declared record with a property-list condition, and the changelog does not say so
- **Severity:** Low
- **Evidence:** Before `3234c1c`, `<Theme accent="c" if flag { muted="d" } />`, with `Theme` declared in the
  same module, lowered to a record literal, so codegen built it and silently dropped `muted`. It now lowers
  to an element, and `build_element_expression` (`crates/nx-codegen/src/builder.rs:2221-2229`) refuses every
  property-list condition: `nxlang codegen --target javascript` and the wasm `generateNxIr` both fail with
  "conditional element property fragments are not supported by executable codegen yet", while
  `evaluateNx` gives the right value. The playground's preview builds through `generateNxIr`
  (`sites/playground/src/preview/workbench.ts:198`), so such a document now shows "programFailed" where it
  used to preview. That matches how codegen already treats components and imported records, and failing
  loudly is better than a wrong value. But the `src/vscode/CHANGELOG.md` entry says only that the record
  "keeps the properties its active branch sets", and it sits under `## 0.8.0`, which is tagged `v0.8.0`,
  not under `## Unreleased`.
- **Recommendation:** Move the entry under `## Unreleased`, and say there that executable codegen (NX IR,
  TypeScript and JavaScript) reports such a record as unsupported, as it does a component's property-list
  condition, instead of dropping its conditional properties. Add a codegen test that pins the refusal for a
  same-module record. **Confidence in fix: high.**
- **Fix:** The changelog entry moved under `## Unreleased` in a `### Fixes` section. It now says the interpreter keeps the conditional properties, and that executable codegen (NX IR, TypeScript, JavaScript), and so the playground's preview, refuses such a record with `codegen-unsupported-construct`, as it does a component. New codegen test `a_record_with_a_property_list_condition_fails_before_emission` pins the refusal.
- **Verification:** Confirmed. The entry is under `## Unreleased`, in a `### Fixes` section above
  `## 0.8.0`, and says what the interpreter and executable codegen now do. Codegen's refusal is the
  `PropertyEntry::If | ConditionList | Match` arm of `build_element_expression`, the same one a component
  takes, so the "as it does a component" claim holds. The new test fails with `lower.rs` from before
  `3234c1c` (codegen then succeeds), so it pins the refusal. **Confidence: high.**

### ✅ Verified - RF16 A condition makes a direct property's type error repeat once per branch path
- **Severity:** Low
- **Evidence:** `property_paths_for_entries` (`crates/nx-types/src/infer.rs:5851`) expands an element's
  entries into one path per combination of branches, and each path checks its bindings, so a direct
  property is checked once per path. In the language service's diagnostics,
  `<Theme accent=5 if flag { muted="d" } if flag { size=1 } />` reports "Property 'accent' on 'Theme'
  expects string, found int" four times at the same range, and with one `if`, twice. Without a condition,
  `<Theme accent=5 />` reports it once. The repetition predates this change for components and imported
  records, but the RF14 lowering change brings it to records declared in the same module, which reported
  once before.
- **Recommendation:** Report a binding's type mismatch once per value expression (for example, remember
  the `value` ids already reported while checking the paths), and add a test asserting one diagnostic for
  a mistyped direct property beside one and two conditions, for a record and for a component.
  **Confidence in fix: medium.** The right place to deduplicate depends on how the element checker's
  other per-path diagnostics are reported.
- **Fix:** `InferenceContext::finish` drops a diagnostic that repeats an earlier one in every part (`without_repeats`, keyed by message, then compared in full). Any per-path diagnostic that comes out the same on several paths is one defect, so this covers the type mismatch and any other check that runs once per path. `test_a_property_mismatch_under_conditions_is_reported_once_per_element` checks a record with two conditions and a component with one: one mismatch each. It fails (4 and 2) without the change. The changelog's Fixes section says so.
- **Verification:** Confirmed. `check.rs` is the only caller of `finish`, and the language service gets
  its diagnostics through it: in the wasm language snapshot, a same-module record with two conditions, a
  component with two, and an imported record with two each report the mismatch once, while two elements
  with the same error on different lines still report two. Equality is the derived `PartialEq` over
  severity, code, message, every label's file, range and message, help and note, so only diagnostics that
  read the same in every respect merge, and those carry nothing the first one does not. The per-path
  checks already deduplicated unknown and duplicated properties by key and span; the type mismatch was
  the one left. With the drop logged, the whole `cargo test --workspace` (CLI tests included) drops only
  the new test's repeats, and type-checking the repository's 206 `.nx` files drops none, so no test,
  snapshot or count depended on a repeat. With the drop disabled, the new test fails (6 mismatches, not
  2). `cargo test --workspace` passes (3397). The cost on large files is RF17. **Confidence: high.**

## New Findings Discovered During 2026-10-22 09:07 Verification

### ✅ Verified - RF17 `without_repeats` is quadratic in diagnostics that share a message
- **Severity:** Low
- **Evidence:** `without_repeats` (`crates/nx-types/src/infer.rs:8528`) groups diagnostics by message and
  compares a new one with every kept one of the same message. Diagnostics that share a message but not a
  range, such as the same field mistyped in every instance of a record, all land in one bucket, so the
  pass is O(n²) in them. Timed natively in release, `check_str` over N lets, each `<T0 accent=5 />`:
  the pass takes 14 ms at 2,000, 0.57 s at 8,000 and 3.2 s at 16,000, against 0.4, 1.6 and 4.2 ms when the
  messages differ. At 16,000 the whole check goes from 0.69 s to 3.9 s. Through the wasm language
  snapshot, 8,000 such errors take 7.6 s against 5.7 s with distinct messages. A data file of a few
  thousand records whose type changed is plausible, and the language service rechecks on each edit.
- **Recommendation:** Make the lookup constant-time, for example by deriving `Hash` on `Diagnostic` and
  `Label` (`Severity` and `TextRange` already have it) and keeping a set of the diagnostics seen, or by
  keying on the message and the labels together. Add a test or a benchmark that checks many
  same-message diagnostics at distinct ranges. **Confidence in fix: high.**
- **Fix:** `Diagnostic` and `Label` derive `Hash`. `without_repeats` keeps an `FxHashSet` of the diagnostics seen, so each lookup is constant-time. Unit test `without_repeats_keeps_one_of_each_and_every_distinct_range` sends 20,000 same-message diagnostics at distinct ranges, each twice, and checks that 20,000 are kept in order. It counts and does not time, since wall-clock asserts flake in CI.
- **Verification:** Confirmed. `Hash` and `PartialEq` are both derived over the same fields (`Severity`,
  `String`, `Option<String>`, `Vec<Label>`, and in `Label` the file, `TextRange`, message and `primary`,
  all of which hash as they compare), so equal diagnostics hash alike and the set drops exactly what the
  old full comparison dropped. Nothing implements these traits by hand, and every field already had
  `Hash`; `cargo check --workspace --all-targets` and clippy on both crates are clean, the Node binding's
  `pnpm test` passes (37), and the wasm module rebuilds. Timed in release with a scratch probe, the
  pass over N same-message diagnostics at distinct ranges, each twice, takes 1.0, 4.5 and 10 ms at
  2,000, 8,000 and 16,000 (the old code: 29 ms, 1.08 s, 5.5 s, with equal output) and 46 ms at 64,000.
  `check_str` over N `<Theme accent=5 />` lets now costs what the same lets without an error cost: 0.30
  against 0.29 s at 16,000 and 0.70 against 0.68 s at 32,000. The unit test keeps every distinct range
  and the order and drops each repeat, so it fails for a key too coarse (message only keeps 1) or no
  deduplication (40,000); a return to a quadratic pass shows as the test taking about 20 s in debug, against
  0.08 s, rather than as a failure, which is the trade the fix note names. `cargo test -p nx-diagnostics -p
  nx-types` passes. **Confidence: high.**

## New Findings Discovered During 2026-10-22 23:17 Review

Scope of this pass: `git diff 705e5d0..HEAD` (`e7664a2`, the respec, and `0b6e718`, choices and
steppers in place of the slider and named values), checked against the whole change. Checks run:
`packages/viewer` `pnpm test` (155 passed). Suspected bugs were confirmed with throwaway scripts kept
outside the repository, which drive the built viewer under jsdom and the wasm language snapshot. The
removal of the slider, `stops`, braced number lists, `namedValues` and the `reference` value is
complete: no code, CSS, docs or tests still name them, and a written reference opens Source again
(`bindings` tests).

### ✅ Verified - RF18 A selected row's hover steppers vanish before the pointer can reach them
- **Severity:** Medium
- **Evidence:** `#hoverTools` (`packages/viewer/src/index.ts:2423-2445`) keeps the steppers only while
  the pointer is over the value itself (`closest(".editable-value.edit-number")`) or over the tools.
  But `#showTools` draws them after the value's unit (`index.ts:2407-2408`), and the CSS sets them 4px
  further right with `margin-left: 4px` (`styles.ts:661`), and a margin is not hit-tested. On its way
  from `8` to the steppers, the pointer crosses ` px` and that gap. Each one fires `pointerover` on an
  element that is neither the value nor the tools, which hides them. Confirmed under jsdom on
  `card.value.padding` (`8 px`), selected: after `pointerover` on the value the tools are connected.
  After `pointerover` on the unit, or on the `.line-property` around them, they are gone. With a
  unit, the steppers can be reached only by a pointer that jumps the whole unit in one move event.
  Without one, the 4px gap still hides them for any slow move. The viewer spec's "while the pointer
  is over its value on the selected row" cannot then be used. The existing test dispatches
  `pointerover` on the value and presses the stepper directly, so it never crosses the gap.
- **Recommendation:** Keep the tools while the pointer is over the value, its unit, the space between
  them and the tools, or the tools. For example, test `closest(".line-property, .sheet-row")` of the
  hovered value's row instead of the value. Or hide only once the pointer leaves the row or the tools,
  after a short grace like the card's. Use `padding-left` instead of `margin-left`, so the gap belongs
  to the tools' box. Add a test that hovers the value, then the unit, then the tools, and clicks.
  **Confidence: high.**
- **Fix:** In `#hoverTools`, a pointer over anything in the shown value's row (`#rowOf`: the closest `.line-property` or `.sheet-row`, else the value's parent) keeps the tools, so the unit and the gap no longer hide them. The CSS uses `padding-left: 4px` instead of a margin, so the gap is the tools' box. Test "keeps the steppers as the pointer crosses the value's unit and the gap to reach them" hovers the value, the unit, the row and the tools, clicks, and checks the tools go over another property.
- **Verification:** Partly fixed. In a property row the unit and the gap now keep the tools: rechecked under
  jsdom on a one-line `box.value.pad` and a multi-line `tall.value.pad`, both with `px`, and the padding
  makes the gap the tools' own box. But `#rowOf` (`index.ts:2488`) falls back to
  `closest(".editable-value")?.parentElement` outside a `.line-property` or `.sheet-row`. That is the
  value's parent for the value, and `null` for its unit, because `#markUnits` draws the unit as the
  value's sibling, not inside it. So a number with a unit outside a property row still loses its
  steppers on the unit. Confirmed under jsdom with `type Spacing = int`,
  `visualizer Spacing = <ValueVisualizer min=0 max=64 unit="px" />` and `let pad:Spacing = 8`, with
  `pad` selected: hovering `8` shows the tools, and hovering ` px` hides them. The new test covers
  only a property. The same check also caused RF29. Still to do: compare the hovered element with the
  shown value, its unit (`shown.nextElementSibling` when it is a `.unit`) and the tools, or use the
  shown value's own parent as the row for both. Add the `let` case to the test. **Confidence: high.**
- **Fix:** (second round) `#rowOf` is replaced by `#onTheWay(shown, element)`: the tools stay while the pointer is over the shown value, its unit (the value's next sibling with `.unit`), the value's parent, or its `.line-property`/`.sheet-row` itself, but not another value inside it. That covers a `let`'s unit outside any property row. Test "keeps a let's steppers over its unit, and moves them to the next number in a row" (fixture `adding` gained `let pad:Spacing = 8`).
- **Verification:** (second round) Confirmed. `#onTheWay` keeps the tools over the shown value,
  its unit, the value's parent, and the `.line-property` or `.sheet-row` element itself. Under jsdom
  the unit now keeps them in every place checked: a `let`'s number (`let pad:Spacing = 8`), a
  one-line and a multi-line element's `pad`, the items of a braced list in an element, and the items
  of a top-level `let xs:Spacing+ = { 5 6 }`. The space between a list's items (the list's own span)
  keeps them too. They still go over another property's row and over the gap between properties
  (the `.line-element`). The new test covers the `let` and passes with the other 160.
  **Confidence: high.**

### ✅ Verified - RF19 A tree that arrives during a stepper drag drops the drag and leaves the reading stuck in `stepping`
- **Severity:** Medium
- **Evidence:** `#render` calls `#dropSession` (`index.ts:832`). That clears `#stepping`
  (`index.ts:1805`) and removes the tools, but it does not remove the body's `stepping` class, which
  only `#onToolsPointerUp` removes (`index.ts:2511`). That handler then returns at once, because
  `#stepping` is undefined. A name scrub, by contrast, is carried across a new tree
  (`index.ts:862-865`, `#resumeScrub`). Confirmed under jsdom: press the up stepper of `gap` and drag
  20px, so the reading shows `14`. Then set the same tree, as a host does when the tree for an earlier
  edit arrives. The value reads `4` again. Releasing fires no `nx-edit`, and `.body.stepping` stays.
  Its rule `.body.stepping, .body.stepping * { cursor: ns-resize !important; user-select: none; }`
  (`styles.ts:690`) then gives the whole reading a resize cursor and stops text selection. This lasts
  until another stepper drag ends. A click on a stepper followed quickly by a drag, before the
  first click's tree arrives, is enough to reach this. `disconnectedCallback`
  (`index.ts:592-598`) leaves the class the same way.
- **Recommendation:** In `#dropSession`, also remove `stepping` from the body. Better, carry the drag
  across a new tree as `#scrub` is: keep `#stepping` through `#render`, re-insert the tools for its
  key, and show its value again. If the drag cannot go on, finish it from its last value. Add a test
  that sets a tree mid-drag and checks the class and the edit on release. **Confidence: high.**
- **Fix:** `#render` keeps `#stepping` across `#dropSession` and calls the new `#resumeStepping` after the editor is reopened: it redraws the tools past the same number (or the reopened field), recaptures the pointer, puts back `.body.stepping` and shows the reached value. When the number is gone the press ends. `#dropSession` now always removes `stepping` from the body, which also covers `disconnectedCallback`. Test "carries a drag on through a new tree, and ends it cleanly when the reading goes" sets the tree mid-drag, keeps dragging, checks one edit on release and the class cleared, then removes the element mid-drag.
- **Verification:** Confirmed. `#render` keeps the press, and `#resumeStepping` redraws the tools,
  recaptures the pointer and shows the value reached. `#dropSession` always clears `.body.stepping`,
  which also covers `disconnectedCallback`. The new test sets a tree mid-drag, drags on to `16`, and
  checks one edit and the class cleared on release. It then removes the element mid-drag. Also checked
  under jsdom with the field open: press up on `gap`'s open field, drag to `9`, set the tree, drag on
  and release. The field reads `14`, nothing fires, and `stepping` is gone. When the number is no
  longer in the tree, the press ends with no edit, as the note says. **Confidence: high.**

### ✅ Verified - RF20 A stepper drag that ends where it began leaves the value muted as pending
- **Severity:** Low
- **Evidence:** When a drag ends on its starting value, `#onToolsPointerUp` calls
  `#showStepped(target, stepping.from)` (`index.ts:2534-2538`). That calls `showValue`, which always
  adds `pending` (`editors.ts:69`). Nothing was sent, so no tree comes to clear the mark. Confirmed:
  drag `gap` up 10px and back, then release. There are no edits, and the value reads `4` with
  `.pending`. The name scrub's release handles this case (`index.ts:3034-3042`): it re-renders when
  nothing is pending. The two release paths duplicate the same logic, and the stepper's copy lost
  this branch.
- **Recommendation:** Share one "end a number drag" helper between `#onPointerUp` and
  `#onToolsPointerUp`. It commits when the value changed, and otherwise shows the pending value or
  re-renders. Add a test for a drag back to the start. **Confidence: high.**
- **Fix:** New `#endNumberDrag(target, from, value, commit)` is shared by the name scrub's `#onPointerUp` and `#onToolsPointerUp`: it commits when the value changed and the field would take it, and otherwise shows the pending value or re-renders. Test "shows the value as before when a drag ends where it began".
- **Verification:** Confirmed. `#endNumberDrag` is now the one release path for both the name
  scrub and a stepper drag with the field closed. A drag back to the start re-renders when nothing is
  pending, so the value reads `4` and is not muted (test). A cancelled drag with the field open puts
  the start back in the field. One side effect, not a defect: the re-render also drops the steppers
  under the pointer until it enters the value again. The name scrub's release already did the same.
  **Confidence: high.**

### ✅ Verified - RF21 Picking a choice skips the checks the field makes, and analysis lets through choices the field would refuse
- **Severity:** Medium
- **Evidence:** `#pickChoice` (`index.ts:2362-2372`) commits `row.dataset.value` without
  `#problemOf`. The viewer spec says "Choosing a choice SHALL commit its value as the field would".
  Confirmed under jsdom with `type Spacing = int` hinted `min=0 max=10` and the choices `2.5`, `99`
  and `-4`. Picking `99` fires `setValue` `99`, which the field refuses as `At most 10`. Analysis
  reports nothing for these choices: `check_choices` (`crates/nx-types/src/infer.rs:1811-1850`) only
  asks for a number literal, and `fitting_choices` (`crates/nx-language-service/src/source_tree.rs:2469`)
  passes all three to the tree. As a result:
  - `2.5` on an `int` writes `padding=2.5`, a type error, into the reader's document with one click.
  - `-4` on an item of a braced list (`signed: false`) is sent, refused as `invalidValue` by
    `set_value` (`source_edit.rs:233-237`), and stays shown, muted, until the next tree.
  - a color choice that is not a hex color (say `"red"` under `format=color`) is written, though the
    hex field refuses it.
- **Recommendation:** In the viewer, check a picked choice with `#problemOf`. When the check fails,
  keep the field open with the choice in it and the note, as Enter on a typed entry does. Or leave
  failing choices out of the list in `choicesOf`. In analysis, warn `editor-hint-mismatch` for a
  non-integer choice on an `int`, `int32` or `int64` property or alias, and leave such choices out in
  `fitting_choices`. Consider warning for a choice outside the resolved `min`/`max`. Add the viewer
  and analysis tests. **Confidence: high.**
- **Fix:** Viewer: `#pickChoice` checks a non-default choice with `#problemOf`; a refused one closes the list and stays in the field (or the color's hex input), selected, with the note shown at once. Analysis: `check_choices` takes an `integral` flag (`is_integral`: int, int32, int64) and warns `editor-hint-mismatch` ("must be a whole number") for a fractional choice; `fitting_choices` leaves such choices out for the new `HintValueKind::Integer`. Out-of-range choices are left to the viewer's check. Tests: viewer "keeps a choice the field would refuse in the field, with its note" (fixture `count` gained choices `5` and `99` under `max=10`), Rust `choices_keep_their_literals_and_a_whole_number_takes_no_fraction` and a new mismatch case in `visualizer_declarations_tests`. Spec, README: a refused choice stays in the field; new scenario "A refused choice".
- **Verification:** Confirmed in both layers. Viewer: `#pickChoice` checks a non-default choice
  with `#problemOf`. The new test picks `99` under `max=10`: nothing fires, the field holds `99` with
  `aria-invalid`, and the list is closed. Picking `5` then commits. A color, checked under jsdom:
  `<ValueChoice value="red" />` under `format=color` leaves `red` in the hex input with `aria-invalid`,
  and `#00ff00` commits. Analysis, through the wasm snapshot: a fractional choice warns
  `editor-hint-mismatch` on an alias of `int` (`ValueVisualizer`), an alias chain (`gap:Gap` with
  `Gap = Spacing`), an optional `int`, an `int64` and an `int+` property. The tree leaves each one
  out. It keeps `4.0` (as `4`) on an `int` and every choice on a `float32` or `float64`. Choices out of
  range stay listed and are refused when picked, as the spec now says. **Confidence: high.**

### ✅ Verified - RF22 An integer choice past 2^53 is written as a different number
- **Severity:** Low
- **Evidence:** A choice's number is read as `VisualizerAttribute::Number(OrderedFloat)`
  (`crates/nx-api/src/program_visualizers.rs:333`), and `fitting_choices` spells it with
  `number.0.to_string()` (`source_tree.rs:2478`). Confirmed through the wasm snapshot:
  `<ValueChoice value=9007199254740993 />` on an `int` reaches the tree as
  `{"kind":"number","text":"9007199254740992"}`. Picking it writes a number the library did not
  declare. `int64` ids and bit masks are plausible choices. A hex literal is respelled too (`0x10`
  becomes `16`), which is harmless but not as written.
- **Recommendation:** Keep a number choice's source text: store the literal's text beside the parsed
  value in `VisualizerChoice`, and pass it through as `SourceEditValue::Number { text }`. Add a
  source-tree test with a large integer, a negative real and a hex literal. **Confidence: high.**
- **Fix:** `VisualizerChoice` gained `number_text`, the literal's own value text (`choice_value_expr`, `number_text` in `program_visualizers.rs`: ints as written in decimal, hex as its value, floats as parsed, a sign kept); `fitting_choices` uses it. Test covers `9007199254740993`, `-4`, `0x10` (reads `16`) and `-2.5`.
- **Verification:** Confirmed. Through the wasm snapshot, `9007199254740993` on an `int64`
  property reaches the tree with every digit, `4.0` on an `int` reads `4`, and `-2.5` and `0.1` on
  reals read as written. The source-tree test covers a large integer, a sign, a hex literal (written
  as its value, `16`, which the finding called harmless) and a dropped fraction. `number_text` mirrors
  `number_value`, negation included. **Confidence: high.**

### ✅ Verified - RF23 With unlabeled choices, the default's row stacks `Default` over its value
- **Severity:** Low
- **Evidence:** `choiceRows` (`packages/viewer/src/editors.ts:196-243`) always gives the default's row
  two spans: a `.choice-label` reading `Default`, and the value. But the list takes the `labeled` class,
  and with it two columns, only when a choice has a label (`index.ts:2203`). An unlabeled list is
  `grid-template-columns: minmax(0, 1fr)`, and each row is a `subgrid` of it (`styles.ts:617-622`).
  So the default's two spans fall into two grid rows: `Default` above and `16` below. The spec reads
  the row as "`Default` and the default". No test or screenshot covers this. The fixture's and the
  playground's unlabeled choices (`tone`, `badge`) are on optional properties, which have no default.
- **Recommendation:** Give the list two columns whenever it has a default's row. Or lay the default's
  row out on its own (`display: flex; gap`) in an unlabeled list. Add a fixture property with
  unlabeled choices and a default. **Confidence: medium-high** (read from the CSS; jsdom does not lay
  out grids).
- **Fix:** CSS: in an unlabeled list, the default's row is `display: flex; gap: 16px`, so `Default` and the default share a line. The `count` fixture is an unlabeled list with a default; the test checks the list is not `labeled` (jsdom does not lay out grids).
- **Verification:** The default's row now stays on one line in an unlabeled list, but the new rule
  breaks the filter. `.popover .choices-list:not(.labeled) .option.choice-default` (`styles.ts:631`)
  has five class-level selectors and sets `display: flex`. That outranks
  `.popover .option.choice[hidden] { display: none; }`, which has four. Typing in the field hides the
  rows that do not match (`#filterChoices`), so in an unlabeled list the default's row stays in sight
  after it is filtered out, and its rule does not. Under jsdom, typing `9` in `card.value.count`
  (choices `5` and `99`, default `0`) sets `hidden` on the default's row. In headless Chromium, a page
  with the same four rules computes `display: flex` for that hidden row, and `none` for a hidden
  choice. The test checks only the `labeled` class, so it cannot see this. Still to do: add
  `:not([hidden])` to the new rule, and add a test that a filtered-out default's row is hidden.
  **Confidence: high.**
- **Fix:** (second round) The rule is `.choices-list:not(.labeled) .option.choice-default:not([hidden])`, so filtering hides the default's row again.
- **Verification:** (second round) The `[hidden]` half is fixed: in headless Chromium, with the
  viewer's whole built stylesheet and rows marked up as `choiceRows` draws them, a hidden default's
  row computes `display: none` in both an unlabeled and a labeled list. But the same check shows the
  original finding was never fixed, which the first verification missed. `.popover .option`
  (`styles.ts:493-496`) sets `display: flex; flex-direction: column`, and the new rule changes only
  `display` and `gap`. So in an unlabeled list the default's row is still a column: `Default` sits at
  top 20 and `0` at top 55, 16px further down because of the `gap`. In a labeled list both spans sit
  at the same top (20). No test can see this under jsdom, and the second round added no test that a
  filtered-out default's row is hidden. Still to do: give the rule `flex-direction: row` and
  `align-items: baseline`, or lay the row out as `grid-template-columns: max-content minmax(0, 1fr)`.
  Check it in a browser or a screenshot of the playground's unlabeled choices with a default.
  **Confidence: high.**
- **Fix:** (third round) The rule also sets `flex-direction: row; align-items: baseline`, overriding the base `.popover .option` column. Measured in headless Chromium with the built stylesheet: in an unlabeled list, `Default` and `0` share a top (15px) with a 16px gap, as in a labeled list.
- **Verification:** (third round) Confirmed in headless Chromium, with the viewer's rebuilt stylesheet
  and rows marked up as `choiceRows` draws them. In an unlabeled list, the default's row computes
  `display: flex` and `flex-direction: row`. `Default` and `0` share a top, and the value starts 16px
  past the label, as in a labeled list. A filtered-out (`hidden`) default's row still computes
  `display: none` in both list kinds, and so does a hidden choice. jsdom cannot test the layout, so
  this rests on the browser check. Viewer tests (161) pass. **Confidence: high.**

### ✅ Verified - RF24 The choices field's ARIA does not form a combobox
- **Severity:** Low
- **Evidence:** A field with choices keeps `role="textbox"` (`index.ts:1522`). It gains
  `aria-haspopup`, `aria-controls`, `aria-activedescendant` and `aria-expanded` (`index.ts:2217-2220`,
  `2265`). ARIA does not support `aria-expanded` on `textbox`, so a screen reader is not told that the
  list opened or closed. That is how the list is reached by keyboard. While the list is closed, the
  popover is emptied, so `aria-controls` names an id that is not in the document. The rule between the
  default's row and the choices is a `role="separator"` inside the `listbox` (`index.ts:2208-2213`),
  and a listbox may own only `option` and `group`.
- **Recommendation:** Give a field with choices `role="combobox"`, with `aria-autocomplete="list"`,
  and keep `aria-expanded` in sync. Set `aria-controls` only while the list is in the document. Make
  the rule presentational (`role="none"`, `aria-hidden`), or put the choices in a `role="group"` after
  the default's option. **Confidence: medium-high.**
- **Fix:** A field with choices gets `role="combobox"` and `aria-autocomplete="list"`; `aria-controls` is set only while the list is open, and removed on close. The rule is `role="none"` with `aria-hidden`. Asserted in the refused-choice test.
- **Verification:** Confirmed. A field with choices takes `role="combobox"` and
  `aria-autocomplete="list"`, and `aria-expanded` follows the list. `aria-controls` is set only while
  the list is drawn and is removed on close. The test asserts this before opening, while open and
  after a refused pick. The rule is `role="none"` and `aria-hidden`, so the listbox owns only options.
  A color's input keeps a fixed `aria-controls`, which is right, since its list is always drawn in its
  popover. **Confidence: medium-high** (no screen reader was run).

### ✅ Verified - RF25 A stepper drag with the field open does not commit on release, as the spec says it does
- **Severity:** Low
- **Evidence:** The viewer spec says "Pressing a stepper and dragging up or down SHALL ... commit it
  once on release", and the design and the viewer README say the same. With the field open,
  `#onToolsPointerUp` puts the dragged value in the field and selects it (`index.ts:2529-2532`).
  Nothing is committed until Enter or blur. Only the click is specified as "into the field while it
  is open". Keeping the field's draft is a reasonable behavior, but the spec, the README and the code
  disagree, and no test covers a drag with the field open.
- **Recommendation:** Decide which is wanted. Either commit on release with the field open too, or
  change the spec scenario and the README to say a drag with the field open fills the field, as a
  click does. Add a test either way. **Confidence: high.**
- **Fix:** Kept the behavior and changed the docs: a drag with the field open fills the field, as a click does. Viewer spec, design and README now say so, with a new scenario "A stepper dragged". Test "fills an open field from a drag, which commits".
- **Verification:** Confirmed. The spec, the design and the README now say a drag with the field
  open fills the field, which matches the code. A scenario covers the drag with the field closed. The
  test drags up on `count`'s open field to `5`: nothing fires, and Enter commits `5`.
  **Confidence: high.**

### ✅ Verified - RF26 On a one-line element, the steppers and the chevron cover the start of the next property
- **Severity:** Low
- **Evidence:** The design draws the steppers "over the gap just past a number, as the case chevron is
  drawn". The case chevron is 8px, inside the one-space gap (`styles.ts:397-410`). The tools are a
  4px margin plus a 14px stepper column, plus 2px and a 12px chevron while a field with choices is open
  (`styles.ts:654-689`). On an opaque field background, that is 18 to 32px over a 13px monospace
  line, whose gap is one 7.8px space. On a line such as `padding=8 px gap=4`, they hide `ga` or the
  whole `gap=`. The tools show while the field is open, so the reader typing loses sight of the
  neighbouring property. Sheet rows have only the row menu past the value, so they are not affected.
- **Recommendation:** Accept this and say so in the design. Or, on one-line elements, draw the tools
  above or below the line, or narrower (a 10px column, with the chevron folded into it). Check it in
  the "Visual test cases" screenshots at 375 pixels wide. **Confidence: medium** (from the CSS; not
  seen in a browser).
- **Fix:** Accepted as a trade-off and written into the design's Steppers entry: they show only on the selected row and while the field is open, and drawing them above or below would cover a neighboring line instead.
- **Verification:** Confirmed. The design's Steppers entry now states the trade-off, as the
  recommendation's first option allows. **Confidence: high.**

### ✅ Verified - RF27 Small leftovers in the docs after the respec
- **Severity:** Low
- **Evidence:**
  - `sites/playground/src/language/worker.ts:2` now reads "Hover, completions, diagnostics, document
    symbols and the source tree". The service still answers `sourceEdit` (`worker.ts:44-45`); removing
    named values also dropped the "source edits" from the doc.
  - `packages/language-core/src/answer.ts:46-48` merged two doc comments into one ("The node a
    source-edit query edits, and the edit."), on `key`, and left `edit` undocumented.
  - Task 6.3 says the language packages' READMEs document the hints. `packages/language-protocol/README.md`
    describes neither `SourcePropertyVisualizer.editor` nor the new `SourceChoice`. That includes the
    point that a choice's `value` is a `setValue` value a client passes on as it is.
- **Recommendation:** Restore "and source edits" in the worker's doc, and give `key` and `edit` their
  own comments. Add a short paragraph and example for the visualizer `editor` value, with `choices`,
  under `sourceTree` in the protocol README. **Confidence: high.**
- **Fix:** Worker doc names source edits again; `answer.ts` documents `key` and `edit` separately; the protocol README describes the `visualizers` option, `editor` and its hints, and `choices` with an example noting a choice's value is passed on as a `setValue` value.
- **Verification:** Confirmed. The worker's doc names source edits, and `key` and `edit` have their
  own comments. The protocol README's new paragraph and example match `SourceTreeVisualizersOption`,
  `SourceEditorHints` and `SourceChoice` field for field. **Confidence: high.**

### ✅ Verified - RF28 Rust tests miss the choice paths most likely to regress
- **Severity:** Low
- **Evidence:** `source_tree_tests.rs:1925-2010` covers an alias's choices, a property's replacing them,
  and one mismatched string choice beside a fitting one. Nothing tests the following:
  - that a property whose choices all mismatch keeps its type's choices. This is the
    `.filter(|choices| !fitting_choices(choices, kind).is_empty())` in `source_tree.rs:2139-2142`,
    RF9's rule applied to choices.
  - choices reached through an alias chain (`alias_hints`, `source_tree.rs:2366`), or merged along
    inheritance (`merge_child`, `visualizers.rs:276-279`).
  - how signed, real and hex choice literals are spelled into `SourceEditValue::Number` (see RF22).
    `hints_take_signed_and_real_numbers` only checks that analysis is clean.
- **Recommendation:** Add source-tree tests for each, one assertion apiece. **Confidence: high.**
- **Fix:** New source-tree tests over a `CHOICES` library: `choices_keep_their_literals_and_a_whole_number_takes_no_fraction` (spellings), `choices_that_all_mismatch_leave_the_types_choices`, and `choices_reach_through_an_alias_chain_and_inheritance` (`Gap = Spacing`, and a base's property visualizer inherited).
- **Verification:** Confirmed. The three tests pass, and each catches its regression. Removing the
  all-mismatch filter leaves `tight` with no choices instead of `4, 8`. `gap:Gap` checks the alias
  chain, `pad` the inheritance through `merge_child`, and `big` the spellings.
  `cargo test -q -p nx-language-service -p nx-api -p nx-types` passes, and clippy is clean on the three
  crates. **Confidence: high.**

## New Findings Discovered During 2026-10-22 23:36 Verification

Scope of this pass: `git show a12f726`, the fixes for RF18 to RF28. Checks run: `packages/viewer`
`pnpm test` (160 passed); `cargo test -q -p nx-language-service -p nx-api -p nx-types` (all passed);
clippy on those three crates (clean); `openspec validate add-visual-property-editing --strict`
(valid). Suspected bugs were confirmed with throwaway scripts, kept outside the repository, that drive
the built viewer under jsdom and the wasm language snapshot. One CSS cascade was checked in headless
Chromium.

### ✅ Verified - RF29 Within one row, the steppers stay on the first number hovered and never move to another
- **Severity:** Low
- **Evidence:** RF18's fix makes `#hoverTools` return early whenever the hovered element is in the
  shown value's row (`packages/viewer/src/index.ts:2459-2463`). It returns before it looks at which
  value is hovered. A row can hold more than one number: the items of a braced list (`items={ 1 2 }`)
  share one `.line-property`. Confirmed under jsdom on the `editingEdges` fixture with `box.value`
  selected: hovering `items[0]` shows its tools, and hovering `items[1]` then leaves them on
  `items[0]` (`data-tools-for` does not change). Before `a12f726`, the tools moved to `items[1]`. A
  multi-line element's list row behaves the same. To reach `items[1]`'s steppers, the pointer must
  leave the row and come back in on `items[1]` without crossing `items[0]`. Nothing is edited by
  mistake, because the tools stay drawn beside the number they step. But the spec's "while the pointer
  is over its value on the selected row" no longer holds for the second item.
- **Recommendation:** Check the hovered value first, and apply the row rule only when the hovered
  element is not inside another `.editable-value.edit-number`. This fits with the fix RF18 still
  needs. Add a test that hovers two items of one list. **Confidence: high.**
- **Fix:** Fixed with RF18's second round: only the row or line element itself keeps the tools, so hovering another number in the row moves them to it. Covered by the same test, over `items={ 1 2 }` in `editingEdges`.
- **Verification:** Confirmed. `#onTheWay` keeps the tools only for the row or line element
  itself, not for another value in it, so `#hoverTools` goes on to the hovered value. Under jsdom on
  `editingEdges`, hovering `items[0]` and then `items[1]` moves the tools to `items[1]`. Then
  `items[1]`'s unit and the tools keep them. The new test asserts the move. **Confidence: high.**

### ✅ Verified - RF30 `is_integral` carries `check_choices`' old doc comment
- **Severity:** Low
- **Evidence:** The fix inserted `is_integral` (`crates/nx-types/src/infer.rs:1810-1814`) between
  `check_choices`' old doc comment and its function. `check_choices` got a new comment, so the old
  three lines are left over, and `is_integral`'s rustdoc now reads "Checks each `ValueChoice` among
  `content` against a value of `kind`, ... and nothing else takes choices. Whether a value of `ty` is
  a whole number: ...".
- **Recommendation:** Delete the three stale lines above `is_integral`'s own comment.
  **Confidence: high.**
- **Fix:** The three stray lines are gone; their content ("a number takes number literals, a string string literals, and nothing else takes choices") is folded into `check_choices`' own doc comment.
- **Verification:** Confirmed. `is_integral` has only its own comment, and `check_choices`' comment
  now holds the folded-in sentence. `cargo test -q -p nx-types` passes, and clippy is clean.
  **Confidence: high.**

## New Findings Discovered During 2026-10-23 07:56 Review

Scope of this pass: the whole change, `git diff 806223f...HEAD` at `a8598c4`, against the current
proposal, design, specs and tasks. It focuses on what landed since `7fc84d0`: larger steppers and their
6 px slop and 4 px drag step, the unset-properties wording, the removal of `unit`, room on a line and
the 300 ms rest, hover tools anywhere, `#rehover`, `fieldText`/`withEndOfLines`, Up and Down from the
current choice, the case chevron drawn as `.value-tools`, and a click on a value leaving the selection.
Checks run: `packages/viewer` `pnpm test` (167 passed); `cargo test -p nx-types -p nx-api -p
nx-language-service`, `cargo clippy --workspace --all-targets` and `cargo fmt --all --check` (all
clean); `sites/playground` `pnpm test` (24 examples match); `openspec validate
add-visual-property-editing --strict` (valid). Behavior was checked in headless Chromium against the
playground's "Visual test cases" with throwaway Playwright scripts kept outside the repository. These
work as specified: repeated stepper clicks on the unselected `strip` `titleSize` step 20 to 24 and fire
one `setValue` each, including a click 50 ms after the last one, and the selection stays empty.
`#rehover` brings the tools back after each commit. A trailing Shift+Enter commits `"Fresh\n"` without
the browser's extra line break, and a line break in the middle commits as typed. The case chevron opens
and closes `align`'s popover without selecting anything. `unit` is gone from the library, analysis,
the source tree, the protocol, the viewer and the docs.

### ✅ Verified - RF31 A hover tool taking room on a line that just fits wraps the line and moves the whole reading below it
- **Severity:** Medium
- **Evidence:** `#showTools` puts the tools into the line as static content when more of the line
  follows (`packages/viewer/src/index.ts:2466-2475`, `.value-tools.in-line { position: static }` in
  `styles.ts:709`). A `.line-element.alone` wraps (`white-space: pre-wrap`, `styles.ts:185-196`). What
  fits on a line is measured only when the reading is drawn (`index.ts:905-922`, `#wrapped`
  `index.ts:996`). So a line that fits with nothing to spare overflows once the tools join it. In
  Chromium, with `<nx-viewer>` 462 px wide, the menu's `<Drink code="LAT" … size=small iced=false
  shots=2 />` is one 18 px line. After the pointer rests 300 ms on `size`, the line is 36 px tall,
  `shots=2` has wrapped, and every item below has moved down 18 px. They move back when the pointer
  leaves. The spec's "Room on a line" lets the tools move what follows *along the line*. The design
  adds the rest timer "so a line does not jump as the pointer passes over it". Here the whole reading
  jumps on hover, with no editor open. Opening a field there has the same effect (`index.ts:1464`).
- **Recommendation:** Before taking room, check whether the line would wrap: compare the line's
  remaining width with the tools' width, or insert them and compare the line's client rects. If it
  would wrap, draw the tools over the gap as in a sheet row. Another option is to reserve the tools'
  width when measuring what fits. Add a test that narrows the reading until a one-line element just
  fits, rests on a value with tools, and checks that the element keeps one line and the next unit's
  top stays put.
- **Fix:** `#showTools` measures the line (`.line-element`, header, tag line or sheet value) before and after the tools join it; if it grew, they lose `in-line` and cover the gap, as in a sheet row, so nothing below moves. Test "covers the gap rather than wrap a line that only just fits"; your `wrap3.mjs` at 462 px now keeps the Latte line at 18 px and the next item in place. Spec, design and README say so.
- **Verification:** Verified at `26df6a8`. `#showTools` takes the tools out, measures the line's height, puts them back and drops `in-line` if the line grew (`index.ts:2474-2485`). With `<nx-viewer>` 462 px wide in headless Chromium, resting on the LAT Drink's `size` now draws the chevron over the gap: the line stays 18 px and the next item stays at y = 455. At 502 px, where the room fits, the tools still take room in the line and the line grows from 391 to 407 px wide, still on one line. The jsdom test mocks the height, which is fine for a layout check. The spec and design describe the fallback.

### ✅ Verified - RF32 Closing a case's popover from its chevron removes the chevron under the pointer, and the next click selects the element
- **Severity:** Medium
- **Evidence:** With a case's popover open, a press on its chevron reaches `#onDocumentPointerDown`
  first, in the capture phase (`index.ts:403-425`). The press is outside the popover, so the handler
  cancels the session, and `#endSession` → `#hideTools` (`index.ts:1817`) takes the tools out and
  clears `toolsFor`. `#onToolsPointerDown` then finds no key and does nothing (`index.ts:2599-2603`).
  The branch meant to close the popover (`index.ts:2606-2613`) never runs. The popover closes, but
  unlike a stepper click (`#rehover`), nothing brings the tools back. On a line they had taken room,
  so the line gives the room back and shifts left under the pointer. In Chromium on `strip`'s
  `align=start`, three things happen in turn. Rest, then click the chevron: the popover opens.
  Click it again: the popover closes and the tools are gone. Click a third time at the same spot,
  which is now `.line-gap`: `nx-select` fires for `strip.value`, and the `Show 6 unset properties`
  row appears under the line. That pushes the reading down, which is what task 5.9 set out to stop.
  The jsdom test (`editing.test.ts:1230-1247`) only checks that the popover hides, so it passes.
- **Recommendation:** In `#onDocumentPointerDown`, leave a press whose `composedPath()` includes
  `#tools` to `#onToolsPointerDown`, as it already does for the popover. After the chevron closes the
  popover, show the tools again for that key, as `#rehover` does after a stepper click. Extend the
  test: after closing, the chevron is still shown, and a second press opens the popover again.
- **Fix:** `#onDocumentPointerDown` leaves a press on `#tools` to `#onToolsPointerDown`, and closing a case's popover from its chevron shows the tools again for that key. The case-chevron test now closes, checks the chevron stays in-line, presses again to reopen, and that nothing is selected. In Chromium, three clicks on `strip`'s `align` chevron open, close and reopen with the selection empty.
- **Verification:** Verified at `26df6a8`. A press on `#tools` is now left out of the document handler's cancel (`index.ts:420-422`), and the chevron's own handler closes the popover and shows the tools again (`index.ts:2625-2633`). In Chromium on `strip`'s `align`, the first click opens the popover. The second closes it, and the chevron stays under the pointer. The third opens it again. No `nx-select` fires, and the unset-properties row never shows. The extended jsdom test covers the reopen and the empty selection.

### ✅ Verified - RF33 In an unset-properties row, the tools take room and push `default` aside, and under a one-line element they wait as if outside the selection
- **Severity:** Low
- **Evidence:** A dimmed row reads `value  default`. `#followedOnItsLine` counts the quiet
  `.more-default` sibling as more of the line (`index.ts:2487-2504`), so the tools get `in-line` and
  push `default` right. The design and spec instead say a sheet's row draws the tools over the gap,
  and nothing moves. A one-line element's list is drawn after the element (`element.after(block)`,
  `index.ts:2872`), outside its rendered node. So the `onSelected` test (`index.ts:2555-2556`) fails,
  and the tools wait 300 ms, although these rows are drawn inside the selection's highlight and exist
  only on the selected element. In Chromium, with `strip` selected and its list open, hovering the
  dimmed `gap` shows nothing at 50 ms. At 450 ms it shows `value-tools with-steppers in-line`, with
  `default` pushed to x = 251, past the tools at 221–243. Under a sheet (`banner`), the same row would
  take room at once.
- **Recommendation:** Stop `#followedOnItsLine` at `.more-value`, or skip `.more-default`, so a dimmed
  row's tools overlay as a sheet row's do. Also count the open `#moreBlock` as part of the selection
  in `#hoverTools`. Add a test on both.
- **Fix:** Partly as recommended. `#hoverTools` counts a value inside the open `#moreBlock` as on the selection, so the tools show at once (test "shows an unset property's tools at once, under a one-line element selected"). I kept the tools taking room before the quiet `default`, rather than covering it: drawing them over the gap would hide the word, and moving it within its own row moves nothing else. The spec's "Room on a line", the design and the README now say an unset property's value moves its `default` aside.
- **Verification:** Verified at `26df6a8`. The fixer's variant is acceptable. Moving the quiet `default` aside within its own row moves nothing else, and covering the word would hide it. The behavior is now specified in the viewer delta's "Room on a line", the design and the README, so code and spec agree. The 300 ms wait is gone: a value inside the open `#moreBlock` counts as on the selection (`index.ts:2571-2574`). In Chromium, with `strip` selected and its list open, `gap`'s tools are `in-line` 50 ms after the pointer arrives. A new jsdom test covers this.

### ✅ Verified - RF34 The specs and docs still describe the earlier behavior in places
- **Severity:** Low
- **Evidence:**
  - The base `viewer` requirement "A selection is a unit of the reading"
    (`openspec/specs/viewer/spec.md:214-231`) says a click SHALL select the unit it falls in. Its
    scenario says clicking a word of a one-line element selects the element. The delta modifies only
    "Values edit in place", which now says such a click leaves the selection. With `editable` set,
    the two requirements contradict each other. The README's "Selection" section repeats the
    unqualified rule (`packages/viewer/README.md:191`).
  - `proposal.md:35` says steppers show "while its field is open or on a selected row". The design's
    Risks say they are "drawn only where they help (an open field, the selected row)"
    (`design.md:178-179`). Both are stale since hover tools show anywhere. The proposal's
    Capabilities line for `viewer` does not mention the modified "Values edit in place" requirement.
  - `tasks.md` 5.7 says that where more of the line follows there are "no hover tools". The code and
    spec now show them there after the rest.
  - "Room on a line" in the design and spec covers one-line elements and group headers, and says that
    in a sheet's row the tools "stay drawn over the gap". But `#followedOnItsLine` also takes room
    (and waits) for a braced list's earlier items in a sheet row, as `ratings={ 4 2 5 }`, since it
    checks the value's own siblings before stopping at `.sheet-value` (`index.ts:2488-2501`). That
    is reasonable, since covering the next item would be worse, but it is unspecified.
  - The CHANGELOG's `<nx-viewer>` entry (`src/vscode/CHANGELOG.md:32-36`) mentions neither the hover
    tools nor that a click on a value no longer selects its unit. Phase 1's entry (line 14) is in the
    same Unreleased section.
- **Recommendation:** Add a MODIFIED "A selection is a unit of the reading" to the viewer delta,
  carving out a click on an editable value when `editable` is set, and qualify the README's
  Selection paragraph. Bring the proposal, the design's Risks and task 5.7 up to date. Either say in
  "Room on a line" that items of a braced list take room too, or stop there. Add a clause to the
  changelog entry.
- **Fix:** Added a MODIFIED "A selection is a unit of the reading" to the viewer delta (a click on an editable value with `editable` set opens its editor and leaves the selection; the scenario names a word that is not an editable value), and qualified the README's Selection section. Proposal (steppers and the Capabilities line), the design's Risks and task 5.7 are updated. "Room on a line" now covers earlier items of a braced list, an unset property's value, and the wrap fallback. The CHANGELOG's `<nx-viewer>` entry mentions the hover tools and that a click on a value no longer selects.
- **Verification:** Verified at `26df6a8`. The viewer delta has a MODIFIED "A selection is a unit of the reading". It matches the base requirement word for word, except that it carves out a click on an editable value with `editable` set and narrows the "Selecting a line" scenario to a word that is not an editable value, and it keeps all five scenarios. The README's Selection section, the proposal's stepper wording and Capabilities line, the design's Risks, task 5.7, the "Room on a line" text (braced-list items, unset values, the wrap fallback) and the CHANGELOG entry are all updated. `openspec validate --strict` passes.

### ✅ Verified - RF35 Small leftovers in the viewer's code after the latest commits
- **Severity:** Low
- **Evidence:**
  - `TOOLS_REST_MS` was inserted between `DRAWN_UNITS` and its doc comment, which leaves two doc
    comments stacked over `TOOLS_REST_MS` and none on `DRAWN_UNITS` (`index.ts:173-176`).
  - `const value = target.kind === "number" ? raw.trim() : raw.trim();` has the same expression on both
    branches. It is left over from removing `numberEntry` (`index.ts:2185`).
  - `#showTools`'s doc says the chevron shows only "while its field is open with choices"
    (`index.ts:2443-2446`). It also shows past a case and past a hovered value with choices.
  - `#openNext`'s doc still says it opens the next value "selecting its unit" (`index.ts:1875-1878`).
  - `CHOICES_TITLE` has no doc comment, unlike the constants around it (`index.ts:141`).
- **Recommendation:** Move `TOOLS_REST_MS` below `DRAWN_UNITS` with its own comment, collapse the
  ternary to `raw.trim()`, and update the two doc comments.
- **Fix:** `TOOLS_REST_MS` moved below `DRAWN_UNITS` with its own comment; the ternary is `raw.trim()`; `#showTools` and `#openNext` docs updated; `CHOICES_TITLE` has a doc comment.
- **Verification:** Verified at `26df6a8`. `DRAWN_UNITS` has its own doc comment again, and so does `TOOLS_REST_MS` below it (`index.ts:175-179`). The ternary is `raw.trim()`. The `#showTools` and `#openNext` doc comments describe the current behavior, and `CHOICES_TITLE` has a doc comment.

## Follow-ups From the Questions (2026-10-23)

Bret asked for the two open questions worth acting on to be done now and verified by an independent
reviewer: the merged-range warning and a derived visualizer over its base.

### 🟡 Fixed - RF36 A derived visualizer's mismatched hint or choices hide its base's
- **Severity:** Low
- **Evidence:** `merge_child` (`crates/nx-language-service/src/visualizers.rs`) replaced a base's hint
  and choices with the derived visualizer's before they were checked against the property's type. A
  derived `<ValueChoice value="x" />` or `editor=dropdown` on an `int` left the property with no
  choices or no kind, though the base's fit. RF9's rule covered only a property over its alias.
- **Recommendation:** Keep every layer and let the reader take the first that fits, for attributes
  and choices alike.
- **Fix:** `ResolvedVisualizer` and `ResolvedChild` keep every hint and every set of choices, latest
  first (`hints: Vec<(String, String)>`, `choices: Vec<Vec<VisualizerChoice>>`). `Hints` in
  `source_tree.rs` holds the property's own layers followed by the alias's; `editor_hints` takes the
  first hint of each name that fits and the first set of choices with any that fit. Test
  `a_derived_visualizers_mismatched_hints_leave_its_bases_in_force` (fails before the fix). Spec:
  `visualizer-declarations` "A property's editor hints come from its visualizer or its type" (new
  scenario) and `source-tree`; design, website doc and changelog say so. Task 5.10.

### 🟡 Fixed - RF37 A range that only merging empties gives no warning
- **Severity:** Low
- **Evidence:** `editor-hint-range` was checked per visualizer element in analysis. A
  `PropertyVisualizer`'s `min=100` over its alias's `max=64` was left out of the tree with no warning
  (the merged-range question above).
- **Recommendation:** Check the merged hints and warn where the author can fix it.
- **Fix:** `merged_range_warnings` (`source_tree.rs`) walks each `ValueVisualizer` alias and each
  `ElementVisualizer` child a document declares, with the hints it merges with (its alias's, its
  base's, an earlier declaration's). Where the visualizer's own `min` and `max` leave a range but the
  resolved ones do not, it warns at the written `min` or `max` that is in force. An element's own empty
  range is left to analysis, so it warns once. `NxLanguageService::diagnostic_report` adds these as
  `editor-hint-range` warnings (`merged_hint_diagnostics`, `lib.rs`). Tests
  `a_range_only_merging_empties_warns_where_it_is_written`,
  `a_derived_range_over_its_bases_warns_on_the_derived_one` and
  `an_elements_own_empty_range_warns_once`. Spec: `visualizer-declarations` (new scenario "A range
  that merging empties"); design, website doc and changelog. Task 5.10.

## Questions
- RF7 and RF8 were product calls. The fix took the reviewer's first option for each: a conditional write
  counts as written, and case elements are supported. Should a property written only under a condition count as written? Should
  union-case elements list their unwritten properties, or should the spec exclude them?
  - **Answer (2026-10-23):** Kept as implemented; Bret's go-ahead on the change accepted both.
- `namedValues` orders imports by module identity, not by import order (`named_values.rs` sort key). Is
  that what "then each import's" means?
- `namedValues` leaves out only the `let` the element is in. A pair of lets can still be made to refer to
  each other (`a.tint={b.tint}` and `b.tint={a.tint}`). Is that acceptable for now?
  - **Answer (2026-10-23):** Both `namedValues` questions are moot: named values were removed (task 5.2).
- A range that only merging makes empty (a `PropertyVisualizer`'s `min=100` over an alias's `max=64`)
  is now left out of the tree with no warning, because `editor-hint-range` only checks the hints one
  visualizer element writes. Should analysis also check the merged hints?
  - **Answer (2026-10-23):** Yes, see RF37.
- Since RF3, an optional property is also offered the optional members of every record `let` in
  scope, including members the record never writes. For `tint?:string`, `<Drink code="C" />` as
  `only` offers `only.tint`, which is always absent. That is type-correct but of little use. Should
  unwritten members be left out?
  - **Answer (fixer):** Yes. An optional member the record neither writes nor defaults is now left out,
    with the source-edits spec saying so and `an_optional_property_takes_optional_members_and_values`
    covering it (`plain.accent` is offered, `plain.muted` is not).
  - **Verification:** The rule is right for a record literal, and `plain.muted` is left out as the test
    says. But the check is syntactic, so it also leaves out members that are written. See RF14.
- **Fixer's note on the merged-range question:** that question is left open as a design question. The
  tree leaves the merged range out, which is safe, and a warning on merged hints can come later.
- (2026-10-22 23:17) Inheritance merges a derived `PropertyVisualizer`'s choices over its base's
  before they are checked against the property's type (`merge_child`, `visualizers.rs:276-279`). A
  derived `<ValueChoice value="x" />` on an `int` therefore leaves the property with no choices,
  though the base's `4` fits. Attributes merge the same way, so this matches today's rule. RF9's
  "a mismatched hint does not hide a valid one" covers only a property over its type. Should it also
  cover a derived visualizer over its base, for choices and attributes alike?
  - **Answer (2026-10-23):** Yes, see RF36.

## Summary
- The change is largely complete and well tested at the unit level, and the Rust, TypeScript and playground
  layers are wired consistently, with ABI 10 in both places.
- The main problem is RF1. The viewer's new reference editing reaches parameters and loop variables, so a
  click and a typed value can rewrite a component's or a loop's binding into a literal.
- RF2 and RF3 are smaller correctness gaps in hint ranges and in named values for optional properties.
- The rest are edge cases, a convention nit and missing tests.
- Verification (2026-10-22): RF1-RF7 and RF9-RF12 are verified. RF8 is reopened, because
  `namedValues` and the declaration table's property types still leave out a union case's properties.
  RF13 is new: values under a property-list condition take their property's hints but not its format
  or hover card.
- Second verification (2026-10-22 08:33): RF8 and RF13 are verified. The answer to the
  "unwritten members" question works for record literals, but it also leaves out the written optional
  members of a `let` whose value is a reference or a call. That is RF14, which is open.
- Third verification (2026-10-22 08:50): RF14 is verified, along with the lowering change that keeps a
  declared record's conditional properties. Two Low findings are new: executable codegen now refuses
  such a record, which the changelog does not say and files under a released version (RF15), and a
  condition repeats a direct property's type error once per branch path (RF16).
- Fourth verification (2026-10-22 09:07): RF15 and RF16 are verified. RF17 is new and Low: the
  deduplication that fixed RF16 is quadratic in diagnostics that share a message, which takes seconds at
  several thousand of them.
- Fifth verification (2026-10-22 09:18): RF17 is verified, and nothing new was found. The deduplication
  is now linear, and a file of many same-message errors checks as fast as one without them. In this
  environment the wasm package's `recursion-stack` test fails (the `forLoop` shape traps under the
  0.5 MB worker stack), but it fails the same way built from the change's base, `806223f`, so it is not
  this change's.
- Review pass after the respec (2026-10-22 23:17): the slider, `stops`, named values and the
  `reference` value are fully removed, and the Rust side of choices matches the specs. Eleven findings
  are new. Three are Medium: the hover steppers cannot be reached across a unit (RF18), a tree that
  arrives during a stepper drag drops it and leaves the reading's `stepping` cursor stuck (RF19), and
  a picked choice skips the field's checks while analysis lets an `int` take a fractional choice
  (RF21). The rest are Low: RF20 and RF22 to RF28.
- Verification of the respec fixes (2026-10-22 23:36): RF19 to RF22 and RF24 to RF28 are verified.
  Two are reopened. RF18: outside a property row, as for a `let`'s number, the unit still hides the
  steppers. RF23: the new rule for the default's row outranks `[hidden]`, so a filtered-out default's
  row stays in sight. Two Low findings are new: RF29, where the steppers stay on the first number
  hovered in a row, such as a braced list's first item (a regression from the RF18 fix), and RF30, a
  stale doc comment on `is_integral`. Viewer tests (160), the three crates' Rust tests, clippy and
  `openspec validate --strict` pass.
- Verification of the second round (2026-10-22 23:47, commit `40335dc`): RF18, RF29 and RF30 are
  verified. RF23 is reopened. Filtering hides the default's row again, but headless Chromium shows
  that in an unlabeled list the row still stacks `Default` over its value. The base `.popover .option`
  rule sets `flex-direction: column`, and the first verification missed this. Nothing new was found.
  Viewer tests (161), `nx-types` tests and clippy pass.
- Verification of the third round (2026-10-22 23:52, commit `58a101e`): RF23 is verified. In headless
  Chromium, the default's row in an unlabeled list is one line, and it is hidden when filtered out.
  Nothing new was found. Every finding from RF1 to RF30 is now verified.
- Review pass at `a8598c4` (2026-10-23 07:56): a fresh review of the whole change, focused on the
  2026-10-23 behavior changes (room on a line, the rest timer, hover tools anywhere, `#rehover`, the case
  chevron, `unit` removed, a click on a value leaving the selection). Two Medium findings are new. A
  hover tool that takes room on a line that just fits wraps it and moves the reading below (RF31). Closing
  a case's popover from its chevron removes the chevron, so the next click selects the element and
  shows its unset-properties row (RF32). Three are Low: RF33 to RF35. Viewer tests (167), the three
  crates' Rust tests, clippy, fmt, playground tests and `openspec validate --strict` pass.
- Verification of the 2026-10-23 review's fixes (2026-10-23, commit `26df6a8`): RF31 to RF35 are
  verified, and nothing new was found. RF33 was fixed partly differently from the recommendation: the tools
  still take room before the quiet `default`. That is now specified, and it is acceptable. Checked in headless
  Chromium against the rebuilt viewer. Viewer tests (169) and `openspec validate --strict` pass. Every
  finding from RF1 to RF35 is now verified.
