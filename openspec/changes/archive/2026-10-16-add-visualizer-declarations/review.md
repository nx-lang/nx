# Review: add-visualizer-declarations

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/visualizer-declarations, specs/visualizer-library,
specs/component-props-type, specs/record-type-parameters, specs/property-references, specs/update-records,
specs/component-syntax, specs/source-tree, specs/viewer, specs/playground, specs/cli-code-generation,
specs/editor-syntax-highlighting  
**Reviewed code:** every commit in `origin/main..HEAD`: `crates/nx-syntax` (grammar, ast), `crates/nx-hir`
(`lower.rs`, `records.rs`, `visualizers.rs`, `lib.rs`, `components.rs`), `crates/nx-types` (`infer.rs`, `check.rs`,
tests), `crates/nx-api` (`program_visualizers.rs`, `artifacts.rs`, `std/visualizer/visualizer.nx`, tests),
`crates/nx-codegen/src/builder.rs`, `crates/nx-cli/src/typegen`, `crates/nx-language-service` (`visualizers.rs`,
`source_tree.rs`, tests), wasm and Node bindings, `packages/language-protocol`, `language-core`,
`language-http`, `packages/viewer` (`render.ts`, `index.ts`, `hover.ts`, `styles.ts`, tests),
`sites/playground`, `src/vscode` grammar, website docs.  
**Checks run:** `cargo test` for nx-syntax, nx-hir, nx-types, nx-api, nx-codegen, nx-language-service and
nx-cli (all pass); `packages/viewer` `pnpm test` (93 pass); `src/vscode` grammar tests (287 pass); probes
through the wasm SDK's language snapshot and `<nx-viewer>` under jsdom, and the built `nxlang typegen`.

## Findings

