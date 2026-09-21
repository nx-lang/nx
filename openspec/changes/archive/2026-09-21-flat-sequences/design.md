## Context

See proposal.md for motivation. The facts that shape the approach:

- Nesting is admitted structurally everywhere. The grammar's `type` rule repeats `[]`, post-parse
  validation rejects only a duplicate `?`, `TypeRef::Array` and `Type::Array` wrap without limit,
  and both type generators recurse.
- Three constructs produce a nested sequence: an explicit `T[][]`, a `for` whose body is list-typed
  (`infer.rs` wraps the body type in one more list; the interpreter, the IR runtime and the emitted
  JS all `map`), and a generic instantiated with a list argument (`<Page T=int[]/>`, which the
  record-type-parameters spec and `examples/nx/generic-records.nx` exercise on purpose).
- Two collecting positions already disagree. Body content splices one level in the checker
  (`normalized_sequence_type`), the interpreter (`eval_content_expressions`) and the IR runtime
  (`spliceContent`), but not in the emitted-JS backend (`emit_content_value`). A multi-item braced
  value splices nowhere.
- The checker's scalar-to-list lift is one level; the interpreter's `coerce_value_to_resolved_type`
  and the IR runtime's `normalizeValue` recurse.
- An `if` with no `else` infers `Type::void()` (`infer.rs` ~572), as does a block with no trailing
  expression (~697) and the uncovered path of a match (~1324, ~1327). The join of `void` with
  anything is `object`, which is why `{<A/>{if c { <A/> }}}` at `A[]` reports `found object[]`. At
  runtime `eval_if` returns `Value::Null`, the emitter writes `(c ? … : null)`, and the IR runtime's
  `nodeKinds.if` returns `null`; the content splice then keeps that null as an item.
- Nullable narrowing is already enforced: `let s:string? = null` then `let a:string = {s}` is
  rejected statically with `expects string, found string?`. The value-position rule below needs
  exactly that and nothing more.
- A written null item is legal: `let xs:string?[] = { "a" null }`. So a runtime cannot recognise an
  untaken conditional by the null it used to produce.

## Goals / Non-Goals

**Goals:**
- One function, in the checker and in each runtime, that says what an item contributes to the
  sequence it sits in: its elements if it is a sequence, nothing if it is an untaken conditional,
  itself otherwise. Applied identically by the checker, the interpreter, the IR runtime and the
  emitted JS.
- Nested sequence types unspellable, so the runtime invariant "a sequence value never holds a
  sequence value" is guaranteed by the type system rather than by discipline.
- No reachable producer of the unit type, and no diagnostic that renders one as `void`.

**Non-Goals:**
- Changing the `T[]` spelling, or adding `T+`, `{T}` or any other sequence syntax.
- Adding a map, array or tuple type for nested external data. The docs say a record is how data
  nests, and that a distinct type is the future answer for JSON interop.
- Removing the unused `Expr::Index` and `SEQUENCE_EXPRESSION` remnants; they are dead already.
- Changing what `object` may hold. An `object` value may be a sequence; it is opaque as an item and
  splices as a value, the one place the static and dynamic views of a sequence differ.
- Nullable narrowing beyond what exists, better non-exhaustive-match messages, `if` as a statement.

## Decisions

### Reject nested suffixes in the post-parse validator, not the grammar
`validate_type_suffix_chain` already walks a suffix chain and rejects a second `?` on one layer. A
second `[]` is the same shape of rule, gets a span on the offending token, and leaves the grammar's
`repeat` alone so error recovery and highlighting are unchanged. Parentheses and function results
are part of the chain: `(string[])[]` and `<function />: string[][]` are rejected by carrying
"already a sequence" across them, as the nullable rule carries "already nullable".

### Reject alias-based nesting at type resolution
The validator cannot see that `Names` in `Names[]` is an alias for `string[]`. Resolution in
`semantics.rs` is where the inner type is known, so it reports there, naming the alias, with the
same wording as the validator so a reader meets one rule.

### Type arguments are item types, checked once at resolution
Both argument paths, an applied type in an annotation and a bare-name argument at a construction or
component use site, resolve to a `Type` before substitution. One check on that resolved type covers
`T=int[]`, `T=Ints` and `TItem=Names`. Alternative considered and rejected: allow a sequence
argument and reject only an instantiation whose substituted shape nests, which makes the rule a
property of the pair and produces diagnostics about fields the author did not write. Nullable
arguments stay legal; `int?` is an item type.

### One contribution rule, and the conditional is not a case of it
The checker gets a single rule for what an item expression in a collecting position contributes to
the item type: the element type if the item's type is a sequence, `object` if it is `object`, and
the item's type otherwise. `Expr::Array` inference, `for` body inference and the content-property
check all use it, replacing `normalized_sequence_type` and the per-site handling. It reads only the
item's own type.

An `if` or match with no `else` is not a case of that rule. It is read as having an implicit
`else { }`, so its type is the join of its branches with `never[]`, and the join lifts an item to a
sequence: `join(T, U[])` is `join(T, U)[]` when exactly one side is sequence-shaped. A branch
producing `T` therefore gives `T[]`, a branch that is already a sequence gives that sequence back,
and a conditional nested in one needs nothing, because a sequence never contains a sequence. Its
value is the empty sequence when no branch is taken. The rule governs the expression, not its
position, so the conditional has one type and one value wherever it is written.

