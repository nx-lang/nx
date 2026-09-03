## 1. Pin the current behavior

- [x] 1.1 Add a failing test for the same-name collision, in a workspace where the consumer declares a union sharing a name with the one typing an imported component's property. `replace-enums-with-unions` collapsed enums into unions, so there is one nominal kind to collide and one test rather than two. With `widgets.nx` exporting `type S = idle | busy { n: int }` and `<Draw s: S />`, assert the case that is still accepted today: a consumer declaring `type S = | idle | busy { n: string }` writing `<Draw s={<S.busy n="wrong" />} />`, where a `string` reaches a field the declaring module typed `int`. The sibling case — a consumer declaring `type S = | idle | spinning` writing `<Draw s=spinning />` — is **already rejected**: that change made `(UnionCase, Union)` compatibility require the union to declare the case, and made contextual resolution search the resolved type's own cases. Assert it stays rejected rather than expecting it to fail (RF6 in `contextual-literal-binding`'s `review.md`, whose same-name/different-case-names half closed there)
- [x] 1.2 Add a failing test asserting a bare case of a non-imported union lowers and evaluates to the same value as the qualified form; verify it fails with the current import guidance
- [x] 1.4 Record the current bare-vs-qualified IR equivalence output for the example corpus as the baseline this change must preserve; verify the recorded IR matches a clean build at `HEAD`

## 2. Origin on nominal types

- [x] 2.1 Add a declaring-origin field to `UnionType` and `UnionCaseType` in `crates/nx-types/src/ty.rs`, carrying module identity and definition id; verify the crate builds and every construction site supplies an origin
- [x] 2.2 Populate origin wherever nominal types are built from prepared items in `crates/nx-types/src/infer.rs` (`register_type_definitions`, `nominal_type_in_module`, `union_type_from_def`); verify a type built from a peer module carries that peer's identity
- [x] 2.3 Make type equality and `is_compatible_with` decide nominal sameness by origin, including the `Type::UnionCase` to `Type::Union` relation that currently compares `case.union == union.name`; verify task 1.1's collision test now passes
- [x] 2.3a Resolve a contextual union case against the union the expected type already resolved to, instead of reducing that type to its name and looking the definition back up local-first. **Done by `replace-enums-with-unions`**, which had to: collapsing `enum_defs` and `union_defs` into one resolution path made the name-based lookup the only one left, and leaving it would have regressed every enum. `contextual_name_type` now searches the resolved `UnionType`'s own cases and accepts a definition reached by that name only when it is the same type
- [x] 2.4 Settle whether `Type::Named` is removed or retained as an explicit unresolved marker (design Open Question), and make a `Type::Named` surviving resolution mean resolution failed; verify no path can produce a nominal match on spelling alone
- [x] 2.5 Run the full workspace suite and account for every newly failing test as either a genuine collision the change is meant to reject or a regression to fix; verify the suite is green with each intentional change recorded

## 3. A contract resolves in the namespace of the module that wrote it

Retargeted while implementing — see `design.md`, D3 (revised). The published-interface
representation the original tasks called for is not what closes the gap: a library already registers
every one of its modules as a peer of the consuming module, so its definitions are readable the way
a workspace peer's are. What was missing is that a declaration's type references were resolved among
the declaring module's own items only, so a property typed by a union that module *imported* fell
back to the consumer's namespace.

- [x] 3.1 Collect each module's type namespace from its prepared bindings and register it on every
  module that has it as a peer (`TypeNamespaceEntry`, `type_namespace_from_bindings`,
  `PreparedModule::add_peer_type_namespace`); verify a peer's namespace resolves a name that peer
  imported rather than declared
- [x] 3.2 Prepare every module before analyzing any of them, in both the workspace graph
  (`analyze_logical_module_graph`) and the library build (`build_library_artifact`), because a
  peer's namespace is only known once its own imports have been applied; verify a union-typed
  property of a component imported from a built library resolves without importing the union
- [x] 3.3 Resolve through the peer namespace in `nominal_type_in_module` and remove the binding scan
  added by `contextual-literal-binding`, which the peer path subsumes; verify the cross-module tests
  in `crates/nx-api/src/artifacts.rs` still pass
- [x] 3.4 Cover the transitive case in both directions — a workspace peer and a library sibling
  module — with a consumer declaring a same-named union; verify each is rejected against the
  declaring module's cases

## 4. Origin through lowering and code generation

- [x] 4.1 Extend `ContextualResolution` in `crates/nx-types/src/infer.rs` to carry the resolved type's origin; verify it is populated for both constant and payload union cases
- [x] 4.2 Change the rewrite target in `crates/nx-hir/src/components.rs` from `Member { base: Ident(type_name), .. }` to a node carrying the resolved origin; verify no consumer of the rewritten expression resolves it by visible name
- [x] 4.3 Build `CodegenReference` from the carried origin in `crates/nx-codegen/src/builder.rs` instead of `resolve_visible_reference`; verify generated IR for a non-imported union case contains no `unresolved:` slot
- [x] 4.4 Resolve union and union-case definitions by origin in `crates/nx-interpreter/src/interpreter.rs`, including the base record shape a union case inherits; verify a foreign payloadless case with an abstract base evaluates with its base fields and defaults
- [x] 4.5 Remove the `contextual-name-requires-import` guard and its union counterpart from `crates/nx-types/src/infer.rs`; verify task 1.2's test passes and the `enum-values` and `discriminated-unions` scenarios for non-imported types pass
- [x] 4.6 Confirm the bare and qualified forms still produce identical IR against the task 1.4 baseline; verify the equivalence test from `contextual-literal-binding` is green and the example corpus IR is unchanged

## 5. Records and components

Retargeted while implementing — see `design.md`, Q2 (reversed) and D2 (revised). Task 5.1 first
deferred records, on a cost estimate that had not been measured and a framing that read as a scoping
line rather than a live defect. A probe showed two same-named records in different modules produce no
diagnostic and evaluate with a `string` in a field the declaring module typed `int`. Components share
the `Type::Named` variant and the same substitution, so fixing one without the other would leave one
variant with two identity rules.

- [x] 5.1 Decide whether record types carry origin in this change or a follow-on (design Open
  Question), and record the decision in `design.md`; verify no spec scenario depends on the outcome.
  **Reversed after measuring** — records and components carry origin here, and the spec now has
  scenarios for both
- [x] 5.2 Move `DeclaringOrigin` into `crates/nx-hir` and make the peer namespace entry that change
  introduced be that type, so one origin type serves analysis, inheritance resolution, and codegen;
  verify no second `(module identity, definition id)` spelling remains
- [x] 5.3 Give `Type::Named` a `NamedType { name, origin }` payload with the same equality rule the
  union types use, and populate origin for every visible record and component name
  (`record_origins`, `component_origins`, `nominal_named_type`); verify `Element`, `object`, and a
  name reaching no declaration stay origin-less
- [x] 5.4 Resolve a record's or component's `extends` clause in the module that wrote it, and carry
  the declaration each link reached on the ancestor chain (`RecordAncestor`, `ComponentAncestor`,
  `resolve_in_module`, `ModuleNamespace`'s element half); verify a lineage the consumer cannot name
  still satisfies the base it actually extends
- [x] 5.5 Decide record and component subtyping by declaration rather than by spelling
  (`is_record_subtype`, `is_component_subtype`, `common_record_supertype`,
  `common_component_supertype`); verify the collision, identical-shape, transitive-import, and
  same-named-lineage cases are each rejected and each positive case still accepted
- [x] 5.6 Qualify a same-named record mismatch by declaring module on both sides; verify
  `expects Point, found Point` becomes `expects widgets.nx:Point, found app.nx:Point`

## 6. Union cases as a runtime value kind — removed

`replace-enums-with-unions` made a constant case a scalar `Value::UnionCase` rather than an empty
dotted record, so there is no union-case discriminator left to add and no heuristic left to replace.
The group's four tasks were its tasks 5.1–5.4 and 5.5. Numbering below is unchanged so the
cross-references in groups 7–9 still resolve.

## 7. Diagnostics

- [x] 7.1 Qualify nominal type names in a diagnostic by declaring module when two types in that message share a display name; verify `expects Fit, found Fit` becomes distinguishable
- [x] 7.2 Confirm messages that name only one nominal type are unqualified; verify existing diagnostic tests are unchanged

## 8. Language service

- [x] 8.1 Resolve elements and property types through the document's import graph in `crates/nx-language-service/src/lib.rs`, replacing the flat `workspace_declarations` join by declaration name; verify completions work for an element written under an import alias
- [x] 8.2 Exclude declarations not visible to the document being edited; verify a same-named declaration in an unimported document offers no members
- [x] 8.3 Add tests for selective import, wildcard alias, non-visible declarations, and duplicate type names in different modules; verify each has a case

## 9. Verification

- [x] 9.1 Run the full workspace suite and the VS Code grammar suite; verify both pass with no pre-existing test modified except where an origin-based rejection is intended and recorded. Re-run after group 10
- [x] 9.2 Walk every scenario in `specs/nominal-type-identity/spec.md` and the five delta specs and confirm each has a corresponding test
- [x] 9.2a Align every MODIFIED block with the specs as `replace-enums-with-unions` leaves them, and re-measure: a MODIFIED requirement replaces the whole block, so it may not silently drop a scenario the promoted spec has. The mismatch recorded here was measured against `contextual-literal-binding`'s promotion and is now stale — this change's `enum-values` delta has been retargeted onto *Constant cases are referenceable without naming the union type*, the requirement `replace-enums-with-unions` adds in place of *Enum members are referenceable without naming the enum type*, and `openspec validate nominal-type-identity --strict` passes today. It must be re-run once that change archives, because the promoted spec it validates against changes then. Note also that REMOVED plus ADDED of the same requirement name is **rejected** by OpenSpec (*"Requirement present in both ADDED and REMOVED"*), so a requirement that renames scenarios has to keep them under their promoted names — see that change's task 10.2a
- [x] 9.3 Verify the example corpus produces identical IR and identical diagnostic counts before and after, except for the intended collision rejections. Re-run after groups 5 and 10; the corpus still matches the task 1.4 baseline for all 13 examples
- [x] 9.4 Update `review.md` in `contextual-literal-binding` to mark RF1's deferred half, RF3, and RF6's remaining half resolved here; verify each finding points at the covering test. Not RF2 or RF5 — `replace-enums-with-unions` fixed both and recorded their covering tests. RF6's same-name/different-case-names half closed there as well; only the payload-field-type half is this change's

## 10. The host input boundary

Added while implementing — see `design.md`, D6. Q2 first recorded the runtime half as a follow-on.
It is the same rule as D3 one layer down, it reuses group 5's subtyping wholesale, and a probe
showed the defect is live rather than inferred: a host action carrying an `int` was accepted against
a library declaration typing that field `string`, and evaluated. Splitting it out would have shipped
the static invariant with a hole under it in the same release.

- [x] 10.1 Pin the current behavior with a failing test dispatching a host action to a handler bound
  on an imported component, where the binding module declares its own action sharing the emitted
  action's name; verify the wrongly typed host value is accepted today
- [x] 10.2 Carry the emit's declaring module on the resolved contract (`EffectiveEmit`), through the
  handler rewrite and `ast::Expr::ActionHandler`, to `Value::ActionHandler`; verify a binding lowered
  straight from source records the module it was written in, which is the module that declared the
  emit it reached
- [x] 10.3 Resolve the expected action record in the emit's declaring module in
  `validate_handler_input` rather than in the module that wrote the binding; verify task 10.1's test
  now rejects, and that host input matching the emitted declaration is still accepted
- [x] 10.4 Decide record and component subtyping at the boundary by declaration rather than by
  spelling, reusing `is_record_subtype` and `is_component_subtype`; verify a host label whose lineage
  only shares a name with the expected base is rejected and a descendant declared in a third module
  is still accepted
- [x] 10.5 Register each peer's own namespace on the runtime prepared module, not just the peer
  module (`ResolvedProgram::local_items`, `runtime_module_namespace`); verify a foreign
  declaration's `extends` clause reaches a base that peer imported. Found by task 10.4's positive
  case, which over-rejected without it — the D3 gap, unfixed at runtime because nothing had asked
  the question there
- [x] 10.6 Let a peer namespace address a declaration back in the asking module
  (`PreparedModule::item_at`); verify a peer extending a base the asking module exports resolves.
  Latent on the analysis side too, where no existing scenario had a peer's `extends` point back at
  the consumer
- [x] 10.7 Evaluate a component in the module that declares it, not the one that named it
  (`find_component` returning the declaring module, shadowed through
  `initialize_component_with_limits` and `evaluate_component_with_limits`); verify a fully imported
  component initializes against its own arena. `Component::body` and prop defaults are `ExprId`s
  into the declaring module, so the module-taking API read a different expression at the same index
  — the program-entrypoint API already resolved the declaring module through `entry_component`,
  which is why every existing library test passed over it. Pre-existing and independent of nominal
  identity, but it made this group's own rule unreachable for the ordinary embedding shape: a
  handler bound on an imported component
- [x] 10.8 Cover the boundary rule where the component is imported outright rather than extended
  locally, which task 10.7 unblocked; verify host input is checked against the library's declaration
  and that input matching it is still accepted

## 11. Post-resolution lookup (review finding RF1)

- [x] 11.1 Select a union's definition by the declaration its resolved type names, not by the
  spelling it was reached under (`InferenceContext::union_entry_for`); route base-through-record
  compatibility, shared and case-specific member access, and common-supertype construction through
  it. See D7
- [x] 11.2 Resolve a union's abstract base, and each inherited field's type, in the module that
  declared them rather than in the asking module — the base through its own declaring origin, each
  field through `EffectiveField::module_identity`
- [x] 11.3 Resolve a union case's own field types in the union's declaring module when checking
  `<Union.case ... />`, threading the declaring module the way the record and component element
  checks already do. Found while building 11.1's test vehicle
- [x] 11.4 Resolve an imported value's type annotation in the module that declared the value, not in
  the consumer (`register_value_bindings`, both the workspace-peer and library-interface arms).
  Also found by 11.1's test vehicle, and the reason the collision was unreachable from a test: the
  value carrying a foreign union across the boundary was re-typed on arrival
- [x] 11.5 Cover all four with collision tests — a foreign union satisfying the base it extends, a
  same-named local base not answering for a foreign union, a shared field typed by the foreign
  base, and a case's field type resolving in its own module; verify each fails with the selector
  reverted to name-only lookup
- [x] 11.6 Box `PreparedSourceFile::Prepared`'s module so the variant this change introduced does
  not trip `clippy::large_enum_variant`

## 12. Foreign caching and imported signatures (review findings RF1 reopened, RF2)

- [x] 12.1 Key `foreign_union_defs` by `DeclaringOrigin` rather than by the name a union arrived
  under, so two foreign unions a consumer receives under one spelling are both retained; address a
  foreign entry by origin in `union_entry_for`. See D7
- [x] 12.2 Route the contextual-name lookup through `union_entry_for` as well, so the last
  name-keyed reach into the foreign map is gone
- [x] 12.3 Resolve an imported function's parameter and return annotations in the module that
  declared the function — the workspace-peer arm and the library-interface arm of
  `register_function_signatures`, and `bind_function_signature_from_parts`. See D7
- [x] 12.4 Cover the collisions with tests: two foreign same-named unions in one consumer, and a
  peer and a library function whose parameter and return types share a spelling with an unrelated
  local union; verify each fails with its own fix reverted

## 13. Aliased contracts, diagnostics, and inheritance walks (review findings RF3-RF10)

- [x] 13.1 Follow `Item::TypeAlias` in `foreign_nominal_type`, resolving the alias target in the
  declaring module recursively, guarded by a stack of the alias declarations being followed. See D3
- [x] 13.2 Cover the aliased contract with tests: a union alias and a record alias whose spelling
  collides with a consumer declaration, and the legitimate case an alias must still accept
- [x] 13.3 Decide diagnostic qualification on the nominal parts of the two types rather than on
  their rendered strings, so `Union Fit` against `UnionCase Fit.cover` qualifies both. See D5
- [x] 13.4 Qualify a union's name in `report_unknown_union_case` and in the bare contextual-name
  message when a different union of that name is visible at the use site. See D5
- [x] 13.5 Key the record and component inheritance walks by `DeclarationKey` rather than by
  `Name`, and resolve each base in the module that wrote its `extends` clause. See D8
- [x] 13.6 Cover both walks with a positive test for a record and a component extending a same-named
  base in another module, alongside the existing genuine-cycle tests
- [x] 13.7 Share peer `ModuleNamespace`s behind an `Arc` so registering a peer is a refcount rather
  than a clone of two maps per module pair
- [x] 13.8 Compute the language service's workspace analysis once per `WorkspaceSnapshot` and reuse
  it, so a completion request no longer type checks every workspace module
- [x] 13.9 Resolve a member chain against the whole visible name it spells, so a case reached
  through a selective import alias (`ui.Fit.cover`) lowers to a resolved reference instead of an
  `unresolved:` slot and a null reference in NX IR
- [x] 13.10 Promote one `nx_hir::same_declaration` and call it from `ty.rs`, `records.rs`,
  `components.rs`, and `union_entry_for`. See D9
- [x] 13.11 Report an internal-error diagnostic when a recorded contextual resolution carries no
  origin, so an unrewritten contextual name cannot evaluate to `null` in silence
- [x] 13.12 Assert the specific message in the headline collision test, including both declaring
  modules
