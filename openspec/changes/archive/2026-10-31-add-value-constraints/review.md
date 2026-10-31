# Review: add-value-constraints

## Scope
**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md`, and every delta spec under
`specs/` (`value-constraints`, `nx-ir-format`, `rust-ir-runtime`, `typescript-ir-runtime`,
`declaration-schema-export`, `source-tree`, `viewer`, `visualizer-declarations`). `proposal-review.md`
is the earlier proposal review and was not revisited.  
**Reviewed code:** the implementation commits `b7ba704..6d6dd03` (`git diff origin/main...HEAD`):
`crates/nx-syntax` (grammar, validation), `crates/nx-hir` (types, lowering, rewrites),
`crates/nx-types` (`ty.rs`, `infer.rs`, `infer/constraints.rs`, `semantics.rs`, `check.rs`),
`crates/nx-ir`, `crates/nx-codegen` (`builder.rs`, `ir.rs`, `emit.rs`, `runtime.rs`),
`crates/nx-ir-runtime` (`eval.rs`, `module.rs`, `normalize.rs`), `runtime/typescript/src/index.ts`,
`crates/nx-interpreter`, `crates/nx-api` (`schema.rs`, `artifacts.rs`), `crates/nx-language-service`
(`lib.rs`, `source_tree.rs`), `packages/language-protocol`, `packages/viewer` (`edit.ts`,
`editors.ts`), `packages/agent` test, the conformance program `specs/ir-conformance/constraints`,
the schema-agreement fixture, `docs/nx-ir-format.md`, `specs/future.md` and the website pages. The
generated parser, grammar JSON and `runtime/typescript/dist` were not read (the committed `dist` was
confirmed to match a fresh `tsc` build).  

**How findings were confirmed:** scratch NX programs compiled with `nxlang codegen --target nx-ir`
and read with `nxlang ir explain`, then run in both IR runtimes (the TypeScript runtime from its
`dist`, the Rust runtime through a scratch binary over `nx-ir-runtime`, functions and component
init/state/patch/dispatch), the interpreter through `nxlang run`, and the source tree through a
scratch binary over `nx-language-service`. `cargo test -p nx-types --test value_constraints`,
`cargo test -p nx-codegen --lib constrain`, and the TypeScript runtime's `runtime.test.js` and
`corpus.test.mjs` pass. A wider `cargo test` run could not finish because the container's disk filled.

Note: while this review ran, uncommitted edits appeared in the working tree
(`crates/nx-interpreter/src/interpreter.rs`, `src/vscode/test/grammar/keyword-inventory.test.ts`),
from another process. This review is of commit `6d6dd03`. The interpreter edit moves the numeric
pattern match into `numeric_pattern_matches` but still uses `interval_as_written`, so RF2 still
applies to it.

## Findings

### ✅ Verified - RF1 A record or union-case field default computed from another field gets no check node, so its failure is reported as `nx-ir-boundary-constraint`, not `nx-ir-constraint`
- **Severity:** Medium
- **Evidence:** For `type Paint = { a: int  b: int where >= 0 = {a - 1} }`, `nxlang ir explain`
  prints `b: int where >= 0 = (a sub 1)` with no `check` node, and the same holds for a union case
  field (`case c … b: float64 where in 0.0..=1.0 = (a mul 2.0)`). Component prop and state defaults,
  and parameter defaults, do get one (`(check b: (a mul 2.0) …)`, `(check s: …)`). Since the default
  is unchecked, the value is caught only when the runtime normalizes the record against its field
  types: `mk(0)` (in-program `<Paint a={a} />`) and `take({"$type":"Paint","a":0})` both fail with
  `nx-ir-boundary-constraint` ("Expected Paint.b to be in >= 0, got -1."), with no argument, in the
  Rust and TypeScript runtimes alike (`crates/nx-ir-runtime/src/normalize.rs:224-243`,
  `runtime/typescript/src/index.ts:5253-5258`). The specs say otherwise: `rust-ir-runtime` and
  `typescript-ir-runtime` both say "A default the runtime computes that misses its own field's
  constraint SHALL fail with `nx-ir-constraint` and SHALL name no argument", and `value-constraints`
  lists defaults among the check sites. A boundary code for a failure inside the program also
  confuses anything that reads the `nx-ir-boundary-` family as "the host's input was wrong". The
  likely cause: `validate_local_record_defaults` and `validate_local_union_defaults` run inside
  `InferenceContext::new` (`crates/nx-types/src/infer.rs:551-552`), where a sibling field such as `a`
  has no type yet. `check_typed_binding_for` then returns early at
  `self.type_satisfies_expected(actual, expected)` (`infer.rs:6972`), because an error or unknown type
  satisfies everything, so the check is never recorded (`infer.rs:6977-6993`). Task 3.3 lists
  "defaults", but the corpus (`specs/ir-conformance/constraints/main.nx`) has only the constant
  default `layers … = 1`, so nothing exercises this path.
- **Recommendation:** Record a check for record and union-case field defaults that are not
  constants, whatever type the default had at validation time. Either check them in a pass where
  earlier fields are in scope, or record the check before the early `type_satisfies_expected`
  return whenever the expected type is constrained and the value is not a proven constant. Add a
  corpus case with a computed record default that fails (`nx-ir-constraint`, no argument), reached
  both from a host-passed record and from an in-program construction.
- **Fix:** The checker records a check for a default whose type it cannot know when it checks the default, which is a record or union-case default read from a sibling field (`check_typed_binding_for` in `crates/nx-types/src/infer.rs`). `nxlang ir explain` now shows `(check b: (a sub 1) …)` for both. The corpus gained `Stroke` (`inset: int where >= 0 = {width - 1}`) with three cases: a passing in-program construction, a failing one, and a host-passed record. Both fail with `nx-ir-constraint` and no argument, in both runtimes.
- **Verification:** Confirmed. `nxlang ir explain` now shows `(check b: (a sub 1) as int where >= 0)` for the record and `(check b: (a mul 2.0) …)` for the union case, and a `{a ?? -5}` and an `if` default read from a sibling get one too. The corpus `Stroke` cases record `nx-ir-constraint` with no argument for both the in-program construction and the host-passed record, and `corpus_results_match_the_interpreter`, the Rust operation/origin checks and the TypeScript `corpus.test.mjs` pass. No other corpus program's image changed, so the new path adds no checks elsewhere. The check is conservative: see RF9.

### ✅ Verified - RF2 The interpreter matches numeric patterns against unrounded bounds, so a `float32` scrutinee can take a different arm than in the IR runtimes
- **Severity:** Medium
- **Evidence:** `let tenth: float32 = 0.1` with `if tenth is { <= 0.1 => "in" else => "out" }` gives
  `"out"` under `nxlang run` (the interpreter) and `"in"` in both IR runtimes. The interpreter tests
  the scrutinee against `NumericPattern::interval_as_written()` (`crates/nx-hir/src/ast/types.rs:256`,
  used at `crates/nx-interpreter/src/interpreter.rs:2928` / `8215-8225`), which keeps `0.1` as a
  `float64`. The checker and emitter round a `float32` bound to `float32`
  (`crates/nx-types/src/infer/constraints.rs:244`), and the scrutinee's `float32` value
  `0.10000000149…` is above `0.1` as a `float64`. The `value-constraints` requirement on the
  interpreter says it "SHALL evaluate numeric match patterns with the results the IR runtimes give".
- **Recommendation:** Have the interpreter use the interval the checker computed (the pattern's
  recorded type, as the IR builder does), or `nx_types::pattern_interval_at` with the scrutinee's
  primitive, instead of `interval_as_written`. Add an interpreter test with a `float32` scrutinee and
  a bound that `float32` cannot hold exactly.
- **Fix:** `numeric_pattern_matches` reads the bounds with `nx_types::pattern_interval_at` at the value's own primitive, so a `float32` value is compared with `float32`-rounded bounds, and falls back to `interval_as_written` only when that gives nothing. New interpreter test `the_interpreter_reads_a_float32_scrutinees_bounds_at_float32`.
- **Verification:** Confirmed. The evidence program (`let tenth: float32 = 0.1`, `if tenth is { <= 0.1 => "in" … }`) now gives `"in"` under `nxlang run`, as the IR runtimes do, and `the_interpreter_reads_a_float32_scrutinees_bounds_at_float32` passes, covering an inclusive and an exclusive end.

### ✅ Verified - RF3 The source tree drops `bounds` when the constrained base is an alias of an unconstrained primitive, so the viewer and hover lose the constraint
- **Severity:** Medium
- **Evidence:** With `type Weight = int`, `type Heavy = Weight where >= 10` and
  `type Paint = { w: Weight where in 1..=1000  h: Heavy … }`, the source tree gives `bounds: null` for
  `Heavy`, `w` and `h`, while `Ratio`-style aliases of a primitive work. In
  `crates/nx-language-service/src/source_tree.rs:2947-2985`, `type_interval` for
  `TypeRef::Constrained` resolves a non-primitive base name by calling itself on that name. For
  `Weight`, that call reaches the alias target `int` as a `TypeRef::Name`, looks `int` up in the
  module's type namespace, finds nothing, and returns `None`. The `?` then discards the whole
  constraint. `value-constraints` allows the base to be "an alias of one, constrained or not", and
  `source-tree` requires `bounds` "when its type, directly or through aliases, is constrained". The
  viewer therefore leaves these numbers unbounded. `alias_constraint_note`
  (`crates/nx-language-service/src/lib.rs:1839`) uses the same helper, so hover through such an alias
  loses its note too. The checker, the IR and the schema all handle this case
  (`build_type_ref_resolving_aliases`, `Program::denote`).
- **Recommendation:** In `type_interval`, resolve a base name to a primitive at the end of the alias
  chain: return `(primitive, Interval::ALL)` when an alias target is a primitive name, rather than
  `None`. Add source-tree tests for `Weight where …` and for an alias of it.
- **Fix:** `type_interval` now goes through `numeric_interval`, which resolves a primitive name at the end of an alias chain to `(primitive, Interval::ALL)`; `type_interval` keeps only a result that is constrained. Hover's alias note uses the same helper. New source-tree tests for `Weight where …`, an alias of it, and union-case bounds.
- **Verification:** Confirmed. `numeric_interval` resolves `Weight` to `(int, ALL)`, and `type_interval` keeps only a constrained result, so a plain alias still has no `bounds`. `a_constraint_on_an_alias_of_a_plain_number_carries_its_bounds` covers `Weight where …`, `Heavy` (an alias of it) and a plain `Weight` property, and passes with the rest of the language-service bound, constraint and hover tests (93 passed). Hover's `alias_constraint_note` goes through the same helper.

### ✅ Verified - RF4 A comma in a `where` clause is a bare syntax error, not `invalid-constraint`
- **Severity:** Low
- **Evidence:** `type Edge = int where < 0, > 9` reports only "Syntax error" at the comma, and in a
  record field (`{ a: int where < 0, > 9 }`) the error is "Invalid record definition". The
  `where_clause` rule (`crates/nx-syntax/grammar.js:278`) takes one `numeric_pattern` or a
  `value_expression`, and neither accepts a comma. `value-constraints` says: "In a `where` clause,
  anything else, including `||`, a comma, `!=` …, SHALL be rejected with `invalid-constraint` … it
  SHALL NOT be a bare parse error." `||`, `!=`, calls and a leading operand are handled and tested;
  the comma is not tested.
- **Recommendation:** Where the comma is not a list separator (alias targets, record and union-case
  fields, `let` annotations), parse `, <pattern>` after the clause as an unsupported term and report
  `invalid-constraint` ("a match arm lists alternatives with commas; a constraint has one interval").
  In a parameter list the comma has to stay a separator; say so in the spec, or report the
  unparsable result there with the same hint. Add a test.
- **Fix:** `walk_and_collect_errors` (`crates/nx-syntax/src/validation.rs`) recognizes a parse failure at a comma that follows a `where` clause's pattern on the same line, and reports `invalid-constraint` ("A constraint has one interval and cannot list alternatives", with an `&&` hint) at the comma instead of a syntax error, once per comma. Tested for an alias target, a record field, a `let` annotation and a parameter list.
- **Verification:** Confirmed for the evidence: `type Edge = int where < 0, > 9`, the record field, the `let` annotation and the parameter list each give `invalid-constraint` at the comma with the `&&` hint and no `syntax-error`; a valid parameter list after a `where` clause (`let f(x: int where >= 0, y: int)`) is untouched, and an unrelated error later on the line is still a syntax error. The heuristic also claims commas that are not alternatives: see RF10.

### ✅ Verified - RF5 Bounds that are empty, or out of range, only at the base type's limits are accepted, and the IR and schema then carry them
- **Severity:** Low
- **Evidence:** All of these compile with no diagnostic:
  - `int32 where < -2147483648` emits `int32 where <= -2147483649`, and `int32 where > 2147483647`
    emits `>= 2147483648`. Both admit no `int32`, and the bounds lie outside `int32` in the IR and in
    the schema `minimum`/`maximum`.
  - `int where > 9007199254740991` emits `>= 9007199254740992`, outside the ±(2^53−1) that the
    `nx-ir-format` spec allows for an integer bound.
  - `float32 where > 0.99999994 && < 1.0` admits no `float32`.
  - `float32 where <= 16777217` is accepted, although `let x: float32 = 16777217` is rejected as "not
    exactly representable as float32", and the spec says a bound is "checked as a literal written at
    the base type".

  The cause: `bound_value` (`crates/nx-types/src/infer/constraints.rs:244`) checks each bound before
  normalization, and `Interval::is_empty` (`crates/nx-hir/src/ast/types.rs:537`) does not intersect
  the interval with the base type's own range.
- **Recommendation:** After the integer normalization, intersect the interval with the base's range
  (`int32` limits, ±(2^53−1) for `int`/`int64`, finite `float32`). Report `empty-constraint` when
  nothing is left, and reject an integer bound on a `float32` base that `float32` cannot hold
  exactly, as the literal rule does. For `float32`, test emptiness on the rounded neighbours.
- **Fix:** New `admits_none` intersects the interval with the base's own range (`int32` limits, ±(2^53−1), finite `float32`) and reads a `float32`'s exclusive ends as the adjacent `float32`, for the emptiness test only, so `int32 where >= 0` gains no upper bound. All three emptiness checks use it. An integer bound a `float32` cannot hold exactly is now `invalid-constraint`, naming the nearest one it can. Tests cover each case in the evidence.
- **Verification:** Confirmed. Each evidence case now reports: `int32 where < -2147483648` and `> 2147483647`, `int where > 9007199254740991` and `float32 where > 0.99999994 && < 1.0` are `empty-constraint`, as are `float32 where > 3.4028235e38` and the matching match-arm patterns; `float32 where <= 16777217` is `invalid-constraint` naming `16777216`. `int32 where >= 0` still admits `2147483647`, and `> 0.99999994 && <= 1.0` admits `1.0`, so the base's range narrows only the emptiness test. `f32::next_up` is within the crate's `rust-version` (1.98.1).

### ✅ Verified - RF6 Diagnostics, hover and runtime messages show `float32` bounds rounded and the resolved type, not the interval as written
- **Severity:** Low
- **Evidence:** For `type Tenth = float32 where in 0.0..=0.1`, `let t: Tenth = 0.2` reports
  "`0.2` is outside the constraint for `t`: `float32 where in 0.0..=0.10000000149011612` admits values
  in 0.0..=0.10000000149011612". Both runtimes fail with "… outside its constraint
  0.0..=0.10000000149011612.". Hover's alias note (`lib.rs:1839`, through `Type::constrained`'s
  `Display`, `crates/nx-types/src/ty.rs:1090`) and `nxlang ir explain` spell the bound the same way.
  The spec asks for "the interval as written" in `constraint-violation` and for hover to "spell the
  type with its constraint as written". The violation message also repeats the interval and names
  the resolved type rather than the alias (`Tenth`). The `name?:` fix-it for `r: Ratio?` likewise
  suggests `r?:float64 where in 0.0..=1.0` instead of `r?: Ratio`.
- **Recommendation:** Spell `float32` bounds with the shortest decimal that rounds to the same
  `float32` (`crate::text::float32_text` exists in the runtime; the TypeScript runtime can
  shortest-print through `Math.fround`), or carry the written spelling beside the interval. Name the
  alias when the site's type was written as one, and drop the duplicated interval from the message.
- **Fix:** Partly fixed. A `float32` bound is spelled with the shortest digits that round to it in the checker (`Type` display, so hover too, and `constraint-violation`), in the Rust and TypeScript runtimes' check and boundary messages (bounds and the value), and in the source tree's pattern text. The violation message no longer repeats the interval: "`0.2` is outside the constraint for `t`: `float32 where in 0.0..=0.1`". The `name?:` fix-it now names a constrained item as written (`r?:Ratio`). Not changed: the violation message still names the resolved type rather than the alias, since the site's written alias is not available where constants are checked, and `nxlang ir explain` still prints the stored constant, which is the IR's exact value.
- **Verification:** Confirmed, and what remains is acceptable. `let t: Tenth = 0.2` now reports "`0.2` is outside the constraint for `t`: `float32 where in 0.0..=0.1`"; both runtimes say "Expected x to be in <= 0.1, got 0.2." and "The value 0.2 for 'x' is outside its constraint <= 0.1." (the Rust test asserts both, the TypeScript test the boundary message); the duplicated interval is gone; `r: Ratio?` suggests `r?:Ratio` and `r: Ratio*` suggests `r?:Ratio+`. The message still spells the site's type rather than its alias, and `nxlang ir explain` still prints the exact stored constant. Neither misleads: the message names the value, the interval and the site, and `ir explain` shows what the IR holds. The spec's "as written" no longer quite matches the message for other reasons: see RF11.

### ✅ Verified - RF7 The viewer checks `float32` bounds as `float64`, so it can commit a value the checker rejects
- **Severity:** Low
- **Evidence:** The source tree carries `float32` bounds already rounded, for example
  `f: float32 where < 0.1` → `{"max":0.10000000149011612,"maxExclusive":true}`. `rangeProblem`
  (`packages/viewer/src/editors.ts:146-157`) compares the typed number unrounded. Typing `0.1`
  therefore passes (`0.1 < 0.10000000149…`) and is committed, but the checker rounds `0.1` to the
  bound and reports `constraint-violation`. Going the other way, the note for that field reads
  `Below 0.10000000149011612`, and `float32 where > 0.1` refuses `0.1` with
  `Above 0.10000000149011612`. The `viewer` spec requires that a typed number outside the bounds
  not be committed, and the note to read `Below N` with the bound.
- **Recommendation:** Pass the declared base to `rangeProblem`/`clamp` and compare
  `Math.fround(value)` for a `float32`, and show bounds in their shortest `float32` spelling (or have
  the source tree carry the written bound). Add a viewer test with an exclusive `float32` end that
  `float32` cannot hold exactly.
- **Fix:** `boundedHints` reads a `float32`'s bounds as their shortest `float32` number and marks the hints `float32`; `rangeProblem` then compares the value and the bounds after `Math.fround`. Typing `0.1` for `float32 where < 0.1` is refused with `Below 0.1`. New viewer test.
- **Verification:** Confirmed. `boundedHints` gives `{ max: 0.1, maxExclusive: true, float32: true }` for `fine: float32 where < 0.1`, `rangeProblem` (and so `stepNumber`) compares after `Math.fround`, and the new viewer test refuses `0.1` with `Below 0.1` and commits `0.09`. `npm test` in `packages/viewer` passes (231 tests).

### ✅ Verified - RF8 Several risky paths and listed scenarios have no tests
- **Severity:** Low
- **Evidence:**
  - Host-boundary constraint failures for component props, component state, state patches and
    dispatched action records (tasks 4.1 and 4.2; the `rust-ir-runtime` and `typescript-ir-runtime`
    requirements) are not tested in either runtime. Only function arguments and a nested record
    field are (`crates/nx-codegen/src/ir_runtime_tests.rs`, `runtime/typescript/test/runtime.test.ts`).
    Run by hand, both runtimes agree on all four (`Counter props.step`, `Counter state.count`, a
    patch to `count`, `Bump action.by`) and on an in-handler `State.Update` that overflows.
  - Resolving a constrained alias from an interface-only module (task 3.2; `nx-ir-format`: "including
    one the build reads only as an interface") has no test. `ir_tests.rs` covers only a source
    workspace. The comment at `crates/nx-codegen/src/builder.rs:3153` ("stays the nominal reference
    it was before") no longer describes `type_alias_target`, which now falls back to
    `module.lowered_module`.
  - Signature help on a constrained parameter (the `value-constraints` hover requirement) has no
    test. The hover test does not cover it.
  - Source-tree `bounds` on union-case properties (the `source-tree` requirement) have no test.
  - The conformance corpus has no computed-default case (see RF1).
- **Recommendation:** Add runtime tests in both runtimes for the four component-boundary paths, and
  add each to the corpus as a lifecycle so the runtimes are compared. Add an IR test that builds
  from a library loaded as an interface (as
  `directory_loaded_cross_library_type_references_survive_per_module_emission` does) and asserts a
  kind-7 entry and a `check`. Fix the stale comment, and add the signature-help and union-case
  bounds tests.
- **Fix:** Added: component props, state and dispatched-action boundary failures and an in-handler update overflow, in both runtimes (`a_component_boundary_outside_its_constraint_is_refused` in Rust; the `Meter` component in the constraints corpus, driven by a TypeScript runtime test); an IR test building from a library loaded from a directory; source-tree bounds on union-case properties; the computed-default corpus cases (RF1). The stale comment at `builder.rs` is rewritten. Signature help: the language service has none (it is a reserved query), so the requirement now covers hover only and `specs/future.md` records signature help for later.
- **Verification:** Confirmed. `a_component_boundary_outside_its_constraint_is_refused` (Rust) and the TypeScript test over the corpus's `Meter` assert the same codes and messages for props, state, a dispatched action and an in-handler update, so the runtimes are compared even though `Meter` is not a corpus lifecycle. `a_constrained_alias_from_a_library_read_as_an_interface_is_written_as_its_entry`, the union-case bounds test and the `Stroke` corpus cases pass, and the comment at `builder.rs:3153` now describes `type_alias_target`. Narrowing the requirement to hover, with signature help recorded in `specs/future.md` and the proposal, design and task 5.2 updated, is a fair resolution: the language service has no signature help to test. `openspec validate --strict` passes.

## New Findings Discovered During 2026-10-31 03:15 Verification

### ✅ Verified - RF9 A record or union-case default read from a sibling field is checked even when the sibling's type already satisfies the constraint
- **Severity:** Low
- **Evidence:** The RF1 fix records a check whenever the default's type is `Error` at validation
  time (`check_typed_binding_for`, `crates/nx-types/src/infer.rs:6972-6988`), which is every default
  that reads a sibling field, since the sibling is not in scope there. So
  `type T = { a: int where >= 0  c: int where >= 0 = {a} }` emits
  `c: int where >= 0 = (check c: a as int where >= 0)`, and the union case
  `c { a: float64 where >= 0.0  b: float64 where >= 0.0 = {a} }` likewise checks `b`. `a`'s type
  satisfies the site with no check, so `value-constraints` requires none ("whenever the value is
  bound to a constrained site its type does not satisfy with no check"), and `nx-ir-format` says the
  emitter "SHALL write the node exactly where `value-constraints` requires a runtime check and
  nowhere else". The extra node never fails and costs two operations per construction, so the
  behaviour is right, but the IR and the cost model disagree with the spec.
- **Recommendation:** Check record and union-case defaults where the earlier fields are in scope
  (with their declared types), so a default proven by its type gets no check, and keep the
  `Error` fallback only for what is still unknown there. Or, if that is not worth it now, say in
  `value-constraints` that a default read from a sibling field is always checked, and record the
  sharper rule in `specs/future.md`. Either way, add an IR test for the `{a}` case.
- **Fix:** Took the second option. Binding the earlier fields while checking defaults was tried and drops the `{a}` check, but it also types every default against its siblings, so a default that does not fit its field becomes a compile error: the runtime tests' `type Unfit = { n:int label:string = { n } }` and the corpus's `Stepper` default `{ step(1) }` stop compiling. That changes what programs compile, beyond this change. `value-constraints` now says a record or union-case default that reads a sibling field is checked when it runs, whatever the sibling's type, with a `Stroke` scenario; `specs/future.md` records typing a default against its siblings as its own change. New IR test `a_default_read_from_a_sibling_field_is_always_checked` pins the `{a}` case.
- **Verification:** The resolution is fair: typing defaults against their siblings would turn defaults
  such as `Unfit`'s into compile errors, which is a language change of its own, and components and
  parameters already type defaults with earlier props and parameters in scope (`b: int where >= 0
  = {a}` emits no check there), so only records and union cases need the exception. The test passes
  and `{a}` and `{a - 1}` are checked as described. But the new sentence overstates what the emitter
  does, so `value-constraints` and `nx-ir-format` ("exactly where … and nowhere else") still
  disagree, now in the other direction: with `let abs0(x: int): int where >= 0 = …`,
  `type Box = { a: int  viaFn: int where >= 0 = {abs0(a)}  cond: int where >= 0 = { if a > 0 {
  abs0(1) } else { abs0(2) } } }` and the union case `dot { a: int  c: int where >= 0 = {abs0(a)} }`
  each read a sibling field, yet `nxlang ir explain` shows no check on `viaFn`, `cond` or `c` (which
  is the right behaviour: their types satisfy the field). The spec says they "SHALL be checked when
  it runs, whatever the sibling's type". `specs/future.md` repeats the overstatement ("a
  constrained field's default is always checked"). The wording follows this review's own
  recommendation, which was too loose. Still to do: say that a record or union-case default is
  typed without its sibling fields in scope, so one whose type depends on a sibling's (`{a}`,
  `{a - 1}`) is checked when it runs whatever that sibling's type, while one whose type satisfies
  the field without them (a call whose result type satisfies it) is not; match `specs/future.md`;
  and, for consistency, add the exception to `design.md` decision 5 ("A value of a type that is
  already inside the interval needs nothing") and to the website's "Where values are checked"
  (`reference/syntax/types.md`), which state the general rule with no exception. An IR assertion
  that `{abs0(a)}` gets no check would pin the boundary.
- **Fix (2):** The sentence now says what the emitter does: a record or union-case default is typed without its sibling fields in scope, so a default whose type depends on a sibling field's type is checked when it runs, and one whose type does not (a call with a constrained return type) is not. The `Stroke` scenario gains an `{abs0(width)}` case with no check. `specs/future.md`, `design.md` decision 5 and the website's "Where values are checked" say the same. The IR test, renamed `a_default_whose_type_depends_on_a_sibling_field_is_always_checked`, also asserts `viaFn = {abs0(a)}` gets no check.
- **Verification (2):** The positive half now matches the emitter, and `design.md` decision 5 is
  accurate. But the negative half is too broad, and the website keeps the old overstatement:
  - The spec's last clause says a default whose type does not depend on a sibling field's type
    "SHALL NOT" be checked. The parenthetical is only an example, so the clause also covers defaults
    whose type does not satisfy the field, and those are (rightly) checked by the general rule. In
    `type Box = { a: int where >= 0 … }`, `nxlang ir explain` shows a check on `{inc(a)}` with
    `let inc(x: int): int`, on `{abs0(a) - 1}`, on `{ if a > 0 { 1 } else { 2 } }` (an `if` of
    literals is `int`, checked in a function result too) and on `{five()}`, which reads no sibling at
    all; the union case `dot { a: int  e: int where >= 0 = {inc(a)} }` likewise. Each one's type
    does not depend on a sibling, so the requirement forbids the check it needs. The rule the
    emitter follows (`check_typed_binding_for`, `crates/nx-types/src/infer.rs:6971-6987`) is: a
    default whose type is unknown because it depends on a sibling is always checked, and any other
    default is checked exactly when its type does not satisfy the field, as anywhere else.
  - The website (`reference/syntax/types.md`, "Where values are checked") says "a default computed
    from another field … is checked when it runs". `{abs0(width)}` is computed from another field
    and gets no check: this is the wording the previous verification reopened the spec for. It also
    names only record fields, though union-case defaults behave the same.
  - `specs/future.md` says such a default "is proven by that type as anywhere else". `{inc(a)}` is
    not proven; "is treated as anywhere else" (proven when its type satisfies the field, checked
    otherwise) would be exact.

  Still to do: end the spec sentence with "and any other default is checked only when its type does
  not satisfy the field, as anywhere else (a call whose return type satisfies it gets no check)",
  say "a default whose type depends on another field" (and "record or union-case") on the website,
  and adjust `specs/future.md` as above. The IR test and the `Stroke` scenario's `{abs0(width)}`
  case are right as they are. `{a}` and `{a - 1}`, `{a + a}`, `{o ?? 3}` with `o?: int`, an `if`
  returning a sibling, and the union-case `{a}` and `{r * 2.0}` are all checked, and `{abs0(a)}` in
  both a record and a union case is not, as the positive half says. See also RF14, where a
  default that depends on a sibling gets no check.
- **Fix (3):** The spec sentence now ends as proposed: a default whose type depends on a field's type is checked whatever that field's declared type, a top-level value sharing the field's name does not type it (RF14), and any other default is checked only when its type does not satisfy the field, as any other value is. The website says "whose type depends on another field" and covers union cases; `specs/future.md` says "treated as anywhere else"; `design.md` matches.
- **Verification (3):** Confirmed. The spec sentence now matches the emitter in every case tried
  with `nxlang ir explain`: defaults whose type depends on a field are checked (`{a}`, `{a - 1}`,
  `{a + a}`, `{o ?? 3}` with `o?: int`, an `if` returning a sibling, the union-case `{r}`, and `{w}`
  read from a field inherited through two abstract bases); any other default is checked only when
  its type does not satisfy the field (`{inc(a)}`, `{five()}`, an `if` of literals and the union-case
  `{inc(1)}` are checked; `{abs0(a)}`, an `if` whose branches are `abs0(…)` calls, the union-case
  `{abs0(1)}`, `{limit}` with `let limit: int where >= 0 = 3`, `{helper(1)}` with a constrained
  return, and the constant `3` are not; `{0 - 1}` is `constraint-violation` at compile time). A
  sibling passed to a constrained parameter (`{g(a)}`) is checked at the call. `design.md` decision 5,
  the website's "Where values are checked" (now "record or union-case" and "whose type depends on
  another field") and `specs/future.md` agree with it. One nit: the rewritten `specs/future.md`
  sentence leaves a 168-character line (line 1036) where the paragraph is otherwise wrapped at 100.
  The spec's "each of its record's or case's fields" is what RF14 is reopened against.

### ✅ Verified - RF10 The comma heuristic reports a field separator or a stray parameter comma as a constraint listing alternatives
- **Severity:** Low
- **Evidence:** `constraint_comma` (`crates/nx-syntax/src/validation.rs`) claims any comma that
  follows a `where` clause's pattern on the line of an error node, whatever comes after it:
  - `type Box = { a: int where >= 0, b: int }` and `type Box = { a: int where in 0..=9, b: string }`
    report "A constraint has one interval and cannot list alternatives" with the hint "Join bounds
    with `&&`". The author used a comma as a field separator, which NX does not have; without the
    `where` clause, `{ a: int, b: int }` reports "Invalid record definition" with the expected
    shape, which is the useful message here.
  - A trailing comma, `{ a: int where in 0..=9, }` or `let f(x: int where >= 0,) = …`, gets the same
    message, where `let f(x: int,)` gets "Invalid function definition".
  - `let f(x: int where >= 0, , y: int)` attributes the error to the first comma, which is a valid
    parameter separator, as listing alternatives.

  None of these is a constraint with alternatives, and the `&&` hint points the author the wrong
  way.
- **Recommendation:** Claim the comma only when the text after it starts a pattern term (`<`, `>`,
  `in`, a digit, `-` or `.`), not a field or parameter name followed by `:`, a closing delimiter or
  another comma; otherwise fall through to `push_syntax_error`. Add the record-separator and
  trailing-comma cases as negative tests.
- **Fix:** `constraint_comma` claims a comma only when `starts_pattern_term` holds for the text after it: `<`, `>`, `in` and a space, a number, or `-`/`.` and a digit. Anything else falls through to `push_syntax_error`. New test `a_separator_comma_after_a_constraint_is_a_plain_syntax_error` covers the two record separators, the trailing commas in a record and a parameter list, and the doubled parameter comma, and checks that alternatives starting with `in` or a negative number are still named.
- **Verification:** Confirmed. `starts_pattern_term` gates the claim as described, and the loop
  moves on to a later `where` on the line. Re-tried through `nxlang codegen`: the record separators
  (`, b: int`, `, inset: int`, `, in_range: int`), the union-case separator, the trailing and
  doubled commas all give the general syntax error ("Invalid record definition", "Invalid
  discriminated union definition", "Invalid function definition"); `< 0, 5`, `< 0,> 9`, `< 0,\t> 9`,
  `< 0.5, .5`, `< 0 && > -5, in 7..=9`, and `y: int where < 0, > 9` after a valid
  `x: int where >= 0,` parameter are still named `invalid-constraint` at the right comma. A
  field or parameter name that starts with `in` is not mistaken for `in`. `cargo test -p nx-syntax`
  and `-p nx-types --test value_constraints` pass. One regression for alternatives that continue
  on the next line: see RF13.

### ✅ Verified - RF11 `constraint-violation` still promises "the interval as written", but the message spells the site's resolved interval
- **Severity:** Low
- **Evidence:** `value-constraints` (spec line 150) requires a `constraint-violation` message
  "naming the value, the interval as written and the site". After the RF6 fix the message spells
  the resolved type's interval, which differs from what was written in ways the design intends:
  `type Small = int where < 10` with `let s: Small = 10` reports "`10` is outside the constraint for
  `s`: `int where <= 9`" (integer exclusive ends are inclusive, `design.md` "Typing"), and
  `type Low = Ratio where <= 0.5` with `let l: Low = 0.75` reports `float32 where in 0.0..=0.5` (the
  alias's terms intersected), naming neither `Low` nor `<= 0.5`. The behaviour is reasonable; the
  requirement's wording is what overstates it.
- **Recommendation:** Reword the requirement to what the checker can give: the value, the site and
  the site's interval, spelled as its type is (integer ends inclusive, `float32` bounds in their
  shortest digits). Naming the alias, as RF6 originally suggested, can go to `specs/future.md`.
- **Fix:** The requirement now says the message names the value, the site, and the site's constrained type as resolved (an integer's exclusive end as the inclusive end next to it, an alias's terms intersected, a `float32` bound in its shortest digits), with a scenario for `type Small = int where < 10` naming `int where <= 9`, tested by `a_violation_spells_the_sites_resolved_interval`. Naming the alias is in `specs/future.md`.
- **Verification:** Confirmed. The requirement and its new scenario match the checker:
  `let s: Small = 10` reports "`10` is outside the constraint for `s`: `int where <= 9`",
  `let l: Low = 0.75` names `float32 where in 0.0..=0.5`, a `float32` `0.1` bound prints as `0.1`,
  `int where > 0 && < 5` prints as `in 1..=4`, and the `Paint` scenario's message still contains
  `0.0..=1.0` and `opacity`. `a_violation_spells_the_sites_resolved_interval` passes, and the alias
  naming is recorded in `specs/future.md`. No other artifact promises "as written" for this message
  (the remaining "as written" in hover, source-tree and schema export are other requirements).

### ✅ Verified - RF12 The TypeScript runtime's `intervalText` lost its doc comment to the new helper
- **Severity:** Low
- **Evidence:** The RF6 fix inserted `intervalNumberText` between `intervalText` and its doc comment
  (`runtime/typescript/src/index.ts:843-849`): two JSDoc blocks now stand above
  `intervalNumberText`, and `intervalText` has none.
- **Recommendation:** Move "The interval as a constraint spells it: …" back above `intervalText`,
  and rebuild `dist`.
- **Fix:** The doc comment is back above `intervalText`, `intervalNumberText` has its own above it, and `dist` is rebuilt.
- **Verification:** Confirmed. Each of `intervalNumberText` and `intervalText` has its own doc
  comment in `src/index.ts` and `dist/src/index.js`. `npm test` (which rebuilds with `tsc`) passes
  and leaves the working tree clean, so the committed `dist` matches the source.

## New Findings Discovered During 2026-10-31 03:32 Verification

### ✅ Verified - RF13 A constraint whose alternative is on the next line is no longer named
- **Severity:** Low
- **Evidence:** The RF10 fix finds the text after the comma by skipping only spaces and tabs
  (`constraint_comma`, `crates/nx-syntax/src/validation.rs:1843-1845`), then requires
  `starts_pattern_term` there. When the next term is on the following line, that text starts with
  `\n` (or `\r\n`), so the comma is not claimed:
  ```nx
  type Edge = int where < 0,
    > 9
  ```
  now reports a bare "Syntax error … Check the syntax and try again" at the comma. Before the fix the
  same comma was claimed (the error node starts at the comma, so `start <= next_term` held) and
  reported as `invalid-constraint` with the `&&` hint. A one-line `int where < 0, > 9` is still
  named.
- **Recommendation:** Skip all whitespace, newlines included, when finding the text after the comma
  (`c.is_whitespace()`). A separator followed by a field on the next line still starts with a name,
  so it still falls through to the syntax error. Add the two-line case to
  `a_comma_in_a_constraint_is_named_not_a_parse_error` and a record with one field per line
  (`a: int where >= 0,` then `b: int`) to `a_separator_comma_after_a_constraint_is_a_plain_syntax_error`.
- **Fix:** `constraint_comma` skips all whitespace after the comma, newlines included. The two-line `type Edge = int where < 0,` / `> 9` case is in `a_comma_in_a_constraint_is_named_not_a_parse_error`, and a record with one field per line (`a: int where >= 0,` then `b: int`) is in `a_separator_comma_after_a_constraint_is_a_plain_syntax_error`; both pass.
- **Verification:** Confirmed. `constraint_comma` now skips `c.is_whitespace()` after the comma.
  Re-tried through `nxlang codegen`: an alternative on the next line (`> 9`, `in 7..=9`, `-5`
  after a blank line, `.5`, with LF or CRLF endings), in a type alias, a multi-line record field and
  a multi-line parameter list, and a second constrained parameter after a valid
  `x: int where >= 0,` line, are each named `invalid-constraint` at the comma. RF10's separators
  are unchanged with a newline after the comma: a record with one field per line (LF and CRLF, a
  blank line between, a next field named `inset` or `in_range`), a trailing comma before `}` or
  `)`, a union-case field separator and a doubled parameter comma across lines all give the general
  syntax error ("Invalid record definition", "Invalid function definition", "Invalid discriminated
  union definition"), and a valid multi-line parameter list (`x: int where >= 0,` then `y: int`)
  still compiles. One case is not named: a line comment between the comma and the next term
  (`int where < 0, // c` then `> 9`) is a bare syntax error, as it already was after RF10. That is
  rare enough to leave. `cargo test -p nx-syntax` and `-p nx-types --test value_constraints` pass.

