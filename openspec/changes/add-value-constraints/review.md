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

### 🟡 Fixed - RF1 A record or union-case field default computed from another field gets no check node, so its failure is reported as `nx-ir-boundary-constraint`, not `nx-ir-constraint`
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

### 🟡 Fixed - RF2 The interpreter matches numeric patterns against unrounded bounds, so a `float32` scrutinee can take a different arm than in the IR runtimes
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

### 🟡 Fixed - RF3 The source tree drops `bounds` when the constrained base is an alias of an unconstrained primitive, so the viewer and hover lose the constraint
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

### 🟡 Fixed - RF4 A comma in a `where` clause is a bare syntax error, not `invalid-constraint`
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

### 🟡 Fixed - RF5 Bounds that are empty, or out of range, only at the base type's limits are accepted, and the IR and schema then carry them
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

### 🟡 Fixed - RF6 Diagnostics, hover and runtime messages show `float32` bounds rounded and the resolved type, not the interval as written
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

### 🟡 Fixed - RF7 The viewer checks `float32` bounds as `float64`, so it can commit a value the checker rejects
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

### 🟡 Fixed - RF8 Several risky paths and listed scenarios have no tests
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
