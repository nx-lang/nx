# Review: contextual-literal-binding

## Scope
**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md`, and all four delta specs  
**Reviewed code:** working-tree changes in `crates/nx-syntax`, `crates/nx-hir`, `crates/nx-types`,
`crates/nx-interpreter`, `crates/nx-cli`, `crates/nx-language-service`, `crates/nx-api`,
`crates/nx-codegen`, `src/vscode`, examples, and documentation  
**Verification:** `cargo test` passes, with a duplicate-test-attribute warning in
`crates/nx-cli/src/main.rs`
- **Fix:** Removed the stray second `#[test]` on `test_cli_contextual_literal_matches_qualified_form`
  (`crates/nx-cli/src/main.rs:2171`). The workspace now builds warning-free. The duplicate had been
  registering that test twice, so the suite total moves from 1242 + 1 phantom entry to **1242 passed,
  0 failed, 0 warnings**; the test itself still runs and passes.

## Findings

### ✅ Resolved (remainder carried by `nominal-type-identity`) - RF1 Contextual names cannot resolve through an imported component's unimported nominal property type
- **Severity:** High
- **Evidence:** The motivating enum scenarios require the enum type not to be imported and explicitly
  cover a wildcard alias. Imported component properties are converted with
  `type_from_type_ref` at `crates/nx-types/src/infer.rs:1377`, but nominal definitions are populated
  only from type bindings visible in the current module at `crates/nx-types/src/infer.rs:2089`.
  Consequently an imported component's `Fit` property remains `Type::Named("Fit")` when only the
  component is selectively imported, or when the visible enum binding is `ui.Fit`. Resolution then
  consults only `enum_defs`/`union_defs` keyed by that unresolved name at
  `crates/nx-types/src/infer.rs:1833` and `crates/nx-types/src/infer.rs:1877`, and reports that the
  expected type is non-nominal. The successful resolution is also rewritten to a qualified
  `Ident(type_name)` downstream, which would still assume that type name is lexically available.
- **Recommendation:** Resolve property type references in the declaring module/interface context and
  carry a canonical nominal identity (including module provenance) in `ContextualResolution`; add
  workspace/library tests for a selective component-only import and `import "../ui" as ui` without
  importing the enum or union type.