## New Findings Discovered During 2026-10-31 03:38 Verification

### ✅ Verified - RF14 A sibling field that shares its name with a constrained top-level value proves the default that reads it, so the default gets no check
- **Severity:** Low
- **Evidence:** A record or union-case default is typed without its sibling fields in scope, so a
  sibling name resolves to whatever an outer scope binds under that name. With
  ```nx
  let a: int where >= 0 = 5
  type Box = { a: int  c: int where >= 0 = {a} }
  let main(): Box = <Box a={0 - 7} />
  ```
  the checker types `{a}` as the top-level `int where >= 0`, which satisfies `c`, so `nxlang ir
  explain` shows `c: int where >= 0 = a` with no check. At run time the default reads the sibling
  (`-7`), and the TypeScript runtime refuses the record with `nx-ir-boundary-constraint` ("Expected
  Box.c to be in >= 0, got -7."), the symptom RF1 fixed, also when the record is only built and read
  inside the program (`{ (<Box a={0 - 7} />).c }`). `value-constraints` says a default whose type
  depends on a sibling field's type SHALL be checked; this one is not. (`{a - 0}` in a union case
  under the same top-level `a` is checked, since `sub` yields plain `int`.) The same mis-resolution
  outside constraints is older: with `let a: string = "x"`, `{ a: int  c: int = {a} }` reports
  "Default value for record property 'c' expects int, found string", though the default reads the
  `int` sibling.
