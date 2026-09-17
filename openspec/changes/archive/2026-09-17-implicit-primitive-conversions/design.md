## Context

See proposal.md for motivation. The pieces this design has to move:

- **Compatibility lives in two places.** `Type::is_compatible_with` (`crates/nx-types/src/ty.rs`)
  is the structural relation, and it makes any two integer widths and any two float widths
  compatible in both directions. `InferenceContext::type_satisfies_expected` in `infer.rs` is the
  richer relation (unions, records) and delegates primitive pairs to the structural one. Widening
  must be changed once, in the structural relation, so every caller (`common_supertype`, the
  interpreter's `value_matches_expected_type`, the checker) inherits it.
- **`+` is disambiguated too early.** HIR lowering (`crates/nx-hir/src/lower.rs`) rewrites `Add` to
  `BinOp::Concat` when its own `TypeTag` for both operands is `String`. That tag is populated only
  for literals and annotated names, so `"Reorder " + item.title` stays `Add`, the checker accepts it
  (string × string), and both the interpreter and the IR emitter then treat it as numeric addition.
  The checker is the only place that knows every operand's type.
- **The interpreter is value-typed.** It evaluates HIR against runtime `Value`s and consults
  declared types only at binding sites (`coerce_non_record_value`). It has no per-expression static
  type, so operator behavior has to be decided from operand values, and widening has to happen where
  a declared type is in hand.
- **The IR runtime has no types at all at run time.** Every numeric primitive is a JavaScript
  `number`, which makes widening free and makes `float32` text formatting impossible without help
  from the emitter. The emitter does have the checked type of every expression
  (`CodegenExpression::ty`, already used to choose `idiv` over `div`).
- **Contextual literal typing already has the plumbing this change extends.** The checker collects
  `converted_int_literals` and `check.rs` calls `nx_hir::apply_int_literal_conversions` to rewrite
  them in the prepared module before evaluation or emission. Recording the checker's decisions and
  applying them to the prepared module is the established pattern.
- **IR schema 3 is implemented but unreleased**, so bumping it is cheap now and gets dearer once the
  fiddle consumes a published package.

## Goals / Non-Goals

**Goals:**
- One structural widening relation, consumed everywhere, with no second copy of the lattice.
- The checker decides once whether a `+` concatenates; the interpreter and the emitter read that
  decision rather than re-deriving it.
- The three backends print identical text for the same primitive value.
- Programs that type-check today and did not rely on narrowing continue to type-check and evaluate
  to the same values.

**Non-Goals:**
- Explicit conversion functions. The proposal names them as the follow-up.
- Enforcing the `int` range or checked arithmetic; that is deferred by `primitive-type-names` and
  this change does not touch it.
- A distinct `int64` runtime carrier. `int64` values still evaluate to `Value::Int` in the
  interpreter and to `number` in the IR runtime, and the static narrowing rule does not depend on
  the carrier.
- Type-checking element content under built-in tags (`specs/future.md`). Text stringification
  applies only at declared `string` content properties.

## Decisions

### 1. Widening is a one-directional relation on `Primitive`, replacing the symmetric category check

Replace the "same category ⇒ compatible" block in `is_compatible_with` with a
`Primitive::widens_to(self, target) -> bool` that encodes the lattice as data:
`int32 → {int, int64, float64}`, `int → {int64, float64}`, `float32 → {float64}`, plus
reflexivity. `numeric_promotion` becomes "the narrowest primitive both operands widen to" computed
from that same function over a fixed rank order (`int32, int, int64, float32, float64`), so the
lattice is written once. `common_supertype` in `semantics.rs` needs no change beyond what
`numeric_promotion` now returns.

*Alternative considered:* keep symmetric compatibility and add a separate narrowing diagnostic.
Rejected because compatibility is consulted from `common_supertype`, the interpreter and the checker
alike; a side check would have to be repeated at each and would drift.

### 2. The checker owns the `+` decision and records it on the module

`infer_binop` types `Add` as concatenation when either operand is `string` and the other is
`string` or a stringifiable primitive. It records two things in the inference context, alongside
`converted_int_literals`: the set of `ExprId`s whose `+` concatenates, and for each non-string
operand of those, the `ExprId` and its primitive type. `check.rs` applies both to the prepared
module with a new `nx_hir::apply_string_conversions`, which rewrites the recorded `+` to a new
HIR node and wraps each recorded operand in a new `Expr::ToText { expr, ty }`.

`BinOp::Concat` and the lowering-time `TypeTag` rewrite are deleted. Lowering emits `Add` for every
`+`, and nothing downstream has to guess. Rather than keep `Concat` as the rewritten operator, the
rewrite produces a dedicated `Expr::Concat { lhs, rhs }`, so an operator table never has a case
that only exists after analysis. `TypeTag` stays for literal tagging and `combine_numeric` until
those have a reason to move; removing `Concat` is what this change needs.

*Alternative considered:* let the interpreter dispatch `Add` on runtime values (`String + Int ⇒`
concat) and let the emitter choose `concat` from `CodegenExpression::ty`. That works for the
decision but not for `float32` text: the interpreter can format from the value, but the emitter
would still need to wrap operands, so the explicit `ToText` node is needed anyway. Recording once in
the checker keeps the interpreter and emitter symmetric and makes the HIR self-describing.

### 3. Widening at run time is a binding-site conversion in the interpreter, a no-op in the IR runtime

The interpreter converts in `coerce_non_record_value` after `value_matches_expected_type` passes:
`Int32 → Int`, `Int32 | Int → Float(f64)`, `Float32 → Float(f64)`, recursively through arrays and
nullable types. That is the one place a declared type meets a value, and it already handles
scalar-to-list. Arithmetic (`eval_add` and siblings) and comparison (`values_equal`, `eval_lt` and
siblings) gain the cross-category arms by converting the integer operand to `f64`; they stay
value-dispatched because the interpreter has no static types to read. `int64` widening to `int`
carrier is already the case.

The IR emitter records a widened operand at its own type and emits nothing, since the TS runtime's
`number` makes the widening unobservable. Generated JavaScript reads the same static types: an
integer `/` is `nxIntDiv`, which truncates and refuses a zero divisor as `idiv` does (RF20; the
emitter had always written a bare `/`). Every `/` and `%` goes through a helper for the same reason
— `nxDiv` and `nxMod` raise a run-time error on a zero divisor rather than returning `Infinity` or
`NaN`, which is what the interpreter and the IR runtime do (RF22). Canonical JSON output already
prints `3` and `3.0` identically there. The interpreter's canonical output distinguishes them, which
is why the spec requires the binding-site conversion rather than leaving the `Int` in place.

### 4. The IR gets a `text` node; `concat` becomes strings-only; schema goes to 4

New node kind `20`, `[20, node, str]`, emitted from `Expr::ToText`, naming the operand's primitive.
`binary_operator` in `ir.rs` emits `concat` for `Expr::Concat` and `add` for `Add`. The TS runtime
evaluates `text` with `String(value)` except for `float32`, where it prints the shortest
round-trip `float32` digits (`Math.fround` to confirm the value, then a shortest-digits search over
precision 1..9 via `toPrecision`, matching what `ryu`-style formatting produces). `concat` drops its
`String()` coercion and fails on non-strings. Schema version 3 → 4 in the emitter, the reader, the
TS runtime's `nodeKinds`, `docs/nx-ir-format.md`, and the conformance corpus.

*Alternative considered:* define the `float32` text form as its `float64` expansion so no node is
needed. Rejected: `0.1` printing as `0.10000000149011612` is exactly the kind of surprise this
change exists to remove. *Alternative:* a `str32` unary operator. Rejected as a one-off; the `text`
node carries any primitive name and covers the next types (enums, dates) without another kind.

### 5. One float formatter, ECMAScript rules, implemented in Rust for the interpreter

`Value`'s text form is a new `Value::to_text()` (not `Display`, which prints lists and records for
diagnostics). For `f64` it produces the ECMAScript `Number::toString` output: Rust's `{}` already
prints shortest round-trip digits, so the formatter only has to add the exponent form for magnitudes
≥ 1e21 or < 1e-6, normalise `-0` to `0`, and spell the non-finite values `NaN`/`Infinity`. For
`f32` the same rules run on the `f32` value, which Rust formats shortest-round-trip as a `float32`.
Generated JavaScript (`emit.rs`, `binop_text`) maps `Concat` to `+`, whose `String()` semantics are
the ECMAScript rules by definition; it wraps a `float32` operand in the same shortest-`float32`
helper the runtime uses, exported from the runtime package.

