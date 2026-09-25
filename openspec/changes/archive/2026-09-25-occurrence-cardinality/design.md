## Context

See proposal.md for motivation. The facts that shape the approach:

- **Two wrappers where one is wanted.** `TypeRef::Array`/`TypeRef::Nullable` in `nx-hir` and
  `Type::Array`/`Type::Nullable` in `nx-types` compose in source order; `validate_type_suffix_chain`
  (`crates/nx-syntax/src/validation.rs`) enforces at most one `[]` and no repeated `?` per layer.
  The IR mirrors them as the `array` and `nullable` type kinds (`crates/nx-codegen/src/ir.rs`,
  schema 4), and both type generators recurse through them.
- **`null` is a value with no operators.** `Literal::Null`, `Value::Null` (48 sites outside tests),
  `NxValue::Null`, `SerializedValue::Null`, the IR `null` node kind and the runtime's `null` handling
  exist, but nothing branches on null: there is no `?.`, `??`, `== null` or `is null`, and the only
  test is the `null =>` match pattern. `eval_if` with no `else` already returns the empty array, not
  null, since `flat-sequences`.
- **The join already has the shape.** `common_supertype` (`crates/nx-types/src/semantics.rs` and
  the inference context's override) lifts an item to a sequence and treats the bottom type as the
  identity; `item_contribution` (`infer.rs`) is the one place a collecting position asks what an
  item adds. `implicit-primitive-conversions` lifts nullability out of the join separately. Those
  are two rules for what will be one occurrence.
- **Narrowing exists for one shape.** A match arm whose scrutinee is a local identifier rebinds it
  in a pushed scope (`infer.rs` ~1352). Nothing narrows a member path, and there is no boolean
  condition that narrows anything.
- **The ternary is parsed and lowered, never specified.** `conditional_expression` in `grammar.js`
  lowers to `Expr::If` (`lower.rs` ~1631); three example files use it; only the highlighting spec
  mentions it, as a contrast case.
- **Absence already has two levels.** `update-records` distinguishes "key absent" from "key present
  with null" and carries it by key presence; canonical JSON omits absent keys. Everywhere else an
  absent nullable field is emitted as `null`.
- **Record values do not carry field types.** `nx-value`'s serializer is value-directed: it emits
  what is stored, with `$type` as the only type information. The .NET and Node SDKs and the
  TypeScript IR runtime each re-implement the canonical encoding.
- **`object` may hold a sequence.** `sequence-model` makes that value opaque as an item and
  splicing as a value — already the one place static and dynamic views of a sequence differ.

## Goals / Non-Goals

**Goals:**
- One representation of "how many" in every layer — grammar, HIR, checker, IR, runtimes, typegen —
  so that no layer can express an optional sequence or a sequence of optionals.
- One representation of "nothing" at runtime, the empty array, and at the host boundary the host's
  own spelling: an omitted key for a record field and `null` for an entry call's `T?` result, with
  every decoder accepting `null`, `[]` and a missing key alike.
- The join, the collecting positions and `for` computing occurrences from one four-point lattice,
  so `conditional-result-types`' "join of item and sequence" rule and `implicit-primitive-conversions`'
  "lift nullability out" rule become the same code.
- Narrowing that is cheap because values are immutable: a path-keyed map pushed for a branch and
  popped after, never invalidated.
- Three engines agreeing on the three operators and on lone-child content binding, pinned by the
  existing three-engine parity test.

**Non-Goals:**
- Flow typing beyond the enumerated forms (`p?`, `!p?`, `&&`, the `{}` arm). No narrowing through
  `||`, calls, `for` bodies, or `let` bindings of a test result.
- A general min/max cardinality (`T{2,5}`) or a mapping `?.` over sequences. The lattice has four
  points on purpose; the internal representation below can express more, but the language does not.
- Changing how `object` behaves beyond stating how `object?` meets an empty held sequence.
- Making `never` writable, renaming `Element`, or the string-escape questions in `future.md`.

## Decisions

### One `Seq` wrapper carrying an occurrence, with exactly-one as no wrapper
`Type::Array` and `Type::Nullable` become `Type::Seq { item: Box<Type>, occ: Occurrence }`, and
`TypeRef` and the IR follow. Exactly-one is the absence of the wrapper, so the invariant "an item
type is never a `Seq`" is a structural fact the constructor enforces, and every existing
`matches!(ty, Type::Array(_))` site becomes a check on the wrapper alone.

`Occurrence` is represented as two flags, `may_be_empty` and `may_be_many`, rather than an enum of
`?`/`+`/`*`. The three language values are the three non-`(false,false)` combinations, and every
lattice operation is a bit operation: join is `(a.empty | b.empty, a.many | b.many)`; the sum of
two items in a collecting position is `(a.empty & b.empty, true)`, since two items that may each be
present may be two, and the sum folds left over a braced value's items with a single item left
unchanged; the product in a `for` is `(a.empty | b.empty, a.many | b.many)`; `??` removes zero from
its left operand, `(false, a.many)`, then joins with the right. The empty type is
`Seq { item: never, occ: (true, false) }`; summing it with an item over-approximates to `+`, which is
sound and never reached from source except by writing `{ {} x }`. Rendering maps the three
combinations to `?`, `+`, `*`.
Alternative considered: an enum with a hand-written join table — nine cases each for join, sum and
product, easy to get one wrong. Alternative: `(min, max)` integers — more general than the language
and invites `T{2,5}` by accident.

### The property-slot rule is enforced at type resolution, in one place
`name:T?` and `name:T*` are rejected where the property's type reference is resolved in `nx-types`,
because that is the only place an alias to a `?`/`*` type is visible. The diagnostic carries the
`name?:T` fix-it. The grammar and `validate_type_suffixes` stay permissive about which suffix
appears in which slot so that the message is the same whether the author wrote `string?` or
`Maybe`. Alternative: reject the spelled case in `validation.rs` for an earlier error — rejected
because it produces two diagnostics with two wordings for one rule.

The optional mark lives on the property definition (`PropertyDefinition.optional`) in HIR, not on
its type. Reading a property computes `T?` or `T*` from the mark and the declared occurrence at the
member-access site, so `T.Update` and `T.Property` derivation see the mark and the field's type
separately — which is what the clearable rule needs.

### The empty value is the empty array, and an empty optional field is not stored
`Value::Null`, `NxValue::Null` and `SerializedValue::Null` are removed. The empty value is
`Value::Array(vec![])` in the interpreter and `[]` in both JavaScript engines. A `?` value that
holds an item is the item itself, not a one-element array, matching the canonical encoding; a `+` or
`*` value is always an array. Coercion at a typed site normalizes: `[x]` at a `?` site becomes `x`,
`x` at a `+`/`*` site becomes `[x]`, `[]` is left alone, and `[]` at a `+` or exactly-one site is an
error.

Record construction stores no entry for an optional field whose value is empty. A member read of a
declared-optional field that is not stored yields the empty value. This makes the canonical encoding
value-directed — the serializer emits what is stored, so an empty optional field is an omitted key
with no type information needed — and it leaves `nx-value`, the .NET SDK, the Node SDK and the IR
runtime's encoders as they are structurally. An update record is the one exception: a present empty
field *is* stored (as `[]`) so that key presence carries "cleared", and it is written as `null`,
recognised by the `.Update` suffix on `$type` exactly as the update-record encoding is recognised
today (in Rust, `to_nx_value` makes it `NxValue::Null`). Alternative considered: store `[]` and have the encoder consult field
declarations — rejected because four encoders would each need type information they do not carry.

Consequence to document rather than fix: a value of static type `object?` holding an empty
sequence reads as absent. `object` is already the one place a held sequence is opaque; `object?` is
where that meets the empty value. `object` itself stays exactly-one: `{}` at an `object` site is
rejected like `{}` at an `int` site, and so is a `+` or `*` value, since neither satisfies an
exactly-one occurrence. That retires the old "every list satisfies `object`" rule — from NX source
a sequence reaches an `object` slot only through a host, which is what keeps `object`'s held
sequence opaque rather than a second sequence representation. A join with no common item type
follows the lattice too: `string?` joined with `float64` is `object?`, not `object`.

### Narrowing is a path-keyed map scoped to a branch
The inference environment gains a narrowing map from a *path* — an identifier followed by member
names — to a type, pushed when a branch is entered under a narrowing condition and popped after.
`Expr::Identifier` and `Expr::Member` inference consult it after computing the unnarrowed type: if
the expression is a path with an entry, the entry wins. `Expr::OptionalMember` on a narrowed
receiver reports the unnecessary `?.`.

Conditions are decomposed syntactically: `Exists(path)` narrows `path` in the then-branch;
`Not(Exists(path))` narrows it in the else-branch; `And(a, b)` unions the narrowings of both sides
for the then-branch; anything else narrows nothing. A tested chain `a?.b?` produces entries for `a`
and `a.b`. The `{}` match arm reuses the same map for the `else` arm and the arms after it. Because
values are immutable there is no invalidation; the map is popped with the branch. Alternative
considered: a binding form (`if let`) — rejected in the proposal; it does not help chains.
Alternative: rebinding the root identifier to a narrowed record type — impossible, records are
nominal.

### The three operators are HIR expressions and IR node kinds, not desugarings
`Expr::Exists`, `Expr::OptionalMember` and `Expr::Coalesce` are lowered as themselves, typed by the
rules in `presence-operators`, and emitted as the IR node kinds `exists`, `optionalMember` and
`coalesce`. Desugaring `x?.m` to `if x? { x.m }` would evaluate `x` twice or need a binding form the
language does not have; desugaring `??` to an `if` would lose the "left operand cannot be empty"
diagnostic's anchor. Each engine implements the three directly on the empty-array representation:
`exists` is non-emptiness, `coalesce` evaluates its right operand only on empty, `optionalMember`
returns `[]` on empty and member access otherwise.

### Grammar: `??` binds above arithmetic; postfix `?` and `?.` are member-level; the ternary goes
`conditional_expression` is removed. `??` is a right-associative binary operator at precedence 125,
above `*`/`/`/`%` (120) and below prefix unary, so `"a" + x ?? "b"` is `"a" + (x ?? "b")` and
`-x ?? 1` is `(-x) ?? 1` (which is then a type error, correctly). Postfix `?` and `?.` are postfix
forms at member-access precedence. `?.` and `??` are single tokens, so the lexer's longest match
keeps `x?.m` from lexing as a presence test followed by `.`; the formatter never emits `x? .m`. The
type rule's `repeat(choice('?', '[]'))` becomes `optional(choice('?', '+', '*'))`, and
`validate_type_suffixes` keeps only the "one suffix per chain, across parentheses and function
results" check. `[]` stays in the grammar as a rejected token so its diagnostic can name `*` and `+`.
`{}` becomes a `pattern` alternative; it lowers to the empty `Expr::Array` the checker already
knows.

### IR schema 5 with a `seq` type kind, and `occurrence-v1` for the operators
The `array` and `nullable` type kinds and the `null` node kind keep their numbers and are never
emitted. A new `seq` type kind carries `[SEQ, item, occ]`. The three operator node kinds get new
numbers. A module that uses any of them declares `occurrence-v1` in its required features, mirroring
`ranges-v1`, so a runtime that predates them refuses by name rather than failing on an unknown node.
The IR runtime's `normalizeValue` gains the `?`-site un-lift and the `+`-site non-empty check.

### Typegen spells occurrences as the host language does, with `+` checked at the boundary
C#: `T?` → nullable `T`; `p?:T` → a nullable property; `T+` and `T*` → the existing list mapping.
TypeScript: `p?:T` → `p?: T`; `T?` elsewhere → `T | null`; `T+`/`T*` → `T[]`. Update companions keep
`p?: T | null` for a clearable field and `p?: T` for a non-clearable one, and C# keeps the
unset-versus-null API with "null" meaning cleared. `+` has no static expression in either host and
is validated when a value crosses into NX. Every decoder accepts `null`, `[]` and a missing key as
the empty value at a `?`/`*` site, so a host that sends `null` for an optional field is unaffected
and a host that reads an absent field reads a missing key.

### An entry call returns an empty `T?` result as `null`
A standalone `T?` value reaches a host in exactly one way: as the result of an entry call. A record
field that is empty is omitted, and `T?` is never an array item, a type argument or a field type.
So the encoders stay value-directed, and the entry-call path alone chooses the host's spelling.
When the function's result type, declared or inferred, is `T?` and the value is empty, the result
is `null` (MessagePack `nil`). Typegen's `T | null` and C#'s `T?` then describe what the host
receives, and `NxRuntime.Evaluate<string?>` reads it. `nx-api`'s `entry_result_to_nx_value` does
this for the interpreter, with the result type taken from the checker's binding. The IR runtime's
`evaluateFunction` and `callFunction` do it from the optional-result flag on the IR function
declaration. `NxValue::Null` is the host's `null`: a decoded host `null` is `Null`, which NX reads
as the empty value, and a cleared update-record field converts to it too, so the serializer no
longer needs to recognize `.Update`.

The same declaration also carries its declared result type, and a value its declared type. Each
runtime normalizes the result to that type as the interpreter coerces it. Normalization covers
declared types only, because the interpreter coerces only there.

Inside the program the empty value is always `[]`. A generated JavaScript function is typed NX
code, not a host boundary: it returns `nxEmpty` for an empty `T?`, as its `T | NxEmpty` signature
says, since a `null` there would reach internal lifts such as `[v].flat()`. Alternatives
considered:
- Encode an empty top-level value as `null` in every encoder. That is the type-directed encoder
  rejected above.
- Represent the empty `?` as `null` inside the runtimes. Every emptiness check (splice, `for`,
  `??`, the `{}` arm, equality, record construction) would have to handle two representations.

### Content binding: one lone-child rule
A content property declared `+` or `*` always binds an array — one child binds a one-element array;
a content property declared `?` or exactly-one binds the child itself. This is the same normalization
`coerce_value_to_resolved_type` applies to any typed site, applied to content in all three engines,
which closes RF28 of the `flat-sequences` review: with `T?[]` and `T[]?` gone, the checker's lone-
child and many-child paths compute the same type, so the engines only have to agree on the value.

### Corpus migration is by hand, `+` by default for props lists
No script can decide `+` versus `*`. The sweep over 45 `.nx` files, 6 reference pages, the two
grammar documents, `specs/ir-conformance`, the playground examples and the DrawnUI catalog
generators rewrites `T[]` as `T+` unless the text says the sequence may be empty, `x:T?` as `x?:T`,
`null` as `{}` or omission, the `null =>` arm as `{} =>`, and each ternary as `if/else`. The
whole-corpus before/after diff of diagnostics and canonical output, as in `flat-sequences`, is how
the sweep is checked: every file that parsed must parse, and every canonical output must be
identical except for the encoding of absence.

## Risks / Trade-offs

- [Tree-sitter conflicts from postfix `?` in expression position, and `?` in a type position after
  a name] → Type and expression are separate parser states; `let x:T? = e` and `x? && y` do not
  share one. Confirm with `tree-sitter generate` conflict output before lowering anything; if a
  conflict appears in property-list fragments (`<A b=x? />` is not admitted — unbraced values are
  literals and names), it is handled by requiring braces there, which is the existing rule.
- [`object?` reads an empty held sequence as absent] → Documented in `sequence-model`; `object`
  stays exactly-one; a `?` test on a plain `object` receiver is a diagnostic, so the ambiguity is
  reachable only through an `object?` slot fed by a host.
- [Hosts that send `[]` or `null` for a field that is now `+`] → Construction fails with a named
  field, where before it silently bound an empty list. This is the intended tightening; the
  catalog generators pick `?:T+` for optional TS array props so an empty host array is absence, not
  an error.
- [Every `.nx` file in the corpus changes, so the test suite alone cannot show the sweep is right]
  → The corpus diff, with a baseline binary from `main`, is a task with an acceptance criterion:
  identical canonical output modulo absence encoding, and no new diagnostics except the intended
  ones.
- [Narrowing through paths grows into flow typing] → The four forms are enumerated in the spec and
  the map is popped with the branch. Anything not on the list narrows nothing, by construction.
- [`??` above arithmetic surprises a C# or Swift reader] → The low-binding parse is a type error in
  NX in every case, so the surprise can only be a diagnostic pointing at the operand, never a wrong
  value. The reference notes the difference.
- [A generated JavaScript function returns `nxEmpty` for an empty `T?` where an entry call through
  a runtime returns `null`] → The generated function's own TypeScript signature says `T | NxEmpty`,
  so a caller is not misled; a host that serializes a generated result reads `[]`, which every
  decoder accepts.
- [Schema bump breaks fiddle shares carrying schema-4 IR] → The fiddle re-pins to the new npm
  release and regenerates; existing shares are re-compiled on open, which the fiddle already does
  when its runtime version changes.

## Migration Plan

One branch, landing as a minor version (`v0.4.0`) with the release order in `docs/deployment.md`.
Order of work is the task list's: grammar and HIR first so every later layer compiles against the
new shapes; checker; interpreter; IR and the two JavaScript engines with the parity test; typegen
and SDKs; formatter, TextMate and language service; then the corpus and docs sweep with the
before/after diff; then `specs/future.md`. Rollback is reverting the branch; there is no data
migration, since canonical JSON produced before the change decodes under the new rules (`null` is
accepted everywhere the empty value is).

## Open Questions

- Whether `T*` should ever be admitted in a property slot as a readability affordance
  (`tags:string*` versus `tags?:string+`). The spec says no; the corpus sweep will show how often
  authors reach for it, and re-admitting it later is additive.
- Whether hover should render the type of `{}` as `{}` or as the expected type at the site when one
  is known. The spec pins diagnostics only.
