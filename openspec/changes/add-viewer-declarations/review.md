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

### 🟡 Fixed - RF8 Dropping an erroneous viewer declaration before duplicate detection lets a reported duplicate apply, and hides a cross-module duplicate
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
