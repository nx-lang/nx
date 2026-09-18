## Why

NX has no way to put a number into a string. `"Total: " + count` is a type error, and a text body
such as `<Label>Total: {count}</Label>` cannot bind to a `string` content property at all. That is
the single most common thing a TypeScript user writes on their first day, and it is the idiom NX's
own playground examples had to route around (`sites/playground/src/examples/nx/reorder.nx` notes
that `"Reorder " + Item.Title` compiles but lowers to a numeric add and fails at run time). At the
same time the numeric rules are lax in the wrong direction: `int64` silently binds at an `int32`
site and `float64` at `float32`, a narrowing no strongly typed language permits implicitly, while an
`int` expression is refused at a `float64` site even though NX defines `int` as exactly the range
`float64` represents without loss.

This change lays down one rule for which conversions happen on their own and applies it to the
primitive types: a conversion is implicit only when it is total, exact, and has one obvious result.
That gives TypeScript users the behavior they expect, keeps NX strongly typed, and leaves a stated
rule to decide future types by.

## What Changes

- **Numeric widening in one direction only.** `int32 → int → int64` and `float32 → float64` are
  accepted wherever a value binds, and so are the two exact crossings into floating point,
  `int32 → float64` and `int → float64`. The lossy crossings (`int64 → float64`, anything to
  `float32`) are not. Widening applies to expressions, not just literals: a parameter typed `int`
  now binds at a `float64` property. Mixed arithmetic and comparison follow the same rule, so
  `n + 1.5` with `n:int` is `float64` and `n == 1.5` is `boolean`.
- **BREAKING: narrowing is rejected.** An `int64` expression at an `int32` or `int` site, an `int`
  at an `int32` site, and a `float64` at a `float32` site are type errors. Today all four are
  accepted.
- **A join widens its branches, and lifts nullability.** The branches of an `if`/`else`, the arms
  of a match, and the elements of a list are typed at the narrowest type they all widen to, and a
  narrower branch evaluates at that type, so `(if b { n } else { x }) / d` divides as floats on
  every backend. Joining with `null` or a nullable branch yields the nullable join, so
  `if b { n } else { null }` is `int?` rather than `object`.
- **Integer and real literals take the numeric type of their site.** An integer literal at an
  `int32` or `int64` site is typed as that width (rejected if out of range), and a real literal at a
  `float32` site is typed `float32`. Without this the narrowing rule would refuse `1` at `int32` and
  `1.5` at `float32`. A literal operand of a binary operator takes the numeric type of the other
  operand on the same terms, so `width * 1.5` stays `float32` when `width` is.
- **`+` concatenates when either operand is a string.** The other operand may be any numeric
  primitive or `boolean`, and is rendered in its canonical text form. `null`, nullable types,
  records, lists, functions, and union cases are not stringified: `"a" + null` and
  `"a" + someRecord` remain errors. `+` is left-associative, so `1 + 2 + " items"` is `"3 items"`.
- **Text body content binds to a `string` content property as one string.** A body made of text
  runs and braced values whose types are stringifiable primitives binds as the concatenation of
  their text forms, so `<Label>Total: {count}</Label>` works when `Label` declares
  `content text:string`. Line breaks in such a body are layout and read as one space, and its outer
  whitespace is trimmed. That includes a body of a single text run, which is a behavior change: a
  body written on its own lines between the tags binds `"Total"` where it bound `"Total\n"`.
  Element bodies under built-in tags, which are not type-checked today, are unchanged.
- **One canonical text form per primitive**, shared by the interpreter, the NX IR runtime, and
  generated code: integers as decimal digits, floating-point values in the ECMAScript
  `Number.prototype.toString` form (shortest round-trip, `1.0` prints as `1`), and `boolean` as
  `true` or `false`. A `float32` prints as the shortest text that round-trips as a `float32`, so
  `0.1` prints as `0.1`.
- **NX IR makes the string conversion explicit.** A non-string operand of a string `+` is wrapped in
  a new `text` node that names the operand's primitive type, so the IR runtime formats a `float32`
  correctly without static types, and `concat` only ever sees strings. This bumps the IR schema to 4.