### 6. Literals take their site's width, and a literal operand takes the other operand's width

`convert_int_literals` generalises to integer targets: at an `int32`/`int64` site it records the
literal's target width with a range check (`i32::MIN..=i32::MAX` for `int32`; the `int64` and
`int` ranges are what an `i64` literal already satisfies). Real literals at `float32` sites record
`float32`; the conversion is a rounding, not an exactness check. `infer_binop` applies the same
conversion to a literal operand against the other operand's numeric type before promotion, which
is what keeps `w * 1.5` a `float32`. The recorded type is what `apply_int_literal_conversions` and
its float counterpart write back, so the IR and the interpreter see the width the site chose.

A constant expression (arithmetic over numeric literals only) follows the Go, Ada, Zig and Odin rule
for constants (review finding RF17). `convert_literals` folds it with `fold_constant` at the types
its literals infer on their own, as the interpreter would (`7 / 2` is `3`), and then converts the
folded literal exactly as it would a written one, so `1.5 * 2` binds at `float32` and `1000000 *
3000` is out of range for `int32`. The folded value goes in `folded_constants`, and
`nx_hir::apply_constant_folds` replaces the expression with it before the literal conversion runs.
Folding happens only where a site or operand gives the expression a type other than the one it
infers, so `1 / 0` unconstrained or at an `int` site is still a runtime error. A literal operand
that the other operand's type cannot hold stays an error rather than falling back to promotion, as
in Go, Rust and Swift: `n32 > 3000000000` is always false, so it is far more likely a mistake than
an intent.

