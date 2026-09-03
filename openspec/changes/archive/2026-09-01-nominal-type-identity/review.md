# Review: nominal-type-identity

## Scope
**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md`, and all five delta specs  
**Reviewed code:** all implementation files in the working-tree diff across `nx-hir`, `nx-types`,
`nx-codegen`, `nx-interpreter`, `nx-api`, `nx-language-service`, and the CLI adapters  

## Findings

### ✅ Verified - RF1 Resolved union operations can still be captured by a same-named visible union
- **Severity:** High
- **Evidence:** `Type::Union` and `Type::UnionCase` carry an origin, but union-to-record
  compatibility discards it and calls `union_type_satisfies_record` with only the display name
  (`crates/nx-types/src/infer.rs:2845` and `crates/nx-types/src/infer.rs:2848`). That helper then
  selects `union_defs` by name before consulting `foreign_union_defs`
  (`crates/nx-types/src/infer.rs:2768`). Consequently, when a foreign union and a visible local
  union share a name, checking whether the foreign union/case satisfies an abstract record base can
  use the local union's base and return the wrong result. The same origin loss remains in member
  lookup (`crates/nx-types/src/infer.rs:943`, `crates/nx-types/src/infer.rs:968`, and
  `crates/nx-types/src/infer.rs:990`) and common-supertype construction
  (`crates/nx-types/src/infer.rs:2893`), which reselect union metadata by spelling. This conflicts
  with the requirement that a nominal type's name is display information and that a visible
  same-named declaration cannot capture a resolved reference. Existing tests cover union-to-union
  equality and runtime base construction, but not these post-resolution static paths.
- **Recommendation:** Pass the resolved `UnionType`/`UnionCaseType` (or its `DeclaringOrigin`) into
  base, field, and common-supertype lookup; select `UnionEntry` by origin and resolve its base and
  field types in the union's declaring module. Add collision tests for foreign union/case-to-record
  compatibility and foreign union member access with a same-named local union.
- **Fix:** Confirmed as reported and fixed as recommended, recorded as design D7 and tasks 11.1-11.6.
  A single selector `InferenceContext::union_entry_for(name, origin)` now answers every
  post-resolution lookup — `union_type_satisfies_record`, `union_shared_field_type`,
  `union_has_case_field`, `union_case_field_type`, and the `common_supertype` union-case arm — trying
  the name as a fast path and falling back to a scan of `union_defs` and `foreign_union_defs` by
  declaration. The union's base is resolved through its own declaring origin and each inherited
  field's type through `EffectiveField::module_identity`.

  Building the test vehicle surfaced two further leaks of the same D3 rule, fixed with it: a union
  case's own field types were resolved in the consumer rather than in the union's module when
  checking `<Union.case ... />`, and an imported value's type annotation was resolved in the
  consumer's namespace (`register_value_bindings`, both arms) — the latter being why the collision
  was unreachable from a test at all, since the value carrying a foreign union across the boundary
  was re-typed on arrival.

  Four collision tests added in `crates/nx-api/src/artifacts.rs`:
  `a_foreign_unions_base_is_read_from_its_own_declaration`,
  `a_same_named_local_unions_base_does_not_answer_for_a_foreign_union`,
  `a_shared_field_on_a_foreign_union_is_read_from_its_own_base`, and
  `a_foreign_union_cases_field_types_resolve_in_its_own_module`. Each was verified to fail with the
  selector reverted to name-only lookup, and the fourth with the case-field resolution reverted.
  Full workspace suite green (47 suites), corpus IR unchanged for all 13 examples, `--strict` valid.
- **Verification:** Reopened. The new lookup paths preserve origin when the requested declaration
  remains cached, and all four added regression tests pass. However, `foreign_union_defs` is still
  `FxHashMap<Name, UnionEntry>` (`crates/nx-types/src/infer.rs:157`), and every foreign union
  resolution inserts by display name (`crates/nx-types/src/infer.rs:1626`). If a consumer receives
  values typed by two different foreign unions both named `Shape`, registering the second replaces
  the first. The two values retain distinct origins, but `union_entry_for` can scan only the one
  surviving entry, so post-resolution lookup for the overwritten declaration still fails. Store
  foreign entries by origin (or otherwise retain every same-named declaration) and add a test using
  two foreign same-named unions in the same consumer.
- **Fix (reopening):** Confirmed and fixed as recommended. `foreign_union_defs` is now keyed by
  `DeclaringOrigin` rather than by the name a union arrived under, and `union_entry_for` addresses a
  foreign entry by origin outright instead of by spelling. That key was never a namespace answer —
  nothing in a consumer names these unions — so keying it by declaration is the question the map
  actually answers; `union_defs` keeps its name key, which is what a name key is for. The last
  name-keyed reach into the foreign map, in `resolve_contextual_name_in`, now goes through
  `union_entry_for` too, which also removed a duplicated `same_union` closure.
  New test `two_foreign_unions_sharing_a_name_stay_distinct` in `crates/nx-api/src/artifacts.rs`
  gives one consumer values from two peers that each export a `Shape` over a base of their own; it
  fails with `"Property 'd' on 'Ink' expects Drawable, found Shape"` when the cache is reverted to
  name keying. Recorded as design D7 (amended) and tasks 12.1, 12.2, 12.4.
- **Verification:** Verified. `foreign_union_defs` is keyed and addressed by `DeclaringOrigin`, so
  same-named foreign declarations coexist, and the contextual-name path also uses
  `union_entry_for`. The targeted `two_foreign_unions_sharing_a_name_stay_distinct` regression and
  the full workspace suite pass.

## New Findings Discovered During 2026-08-31 23:28 Verification

### ✅ Verified - RF2 Imported function signatures resolve nominal types in the consumer's namespace
- **Severity:** High
- **Evidence:** The fix correctly changed imported value annotations to use
  `type_from_type_ref_in`, but `register_function_signatures` still resolves a peer function's
  annotated return through `type_from_type_ref` (`crates/nx-types/src/infer.rs:2527`) and resolves
  library-interface parameters and returns the same way (`crates/nx-types/src/infer.rs:2548`).
  `bind_function_signature_from_parts` also resolves raw function parameter annotations in the
  current inference context. Therefore an imported function declared with parameter or return type
  `Shape` can silently acquire the consumer's unrelated `Shape`, violating the requirement that a
  declaration's type references resolve in the namespace of the module that wrote them.
- **Recommendation:** For non-local raw functions and imported interface functions, resolve every
  parameter and return annotation with `type_from_type_ref_in(Some(declaring_module), ...)`. Add
  workspace-peer and library tests covering same-named nominal parameter and return types.
- **Fix:** Confirmed and fixed as recommended. `register_function_signatures` takes
  `resolved.module_identity()` the way `register_value_bindings` does and resolves the raw arm's
  return annotation and the interface arm's parameters and return with `type_from_type_ref_in`;
  `bind_function_signature_from_parts` gained a `declaring_module` parameter and resolves each
  parameter annotation there, with the local caller passing `None`. `nominal_type_in_module` returns
  early for this module, so a local function resolves exactly as before.
  Four tests in `crates/nx-api/src/artifacts.rs`:
  `a_peer_functions_parameter_type_resolves_in_its_own_module`,
  `a_peer_functions_return_type_resolves_in_its_own_module`,
  `a_library_functions_parameter_type_resolves_in_its_own_module`, and
  `a_library_functions_return_type_resolves_in_its_own_module`. Reverting only the parameter
  resolution fails both parameter tests (`"Argument 0 expects app.nx:Shape, found
  widgets.nx:Shape"`); reverting only the return resolution fails both return tests
  (`"Property 'fit' on 'Img' expects .../ui/widgets.nx:Fit, found .../app/main.nx:Fit"`).
  Recorded as design D7 (amended) and task 12.3.