- The HIR `Concat` operator is removed. Lowering chose it from a local type tag that is `Unknown`
  for any non-literal operand, which is the `reorder.nx` bug. Whether `+` adds or concatenates is
  now decided once, by the type checker, and both the interpreter and the IR emitter read that
  decision.

- **The playground's examples use the conversion.** Seven ported DrawnUI pages drew a fixed
  "Tapped 0×" or "SelectedIndex=1" where the original formats a number, and spelled out rows
  numbered by hand, because a number could not become text. Their readouts are wired to component
  state and their numbered rows become loops, so the conversion is exercised by real programs in a
  browser and not only by unit tests. The gaps wiring them finds are recorded as findings.

Not in this change: explicit conversion functions (`string(n)`, `float64(n)`, `int32(n)`), which
are the natural next step and the escape hatch for the narrowings this change rejects;
string-to-number parsing; `boolean` to and from numbers; truthiness in conditions; stringifying
records or lists; type-checking of element content under built-in tags.

## Capabilities

### New Capabilities
- `implicit-primitive-conversions`: the rule for which primitive conversions are implicit; the
  one-directional numeric widening lattice; string `+` with a primitive operand; text body content at
  a `string` content property; the canonical text form of each primitive; and the runtime behavior
  each backend must share.

### Modified Capabilities
- `primitive-type-names`: the requirement *Numeric compatibility between and within the numeric
  categories* changes from symmetric compatibility to one-directional widening, and its scenario
  rejecting an integer-typed expression at a float site is inverted for `int` and `int32`.
- `contextual-numeric-literals`: the requirement *Contextual typing does not widen integer-typed
  expressions* is removed; its rationale (that `int` spans a 64-bit range) predates `int` being
  specified as ±(2^53−1). A real literal at a `float32` site now records `float32`, answering the
  question that spec left open, and an integer literal takes an integer width from its site.
- `nx-ir-format`: the eager expression set gains the `text` conversion node and the schema version
  becomes 4; `concat` is specified as taking string operands only.
- `typescript-ir-runtime`: evaluates the `text` node with the canonical text form, and `concat`
  no longer coerces non-string operands.

- `playground`: a new requirement that an example formats a number or a boolean into a readout
  where the original does, and the definition of a *static* example changes from one where nothing
  responds to one where some of the original's motion or interaction is absent, since a wired
  example responds and may still leave out what the original does by calling into a control.

## Impact

- `crates/nx-types`: `Type::is_compatible_with` and `Primitive::numeric_promotion` (`ty.rs`),
  `common_supertype` (`semantics.rs`), `infer_binop` and the literal conversion path in `infer.rs`,
  plus the checker recording which `+` expressions concatenate and which operands need a string
  conversion, alongside the existing `converted_int_literals` mechanism.
- `crates/nx-hir`: `BinOp::Concat` and the `TypeTag`-based operator rewrite in `lower.rs` go away.
- `crates/nx-interpreter`: `eval_add` and `eval_concat` in `eval/arithmetic.rs`, mixed-category
  arithmetic and comparison in `eval/arithmetic.rs` and `eval/logical.rs`, widening at binding sites
  in `coerce_non_record_value`, and a canonical float formatter for `Value`'s text form.
- `crates/nx-codegen`: `binary_operator` in `ir.rs` chooses `concat` from the checked type, the new
  `text` node, schema version 4, `binop_text` in `emit.rs`, and the conformance corpus under
  `specs/ir-conformance`.
- `runtime/typescript`: the `text` node kind, the schema-4 validator, and `evalBinary`'s `concat`.
- Documentation: `docs/nx-ir-format.md`, `docs/src/content/docs/language-tour/types.md` (which
  currently documents the no-widening rule), and the expressions reference, which has no operator
  section today and gains one.
- `sites/playground`: seven examples under `src/examples/nx` (Transforms, Accessibility, Common
  Controls, Lottie & GIF, SkiaScroll, Carousel & Drawer, Editor) move their readouts into a page
  component's state; `examples.json` restates their coverage, with Common Controls becoming
  complete; the static coverage note, the README and `docs/FINDINGS.md` follow.
- Existing programs that relied on silent narrowing stop type-checking. The DrawnUI catalog uses
  `float64` exclusively and the playground examples contain no cross-width arithmetic, so no shipped
  example is affected.