- **Status:** Left open — **confirmed by reproduction**, and larger than the report states. Both
  scenarios under `enum-values` → *Enum members are referenceable without naming the enum type* fail
  today, so this is an unimplemented requirement, not a latent edge case:

  ```
  // widgets.nx: export enum Fit = fill | contain | cover
  //             export let <Img fit: Fit = fill /> = <div class="img" />

  import { Img } from "./widgets.nx"          // selective, component only
  let root() = { <Img fit=cover /> }
  // error: Property 'fit' on 'Img' expects Fit, and a bare name resolves only
  //        against an enum or union; for a string value write "cover"

  import "./widgets.nx" as ui                 // wildcard alias
  let root() = { <ui.Img fit=cover /> }
  // error: Property 'fit' on 'ui.Img' expects Fit, ...
  ```

  Adding `Fit` to the import list makes both pass, which is exactly the dependency the proposal set
  out to remove. The fix is not confined to the type checker: `apply_contextual_name_resolutions`
  rewrites to `Member { base: Ident("Fit"), .. }`, and the interpreter resolves that base by
  module-level lookup (`resolve_enum_definition`, `crates/nx-interpreter/src/interpreter.rs:2773`),
  which fails when `Fit` has no visible binding. So it needs (1) registering nominal definitions
  reachable through an imported component's property types, (2) canonical provenance in
  `ContextualResolution`, and (3) a rewrite target that does not depend on a visible name.

  Step 3 is a design decision, not a mechanical change — the strongest option is a dedicated
  resolved-member HIR node carrying the value directly, which would be simpler than today's rewrite
  but reverses D2/Option A. Deferring to the author rather than choosing unilaterally.

  Task 9.2 has been reopened, since its verification claim ("every scenario has a corresponding
  test") was wrong for these two scenarios; every test in `crates/nx-types/tests/contextual_literals.rs`
  is single-module.
- **Fix (front end):** A declaration's property type references are now resolved in the module that
  declares them, for imported and peer components, functions, and records
  (`type_from_type_ref_in` / `nominal_type_in_module`, `crates/nx-types/src/infer.rs`; new
  `PreparedModule::peer_module` and `resolve_record_definition_with_module`). A bare name at an
  imported component's enum-typed property now resolves against the declaring module's enum with no
  import of that enum, and unknown members are reported against the right member list.

  This also closed the **pre-existing soundness bug** found while verifying this finding: a local
  type sharing a spelling with the declaring module's type silently unified with it, so
  `<Img fit={Fit.stretch} />` compiled clean and put `"stretch"` in the IR at a property typed by an
  enum with no such member. It reproduced at `HEAD` and is now rejected in both the bare and
  qualified forms, because the expected type is a resolved `Type::Enum` carrying its members rather
  than an unresolved `Type::Named` that matches on spelling alone.

  Four cross-module tests were added in `crates/nx-api/src/artifacts.rs` (selective import,
  component-only import, same-name collision, wildcard alias); `check_str` is single-module and
  could not express them. Suite: **1246 passed, 0 failed, 0 warnings**.
- **Status (back end — split out):** Carrying a resolved member through lowering is deferred to its
  own change. Investigating the fix disproved the plan it was based on: enum values do not lower to
  literals, they lower to cross-module references — `CodegenExpressionKind::EnumMember` takes a
  canonical `CodegenReference` and JS emission produces `Fit.cover` plus an emitted import
  (`crates/nx-codegen/src/emit.rs:2767`, `:3527`). The rewrite's `Ident(type_name)` base is resolved
  by *visible name* (`enum_member_reference` → `resolve_visible_reference`,
  `crates/nx-codegen/src/builder.rs:2014`), which fails when the type was never imported, and no
  substitute spelling rescues it: under a wildcard alias `ui.Fit.cover` does not lower either
  (*"Member access not yet implemented: .cover"* — the original NXE14 finding). So this half needs
  canonical origin on nominal types, which is the nominal-identity change, not a slice of this one.

  Meanwhile a bare name whose type is not nameable in the using module now reports the needed import
  and the qualified form to write, instead of type checking and then emitting an `unresolved:` slot
  into generated IR. The `enum-values` requirement was restated to match what holds.
- **Closed by `nominal-type-identity`.** The deferred back end landed there. A resolved contextual
  name now rewrites to `ast::Expr::ResolvedUnionCase`, which carries the union's declaring origin
  rather than an `Ident` base resolved by visible name; code generation builds its `CodegenReference`
  from that origin (`build_union_case_from_origin`) and the interpreter reaches the declaration the
  same way (`eval_resolved_union_case`), so the base record shape an abstract-based case inherits is
  resolved in the declaring module. The `contextual-name-requires-import` guard is gone. Covering
  tests, all in `crates/nx-api/src/artifacts.rs` unless noted:
  `a_bare_case_of_an_unimported_union_evaluates_as_the_qualified_form_does`,
  `bare_name_at_an_imported_property_needs_no_import_of_its_type`,
  `bare_name_under_a_wildcard_alias_is_accepted`,
  `a_foreign_case_inheriting_an_abstract_base_carries_its_base_fields`,
  `a_visible_name_does_not_capture_a_reference_to_a_different_definition`, and
  `nx_ir_carries_no_unresolved_reference_for_a_case_of_an_unimported_union`
  (`crates/nx-codegen/src/tests.rs`). The same-name half closed there too — see RF6.
- **Verification:** Reopened. The front-end resolution tests pass, as does the full workspace suite,
  but the same-name isolation fix is incomplete. `EnumType` derives equality from only `name` and
  `members` (`crates/nx-types/src/ty.rs:409-414`), and `nominal_is_nameable_here` accepts a local enum
  when `local == info` (`crates/nx-types/src/infer.rs:2566-2570`). Therefore a local `Fit` with the
  same members as the foreign `Fit` is treated as nameable and structurally satisfies the foreign
  expected type in both the bare and qualified paths. The new collision test uses different members
  (`stretch | squish` versus `fill | contain | cover`), so it does not exercise this case. This still
  violates the amended scenario that a same-named local enum must not stand in for the declaring
  module's enum. Add a same-name, same-members workspace test and keep this finding open until nominal
  origin participates in identity, or narrow the scenario and task claims to the structural check
  actually delivered here.
- **Fix (verification follow-up):** Confirmed by reproduction, and the second remedy taken: the
  scenario and task claims are narrowed to what the structural check delivers. Measured against the
  working tree, with `widgets.nx` exporting `enum Fit = fill | contain | cover` and `<Img fit: Fit />`:

  | local `Fit` in the consuming module | `fit=cover` | `fit={Fit.cover}` |
  | --- | --- | --- |
  | `fill \| contain \| cover` (identical) | accepted silently | accepted silently |
  | `fill \| contain \| cover \| extra` | reported | `expects Fit, found Fit` |

  The identical-members row is worse than the report states for the bare form: it is not merely
  accepted structurally, it never reaches the `contextual-name-requires-import` guard at all, because
  `nominal_is_nameable_here` finds a local enum that compares equal and concludes the type is
  nameable here. What it lowers to is the *local* `Fit.cover`.

  That is an identity defect, not a value defect. An `EnumMember` carries only a name and a span — no
  ordinal, no backing value — and the wire form is the bare authored member string, so two
  member-identical enums produce the same value through either binding. Nothing observable is wrong
  today; what is missing is the ability to tell the two declarations apart at all.

  The `{name, members}` and `{name, cases, base}` equality this rests on is pre-existing and untouched
  by this change (`git diff crates/nx-types/src/ty.rs` adds only `Type::ContextualName`). This change
  therefore narrowed a pre-existing hole rather than opening or closing one: same-named enums with
  differing members are now rejected, which is the soundness bug the Fix block describes. Closing the
  remainder means putting declaring origin into identity, which is `nominal-type-identity`'s defining
  purpose and cannot be a slice of this change.

  So: the `enum-values` scenario is renamed to *A same-named local enum with different members does
  not stand in for the declaring module's enum* and its requirement now records that two same-named
  enums are told apart by declared members rather than by origin; task 10.2 records the same limit.
  `nominal-type-identity` tasks 1.1 and 2.3 gain the same-name, same-members workspace test the
  verification asked for, and its identity requirement gains a scenario that a same-named type is
  rejected even when it declares an identical shape — the trap this change's collision test fell into
  by using `stretch | squish`.

### ✅ Resolved (fix carried by `replace-enums-with-unions`) - RF2 The formatter identifies payloadless union cases by a dotted record name heuristic
- **Severity:** Medium
- **Evidence:** `payloadless_union_case` treats every empty `Value::Record` whose `type_name` contains
  a dot as a union case (`crates/nx-cli/src/format.rs:227`). Dotted record names are not a union-case
  discriminator: runtime records/actions/components can also carry qualified names, and intrinsic
  element values directly preserve the authored tag in `type_name`
  (`crates/nx-interpreter/src/interpreter.rs:2356`). Such a value is marked non-complex at
  `crates/nx-cli/src/format.rs:264` and emitted as only the last segment (for example `Empty`), which
  changes the value and generally will not type-check on reparse.
- **Recommendation:** Preserve explicit runtime type-kind metadata for union cases, or pass the
  expected schema/type into source formatting, instead of inferring the kind from `fields.is_empty()`
  plus a dot. Add a round-trip test containing an empty qualified non-union record/action alongside a
  payloadless union case.
- **Status:** Nothing further is owed by this change — **confirmed, and it silently corrupts the
  value**:

  ```
  let root() = { <div data={<foo.bar />} /> }
  //   nxlang run --format nx  =>  <div data=bar />
  ```

  The `foo.bar` record becomes the bare name `bar`, which reads back as something else entirely. Any
  empty qualified record hits this, including a no-prop aliased component such as `<ui.Button />`.

  Not fixed here, and deliberately not patched here either. The two fixes originally laid out were
  (a) give union-case values a runtime discriminator, and (b) drop the bare rendering for union
  cases in attribute position while keeping it for enums, recorded as the recommended stopgap.
  **(b) has since been disproved as a stopgap**: the non-bare path drops the property name when it
  renders a record-valued property as a child element, so it trades one silent corruption for
  another (see RF5, which that change now also carries). And (a) is no longer the right shape either — measured, it is a `kind` field on
  `Value::Record` touching ~218 construction sites, all to describe a distinction that should not
  exist in the first place.

  **Resolved by `replace-enums-with-unions`, which dissolves this rather than patching it.** Under
  that change a case that declares no fields in a union with no base is a *constant case* and
  evaluates to a scalar value, the same representation `Value::EnumValue` had. No union case is ever
  an empty record, so no empty record is ever rendered bare, `payloadless_union_case` is deleted
  rather than corrected, and an empty qualified record renders in element form and reads back as
  itself. The requirement text stops being self-conflicting because there is one closed nominal set
  rather than two representations of one.

  The covering test is task 1.3 of that change — a round-trip formatting an empty qualified record
  that is not a union case — which is written to fail against this change's code and pass after it.
  It passes only with the formatter rewrite in that change's task group 6 as well: deleting the
  heuristic alone reclassifies the empty record as complex and sends it into the path that drops the
  property name (RF5), so `<div data={<foo.bar />} />` would render as `<div><foo.bar /></div>`.

  Nothing is changed in this change for RF2. Rendering `<div data={<foo.bar />} />` as
  `<div data=bar />` is a live corruption until `replace-enums-with-unions` lands, and that is
  recorded here rather than worked around.
- **Fixed by `replace-enums-with-unions` (2026-08-30):** The heuristic is gone, and so is the value
  shape it guessed at — a payloadless case of a base-less union is now a `Value::UnionCase` scalar
  rather than an empty dotted record, so an empty record is unambiguously a record. Covered by
  `format::tests::test_format_empty_qualified_record_is_not_rendered_as_a_union_case` (that change's
  task 1.3), which asserts the value does not render as `<div data=bar />` and that both the property
  name and the record's own type name survive. It passed only once task group 6 landed, exactly as
  predicted here.
- **Verification:** The deferral is fully specified in the planning-complete
  `replace-enums-with-unions` change: task 1.3 pins the empty-qualified-record failure, task 5.4
  deletes `payloadless_union_case`, and task group 6 fixes the coupled property-rendering path. This
  is resolved as an explicit successor-change disposition, not verified as fixed in the current
  implementation; the live corruption remains until that successor is implemented.

### ✅ Resolved (fix carried by `nominal-type-identity`) - RF3 Property-value completions ignore imports and aliases
- **Severity:** Medium
- **Evidence:** Completion lookup searches a flat list of every workspace declaration and requires
  the raw declaration name to equal the authored tag (`crates/nx-language-service/src/lib.rs:1401`),
  then independently finds the first raw enum/union declaration with a matching type name
  (`crates/nx-language-service/src/lib.rs:1414`). A use such as `<ui.Img fit= />` therefore cannot
  match the declaration named `Img`, while unrelated or duplicate declarations from documents not
  visible to the current module can be selected. This misses the proposal's wildcard-alias use case
  and can offer members from the wrong nominal type.
- **Recommendation:** Derive the element and property type from the prepared workspace/import graph,
  preserving aliases and module identity, rather than joining raw declarations by strings. Cover
  selective imports, wildcard aliases, non-visible declarations, and duplicate type names in
  different modules.
- **Status:** Nothing further is owed by this change — accurate, but it describes the
  **pre-existing language-service architecture** rather than something this change introduced. `workspace_declarations`
  (`crates/nx-language-service/src/lib.rs:481`) is a flat `flat_map` over every document with no
  import or visibility filtering, and *every* completion context is built on it; the new
  `property_value_context` inherits that limitation rather than adding one. Making completions
  import-aware is a language-service feature in the same category as the go-to-definition and rename
  gaps recorded under tasks 7.3/7.4.

  The narrow part — matching `<ui.Img` against the declaration named `Img` by its last dotted
  segment — is a contained fix, but it is deliberately **held until RF1 lands**: while contextual
  resolution fails across module boundaries, offering members at `<ui.Img fit=` would suggest
  completions that then do not type check. These two should be fixed and tested together.

  **Carried by `nominal-type-identity`, and already specified there.** That change's
  `editor-language-service` delta requires element and property lookup to follow the document's
  import graph, forbids drawing completions from declarations the document cannot see, and forbids
  selecting a declaration by matching its name against an authored tag as plain text; it adds
  scenarios for an aliased element (`<ui.Img fit=`) and for a same-named declaration in an
  unimported document. Task group 8 (8.1–8.3) implements it, and its task 8.3 covers selective
  import, wildcard alias, non-visible declarations, and duplicate type names — the four cases this
  finding's Recommendation asks for. The coupling to RF1 is honoured by construction, since that
  group sits behind groups 3 and 4 in the same change.

  This change's own `editor-language-service` delta was narrowed to record the limitation rather
  than paper over it (task 9.4), so archiving promotes a description of the flat name-based lookup
  that exists rather than a promise of the import-aware lookup that does not.
- **Verification:** The current delta spec accurately records the flat workspace-name lookup, and
  the planning-complete `nominal-type-identity` change assigns the actual fix to tasks 8.1-8.3 with
  selective-import, wildcard-alias, non-visible-declaration, and duplicate-name coverage. This is a
  verified deferral; the completion implementation in this change remains intentionally
  import-unaware.
- **Closed by `nominal-type-identity`.** `workspace_declarations` — the flat join of every document's
  declarations by name — is replaced by `DocumentScope`, built from the prepared bindings
  `analyze_workspace_modules` returns, which is the resolution the compiler itself performed. A tag
  is resolved in the editing document's own namespace, and a property's declared type in the
  namespace of the module that declared the property. Covering tests in
  `crates/nx-language-service/src/lib.rs`: `member_completions_follow_a_selective_import`,
  `member_completions_follow_a_wildcard_import_alias`,
  `completions_are_not_drawn_from_a_document_that_is_not_imported`, and
  `member_completions_come_from_the_declaring_module_not_a_same_named_local_type`.

### ✅ Verified - RF4 The syntax AST task is marked complete without implementing the promised typed nodes or accessors
- **Severity:** Low
- **Evidence:** Task 1.7 requires `ast.rs` support and accessors for contextual-name text/span and
  signed-literal sign/numeric token (`openspec/changes/contextual-literal-binding/tasks.md:9`), but
  `crates/nx-syntax/src/ast.rs` has no `CONTEXTUAL_NAME` or `SIGNED_NUMERIC_LITERAL` node type or
  accessor. Only `SyntaxKind` and generated tree-sitter metadata were changed. This leaves public
  syntax consumers to manually inspect generic children and means the stated accessor verification
  does not exist.
- **Recommendation:** Add typed AST wrappers and focused accessor tests for both forms, or revise the
  task/artifacts explicitly if generic `SyntaxNode` access is the intended API.
- **Fix:** Took the second option and amended task 1.7. `crates/nx-syntax/src/ast.rs` holds wrappers
  for exactly six top-level declaration constructs (`FunctionDef`, `Element`, `TypeDef`, `UnionDef`,
  `RecordDef`, `ComponentDef`); no expression- or literal-level node has one, not even `int_literal`,
  `string_literal`, or `identifier`. Generic `SyntaxNode` access is the established API at that level
  and is how `crates/nx-hir/src/lower.rs:1196` reads both new forms, so wrappers for only these two
  would have been inconsistent with every neighbouring node. The task now scopes to `syntax_kind.rs`
  and `node-types.json` — both genuinely done — and records why the `ast.rs` half was dropped.
- **Verification:** Verified. Task 1.7 now explicitly adopts generic `SyntaxNode` access and records
  the dropped wrapper requirement; `ast.rs` still contains only the six documented top-level
  wrappers, while both syntax kinds and their tree-sitter name mappings are present. The full
  workspace suite, including `nx-syntax`, passes without warnings.

### ✅ Resolved (fix carried by `replace-enums-with-unions`) - RF5 Value output spells properties as elements
- **Severity:** High
- **Evidence:** `format_nested_element` takes the property key as `tag_name`, but its `Value::Record`
  arm ignores it entirely, delegating to `format_nested_record(type_name.as_str(), ...)`
  (`crates/nx-cli/src/format.rs:125-129`); its `Value::Array` arm does use the key, but as an element
  tag. NX has no property-element syntax — an element body binds to the single field marked
  `is_content` (`EffectiveRecordShape::content_property`, `crates/nx-hir/src/records.rs:17`) — so a
  property name written in body position has nowhere to go, and one written as a tag names an element
  that does not exist.
- **Recommendation:** Emit every field in property position as `key=<value>`, never as body content
  and never as an element named after the property. `rhs_expression` already admits `$.element`
  (`crates/nx-syntax/grammar.js:409-414`), so a record value needs no braces and a list needs them.
  Add a round-trip property test that formats, reparses, evaluates, and compares against the original
  value, rather than asserting on the emitted string.
- **Status:** Nothing further is owed by this change — **confirmed, and worse than first filed**. Measured directly against
  `format_value`:

  ```
  Person { name: "Alice", home: Address { city: "Boston" } }
  =>  <Person name="Alice">
        <Address city="Boston" />
      </Person>
  ```

  Fed back through `nx_types::check_str` that output does not compile, reporting two errors —
  `missing-content-property` ("Record 'Person' passes body content, but 'Person' does not declare a
  content field") and `missing-property` ("Element 'Person' requires property 'home'"). The failure
  is only loud because `Person` has no content property; where the target does declare one, the body
  binds to that field instead and the corruption is silent.

  Two further defects the original filing missed:

  ```
  Person { home: Address { city: "A" }, work: Address { city: "B" } }
  =>  <Person>
        <Address city="A" />
        <Address city="B" />
      </Person>                    // home and work are indistinguishable

  Box { items: [Item, Item] }
  =>  <Box>
        <items>
          <Item />
          <Item />
        </items>
      </Box>                       // `items` emitted as an element tag, not a property
  ```

  With two properties of one type the value is unrecoverable even by a human; only sorted field order
  survives. The array arm invents a component named after the property — silently wrong if one by
  that name exists, loud otherwise. So the defect is not the `Value::Record` arm in isolation: the
  whole nested-element path spells properties as elements.

  Every property-position spelling was verified to type check clean: `home=<Address city="Boston" />`
  unbraced, `home={<Address city="Boston" />}` braced, and `items={<Item /> <Item />}`. Supplying a
  content property by name rather than by body is explicitly legal — `content-properties` only
  forbids supplying it *both* ways — so the uniform property form needs no exception.

  **Corrected from the original filing: this is not independent of RF2, and it is now carried by
  `replace-enums-with-unions`.** Deleting `payloadless_union_case` makes `is_complex_value` return
  true for an empty qualified record, which routes it out of attribute position and into this path,
  so `<div data={<foo.bar />} />` would format to `<div><foo.bar /></div>` — a different wrong value.
  RF2's own repro therefore cannot round-trip until RF5 is fixed, and RF2's covering test (task 1.3
  of that change) stays red after the RF2 fix alone. The two are fixed together in task group 6 of
  `replace-enums-with-unions`, under design decision D7; tasks 1.5 and 1.6 pin the record and list
  cases as failing tests first.

  Two values are left with no NX spelling at all, exposed rather than created by the fix: an empty
  list (`items={}` does not parse; today it is emitted as `items="..."`, a string) and
  `Value::ActionHandler` (emitted as a synthetic `<ActionHandler ... />` that is not a real element).
  That change makes both fail loudly and files the empty-list spelling as a separate language
  question.
- **Verification:** The deferral is fully specified in `replace-enums-with-unions`: tasks 1.5 and
  1.6 pin record- and list-valued property failures, design D7 requires every field in property
  position, and tasks 6.1-6.6 implement and verify that formatter rewrite together with RF2. This is
  resolved as an explicit successor-change disposition; the current formatter defect remains live
  until that successor is implemented.

- **Fixed by `replace-enums-with-unions` (2026-08-30):** `format_nested_element` and
  `format_nested_record` are deleted. Every field is emitted in property position — a record value as
  an unbraced element (`home=<Address city="Boston" />`) and a list as a braced sequence
  (`items={<Item /> <Item />}`) — so no property name reaches element-tag position or body position.
  Covered by three round-trip tests in `crates/nx-cli/src/main.rs` (that change's tasks 1.5 and 1.6):
  `record_valued_property_round_trips`, `two_properties_of_the_same_record_type_stay_distinguishable`,
  and `list_valued_property_round_trips`. Each formats a value, re-evaluates the formatted source at
  the same typed site, and compares — so the guarantee is measured rather than asserted by shape.
### ✅ Resolved (fix carried by `nominal-type-identity`) - RF6 Same-name isolation was implemented for enums but not for unions
- **Severity:** High
- **Raised by:** this fix pass, while reproducing RF1's reopening. Not in the original review.
- **Evidence:** The enum path keeps the expected type resolved in the declaring module and checks
  membership against it (`crates/nx-types/src/infer.rs:1977`). The union path does not: it reduces the
  resolved type to its *name* (`Type::Union(info) => Some(info.name.clone())`,
  `crates/nx-types/src/infer.rs:2016`) and looks the definition back up by that name, local first
  (`union_def_for_contextual` = `union_defs.get(name).or_else(|| foreign_union_defs.get(name))`,
  `crates/nx-types/src/infer.rs:2592-2596`). `is_foreign` is computed the same way, so whenever the
  using module declares a union of that name, resolution silently proceeds against the *local*
  definition and the `contextual-name-requires-import` guard never fires.

  Measured, with `widgets.nx` exporting `type S = | idle | busy { n: int }` and `<Draw s: S />`:

  | consuming module | result |
  | --- | --- |
  | no local `S`; `<Draw s=idle />` | correctly resolves against the declaring module, reports the needed import |
  | local `S = \| idle \| busy { n: string }`; `<Draw s=idle />` | accepted silently |
  | local `S = \| idle \| busy { n: string }`; `<Draw s={<S.busy n="wrong" />} />` | accepted silently — a `string` reaches a field the declaring module typed `int` |
  | local `S = \| idle \| spinning`; `<Draw s=spinning />` | accepted silently — `spinning` is not a case of the expected union at all |

  The last two rows are value-affecting, and the last is the exact analogue of the pre-existing
  soundness bug RF1's fix closed for enums (`enum Fit = stretch \| squish` is rejected;
  `type S = \| idle \| spinning` is not). `UnionType` equality is over `{name, cases, base}` — case
  *names* only — so payload field types are not part of identity even where the resolved type is
  compared.
- **Recommendation:** Resolve a union case against the union the expected type already resolved to,
  rather than re-resolving its name; and make payload shape reachable from identity. Both follow from
  origin-based identity rather than needing a separate mechanism.
- **Status:** Nothing further is owed by this change. This is pre-existing behavior that the change
  preserved while fixing the enum half — at `HEAD` the expected type was `Type::Named("S")` and the
  local-first lookup produced the same acceptance — so it is not a regression. What the change did do
  is claim the fix for unions as well: the `discriminated-unions` delta said a bare case name resolves
  against "the one the declaring module named, not whatever the use site happens to spell the same
  way". That clause is now narrowed to the case where the use site declares no union of that name,
  with the precedence rule and its deferral recorded alongside. Task 10.2 records that its rejection
  is enum-only.
- **Verification:** `nominal-type-identity` task 1.1a pins the two unsound local-union collision rows
  above as failing tests; the no-local-union import diagnostic is already covered by this change's
  cross-module tests. Task 2.3a resolves the case against the expected type rather than by name. Its
  identity requirement gains *A same-named type is rejected even when it declares an identical
  shape*, which covers the payload-field-type row that case-name comparison cannot. The defect
  remains live until that change is implemented.
- **Closed by `nominal-type-identity`.** The same-name/different-case-names half closed earlier, in
  `replace-enums-with-unions`. The payload-field-type half closed here: `UnionType` and
  `UnionCaseType` carry a `DeclaringOrigin`, and their equality is over that origin rather than over
  `{name, cases, base}`, so two declarations that agree on every observable are still two types.
  Covering tests in `crates/nx-api/src/artifacts.rs`:
  `a_same_named_local_union_does_not_satisfy_a_foreign_union_with_matching_case_names` (the payload
  row), `a_same_named_local_union_is_rejected_even_when_it_declares_an_identical_shape` (identical
  shapes), `a_same_named_local_union_with_different_case_names_stays_rejected` (the half closed
  earlier, asserted so it stays closed), and `one_union_reached_under_two_names_stays_one_type` plus
  `identity_survives_a_rename_at_the_import_boundary` for the converse.

  The same defect in records and components closed there too, though neither was in scope when this
  finding was written. A record type is a `Type::Named`, which carried only a spelling, so two
  same-named records in different modules satisfied each other and a `string` reached a field the
  declaring module typed `int`. `Type::Named` now carries a `NamedType { name, origin }`, and record
  and component subtyping compares declarations along inheritance chains resolved in the module that
  wrote each `extends` clause. Covering tests, also in `crates/nx-api/src/artifacts.rs`:
  `a_same_named_local_record_does_not_satisfy_a_foreign_record`,
  `a_same_named_local_record_is_rejected_even_when_it_declares_an_identical_shape`,
  `a_property_typed_by_a_record_the_declaring_module_imported_does_not_bind_locally`,
  `a_local_lineage_spelled_like_a_foreign_one_does_not_satisfy_it`,
  `a_same_named_local_component_lineage_does_not_satisfy_a_foreign_one`, and — for the converse, that
  origin does not over-reject — `the_imported_record_a_property_names_still_satisfies_it`,
  `a_foreign_record_satisfies_a_base_the_consumer_cannot_name`, and
  `a_foreign_component_satisfies_a_base_the_consumer_cannot_name`.

## Questions
- None.

## Summary
- Six findings in total. Five were raised in the review: two high-severity failures — a cross-module
  resolution failure and a value formatter that spells properties as elements — two medium-severity
  formatter/language-service correctness gaps, and one low-severity incomplete syntax API task. A
  sixth (RF6) was raised by the verification follow-up.
- At review time the full workspace suite passed, but the contextual-literal tests were predominantly
  single-file and did not exercise the import/alias behavior central to the proposal. Four
  cross-module tests were added in `crates/nx-api/src/artifacts.rs` during the fix pass, which
  `check_str` could not express.
- Verification reopened RF1: same-named, same-shaped enums remain indistinguishable, contrary to the
  amended scenario. Reproducing that turned up RF6 — the same-name isolation the change delivered for
  enums was never extended to unions, where resolution re-looks-up the union by name and prefers a
  local declaration whatever cases it declares.
- Both are the same missing thing: nominal types carry no declaring origin, so identity is decided by
  spelling plus shape. That is `nominal-type-identity`'s defining purpose, and neither is a regression
  — the underlying equality is pre-existing and untouched here. This change narrowed the hole for
  enums; the delta specs and task 10.2 now say so rather than claiming it closed.
- After the follow-up, no finding is left open: one verified, one fixed in part here, and four
  resolved for this change with their fixes carried by `replace-enums-with-unions` and
  `nominal-type-identity`.

## Review-fix pass

**Fixed (1):** RF4 — task 1.7 amended to match the codebase's actual syntax-API convention.
Also fixed outside the numbered findings: the duplicate `#[test]` attribute noted in Verification.

**Fixed in part here, remainder carried elsewhere (1):** RF1 — the front end is implemented and
tested here; carrying a resolved member through lowering, and putting declaring origin into type
identity, are `nominal-type-identity`'s.

**Resolved for this change, fix carried elsewhere (4):** RF2 and RF5 to `replace-enums-with-unions`,
RF3 and RF6 to `nominal-type-identity`. Nothing further is owed here for any of them.

**Left open (0).**

**Carried fixes since landed (2026-08-30).** `replace-enums-with-unions` fixed RF2 and RF5, and each
entry above now names its covering test. RF6 was also closed in part there, ahead of schedule: the
unified resolution path required it. Collapsing two nominal kinds into one exposed the same
name-only union matching in two places — a `(UnionCase, Union)` compatibility rule in both
`crates/nx-types/src/ty.rs` and `crates/nx-types/src/infer.rs`, the second shadowing the first — and
leaving either would have regressed every enum, since `EnumType` equality had compared member lists.
Both now require the union to declare the case, and contextual resolution searches the resolved
type's own cases instead of looking the name up again. That closes RF6's same-name/different-case-
names half; the payload-field-type half is still `nominal-type-identity`'s, because `UnionType`
equality covers case names and not payload types.

How each was dispositioned. RF1 and RF2 were both **reproduced** and proved more concrete than the
report indicated. RF1 was an unimplemented spec requirement — both scenarios under `enum-values` →
*Enum members are referenceable without naming the enum type* failed, which is the proposal's
headline motivation. The requirement has since been restated as what holds; the capability itself is
what remains unreachable, and it needs a rewrite-target design decision that touches D2/Option A.
RF2 silently corrupts values today. RF3 is accurate but describes pre-existing language-service
architecture — `workspace_declarations` is flat and every completion context is built on it — and
its narrow half is deliberately coupled to RF1, so it belongs with the change that lands RF1's back
end rather than here. RF5 was found while investigating RF2's proposed stopgap; it was first filed
as an independent formatter defect, and on measurement proved both wider than filed — the whole
nested-element path spells properties as elements — and coupled to RF2, since fixing RF2 alone
routes an empty qualified record straight into it.

Task 9.2 was reopened — its "every scenario has a test" claim was wrong for RF1's two scenarios —
and then closed once the `enum-values` requirement was restated as what holds and its three
scenarios were covered by cross-module tests.

**Disposition of the findings.** RF1's back end and RF3 are carried by `nominal-type-identity`.
RF2 and RF5 are both carried by `replace-enums-with-unions`, and must be fixed together. That
change dissolves RF2: with constant union cases represented as scalar values, no union case is an
empty record, and the formatter heuristic is deleted rather than corrected. Deleting it is what
routes an empty qualified record into RF5's path, so the same change rewrites the formatter to emit
every field in property position (its design decision D7, task group 6). `replace-enums-with-unions`
also retires `nominal-type-identity`'s D5 and task group 6, which existed only to give union cases a
runtime discriminator.

**Verification reopened RF1, and the follow-up closed it by narrowing rather than by building.**
The amended same-name-isolation scenario overstated what the structural comparison delivers, so the
scenario, its requirement, and task 10.2 were narrowed to the differing-members case that is actually
rejected. Reproducing it surfaced RF6: the union path never got the enum path's treatment at all, and
accepts a local union declaring cases the expected union does not have. Both are pre-existing
consequences of identity-by-spelling, both are `nominal-type-identity`'s, and that change's task 1.1
and identity requirement were extended so its own tests cannot repeat this change's mistake of
pinning the collision case with a *differently* shaped type.

**The delta specs were scoped to what holds, so archiving promotes only true statements.** Three
requirements over-claimed against the shipped code and were narrowed:

- `discriminated-unions` → *Payloadless union cases support contextual construction* dropped
  *Resolution SHALL NOT require the union type to be in lexical scope at the use site*, and now
  states the same deferral `enum-values` already states: a case whose union is not nameable in the
  using module is reported as needing an import. `nominal-type-identity` restores the unconditional
  form.
- `editor-language-service` → *Language service exposes conservative completions* now records that
  lookup is by declaration name over the workspace snapshot rather than through the import graph, so
  a same-named declaration the document cannot see may be selected and an aliased element may not
  match. Its property-value scenario no longer claims unrelated declarations are excluded.
  `nominal-type-identity` replaces that paragraph with import-graph lookup.
- `unbraced-literal-forms` → *First-party NX-syntax value output round-trips* now scopes the
  guarantee to values whose fields are all scalars, and records that nested values and empty
  qualified records do not yet read back. `replace-enums-with-unions` removes the scope.

Two more were narrowed after verification, for the same reason:

- `enum-values` → *Enum members are referenceable without naming the enum type* now records that two
  same-named enums are told apart by their declared members rather than by origin, so same-named
  enums declaring the same members are not distinguished. Its same-name scenario is renamed to name
  the differing-members condition it actually tests (RF1).
- `discriminated-unions` → *Payloadless union cases support contextual construction* now records that
  union resolution reaches its definition by name and prefers a local union of that name, so
  resolution against the declaring module holds only where the use site declares no such union
  (RF6). `nominal-type-identity` restores the unconditional form for both.

The `discriminated-unions` scenarios were re-run against the working tree and all three pass; the
rest rest on task 9.2's coverage walk.

**This change is ready to archive.** RF1's amended claims are narrowed to what the change delivers,
RF6 is recorded with the same treatment, and every delta requirement now describes behavior that
holds against the working tree — so archiving promotes only true statements. The two identity gaps
are live defects, but they are pre-existing, are not what this change set out to fix, and are pinned
as failing tests in `nominal-type-identity` before any of its production changes.

**Suite after verification:** `cargo test` passes with 0 failures and no warnings.