*Alternative considered:* push the site's type into the literals, as Swift does. Rejected: added
to NX's existing rules, it would make `7 / 2` evaluate to `3.5` at a `float32` site and `3.0` at a
`float64` one. *Alternative:* promote an out-of-range literal operand, as C# and Kotlin do.
Rejected for the reason above.

### 7. Text bodies at `string` content sites are joined in the checker's binding step, not in lowering

Lowering keeps producing the content list (runs and braced expressions). `check_content_binding`
in `infer.rs`, on finding a `string`/`string?` content property and a multi-piece body, checks each
braced piece is `string` or a stringifiable primitive and records the body as a string join: the
prepared module gets an `Expr::Concat` chain (with `ToText` wrappers) in place of the list, with
the first run's leading and last run's trailing whitespace trimmed. Reusing `Concat`/`ToText` means
the interpreter, the emitter and the IR runtime need nothing content-specific; a joined body is just
a string expression bound to the property. A body of a single text run is a one-piece join too, so
it is trimmed and its line breaks collapsed like any other (see Implementation Notes).

### 8. A join records the widenings it makes, and lifts nullability

An `if`/`else`, a match and a list literal join their branches with `common_supertype`, and a join
can widen: `int` with `float64` is `float64`. The interpreter, which reads values rather than types,
would otherwise carry the `int` branch as an integer and divide it as one. The checker therefore
records each branch narrower than its join (`widened_joins`), and after analysis
`nx_hir::apply_join_widenings` wraps each in an `Expr::Widen` naming the join's numeric type, the
same way `ToText` records a text conversion. The interpreter widens the value with the binding-site
table from Decision 3, item by item in a list; the codegen builder emits the branch unchanged, since
the IR runtime's `number` makes the widening unobservable. The branch keeps its own type in the
codegen model, because that type picks its operators: typing it as the join made `n / 2` in an `int`
branch a `div` and dropped the `float32` rounding from a `float32` product (RF19). A function's
inferred return needs nothing separate: its type is its body's.

