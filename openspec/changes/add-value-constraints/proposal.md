## Why

A number's allowed range lives today in a doc comment that nothing reads. The checker accepts
`opacity=1.5`, the Visual view's steppers run past 1, the JSON Schema a model reads says only
`"number"`, and a function that cares tests its argument by hand. Putting the range in the type
makes it documentation that cannot drift, lets the tooling catch a wrong value where it is written
and offer the right range in the Visual editor without a visualizer, and lets a function rely on
its arguments without checking them.

## What Changes

- Add constrained types: a type followed by `where` and a value pattern with no operand, such as
  `ratio: float32 where in 0.0..=1.0`, `count: int where >= 1`, or a named
  `type Ratio = float32 where in 0.0..=1.0`. Allowed anywhere a type is written except type
  arguments and function types; on a sequence, the item type is named (`type Score = int where >= 0`,
  then `Score+`).
- Numeric value patterns: `in a..b`, `in a..=b`, and `<`, `<=`, `>`, `>=` against a literal,
  joined by `&&`. Other patterns are rejected by name. String length and
  pattern, sequence counts, named-constant bounds and general predicates are planned follow-ups
  that reuse the syntax.
- The same numeric patterns work in match arms, `if x is { in 0.0..1.0 => … > 1.0 => … }`, with
  the existing comma for alternatives. An arm narrows its scrutinee to the matched interval. An
  unbraced arm body no longer swallows a following arm that starts with `<`, `>` or `-`.
- A constrained type is its base type plus a check: values, encoding and arithmetic are the base
  type's. Constrained types are structural, satisfy their base freely, and satisfy each other by
  interval containment.
- The type checker rejects a literal, constant expression or default outside its site's range
  (`constraint-violation`), a malformed or unsupported constraint (`invalid-constraint`) and one
  that admits no value (`empty-constraint`).
- Any other value is checked when the program runs, where it enters a constrained site: an
  argument, a property, a field, a `let`, a returned value or a sequence item. The emitter inserts
  the check; the author writes none.
- NX IR gains a constrained type kind, a check node, an interval pattern node and the required feature `constraints-v1`.
  Both IR runtimes enforce constraints identically: `nx-ir-constraint` inside a program,
  `nx-ir-boundary-constraint` naming the argument for a host value.
- Declaration schema export writes `minimum`, `maximum`, `exclusiveMinimum` and
  `exclusiveMaximum`.
- The source tree carries each property's bounds, and the Visual number editor keeps within them
  with no visualizer; `@nx/visualizer` `min`/`max` hints may narrow the range offered but not widen
  it.
- Hover and signature help show constraints. The types reference, grammar documents and style
  guide document the feature.

## Capabilities

### New Capabilities

- `value-constraints`: the `where` syntax, numeric value patterns in `where` and in match arms, constrained types in the type
  system, compile-time checking and its diagnostics, where runtime checks happen, and hover.

### Modified Capabilities

- `nx-ir-format`: the constrained type kind, the check node and the `constraints-v1` feature.
- `rust-ir-runtime`: enforce constraints in programs and at the host boundary.
- `typescript-ir-runtime`: the same, with the same results.
- `declaration-schema-export`: map constraints to JSON Schema bound keywords.
- `source-tree`: properties and aliases carry their bounds.
- `viewer`: a number's editor keeps within its constraint.

## Impact

Grammar (`crates/nx-syntax`), HIR lowering and type references (`nx-hir`), the type checker and
widening rules (`nx-types`), IR model, image, reader and emitter (`nx-ir`, `nx-codegen`), both IR
runtimes (`nx-ir-runtime`, `runtime/typescript`), schema export (`nx-api`), the language service
and protocol (`nx-language-service`, `packages/language-protocol`), the viewer
(`packages/viewer`), syntax highlighting queries, the conformance corpus, and the website's types
reference, style guide and `nx-grammar.md`.

No value encoding changes and no existing program changes meaning; `where` becomes a keyword only
after a type. The interpreter gets the checker's errors but no runtime checks.
Generated C# and TypeScript host types keep the base type.
