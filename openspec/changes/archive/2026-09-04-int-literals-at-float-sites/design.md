## Context

See `proposal.md` — Why. The constraints that shape the approach:

**Compatibility is type-only and cannot see literals.** `Type::is_compatible_with`
(`crates/nx-types/src/ty.rs:312`) takes two `Type` values. It has no way to distinguish `24` from an
`int`-typed variable, so any rule expressed there necessarily admits both — which is precisely the
line this change must not cross.

**The checker already has the seam.** `check_typed_binding_for`
(`crates/nx-types/src/infer.rs:2433`) takes `Option<ExprId>` alongside the actual and expected types,
"so a resolved contextual name can be rewritten to its qualified form after analysis". That is the
existing mechanism for a value whose meaning depends on the site it is written at: a bare `Center`
is `Type::ContextualName` until the expected type says which union it belongs to.

**That resolution is applied by rewriting HIR, once, before anything downstream runs.**
`InferenceContext` accumulates `resolved_contextual_names: FxHashMap<ExprId, ContextualResolution>`,
and `check.rs:199-238` drains it into `nx_hir::apply_contextual_name_resolutions`, which mutates the
prepared module *before it is snapshotted*, "so every consumer after type checking sees the
qualified member access rather than the bare source spelling."

**Four consumers read literals independently.** `crates/nx-interpreter/src/interpreter.rs:1870`
(`Literal::Int(n) => Value::Int(*n)`), `crates/nx-codegen/src/ir.rs:1688` (`NxIrLiteral::Int`),
`crates/nx-codegen/src/emit.rs:4030` (C# spelling), and the value formatter. None of them has the
expected type in hand.

## Goals / Non-Goals

**Goals:**

- One place decides that a literal is a float, and everything downstream sees a float without asking.
- No consumer of HIR below type checking learns about contextual typing.
- The corpus update is verified by output equality, not by "it still compiles".

**Non-Goals:**

- Bidirectional type inference as a general mechanism. This is one rule for one literal kind at sites
  that already carry a declared type; it is not a step toward a full bidirectional checker, and the
  design deliberately does not build infrastructure for one.
- Any change to how `float32` is distinguished from `float64` in HIR. It is carried by the declared
  type today and continues to be.
- Contextual typing for string, boolean, or list literals.

## Decisions

### D1: Type the literal by context; do not widen the `int` type

**Decision.** The rule attaches to `Expr::Literal(Literal::Int(_))` appearing at a site whose
expected type resolves to a float primitive. It does not attach to `Type::Primitive(Int)`.

**Why.** A literal's value is known during analysis, so exactness (D3) is decidable then. `int` is
64-bit, and its upper range is not exactly representable in any float type; a widening rule stated
over types would have to either reject values it cannot see or accept silent precision loss at run
time. The narrow rule also has a bounded blast radius: `common_supertype` and numeric promotion are
untouched, so `1 + 1.5` remains whatever it is today rather than quietly acquiring a meaning.

**Alternative considered — allow `int`→`float` in `is_compatible_with`.** One line, and it would
make the examples compile. Rejected: it cannot distinguish the literal from the expression, which
means shipping the lossy case in order to get the notational one, and it would make the compatibility
relation asymmetric in a way `common_supertype` and array/function variance would then inherit.

### D2: Rewrite the literal in HIR after analysis, mirroring the contextual-name pass

**Decision.** `InferenceContext` gains a second resolution map — `ExprId → float primitive` — filled
where `check_typed_binding_for` accepts an integer literal at a float site. `check.rs` drains it
alongside `resolved_contextual_names` and applies it through a sibling of
`apply_contextual_name_resolutions` that replaces `Literal::Int(n)` with `Literal::Float(n as f64)`
in the prepared module before the snapshot.

**Why.** It makes the interpreter, the IR emitter, C# emission, and the formatter require *no
changes at all*: by the time they see the module, `Spacing=24` is already `Literal::Float(24.0)` and
is indistinguishable from source that wrote `24.0`. That indistinguishability is not a convenience —
it is the strongest available guarantee that the spec requirement "a program written with `24` at a
floating-point site SHALL be indistinguishable ... from the same program written with `24.0`" holds
for consumers that do not yet exist.

**Alternative considered — teach each consumer to consult the expected type.** Rejected: it
duplicates the same decision in three crates, each of which would need type information it currently
does not carry, and the first consumer to forget produces an integer where a double was promised —
which in the fiddle means DrawnUI receiving an `int` for a `double` property.

**Alternative considered — convert during lowering.** Rejected: declared types are not resolved yet
at lowering time.

### D3: Reject a literal that is not exactly representable; do not round

**Decision.** Acceptance requires that the literal's value round-trip through the target float type
unchanged (`value as f64 as i64 == value`, and the `f32` equivalent for `float32`). Failure is an
error naming the literal, the float type, and the fact of inexactness.

**Why.** The change's whole claim is that `24` and `24.0` mean the same thing. That claim is only
true where the conversion is exact, and an author who writes a 17-digit constant at a `float64`
property has a real problem that rounding would hide. Rejecting is also forward-compatible: the rule
can be relaxed later, while un-rounding a shipped program cannot.

**Alternative considered — a single 2^53 bound.** Rejected: it is wrong for `float32`, which loses
exactness at 2^24. Round-tripping the actual value is both simpler to state and correct for each
width.

### D4: `float32` is carried by the declared type, not by the literal

**Decision.** The rewrite produces `Literal::Float`, whose payload is an `f64`, even when the site is
`float32`. The narrowing to 32 bits happens where it happens today.

**Why.** This is already how an explicit `1.5` at a `float32` site behaves; the literal node has no
width and the declared type is authoritative. Introducing a width-carrying literal to serve this
change would be a larger and unrelated change to HIR, and it would make the rewritten `24` *differ*
from a written `24.0` — the opposite of D2's guarantee.

### D5: The formatter keeps emitting `24.0`

**Decision.** Rendering a float value still produces a real-literal spelling. Only the reason
changes, per the `unbraced-literal-forms` delta.

**Why.** Round-tripping has to hold at sites with no expected type. `let x = 24` infers `int`; if the
formatter shortened a float `24.0` to `24`, a rendered value re-read into an unannotated binding
would come back as a different type. The new acceptance rule helps a reader at a *declared* site
only, and the formatter cannot know it is emitting into one.

### D6: Apply the rule only where the expected type resolves to a float primitive

**Decision.** The trigger is: strip nullability, and if the result is `Type::Primitive` of `float32`
or `float64`, the literal is converted. Every other expected type — `object`, an unresolved type
variable, a union, `int`, absent — leaves the literal as `int`.

**Why.** `type_satisfies_expected` accepts anything at an `object` site, so without this restriction
an integer literal bound to `object` would silently become a float and change the value a host
receives. An unresolved type variable must also not trigger: the literal would be converted on the
basis of an expectation that has not been decided yet.

**Consequence to handle.** The scalar-to-list coercion in `type_satisfies_expected_with_coercion`
means a literal can be bound at a `float64[]` site. There the element type is the expectation, so
the trigger must be evaluated against the coercion target rather than the annotation as written.

**Second consequence.** Element body content has no expression of its own: it is a *sequence* of
expressions, and the binding site names only the content property. So the three content paths were
the only ones calling `check_typed_binding` with no `ExprId`, and the rule could not reach them —
found by review rather than by the first pass. They now get the same treatment a list does: one
content expression is checked as itself, several are checked as the elements of the declared list.

### D7: Verify the corpus edit by IR equality, not by compilation

**Decision.** Stripping `.0` from the fiddle examples is scripted, and the acceptance test is that
the emitted NX IR for each example is byte-identical before and after the edit. The existing
`npm run check-examples` (compiles, evaluates, declares coverage) runs on top of that, not instead.

**Why.** 634 mechanical edits across 12 files is exactly the kind of change where "it still compiles"
hides a mistake — a `.0` stripped from a string literal, a color, or a version. IR equality is a
total check and costs one script.

## Risks / Trade-offs

**A `.0`-stripping script edits something that is not a float property** (a string, a color, a
version in a comment) → The script only rewrites `=N.0` in property position in `.nx` files, and D7's
IR-equality gate catches any edit that changed meaning, including ones that still compile.

**An author now cannot tell from the source whether a property is `int` or `float`** → Real, and
accepted: it is the cost of the notation, and the same cost every language with contextual literals
pays. The declaration remains the answer, as it already is for `Center`. The language service is the
natural place to surface it without opening the declaration — but it cannot today and this change
does not make it: `hover` matches top-level document symbols only, and there is no expression-level
type API to hang it on. That is the Open Question below, deliberately left to its own change, so
this risk is **accepted unmitigated** rather than mitigated.

**Contextual typing spreads.** Once literals are typed by context, the next request is for `int`
expressions, or for string literals against unions → The spec states the boundary as a requirement
("Contextual typing does not widen integer-typed expressions") with its reasoning, so a future
proposal has to argue against a stated position rather than extend an unstated one.

**The rewrite makes a source expression differ from its HIR.** A tool reading post-check HIR sees
`24.0` where the file says `24` → This is already true of contextual names, and spans are unchanged,
so diagnostics and source maps still point at the written text. Anything needing the original
spelling reads the source, as it must today.

**Two rewrite passes over the prepared module** → They are independent (a contextual name is not an
integer literal), so ordering does not matter; they can share one traversal if it matters, which it
likely does not.

**The recorded type has to move with the value, and it is not the target.** Rewriting the literal
node alone left the IR carrying a float literal under an integer type annotation — found by the
equivalence test, not by reading. The expression's recorded type becomes `float64`, which is what a
written real literal takes at the same site; recording the *target* would make a converted `24` more
precisely typed than the `24.0` it must be indistinguishable from, at a `float32` site. Whether a
float literal should narrow at a `float32` site is a real question, but it is the same question for
both spellings and not one this change answers.

**Type generation does not go through type checking.** `nx-cli`'s `generate_types` reads a lowered
module, so the HIR rewrite has not run when it emits a field default. `= 0` on a `double` compiles,
but the same declaration would generate different text depending on which pipeline produced it, so
that path settles the spelling from the field's own type instead.

## Migration Plan

No migration. The change is purely additive to what type checking accepts, so every existing program
keeps compiling and keeps its meaning. The compiler change and the corpus cleanup are independent and
can land in either order; the corpus cleanup simply cannot land first.

Rollback is reverting the compiler change, which re-rejects sources written in the new style — so the
corpus cleanup should land after the compiler change has been exercised, not in the same commit.

## Open Questions

- Whether the language service should surface the contextually chosen float type in hover or as an
  inlay hint. It is a presentation decision that changes no spec requirement and no task here — and
  not a small one: `hover` (`crates/nx-language-service/src/lib.rs:316`) matches a position against
  top-level document symbols only, and the service exposes no expression-level type API at all, so
  surfacing it means mapping a byte offset to an `ExprId` and reading the analysis type environment.
  That is infrastructure serving every expression, not this rule.
- ~~Whether the catalog's own defaults should be rewritten in the same pass as the examples.~~
  **Resolved during implementation: there is nothing to rewrite.** The catalog declares no property
  defaults at all — every property is optional, because "DrawnUI's own defaults are the defaults"
  (`scripts/generate-catalog.mjs`) — so it contains no float literal, and the generator emits none.
  `npm run generate-catalog` produces no diff.