The join was also missing a rule for nullable types. The `null` literal is typed `T?` for a fresh
`T`, and a type variable satisfies nothing, so `if b { n } else { null }` joined to `object`, which
left nothing to widen and nothing a nullable site would accept. `nullable_join` in `semantics.rs`
lifts nullability out of the join (both the generic join and the checker's record- and union-aware
one use it, passing their own join for the inner types): a `null` branch contributes only
nullability, and `A?` with `B` is `join(A, B)?`, except that an `object` join stays `object`.
The alternative, narrowing the spec to declared sites, would have accepted an interpreter result
that differs from the type and from JavaScript.

### 9. `float32` arithmetic gets its own IR operators

Letting literals take `float32` makes `float32` arithmetic reachable (`w * 1.5` stays `float32`),
and a JavaScript runtime computes it as `float64` arithmetic. The text form was already right,
since `text<float32>` rounds before printing, but a widened or compared result was not:
`v * 3` with `v` holding `2.3` is `6.899999618530273` under the interpreter and
`6.8999998569488525` unrounded. `binary_operator` therefore picks `fadd32`, `fsub32`, `fmul32` or
`fdiv32` (`16` to `19`) when the expression's checked type is `float32`, just as it picks `idiv`
from an integer type. The TS runtime evaluates each as the `float64` operation wrapped in
`Math.fround`, which is exactly the `float32` operation (the rounding asm.js relies on), and
generated JavaScript wraps the same four operators in `Math.fround`. A remainder of two `float32`
values is exact and negation never rounds, so `mod` and `neg` need no variant. The invariant this
rests on, that every `float32` a runtime holds is rounded, also needs the host boundary: both JS
targets round a number supplied at a `float32` site and refuse an integer a `float32` does not
hold exactly, as the interpreter does. The TS runtime also refuses a non-integer or out-of-range
number at an `int32` site. Generated JavaScript does not check `int32` sites yet; that belongs to
the deferred range-enforcement change, as does `int32` overflow, which the interpreter wraps and
JavaScript does not.

*Alternative considered:* a trailing type name on every arithmetic `binary` node, as `text` has.
Rejected: it makes the node's layout vary, and the operator already encodes the numeric domain
(`div` or `idiv`). *Alternative:* a separate rounding node around each result. Rejected: it doubles
the nodes for a chain of `float32` operations and could be placed where no arithmetic happens.
*Alternative:* document the difference and soften the spec. Rejected: schema 4 is unreleased, so
the operators cost little now and more later.

## Risks / Trade-offs

- [Existing NX that relied on silent narrowing stops compiling] → The catalog is `float64`-only
  and the examples contain no cross-width arithmetic, so nothing shipped breaks. The diagnostic
  names both types and says the conversion is lossy. Explicit conversions are the follow-up that
  gives authors an escape hatch; until then a narrowing is written by changing the declared type.
