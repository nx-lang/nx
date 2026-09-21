## Why

NX sequences are meant to be the XML content model: items written side by side, a single item
indistinguishable from a sequence of one, no way for a sequence to contain another sequence, and a
child that does not fire contributing nothing. The toolchain does not say so, and in places it says
the opposite.

**Nested sequences are a type you can name but only construct by accident.** The grammar accepts
`T[][]` to any depth, the checker's `Type::Array` documents `string[][]` as an example, a `for`
whose body is list-typed produces a list of lists, a generic instantiated with a list argument
produces a nested field, and a multi-item braced value with list-typed items nests them where body
content in the same position splices them. The documentation site shows `int[][] = [[1, 2], [3, 4]]`,
which does not parse, and calls nested sequences "straightforward". The interpreter, the IR runtime
and the emitted-JS backend disagree about what a nested value contains.

**A conditional child that does not fire contributes a `null` item.**

```nx
type A = { n:int = 1 }
type Box = { content items:A[] }
let c = false
let root() = { <Box><A/>{if c { <A/> }}</Box> }
```

Type checking rejects this today with `expects A[], found object[]`, because the untaken branch has
the internal unit type and the join climbs to `object`. Where the check is not in the way, the
interpreter and the emitted JS both produce `[ {A}, null ]`: `emit.rs` writes `(c ? … : null)` as an
array element. In a value position the same form evaluates to `null` but is typed `void`, so the one
annotation that matches what happens, `v:string?`, is rejected with `expects string?, found void`.
And since `empty-list-spelling` freed `void` for user declarations, `expects void, found void` is
reachable in three lines.

These are one problem. An untaken conditional is an item whose contribution is empty, exactly as a
sequence-valued item's contribution is its elements, and both need the same rule about what an item
contributes to the sequence it sits in. Committing to that rule now, while `T[]` stays the spelling,
removes a class of divergent behaviour, fixes the null-item defect, gives type arguments and `for` a
single rule, and retires the unit type's last producers.

## What Changes

- **A sequence is flat.** A sequence never contains a sequence. A single item is a sequence of one
  wherever a sequence is expected; a sequence appearing where items are being collected contributes
  its items. Stated as a language rule rather than left to fall out of individual sites.
- **BREAKING** `T[][]`, `T[]?[]` and `(T[])[]` are rejected at parse validation, the way `T??` is
  today: at most one `[]` per type reference chain, including across parentheses. An alias that
  names a sequence type is rejected as the base of a further `[]`.
- **BREAKING** A type argument must be an item type. `<Box T=int[]/>` in an annotation, and a
  use-site argument that resolves through an alias to a sequence type (`<Box T=Ints/>`,
  `<List TItem=Ints/>`), are rejected.
- **BREAKING** A `for` concatenates what its body yields instead of nesting it. A body that yields a
  sequence contributes that sequence's items, a body that yields nothing contributes nothing, and
  the result type is a sequence of the body's item type. `let xs:string[][] = {for y in ys {}}` is
  rejected; `let xs:string[] = {for y in ys {}}` is accepted.
- **BREAKING** Items of a multi-item braced value splice. `{xs ys}` with two `string[]` values is a
  `string[]` holding both sequences' items, and `{xs "a"}` is a `string[]`, where today the first
  infers `string[][]` and the second `object[]` holding a nested array. Braced values, call
  arguments, `if` and `for` results and body content all follow the rule body content follows now.
- **BREAKING** **A conditional with no `else` carries an implicit `else { }`.** When no branch is
  taken it evaluates to the empty sequence, and its type is the join of its branches with `{}` — a
  `T[]` where the branches produce `T`. So in a collecting position it contributes nothing by the
  splice rule above, with no rule of its own: it never contributes `null`, never widens the item
  type, and a body that is only an untaken conditional binds the empty list. The `for` body is a
  collecting position, which is what makes `for n in ns { if (n % 2 == 0) { n } }` a filter.
- **BREAKING** **The join lifts an item to a sequence.** `join(T, U[])` is `join(T, U)[]`, because an
  item is a sequence of one. This is what types the implicit `{}`, and it means an empty arm written
  out — `if c { <A/> } else { }` — is an `A[]` rather than joining to `object`.
- **BREAKING** **A nullable value needs its `else` written.** `let v:string? = {if c { "x" }}` is
  rejected, naming `string[]`; `{if c { "x" } else { null }}` is the nullable form. This replaces the
  nullable reading an else-less conditional had, which was revised during review: a type and a
  separate notion of what the conditional contributes proved to be the cause of four review
  findings, and one rule removes both. See the review's "Rule Change" section.
- **A non-exhaustive union match stops reporting `object`.** The uncovered path follows the absent
  `else` rule; the exhaustiveness error itself is unchanged.
