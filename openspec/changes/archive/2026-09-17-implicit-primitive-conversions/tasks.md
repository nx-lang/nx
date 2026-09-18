## 1. Types: the widening lattice

- [x] 1.1 Add `Primitive::widens_to` to `crates/nx-types/src/ty.rs` encoding
  `int32 → {int, int64, float64}`, `int → {int64, float64}`, `float32 → {float64}` plus
  reflexivity, and rewrite the same-category block of `Type::is_compatible_with` to call it; verify
  new unit tests in `ty.rs` accept the six widenings and reject `int64 → int`, `int64 → int32`,
  `int → int32`, `float64 → float32`, `int64 → float64`, `int → float32`, `int32 → float32`
- [x] 1.2 Reimplement `Primitive::numeric_promotion` as the narrowest primitive both operands widen
  to over the rank order `int32, int, int64, float32, float64`; verify unit tests give
  `int + float64 = float64`, `int32 + float32 = float64`, `int + int64 = int64`,
  `int32 + int = int`, and `None` for `int64` with either float
- [x] 1.3 Update `infer_binop` in `crates/nx-types/src/infer.rs` so arithmetic on two numeric
  operands uses the new promotion and the rejection diagnostic names both types and says the
  conversion is lossy; verify `check.rs` tests match the spec scenarios *int plus float64 is
  float64*, *int compared with float64 is boolean*, *int32 plus float32 is float64*, *int64 plus
  float64 is rejected*, *Integer division stays integer*
- [x] 1.4 Add `check.rs` tests for widening at every binding-site kind (property, default, record
  field, annotated `let`, return type, argument, list element, nullable, content property) and for
  the narrowing rejections with diagnostics naming both types; verify the tests pass and the
  existing `type_checker_tests.rs` assertions on `is_compatible_with` still hold
- [x] 1.5 Fix the existing tests and fixtures that relied on symmetric width compatibility, found
  by running `cargo test -p nx-types -p nx-interpreter -p nx-codegen -p nx-language-service` after
  1.1; verify the workspace test run is green with no test asserting a narrowing is accepted

## 2. Types: literals take their site's width

- [x] 2.1 Generalise `convert_int_literals` in `infer.rs` to integer targets: at an `int32` site
  record the literal as `int32` with an `i32` range check and a diagnostic naming the type, and at
  an `int64` site record `int64`; verify `check.rs` tests match *Integer literal binds at an int32
  site as int32* and *Integer literal out of range for int32 is rejected*
- [x] 2.2 Record a real literal at a `float32` site as `float32` (rounded, no exactness check), and
  extend the recorded-conversion set and `nx_hir::apply_int_literal_conversions` (renamed to cover
  both literal kinds) so the prepared module carries the width; verify *Real literal binds at a
  float32 site as float32* and the modified `contextual-numeric-literals` scenario *Integer literal
  binds at a float32 property* (recorded type `float32`) pass in `contextual_literals.rs`
- [x] 2.3 In `infer_binop`, apply the same literal conversion to a literal operand against the
  other operand's numeric type before promotion; verify a `check.rs` test infers `w * 1.5` as
  `float32` for `w:float32` and `n + 1` as `int32` for `n:int32`, and `let n = 42` / `let x = 1.5`
  still infer `int` / `float64`

## 3. HIR and types: the `+` decision moves to the checker

- [x] 3.1 Add `Expr::Concat { lhs, rhs }` and `Expr::ToText { expr, ty: Primitive }` to
  `crates/nx-hir/src/ast/expr.rs`, delete `BinOp::Concat`, and remove the `TypeTag`-based `Add →
  Concat` rewrite from `lower_expr` in `lower.rs`; verify `cargo build --workspace` passes with
  every `BinOp` match updated and the two `lower.rs` tests that asserted `BinOp::Concat` now assert
  `BinOp::Add`
- [x] 3.2 In `infer_binop`, type `Add` as `string` when either operand is `string` and the other is
  `string` or a stringifiable primitive, recording the expression id and each non-string operand's
  primitive in the inference context; reject any other non-numeric pairing with a diagnostic naming
  the operand type; verify `check.rs` tests match the spec scenarios *String plus int concatenates*
  (types only), *A string field access concatenates*, *String plus a record is rejected*, *String
  plus null is rejected*, *String plus a list is rejected*
- [x] 3.3 Add `nx_hir::apply_string_conversions` that rewrites each recorded `+` to `Expr::Concat`
  and wraps each recorded operand in `Expr::ToText`, and call it from `check.rs` next to
  `apply_int_literal_conversions`; verify a `check.rs` test over the prepared module shows
  `"Total: " + count` as `Concat(Literal, ToText(Ident, int))` and `"a" + "b"` as `Concat` with no
  `ToText`