- **Recommendation:** When typing a record or union-case default, bind each sibling field's name to
  an unknown type, so it shadows any outer binding: the default is then checked as the spec says,
  and the bogus string error goes away, while defaults are still not typed against their siblings'
  types (RF9's decision). Add an IR test with a constrained top-level value named like a sibling.
  If that is not wanted now, record it in `specs/future.md` and say in `value-constraints` that a
  sibling name resolves to an outer binding when one exists.
- **Fix:** `validate_local_record_defaults` binds every field of the record, inherited ones included (from `effective_record_shape`), as `Type::Error` in a scope pushed for its defaults, and `validate_local_union_defaults` does the same per case. A sibling read is therefore unknown whatever top-level value shares its name: `let a: int where >= 0` no longer proves `{a}` (now checked), and `let a: string` no longer makes `{ a: int  c: int = {a} }` a type error. Tests: `a_default_reads_its_sibling_not_a_top_level_value_of_the_same_name` (IR, record and union case) and `a_default_is_not_typed_by_a_top_level_value_sharing_a_field_name` (checker). All nx-types, nx-syntax, nx-codegen, nx-interpreter, nx-language-service and nx-api tests pass.
- **Verification:** The record path is fixed, but a union case still misses the fields it inherits
  from the union's abstract base. Confirmed with `nxlang ir explain` and the TypeScript runtime:
  with `let a: int where >= 0 = 5`, the record `{ a: int  c: int where >= 0 = {a} }` and the union
  case `dot { a: int  c: … = {a} }` now emit `(check c: a …)` and `<Box a={0 - 7} />` fails with
  `nx-ir-constraint`; a record default reading a field inherited through two abstract bases
  (`type Kid extends Mid`, `Mid extends Base`, `k: int where >= 0 = {w}` under a top-level
  `let w: int where >= 0`) is checked, also when `Base` is in another module of a workspace, and
  so is a default on the abstract base itself. A case that does not have the field
  (`sq { s: int  c: … = {a} }`) still reads the top-level `a` with no check, which is right.
  Defaults that read a non-field
  top-level value or function still type normally (`{limit}` and `{helper(1)}` get no check), an
  outright mismatch with no sibling read is still `record-default-type-mismatch` /
  `union-case-default-type-mismatch` (`{5}` at `string`, `{"s"}` at a constrained `int`), and
  `{zzz}` is still refused by the emitter ("Name 'zzz' reaches no binding …"), since lookup of a
  non-field name is unchanged. But `validate_local_union_defaults` binds only `case.fields`, not the
  fields of `union_def.base`:
  ```nx
  let w: int where >= 0 = 5
  abstract type Base = { w: int  t: int }
  type Shape extends Base =
    | dot { r: int  k: int where >= 0 = {w} }
  let main(): Shape = <Shape.dot w={0 - 7} t=1 r=1 />
  ```
  emits `k: int where >= 0 = w` with no check, and `main()` fails in the TypeScript runtime with
  `nx-ir-boundary-constraint` ("Expected Shape.dot.k to be in >= 0, got -7."), the RF1 symptom.
  Likewise `let t: string = "x"` with a case `sq { s: int  u: int = {t} }` still reports "Default
  value for union case field 'Shape.sq.u' expects int, found string". The spec now says a default is
  typed with each of its "case's fields" as unknown, and inherited fields are the case's fields
  (`ir explain` lists `w` and `t` under `case dot`). Still to do: in
  `validate_local_union_defaults`, also bind the base's effective fields per case (the shape
  `check_element_bindings_against_union_case` reads for `union_def.base`,
  `crates/nx-types/src/infer.rs:5229-5236`), and add the union-with-base case to
  `a_default_reads_its_sibling_not_a_top_level_value_of_the_same_name` and the `let t: string` case
  to `a_default_is_not_typed_by_a_top_level_value_sharing_a_field_name`.