- [`int32 + float32` typing as `float64` surprises a C# author] → It falls out of the single rule
  (`float32` is not an exact target for any integer). The spec states it, and the diagnostic-free
  path is the safe one: nothing is lost. Authors who want `float32` write the literal at the
  `float32` operand, which contextual typing now honours.
- [IR schema bump collides with `ir-action-handlers`, also unarchived on this branch] → That change
  adds kind 19 under schema 3 and does not touch the versioning requirement; this one adds kind 20
  and bumps to 4. Whichever archives second merges cleanly; the corpus fixtures regenerate with
  `NX_UPDATE_CORPUS`.
- [Three float formatters drift] → The corpus gains programs that concatenate `1.0`, `0.1`, `1e21`,
  `1e-7`, `-0.0` and a `float32` `0.1`, and the emitter-side and runtime-side corpus tests both
  check them.
- [Deleting `BinOp::Concat` touches every match on `BinOp`] → The compiler lists them; each site
  either has an `Expr::Concat` arm now or never handled strings.
- [Joins with `null` change type from `object` to `T?`] → A program that passed such a join to an
  `object` site still checks, since `T?` satisfies `object`. One that passed it to a narrower
  nullable site now checks where it did not. The playground examples and the test suites are
  unaffected.
- [Trimming a `string` body's outer whitespace changes an existing single-run binding] → Accepted.
  A body written across lines, such as `<Label>` + newline + `Total` + newline, now binds `"Total"`
  where it bound `"Total\n"`. The layout whitespace was never meant as text, and treating a single
  run like any other body is what makes re-indenting or re-wrapping a body change nothing. The
  playground examples and the test suites contain no body that relied on it.

## Migration Plan

No data migrates. Land in one PR: types and HIR first (compiler-driven), then interpreter, then
emitter + runtime + corpus, then docs. `sites/playground/src/examples/nx/reorder.nx` drops its
workaround comment and uses `"Reorder " + Item.Title` once the interpreter and runtime both pass.
Rollback is reverting the PR; no persisted artifact depends on schema 4 until a package is
published.

## Open Questions

- Whether `Display for Value` should itself use the canonical float form for diagnostics. It is
  not required by the specs and can follow later without changing them.

## Implementation Notes

Where building it refined the decisions above:

- **`ToText` names its type with a HIR enum, not `nx_types::Primitive`.** `nx-types` depends on
  `nx-hir`, so the HIR cannot name the checker's `Primitive`. `nx_hir::ast::PrimitiveType` holds the
  primitives a value can have; `Primitive::hir_type()` maps onto it. The same enum is the target
  width `apply_literal_conversions` (the renamed `apply_int_literal_conversions`) rewrites to.
- **The HIR literal carries its width (Decision 6).** `Literal::Int32` and `Literal::Float32` are
  what the rewrite produces, because the interpreter has no static types and would otherwise
  evaluate a literal the checker typed `int32` or `float32` as an `int` or `float64`, which the
  one-directional lattice then refuses at the binding site. `int64` keeps `Literal::Int`, since it
  has no carrier of its own. Lowering never produces the two new variants.
- **A literal is converted before ordinary satisfaction is tried,** not after, so that `1` at an
  `int64` site records `int64` rather than staying an `int` that merely widens.
  `numeric_literal_target` (formerly `float_literal_target`) still declines `object` and an
  undecided type variable.
- **Whitespace between body pieces comes from the source, not from text runs (Decision 7).** The
  grammar treats whitespace in a body as an extra: `{first} {last}` has no run between the braces,
  and the run in `{count} items` starts at `items`. Lowering therefore records the gap before each
  content piece in `Element::whitespace_runs`, and the join puts those back. A body of elements
  never sees them.
- **Only a text run is trimmed, and a lone braced number is a join too.** Lowering records which
  pieces came from text (`Element::text_runs`), because a braced string literal lowers to the same
  expression and keeps its spaces. A plain `TEXT_RUN` and a typed text body's `EMBED_TEXT_RUN` are
  recorded, and raw text is not, so it keeps its spaces too. Lowering used to skip an
  `EMBED_TEXT_RUN` altogether, which silently dropped a typed text body's text. A body that is a
  single text run, or a single braced number or boolean, at a `string` property is a one-piece
  join; a single braced `string` keeps the plain binding path.
- **A typed body keeps its line breaks.** A `<Note:markdown>` body is text for a processor the host
  supplies, and that processor reads the line breaks itself: a blank line is a paragraph break and a
  leading `-` is a list item, so the plain-body rule below would destroy it (RF23). Lowering records
  the tag's text type on `Element`, and the join gives a typed body its own layout rule: the
  indentation its lines share comes off, along with the line break after the open tag and a
  whitespace-only line before the close tag, and everything else is kept as written. The
  alternative, documenting the collapse as a limitation, would have left `:markdown` producing text
  no Markdown processor reads correctly.
- **Line breaks are layout, in a plain body.** The join turns each whitespace stretch that contains
  a line break, in a gap or inside a text run, into one space (`collapse_line_breaks`), and keeps
  whitespace within a line. A body therefore reads the same on one line or several, and re-indenting
  it changes nothing. JSX instead drops a line break stretch entirely, which makes `{first}` and
  `{last}` on separate lines read `AdaLovelace`; HTML and XAML collapse every whitespace run, which
  would also change spaces an author typed on one line. An exact line break is written inside a
  braced string, which is kept as is.
- **The JavaScript `float32` helpers round first.** JavaScript computes `float32` arithmetic as
  `float64`, so the helpers apply `Math.fround` before the shortest-digits search, not only to
  check a candidate.
- **The content join needs the element, so `ElementId` is threaded through the binding checks**
  down to `check_content_binding`. `StringConversions` (in `nx-hir`) carries the three recorded
  sets, and `apply_string_conversions` returns the nodes it creates so `check.rs` can type them
  `string`. Operands are wrapped by allocating the `ToText` and repointing the `Concat`, so existing
  expression ids and their recorded types stay valid.
- **The interpreter's relational operators share one `ordering` helper** instead of gaining a
  cross-category arm each; `NaN` compares false every way, as before.
- **The TypeScript validator rejects an unknown `text` type name at preparation**
  (`nx-ir-malformed`), as task 6.2 asks, so the `nx-ir-operator` failure the `typescript-ir-runtime`
  spec describes for such a node is reachable only for an operand of the wrong runtime type. The
  evaluator keeps the `nx-ir-operator` arm for the name as well. The spec sentence may want
  rewording to say preparation refuses it.
- **The numeric documentation lives in `reference/syntax/types.md`,** not in the language-tour page
  task 7.2 names. The section there was rewritten; the tour page gained a short *Numbers* section
  with the accepted `scaled` example and a link.
- **Tests that used the old behavior as their example were given another.** Two checker tests used
  `1 + "a"` as a sample type error, and the playground's `expand-components.test.mjs` used F22
  itself as a sample runtime failure; they now use `1 + true` and a division by zero, and the
  playground test also pins that the F22 expression evaluates.
- **The shortest-`float32` search in JavaScript can, at a power-of-two boundary, settle on one
  digit more than Rust's shortest formatter,** because `toPrecision` rounds to the nearest decimal
  and the round-trip interval is asymmetric there. The text still round-trips; the two backends
  could differ in length for such a value. No corpus value is affected.

### Playground examples (tasks 8.x)

The change's scope grew to wire the readouts the playground's ports had drawn as fixed strings.

- **The pattern is a page component.** The root function is evaluated, not instantiated, so each
  wired example wraps its tree in `component <Page />` with the readouts' values in `state`, as
  `text.nx` and `shapes.nx` already did. A helper component that holds controls (`Toggle`, the
  Common Controls `Card`) emits to the page and keeps no state of its own.
- **A method call becomes the property it would have set.** `GoPrev()`, `ScrollTo(2)`, `Close()`
  and `editor.Text = ...` are written as updates to the state a property is bound to
  (`SelectedIndex={index}`, `IsOpen={open}`, `Text={text}`). Two differences follow and are noted
  in `snapping.nx`: Prev and Next stop at the ends where the methods wrap a looped carousel, and
  "ScrollTo(0, no anim)" animates. Calls with no property behind them (`Seek(30)`, `SelectAll()`)
  stay inert and are tagged `code-behind`, whose definition now also covers calling into or reading
  from a control.
- **Binding `IsOpen` and `Text` both ways works.** The drawer reports `IsOpenChanged` while the
  bound flag is being applied, and an editor reports `TextChanged` on every key; neither loops,
  because the engine's setters ignore an unchanged value. Typing at 25 ms a key keeps every
  character. Playwright's zero-delay typing drops some, which no person reproduces.
- **`float64` text is what the original's template literal prints,** so `0.5x`, `1x`, `2x` and a
  slider's `29` match. `toFixed(1)` has no equivalent: the carousel's speed reads `1x`, not `1.0x`.
- **Static no longer means inert.** Six of the seven examples stay *static*, now for `code-behind`
  or `animation` alone, and respond to most of what the original responds to. The static coverage
  note and the `playground` spec say "some of its motion or interaction is absent". Common Controls
  became *complete*.
- **Four findings, F26 to F29,** are in `sites/playground/docs/FINDINGS.md`: the renderer drops the
  second of two events raised in one gesture, which a radio group does; the docs give the loop
  variables as `index, item` where the language is `item, index`; an `if` without `else` is `void`,
  so a loop cannot filter, contrary to the expressions reference; and ranges, list indexing, string
  length, list append, number formatting and `else if` are missing. None is fixed here.

