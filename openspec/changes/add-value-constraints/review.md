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

### 🟡 Fixed - RF9 A record or union-case default read from a sibling field is checked even when the sibling's type already satisfies the constraint
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

### 🟡 Fixed - RF10 The comma heuristic reports a field separator or a stray parameter comma as a constraint listing alternatives
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

### 🟡 Fixed - RF11 `constraint-violation` still promises "the interval as written", but the message spells the site's resolved interval
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

### 🟡 Fixed - RF12 The TypeScript runtime's `intervalText` lost its doc comment to the new helper
- **Severity:** Low
- **Evidence:** The RF6 fix inserted `intervalNumberText` between `intervalText` and its doc comment
  (`runtime/typescript/src/index.ts:843-849`): two JSDoc blocks now stand above
  `intervalNumberText`, and `intervalText` has none.
- **Recommendation:** Move "The interval as a constraint spells it: …" back above `intervalText`,
  and rebuild `dist`.
- **Fix:** The doc comment is back above `intervalText`, `intervalNumberText` has its own above it, and `dist` is rebuilt.

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