## 4. Types: text bodies at `string` content properties

- [x] 4.1 In `check_content_binding` (`infer.rs`), when the content property is `string`/`string?`
  and the body has more than one piece, check each braced piece is `string` or a stringifiable
  primitive (diagnostic naming the type otherwise) and record the body as a join; extend
  `apply_string_conversions` to replace the content list with a `Concat`/`ToText` chain whose first
  run is left-trimmed and last run right-trimmed; verify `check.rs` tests match *Text and a braced
  int bind as one string* (accepted), *A braced record in a string body is rejected*, and that an
  `Element` content property body is untouched
- [x] 4.2 Add interpreter tests in `crates/nx-interpreter/tests` for the three accepted content
  scenarios (`"Total: 3"`, `"Ada Lovelace"`, the multi-line indented body) and for a single-run
  body; verify they pass
- [x] 4.3 Treat line breaks as layout in a joined body: in `apply_string_conversions`, turn each
  whitespace stretch containing a line break (in a gap or a text run) into one space, join a lone
  text run too, and keep raw text as written; add interpreter tests for the *Line breaks between
  pieces read as one space*, *Braced values on separate lines are joined with a space*, *A text run
  wrapped across lines reads as one line* and *A line break inside a braced string is kept*
  scenarios; verify they pass

## 5. Interpreter

- [x] 5.1 Add `Value::to_text()` in `crates/nx-interpreter/src/value.rs` implementing the canonical
  text form (integers as decimal; `f64` and `f32` via a shared formatter that adds ECMAScript
  exponent form for magnitudes ≥ 1e21 or < 1e-6, prints `-0` as `0`, and spells `NaN`, `Infinity`,
  `-Infinity`; booleans as `true`/`false`); verify unit tests cover `1.0 → "1"`, `0.1 → "0.1"`,
  `1e21 → "1e+21"`, `1e-7 → "1e-7"`, `-0.0 → "0"`, `f32 0.1 → "0.1"`, `123456789012345680000 → "123456789012345680000"`
- [x] 5.2 Evaluate `Expr::Concat` and `Expr::ToText` in `interpreter.rs`, delete `eval_concat` and
  the `BinOp::Concat` arm from `eval/arithmetic.rs`; verify interpreter tests in `operators.rs`
  match *String plus int concatenates*, *Primitive on the left concatenates*, *Boolean
  concatenates*, *Concatenation is left-associative*, *A string field access concatenates*
- [x] 5.3 Add cross-category arms to `eval_add`/`eval_sub`/`eval_mul`/`eval_div`/`eval_mod` and to
  `values_equal`/`eval_lt`/`eval_le`/`eval_gt`/`eval_ge` that convert the integer operand to `f64`;
  verify `floats.rs` tests give `f(1, 1.5) = 2.5`, `2 == 2.0` is `true`, `2 < 2.5` is `true`, and
  `7 / 2` with `int32`/`int` operands is `3`
- [x] 5.4 Widen at binding sites in `coerce_non_record_value` (`Int32 → Int`, `Int32|Int → Float`,
  `Float32 → Float`, recursively through arrays and nullable), so a widened value carries the
  site's type; verify interpreter tests show an `int` parameter bound at a `float64` property
  evaluates to a float value, a mixed `int`/`int32` list at `float64[]` evaluates to floats, and an
  `int` at `float64?` is accepted

## 6. NX IR and the TypeScript runtime

- [x] 6.1 Add `kinds::node::TEXT = 20` (with its `NAMES` entry) to `crates/nx-codegen/src/ir.rs`,
  `CodegenExpressionKind::Concat` and `CodegenExpressionKind::ToText { expr, ty }` to `model.rs`
  built from the new HIR nodes in `builder.rs`, emission as `[20, node, str]` and `concat` for
  `Concat`, the layout row in `ir_image.rs`, and an `ir_explain.rs` rendering; bump
  `NX_IR_SCHEMA_VERSION` to 4; verify `ir_tests.rs` cases over explained text match the four
  `nx-ir-format` scenarios (*concat over a text node*, *float32 operand names its type*, *string
  field access*, *widened operand carries no conversion node* including `div` not `idiv`)
- [x] 6.2 In `runtime/typescript/src/index.ts`, add `text: 20` to `nodeKinds`, validate the
  `[20, node, str]` layout and the primitive name, evaluate it with `String(value)` except
  `float32` (shortest round-trip `float32` digits, checked with `Math.fround`), make `concat` fail
  with `nx-ir-operator` on a non-string operand, and accept schema 4 / refuse 3 naming both;
  verify `runtime.test.ts` cases match the six `typescript-ir-runtime` scenarios