- **Fix (2):** `validate_local_union_defaults` now also binds the fields of the union's abstract base (from `effective_record_shape` of `union_def.base`) as unknown, beside each case's own fields. With `let w: int where >= 0 = 5`, `type Shape extends Base = | dot { … k: int where >= 0 = {w} }` checks `k`, and `let t: string` no longer types a case default `{t}`. Both tests gain the inherited-base case. The 168-character line in `specs/future.md` is rewrapped.
- **Verification (2):** Confirmed. `validate_local_union_defaults` binds the base's effective
  fields (all levels) beside each case's own. Re-tried with `nxlang ir explain` and the TypeScript
  runtime: the union-base example above now emits `(check k: w as int where >= 0)` and
  `<Shape.dot w={0 - 7} … />` fails with `nx-ir-constraint`, not `nx-ir-boundary-constraint`; with
  `let t: string`, a case default `{t}` reading the base's `t: int` is clean. A field inherited
  through two abstract bases (`Mid extends Top`, `{w}` from `Top` and `{m}` from `Mid`) is checked,
  also when both bases are in another module of a workspace. A case whose base lacks the name still
  reads the top-level value (`{t}` with `Base = { w: int }` is a `string` mismatch, rightly).
  Non-field names still type normally (`{limit}` and `{helper(1)}` get no check), `{"s"}` and `{5}`
  are still `union-case-default-type-mismatch`, and `{zzz}` is still refused by the emitter. The new
  test cases cover the IR and checker halves. The `specs/future.md` paragraph is now wrapped at 100.
  `cargo test -p nx-types`, `-p nx-codegen --lib` and `openspec validate --strict` pass.