Each runtime needs nothing conditional-specific either. `eval_if` and `eval_match` return an empty
`Value::Array` for a missing branch, the IR runtime's `if` node returns `[]`, and the emitter writes a
missing `else` as `[]`. Every collecting position then evaluates its items through one helper that
extends with a sequence value's elements and pushes anything else. Because an untaken conditional
evaluates to an empty sequence rather than to null, nothing has to inspect the source to tell it
from a written `null`, which stays an item in a `string?[]`.

**Revised during review.** The first implementation gave the conditional two notions: a nullable
*type* for value positions, and a separate *contribution* — the join of its branches' contributions,
or nothing — for collecting positions, recorded in a side table and recognised by each runtime from
the source node. That duality was the cause of four review findings (RF1, RF2, RF7, RF8), each an
engine computing the contribution differently from the others, and a nested conditional, an arity-one
braced value and a closed conditional with an empty arm each fell through a different gap. Reading
the missing branch as `{}` removes the second notion entirely. The cost is that a nullable site
needs the `else` written: `if c { "x" } else { null }`.

Alternative considered: keep the conditional typed `T?` and add an implicit `T? → T[]` conversion.
Rejected because today's `T?` means "one value, possibly null", which collides with "zero or one
items" at `T?[]`: unannotated, `{ "a" if c { "b" } }` would infer `string?[]` and hold a null item.
That alternative becomes correct once `?` is an occurrence indicator and `null` is gone, which is
recorded in `specs/future.md`; under it the `T[]` given here narrows to `T?` with no change in
behaviour.

Alternative considered: have the IR mark which items are statically sequence-typed and splice only
those, keeping the static and dynamic counts aligned for `object`-typed items. Rejected because it
lets a nested sequence reach the value model through `object`, which is what this change rules out.
The count mismatch it avoids is unobservable to the type system.

### `for` yields a sequence of the body's item contribution
`For` inference becomes `Type::array(contribution(body))`. `never[]` as a body gives `never[]`, so
`let xs:string[] = {for y in ys {}}` is accepted and `let a = {for y in ys {}}` still asks for an
annotation. `for n in ns { if (n % 2 == 0) { n } }` is the canonical filter and now types as `int[]`.

### Retire the unit type
With the absent `else` and the uncovered match path retyped, the block-without-trailing-expression
site is the last constructor of `Type::void()`. The implementation checks whether that block form is
reachable from parsed source. If it is not, `Primitive::Void`, `Type::void()` and the `"void"`
rendering are removed, and the `primitive-type-names` requirement that described the unit type is
removed with them. If it is, the rendering becomes a string that is not a legal identifier so no user
declaration can collide with it. Either way no diagnostic can say `found void`.

### Simplify the runtime lift to one level
With no nested sequence type reachable, the recursive arm of `coerce_value_to_resolved_type` and
the elementwise recursion in `normalizeValue`'s `array` case only ever recurse into item types.
They become a single wrap-then-coerce-elements step, which also retires the checker/runtime
divergence noted in Context.

### Docs describe the model, not the mechanism
The sequences reference page is rewritten around three sentences: an item is a sequence of one, a
sequence never contains a sequence, an item in a collecting position contributes its items (and an
untaken conditional contributes none). It shows braced literals only, the filter idiom, a record as
how data nests, and drops the bracket-literal and tuple examples that have never parsed.

## Risks / Trade-offs

- [Source that relied on `for` nesting changes shape silently rather than failing] → The type
  changes from `T[][]` to `T[]`, so any annotation or binding that named the nested type fails
  loudly. Where it was never named the flattened value is what the content model always meant. The
  task list sweeps examples, playground snippets and the fiddle catalogue.
- [Corpus output changes wherever a conditional child is false] → Deliberate. The corpus diff is
  reviewed as part of verification and each changed file should lose exactly its null items.
- [External data with nested arrays cannot be declared] → Stated in the docs as a known limit with a
  record as the workaround and a distinct array or map type as the future answer, following
  XPath 3.1, which added one for JSON for the same reason.
- [`object` holding a sequence is the one hole in "flat"] → Accepted and documented. It cannot
  produce a nested sequence value, only a count the type system does not observe.
- [Two diagnostics for one nesting rule (validator and resolver)] → Same wording, both covered by
  scenarios.
- [Removing `Primitive::Void` touches a public enum] → It is used by the checker, the language
  service's rendering and the generators' unreachable arms; the compiler finds every use. If the
  block form proves reachable the variant stays with a safe rendering, which is the smaller change.

## Migration Plan

Land in three steps inside the one change so each is revertible on its own: the collecting-position
work (suffix rejection, type arguments, splicing, untaken conditionals contribute nothing), then the
conditional's own rule, then the unit type's removal once nothing constructs it. The second step
was first landed as a value-position nullable rule and replaced during review by the implicit
`else { }` (task section 11). No data or deployment migration; consumers of generated types see no
change except the absence of nested arrays, which no shipped consumer uses.

Source that relied on the nullable reading moves to an explicit `else { null }`. No `.nx` file in the
repository does: a checker instrumented to report every branch the join lifts — implicit `{}`
fills and closed conditionals mixing an item arm with a sequence arm alike — found none across the
39 corpus files, within the limit that a file failing analysis is covered only as far as inference
got. The only conditional without an `else` arm that either sweep found, the
`if loadState is { … }` at `examples/nx/types.nx:80`, covers every case of its union, so it has no
uncovered path to fill and keeps its scalar type (`an_exhaustive_union_match_is_not_made_a_sequence`).
The two-source corpus diff is byte-identical.
