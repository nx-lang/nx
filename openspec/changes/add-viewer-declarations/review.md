# Review: add-viewer-declarations

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/viewer-declarations, specs/viewer-library,
specs/component-props-type, specs/record-type-parameters, specs/property-references, specs/update-records,
specs/component-syntax, specs/source-tree, specs/viewer, specs/playground, specs/cli-code-generation,
specs/editor-syntax-highlighting  
**Reviewed code:** every commit in `origin/main..HEAD`: `crates/nx-syntax` (grammar, ast), `crates/nx-hir`
(`lower.rs`, `records.rs`, `viewers.rs`, `lib.rs`, `components.rs`), `crates/nx-types` (`infer.rs`, `check.rs`,
tests), `crates/nx-api` (`program_viewers.rs`, `artifacts.rs`, `std/viewer/viewer.nx`, tests),
`crates/nx-codegen/src/builder.rs`, `crates/nx-cli/src/typegen`, `crates/nx-language-service` (`viewers.rs`,
`source_tree.rs`, tests), wasm and Node bindings, `packages/language-protocol`, `language-core`,
`language-http`, `packages/viewer` (`render.ts`, `index.ts`, `hover.ts`, `styles.ts`, tests),
`sites/playground`, `src/vscode` grammar, website docs.  
**Checks run:** `cargo test` for nx-syntax, nx-hir, nx-types, nx-api, nx-codegen, nx-language-service and
nx-cli (all pass); `packages/viewer` `pnpm test` (93 pass); `src/vscode` grammar tests (287 pass); probes
through the wasm SDK's language snapshot and `<nx-viewer>` under jsdom, and the built `nxlang typegen`.

## Findings