## Findings From the 2026-10-31 Review of be48fd6 (hint range and step)

Scope: `git show be48fd6` only (the checker's constraint check on `min`/`max` hints, the viewer's
step from the offered range, and the spec, design, proposal, task and website wording). Checked by
probing `nxlang run` on scratch files (inline, alias, `Ratio?`, `Ratio+`, optional and inherited
fields, exclusive ends, `float32` rounding at `16777217`, hex and fractional hints) and
`boundedHints` directly. `cargo test -p nx-api --lib visualizer_declarations`, the viewer's
`npm test` and `openspec validate --strict` pass. The warning-not-error decision is stated the same
way in the spec delta, `design.md`, `proposal.md`, `tasks.md` and the website; the source tree still
leaves out only an empty resolved range, so a hint that now warns still reaches the viewer, which
intersects it with the bounds as `viewer` requires.

### ✅ Verified - RF15 A hint outside a constrained alias that also empties the merged range gets two `editor-hint-range` warnings at one span, with messages that disagree on what the viewer does
- **Severity:** Low
- **Evidence:** With `type Spacing = int where in 0..=64`, `visualizer Spacing = <ValueVisualizer
  max=64 />` and `<PropertyVisualizer property=padding min=100 />` on `padding: Spacing`, analysis
  now warns at `min=100` ("outside the constraint of 'padding', `int where in 0..=64`; a viewer
  keeps within the constraint", `crates/nx-types/src/infer.rs:1890-1912`, confirmed with `nxlang`),
  and the language service's merged-range pass also warns at the same `min=100` ("the range is
  empty and a viewer ignores both", `crates/nx-language-service/src/source_tree.rs:2813-2857`, which
  reports any merged `min > max` without regard to the constraint). The editor shows both at one
  span under one code. Both are true (the source tree drops both ends, the viewer falls back to the
  bounds), but the pair reads as two problems and their advice differs. `merged_range_warnings`'s
  doc says it does not repeat what analysis reports; that now holds only for one element's own empty
  range.