- [x] 6.3 Update `docs/nx-ir-format.md` (schema 4, the `text` node row, `concat` as strings-only,
  the schema-differences note) and the TS runtime README's schema mention; verify the docs name
  kind 20 and version 4 and `runtime/typescript/dist` is rebuilt with `pnpm --filter
  @nx-lang/ir-runtime build`
- [x] 6.4 Add conformance programs under `specs/ir-conformance` for string `+` over `int`, `float64`
  (`1.0`, `0.1`, `1e21`, `1e-7`, `-0.0`), `float32` `0.1`, `boolean`, a string field access, mixed
  `int`/`float64` arithmetic and comparison, and a widened `int` at a `float64` property; regenerate
  with `NX_UPDATE_CORPUS=1 cargo test -p nx-codegen`; verify `cargo test -p nx-codegen` and
  `pnpm --filter @nx-lang/ir-runtime test` both pass over the new fixtures with identical text
- [x] 6.5 Update `binop_text` and the `Binary` arms in `crates/nx-codegen/src/emit.rs` for the new
  HIR shape: `Concat` emits `+`, `ToText` emits `String(x)` or the exported `float32` helper for a
  `float32` operand; verify the codegen tests in `tests.rs` pass and a new case shows the `float32`
  helper in generated JavaScript

## 7. Language service, bindings and docs

- [x] 7.1 Run `cargo test --workspace` and the .NET and Node SDK test suites
  (`bindings/dotnet/tests`, `bindings/node/test`) against schema 4; verify all pass and any snapshot
  of an IR image or schema version is updated
- [x] 7.2 Rewrite the numeric section of `docs/src/content/docs/language-tour/types.md` to describe
  one-directional widening, the exact `int → float64` crossing, literal widths, and the rejected
  narrowings, replacing the paragraph and example that say an `int` expression is not converted;
  verify the page contains the `let scaled(n: int) = { <Box width={n} /> }` example as accepted
- [x] 7.3 Add an *Operators* section to `docs/src/content/docs/reference/syntax/expressions.md`
  covering arithmetic promotion, comparison, string `+` with a primitive operand (and what it
  rejects), left-associativity, and the canonical text forms; and document text bodies at `string`
  content properties in `language-tour/textual-content.md`; verify `pnpm --dir docs build` succeeds
- [x] 7.4 Update `sites/playground/src/examples/nx/reorder.nx` to use `"Reorder " + Item.Title` for
  the accessibility label and delete the workaround comment; verify
  `node sites/playground/scripts/check-examples.mjs` passes and the playground renders the example
- [x] 7.5 Remove finding F22 (fixed) from `sites/playground/docs/FINDINGS.md`, which lists only
  open findings, and update `specs/future.md` if it references the lowering-time `+` rewrite; verify
  `grep -rn "BinOp::Concat" --include=*.rs --include=*.md .` returns nothing outside
  `openspec/changes/archive`

## 8. Playground examples

- [x] 8.1 Wire the tap counters in `transforms.nx` and `accessibility.nx` to page state with
  `"Tapped " + count + "×"`, and the Accessibility toggles and "last activated" readout with them;
  verify in a browser that a tap moves "Tapped 0×" to "Tapped 1×"
- [x] 8.2 Wire the Common Controls "Last:" readout in `looks.nx`: each card emits one line of text,
  built from a toggle's boolean or a slider's `float64`, and the page keeps the latest; verify in a
  browser that a switch reports `switch: false` and a slider a whole number such as `slider: 29`
- [x] 8.3 Wire `animations.nx`: the SpeedRatio buttons generated from a `float64` list
  (`0.5x`, `1x`, `2x`), the IsOn toggle with `"IsOn=" + toggle` in its title, and the Lottie and GIF
  status readouts; leave the Start/Stop/Seek buttons inert with a code-behind note
- [x] 8.4 Wire `scroll.nx`: the two CurrentIndex readouts from `onCurrentIndexChanged`, and replace
  the hand-numbered rows and tiles with loops that number them from the loop index; verify the page
  still draws "Row 1" to "Row 14", "H 1" to "H 14" and "Snap 0" to "Snap 9", and that a swipe moves
  CurrentIndex off `-1`
- [x] 8.5 Wire `snapping.nx`: the toggles, SidesOffset, Prev/Next/ScrollTo/Set index through
  `SelectedIndex`, the stretching indicator dot, the status line, the Swipe Speed buttons, the four
  SelectedIndex readouts, the twelve looped slides as a loop, and the drawer's `IsOpen`; verify in a
  browser that the drawer opens and closes from its buttons and the readout follows
- [x] 8.6 Wire `editor.nx`: Text, IsFocused and the submitted value in the first card's title, the
  Focus, Set Text and Clear buttons, and the chat card's sent count; verify typing, Clear and two
  submitted messages in a browser