### ✅ Verified - RF1 Viewer declarations that fail type checking still apply, so type-invalid values reach the source tree
- **Severity:** Medium (confidence: high, reproduced)
- **Evidence:** `program_viewers` (`crates/nx-api/src/program_viewers.rs:130`) keeps every viewer declaration
  whose target resolves and whose tag *spelling* matches the record, and `read_viewer_value` /
  `attribute_value` (`:308`, `:370`) read the attributes syntactically, so a declaration with type errors
  still contributes whatever it wrote. Probes:
  - A host module with `viewer Choice = <ElementViewer name="Host" title=lable><PropertyViewer of=valu … />`
    (two `unresolved-contextual-name` errors in `host.nx`) still overrides the library's `name="Lib"`:
    the resolved viewer reads `name: "Host"`. The spec says a host viewer module that does not analyze
    cleanly "SHALL contribute the viewer declarations that do" (viewer-declarations, "gathered from the
    program and from host modules"), i.e. not the ones that don't.
  - `viewer Choice = <ElementViewer layout="row bogus" title="label"><PropertyViewer of="value" format="image" /></ElementViewer>`
    reports four `record-field-type-mismatch` errors, yet the answer carries
    `{"title":"label","layout":"row bogus","properties":[{"name":"value","format":"image",…}]}`. The
    `layout`, `place` and `format` strings are passed through unvalidated
    (`crates/nx-language-service/src/source_tree.rs:2001`, `:2020`), violating the protocol's
    `SourceViewerLayout` / `SourceValueFormat` / `"body" | "details"` unions; the viewer then treats
    `"row bogus"` as a non-card layout and folds every row into the details strip.
- **Recommendation:** Only gather viewer declarations that type-checked cleanly. For example, have
  `infer_viewer` record a per-declaration "clean" flag (no error diagnostic within `viewer.span`) on the
  module artifact, or filter in `program_viewers` by the module's error diagnostics whose primary span lies
  inside the viewer declaration. Independently, read case-typed attributes (`layout`, `place`, `format`,
  `of`, `title`…) only from `Expr::ResolvedUnionCase` (not string literals), and validate `layout`/`place`/
  `format` against their known cases before emitting them. Add tests: a host module with a misspelled
  property does not override the library's name; a quoted `layout="row"` is not applied.
- **Fix:** `program_viewers` (`crates/nx-api/src/program_viewers.rs`) now leaves out, whole, any viewer
  declaration with an error diagnostic whose primary label lies inside its span (`has_errors_within`;
  `duplicate-viewer` is excluded so duplicate detection does not feed back on itself). As defense in depth,
  `ResolvedViewer::apply` and `merge_child` (`crates/nx-language-service/src/viewers.rs`) drop any `layout`,
  `place` or `format` value outside the protocol's unions (`is_known_value`). Test
  `a_viewer_declaration_with_an_error_does_not_apply` covers a host module with `title=lable` (the library's
  name stands) and a quoted `layout="inline"` / `format="image"` (neither applies).
- **Verification:** Re-ran the original probes against the rebuilt wasm. The host module with `title=lable` / `of=valu` no longer overrides the library: `name` stays "Lib". The quoted `layout="row bogus"` / `format="image"` declaration and the one with an unknown `whatever` attribute now contribute nothing. An error elsewhere in the module, outside the viewer's span, does not drop the viewer. `is_known_value` also filters `layout`, `place` and `format` in `apply` and `merge_child`. `a_viewer_declaration_with_an_error_does_not_apply` passes. Applying the rule before duplicate detection opens a new gap, filed as RF8.

### ✅ Verified - RF2 An unknown format makes the viewer render a string literal as an empty span, losing its text
- **Severity:** Low (confidence: high, reproduced)
- **Evidence:** `formatted()` (`packages/viewer/src/render.ts:855`) switches over the known formats with no
  default branch, so any other `format` value leaves the marked span empty. With
  `<PropertyViewer of=value format="bogus" />` (see RF1) and `<Choice value="SECRET-TEXT" />`, the rendered
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
  `viewer Tag = <ElementViewer layout=inline title=name subtitle=note badge=code />` renders an
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

### ✅ Verified - RF5 A second `PropertyViewer` for the same property in one viewer declaration is silently dropped
- **Severity:** Low (confidence: high, reproduced)
- **Evidence:** In `ResolvedViewer::apply` (`crates/nx-language-service/src/viewers.rs:203`–`:220`), the
  second child with the same `of` starts from an empty `ResolvedChild` (the first was already moved out of
  `earlier`) and is then discarded by the `!self.properties.iter().any(…)` guard. With
  `<PropertyViewer of=label label="First" /><PropertyViewer of=label place=details />`, the resolved viewer
  has `label: "First"` and `place: "body"`; the `place=details` is lost, and analysis reports nothing.
- **Recommendation:** Either merge the later child into the entry already in `self.properties`
  (consistent with "merge by `of`, attribute by attribute") or report a diagnostic for a repeated `of`
  within one viewer declaration. The same applies to repeated `CaseViewer`s, which currently merge.
- **Fix:** In `ResolvedViewer::apply`, a later `PropertyViewer` whose `of` is already in the resolved
  properties merges into that entry, attribute by attribute, as `CaseViewer`s do. Test
  `two_property_viewers_of_one_property_in_one_viewer_merge` asserts `label: "First"` and `place: details`.
- **Verification:** Re-ran the probe. `<PropertyViewer of=label label="First" /><PropertyViewer of=label place=details />` now resolves to `label: "First", place: "details"`, and the entry is listed once at the first one's position. `two_property_viewers_of_one_property_in_one_viewer_merge` passes.

### ✅ Verified - RF6 Forgetting `import "@nx/viewer"` gives a self-contradictory diagnostic
- **Severity:** Low (confidence: high, reproduced)
- **Evidence:** Without the import, `viewer Choice = <ElementViewer title=label />` reports only
  `viewer-record-mismatch`: "The viewer of record 'Choice' must be a 'ElementViewer' from "@nx/viewer", not
  'ElementViewer'" (`crates/nx-types/src/infer.rs:1558`). The tag falls back to an unknown host element whose
  type prints as the same name, so the message names the right record as the wrong one and does not say
  what to do. This is likely the most common first mistake (the docs say the module "must
  `import "@nx/viewer"`").
- **Recommendation:** When the value's type has no origin in `@nx/viewer` but its name equals the expected
  record, say that the record is not imported and suggest `import "@nx/viewer"`. Also fix the article
  ("an 'ElementViewer'").
- **Fix:** When the written record's name equals the expected one, the `viewer-record-mismatch` message
  (`crates/nx-types/src/infer.rs`) now reads "must be an 'ElementViewer' from "@nx/viewer", and this
  'ElementViewer' is not that one", followed by "; add `import "@nx/viewer"`" when the module does not
  import the library; the article is chosen by the name (`article`). Test
  `a_viewer_without_the_library_imported_says_to_import_it` asserts the exact message.
- **Verification:** Re-ran the probe without the import. It now reads: "The viewer of record 'Choice' must be an 'ElementViewer' from "@nx/viewer", and this 'ElementViewer' is not that one; add `import "@nx/viewer"`". The component case reads the same way. The article is right, and the import hint is given only when the import is missing.

### ✅ Verified - RF7 The "union a property uses comes along" test does not exercise the case it is named for
- **Severity:** Low (confidence: medium)
- **Evidence:** The source-tree scenario requires `ChoiceLayout` to be listed when "no node of the document
  names `ChoiceLayout`". `the_union_a_property_uses_comes_along`
  (`crates/nx-language-service/src/source_tree_tests.rs:1538`) uses `SURVEY_FORM`, which writes
  `layout=chips`, a `case` node that already brings `ChoiceLayout` into the table. The property-type path in
  `attach_viewers` is covered only for the alias (`ImageUrl`).
- **Recommendation:** Use a form that leaves `layout` unset (as `the_base_name_is_not_inherited` does with
  `MultiChoice`) and assert `ChoiceLayout` still carries its case viewers.
- **Fix:** `the_union_a_property_uses_comes_along` now uses a form that leaves `layout` unset, asserts no
  node names `ChoiceLayout`, and checks `ChoiceLayout` still carries its three case viewers.
- **Verification:** The test now uses a `SingleChoice` that leaves `layout` unset. It asserts that no node refers to `ChoiceLayout` and that `ChoiceLayout` still carries its three case viewers, so it exercises the property-type path in `attach_viewers`. It passes.

## New Findings Discovered During 2026-10-12 18:40 Verification

### ✅ Verified - RF8 Dropping an erroneous viewer declaration before duplicate detection lets a reported duplicate apply, and hides a cross-module duplicate
- **Severity:** Low (confidence: high, reproduced)
- **Evidence:** In `program_viewers` (`crates/nx-api/src/program_viewers.rs`), `has_errors_within` is checked
  before `targets.insert(...)`, and the cross-module duplicate check (`duplicate_program_viewers`) runs over
  that filtered list. RF1's fix introduced this. Two probes:
  - **Same module:** `viewer Choice = <ElementViewer name="One" title=lable />` followed by
    `viewer Choice = <ElementViewer name="Two" />`. Analysis reports `duplicate-viewer` at the second
    declaration, yet the resolved viewer has `name: "Two"`. The skipped first declaration never claimed the
    target, so the second one applies even though it carries an error. That contradicts the spec ("a
    duplicate-viewer error at the second") and the function's own doc comment ("So is the second ... left
    out"). It also breaks RF1's answer that a declaration with any error contributes nothing.
  - **Across modules:** two modules each declare `viewer Choice`, and the one in `a.nx` has `title=lable`.
    The program reports no `duplicate-viewer`, and the resolved viewer takes `b.nx`'s `name: "B"`. The spec
    says two program modules that both declare a viewer for a target are a `duplicate-viewer` error and
    neither applies. Fixing the typo in `a.nx` would then make a duplicate error appear and remove the
    viewer the reader had been seeing.
- **Recommendation:** Run duplicate detection over every viewer declaration whose target resolves and
  whose record matches, before the error filter. In one module, call `targets.insert` before checking
  `has_errors_within`. Across modules, compute duplicates on the unfiltered list. After that, leave out
  the declarations that have errors and both members of any duplicate. Add tests for both probes.
- **Fix:** In `program_viewers`, the first viewer declaration whose target resolves now claims the target
  before anything else is checked, as analysis decides, so a second one in the module never applies. A
  declaration with errors is kept with `clean: false` (new `ProgramViewer::clean`), so
  `duplicate_program_viewers` and the `duplicate-viewer` diagnostic count it; `ProgramViewers::gather`
  applies only clean declarations, after removing duplicates, for the program and for host modules. Tests
  `the_second_of_two_viewers_in_one_module_does_not_apply_when_the_first_has_errors` and
  `a_duplicate_with_errors_still_conflicts` fail without the fix.
- **Verification:** Rebuilt the wasm from f55faf4 and re-ran both probes. In the same-module probe, the first declaration (with `title=lable`) now claims the target. The second is reported as `duplicate-viewer`, and `Choice` has no viewer. In the cross-module probe, `duplicate-viewer` is now reported at `main.nx` for `t/a.nx` and `t/b.nx` with an error in `a.nx`, and neither applies. The same holds when `a.nx` lacks `import "@nx/viewer"` or writes a computed `name={p}`. RF1 still holds: a host module with misspelled properties leaves the library's `name` "Lib" in place; a declaration with quoted case values, an unknown attribute or a repeated `title` contributes nothing; a clean viewer next to an unrelated error in its module still applies. Host modules filter by `clean` too. The viewer tests pass (94/94).

## Questions
- RF1: should a viewer declaration with errors contribute nothing (all-or-nothing per declaration), or
  contribute the attributes that checked? The spec text reads as all-or-nothing per declaration; the
  recommendation assumes that.
  - **Answer:** All-or-nothing per declaration: a viewer declaration with any error contributes nothing
    (RF1's fix).

## Summary
- The change is broad and largely solid: grammar, lowering, `T.Props`, `P.Property`, `Target` binding,
  resolution order and merging, duplicate reporting, the protocol plumbing (wasm, Node, language-core,
  HTTP validation), the playground request and the docs match the specs, and all touched test suites pass.
  The viewer's handling of untrusted source holds up: links and images are limited to `http:`/`https:`
  (with `noopener noreferrer`, `no-referrer`, lazy loading and a host off switch), `tel:` keeps digits
  only, markdown renders no links, and colours go through the CSSOM `backgroundColor` setter, which cannot
  trigger a fetch.
- The main gap is RF1: viewer declarations are read syntactically and applied even when they fail to
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

### 🟡 Fixed - RF10 Wildcard imports still bind `<Name>.State` and the other derived names when `<Name>` itself is taken
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