### ✅ Verified - RF1 Visualizer declarations that fail type checking still apply, so type-invalid values reach the source tree
- **Severity:** Medium (confidence: high, reproduced)
- **Evidence:** `program_visualizers` (`crates/nx-api/src/program_visualizers.rs:130`) keeps every visualizer declaration
  whose target resolves and whose tag *spelling* matches the record, and `read_visualizer_value` /
  `attribute_value` (`:308`, `:370`) read the attributes syntactically, so a declaration with type errors
  still contributes whatever it wrote. Probes:
  - A host module with `visualizer Choice = <ElementVisualizer name="Host" title=lable><PropertyVisualizer of=valu … />`
    (two `unresolved-contextual-name` errors in `host.nx`) still overrides the library's `name="Lib"`:
    the resolved visualizer reads `name: "Host"`. The spec says a host visualizer module that does not analyze
    cleanly "SHALL contribute the visualizer declarations that do" (visualizer-declarations, "gathered from the
    program and from host modules"), i.e. not the ones that don't.
  - `visualizer Choice = <ElementVisualizer layout="row bogus" title="label"><PropertyVisualizer of="value" format="image" /></ElementVisualizer>`
    reports four `record-field-type-mismatch` errors, yet the answer carries
    `{"title":"label","layout":"row bogus","properties":[{"name":"value","format":"image",…}]}`. The
    `layout`, `place` and `format` strings are passed through unvalidated
    (`crates/nx-language-service/src/source_tree.rs:2001`, `:2020`), violating the protocol's
    `SourceVisualizerLayout` / `SourceValueFormat` / `"body" | "details"` unions; the viewer then treats
    `"row bogus"` as a non-card layout and folds every row into the details strip.
- **Recommendation:** Only gather visualizer declarations that type-checked cleanly. For example, have
  `infer_visualizer` record a per-declaration "clean" flag (no error diagnostic within `visualizer.span`) on the
  module artifact, or filter in `program_visualizers` by the module's error diagnostics whose primary span lies
  inside the visualizer declaration. Independently, read case-typed attributes (`layout`, `place`, `format`,
  `of`, `title`…) only from `Expr::ResolvedUnionCase` (not string literals), and validate `layout`/`place`/
  `format` against their known cases before emitting them. Add tests: a host module with a misspelled
  property does not override the library's name; a quoted `layout="row"` is not applied.
- **Fix:** `program_visualizers` (`crates/nx-api/src/program_visualizers.rs`) now leaves out, whole, any visualizer
  declaration with an error diagnostic whose primary label lies inside its span (`has_errors_within`;
  `duplicate-visualizer` is excluded so duplicate detection does not feed back on itself). As defense in depth,
  `ResolvedVisualizer::apply` and `merge_child` (`crates/nx-language-service/src/visualizers.rs`) drop any `layout`,
  `place` or `format` value outside the protocol's unions (`is_known_value`). Test
  `a_visualizer_declaration_with_an_error_does_not_apply` covers a host module with `title=lable` (the library's
  name stands) and a quoted `layout="inline"` / `format="image"` (neither applies).
- **Verification:** Re-ran the original probes against the rebuilt wasm. The host module with `title=lable` / `of=valu` no longer overrides the library: `name` stays "Lib". The quoted `layout="row bogus"` / `format="image"` declaration and the one with an unknown `whatever` attribute now contribute nothing. An error elsewhere in the module, outside the viewer's span, does not drop the viewer. `is_known_value` also filters `layout`, `place` and `format` in `apply` and `merge_child`. `a_visualizer_declaration_with_an_error_does_not_apply` passes. Applying the rule before duplicate detection opens a new gap, filed as RF8.

### ✅ Verified - RF2 An unknown format makes the viewer render a string literal as an empty span, losing its text
- **Severity:** Low (confidence: high, reproduced)
- **Evidence:** `formatted()` (`packages/viewer/src/render.ts:855`) switches over the known formats with no
  default branch, so any other `format` value leaves the marked span empty. With
  `<PropertyVisualizer of=value format="bogus" />` (see RF1) and `<Choice value="SECRET-TEXT" />`, the rendered
  literal is `<span … data-key="c.value.value.value"></span>` and the text appears nowhere in the reading.
  The key is still present, so the coverage test cannot catch it, but the reading is no longer lossless
  in substance ("A viewer SHALL never hide a node"). The same trust-the-tree pattern applies to `layout`
  (`render.ts:511`, `:549`: any value other than `"card"` folds everything).
- **Recommendation:** Add a `default:` branch that renders the value as plain text (as `text` does), and
  treat an unknown `layout` as `card`. This is defense in depth even after RF1, since the viewer accepts
  trees from any service version.
- **Fix:** `formatted()` (`packages/viewer/src/render.ts`) has a `default:` branch that shows the value as
  plain text, as `text` does, and `renderElement` reads any layout other than `row` or `inline` as `card`.
  Test "shows a value whose format or layout it does not know as plain text in a card" rewrites the survey
  tree's viewers to `layout: "bogus"` and `format: "bogus"`; it fails without the fix.
- **Verification:** Probed `<nx-viewer>` with a real tree whose viewer was rewritten to `layout: "bogus"` and `format: "bogus"`. The value `SECRET-TEXT` is rendered as plain text, and the element is a plain `card` with no `-layout` class. The new viewer test passes (94/94).

### ✅ Verified - RF3 The `inline` layout keeps the subtitle, badge and image in the line instead of the details strip
- **Severity:** Low (confidence: high, reproduced)
- **Evidence:** The spec (viewer, "Viewers shape a card") says an `inline` layout "SHALL show the element …
  as its heading and title, with every other row in its details strip"; the website page and the
  `ElementLayout.inline` doc comment say the same. `renderElement` moves every part in `PARTS`
  (`render.ts:498`, loop at `:553`–`:562`) into the header regardless of layout. Probe:
  `visualizer Tag = <ElementVisualizer layout=inline title=name subtitle=note badge=code />` renders an
  `inline-layout` span whose header holds the `title`, `subtitle` and `badge` parts.
- **Recommendation:** For `layout === "inline"`, only bind `title` into the header and let `subtitle`,
  `badge` and `image` attributes fall through to `detailRows` (they would then read as ordinary rows with
  their labels). Add a viewer test for an inline element with a subtitle and badge.
- **Fix:** For `layout === "inline"`, `renderElement` binds only `title` into the line; the subtitle, badge
  and image attributes fall through to rows in the details strip. The `inlineTag` fixture now sets
  `subtitle=note badge=code`, and the inline test asserts the line holds no subtitle or badge part and the
  details strip holds `note` and `code` rows; it fails without the fix.
- **Verification:** Re-ran the `Tag` probe with `layout=inline title=name subtitle=note badge=code`. The line now holds only the kind and the `title` part. `note` and `code` read as labelled rows in the details strip, and their keys are kept.

### ✅ Verified - RF4 `typegen` rejects any exported type that names a type called `Props` through a namespace
- **Severity:** Low (confidence: high, reproduced)
- **Evidence:** `props_record_reference` / `reject_props_record_references`
  (`crates/nx-cli/src/typegen/model.rs:1257`, `:1271`) decide by spelling (`is_props_record_name`: any
  name ending in `.Props`). A library `shared` exporting `type Props = { a:string }`, imported with
  `import "../shared" as shared` into a library that exports `type Saved = { p:shared.Props }`, fails with
  "Exported declaration 'Saved' refers to the props record 'shared.Props', and typegen does not generate
  props records yet", although `shared.Props` is an ordinary record. Nothing in the change reserves the
  type name `Props`.
- **Recommendation:** Resolve the referenced name through the module's prepared bindings and reject only
  when it reaches a record with `RecordKind::Props` (or its update record / property union), e.g. by
  checking `props_component()` / `update_target()` on the resolved item, as `is_props_record_item` already
  does for the module's own items. Add a CLI test with a namespaced `Props` type.
- **Fix:** `props_record_reference` (`crates/nx-cli/src/typegen/model.rs`) now resolves the record name
  through the module's prepared bindings and rejects it only when it reaches a record of kind
  `RecordKind::Props` (raw item or imported interface item); a name that does not resolve is still judged
  by its spelling. CLI test `test_cli_typegen_accepts_a_namespaced_type_named_props` uses the two-library
  layout from the probe; the existing `Card.Props` rejection test still passes.
- **Verification:** Re-ran the `nxlang typegen` probe. `shared.Props` imported through `import "../shared" as shared` now generates `p: shared_Props`. `ui.Card.Props` from another library is still rejected with the props-record message, so the resolution reaches imported `RecordKind::Props` interface items. The nx-cli typegen tests pass (216).

### ✅ Verified - RF5 A second `PropertyVisualizer` for the same property in one visualizer declaration is silently dropped
- **Severity:** Low (confidence: high, reproduced)
- **Evidence:** In `ResolvedVisualizer::apply` (`crates/nx-language-service/src/visualizers.rs:203`–`:220`), the
  second child with the same `of` starts from an empty `ResolvedChild` (the first was already moved out of
  `earlier`) and is then discarded by the `!self.properties.iter().any(…)` guard. With
  `<PropertyVisualizer of=label label="First" /><PropertyVisualizer of=label place=details />`, the resolved visualizer
  has `label: "First"` and `place: "body"`; the `place=details` is lost, and analysis reports nothing.
- **Recommendation:** Either merge the later child into the entry already in `self.properties`
  (consistent with "merge by `of`, attribute by attribute") or report a diagnostic for a repeated `of`
  within one visualizer declaration. The same applies to repeated `CaseVisualizer`s, which currently merge.
- **Fix:** In `ResolvedVisualizer::apply`, a later `PropertyVisualizer` whose `of` is already in the resolved
  properties merges into that entry, attribute by attribute, as `CaseVisualizer`s do. Test
  `two_property_visualizers_of_one_property_in_one_visualizer_merge` asserts `label: "First"` and `place: details`.
- **Verification:** Re-ran the probe. `<PropertyVisualizer of=label label="First" /><PropertyVisualizer of=label place=details />` now resolves to `label: "First", place: "details"`, and the entry is listed once at the first one's position. `two_property_visualizers_of_one_property_in_one_visualizer_merge` passes.

### ✅ Verified - RF6 Forgetting `import "@nx/visualizer"` gives a self-contradictory diagnostic
- **Severity:** Low (confidence: high, reproduced)
- **Evidence:** Without the import, `visualizer Choice = <ElementVisualizer title=label />` reports only
  `visualizer-record-mismatch`: "The visualizer of record 'Choice' must be a 'ElementVisualizer' from "@nx/visualizer", not
  'ElementVisualizer'" (`crates/nx-types/src/infer.rs:1558`). The tag falls back to an unknown host element whose
  type prints as the same name, so the message names the right record as the wrong one and does not say
  what to do. This is likely the most common first mistake (the docs say the module "must
  `import "@nx/visualizer"`").
- **Recommendation:** When the value's type has no origin in `@nx/visualizer` but its name equals the expected
  record, say that the record is not imported and suggest `import "@nx/visualizer"`. Also fix the article
  ("an 'ElementVisualizer'").
- **Fix:** When the written record's name equals the expected one, the `visualizer-record-mismatch` message
  (`crates/nx-types/src/infer.rs`) now reads "must be an 'ElementVisualizer' from "@nx/visualizer", and this
  'ElementVisualizer' is not that one", followed by "; add `import "@nx/visualizer"`" when the module does not
  import the library; the article is chosen by the name (`article`). Test
  `a_visualizer_without_the_library_imported_says_to_import_it` asserts the exact message.
- **Verification:** Re-ran the probe without the import. It now reads: "The visualizer of record 'Choice' must be an 'ElementVisualizer' from "@nx/visualizer", and this 'ElementVisualizer' is not that one; add `import "@nx/visualizer"`". The component case reads the same way. The article is right, and the import hint is given only when the import is missing.

### ✅ Verified - RF7 The "union a property uses comes along" test does not exercise the case it is named for
- **Severity:** Low (confidence: medium)
- **Evidence:** The source-tree scenario requires `ChoiceLayout` to be listed when "no node of the document
  names `ChoiceLayout`". `the_union_a_property_uses_comes_along`
  (`crates/nx-language-service/src/source_tree_tests.rs:1538`) uses `SURVEY_FORM`, which writes
  `layout=chips`, a `case` node that already brings `ChoiceLayout` into the table. The property-type path in
  `attach_visualizers` is covered only for the alias (`ImageUrl`).
- **Recommendation:** Use a form that leaves `layout` unset (as `the_base_name_is_not_inherited` does with
  `MultiChoice`) and assert `ChoiceLayout` still carries its case visualizers.
- **Fix:** `the_union_a_property_uses_comes_along` now uses a form that leaves `layout` unset, asserts no
  node names `ChoiceLayout`, and checks `ChoiceLayout` still carries its three case visualizers.
- **Verification:** The test now uses a `SingleChoice` that leaves `layout` unset. It asserts that no node refers to `ChoiceLayout` and that `ChoiceLayout` still carries its three case visualizers, so it exercises the property-type path in `attach_visualizers`. It passes.

## New Findings Discovered During 2026-10-12 18:40 Verification

### ✅ Verified - RF8 Dropping an erroneous visualizer declaration before duplicate detection lets a reported duplicate apply, and hides a cross-module duplicate
- **Severity:** Low (confidence: high, reproduced)
- **Evidence:** In `program_visualizers` (`crates/nx-api/src/program_visualizers.rs`), `has_errors_within` is checked
  before `targets.insert(...)`, and the cross-module duplicate check (`duplicate_program_visualizers`) runs over
  that filtered list. RF1's fix introduced this. Two probes:
  - **Same module:** `visualizer Choice = <ElementVisualizer name="One" title=lable />` followed by
    `visualizer Choice = <ElementVisualizer name="Two" />`. Analysis reports `duplicate-visualizer` at the second
    declaration, yet the resolved visualizer has `name: "Two"`. The skipped first declaration never claimed the
    target, so the second one applies even though it carries an error. That contradicts the spec ("a
    duplicate-visualizer error at the second") and the function's own doc comment ("So is the second ... left
    out"). It also breaks RF1's answer that a declaration with any error contributes nothing.
  - **Across modules:** two modules each declare `visualizer Choice`, and the one in `a.nx` has `title=lable`.
    The program reports no `duplicate-visualizer`, and the resolved visualizer takes `b.nx`'s `name: "B"`. The spec
    says two program modules that both declare a viewer for a target are a `duplicate-visualizer` error and
    neither applies. Fixing the typo in `a.nx` would then make a duplicate error appear and remove the
    viewer the reader had been seeing.
- **Recommendation:** Run duplicate detection over every visualizer declaration whose target resolves and
  whose record matches, before the error filter. In one module, call `targets.insert` before checking
  `has_errors_within`. Across modules, compute duplicates on the unfiltered list. After that, leave out
  the declarations that have errors and both members of any duplicate. Add tests for both probes.
- **Fix:** In `program_visualizers`, the first visualizer declaration whose target resolves now claims the target
  before anything else is checked, as analysis decides, so a second one in the module never applies. A
  declaration with errors is kept with `clean: false` (new `ProgramVisualizer::clean`), so
  `duplicate_program_visualizers` and the `duplicate-visualizer` diagnostic count it; `ProgramVisualizers::gather`
  applies only clean declarations, after removing duplicates, for the program and for host modules. Tests
  `the_second_of_two_visualizers_in_one_module_does_not_apply_when_the_first_has_errors` and
  `a_duplicate_with_errors_still_conflicts` fail without the fix.
- **Verification:** Rebuilt the wasm from f55faf4 and re-ran both probes. In the same-module probe, the first declaration (with `title=lable`) now claims the target. The second is reported as `duplicate-visualizer`, and `Choice` has no viewer. In the cross-module probe, `duplicate-visualizer` is now reported at `main.nx` for `t/a.nx` and `t/b.nx` with an error in `a.nx`, and neither applies. The same holds when `a.nx` lacks `import "@nx/visualizer"` or writes a computed `name={p}`. RF1 still holds: a host module with misspelled properties leaves the library's `name` "Lib" in place; a declaration with quoted case values, an unknown attribute or a repeated `title` contributes nothing; a clean viewer next to an unrelated error in its module still applies. Host modules filter by `clean` too. The viewer tests pass (94/94).

## Questions
- RF1: should a visualizer declaration with errors contribute nothing (all-or-nothing per declaration), or
  contribute the attributes that checked? The spec text reads as all-or-nothing per declaration; the
  recommendation assumes that.
  - **Answer:** All-or-nothing per declaration: a visualizer declaration with any error contributes nothing
    (RF1's fix).

## Summary
- The change is broad and largely solid: grammar, lowering, `T.Props`, `P.Property`, `Target` binding,
  resolution order and merging, duplicate reporting, the protocol plumbing (wasm, Node, language-core,
  HTTP validation), the playground request and the docs match the specs, and all touched test suites pass.
  The viewer's handling of untrusted source holds up: links and images are limited to `http:`/`https:`
  (with `noopener noreferrer`, `no-referrer`, lazy loading and a host off switch), `tel:` keeps digits
  only, markdown renders no links, and colours go through the CSSOM `backgroundColor` setter, which cannot
  trigger a fetch.
- The main gap is RF1: visualizer declarations are read syntactically and applied even when they fail to
  type-check, which contradicts the spec and pushes out-of-protocol values to readers; RF2 makes the viewer
  robust to such values. RF3–RF7 are smaller spec divergences, a typegen false positive, a silent merge
  loss, a confusing diagnostic and a test gap.

## Follow-up review: d04ad7b

**Reviewed code:** d04ad7b "Bind derived declarations of a library's selective imports in libraries":
`crates/nx-api/src/artifacts.rs` (`apply_graph_imports`, `apply_build_context_imports`,
`add_derived_interface_bindings`, `add_imported_interface_bindings`, and for comparison
`add_workspace_import_bindings`, `add_library_wildcard_bindings` and the runtime `visible_imports` maps),
`crates/nx-cli/src/typegen/model.rs` (`ImportedTypeCollector::collect_for_module`, `derived_imported_type`),
and the two new tests.  
**Checks run:** both new tests pass. Probes (sources under the session scratchpad `rvw2/`) through the built
`nxlang typegen`, and through a scratch binary that loads libraries into a `LibraryRegistry`, builds a
program with `build_program_artifact_from_source` and evaluates it:
- In a library, `import { Card, User } from "../ui"` with fields typed `Card.State`, `Card.State.Update`,
  `Card.State.Property`, `User.Update` and `User.Property` now type-checks. TypeScript typegen imports
  `Card_state as Card_State`, `Card_state_update as Card_State_Update`, `Card_state_property as
  Card_State_Property`, `User_update as User_Update` and `User_property as User_Property` from `"ui"`, all
  of which `ui`'s own typegen exports. C# references `global::Nx.Ui.Card_state` and the rest. The aliased
  form `import { Card as Ui.Card, User as Ui.User }` gives `Ui_Card_State` and so on. `Card.Props` still
  stops typegen with the existing "typegen does not generate props records yet" error, as for a program.
- Runtime: a library function `mk():Panel.State = { <Panel.State count=3 /> }`, along with
  `User.Update`, `Panel.State.Property` and `User.Property` values built in that library, evaluates
  correctly when a program calls it. The runtime `visible_imports` map already carried derived names for
  library modules.
- Diagnostics: the new bindings cannot add duplicate or ambiguity noise. `add_imported_interface_bindings`
  skips a name that is already bound (`artifacts.rs:3395`) before its "provided by both" check
  (`:3405`), so two imports of the same name resolve silently, first one wins. That was already the case
  for programs (see RF9 for what this means for derived names).
- Wildcard imports, with or without an alias, already bound derived names, because
  `library.exported_items` lists them. The workspace selective path and both `visible_imports` paths
  already expanded `DERIVED_DECLARATION_SUFFIXES`.

### ✅ Verified - RF9 A derived name binds even when its base name came from somewhere else, so `Card` and `Card.State` can name different components
- **Severity:** Low (confidence: high, reproduced)
- **Evidence:** `add_derived_interface_bindings` (`crates/nx-api/src/artifacts.rs:3337`) binds each
  `<Name>.<suffix>` on its own. It does not check whether the base binding for `<Name>` was actually made by
  this import. `add_imported_interface_bindings` quietly skips a name that is already bound
  (`:3395`). So when `Card` is already bound, by a local declaration or by an earlier import, while
  `Card.State` is not, the derived name still binds to the newly imported component. Before this commit a
  library build rejected such code ("component 'Card' declares no state"). Now it compiles. Probes, where
  `plain` exports a stateless `component <Card label:int />` and `ui` exports `Card` with
  `state { open:boolean = false }`:
  - A library module with `import { Card } from "../plain"`, `import { Card } from "../ui"`,
    `export let x():Card.State = { <Card.State open=true /> }` and `export let y() = { <Card label=3 /> }`
    loads with no diagnostic. A program calling both prints `content={true <Card label=3 />}`: `Card` is
    `plain`'s component and `Card.State` is `ui`'s.
  - The same thing happens with a local `export component <Card label:int />` next to
    `import { Card } from "../ui"`. The local `Card` shadows the import silently, and `Card.State` still
    reaches `ui`'s.
  - `nxlang typegen` on a library with the two imports and `export type Saved = { s:Card.State }` exits 0.
    It prints only warnings ("no generated companion 'Card_state'", "Skipping imported type 'Card' from
    '../ui'"), and writes `s: Card_State;` with no import of `Card_State`, so the generated TypeScript
    does not compile.

  The program path (`apply_graph_imports` at `:2674`, the workspace path at `:2885`, and the
  `visible_imports` expansions at `:3688` and `:3857`) behaved this way before. This commit extends it to
  library builds, the one place that had rejected it.
- **Recommendation:** Bind a selective import's derived names only when the base name bound to the same
  declaration in this import. One way: have `add_imported_interface_bindings` / `add_workspace_item_bindings`
  return whether they bound anything, and call the derived expansion only then. Apply the same guard in
  the two `visible_imports` expansions so runtime resolution agrees. The derived-suffix expansion now
  appears in four places (`add_derived_interface_bindings`, `add_workspace_import_bindings`, and the two
  `visible_imports` loops). Sharing one name-pair iterator, such as `derived_names(entry, visible_name)`,
  would keep them in step. Separately, consider whether two selective imports of the same name should
  report "provided by both", since the `has_binding` check before it makes that diagnostic unreachable
  for them. Add a library test: `Card` from a stateless source plus `Card.State` reports "declares no
  state" (or a duplicate-import error) rather than resolving to the other component.
- **Fix:** A selective import now binds its derived names only when it bound the base name to that same
  declaration (`binds_to`, checked in `add_derived_interface_bindings` for both library paths and in
  `add_workspace_import_bindings`). `add_derived_interface_bindings` takes the base's `item_indices` for
  this. A name taken by a local declaration or an earlier import keeps its own derived names, so
  `Card.State` there reports "component 'Card' declares no state". Tests
  `a_library_does_not_take_derived_declarations_from_an_import_that_lost_the_name`,
  `a_local_component_keeps_its_own_derived_declarations_beside_an_import` and
  `a_program_does_not_take_derived_declarations_from_an_import_that_lost_the_name` all fail without the
  guard. Left as is: the two `visible_imports` expansions, because type checking now rejects any program
  that would reach them with a mismatched pair; and the unreachable "provided by both" diagnostic for
  two selective imports of one name, which predates this change and would change behavior beyond this fix.
- **Verification:** I rebuilt from be1b9c5 (`nxlang` and a scratch probe binary on `nx-api`) and re-ran the
  probes. Each of these now reports "component 'Card' declares no state" instead of resolving
  `Card.State` to `ui`'s component:
  - In a library: `import { Card } from "../plain"` followed by `import { Card } from "../ui"`, and a
    local `component <Card label:int />` next to `import { Card } from "../ui"`.
  - In a program built from source: the same two cases.
  - In a workspace (`./plain.nx`, `./ui.nx`): the same two cases.

  `nxlang typegen` on the two-import library now fails with that error (exit 1) instead of writing an
  unimported `Card_State`. With two stateful `Card`s (p2), the first import still provides both `Card`
  and `Card.State`, so the pair stays consistent. d04ad7b's behavior still holds:
  - The selective `Card.State` / `.State.Update` / `.State.Property` / `User.Update` / `User.Property`
    library still type-generates the same TypeScript and C# imports. So does the aliased
    `import { Card as Ui.Card, User as Ui.User }`.
  - The runtime probe (a library function building `<Panel.State count=3 />`, `User.Update` and property
    values, called from a program) still evaluates.

  `cargo test -p nx-api` (435 + 10 + 29) and `cargo test -p nx-cli` (268) pass. The fix covers selective
  imports, which is everything RF9 recommended. Wildcard imports still pair names across sources; that is
  filed as RF10.

## New Findings Discovered During 2026-10-12 20:10 Verification

### ✅ Verified - RF10 Wildcard imports still bind `<Name>.State` and the other derived names when `<Name>` itself is taken
- **Severity:** Low (confidence: high, reproduced)
- **Evidence:** A wildcard import binds every export separately, including the derived `Card.State`,
  `Card.State.Update`, and so on. `add_library_wildcard_bindings` (`crates/nx-api/src/artifacts.rs`)
  iterates `library.exported_items`, and the wildcard branch of `add_workspace_import_bindings` iterates
  every exported item. So a derived name binds even when its base name was skipped because it was already
  bound, the same split RF9 fixed for selective imports. `plain` exports a stateless
  `component <Card label:int />` and `ui` exports `Card` with `state { open:boolean = false }`. Each of
  the following type-checks and evaluates `x().open` to `true`, with `Card` naming one component and
  `Card.State` the other:
  - Library module: `import "../ui"` plus a local `export component <Card label:int />` and
    `export let x():Card.State = { <Card.State open=true /> }`.
  - Library module: `import "../plain"` then `import "../ui"`.
  - Library module: `import { Card } from "../plain"` then `import "../ui"`.
  - Workspace `main.nx`: `import "./ui.nx"` plus a local stateless `component <Card label:int />`.
- **Recommendation:** In both wildcard paths, bind a `<Name>.<suffix>` export only when the same import
  bound `<Name>` to its declaration: reuse `binds_to`, or skip derived export names and expand them from
  each base the way the selective paths do. Apply the same rule to the two `visible_imports`
  expansions, or rely on the type checker as RF9's fix does. Add a library test and a workspace test: a
  local stateless `Card` plus a wildcard import of a stateful one reports "declares no state".
- **Fix:** Both wildcard paths (`add_library_wildcard_bindings` and the wildcard branch of
  `add_workspace_import_bindings`) now bind a module's own declarations first, then its derived ones, and
  bind a derived declaration only when the import bound its base (`derived_base`, longest suffix first, so
  `Card.State.Update` derives from `Card`; then `binds_to`). Tests
  `a_library_wildcard_import_does_not_bring_derived_declarations_of_a_name_it_lost` (two imports, and a local
  `Card` beside an import) and `a_workspace_wildcard_import_does_not_bring_derived_declarations_of_a_name_it_lost`
  fail without the guard; `a_wildcard_import_still_brings_derived_declarations_beside_their_base` covers
  plain and aliased wildcard imports still bringing `Card.State`, `Card.State.Update` and `Ui.User.Update`.
- **Verification:** I rebuilt from b18650c and re-ran the four wildcard probes. Each now reports "component
  'Card' declares no state" instead of pairing `Card` with `ui`'s `Card.State`:
  - Library: a local `Card` beside `import "../ui"`.
  - Library: `import "../plain"` then `import "../ui"`.
  - Library: `import { Card } from "../plain"` then `import "../ui"`.
  - Workspace: a local `Card` beside `import "./ui.nx"`.
  - The same two-wildcard and local-plus-wildcard programs built from source report the error too.

  Derived names still come along beside their base:
  - A library with a plain `import "../ui"` loads and evaluates `Card.State`, `Card.State.Update`,
    `Card.State.Property`, `Card.Props`, `User.Update` and `User.Property` values.
  - A library with `import "../ui" as ui` loads with `ui.Card.State`, `ui.Card.Props`, `ui.User.Update`
    and `ui.User.Property` parameters. `ui.Card.Stat` still gets the "did you mean `ui.Card.State`"
    hint.
  - A program with `import "../ui"` evaluates `x().open`.
  - `nxlang typegen` emits the companion imports for both wildcard forms: `Card_state as ui_Card_State`
    and the rest for the aliased form, `Card_state as Card_State` and the rest for the plain form.

  RF9 still holds: both selective library probes, the two-import typegen (exit 1) and the workspace
  selective probe report the error. A lone selective import still works, and so do d04ad7b's typegen
  (plain and aliased) and runtime probes. Two stateful `Card`s still resolve to the first import.
  `cargo test` passes for `nx-api` (438 + 10 + 29), `nx-cli` (268) and `nx-language-service` (222).

## Out-of-scope observation (verification 2026-10-12 20:10)
- This one predates this change. The PR does not touch `nx-interpreter` or the runtime `visible_imports`
  maps. Qualified (aliased) imports of records type-check but fail at run time or report wrong
  diagnostics. With `import "../ui" as ui`, in a program as well as in a library:
  - `<ui.User name="a" />` fails at run time with "Record type not found: User".
  - `<ui.Card.State open=true />` fails with "Record type not found: Card.State".
  - `<ui.User.Update name="a" />` is rejected with "Element 'ui.User.Update' requires property 'age'",
    although an update record has only optional fields.
  - A function returning `ui.User.Property.name` fails with "expected ui.User.Property, got
    User.Property.name".

  `import { Card as ui.Card }` does the same. The unqualified forms work. Worth its own issue or change.

## New Findings Discovered During 2026-10-15 05:40 Review

**Scope of this pass:** the source-first Read view merged in at 9b4278f (`git log 9b4278f^2..5372545`,
diffed against `eced3fe`, the PR head before it).  
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/viewer, specs/visualizer-library,
specs/source-tree, specs/playground.  
**Reviewed code:** `packages/viewer/src/{index,source,styles,tree}.ts` and its tests and fixtures,
`crates/nx-language-service/src/{source_tree,visualizers}.rs`, `crates/nx-api/src/std/visualizer/visualizer.nx`,
`packages/language-protocol`, `packages/value-view` exports, `sites/playground` (`App.tsx`, `app.css`,
`theme.ts`, `reading/ReadingView.tsx`, `NxEditor.tsx`, `OutputPane.tsx`, `PreviewView.tsx`, examples,
`check-examples.mjs`, package.json), `packages/viewer/README.md`, `sites/playground/README.md`, and the
website's visualizer-declarations and `@nx/visualizer` pages.  
**Checks run:**
- `cargo fmt --all --check`: clean.
- `cargo test -p nx-language-service -p nx-api`: all pass (439 + 10 + 29 nx-api, 222 nx-language-service).
- `openspec validate add-visualizer-declarations --strict`: **fails** (RF11).
- `packages/viewer` `pnpm test`: 26/26 pass. That needed a rebuild first: the checked-out
  `language-protocol` dist and wasm SDK were stale (5 tsc errors, then 4 test failures), and the old
  `dist` still held the deleted tests (17 failures until it was removed; see RF22).
- `sites/playground` `pnpm test`: 121/121 pass, and `check-examples` reports all 23 examples match.
  `pnpm typecheck` passes after `pnpm install`. Before the install it failed because `lucide` was missing
  from the stale `node_modules`.
- Probes of `<nx-viewer>` under jsdom on trees from the rebuilt wasm SDK, to confirm RF12 to RF16
  and RF18.

### ✅ Verified - RF11 `openspec validate add-visualizer-declarations --strict` fails on the new "Code reads as written" requirement
- **Severity:** Medium (confidence: high)
- **Evidence:** The validator reports `viewer/spec.md: ADDED "Code reads as written, in the editor's
  colors" must contain SHALL or MUST`. It reads the requirement's first line, and that line
  (`specs/viewer/spec.md:84`, "Text that is not part of an element read as a group or a line, such as a
  declaration, a function's") holds no SHALL. The SHALL wraps onto line 85. Every other requirement has
  SHALL on its first line. tasks.md 8.1 says this validation was run.
- **Recommendation:** Open the requirement with a sentence that holds SHALL on its first line, for
  example "The element SHALL read text that is not part of an element read as a group or a line …".
  Then rerun the strict validation.
- **Fix:** Opened the requirement with "The element SHALL read text as the document writes it …", so SHALL
  is on its first line. `openspec validate add-visualizer-declarations --strict` passes.
- **Verification:** `openspec validate add-visualizer-declarations --strict` now reports the change valid. The
  requirement opens with "The element SHALL read text as the document writes it …".

### ✅ Verified - RF12 A union case element never gets an icon, though the spec, README and docs promise `tag`
- **Severity:** Medium (confidence: high)
- **Evidence:** `elementIcon` (`packages/viewer/src/source.ts:317-327`) finds the kind through
  `node.declaration`. The source tree gives a union case element no declaration: the element
  nodes of `<Status.shipped carrier="x" />`, alone in a function body or as a property's value, have
  `declaration: undefined`. So `KIND_ICONS.union = "tag"` (`source.ts:60`) can never be reached. In a
  probe, that element renders with no icon at all, while a record beside it gets `box`. Several places
  promise `tag` for a union case: `specs/viewer/spec.md:173`, `packages/viewer/README.md:139`, and the
  website page ("an element with none takes one for its kind of declaration").
- **Recommendation:** Give union case elements the union's declaration index in the source tree, or
  resolve it in the viewer from the element's name (`Status.shipped` → `Status`). Add a test with a case
  element alone on its line. If case elements are meant to stay unmarked, drop `tag` from the spec, the
  README and `KIND_ICONS`.
- **Fix:** The real gap was the source tree: `element` in `source_tree.rs` now falls back to the union a
  case element's type names (`case_declaration`), so `<Status.shipped … />` refers to `Status` and the
  viewer's `KIND_ICONS.union` (`tag`) applies. Added the Rust test `a_case_element_refers_to_its_union`
  (alone and as a property's value), a `Status.closed` element to the `icons` fixture with its `tag` icon
  asserted, and a line to the source-tree delta spec.
- **Verification:** `a_case_element_refers_to_its_union` passes. Rerunning the earlier probe, `<Status.shipped
  carrier="x" />` alone in a function body now starts with `tag`, and the `icons` test asserts `Status.closed`
  → `tag`. Union entries carry no `properties`, so giving case elements their union does not change the
  uitext, format or slot-type lookups.

### ✅ Verified - RF13 The arrow keys jump to the top of the document when the selection is not a unit
- **Severity:** Medium (confidence: high)
- **Evidence:** `#stepUnit` (`packages/viewer/src/index.ts:366-371`) looks the selected element up in
  the list of units, and when the selection is not itself a unit (`at < 0`) it selects the first unit,
  or the last one for Up. A selection is often not a unit. A click outside every unit selects the
  smallest node there. The playground selects the smallest node at the editor's cursor on entering
  Read (`sites/playground/src/reading/selection.ts:18`, called from `App.tsx:182`). In a probe, the host
  selected the literal `"z"` in a group's row. Down then selected `a.value`, the first element of the
  document, and Left did the same instead of selecting the row that holds `"z"`. The spec says the
  arrows move to the previous or next unit and that Left selects "the unit that holds the selected one".
- **Recommendation:** Before stepping, map a selection that is not a unit to its unit with
  `#unitOf(current)`, or to the nearest unit holding it. Left then selects that unit, and Up and Down
  step from it. Add a keyboard test that starts from a selection set by the host inside a row.
- **Fix:** `#stepUnit` maps a selection that is not a unit to the unit that holds it (`#unitOf`): Left
  selects that unit, Up and Down step from it, and Right takes the first unit the selection holds. Outside
  every unit, Down and Up select the nearest unit after or before it. Added the keyboard test "moves from
  the unit that holds a selection the host makes inside it" (fixture `rows`), a spec sentence and a
  scenario, and a README line.
- **Verification:** Rerunning the earlier jsdom probe on the same document, Left from the host-selected `"z"`
  now selects its row `c.value.note`, not `a.value`. Down moves from that row; it is the last unit there, so
  it stays on it. The new keyboard test (`rows`) covers Left, Down and Up from inside a row, and Down and Up
  from a type's field outside every unit.

### ✅ Verified - RF14 A comment inside an attribute of a group drops a node, losing text or a key
- **Severity:** Low (confidence: high)
- **Evidence:** `sheetable` (`source.ts:386-388`) skips comments when it counts an attribute's values,
  but the renderers do not put them back:
  - In a row, `writeValue` (`source.ts:414-422`) writes only the one value. In a probe,
    `name= /* why */ "a"` on a line of its own rendered the row as `name a`: the comment's text
    disappeared and its node (`x.value.name.value:comment[0]`) was not rendered.
  - On the header line, `onHeader` attributes go through `namedPart` → `writeLineValue`, which takes
    `childrenOf(attribute)[0]` (`source.ts:582`), here the comment. In a probe,
    `<Item name= /* c */ "a"` rendered the comment as the value and `"a"` as plain text. The node
    `x.value.name.value` had no span, so it cannot be selected.

  Both break "The reading keeps every node". The repository coverage test misses them because no
  `.nx` file in the repository has this shape.
- **Recommendation:** Make `sheetable` fall back to the element as written when a row or header
  attribute holds a comment, or have `writeValue` and `writeLineValue` write the comment nodes too. Add
  the two shapes to the fixtures with a check that every key is rendered.
- **Fix:** `sheetable` now reads an element as written when a row or header attribute holds a comment, and
  `elementIdPlans` leaves an id attribute that holds a comment in place. Added the `attributeComments`
  fixture (a row, a header attribute and an id, each with a comment) with a test that every comment's text
  and every key is rendered once. The coverage test's fixture case now renders every fixture in
  `sources.json`, so it fails on the old code.
- **Verification:** Rerunning the earlier probes, `name= /* why */ "a"` on a row and `<Item name= /* c */ "a"`
  on a header now read as written, with every comment's text and every key present (no missing nodes). An id
  attribute that holds a comment stays in place. The coverage test now renders every fixture,
  `attributeComments` included.

### ✅ Verified - RF15 The reading drops an element's text type, so `<Note:markdown>` reads as `Note`
- **Severity:** Low (confidence: high)
- **Evidence:** `writeSheet` (`source.ts:501-502`) and `lineParts` (`source.ts:654`) write only
  `node.name.length` characters after the `<`. A probe of `<Note:markdown>` over several lines rendered
  the header `Note`. On one line, `<Note:markdown>Hello *there*</Note>` rendered
  `<Note  Hello *there*/>`. In both, `:markdown` is gone, and the reading no longer says how the text
  is parsed. `elementIdPlans` already accounts for a `:type` after the name (`source.ts:883-889`).
- **Recommendation:** Write the name through the end of the text type in both places, reusing the
  `nameEnd` logic from `elementIdPlans`. Add a fixture.
- **Fix:** A shared `nameEnd` (moved out of `elementIdPlans`) writes the name through its text type in
  `writeSheet` and `lineParts`. Added the `textTypes` fixture and a test: the line reads `<Note:markdown
  Hello *there*/>` and the group's header `Note:markdown`.
- **Verification:** Probe and the `textTypes` test: the group's header reads `Note:markdown`, and the line
  reads `<Note:markdown Hello *there*/>`.

### ✅ Verified - RF16 In a group, `idPosition=end` is ignored and the id always follows the name
- **Severity:** Low (confidence: high)
- **Evidence:** `writeSheet` puts the id chip right after the name whatever `plan.atEnd` says
  (`source.ts:503-506`). With `visualizer Choice = <ElementVisualizer title=label id=value idPosition=end />`, a
  `<Choice value="yes" label="Yes"` written over several lines renders the header `Choice yes  label=Yes`.
  The id comes before the header attribute, and the chip still gets the `id-end` class meant for a
  chip that closes a tag. The `@nx/visualizer` reference page says `end` puts the id "last in its tag, after
  the properties that stay".
- **Recommendation:** For `atEnd`, append the chip after the header's attributes. Or state in the spec
  and the docs that a group always shows its id after its name, and leave off the `id-end` class there.
- **Fix:** `writeSheet` puts an `atEnd` id after the header's attributes, with the usual two-space gap.
  Added the `groupIdAtEnd` fixture and a test (`Choice label=Yes yes`, the chip last in the header), and
  the group requirement and README now say where the id goes.
- **Verification:** The probe of a `Choice` with `idPosition=end` written over several lines now reads the
  header `Choice label=Yes yes`, with the chip last. The `groupIdAtEnd` test, the spec and the README agree.

### ✅ Verified - RF17 Long one-line elements and row values outside a written-as-is item do not wrap
- **Severity:** Low (confidence: medium; reasoned from the CSS, not checked in a real browser)
- **Evidence:** `.source` is `white-space: pre` (`styles.ts:57`). Only these get `pre-wrap`: an element
  inside a `.sheet-item.as-written` (`styles.ts:177-188`), a group's header (`styles.ts:190`), and
  `uitext` bodies. A one-line `<Choice …/>` placed directly in a group's held list, a one-line element in
  a function body, and a long plain string in a row's value column therefore run past the pane, except
  where their text is `uitext`. The spec requires the wrap at `specs/viewer/spec.md:112`, "A line too
  long for the pane SHALL wrap a step in from where its unit starts", and the README says the same.
- **Recommendation:** Give `.line-element`, `.tag-line` and `.sheet-value` the same wrapping as the
  rule at `styles.ts:177-188`, wherever they sit. Or narrow the spec and README to the cases that wrap.
  Check the result in a browser at 375px.
- **Fix:** Fixed in CSS where the fix is clear, and narrowed the spec for the rest. A row's value
  (`.sheet-value`) is now `pre-wrap`, and an element on one line alone on its line (new class `alone`)
  gets the same wrapping as the as-written rule wherever it sits. An element written as its tags, or code
  read as written outside a group, keeps its source lines: wrapping those would double their indentation.
  The spec and README now name the units that wrap. No browser was available here, so this is not checked
  at 375px.
- **Verification:** Checked in the preinstalled Chromium (Playwright, `chromium-1194`) with the reading 375px
  wide. A long plain-string row value wrapped to 4 lines and stayed inside the pane (right edge 347 of 359).
  Two long one-line elements alone on their lines (one in a group's held list, one in a function body) wrapped
  to 5 and 4 lines, inside the pane (339), their later lines a step in. The playground's `.reading` is
  `overflow: auto`, so the page itself does not scroll sideways. Two things still run past the edge, as the
  narrowed spec allows: a one-line element that shares its line (`let pick = <Choice …/>`), and code read as
  written. Cosmetic: at that width, `overflow-wrap: break-word` can split a long id chip across two lines
  (`one-very-long-` / `identifier`).

### ✅ Verified - RF18 An `icon` that Lucide does not have removes the element's icon instead of falling back to its kind's
- **Severity:** Low (confidence: high)
- **Evidence:** `elementIcon` (`source.ts:323-327`) picks the viewer's `icon` when it is set and returns
  nothing when the host cannot draw it. In a probe, `visualizer A = <ElementVisualizer icon="no-such-icon" />`
  left `<A …/>` with no icon, where without the attribute it gets `box`. Nothing reports the bad name:
  `icon` is a plain `string`. The spec says the icon is "the one its viewer's `icon` names, or else one
  for its kind".
- **Recommendation:** When `layout.icons(name)` returns nothing for the viewer's icon, try the kind's
  icon. Optionally, document that names are not checked.
- **Fix:** `elementIcon` tries the viewer's icon, then the kind's when the host draws nothing for that
  name. The `icons` test now has a `Tip` whose viewer names `no-such-icon` and reads `box`. Spec sentence
  and scenario added, and the README says so.
- **Verification:** The probe with `icon="no-such-icon"` now starts the element with `box`, and the `icons`
  test covers `Tip`. The spec sentence and scenario match.

### ✅ Verified - RF19 tasks.md still marks the removed cards-view work as done
- **Severity:** Low (confidence: high)
- **Evidence:** `tasks.md:46-49` are checked, but what they describe was removed in cf8edda:
  - 6.1: kind names; title, subtitle, badge and image parts; labels; boolean phrases; the details strip.
  - 6.2: the `row` and `inline` layouts.
  - 6.3: case labels; `url`/`email`/`phone`/`image`/`markdown`/`text` formats; the `images` property.
  - 6.4: "Viewer for …".

  Three more items are off:
  - 6.5's survey fixture is no longer used by any test (see RF20).
  - 8.1 records a strict `openspec validate` that now fails (RF11).
  - No task covers the Read view itself: groups, lines, ids, icons, colors, Geist and Lucide in the
    playground, and the larger examples.
- **Recommendation:** Rewrite section 6 to match the design's "What the viewer does with visualizer
  declarations" and the new viewer spec, and add the Read view and larger-examples tasks.
- **Fix:** Rewrote section 6 of tasks.md for the Read view: a note on the replaced cards work, then the
  Read view's tasks (reading, visualizer values, selection and keys, coverage and fixtures, the playground's
  Read view, the larger examples), all checked. 8.1 holds again now that strict validation passes.
- **Verification:** tasks.md section 6 now describes the Read view: the reading, visualizer values, selection and
  keys, coverage and fixtures, the playground's Read view, and the larger examples. 8.1 holds now that strict
  validation passes. My original evidence was partly wrong: the six cards-era fixtures were still used by the
  coverage test's viewer case. That case now covers every fixture.

### ✅ Verified - RF20 Leftovers of the removed cards view and menus remain in code, tests and package metadata
- **Severity:** Low (confidence: high)
- **Evidence:**
  - `packages/viewer/package.json` `description` still says "cards, sentences and values instead of
    syntax".
  - `packages/viewer/src/source.ts:1-7` says "Nothing is rewritten or moved" and refers to "the card
    reading". The reading now drops tags and quotes, moves ids and replaces booleans with marks.
  - `packages/viewer/test/fixtures/sources.json`: `survey`, `surveyRenamed`, `viewerDeclaration`,
    `inlineTag`, `formats` and `surveyOptional` are not used by any test. They were the deleted
    `viewers.test.ts`'s fixtures.
  - The removed Theme menu left scaffolding in the playground:
    - `const themes = EDITOR_THEMES` aliases in `NxEditor.tsx:67-69` (with a `latestThemes` ref),
      `OutputPane.tsx:38` and `PreviewView.tsx:272,353`.
    - In `ReadingView.tsx:49-66`, a highlighter stored "with the pair it colors with" and an effect
      keyed on a constant.
    - `EditorThemes` and `ALL_EDITOR_THEMES` in `theme.ts:15-28`.
  - `App.tsx:66-67` has two doc comments on `SourceMode`. `mode !== "edit"` stands in for
    `mode === "read"` in five places, and `MODE_LABELS` is left from a third mode.
- **Recommendation:** Update the description and the `source.ts` header. Delete the unused fixtures.
  Use `EDITOR_THEMES` directly and drop the aliases, the ref and `EditorThemes`. Remove the duplicate
  comment and test `mode === "read"`.
- **Fix:** Updated the `package.json` description and the `source.ts` header. Removed the `themes`
  aliases, the `latestThemes` ref, `EditorThemes`, `ALL_EDITOR_THEMES` (inlined in `NxEditor`) and the
  highlighter-with-its-pair state and its constant-keyed effect in `ReadingView`. Removed the duplicate
  `SourceMode` comment and `MODE_LABELS`, and `App.tsx` tests `mode === "read"`. The six cards fixtures
  were still used, by the coverage test's viewer case, but only for cards-era attributes. They are
  deleted, and that case now covers every fixture.
- **Verification:** The `package.json` description and the `source.ts` header are rewritten. None of
  `MODE_LABELS`, `mode !== "edit"`, `ALL_EDITOR_THEMES`, `EditorThemes`, `latestThemes` or the `const themes
  =` aliases remain in `sites/playground/src`, and there is one `SourceMode` comment. The playground typecheck
  and tests pass.

### ✅ Verified - RF21 The new viewer fields have no Rust tests, and the playground test still checks the old scenario
- **Severity:** Low (confidence: high)
- **Evidence:**
  - `source_tree_tests.rs` has no assertion on `id`, `idPosition` or `icon`. Untested are
    `idPosition` defaulting to `beginning` only when `id` is set and dropped otherwise
    (`source_tree.rs:2029-2035`), `is_known_value("idPosition", …)`, and `icon` inherited through
    `extends`. The JS tests reach `id` and `icon` only through the wasm SDK, and none covers
    inheritance or the `idPosition` default.
  - `sites/playground/src/worker/session.test.mjs:123-139` still asserts
    `<ElementVisualizer name="Option" title=label />`. The playground spec's scenario changed to
    `title=label id=value`, read as `Apples` with the id `a`.
- **Recommendation:** Add a `source_tree_tests` case covering `id` + `idPosition` (default, `end`, and
  `idPosition` without `id`) and an inherited `icon`. Update the playground test to the spec's scenario.
- **Fix:** Added `an_id_reads_where_its_position_puts_it` (`idPosition` defaults to `beginning` with an
  id, `end` kept, dropped without an id) and `an_icon_is_inherited_through_extends` (a base's icon
  inherited, a kind's own winning). `session.test.mjs` now uses the spec's `title=label id=value` scenario
  and checks that the title and id attributes hold `"Apples"` and `"a"`.
- **Verification:** `an_id_reads_where_its_position_puts_it` and `an_icon_is_inherited_through_extends` pass
  (nx-language-service now has 225 tests, up from 222). `session.test.mjs` asserts the spec's `title=label
  id=value` scenario.

### ✅ Verified - RF22 The build does not clean `dist`, so deleted visualizer modules and tests survive a rebuild
- **Severity:** Low (confidence: medium)
- **Evidence:** `packages/viewer` builds with plain `tsc -p tsconfig.json`. A checkout that built before
  cf8edda keeps these files in `dist`:
  - `dist/src/{render,hover,text,words}.js`
  - `dist/test/{viewers,text}.test.js`

  `pnpm test` runs `dist/test/**/*.test.js`, so it ran the deleted tests here and reported 17
  failures until `dist` was removed. `prepack` builds the same way, and `files: ["dist", "!dist/test"]`
  would publish the deleted modules from such a checkout.
- **Recommendation:** Clean `dist` before building, in `build` or in `test` and `prepack`.
- **Fix:** `packages/viewer` now has `clean` (`scripts/clean.mjs`, as `bindings/wasm` and `bindings/node`
  do), and `build` runs it first. `test` runs `build`, and `prepack` already did, so a rebuild never keeps
  deleted modules or tests.
- **Verification:** I put a stale `dist/test/stale.test.js` and `dist/src/render.js` into `dist` and ran `pnpm
  run build`; both were gone afterwards. `test` and `prepack` build through it.

## New Findings Discovered During 2026-10-16 01:56 Review

**Scope of this pass:** the cut of `@nx/visualizer` from nine types to five in 20d8efe and 5205726
(`git diff 2c3920d..HEAD`), weighted over the rest of the change.  
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/visualizer-library,
specs/visualizer-declarations, specs/source-tree, specs/editor-syntax-highlighting, specs/viewer,
specs/playground, specs/cli-code-generation.  
**Reviewed code:** `crates/nx-api/src/std/visualizer/visualizer.nx`, `crates/nx-api/src/program_visualizers.rs`,
`crates/nx-api/src/visualizer_declarations_tests.rs`, `crates/nx-hir/src/visualizers.rs`,
`crates/nx-types/src/infer.rs` (visualizer parts), `crates/nx-language-service/src/{source_tree,visualizers,lib}.rs`
and `source_tree_tests.rs`, `crates/nx-cli/src/main.rs`, `crates/nx-codegen/src/ir_runtime_tests.rs`,
`crates/nx-syntax/tests/parser_tests.rs`, `packages/language-protocol`, `packages/viewer` (`source.ts`, README,
fixtures), `packages/language-http` test, `bindings/node/src/index.ts`, `bindings/wasm/test/parity.test.ts`,
the playground examples and `session.test.mjs`, the website's visualizer-declarations, `@nx/visualizer` and
modules pages, `nx-grammar.md`, and the VS Code grammar test. I searched the whole repository (outside
`openspec/changes/archive`) for `UnionVisualizer`, `CaseVisualizer`, `ElementLayout`, `Placement`,
`whenTrue`/`whenFalse`, `SourceCaseVisualizer`, `SourceVisualizerLayout`, `beginning`, `of=`, `name=`,
`subtitle=`, `badge=`, `image=`, `layout=`, `place=` and the removed formats. No code, binding, test, example,
grammar or snippet still uses them; the leftovers are prose only (RF25).  
**Checks run:**
- `pnpm -r build`: all packages build (the checked-out `language-protocol` dist was stale until then).
- `cargo test -p nx-api -p nx-language-service -p nx-hir -p nx-types -p nx-syntax -p nx-cli` and the
  codegen visualizer test: all pass. `cargo fmt --all --check` and `cargo clippy --all-targets` on the
  changed crates: clean.
- Package tests: `packages/viewer` 32/32, `language-protocol` 4/4, `language-http` 21/21, playground 121/121
  plus `check-examples` (23/23), `src/vscode` 287 passing. `bindings/wasm`: 124/125, and the one failure is the
  known `recursion-stack` test.
- `sites/website` `check-code-blocks`: 214 blocks, 0 failures, after the wasm SDK was rebuilt.
- `openspec validate add-visualizer-declarations --strict`: valid.
- `nxlang typegen @nx/visualizer` for TypeScript and C#: both generate (C# warns that the `idPosition`
  default is omitted, as it did before for `beginning`, `card` and `body`).
- CLI probes: the old spellings (`of=`, `name=`, `layout=`, `idPosition=beginning`, `format=image`) each get a
  clear diagnostic, and a union target with an `ElementVisualizer` holding a `PropertyVisualizer`, or with a
  `ValueVisualizer`, gets exactly one diagnostic. Every Lucide icon name in the examples and docs exists in
  the installed `lucide`.

### ✅ Verified - RF23 No test covers a `PropertyVisualizer`'s format overriding its alias's format any more
- **Severity:** Low (confidence: high)
- **Evidence:** The spec scenario "A property visualizer overrides its type's format"
  (`specs/visualizer-declarations/spec.md`, last scenario) has no test. Before the cut,
  `a_host_module_rewords_a_kind_and_merges_property_visualizers` covered this path. `ImageUrl` had
  `<ValueVisualizer format=image />`, and the host's `<PropertyVisualizer of=imageUrl format=url />` had to win.
  The cut removed `visualizer ImageUrl` from `SURVEY` (`crates/nx-language-service/src/source_tree_tests.rs:1408-1424`).
  So the rewritten test (`a_host_module_overrides_and_merges_property_visualizers`, `:1636-1656`) only gives a
  format to a property that had none. Only `tint` has an alias format (`color`), and nothing overrides it. In
  the other tests, every `format=` on a `PropertyVisualizer` names a property whose type has no visualizer. The
  `merged.get("format").or_else(|| type_format(...))` precedence in `element_visualizer`
  (`source_tree.rs:1942-1949`) is therefore unguarded. A swapped order would still pass every test. The
  behavior is correct today. A probe through the Node SDK with `tint?:Color`, `visualizer Color =
  <ValueVisualizer format=color />` and `<PropertyVisualizer property=tint format=id />` resolves `tint` to
  `id`. The cut also dropped the guard assertion that RF7's fix added: the old test checked that no node names
  the alias. `an_alias_formats_every_property_that_uses_it` (`:1594-1601`) states that in a comment but no longer
  asserts it.
- **Recommendation:** In the host test, override `tint`, e.g. `<PropertyVisualizer property=tint format=id />`,
  and assert `format_of(choice, "tint") == Some("id")`. Restore the
  `!tree.nodes.iter().any(|node| … name == "Color")` guard in `an_alias_formats_every_property_that_uses_it`.
- **Fix:** The host in `a_host_module_overrides_and_merges_property_visualizers` now also sets `<PropertyVisualizer property=tint format=id />` and the test asserts `tint` resolves to `id` over its alias's `color`; swapping the precedence in `element_visualizer` makes it fail (checked). `an_alias_formats_every_property_that_uses_it` again asserts that no node names `Color`.
- **Verification:** Both assertions are in place, and `cargo test -p nx-language-service` passes (224). I checked it independently on a scratch copy of the workspace: with `type_format` tried before the property visualizer's `format` in `element_visualizer`, only `a_host_module_overrides_and_merges_property_visualizers` fails, and the fixture-dependent tests that fail in the copy fail with or without the change. The restored guard asserts that no node of the form refers to `Color`, and it passes, so the "alias comes along" assertion is not vacuous.

### ✅ Verified - RF24 A `T.Property` target is now told "it is a union, and unions cannot have visualizers yet"
- **Severity:** Low (confidence: high)
- **Evidence:** `classify` (`crates/nx-hir/src/visualizers.rs:166`) folds every union into one arm. Before
  the cut, a derived property union had its own reason, "it is a derived property union". Now
  `visualizer User.Property = <ElementVisualizer />` reports
  `'User.Property' cannot have a visualizer: it is a union, and unions cannot have visualizers yet`
  (reproduced with `nxlang run`). The spec (`specs/visualizer-declarations/spec.md`, "A visualizer declaration's
  target decides its visualizer record") and the website list `T.Property` among the derived declarations,
  which are rejected for good. The word "yet" tells the author it will work once unions do.
  `a_derived_declaration_is_not_a_target` (`crates/nx-api/src/visualizer_declarations_tests.rs:297-316`)
  checks only the code and the "cannot have a visualizer" prefix, so it did not notice.
- **Recommendation:** Restore the check before the union arm:
  `Item::Union(union_def) if union_def.property_target().is_some() => not_allowed("it is a derived property union")`.
  Extend the test to assert the reason for `User.Property`.
- **Fix:** `classify` again reports "it is a derived property union" before the general union arm. `a_derived_declaration_is_not_a_target` now asserts each target's full reason, including `User.Property` and `Card.Props.Property`.
- **Verification:** `classify` (`crates/nx-hir/src/visualizers.rs:166-168`) checks `property_target()` before the general union arm. `a_derived_declaration_is_not_a_target` now asserts the full `'<target>' cannot have a visualizer: <reason>` text for all six derived targets. On the scratch copy, removing the new arm makes exactly that test fail. nx-api's 437 lib tests pass, and fmt and clippy are clean.

### ✅ Verified - RF25 Prose about the removed features remains in doc comments, the design, tasks and the website
- **Severity:** Low (confidence: high)
- **Evidence:** The library and the code are clean, but these statements now describe things that no
  longer exist, or are false:
  - `crates/nx-language-service/src/source_tree.rs:259-261`: `SourceVisualizer` "holds the heading parts, the
    layout … for a union, every case once".
  - `source_tree.rs:365-366` (`source_tree_with_options`) and `:1868-1869` (`attach_visualizers`): the table
    "lists each union and alias with a visualizer". Only aliases are listed now.
  - `design.md:121-123`: the `properties` list carries the "resolved label, phrases, format and placement", and
    "For a union it lists every case". `design.md:129-132` still argues for not inheriting `name` ("A base
    called 'Question' …, which is the default default"), now about a field that is gone. `design.md:200-201`
    keeps the Open Question "Is a base's `name` not being inherited the right call?", which is moot.
  - `tasks.md:46-49`: the removed card parts "stay in the source tree for any reader". After 6.8 that is false.
    `tasks.md` 3.3 says "payload unions" (now every union), 4.1 says "the eight types" (now five), 5.4 says
    "list unions and aliases with visualizers", and 7.5 says the example has "a small record and union with
    visualizers". `visualizer-declarations.nx` has no union visualizer.
  - `sites/website/src/content/docs/reference/concepts/visualizer-declarations.md:3`: the page description
    says a visualizer declaration presents "a record, component, union or alias". Lines 173 and 189 offer
    visualizer modules for a team's "own wording", but nothing in the library words anything now. The same
    goes for the spec scenario "A host module rewords a library" (`specs/visualizer-declarations/spec.md:118`),
    which now changes an icon.
- **Recommendation:** Reword these to the five-type library: element visualizers carry title, id, id position,
  icon and per-property formats, and aliases carry a format. The table lists aliases only. Drop the `name`
  rationale and its Open Question. Fix tasks 3.3, 4.1, 5.4 and 7.5 and the note at `tasks.md:46-49`. Remove
  "union" from the page description. Replace "wording"/"rewords" with "presentation" or "overrides".
- **Fix:** Reworded the `SourceVisualizer`, `source_tree_with_options` and `attach_visualizers` doc comments (aliases only, title/id/icon/formats); design.md's resolution paragraph, dropped the `name` rationale and its Open Question; tasks 3.3, 4.1, 5.4, 7.5 and the section 6 note; the website page description ("record, action, component or alias") and "wording" to "presentation"; the spec scenario is now "A host module overrides a library". The proposal's motivation keeps "wording" as the original intent.
- **Verification:** Every place listed in the evidence is reworded, and the text is accurate for the five-type library. A rescan of the change artifacts, the language-service, nx-api and nx-hir sources, the website reference pages and the viewer README finds no remaining mention of union visualizers, cases, heading parts, layouts, labels, phrases or `name`. The remaining hits are the proposal's motivation, kept on purpose as the original intent, and the `rewording` test helper in `visualizer_declarations_tests.rs:541`. That helper now builds icon overrides; its name is a harmless nit. `openspec validate add-visualizer-declarations --strict` passes, and the website code-block check passes (214 blocks, 0 failures).

## New Findings Discovered During 2026-10-15 22:41 Review

**Scope:** the commits since RF22 was verified: 47ace9b (Source and Visual), 49d5f25 (rename to
visualizer declarations), 9239071, 7f1192e, 67fe892, 5988ad4 and 282737a (folding, chevrons, scroll
anchoring), 2c3920d (Visual first) and 20d8efe (the cut of `@nx/visualizer` to five types). Read
against every delta spec, `packages/viewer` (`index.ts`, `source.ts`), `sites/playground`
(`App.tsx`, `reading/ReadingView.tsx`, examples, README), `crates/nx-hir/src/visualizers.rs`,
`crates/nx-api/src/program_visualizers.rs`, `crates/nx-language-service` (`visualizers.rs`,
`source_tree.rs`, tests), `packages/language-protocol`, `nx-grammar.md` and the website pages.  
**Checks run:** `cargo test --workspace` (pass); `packages/viewer` `pnpm test` (32 pass);
`sites/playground` `pnpm test` (pass, 23 examples match); website `check-code-blocks.mjs` (214
blocks, 0 failures); `openspec validate add-visualizer-declarations --strict` (valid); jsdom probes
of `<nx-viewer>` against the built package; Playwright probes of the playground at `/play/`.

### ✅ Verified - RF26 A fold that hides the selection is undone at once, so a reader cannot fold a group holding the selected row, and Collapse all leaves the selection's containers open
- **Severity:** Medium (confidence: high, reproduced)
- **Evidence:** `#toggleFold`, `collapseAll` and the `folded` setter all end in `#render`
  (`packages/viewer/src/index.ts:307-333`), which re-selects the current selection with
  `#select(selection, false)`. `#select` (`:569-577`) treats any folded unit that holds the selection
  as "a node folded away" and unfolds it through `#setFolded`, firing `nx-fold` again. So:
  - jsdom, `folding` fixture: click the row `title` of the `Task` group, then the group's chevron.
    The group stays open and `folded` is `[]`.
  - `element.selection = <priority attribute>; element.collapseAll()` leaves `folded` as
    `[run…, "Task", "label"]`: neither `tasks` nor its group is folded.
  - Playground, "A full survey": click a row of `QuestionFlow`, then Collapse all. 11 of 12 groups
    fold; `let screener` and the `QuestionFlow` group stay open.
  - A host setting `folded` to keys that hide its own `selection` gets something other than "exactly
    those", against the viewer spec ("setting it SHALL fold exactly those").

  The rule the spec gives, "Selecting a node folded away SHALL unfold what hides it", is about the
  moment of selecting. It should not fight a fold the reader or host asks for later. The existing test
  (`test/element.test.ts:426-451`) selects the group itself before `collapseAll()`, and only checks
  `folded.length >= 3`, so it does not catch this.
- **Recommendation:** Only unfold what hides a node when that node is newly selected, through
  `selection` or a click/arrow, and not when `#render` re-applies an existing selection. When a fold
  hides the selected node, move the selection to the folded unit (as a tree view does) or clear it.
  Add tests: fold a group whose row is selected; `collapseAll()` with a row selected; set `folded` to
  keys that hide the selection.
- **Fix:** `#render` no longer reveals the selection: it re-selects it where it is still drawn, and otherwise selects the folded unit still drawn that holds it, with no event; `#select` takes a `reveal` flag. So a click, `collapseAll` and a host's `folded` all keep their folds. The viewer spec states it, with the scenario "Folding over the selection", and the test "keeps a fold that hides the selection, and selects what holds it" covers all three paths.
- **Verification:** `#render` now re-selects with `reveal=false`, and moves a hidden selection to the outermost drawn fold that holds it (`index.ts:324-340`); `#select` reveals only when asked. A jsdom probe on the `folding` fixture: clicking a row and then its group's chevron leaves `folded = ["tasks.value"]` with the group selected (`nx-select` and then `nx-fold`); `collapseAll()` with a row selected keeps every fold. The new test covers a click, `collapseAll` and the host's `folded`, and all 36 viewer tests pass. A fold by a run, which has no node to select, is RF34.

### ✅ Verified - RF27 An exported element function is counted as a supporting function and folded away; an abstract component ends a run
- **Severity:** Low (confidence: high, reproduced)
- **Evidence:** `supportingKind` (`packages/viewer/src/source.ts:106-110`) tells an element function
  by decoding the node's first 6 bytes and testing `startsWith("let <")`. A declaration with a
  visibility modifier starts with `export` or `private`, so `export let <Badge label:string /> = …`
  is counted as "1 function". Probe: `type A`, `type B`, `export let <Badge …/>`, `let v` gives the
  run `2 types · 1 function`. Without `export`, the same source gives `2 types`. The spec says
  element functions are not supporting declarations, so they show up in the content. The `external`
  test on components (`:103-105`) also makes a non-external `abstract component` (no body, nothing to
  draw) a primary declaration. It splits runs: `type A`, `abstract component <Q/>`, `type B` fold to
  nothing. The website's survey example writes `export abstract component <Question …/>` in this
  form.
- **Recommendation:** Tell an element function from the tree, with a flag or the function's
  declaration entry, not from its leading bytes. Decide whether an abstract component supports the
  document, as an external one does, and make the spec's "the components a host draws" say so. Add a
  case for each to the viewer's folding tests.
- **Fix:** `supportingKind` treats a component flagged `external` or `abstract` as supporting, and recognizes an element function by `^(?:(?:export|private)\s+)*let\s*<`, so modifiers no longer matter. The new `supporting` fixture and test check `1 type · 2 components` with `export let <Badge>` and `let <Card>` left open; the spec, README and doc comments say "components with no body of their own, external or abstract".
- **Verification:** `supportingKind` (`source.ts:103-113`) counts `external` or `abstract` components and matches `^(?:(?:export|private)\s+)*let\s*<` over 48 bytes, which covers both visibility modifiers the grammar has. The `supporting` fixture test checks `1 type · 2 components`, with `export let <Badge>` and `let <Card>` left open, and it passes.

### ✅ Verified - RF28 Back and Forward keep the folds of the document left behind, so the new one opens with nothing folded
- **Severity:** Low (confidence: high, reproduced)
- **Evidence:** `chooseExample` resets `folded` (`sites/playground/src/App.tsx:124`), but the
  popstate path (`:86-101`) only calls `setSource`. Playwright: open "A full survey", open its run, so
  `folded` holds the survey's keys, then press Back. The Hello example opens with
  `1 type · 1 function` unfolded and `type Greeting` showing, because its keys are not in the
  survey's set, and the defaults no longer apply once `folded` is a set. A shared link opened over a
  folded document does the same. The playground spec says what the reader folds is kept across Source
  and Visual and forgotten when they choose an example. It does not say a different document keeps
  them.
- **Recommendation:** Reset `folded` to `undefined` whenever the source is replaced from outside the
  editor (popstate and the first open), or key the kept folds by source.
- **Fix:** `App.tsx` resets `folded` to `undefined` whenever the address opens a document, on first load and on Back and Forward, as choosing an example does.
- **Verification:** `App.tsx` now calls `setFolded(undefined)` when the address opens a document. I re-ran the Playwright probe: opening the survey, opening its run and pressing Back now shows Hello with its own defaults (`run:Greeting` folded).

### ✅ Verified - RF29 Alt+click folds only the nested units that are drawn, so a group open inside an already-folded one stays open
- **Severity:** Low (confidence: medium, from reading the code)
- **Evidence:** A deep fold collects `this.#foldKeys(unit)` from the DOM
  (`packages/viewer/src/index.ts:389-392`). Units inside a folded child are not rendered, so they are
  not collected. Take a group with a folded child that holds an open grandchild: Alt+click on the
  outer group folds it and the child, and the grandchild comes back open when they are opened. The
  spec says an Alt+click SHALL fold "everything inside it as well". `collapseAll` avoids this by
  rendering everything unfolded first, and the deep fold could do the same. There is no test of
  Alt+click (`grep altKey packages/viewer/test` finds nothing).
- **Recommendation:** Collect the inner fold keys from the tree, or from an unfolded render as
  `collapseAll` does, not from the DOM. Add a test for a deep fold and a deep unfold.
- **Fix:** A deep fold first draws the unit open, with only the folds outside it kept, then collects every fold key inside it from that drawing, so groups nested in an already-folded unit fold too. The test "folds and opens everything inside a unit on an Alt+click" checks both directions.
- **Verification:** A deep fold now renders the unit fully open, with only the folds outside it kept, and collects every fold key inside it. The new Alt+click test on the `nesting` fixture checks that folding gives `root.body` and `root.body.items[0]`, and that unfolding gives `[]`. It passes.

### ✅ Verified - RF30 "The reading keeps every node" no longer holds once anything is folded, and the spec does not say so
- **Severity:** Low (confidence: high)
- **Evidence:** The viewer spec says the element "SHALL render every node of the tree once". With
  folding, which is on by default, a folded run, declaration or group's rows are not rendered at all.
  A folded declaration's first line is written as plain text (`source.ts:411-418`), so even the
  nodes on that line lose their keys. The coverage test calls `renderSource(document, index, [])`
  with no `folded` (`test/coverage.test.ts:24`), which turns runs and folding off, so it checks only
  the unfolded reading. The README says the same unqualified "The reading keeps every node".
- **Recommendation:** Scope the requirement and README to the unfolded reading, for example "with
  nothing folded, every node SHALL render once", and add a sentence that folded content is not
  rendered and is reached by selecting it. Alternatively, keep the keyed nodes of a folded
  declaration's first line.
- **Fix:** The requirement now reads "With nothing folded, the element SHALL render every node of the tree once…" and adds that folded content is not rendered and is reached by selecting it; the coverage scenario says "with nothing folded". The README says the same.
- **Verification:** The requirement and the coverage scenario say "with nothing folded" and add that folded content is not rendered, in both `openspec/specs/viewer/spec.md` and the archived delta; the README says the same.

### ✅ Verified - RF31 Stale references to the removed library parts remain in code comments and docs
- **Severity:** Low (confidence: high)
- **Evidence:**
  - `nx-grammar.md:349` still lists "constant union" as a target a visualizer declaration can have.
  - The website page's front matter (`sites/website/src/content/docs/reference/concepts/visualizer-declarations.md:3`)
    says a visualizer declaration presents "a record, component, union or alias".
  - `crates/nx-api/src/program_visualizers.rs:275-280` documents children as "`CaseVisualizer`s
    of a `UnionVisualizer`" and "a `PropertyVisualizer` or a `CaseVisualizer`".
  - `crates/nx-language-service/src/source_tree.rs:365` and `:1868` say the table lists "each union
    and alias with a visualizer", but only aliases are listed now.
  - `packages/viewer/src/index.ts:96-97` lists the element's properties as "`tree`, `text`, `colors`,
    `icons` and `selection`" and leaves out `folded`. The README's JSX declaration
    (`packages/viewer/README.md`, `Pick<NxViewerElement, "tree" | "text" | "colors" | "icons" |
    "stale" | "selection">`) leaves it out too.
  - `sites/playground/shots/final.png` (168 KB), added in 47ace9b, is a screenshot that nothing
    references.
- **Recommendation:** Update the grammar note, the page description and the four comments; add
  `folded` to the doc comment and the JSX `Pick`; delete `shots/final.png`, or ignore `shots/`.
- **Fix:** Dropped "constant union" from `nx-grammar.md`; the page description now says "a record, action, component or alias"; the `program_visualizers.rs` and `source_tree.rs` comments no longer mention case or union visualizers; `folded` is in the `NxViewerElement` doc comment and the README's JSX `Pick`; `sites/playground/shots/final.png` is deleted.
- **Verification:** `nx-grammar.md:348` no longer lists unions; the page description says "record, action, component or alias"; `program_visualizers.rs` and `source_tree.rs` no longer mention case or union visualizers; `folded` is in the doc comment and in the JSX `Pick`; `sites/playground/shots/` is gone.

### ✅ Verified - RF32 design.md and tasks.md still describe the parts the cut removed
- **Severity:** Low (confidence: high)
- **Evidence:**
  - design.md, Goals: "A lossless reading: a visualizer declaration can move a property into the
    folded details strip". There is no details strip or `place` now.
  - design.md, "Resolution happens in the compiler": `properties` carry "resolved label, phrases,
    format and placement", "For a union it lists every case", and "The table also lists unions and
    aliases".
  - design.md, the inheritance paragraph still argues for not inheriting `name` ("A base called
    'Question' should not make every derived kind read 'Question'…"), and the first Open Question
    asks about it. `name` is gone.
  - tasks.md 3.3 rejects "payload unions" (all unions are rejected now); 4.1 says "the eight types"
    (there are five); the section 6 preface says the card-era parts "stay in the source tree for any
    reader" (20d8efe removed them from it); 6.5 and 6.6 say "Read view"; 7.5 describes the example as
    "a small record and union with visualizers" (it has no union visualizer).
- **Recommendation:** Rewrite those design passages to the five-type library, drop the `name` open
  question, and correct tasks 3.3, 4.1, the section 6 preface, 6.5, 6.6 and 7.5.
- **Fix:** Most of this landed in the parallel round's RF25 (5205726, 396aadf), which this rebase keeps. On top of it: design.md drops the details-strip goal, and tasks.md 6.5, 6.6 and the section 6 preface name the reading the Visual view. The viewer spec's changes for RF26, RF27 and RF30 are applied both to the archived delta and to `openspec/specs/viewer/spec.md`.
- **Verification:** design.md has no details-strip goal, no union or label/phrase/placement prose and no `name` open question; tasks.md 3.3, 4.1, 6.5, 6.6, 7.5 and the section 6 preface match the five-type library and the Visual view.

### 🔴 Open - RF33 The new folding and Visual-first requirements have no tests at the playground level, and several viewer folding rules have none
- **Severity:** Low (confidence: high)
- **Evidence:** No playground test checks the following: that Visual is first and the default (spec
  "Reading an example"); that Collapse all and Expand all appear only in Visual; that folds are kept
  across Source and Visual and forgotten when an example is chosen. The only playground test of this
  area is `src/worker/session.test.mjs`, a worker query. In the viewer, nothing tests:
  - Alt+click (RF29);
  - that `if` and `for` blocks do not fold;
  - that setting `folded` to `undefined` goes back to the defaults;
  - that the chevron is drawn on folded units and on hover;
  - the scroll anchoring added in 282737a.

  RF26 also shows that the existing fold test misses the selection interplay.
- **Recommendation:** Add viewer tests for each rule listed. For the playground, add a test of the
  fold state's lifecycle and the default mode, either as a small React test or a Playwright check like
  the ones above.
- **Status:** Partly addressed. Viewer tests now cover Alt+click, `if` and `for` not folding, `folded = undefined` going back to the defaults, the chevron on folded units, the selection interplay (RF26) and the supporting-run rules (RF27). Left open: the playground has no React or DOM test setup, so the Visual-first default, the Collapse/Expand buttons and the fold lifecycle are still checked only in the browser; hover-only chevrons and scroll anchoring need layout, which jsdom does not do. Recommend a Playwright check for the playground once it has a browser test harness.
- **Verification:** The viewer half is done. There are new tests for Alt+click, `if` and `for` not folding, `folded = undefined` going back to the defaults, the chevron on folded units, the selection interplay and the supporting-run rules. They pass. It stays Open for the playground-level checks, as the status says.

## Summary (2026-10-15 22:41 review)
- The rename is complete in code, protocol, tests and docs. The only `viewer`/`…Viewer` names left are
  the tool's, and the historical sentence in design.md. The cut to five types is consistent between
  the library, the type checker, resolution, the source tree, the protocol and the website reference.
  The tests pass and the spec validates.
- One Medium behavior bug (RF26): a fold that hides the selection is reopened at once, which breaks
  folding a selected group's container and Collapse all whenever something is selected. The rest are
  Low: folding classification (RF27), fold state across navigation (RF28), deep fold (RF29), a spec
  statement folding broke (RF30), stale comments and docs (RF31, RF32), and missing tests (RF33).

## New Findings Discovered During 2026-10-15 23:05 Verification

### 🟡 Fixed - RF34 Folding a run over the selection clears it silently, rather than selecting the run as the spec says, and the host is not told
- **Severity:** Low (confidence: high, reproduced)
- **Evidence:** The RF26 fix moves a hidden selection to a folded unit only when that unit is in
  `#rendered` (`packages/viewer/src/index.ts:332-338`). A run's fold key (`run:…`) is not a node and
  has no `data-key`, so it is never in `#rendered`. jsdom probe, `folding` fixture, with
  `selection = "Task.priority"`:
  - clicking the run's chevron gives `folded = ["run:…"]` and `selection = undefined`;
  - `collapseAll()` gives the same.

  In both cases only `nx-fold` fires, with no `nx-select`. The spec ("folding a unit that holds the
  selection … SHALL … select the outermost folded unit still drawn that holds the selection") calls
  for the run to be selected, or at least for the change to be reported. The playground's
  `selection` state keeps the hidden key. Switching to Source then compares it with
  `selectedOnEntry` and can put the cursor on a node the reader no longer sees selected.
- **Recommendation:** Either make a folded run selectable (a key the host can hold, with its head as a
  unit), or say in the spec that a fold with no node clears the selection, and fire `nx-select`, or a
  documented clear, so the host can follow. Add the run case to "keeps a fold that hides the
  selection".
- **Fix:** I took the spec route, since a run has no node key to select. A fold that a run hides the selection in clears it, and `NxFoldDetail` now carries `selection`, the selection after the fold (the holder, or `undefined`), so every fold tells the host where the selection is. The playground's `ReadingView` passes it to `onFold`, and `App.tsx` sets its `selection` from it. Both viewer specs (the archived delta and `openspec/specs/viewer/spec.md`) say so, with the scenario "Folding a run over the selection", and so does the README. The test "clears a selection a folded run hides, and tells the host" covers a run click and the `collapseAll` holder case.