- **Recommendation:** In `merged_range_warning`, skip the hint it would report when the property's
  or alias's type has a constraint that does not admit that hint (analysis already reported it), or
  accept the pair and say so in `merged_range_warnings`'s doc and the `visualizer-declarations`
  requirement. Add a language-service test for the constrained-alias case either way.
- **Fix:** `merged_range_warning` takes the value's constraint (`type_interval` of the alias or the property's type) and leaves a `min` or `max` the constraint does not admit to analysis, so that hint gets one `editor-hint-range`, the "outside the constraint" one. The doc comment of `merged_range_warnings` says so. New language-service test `a_hint_outside_the_constraint_warns_once_as_outside` (the `Spacing = int where in 0..=64` case).
- **Verification:** Verified. Each `editor-hint-range` from the merged-range pass now skips a hint the constraint does not admit, by the same `contains` test (with the `float32` rounding) analysis uses. Checked with temporary language-service cases: an alias-typed, an optional alias-typed (`Spacing?`), a sequence (`Spacing+`) and an inherited inline-constrained property hinted `min=100` (and `max=-5`) over `0..=64` each get one warning, analysis's "outside the constraint"; a `min=64` against `< 64` and a `float32` `min=0.2` against `0.0..=0.1` likewise. A merged-empty range inside the constraint (`min=50` over the alias's `max=40`, plain and optional; inherited `min=20` over the base's `max=10`; `float32` `min=0.1` at the inclusive end) still gets the merged warning. The Value path cannot merge with another alias's hints (an alias of an alias may not have a visualizer), so it has no duplicate to remove. `cargo test -p nx-language-service`: 324 passed.

