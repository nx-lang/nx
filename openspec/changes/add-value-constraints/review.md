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

### 🟡 Fixed - RF1 The "leading operand gets a fix" scenario is a leftover that contradicts its own requirement.
- **Severity:** High (confidence: high)
- **Evidence:** `specs/value-constraints/spec.md:55-57` says that `type Count = int where >= 1` SHALL be reported as
  `invalid-constraint` with the fix `int where >= 1`. That input is the valid operand-free form used everywhere
  else (`design.md:80`, `spec.md:24`, the schema delta's `type Count = int where >= 1`). The scenario was
  evidently `where value >= 1` in the first draft, and the edit stripped the operand from the input as well.
  As written, an implementer must reject the canonical form.
- **Recommendation:** Change the WHEN to `type Count = int where value >= 1` and keep the THEN, which suggests
  `int where >= 1`.

### 🟡 Fixed - RF2 The check node can only name a kind-7 type, but the spec requires runtime checks on sequences and optional sites.
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

### 🟡 Fixed - RF3 Making `where` a keyword after a type breaks a field or prop named `where` that follows an unconstrained field.
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

### 🟡 Fixed - RF4 The viewer resolves a number editor from the type's spelling, and spelling the constraint "as written" defeats it.
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

### 🟡 Fixed - RF5 Constraints on optional properties, and numeric patterns on `T?` scrutinees, are unspecified.
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

### 🟡 Fixed - RF6 tasks.md says "joins drop the constraint", which no spec states and which conflicts with the existing join rule.
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

### 🟡 Fixed - RF7 Host entry points other than arguments and props are not checked.
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

### 🟡 Fixed - RF8 For `float32`, an exclusive end makes the schema wider than the runtime, and the bound's stored value is unspecified.
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

### 🟡 Fixed - RF10 Source-tree `bounds` miss several places the viewer edits.
- **Severity:** Low (confidence: medium-high)
- **Evidence:** `specs/source-tree/spec.md:4-6` limits `bounds` to entries of `properties` and `state` and to
  alias declarations. The viewer also edits union-case fields (`SourceDeclaration.cases[].properties`,
  `source_tree.rs:254-256`) and a parameterless `let`'s value (`isWholeValue`, `edit.ts:225-236`, with its type
  in `SourceDeclaration.ty`, `source_tree.rs:258`), so `let x: Ratio = 0.5` and `| tint { amount: Ratio }` get
  no bounds. The spec also does not say whether a `Ratio+` property, whose sequence items the viewer edits,
  carries its item type's bounds.
- **Recommendation:** Extend the requirement to union-case properties, value declarations and the item type of
  a sequence property, and add a scenario for each of the first two.

### 🟡 Fixed - RF11 The schema-agreement scenario does not list the new boundary code.
- **Severity:** Low (confidence: high)
- **Evidence:** `openspec/specs/declaration-schema-export/spec.md:657-664` ("Schema-valid arguments are accepted")
  enumerates `nx-ir-boundary-type`, `nx-ir-boundary-field` and `nx-ir-arguments`. The delta says the corpus
  SHALL include constrained types but does not modify this scenario. A schema that is wider than a constraint
  would therefore still satisfy the scenario as written.
- **Recommendation:** Add a MODIFIED version of that requirement that includes `nx-ir-boundary-constraint` (or
  "any `nx-ir-boundary-*` code").

### 🟡 Fixed - RF12 A constrained alias imported from an interface-only module would lose its constraint in the IR.
- **Severity:** Low (confidence: medium)
- **Evidence:** `crates/nx-codegen/src/builder.rs:3105-3123` resolves aliases to their target, except that "an
  alias declared by a module compiled to an interface alone has no target to read here, and stays the nominal
  reference it was before". Such a `Ratio` would be emitted as `[1, ref]` and not as kind 7, so the boundary and
  the emitter would skip the check. That contradicts the IR delta's claim that the boundary needs no new
  metadata (`design.md:247-248`).
- **Recommendation:** Require module interfaces to carry a constrained alias's target, or at least its interval,
  and add a cross-module scenario.

### 🟡 Fixed - RF13 Documentation tasks miss pages that the change makes wrong.
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

### 🟡 Fixed - RF14 Small inconsistencies across the artifacts.
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

### 🟡 Fixed - RF15 `int64` bounds beyond 2^53 are unspecified for the TypeScript runtime and the schema.
- **Severity:** Low (confidence: medium)
- **Evidence:** A bound such as `int64 where <= 9007199254740993` is a `bigint` constant (`docs/nx-ir-format.md:117`).
  The TS runtime takes `int64` host values as JS numbers (`index.ts:5065-5071`), and a JSON Schema `maximum` of
  that size loses precision in JS validators. The deltas do not say how either runtime compares such bounds,
  or what the schema writes.
- **Recommendation:** Either restrict bounds to ±(2^53−1) for this change (`invalid-constraint`), or specify the
  comparison and the schema output for large `int64` bounds.

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
