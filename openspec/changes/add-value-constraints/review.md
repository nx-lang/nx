# Review: add-value-constraints

## Scope
**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md`, `specs/{value-constraints,nx-ir-format,rust-ir-runtime,typescript-ir-runtime,declaration-schema-export,source-tree,viewer}/spec.md`  
**Reviewed code:** none changed (spec-only review). Checked for feasibility against `crates/nx-syntax/grammar.js`,
`crates/nx-types/src/infer.rs`, `crates/nx-codegen/src/builder.rs`, `crates/nx-ir/src/model.rs`,
`docs/nx-ir-format.md`, `runtime/typescript/src/index.ts`, `crates/nx-language-service/src/source_tree.rs`,
`packages/language-protocol/src/index.ts`, `packages/viewer/src/{edit,editors}.ts`, the existing specs
(`implicit-primitive-conversions`, `optional-properties`, `update-records`, `unbraced-literal-forms`,
`declaration-schema-export`, `rust-ir-runtime`, `viewer`, `visualizer-declarations`, `discriminated-unions`,
`range-expressions`, `contextual-numeric-literals`) and the website reference docs.  
**Validation:** `openspec validate add-value-constraints --strict` passes; every requirement has SHALL/MUST on its
first line. Scenario arithmetic (dispatch on ranges, `flag`, `g`, IR interval flags, schema keywords, viewer
stepping `0.9 → 1.0 → 1.0`) checks out.

## Findings

### ✅ Verified - RF1 The "leading operand gets a fix" scenario is a leftover that contradicts its own requirement.
- **Severity:** High (confidence: high)
- **Evidence:** `specs/value-constraints/spec.md:55-57` says that `type Count = int where >= 1` SHALL be reported as
  `invalid-constraint` with the fix `int where >= 1`. That input is the valid operand-free form used everywhere
  else (`design.md:80`, `spec.md:24`, the schema delta's `type Count = int where >= 1`). The scenario was
  evidently `where value >= 1` in the first draft, and the edit stripped the operand from the input as well.
  As written, an implementer must reject the canonical form.
- **Recommendation:** Change the WHEN to `type Count = int where value >= 1` and keep the THEN, which suggests
  `int where >= 1`.
- **Verification:** `specs/value-constraints/spec.md:59-61` now uses `int where value >= 1` and suggests `int where >= 1`.

### ✅ Verified - RF2 The check node can only name a kind-7 type, but the spec requires runtime checks on sequences and optional sites.
- **Severity:** High (confidence: high)
- **Evidence:** `specs/nx-ir-format/spec.md:29-35` says `[28, operand, type, site]` takes a `type` that "is a kind-7
  type entry" and that the emitter SHALL write it "exactly where `value-constraints` requires a runtime check".
  `specs/value-constraints/spec.md:147-148` (and `design.md:208`) require checks on "each item of a `for` result
  or sequence bound to a constrained item type". So binding a `float32+` parameter `xs` or a `for` result to a
  `Ratio+` field needs an item-wise check. Optional sites behave the same way: a `p?: Ratio` parameter or field
  has read type `Ratio?`, a `seq` (`docs/nx-ir-format.md:94` and the paragraph after the type table). The node as
  specified cannot express either case. The emitter would have to synthesize a `for` or an `if`, which is
  neither specified nor counted as "one operation".
- **Recommendation:** Let kind 28's `type` also be a `seq` entry whose item is kind 7, and specify the
  semantics: check each item, let the empty value pass, and count one operation per item checked. Add a
  scenario that binds a computed `float32+` to `Ratio+` and one that passes an optional argument. Alternatively,
  specify the emitter lowering explicitly.
- **Verification:** `specs/nx-ir-format/spec.md:34-41` lets kind 28 check each item of a sequence and lets the empty value pass. The new "A check node on a sequence" scenario and the `value-constraints` "Each item of a computed sequence" scenario are correct (`{ 1 0 }` gives `0, -1`, which fails at the second item). `design.md` decision 6 and task 3.2 agree. The cost of an item-wise check is tracked as RF17.

### ✅ Verified - RF3 Making `where` a keyword after a type breaks a field or prop named `where` that follows an unconstrained field.
- **Severity:** Medium (confidence: high)
- **Evidence:** The grammar sets `word: $ => $.identifier` (`grammar.js:66`). With keyword extraction,
  tree-sitter returns the keyword whenever it is valid in the current parse state. After `a: int` in a
  whitespace-separated field list (`property_definition`, `grammar.js:470-503`) or a component signature
  (`grammar.js:376-388`), both a `where_clause` and a next field name are valid. So
  `type R = { a: int  where: string }` and `component <C a:int where:int />` would lex `where` as the keyword
  and fail at `:`. GLR cannot recover, because the lexing decision is already made. This contradicts
  `spec.md:8-9` ("a declaration, field or parameter named `where` elsewhere SHALL be unaffected") and
  `proposal.md:68` ("no existing program changes meaning"). The only scenario (`spec.md:28-30`) puts `where:`
  after a constrained field, the one place where it does work.
- **Recommendation:** In the grammar, also admit the `where` token as a field name
  (`alias('where', $.markup_identifier)` in the name slot, with a declared conflict that `:` settles). Add
  scenarios for `{ a: int  where: string }` and for a component prop, plus a task 1.1 corpus test.
- **Verification:** `spec.md:8-10` now covers a field or prop named `where` directly after another field's type, decided by `:` or `?:`. The scenario adds `{ limit: int  where: string ... }` and an element-function parameter `where?:`, and task 1.1b covers records, component props and parameters. Aliasing the keyword token as a name, with a conflict that `:` settles, works with tree-sitter keyword extraction: in identifier-only positions such as expressions and attributes, the keyword is not valid and still lexes as an identifier.

### ✅ Verified - RF4 The viewer resolves a number editor from the type's spelling, and spelling the constraint "as written" defeats it.
- **Severity:** Medium (confidence: high)
- **Evidence:** `packages/viewer/src/edit.ts:61,216,305-324,353` finds a value's editor by stripping `?+*` from
  `SourceProperty.type` and looking the result up in `NUMBER_TYPES` or the alias table. `source-tree`
  (`specs/source-tree/spec.md:9`) requires `type` to "spell the constraint as written", and an alias
  declaration's `type` would be `float32 where in 0.0..=1.0`. The viewer would therefore stop treating
  `opacity: float64 where in 0.0..=1.0` (or `Unit`) as a number at all, and the viewer scenarios
  (`specs/viewer/spec.md:16-31`) could not pass. Neither the specs nor tasks 5.3/6.1 mention this. Other
  spelling consumers (hint resolution in `hintsOf`, the `declaration` role in `declaredType`) are affected the
  same way.
- **Recommendation:** Either have the viewer strip a trailing `where …` clause before resolving (state this in
  the viewer delta and task 6.1), or add an unconstrained `base` spelling beside `bounds` in `SourceProperty` and
  alias declarations. Add a scenario: an inline-constrained property gets a number editor.
- **Verification:** The viewer delta now requires a constrained numeric type, spelled with `where`, to be edited as a number. Task 6.0 targets the `NUMBER_TYPES` lookups in `edit.ts`, aliases included.

### ✅ Verified - RF5 Constraints on optional properties, and numeric patterns on `T?` scrutinees, are unspecified.
- **Severity:** Medium (confidence: medium-high)
- **Evidence:** `optional-properties` says a property `name?: T` "reads as a type that admits zero", so its read
  type is `T?` (`docs/nx-ir-format.md`, type-table notes). `value-constraints` rejects a constraint "on a type
  with an occurrence suffix" (`spec.md:63-66`) but never says whether `opacity?: float64 where in 0.0..=1.0` or
  `opacity?: Ratio` is allowed. It also does not say that the empty value passes the check, both at runtime and
  at the host boundary, where an omitted optional must not be refused. `spec.md:169` requires "a scrutinee of a
  numeric type" for numeric patterns. That leaves open whether `if size is { > 10 => … }` on an optional
  `size?: int` (type `int?`) is accepted, as literal and `{}` patterns are today (`if.md:87`).
- **Recommendation:** Add a requirement and scenarios: a constrained type is allowed after `name?:`; an absent
  or empty value is not checked, at compile time, runtime or the boundary; a numeric pattern accepts a `T?`
  scrutinee and never matches the empty value, which `{}` or `else` handles.
- **Verification:** `value-constraints` allows `name?: Ratio` and `Ratio*`, applies the constraint to each value present and lets the empty value pass (compile time, `spec.md:112-116`; runtime, `spec.md:170-174`; boundary, through the general rule). The new "An optional constrained property" scenario covers it. `int?` scrutinees are accepted, and the empty value matches only `{}` or `else` (`spec.md:202-206`).

### ✅ Verified - RF6 tasks.md says "joins drop the constraint", which no spec states and which conflicts with the existing join rule.
- **Severity:** Medium (confidence: high)
- **Evidence:** `tasks.md:11` contains "joins drop the constraint". `implicit-primitive-conversions` ("A join takes the
  narrowest type its branches widen to", `openspec/specs/implicit-primitive-conversions/spec.md:446`) would
  make `if c { r1 } else { r2 }` with two `Ratio` branches a `Ratio`, since `Ratio` satisfies `Ratio`. No delta
  modifies that requirement, and `design.md` decision 4 is silent. Behaviour differs observably: whether
  `let x: Ratio = if c { r } else { s }` needs a runtime check, whether the IR has a check node, and what hover
  shows.
- **Recommendation:** Decide and specify. Either keep the constraint when every branch has the same constrained
  type (and take the interval hull, or the base, otherwise), or drop it, which needs a MODIFIED join
  requirement. Add a scenario for if/else and match-arm joins, including a branch that is a literal (`0.5`).
- **Verification:** `value-constraints` now specifies joins (`if`, match, `??`, sequence items), with the "Joins keep a shared constraint only" scenario. Design decision 4 and task 2.2 agree. A residual conflict with the existing join requirement is tracked as RF18.

### ✅ Verified - RF7 Host entry points other than arguments and props are not checked.
- **Severity:** Medium (confidence: medium-high)
- **Evidence:** The runtime deltas (`specs/rust-ir-runtime/spec.md:5-9`, `specs/typescript-ir-runtime/spec.md:5-9`)
  cover only "an argument, a component prop, or a field nested anywhere inside either". The component
  lifecycle (`openspec/specs/rust-ir-runtime/spec.md:199-212`) also takes host values as explicit state,
  normalized complete state, state patches or update records applied to a state, and dispatched action
  records. A host could therefore set a `count: int where >= 0` state field to `-1`, or dispatch
  `<Slide value=1.5/>` with `value: Ratio`, and nothing would check it. The function body would then not be
  able to "rely on" the type, as `design.md:216` promises.
- **Recommendation:** Extend both runtime requirements to every host-supplied value whose declared type is
  constrained: state, patches and update records, dispatched actions, and external function or component
  results if those are typed. Use `nx-ir-boundary-constraint` and a path. Add one lifecycle scenario.
- **Verification:** Both runtime deltas now cover component state, state and record patches, dispatched action records and nested fields, and task 4.1 lists them.

### ✅ Verified - RF8 For `float32`, an exclusive end makes the schema wider than the runtime, and the bound's stored value is unspecified.
- **Severity:** Medium (confidence: high)
- **Evidence:** The TS runtime rounds a non-integer host `float32` with `Math.fround`
  (`runtime/typescript/src/index.ts:5089-5093`) before any comparison (`typescript-ir-runtime` delta line 11).
  For `float32 where < 1.0`, the host value `0.99999999` satisfies `exclusiveMaximum: 1` but rounds to `1.0` and
  is refused. That breaks "SHALL NOT be wider" (`openspec/specs/declaration-schema-export/spec.md:653-654`),
  which the schema delta (`spec.md:9-12`) claims still holds. The same applies to `> a`. Two related gaps:
  - The constant table has no float32 kind, only `float` as f64 (`docs/nx-ir-format.md:116-118`), yet the IR
    scenario (`specs/nx-ir-format/spec.md:16-18`) speaks of "the `float32` constants 0.0 and 1.0".
  - The schema delta does not say whether it writes the bound as written (`0.1`) or as its float32 value
    (`0.10000000149…`).
- **Recommendation:** Specify that a float32 bound is stored as the f64 of its float32 rounding and that the
  schema writes it as written. For an exclusive float32 end, write the next float32 inward as an inclusive
  `minimum`/`maximum`, or accept a narrower schema in some other way. Add a scenario at a float32 exclusive end.
- **Verification:** The schema delta writes `float32` ends as inclusive `float32` values, and an exclusive end as the nearest `float32` inside. With monotone rounding, every schema-valid value then rounds inside the interval. The scenario value `0.9999999403953552` is the largest `float32` below `1.0`. The IR delta stores `float32` bounds pre-rounded, as float constants. Integer exclusive ends are normalized, so the schema writes `int where > 0` as `minimum: 1`.

### 🟡 Fixed - RF9 The viewer delta silently changes existing stepping rules and leaves edge cases open.
- **Severity:** Medium (confidence: medium)
- **Evidence:** The existing viewer requirement says number arrows "step by 1" and steppers step "by `step`, or by
  one" (`openspec/specs/viewer/spec.md:416-418,638`). The ADDED requirement (`specs/viewer/spec.md:11-14`) derives
  `0.1` for a bounded float with no `step` hint. That modifies those requirements without a MODIFIED delta, so
  the archived spec would contradict itself. Three cases are also unspecified:
  - What stepping a float toward an exclusive end does ("SHALL NOT step onto an exclusive end", but where does
    it stop?).
  - What happens when hints and bounds have an empty intersection (hint `min=2000` with a `..=1000` constraint).
    `visualizer-declarations` (`spec.md:235-257`) already defines this case for hints merging to empty.
  - Whether `editor-hint-mismatch` (`visualizer-declarations` `spec.md:230`, "`int` … or an alias of one")
    treats an inline `int where …` as numeric.
- **Recommendation:** Turn the step change into a MODIFIED version of the existing requirements. Specify that a
  float step toward an exclusive end leaves the value unchanged. Define the empty intersection: the editor uses
  the bounds alone, and analysis warns with `editor-hint-range` on a hint outside the constraint. Mention
  constrained types in the hint-fit rule.
- **Verification:** Mostly fixed. The delta is now explicitly additive for properties with `bounds`, a hint range that does not overlap the bounds is ignored, and a step that would reach or pass an exclusive end leaves the value unchanged. One item from the finding is still open: `visualizer-declarations` (`openspec/specs/visualizer-declarations/spec.md:229-230`) still accepts `min`/`max`/`step`/`editor=field` only on "`int` … `float64` or an alias of one". The change never says that an inline `int where …` type fits those hints, so an implementation that matches primitive types could warn `editor-hint-mismatch` on `columns: int where in 1..=1000` with `max=12`, which is the viewer delta's own scenario. Add one sentence, to the viewer or source-tree delta or as a `visualizer-declarations` delta, saying that a constrained numeric type fits numeric hints as its base does.
- **Fix (remaining item):** New `visualizer-declarations` delta: a constrained numeric type, inline or through an alias, takes numeric hints as its base does, and a hint outside the interval is not a mismatch (the viewer intersects). Two scenarios; task 6.0a.

### ✅ Verified - RF10 Source-tree `bounds` miss several places the viewer edits.
- **Severity:** Low (confidence: medium-high)
- **Evidence:** `specs/source-tree/spec.md:4-6` limits `bounds` to entries of `properties` and `state` and to
  alias declarations. The viewer also edits union-case fields (`SourceDeclaration.cases[].properties`,
  `source_tree.rs:254-256`) and a parameterless `let`'s value (`isWholeValue`, `edit.ts:225-236`, with its type
  in `SourceDeclaration.ty`, `source_tree.rs:258`), so `let x: Ratio = 0.5` and `| tint { amount: Ratio }` get
  no bounds. The spec also does not say whether a `Ratio+` property, whose sequence items the viewer edits,
  carries its item type's bounds.
- **Recommendation:** Extend the requirement to union-case properties, value declarations and the item type of
  a sequence property, and add a scenario for each of the first two.
- **Verification:** The source-tree delta now covers union-case properties, annotated value declarations, and sequence or optional types whose item type is constrained, with integer ends given inclusive. Task 5.3 matches. Optional: no scenario was added for the new places.

### ✅ Verified - RF11 The schema-agreement scenario does not list the new boundary code.
- **Severity:** Low (confidence: high)
- **Evidence:** `openspec/specs/declaration-schema-export/spec.md:657-664` ("Schema-valid arguments are accepted")
  enumerates `nx-ir-boundary-type`, `nx-ir-boundary-field` and `nx-ir-arguments`. The delta says the corpus
  SHALL include constrained types but does not modify this scenario. A schema that is wider than a constraint
  would therefore still satisfy the scenario as written.
- **Recommendation:** Add a MODIFIED version of that requirement that includes `nx-ir-boundary-constraint` (or
  "any `nx-ir-boundary-*` code").
- **Verification:** The schema delta states that no schema-valid argument in the corpus SHALL fail with `nx-ir-boundary-constraint`. This closes the gap without a MODIFIED copy of the existing scenario.

### ✅ Verified - RF12 A constrained alias imported from an interface-only module would lose its constraint in the IR.
- **Severity:** Low (confidence: medium)
- **Evidence:** `crates/nx-codegen/src/builder.rs:3105-3123` resolves aliases to their target, except that "an
  alias declared by a module compiled to an interface alone has no target to read here, and stays the nominal
  reference it was before". Such a `Ratio` would be emitted as `[1, ref]` and not as kind 7, so the boundary and
  the emitter would skip the check. That contradicts the IR delta's claim that the boundary needs no new
  metadata (`design.md:247-248`).
- **Recommendation:** Require module interfaces to carry a constrained alias's target, or at least its interval,
  and add a cross-module scenario.
- **Verification:** `specs/nx-ir-format/spec.md:15-17` requires a constrained alias from another module, interface-only included, to be written as a kind-7 entry. Design decision 6 and task 3.2, which cites the `builder.rs` behaviour, agree.

### ✅ Verified - RF13 Documentation tasks miss pages that the change makes wrong.
- **Severity:** Low (confidence: high)
- **Evidence:** Task 7.1 (`tasks.md:43`) covers `syntax/types.md`, the style guide and `libraries/visualizer.md`.
  It omits these pages:
  - `syntax/if.md:54-58`, which lists the pattern forms and does not cover numeric patterns, narrowing, or the
    arm-body rule.
  - `concepts/declaration-schemas.md:46`, the mapping table, which needs the bound keywords.
  - The `min`/`max` doc comments in `libraries/visualizer.md:288-314`, and the visualizer library source they
    come from ("Not a constraint: a document may write a larger one"), which become misleading once a
    constraint caps the editor.
- **Recommendation:** Add these pages to task 7.1, and add the `edit.ts` spelling work (RF4) to task 6.1.
- **Verification:** Task 7.1 now covers `if.md`, `declaration-schemas.md`, the style guide's float-bound advice, and the `@nx/visualizer` `min`/`max` doc comments. Task 6.0 covers the `edit.ts` work.

### ✅ Verified - RF14 Small inconsistencies across the artifacts.
- **Severity:** Low (confidence: high)
- **Evidence:**
  - The occurrence fix-it is `Positive+` in `design.md:181` and `Score+` in `tasks.md:10` and `spec.md:80`.
  - `proposal.md:14-15` lists only type arguments and function types as excluded, and leaves out occurrences.
  - The headline example `opacity: float64 where in 0..=1 = 1` (`design.md:84,142,312`) uses the integer bound
    form that decision 7's style guide advises against.
  - "Constant expression" (`spec.md:120`) is never defined. Today's folder (`fold_constant`,
    `crates/nx-types/src/infer.rs:7041-7085`) folds only literal arithmetic, and the conversion path skips
    folding at `int` sites (`infer.rs:6884-6890`). An implementer needs to know that `let n = 0` followed by
    `fade(n)` is a runtime check, not a compile-time error.
- **Recommendation:** Pick one fix-it name. Mention occurrences in the proposal. Use `0.0..=1.0` in the design
  examples. Define a constant expression as "arithmetic whose operands are all numeric literals" and add a
  scenario with a named `let`.
- **Verification:** The fix-it is `Score+` everywhere. The proposal mentions sequences, the design examples use `0.0..=1.0`, and the spec defines a constant expression (`spec.md:146-148`). No scenario was added for a named `let` operand, which is acceptable.

### ✅ Verified - RF15 `int64` bounds beyond 2^53 are unspecified for the TypeScript runtime and the schema.
- **Severity:** Low (confidence: medium)
- **Evidence:** A bound such as `int64 where <= 9007199254740993` is a `bigint` constant (`docs/nx-ir-format.md:117`).
  The TS runtime takes `int64` host values as JS numbers (`index.ts:5065-5071`), and a JSON Schema `maximum` of
  that size loses precision in JS validators. The deltas do not say how either runtime compares such bounds,
  or what the schema writes.
- **Recommendation:** Either restrict bounds to ±(2^53−1) for this change (`invalid-constraint`), or specify the
  comparison and the schema output for large `int64` bounds.
- **Verification:** `int64` bounds outside ±(2^53−1) are rejected with `invalid-constraint` (`spec.md:71-73`), and design decision 6 and task 2.1 agree.

## New Findings Discovered During 2026-10-31 Verification

### 🟡 Fixed - RF16 An IR scenario still encodes integer exclusive ends, which the fixed requirement forbids.
- **Severity:** Medium (confidence: high)
- **Evidence:** `specs/nx-ir-format/spec.md:13-14` now says "An integer base's exclusive ends SHALL be written
  as the inclusive ends next to them". The "Two alternatives in one arm" scenario (`spec.md:67-70`) scrutinizes
  `n: int` and still expects "a high end `0`, exclusive" and "a low end `100`, exclusive". Under the new rule
  these are high end `-1` and low end `101`, both inclusive (flags `0`). The requirement says
  "constrained type" in places, so it is also unclear whether normalization applies to kind-29 pattern
  intervals or only to kind-7 entries used as types. Both nodes reference kind-7 entries.
- **Recommendation:** Update the scenario to `-1` and `101`, inclusive, and state that the rule covers every
  kind-7 entry, including those referenced by interval patterns. Alternatively, switch the scenario to a
  `float64` scrutinee if the scenario is meant to show exclusive flags.
- **Fix:** The scenario now expects `-1` and `101`, inclusive, and the kind-7 rule says integer ends are normalized in every kind-7 entry, including those interval patterns refer to.

### 🟡 Fixed - RF17 An item-wise check costs "one operation" regardless of how many items it checks, and its sentence no longer parses.
- **Severity:** Low (confidence: high)
- **Evidence:** `specs/nx-ir-format/spec.md:41` says the node SHALL count as one operation, and design decision
  6 says the same. Since the RF2 fix, one node can check a sequence of any length. The cost model exists to
  bound evaluation (`rust-ir-runtime`, "bounds evaluation" and the operation-count requirements), so O(n) work
  would cost O(1). Separately, the RF2 fix spliced the sequence clause into the middle of the sentence
  (`spec.md:36-39`: "…admits it; when the operand is a sequence …, so one node serves … , and otherwise SHALL
  fail…"). As written, "otherwise" reads as attached to the sequence clause.
- **Recommendation:** Count one operation per item checked, with the empty value costing one, and match the
  corpus wording ("record each check as one operation"). Split the sentence: first the single-value rule
  with its failure, then a second sentence on sequences and the empty value.
- **Fix:** Split the sentence (single value and failure first, then sequences); each value checked costs one operation, item by item; corpus scenario, design decision 6 and task 4.1 agree.

### 🟡 Fixed - RF18 The new join rule contradicts the existing "narrowest type" join requirement for nested constrained types.
- **Severity:** Low (confidence: medium)
- **Evidence:** `implicit-primitive-conversions` (`openspec/specs/implicit-primitive-conversions/spec.md:446-449`)
  says a join is "the narrowest type every branch's item type widens to". For `Ratio` and
  `Low = Ratio where <= 0.5`, that type is `Ratio`, since `Low` satisfies `Ratio` with no check. The new rule
  (`specs/value-constraints/spec.md:112-114`) yields `float32`. The design records the choice deliberately
  (decision 4, which rejects interval hulls), but the archived specs would state both rules.
- **Recommendation:** Either add a MODIFIED `implicit-primitive-conversions` join requirement that defers
  constrained types to `value-constraints`, or word the new rule as an explicit exception ("notwithstanding
  the narrowest-type rule"). Add `Ratio` with `Low` to the join scenario so the choice is tested.
- **Fix:** The join rule now follows the existing narrowest-type rule, read through satisfaction: `Ratio` with `Low` joins to `Ratio`; a constrained type with an unrelated one joins to the bases, unconstrained; no hull is formed. Scenario extended with both pairs; design decision 4 and task 2.2 agree.

### 🟡 Fixed - RF19 The new `unreachable-pattern` warning has no scenario.
- **Severity:** Low (confidence: high)
- **Evidence:** `specs/value-constraints/spec.md:203-205` and design decision 7 introduce `unreachable-pattern`,
  and task 2.4a lists it. No scenario covers it, and nothing contrasts it with `empty-constraint`.
- **Recommendation:** Add a scenario, for example `let f(x: Ratio) = { if x is { > 2.0 => 1 else => 0 } }`
  reports the warning `unreachable-pattern` at `> 2.0` and no error. Pair it with `> 5.0 && < 2.0`, which
  reports `empty-constraint`.
- **Fix:** Added the scenario "An unreachable pattern and an empty one" (`> 2.0` on `Ratio` warns `unreachable-pattern`; `> 1.0 && < 0.0` is `empty-constraint`; never both at one pattern).

## Questions
Answered in the spec (2026-10-31): an arm outside a constrained scrutinee's interval is the warning
`unreachable-pattern`, and a pattern whose own terms match nothing stays `empty-constraint`; integer exclusive
ends are normalized to inclusive ones, so `int where > 0` and `int where >= 1` are one type.

- When an arm's pattern lies wholly outside a constrained scrutinee's interval (`x: Ratio`, `> 2.0 =>`), should
  `empty-constraint` (an error) apply, as `spec.md:172-173` reads? Or should it be a warning, given that a dead
  literal arm (`5 =>`) is not an error today?
- Should `int where > 0` and `int where >= 1` be normalized to one type (same IR entry and schema), or only be
  mutually satisfying?

## Summary
- The design is coherent and largely feasible. The arm-body fix is plausible with tree-sitter GLR, since
  `value_if_match_arm` is already in a declared conflict, and the IR kind numbers (type 7, nodes 28 and 29) are
  free. Two high-severity issues need fixing before implementation starts: an inverted leftover scenario (RF1)
  and a check node that cannot express the sequence and optional checks the spec requires (RF2). The medium
  findings are a grammar regression for fields named `where` (RF3), a viewer feasibility gap (RF4), and
  under-specified optional properties, joins, host entry points and float32 schemas (RF5–RF9).
- Verification (2026-10-31): 14 of 15 findings verified. RF9 is reopened for one remaining item: numeric editor
  hints on inline constrained types. New: RF16 (Medium, a leftover IR scenario with exclusive integer ends) and
  RF17–RF19 (Low). `openspec validate add-value-constraints --strict` passes.