- **The unit type loses its reachable producers.** The absent-`else`, uncovered-match and empty-block
  sites are retyped or retired. If nothing constructs it, `Primitive::Void` and its `void` rendering
  are removed; if the block case is reachable from source, it keeps a rendering that is not a legal
  identifier. No diagnostic names an inferred type `void`.
- The interpreter, the TypeScript IR runtime and the emitted-JS backend splice on the same terms.
  Today the emitted-JS backend does not splice body content at all.
- The type generators no longer have nested-list cases to preserve; the composed suffix requirement
  keeps `T?[]` and `T[]?` distinct and drops `T[][]`.
- The runtime scalar-to-list coercion in the interpreter and the IR runtime, which lifts recursively
  today, is reduced to the single level the type system now guarantees.
- Documentation, grammar prose, crate docs, examples and the sequences reference page are rewritten
  to describe flat sequences with braced literals, to name a record as the way to nest data, and to
  drop the bracket-literal and tuple examples that never parsed.

Out of scope, and unchanged: the `T[]` spelling, `T?[]` versus `T[]?`, the scalar-to-list coercion
at typed sites, the `never[]` empty list, exhaustiveness of union matches, nullable narrowing in
general (the checker already rejects a `string?` at a `string` site, which is all the value-position
rule needs), and `if` as a statement.

## Capabilities

### New Capabilities
- `sequence-model`: The flat sequence model. An item is a sequence of one; a sequence never contains
  a sequence; a sequence-valued item in a collecting position contributes its items; the rules every
  collecting position, every runtime and every code generation target follow.
- `conditional-result-types`: What an `if` or `if … is` with no `else` produces when no branch is
  taken, in a collecting position and in a value position, in the checker and in every runtime, and
  the rule that no diagnostic names the unit type.

### Modified Capabilities
- `type-reference-suffixes`: Composed suffixes admit at most one `[]` per type reference chain.
- `braced-value-sequences`: The arity-to-type rule states what a sequence-typed item contributes to
  a multi-item braced value, and what a `for` yields.
- `record-type-parameters`: An applied type's argument must be an item type.
- `component-type-parameters`: A use-site type argument that resolves to a sequence type is
  rejected.
- `cli-code-generation`: The composed-suffix generation requirement preserves `T?[]` versus `T[]?`
  and no longer specifies nested-list output.
- `primitive-type-names`: Removes "The unit type is inference-internal and has no source spelling";
  every clause of it is superseded by `conditional-result-types`. The guarantee that `void` is not a
  primitive in type position lives in a different requirement and is unaffected.

## Impact

- `crates/nx-syntax`: `validate_type_suffixes` gains a repeated-`[]` rule; the grammar's `type`
  rule keeps its `repeat`; parser tests that assert `string[][]`.
- `crates/nx-hir`: doc comments on `Expr::Array`, `Expr::Index` and `TypeRef`; `lower.rs` tests
  asserting `Array(Array(..))`.
- `crates/nx-types`: `Type::Array` docs; type-reference resolution rejects an alias-nested list;
  the item-contribution function shared by `Expr::Array`, `for`, body content and the conditional
  forms; `if` and match result types; applied-type and use-site type-argument checks;
  `common_supertype`'s nested-array arm; `Primitive::Void` and its rendering in `ty.rs`; tests in
  `empty_lists.rs`, `record_type_parameters.rs`, `component_type_parameters.rs`, `nullable_joins.rs`.
- `crates/nx-interpreter`: item evaluation in `Expr::Array`, `eval_for`/`eval_for_range` and
  `eval_content_expressions` splices and drops an untaken conditional; `coerce_value_to_resolved_type`
  lifts one level; ignored nested-array tests removed.
- `crates/nx-codegen` and `runtime/typescript`: `For`, `Array`, `emit_content_value` and the
  conditional-as-item emission; the IR runtime's `for`, `array`, content and `if` node kinds;
  `normalizeValue` lifts one level.
- `crates/nx-cli`: nested-list typegen tests in `typegen.rs` and `main.rs` removed; `format.rs`
  round-trip tests are the check that the formatter is untouched.
- Specs: the six modified capabilities above plus two new ones.
- Docs and examples: `docs/src/content/docs/reference/concepts/sequences-and-objects.md`,
  `language-tour/types.md`, `reference/syntax/types.md`, `nx-grammar.md`, `nx-grammar-spec.md`,
  `crates/nx-types/src/lib.rs`, `examples/nx/generic-records.nx`.
- Downstream: NX source relying on a `for` body of lists producing a list of lists, or on `T[][]`,
  stops compiling or changes shape. Any `.nx` file in the corpus with a conditional child whose
  condition is false today produces a `null` item and will produce none; the corpus diff is the
  evidence the change does the right thing. No shipped example or playground snippet is known to
  rely on nesting; the task list includes a sweep to confirm.