- [x] 8.7 Restate coverage in `src/examples/examples.json` (Common Controls complete; the others
  static with the capability they still lack), reword the static coverage note in
  `src/examples/types.ts`, and update the README; verify `pnpm --dir sites/playground test` and
  `pnpm --dir sites/playground typecheck` pass
- [x] 8.8 Pin the readout in `src/render/instances.test.mjs`: an int, a boolean and a `float64` in
  one label, before and after a tap
- [x] 8.9 Record what the wiring found in `sites/playground/docs/FINDINGS.md` (F26–F29) and remove
  F22 (fixed)

## 9. Joins (review finding RF5)

- [x] 9.1 Record each `if`/`match` branch and list element narrower than its join in the checker
  (`widened_joins`), wrap each in a new `Expr::Widen` with `nx_hir::apply_join_widenings`, widen
  the value in the interpreter, and build the branch unchanged in codegen
- [x] 9.2 Lift nullability out of the join (`nullable_join` in `semantics.rs`, used by both join
  functions), so `null` with `T` is `T?` and `A?` with `B` is `join(A, B)?`, with `object` staying
  `object`
- [x] 9.3 Add checker tests for the rewrite's shape, the wrapper's type and the nullable joins
  (`crates/nx-types/tests/nullable_joins.rs`), and interpreter tests in `tests/floats.rs`
- [x] 9.4 Document joined branches in `reference/syntax/types.md`; verify `cargo test --workspace`
  and the playground's `check-examples` against a rebuilt WASM SDK

## 10. Review fixes (RF11–RF16)

- [x] 10.1 Give a host-supplied number the width of its `int32` or `float32` site in the
  interpreter (`host_number_at_site`, for props, state, dispatch batches and entry-call
  arguments), with range and exactness checks; verify with `nx-api` and `tests/floats.rs` tests
- [x] 10.2 Lower a typed text body's `EMBED_TEXT_RUN` as text so it joins at a `string` content
  property; verify with a `tests/text_content.rs` test
- [x] 10.3 Accept `T?` at `U?` when `T` widens to `U` in both compatibility relations; verify with
  checker and interpreter tests for `int?` at `float64?` and `int?[]` at `float64?[]`
- [x] 10.4 Cover joins across backends: `if`, match and list joins in the `conversions` corpus, an
  `ir_tests.rs` case asserting `div`, a generated-JavaScript agreement case, and a match-arm
  interpreter test
- [x] 10.5 Add the `float32` binary operators `fadd32`, `fsub32`, `fmul32` and `fdiv32` to NX IR
  (RF14), evaluate them with `Math.fround` in the TS runtime and generated JavaScript, round a host
  number at a `float32` site in both JS targets (and check `int32` sites in the TS runtime), and
  verify with `conversions` corpus cases, an `ir_tests.rs` case, generated-JavaScript agreement
  cases and TS runtime tests
- [x] 10.6 Fold constant expressions at a narrowing site or operand (RF17): `fold_constant` and
  `folded_constants` in the checker, `nx_hir::apply_constant_folds`, compile-time diagnostics for
  division by zero and `int` overflow, and a "Constant expressions" section in
  `reference/syntax/types.md`; verify with `tests/floats.rs` and `ir_tests.rs` cases
- [x] 10.7 Keep a widened branch's own type in the codegen builder (RF19), so its operators are
  chosen by its operands; verify with an `ir_tests.rs` case over an `if`, a match arm and a list
  element, `conversions` corpus cases and generated-JavaScript agreement cases
- [x] 10.8 Emit an integer `/` as the runtime helper `nxIntDiv` in generated JavaScript (RF20);
  verify with generated-JavaScript agreement cases
- [x] 10.9 Decode `\@`, `\{` and `\}` in a text run when lowering it (RF21); verify with a
  `tests/text_content.rs` test
- [x] 10.10 Print a `float32` in `nxlang run`'s NX output from its own shortest digits
  (`format.rs`); verify with a `format.rs` test
- [x] 10.11 Emit every `/` and `%` in generated JavaScript through `nxIntDiv`, `nxDiv` or `nxMod`
  (RF22), so a zero divisor fails as it does on the other backends; verify with generated-JavaScript
  agreement cases and a failure test
- [x] 10.12 Keep a typed text body's line breaks and take off the indentation its lines share
  (RF23): record `text_type` on `Element`, give the join a typed-body rule (`dedent_typed_body`),
  and update the spec, the language tour and `tests/text_content.rs`, with a `conversions` corpus
  case
- [x] 10.13 Rewrite the `specs/future.md` entry on typed bodies to what is still open (RF24), and
  move the root `future.md` entries into `specs/future.md` so there is one future-work file