### ✅ Verified - RF16 A float's derived step now depends on whether any constraint exists, not on the range offered: `float64 where >= 0.0` hinted `min=0 max=1` steps by `0.1`, a plain `float64` with the same hints by `1`
- **Severity:** Low
- **Evidence:** `packages/viewer/src/edit.ts:121-125` now derives the step from `low`/`high` after
  the hints narrow the bounds, so a half-bounded constraint whose other end comes from a hint gets a
  derived step (`boundedHints({min:0,max:1}, {min:0}, "float64")` gives `step: 0.1`), while
  `boundedHints` returns the hints untouched when there are no bounds (`edit.ts:92-94`), so the same
  hints on an unconstrained `float64` give no step and the editor steps by `1` across the whole
  `0..1` range. Before this commit both cases stepped by `1` (the step needed both constraint
  ends). Adding `where >= 0.0`, which changes nothing about the offered range, now changes the step.
  No test covers the half-bounded case.
- **Recommendation:** Apply the derived step to any float whose offered range has both ends,
  constraint or not (derive it outside the `bounds === undefined` early return, and widen the
  `viewer` wording beyond the constraint requirement), or keep requiring both constraint ends and
  say so in the spec. Add a test for the half-bounded case whichever is chosen.
- **Fix:** `boundedHints` derives the step through one `derivedStep` helper for every float: from the narrowed range when there are bounds, and from the hints alone when there are none, so a plain `float64` hinted `min=0 max=1` steps by `0.1`, as `float64 where >= 0.0` with the same hints does. The viewer delta says the hints alone give the range when there are no bounds, with an added `AND` clause, and the visualizer reference says the same. The viewer fixture gains `plain` and `upper` properties asserting both.
- **Verification:** Verified. Called the built `boundedHints` directly: a plain `float64` hinted `min=0 max=1` and a half-bounded one give `step: 0.1`; integers, an explicit `step` hint, one-ended hints, an empty hint range with no bounds (`min=2 max=1`, no step), equal ends and a missing base give no derived step; an empty or non-overlapping hint range with bounds falls back to the bounds and steps from them; a `float32` without bounds hinted `0..0.3` steps by `0.01`, and with bounds keeps its `float32` flag and shortest ends. `resolveType` follows aliases, so an unconstrained alias of `float64` steps too. With no bounds the only change is the added `step`, as the delta's "but for the step below, change nothing for one without" says; the delta's scenario `AND` clause and the visualizer reference describe the same rule. Viewer `npm test`: 231 passed; `openspec validate --strict` passes.

