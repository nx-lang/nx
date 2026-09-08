## Why

Nearly every number an NX author writes at a UI property is a whole number, and today every one of
them must be spelled with a decimal point. In the DrawnUI fiddle's ported examples, **634 of 713
float literals are `N.0`** — 89% of the floats in the corpus are integers wearing a `.0` to satisfy
the type checker:

```nx
<SkiaStack Spacing=8.0 WidthRequest=150.0>
  <SkiaLabel FontSize=13.0 TextColor="#ADB5BD" Margin=<Thickness Top=8.0 /> />
```

The `.0` carries no information. It is not a precision claim, not a unit, and not a choice — the
property is `float64`, so there is exactly one type the number could have had. It exists only
because `primitive-type-names` currently says an integer literal is rejected at a float site, and
the cost is paid on every line of every UI written in NX.

Nothing about the value is ambiguous. `8` at a `float64` property means 8.0 and can mean nothing
else. A language that makes the author restate what the declaration already fixed is charging for a
distinction it does not draw.

## What Changes

An integer literal SHALL be accepted at a floating-point binding site and SHALL take that site's
float type, so `Spacing=8` and `Spacing=8.0` mean the same thing.

- An integer literal whose expected type is `float32` or `float64` is bound at that float type
  rather than as `int`, at every site that supplies an expected type: property bindings, property
  and field defaults, annotated `let` bindings, elements of a list at a float-element site, record
  fields, arguments at a float-typed parameter, and element body content. `let x: float32 = 42` and
  `let x: float32 = 42.0` both give `x` the type `float32`. The type recorded for the literal
  *expression* is whichever one an explicit real literal takes at the same site — today `float64`
  everywhere, since a literal node carries no width — because the two spellings have to stay
  indistinguishable.
- The literal is **converted**, not merely tolerated. The value that reaches the interpreter, NX IR,
  and generated C#/TypeScript is a float, so downstream consumers see the type the declaration
  promised rather than an integer they must widen themselves.
- A literal that cannot be represented **exactly** in the target float type is rejected with a
  diagnostic. `float32` runs out of exact integers at 2^24 and `float64` at 2^53; silently rounding
  an author's constant is worse than the `.0` this change removes.
- **Not** general `int`→`float` widening. An expression of type `int` at a float site is still
  rejected. The rule is about how a literal is *typed*, not about converting values of a known
  integer type, which for `int` (64-bit) is lossy above 2^53 and cannot be checked at compile time.
- A float literal at an integer site remains rejected, unchanged. That direction is lossy and no
  argument here applies to it.
- The DrawnUI fiddle's NX examples, the repository's NX examples, and NX snippets in documentation
  drop the redundant `.0` so the corpus reads the way the language now allows.

No existing source breaks: every program that compiles today still compiles and still means the same
thing. This is purely an addition to what is accepted.

## Capabilities

### New Capabilities
- `contextual-numeric-literals`: how an integer literal takes its type from the site it is bound at,
  which sites supply that expectation, the exactness rule that governs when the conversion is
  allowed, and how the converted value is carried through evaluation, NX IR, and code generation.

### Modified Capabilities
- `primitive-type-names`: the requirement *Numeric compatibility is unchanged by the renaming*
  states that the system "SHALL continue to reject an integer value at a floating-point binding
  site", with a scenario asserting that `<B v=1 />` at `v:float64` is rejected. It is replaced by
  *Numeric compatibility between and within the numeric categories*, which narrows the rule to
  integer-typed *expressions*; the literal inverts. The literal-inference requirement is amended to
  say that inference from spelling is the fallback when no type is expected.
- `unbraced-literal-forms`: the requirement *First-party NX-syntax value output round-trips* keeps
  its behavior — a float value is still rendered `24.0` — but its stated reason ("so that it binds
  at a float-typed site rather than as an integer literal") stops being true and is replaced by the
  reason that survives: at a site with no expected type, such as an unannotated `let`, the spelling
  is the only thing that distinguishes a float from an int.

## Impact

**Type checking** — `crates/nx-types`: `Type::is_compatible_with` is type-only and cannot see
literals, so the expectation has to reach the literal. `check_typed_binding_for` already takes the
`ExprId` for exactly this kind of context-sensitive resolution (it is how `Type::ContextualName`
turns a bare `Center` into a union case), which is the seam to reuse. Literal inference in
`infer.rs` gains an expected-type path alongside the existing default of `int`.

**Evaluation and emission** — `crates/nx-interpreter` (`coerce_value_to_resolved_type`),
`crates/nx-codegen` (`NxIrLiteral::Int` vs `Float` selection in `ir.rs`, and float spelling in
`emit.rs`), and `crates/nx-value` (`NxValue::Int` vs `Float`) all have to agree that the converted
literal is a float, or the fiddle's TypeScript IR runtime receives an integer where DrawnUI expects
a double.

**Diagnostics** — the existing "expects float64, found int" message for a literal is replaced by
acceptance; a new message is needed for the inexact case. `nx-language-service` needs no change
beyond agreeing with the checker, which is verified by a diagnostics test. Reporting the float type
a literal took in quick-info would be a good thing to have, but it is **out of scope here**: the
service has no expression-level type API at all, so it is a feature of its own rather than a
consequence of this rule. See design's Open Questions.

**Corpus** — `sample-apps/drawnui-react/src/examples/nx/` (12 files, ~634 literals),
`sample-apps/drawnui-react/catalog/skia.nx` (104 `float64` properties, whose defaults are the other
place `.0` appears), `examples/nx/`, and the NX snippets in `docs/` and the fiddle's `docs/`. These
are mechanical edits, but they are the point of the change: the proof it worked is that the corpus
gets shorter and still compiles, evaluates, and renders identically.

**No migration.** Old spellings stay valid, so the corpus update can land incrementally and
independently of the compiler change.