- **Verification:** Verified. Raw peer and library-interface parameter and return annotations now
  resolve with the declaring module, while the local binding path retains local resolution. All four
  targeted peer/library parameter/return tests and the full workspace suite pass.

## Questions
- None

## Summary
- RF1 and RF2 are verified. Foreign union metadata remains distinct by declaring origin, and
  imported function signatures resolve nominal annotations where they were declared. No review
  findings remain open.

### Fix pass
- RF1 fixed; awaiting verification by the reviewing agent. No findings left open.
- Also fixed while re-checking the change as a whole: `PreparedSourceFile::Prepared`, an enum this
  change introduced, tripped `clippy::large_enum_variant` (464 vs 168 bytes). Its module is now
  boxed. Clippy across `nx-hir`, `nx-types`, `nx-interpreter`, `nx-api`, `nx-codegen`,
  `nx-language-service`, and `nx-cli` was re-diffed against the pre-change tree: no new findings,
  one reduction.

### Verification pass
- RF1 reopened because the name-keyed foreign union cache cannot retain two same-named origins.
- RF2 added for imported function parameter and return annotations resolving in the consumer.
- Full Rust workspace suite, VS Code grammar suite (82 tests), strict OpenSpec validation, and
  `git diff --check` pass.

### Second fix pass
- RF1 (reopening) and RF2 both fixed; awaiting verification by the reviewing agent. No findings left
  open.
- Verified: full workspace suite green (47 suites, 0 failures); corpus IR matches the baseline for
  all 13 examples; `openspec validate --strict` passes. `cargo fmt --all -- --check` reports only
  the four pre-existing `crates/nx-types/tests/contextual_literals.rs` diffs. Clippy on `nx-hir`,
  `nx-types`, `nx-interpreter`, `nx-api`, `nx-codegen`, `nx-language-service`, and `nx-cli` was
  re-diffed against a stashed pre-change tree: no new findings in any touched crate, and one
  removal in `nx-types`.