### ✅ Verified - RF17 No checker test covers the new warning through an alias-typed, optional, sequence or inherited property
- **Severity:** Low
- **Evidence:** `a_hint_wider_than_the_constraint_is_a_warning`
  (`crates/nx-api/src/visualizer_declarations_tests.rs:261-288`) covers inline-constrained
  properties and an alias's own `ValueVisualizer`, but not a `PropertyVisualizer` on a property typed
  through an alias (`opacity: Ratio`), `maybe?: Ratio`, `many: Ratio+`, or a field inherited from an
  abstract base. These all warn today (checked with `nxlang`) because
  `type_from_type_ref_in_quietly` expands the alias and `Type::constraint` looks through the
  occurrence, but nothing guards that. The one alias-property case in
  `a_constrained_number_takes_number_hints` asserts only the clean in-range path.
- **Recommendation:** Extend the warning test with an alias-typed property and an optional or
  sequence one (`maybe?: Ratio max=5`, `many: Ratio+ max=5`), each expecting one
  `editor-hint-range` that names `float32 where in 0.0..=1.0`.
- **Fix:** `a_hint_wider_than_the_constraint_is_a_warning` now also covers a `PropertyVisualizer` on a property typed `Ratio`, `Ratio?` and `Ratio+`, and on a field inherited from an abstract base; each gets one warning.
- **Verification:** Verified. The four new bodies (`Ratio`, `Ratio?`, `Ratio+`, inherited `int where in 1..=5`) each assert one `editor-hint-range`; checked temporarily that each is the "outside the constraint" warning naming the property and constraint and that none of the bodies has an error. The test asserts only the count, not the message, which is adequate since the code filter and the single hint leave no other source. `cargo test -p nx-api --lib visualizer_declarations`: 44 passed.

## Questions
- **Answered:** the node's own operation is intended, as for every node, and `docs/nx-ir-format.md`
  already said so. The `nx-ir-format` delta spec and the design now say n + 1.
- Cost model: a check of one value costs two operations in both runtimes (the node visit, plus one
  for the value), and a check of n items costs n + 1. The test asserts `plain + 4` for three items.
  `nx-ir-format` says "Each value checked … SHALL count as one operation", and the design says "an
  item-wise check of n items costs n". Is the node's own operation intended? If so, the spec and
  design should say so.
- The viewer derives a float step from the constraint's span alone (`packages/viewer/src/edit.ts:116`),
  as the `viewer` spec words it. With `float64 where in 0.0..=1000.0` and a `max=1` hint, the step is
  `100`, larger than the whole range on offer. Should the step come from the hint-narrowed range
  instead? This would be a spec change.

## Summary
- The implementation is broad and mostly sound. Grammar (including the `5 -1 =>` arm fix and fields
  named `where`), checker intervals, joins, narrowing, cross-module alias resolution, the IR
  encoding, schema bounds (including the `float32` exclusive-end rounding) and both runtimes agree
  across every check site exercised: arguments, results, `??`, `if`, sequence items, optional
  arguments, parameter, prop and state defaults, update records, action records, union-case values
  and narrowed arms. The Rust and TypeScript runtimes gave identical codes and messages in every
  case tried.
- Three Medium findings need fixing before verification:
  - RF1: record and union-case defaults computed from another field are unchecked, and their failure
    surfaces with the wrong code.
  - RF2: the interpreter disagrees with the IR runtimes on `float32` numeric patterns.
  - RF3: the source tree and hover lose constraints whose base is an alias of a plain primitive.
- The Low findings are spec-edge correctness (RF4, RF5), messages that do not show the interval as
  written (RF6), a viewer rounding mismatch (RF7), and test gaps (RF8).
- Verification (2026-10-31, commit `09a74f8`): RF1–RF8 are verified. RF6's remainder (the message
  names the resolved type, not the alias; `ir explain` prints the exact constant) is acceptable.
  Four Low findings are new: RF9 (a sibling-reading default is checked even when proven by its
  type), RF10 (the comma heuristic misreads field separators and stray commas) and RF11 (the
  `constraint-violation` wording promises "as written") and RF12 (a misplaced doc comment in
  the TypeScript runtime). None blocks the change. Re-run for this
  verification: `cargo test -p nx-types --test value_constraints`, `-p nx-interpreter --test
  value_constraints`, `-p nx-codegen --lib` (constraint, corpus, interface and component tests),
  `-p nx-language-service` (bound, constraint and hover tests), `-p nx-syntax`, the TypeScript
  runtime's `runtime.test.js` and `corpus.test.mjs` (the committed `dist` matches a fresh `tsc`
  build), `packages/viewer` `npm test`, and `openspec validate --strict`.
- Verification (2026-10-31, commit `b5f0d2c`): RF10, RF11 and RF12 are verified. RF9 is reopened:
  documenting the rule rather than changing what compiles is a fair resolution, but the new sentence
  says every sibling-reading default is checked, and a default such as `{abs0(a)}` whose type
  satisfies the field is (rightly) not, so the spec still disagrees with the emitter; `design.md`
  and the website state the general rule with no exception. RF13 (Low) is new: the RF10 fix stops
  naming an alternative written on the next line. Re-run: `cargo test -p nx-types --test
  value_constraints`, `-p nx-codegen --lib` (check, constraint, default and corpus tests),
  `-p nx-syntax`, the TypeScript runtime's `npm test` (the rebuilt `dist` matches the commit), and
  `openspec validate --strict`.
- Verification (2026-10-31, commit `04c1275`): RF13 is verified. RF9 is reopened again: the spec's
  new "SHALL NOT" clause covers defaults whose type does not depend on a sibling but does not
  satisfy the field (`{inc(a)}`, `{five()}`), which are rightly checked, and the website still says
  every default computed from another field is checked. RF14 (Low) is new: a sibling named like a
  constrained top-level value proves the default that reads it. Re-run: `cargo test -p nx-types
  --test value_constraints`, `-p nx-syntax`, `-p nx-codegen --lib` (default, sibling, check and
  constraint tests), and `openspec validate --strict`.
- Verification (2026-10-31, commit `ea5b5ad`): RF9 is verified: the spec, `design.md`, the website
  and `specs/future.md` now match what the emitter checks across sibling reads, arithmetic, `??`,
  `if`, calls with constrained and unconstrained returns, constants, union cases and inherited
  record fields. RF14 is reopened: records (inherited fields included) and union cases' own fields
  now shadow a same-named top-level value, but a union case's fields inherited from the union's
  abstract base do not, so such a default is still proven by the top-level value (no check,
  `nx-ir-boundary-constraint` at run time) or falsely mistyped. No new findings. Re-run: `cargo test
  -p nx-types`, `-p nx-codegen --lib`, `-p nx-language-service -p nx-interpreter -p nx-api`, and
  `openspec validate --strict`.
- Verification (2026-10-31, commit `0c148f0`): RF14 is verified: a union case's default now reads
  the fields it inherits from the union's abstract base (local, imported or several levels up) as
  unknown, so a same-named top-level value neither proves nor mistypes it. Every finding is now
  verified or resolved. No new findings. Re-run: `cargo test -p nx-types`, `-p nx-codegen --lib`,
  and `openspec validate --strict`.