- Each of the five new tests was checked to fail with its own fix reverted, and no other.
- Observed while building the RF1 test, out of scope and not fixed: element properties are not
  checked at all on an element nested inside another element's content — `<div><Ink d="nonsense"
  /></div>` produces no diagnostic, because `infer_element_expression` falls through to
  `nominal_named_type` for an unrecognized tag without ever inferring `element.content`. This
  predates the change (nothing here touches that fallthrough) and is unrelated to nominal identity,
  but it silently weakens any test that puts its subject inside a host element's body. Worth its
  own change.

### Third verification pass
- RF1 and RF2 verified; no findings reopened and no new findings added.
- The five targeted regressions, full Rust workspace suite, VS Code grammar suite (82 tests), strict
  OpenSpec validation, and scoped diff whitespace check pass.
- A repository-wide `git diff --check` remains affected only by unrelated trailing whitespace in
  `docs/NX-Drawn-UI-MVP-Object-Model-Proposal.md`; all files in this change's implementation and
  artifacts pass the check.

## New Findings Discovered During 2026-09-01 00:01 Review

Independent second-agent pass over all artifacts and the full working-tree diff. The two findings
above were re-read but not re-verified from scratch; everything below is new.

### ✅ Verified - RF3 A foreign contract typed by a *type alias* still resolves in the consumer's namespace
- **Severity:** High
- **Evidence:** `InferenceContext::foreign_nominal_type`
  (`crates/nx-types/src/infer.rs:1613-1636`) matches only `Item::Union`, `Item::Record`, and
  `Item::Component`; an `Item::TypeAlias` returns `None`. `nominal_type_in_module` therefore
  returns `None` for a declaration whose property is typed by an alias the declaring module wrote,
  and `type_from_type_ref_in` (`crates/nx-types/src/infer.rs:1656`) falls back to
  `resolve_named_type` — the consumer's namespace. That is exactly the D3 hole, one indirection
  further out, and it is silent. Reproduced against a two-module workspace:

  ```nx
  // widgets.nx
  export type Fit = fill | contain | cover
  type FitAlias = Fit
  export let <Img fit: FitAlias = {Fit.fill} /> = <div class="img" />

  // app.nx
  import { Img } from "./widgets.nx"
  type FitAlias = stretch | squish
  let root() = { <Img fit=stretch /> }
  ```

  `nxlang codegen --target nx-ir` accepts this with **zero diagnostics** and emits
  `"caseName": "stretch"` into the IR, so a case of the consumer's union reaches a property the
  declaring module typed `fill | contain | cover`. The same source with `fit: Fit` written directly
  is correctly rejected (`'stretch' is not a case of union 'Fit' Cases: fill, contain, cover`), which
  isolates the alias as the cause. This violates the ADDED requirement *A contract resolves in the
  namespace of the module that wrote it* — an alias name is a type reference the declaring module
  wrote — and no spec scenario or test covers an aliased contract.
- **Recommendation:** Follow `Item::TypeAlias` in `foreign_nominal_type`: resolve the alias target
  recursively in the declaring module (`nominal_type_in_module` with a `seen` guard, mirroring
  `resolve_record_definition_inner`'s alias handling), and only then fall back. Add a scenario to
  `specs/nominal-type-identity/spec.md` under *A contract resolves in the namespace of the module
  that wrote it* for a contract typed by an alias, plus record and union variants of the test above.
- **Fix:** Confirmed as reported — reproduced verbatim, with `stretch` reaching the IR and no
  diagnostic anywhere — and fixed as recommended. `foreign_nominal_type` now follows
  `Item::TypeAlias`, resolving the alias target in the declaring module through
  `type_from_type_ref_in` recursively, guarded by a `foreign_alias_stack` of the alias declarations
  currently being followed so a cycle among them terminates (the declaring module reports its own
  cycle; the guard only stops the consumer following one forever). Following the alias also fixes
  the legitimate direction, which was broken the other way: with no same-named alias in the
  consumer the reference resolved to nothing, and `<Img fit={Fit.cover} />` was rejected with
  `"Property 'fit' on 'Img' expects FitAlias, found Fit.cover"`.
  Three tests in `crates/nx-api/src/artifacts.rs`:
  `a_foreign_contract_typed_by_an_alias_resolves_in_its_own_module`,
  `a_foreign_contract_typed_by_an_alias_to_a_record_resolves_in_its_own_module`, and
  `an_aliased_contract_still_accepts_the_type_it_names`. All three fail with the alias arm reverted.
  Recorded as design D3 (amended), tasks 13.1-13.2, and a new spec scenario *A contract typed by an
  alias denotes what the alias names in its own module*.
- **Verification:** Verified. `foreign_nominal_type` now has an `Item::TypeAlias` arm resolving the
  alias target through `type_from_type_ref_in` in the declaring module, guarded by a
  `foreign_alias_stack` of origins (`crates/nx-types/src/infer.rs:1648-1656`). The original repro was
  re-run verbatim: it is now rejected with `'stretch' is not a case of union 'Fit' Cases: fill,
  contain, cover`, where before it produced zero diagnostics and emitted `stretch` into the IR. The
  legitimate direction was re-run too — `import { Img, Fit }` with `<Img fit={Fit.cover} />` through
  the same alias — and generates cleanly. The cycle guard was probed with a mutually recursive alias
  pair across modules: it terminates and reports the declaring module's own `Type alias 'A' forms a
  cycle`, and the diagnostic's triplication there is pre-existing (a local-only alias cycle
  reproduces it identically). All three named tests exist at
  `crates/nx-api/src/artifacts.rs:4198`, `:4232`, and `:4268`, and the spec scenario *A contract
  typed by an alias denotes what the alias names in its own module* is present.

### ✅ Verified - RF4 Same-name mismatches are disambiguated only when both sides render to the identical string
- **Severity:** Medium
- **Evidence:** `display_type_pair` (`crates/nx-types/src/ty.rs:424-430`) returns the unqualified
  pair whenever `plain_lhs != plain_rhs`. A `Type::Union` renders as `Fit` and a `Type::UnionCase`
  as `Fit.cover`, so the qualified-form collision is never qualified. Reproduced on the workspace
  from the proposal's own opening example (consumer declares its own `Fit`,
  `<Img fit={Fit.cover} />`):

  ```
  error: Property 'fit' on 'Img' expects Fit, found Fit.cover
  ```

  which reads as though a case of the *expected* union were being rejected. The bare-name sibling
  (`<Img fit=stretch />` against a local `type Fit = stretch | squish`) reports
  `'stretch' is not a case of union 'Fit' Cases: fill, contain, cover`
  (`report_unknown_union_case`, `crates/nx-types/src/infer.rs:2730`), which names one `Fit` while
  the author is looking at another that does declare `stretch`. Both are the enum-values delta
  scenario *A same-named local union does not stand in for the declaring module's union*, whose
  third bullet requires the diagnostic to "distinguish the two unions by their declaring modules".
  Only the `Union`/`Union` shape is covered, by
  `a_same_name_mismatch_names_the_declaring_modules`.
- **Recommendation:** Decide the qualification on the *nominal parts* rather than on the rendered
  strings: qualify when any two nominal types in the message share a display name but not an
  origin, so `Union Fit` vs `UnionCase Fit.cover` qualifies both. Give
  `report_unknown_union_case` the union's origin so it can name the declaring module when a
  same-named union is visible at the use site. Add tests for both shapes.
- **Fix:** Confirmed and fixed as recommended, both halves. `display_type_pair` now decides on the
  nominal parts rather than on the rendered strings: each side contributes the declarations it
  names — a union case contributing its *union's* name, which is the name the reader has to tell
  apart — and the pair is qualified when one display name covers two declarations. `Union Fit`
  against `UnionCase Fit.cover` therefore qualifies, and `qualified_display` gained a function arm
  so a collision found inside a function type is rendered as one. For the single-type messages, a
  new `InferenceContext::display_union_name` qualifies a union's name when a *different* union of
  that name is visible at the use site and leaves it bare otherwise; `report_unknown_union_case`
  takes the `UnionType` (so it has the origin) and the bare contextual-name message uses it too.
  The workspace now reports `expects widgets.nx:Fit, found app.nx:Fit.cover` and `'stretch' is not
  a case of union 'widgets.nx:Fit'`. Four existing tests pinned the unqualified spelling and were
  updated to the qualified one — that is precisely what each was asserting about.
  New test `a_qualified_form_mismatch_names_the_declaring_modules`; it and the headline test fail
  with the rendered-string rule restored, and the four updated ones fail with `display_union_name`
  reverted. Recorded as design D5 (amended), tasks 13.3-13.4, and two new spec scenarios.
- **Verification:** Verified, both halves, empirically. `display_type_pair` now decides on nominal
  parts via `nominal_parts_collide`/`collect_nominal_parts`, with a union case contributing its
  union's name (`crates/nx-types/src/ty.rs:428-467`), and `qualified_display` covers array,
  nullable, and function positions. The qualified-form repro now reports `Property 'fit' on 'Img'
  expects widgets.nx:Fit, found app.nx:Fit.cover`, and the bare-name repro `'stretch' is not a case
  of union 'widgets.nx:Fit'`. `display_union_name`
  (`crates/nx-types/src/infer.rs:2775-2788`) qualifies only against a *different* declaration of the
  same name, so a message with nothing to disambiguate stays bare — confirmed on the RF3 workspace,
  where no `Fit` is visible in the consumer and the message correctly stays unqualified. The
  same-name check reads `union_defs`, which holds imported unions as well as local ones: probed with
  the colliding `Fit` imported from a third module rather than declared locally, and it still
  qualifies. `a_qualified_form_mismatch_names_the_declaring_modules` exists at
  `crates/nx-api/src/artifacts.rs:4301`.

### ✅ Verified - RF5 Inheritance cycle detection is still keyed by name, so a same-named base in another module is a false cycle
- **Severity:** Medium
- **Evidence:** `resolve_record_shape_inner` tracks its stack as `Vec<Name>` and compares
  `stack.iter().position(|name| name == &record.record.name)`
  (`crates/nx-hir/src/records.rs:658-671`); `resolve_base_record_inner`'s `seen`
  (`crates/nx-hir/src/records.rs:757-800`), `resolve_record_definition_inner`'s `seen`, and
  `validate_record_definition`'s stack are all `Name`-keyed, as are the component counterparts in
  `crates/nx-hir/src/components.rs`. This change makes cross-module lineages a first-class case
  ("a lineage the asking module cannot name SHALL still satisfy the base it actually extends"), so
  the one remaining name-keyed comparison now contradicts the rest. Reproduced:

  ```nx
  // base.nx
  export abstract type Shape = { ink: string }
  // app.nx
  import { Shape as ui.Shape } from "./base.nx"
  type Shape extends ui.Shape = { r: int }
  let root() = { <Shape ink="black" r=1 /> }
  ```

  ```
  error: Record inheritance cycle detected: Shape -> Shape
  error: Record 'Shape' has no field 'ink'
  ```

  Two distinct declarations, no cycle. The keying predates this change, but the scenario it breaks
  is one this change introduces, and the inherited fields silently disappear as well.
- **Recommendation:** Key the cycle stacks and `seen` sets by `DeclaringOrigin` (falling back to
  `Name` only where no origin is available), in both `records.rs` and `components.rs`. Add a
  positive test for a record extending a same-named base in another module, and keep a genuine
  self-cycle test alongside it.
- **Fix:** Confirmed as reported — reproduced verbatim, both diagnostics — and fixed as
  recommended. A new `nx_hir::DeclarationKey` (`Declared(DeclaringOrigin)` / `Spelled(Name)`, the
  fallback mirroring `same_declaration`'s `(None, None)` arm) keys the cycle stacks and the
  memoized validation statuses in `resolve_record_shape_inner`, `validate_record_definition`, and
  their component twins; the name is carried alongside because the cycle a diagnostic prints is a
  chain of names. `validate_record_definition` and `validate_component_definition` now take a
  resolved definition rather than a bare one, which closes a second name-keyed hole in the same
  place: they were resolving an imported base's own base in the consumer's namespace.
  The alias `seen` sets in `resolve_record_definition_inner` and `resolve_base_record_inner` were
  left name-keyed deliberately — an alias chain never leaves `namespace_module`, so a repeated name
  there really is a cycle.
  Two tests, `a_record_extending_a_same_named_foreign_base_is_not_a_cycle` and
  `a_component_extending_a_same_named_foreign_base_is_not_a_cycle`, each failing with its own
  keying reverted; the genuine self-cycle tests in `crates/nx-hir/src/lower.rs` still pass.
  Recorded as design D8, tasks 13.5-13.6, and a new requirement *An inheritance chain is walked by
  declaration*.
- **Verification:** Verified. `DeclarationKey` (`crates/nx-hir/src/prepared.rs:362-377`) keys the
  cycle stacks and memoized validation statuses in both `records.rs` and `components.rs`, and its
  `Declared`/`Spelled` split mirrors `same_declaration`'s `(Some, Some)`/`(None, None)` arms exactly
  — a `Declared` never equals a `Spelled`, matching the `_ => false` arm. The record repro was
  re-run verbatim and is now clean, where before it produced both `Record inheritance cycle
  detected: Shape -> Shape` and `Record 'Shape' has no field 'ink'`. The component repro is clean
  too. Genuine cycles are still caught: a mutually recursive `A extends B` / `B extends A` still
  reports `Record inheritance cycle detected: A -> B -> A`. Both named tests exist at
  `crates/nx-api/src/artifacts.rs:4336` and `:4364`, and the requirement *An inheritance chain is
  walked by declaration* is in the spec.

### ✅ Verified - RF6 Peer namespaces are cloned per module pair, and the language service rebuilds the whole graph per keystroke
- **Severity:** Medium
- **Evidence:** `analyze_logical_module_graph` clones every module's `ModuleNamespace` into every
  other module (`crates/nx-api/src/artifacts.rs:865-880`), and
  `build_library_artifact_with_registry` does the same
  (`crates/nx-api/src/artifacts.rs:590-600`). Each `ModuleNamespace` holds two `FxHashMap`s of
  every visible name, so this is O(n²) maps and O(n²·m) entries in module count — paid even
  between modules that never reference each other. On top of that,
  `WorkspaceSnapshot::document_scope` (`crates/nx-language-service/src/lib.rs:480-486`) calls
  `analyze_workspace_modules`, which runs `finalize_module_artifact` →
  `analyze_prepared_module` — full type checking of every workspace module — and it is called
  unconditionally from `completions()` on *every* request, including the plain keyword path, with
  no caching on the snapshot.
- **Recommendation:** Store peer namespaces as `Arc<ModuleNamespace>` so registration is a
  refcount rather than a map clone, and register only the peers a module actually has. Compute
  `DocumentScope` once per `WorkspaceSnapshot` (it is immutable for the snapshot's lifetime) and
  memoize it, rather than per completion request.
- **Note (same code path, separate concern):** `document_scope` passes
  `ProgramBuildContext::empty()`, so a document importing a library gets no scope entry for
  anything that library exports and offers no completions for it. Task 8.3's cases are all
  workspace-peer cases, so nothing covers this. Not a regression — the old flat join had no
  library declarations either — but the new path is where a fix would go.
- **Fix:** Both measures taken; the third suggestion deliberately not. `peer_namespaces` is now
  `FxHashMap<String, Arc<ModuleNamespace>>`, so registering a peer is a refcount rather than a
  clone of two maps — the O(n²) map copies in `analyze_logical_module_graph` and
  `build_library_artifact_with_registry` are gone, and `LibraryArtifact::namespaces` holds `Arc`s
  to match. `WorkspaceSnapshot` gained a `OnceLock<Arc<WorkspaceDeclarations>>` holding everything
  a document scope reads that does not depend on which document is being edited — `by_origin`, the
  type namespaces, and each module's visible bindings — so `analyze_workspace_modules` runs at most
  once per snapshot instead of once per completion request. `document_scope` now builds only the
  per-document `visible` map on top of it.
  **Not done:** *register only the peers a module actually has*. A declaration's contract can name a
  type from a module the consumer never imported — that is exactly what D3 exists to serve — so
  narrowing peer registration to the direct import graph would reintroduce the fallback into the
  consumer's namespace this change removed. The all-pairs registration is deliberate; making it
  cheap rather than smaller is the right fix.
  **Not done:** the note about `document_scope` passing `ProgramBuildContext::empty()`. It is a
  completion gap for library imports, not a regression (the reviewer says so), and giving the
  editor path a real build context is a feature with its own scope. Worth its own change.
  Recorded as tasks 13.7-13.8.
- **Verification:** Verified, including the two declined recommendations. `peer_namespaces` is
  `FxHashMap<String, Arc<ModuleNamespace>>` (`crates/nx-hir/src/prepared.rs:440`), so the loops at
  `crates/nx-api/src/artifacts.rs:593` and `:870` now clone a refcount rather than two maps.
  `WorkspaceSnapshot` holds `declarations: OnceLock<Arc<WorkspaceDeclarations>>` and
  `document_scope` reads it through `workspace_declarations()`
  (`crates/nx-language-service/src/lib.rs:487-516`); `analyze_workspace_modules` now has exactly one
  caller in the language service, inside `build_workspace_declarations`, so it runs at most once per
  snapshot. The cache is sound because `WorkspaceSnapshot` exposes no `&mut self` API — it is
  built once by `from_documents` and read-only thereafter. The reasoning for not narrowing peer
  registration is correct: a contract can name a type from a module the consumer never imported,
  which is D3's whole subject, so narrowing would restore the fallback this change removes. Deferring
  the `ProgramBuildContext::empty()` completion gap is reasonable — it is a pre-existing gap, not a
  regression, and giving the editor path a real build context is its own feature.

### ✅ Verified - RF7 NX IR still carries an `unresolved:` slot for a selectively-aliased qualified case
- **Severity:** Low
- **Evidence:** The ADDED scenario *Generated output never carries an unresolvable reference* says
  a module analyzed without diagnostics SHALL produce no IR reference that failed to resolve. With
  `import { Img, Fit as ui.Fit } from "./widgets.nx"` and `<Img fit={ui.Fit.cover} />`, codegen
  succeeds with **zero diagnostics** and the emitted `app.nxir.json` contains
  `"slot": "unresolved:ui"` (there is no `ui` binding — the alias binds the single name `ui.Fit`)
  plus an outer `member` node with `"reference": null` for the case. TypeScript emission is
  unaffected (`m1_Fit["cover"]`), so this is an IR-representation defect rather than a codegen
  one. `nx_ir_carries_no_unresolved_reference_for_a_case_of_an_unimported_union`
  (`crates/nx-codegen/src/tests.rs:248`) asserts the scenario only for the bare form, and
  `identity_survives_a_rename_at_the_import_boundary`
  (`crates/nx-api/src/artifacts.rs`) exercises exactly this expression without inspecting the IR.
- **Recommendation:** Either resolve the alias's leading segment when lowering `Member` (the inner
  member already carries the correct union reference), or extend the assertion in the existing
  codegen test to the aliased qualified form and record the gap explicitly against the scenario if
  it is being deferred to NXE14.
- **Fix:** Confirmed as reported and fixed at the source rather than by relaxing the assertion. The
  cause is in the builder, not the IR writer: `build_union_case_for_member` and
  `qualified_member_reference` both required the member chain's base to be a single `ast::Expr::Ident`,
  so `ui.Fit.cover` — where the alias binds the one visible name `ui.Fit`, and there is no `ui` to
  take a member of — fell through to a plain member access on a name that reaches nothing. Both now
  go through a new `flattened_visible_name`, which reads the whole dotted spelling a chain writes
  (checking the lexical scope at the root segment only), so the expression lowers to a resolved
  `UnionCase`. The `unresolved:` slot and the null case reference are both gone; TypeScript output
  is unchanged.
  New test `nx_ir_carries_no_unresolved_reference_for_a_case_of_an_aliased_union` in
  `crates/nx-codegen/src/tests.rs`, asserting no `unresolved:` and no `"reference": null`; it fails
  with the single-segment base restored. Recorded as task 13.9 and an added bullet on the existing
  *Generated output never carries an unresolvable reference* scenario.
- **Verification:** Verified, and fixed at the cause rather than at the assertion.
  `flattened_visible_name` (`crates/nx-codegen/src/builder.rs:1638-1656`) reads the whole dotted
  spelling a member chain writes, returning `None` at the root when the segment is lexically bound,
  and both callers then require the flattened name to resolve to a real visible reference —
  `build_union_case_for_member` additionally requiring `ResolvedItemKind::Union` — so ordinary field
  access on a module-level value cannot be captured by it. The repro was re-run: `<Img
  fit={ui.Fit.cover} />` under `Fit as ui.Fit` now emits with no `unresolved:` slot anywhere, zero
  `"reference": null`, and a resolved `"caseName": "cover"`.
  `nx_ir_carries_no_unresolved_reference_for_a_case_of_an_aliased_union` exists at
  `crates/nx-codegen/src/tests.rs:289`.

### ✅ Verified - RF8 The record and component identity machinery is duplicated line for line, and the shared origin comparison exists in three copies
- **Severity:** Low
- **Evidence:** `resolve_record_reference`/`component_reference`,
  `record_definition_at`/`component_definition_at`, `is_record_subtype`/`is_component_subtype`
  (`crates/nx-hir/src/records.rs:434-515` and `crates/nx-hir/src/components.rs:317-395`) differ
  only in the `Item` variant and the definition type; likewise `record_lineage`/`component_lineage`
  and `common_record_supertype`/`common_component_supertype` in
  `crates/nx-types/src/infer.rs:19-38` and `crates/nx-types/src/infer.rs:3023-3092`. The origin
  comparison itself is written three times: `ty.rs::same_declaration`
  (`crates/nx-types/src/ty.rs:471`), `records.rs::same_record_declaration`
  (`crates/nx-hir/src/records.rs:53`), and the `denotes` closure inside
  `InferenceContext::union_entry_for` (`crates/nx-types/src/infer.rs:986`) — all encoding the same
  `(Some, Some) => eq / (None, None) => name / _ => false` rule. `is_component_subtype` already
  calls the record-named helper, which reads as a mistake at the call site.
- **Recommendation:** Promote one `DeclaringOrigin::denotes(other_origin, self_name, other_name)`
  (or a free `same_declaration`) in `nx-hir` beside `DeclaringOrigin`, and have `ty.rs`,
  `records.rs`, `components.rs`, and `union_entry_for` all call it. Consider a small trait or
  generic helper over `(Item variant, definition type)` for the subtype/lineage pairs so record and
  component identity cannot drift apart.
- **Fix:** The shared rule is promoted; the generic unification is not. `nx_hir::same_declaration`
  now lives beside `DeclaringOrigin` and is called by `ty.rs`, `records.rs`, `components.rs`, and
  `union_entry_for`'s `denotes` closure; `records.rs::same_record_declaration` is deleted, which
  also removes the oddity of `is_component_subtype` calling a record-named helper.
  **Not done:** a trait or generic over the record/component subtype and lineage pairs. They are
  genuinely parallel, but they differ in their `Item` variant, their definition struct, and their
  error enum, and unifying them would trade duplication that reads plainly for machinery that does
  not — in a change whose subject is elsewhere, and with a blast radius across two modules of
  inheritance resolution. Recommended as its own change if a third nominal kind ever appears.
  Recorded as design D9 and task 13.10.
- **Verification:** Verified. `nx_hir::same_declaration` is the single copy
  (`crates/nx-hir/src/prepared.rs:341`), called from `records.rs`, `components.rs`, `ty.rs`, and
  `union_entry_for`'s `denotes`; `same_record_declaration` is gone from the tree entirely, which
  removes the `is_component_subtype`-calling-a-record-helper oddity. Declining the generic over the
  record and component pairs is the right call for this change: they differ in `Item` variant,
  definition struct, and error enum, and the duplication is inert rather than drift-prone now that
  the one rule they both encode lives in one place.

### ✅ Verified - RF9 A contextual resolution with no origin is silently left unrewritten
- **Severity:** Low
- **Evidence:** `apply_contextual_name_resolutions` now takes a fallible `rewrite` closure and
  `continue`s when it yields `None` (`crates/nx-hir/src/components.rs:566-568`), and
  `analyze_prepared_module` returns `None` whenever `resolution.origin` is `None`
  (`crates/nx-types/src/check.rs:200-212`). The previous code rewrote unconditionally. A surviving
  `ast::Expr::ContextualName` is reported by codegen
  (`crates/nx-codegen/src/builder.rs:1235`) but falls into the interpreter's `_ => Ok(Value::Null)`
  catch-all (`crates/nx-interpreter/src/interpreter.rs:1893`), so an analysis-clean program would
  evaluate the property to `null`. Unreachable today — every `UnionEntry` is built with
  `Some(origin)` — but the failure mode is silent where it used to be impossible.
- **Recommendation:** Emit an internal-error diagnostic in `check.rs` when a recorded resolution
  has no origin, the way `resolve_contextual_name_in` already does for a missing `UnionEntry`, so
  the invariant fails loudly rather than turning into a `null`.
- **Fix:** Fixed as recommended. `analyze_prepared_module` now walks the recorded resolutions
  before applying them and reports `contextual-name-origin-missing` — "Internal error: '<case>'
  resolved to '<union>.<case>', but the resolution carries no declaring module" — at the
  expression's span for any resolution with no origin. The invariant now fails loudly instead of
  turning into a `Value::Null` two crates away. Unreachable today, so no test pins it; the
  diagnostic *is* the assertion. Recorded as task 13.11 and a new spec scenario *A resolution that
  reaches no declaration is reported*.
- **Verification:** Verified. `analyze_prepared_module` walks the recorded resolutions before
  applying them and reports `contextual-name-origin-missing` at the expression's span for any
  resolution carrying no origin (`crates/nx-types/src/check.rs:200-219`). The invariant now fails
  loudly in analysis rather than becoming a `Value::Null` two crates away. Leaving it untested is
  right — it is unreachable by construction, and a test would have to break the construction to
  reach it. The spec scenario *A resolution that reaches no declaration is reported* is present.

### ✅ Verified - RF10 The headline collision test asserts only that *some* diagnostic was produced
- **Severity:** Low
- **Evidence:** `a_same_named_local_union_does_not_satisfy_a_foreign_union_with_matching_case_names`
  (`crates/nx-api/src/artifacts.rs`) asserts `!messages.is_empty()`. That is task 1.1's pin — the
  `string`-into-an-`int`-field case that was the change's motivating defect — and it would pass on
  any unrelated diagnostic in either module, including a parse error introduced by a later edit to
  the fixture. The sibling tests around it (`..._with_different_case_names_stays_rejected`,
  `a_same_named_local_record_does_not_satisfy_a_foreign_record`) do assert on message content.
- **Recommendation:** Assert the specific message, and once RF4 is fixed assert that it names both
  declaring modules.
- **Fix:** Fixed as recommended, including the part that depended on RF4.
  `a_same_named_local_union_does_not_satisfy_a_foreign_union_with_matching_case_names` now asserts
  `Property 's' on 'Draw' expects …` naming both `widgets.nx:S` and `app.nx:S`, so it can no longer
  pass on an unrelated diagnostic. It fails with the RF4 display fix reverted. Recorded as task
  13.12.
- **Verification:** Verified. The test
  (`crates/nx-api/src/artifacts.rs:3763`) now requires all three of `Property 's' on 'Draw'
  expects`, `widgets.nx:S`, and `app.nx:S`, so it can no longer pass on an unrelated diagnostic in
  either module, and it pins the RF4 behavior as well.

## Summary (2026-09-01 pass)
- Eight new findings. RF3 is the one that matters: the change's central rule has a live hole
  through type aliases, reproducible with no diagnostic at all, and it is the same soundness defect
  the proposal opens with. RF4 and RF5 are correctness-of-experience gaps against scenarios this
  change's own specs assert. RF6 is a cost regression on the editor path. RF7–RF10 are hygiene.
- The implementation is otherwise sound and unusually well covered: the origin plumbing through
  analysis, lowering, codegen, evaluation, and the host boundary is consistent, and the
  positive/negative test pairing for every rejection scenario is the right discipline for a change
  that narrows type identity. Full workspace suite re-run during this review: green, 0 failures.

### Third fix pass
- RF3-RF10 all fixed; awaiting verification by the reviewing agent. No findings left open. Two
  recommendations inside otherwise-fixed findings were deliberately not taken, each with its
  reasoning recorded on the finding: narrowing peer registration to the direct import graph (RF6),
  which would reintroduce the very fallback D3 removes, and folding the record and component
  machinery into one generic (RF8).
- Verified: full workspace suite green (47 suites, 0 failures); corpus IR matches the baseline for
  all 13 examples; `openspec validate --strict` passes. `cargo fmt --all -- --check` reports only
  the four pre-existing `crates/nx-types/tests/contextual_literals.rs` diffs, unchanged by this
  pass. Clippy on `nx-hir`, `nx-types`, `nx-interpreter`, `nx-api`, `nx-codegen`,
  `nx-language-service` (`--lib`) and `nx-cli` (`--bins`) was re-diffed against a stashed pre-change
  tree after touching every source file to defeat caching: identical, 35 unique findings on both
  sides, no new ones.
- Each new test was checked to fail with its own fix reverted and no other: the three alias tests
  against the `Item::TypeAlias` arm; `a_qualified_form_mismatch_names_the_declaring_modules` and the
  headline collision test against the rendered-string qualification rule; the four updated
  message tests against `display_union_name`; the record and component cycle tests against their
  own walk keying; and the aliased-case IR test against the single-segment member base.
- Four existing assertions were updated rather than preserved, all of them message pins that RF4
  deliberately changed: `a_property_typed_by_a_union_the_declaring_module_imported_does_not_bind_locally`,
  `a_same_named_local_union_with_different_case_names_stays_rejected`,
  `imported_property_type_does_not_bind_to_a_same_named_local_union`, and
  `a_library_property_typed_by_a_sibling_module_union_does_not_bind_locally`. Each now asserts the
  qualified form, which is what each was testing for in the first place.
- The out-of-scope observation from the second fix pass still stands and is still unfixed: element
  properties are not checked on an element nested inside another element's content
  (`<div><Ink d="nonsense" /></div>` produces no diagnostic). Unrelated to nominal identity, worth
  its own change.

### Fourth verification pass (2026-09-01 01:12)
- All eight findings from the 2026-09-01 review pass verified as fixed. None reopened, and no new
  findings. RF3, RF4, RF5, and RF7 were each re-run against the original repro workspaces rather
  than read from the diff, with a control alongside: RF3's legitimate alias direction, RF5's genuine
  `A -> B -> A` cycle, and RF4's no-collision case all behave correctly, so none of the four fixes
  bought its rejection by over-rejecting.
- Independently re-measured: `cargo test --workspace` — 47 suites, 1316 passed, 0 failed;
  `record-corpus-ir.py --check` — "corpus IR matches the baseline for all 13 examples";
  `openspec validate nominal-type-identity --strict` — valid; `cargo fmt --all -- --check` — only
  the four pre-existing `crates/nx-types/tests/contextual_literals.rs` diffs. Clippy on the touched
  crates raises nothing in new code (the one warning that looked new, `needless_lifetimes`, is on
  the pre-existing `contextual_target`).
- Two boundaries were probed for over-reach and neither is one. `flattened_visible_name` cannot
  capture ordinary field access: it stops at a lexically bound root, and both callers require the
  flattened spelling to resolve to a real reference (a union, for the case path). `display_union_name`
  reads `union_defs`, which holds imported unions too, so a collision between two *foreign*
  same-named unions qualifies as well as a local/foreign one — confirmed by probe.
- The two deliberately declined recommendations (RF6's narrower peer registration, RF8's generic
  over the record and component pairs) are each correctly declined; the reasoning recorded on the
  findings holds. RF6's deferred completion gap for library imports remains genuinely out of scope.
- No findings open. Change is ready to archive.
