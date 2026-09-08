## Why

Two parts of NX's source surface are out of step with its type system, in opposite directions: a
value that exists cannot be written, and a type that can be written says nothing an author ever
needs to say. This change closes both gaps.

### An empty list has no spelling

`items={}` is a syntax error — the parser recovers by inserting a zero-width `MISSING identifier` —
and `braced-value-sequences` specifies `{ ... }` as "a single `ValueExpression` or a space-delimited
sequence of **one or more** `ValueListItemExpression` entries", so zero entries is outside the
grammar by construction.

This is reachable. A list-typed property whose value is empty is an ordinary runtime value: a
component that filters a list down to nothing, a record whose list field defaults to empty, a host
that passes `[]` across the FFI boundary. The value exists and is well typed; what is missing is a
way to write it down.

Until now the gap was hidden. The value formatter classified an empty list as a *simple* value and
emitted it in attribute position as `items="..."` — a quoted string, which at a list-typed site is a
type error rather than the empty list it came from. `replace-enums-with-unions` removed that output
(its design D7 and task 6.4): `format_value` now fails explicitly rather than emitting something
that cannot be read back. So the gap is now loud instead of silent, and this change is what closes
it.

### A function cannot be passed a list

A property binding takes a list — `items={"a" "b"}` — but a call argument does not.
`values_braced_expression` is neither a `value_expression` nor a `value_list_item_expression`, and
`call_expression` takes `value_expression`, so `f({"a" "b"})` is a syntax error. The two sites are
otherwise the same kind of site: each has a declared type, each already coerces a scalar to the list
that type declares. Only one of them can be written to.

Giving the empty list a spelling makes the asymmetry harder to live with. `f({})` is the obvious way
to pass an empty list and it does not parse, and the workaround — bind it to an annotated `let`
first — exists only because the argument position cannot supply the type an annotation would.

### `void` is a spelling with nothing behind it

`void` is one of the nine names `primitive-type-names` fixes as the NX primitive set. It parses
(`primitive_type` in `crates/nx-syntax/grammar.js`), it resolves (`builtin_type` in
`crates/nx-types/src/semantics.rs`), it is offered as an editor completion, and code generation maps
it to C# and TypeScript `void`.

Nothing writes it. There is no `void` in any `.nx` file in the repository — not in `examples/`, not
in the DrawnUI corpus, not in a parser fixture, not in a documentation snippet. Nor is there much
for it to say: NX functions are expression-bodied (`let f(x:int): T = expr`), so a function always
produces a value, and NX has no `throw`, `panic`, or `raise` — no expression in the language fails
to produce one.

The internal type is real and is not the problem. `Type::Void` is what inference assigns to an `if`
with no `else`, to a block with no trailing expression, and to a match that may match nothing. Those
are genuine unit-typed results. They simply never need to be *named* by an author, and the language
is smaller if the name is not offered.

## What Changes

### The empty list is spelled `{}`

- `ValuesBracedExpression` admits **zero** entries, so `items={}`, `let xs:string[] = {}`, and
  `<List>{}</List>` parse.
- A zero-item braced expression is **list-valued**, unconditionally. This does not follow from the
  existing arity rule and is stated on its own: one item infers a *scalar* (`{1}` is `int`, not
  `int[]`), so zero is the one arity that cannot be scalar.
- Its element type is the **bottom type**, so `{}` is a `never[]`. That is its type outright rather
  than one a site supplies: `never` is below every type, so `never[]` is below every list type, and
  one value is usable at every list-typed site. It does **not** fall back to `object[]`, which is
  the *top* of the element lattice and so is not assignable to `string[]` at all.
- A binding whose type an empty list fixes is still reported where it carries no annotation —
  `let a = {}`, `let f(x) = {}` — naming the binding. That is a legibility rule rather than a
  typing failure: the system can type them, but a signature saying "a list of nothing in particular"
  tells the next reader nothing.
- First-party formatting emits `{}` for an empty list instead of failing, and `format_value`'s
  empty-list error is removed.
- **Scope: values braces only.** `ElementsBracedExpression` — the body of an element-position `if`
  or `for` — continues to require at least one item, so `<div>if r {} else { <B/> }</div>` stays a
  parse error. Embedded text `@{}` likewise stays rejected: an interpolation that produces nothing
  is not something an author means to write.
- Every existing form parses exactly as it does today.

### A call argument takes a braced value

- `call_expression` takes a braced value in each argument position, so `f({})`, `f({a})`,
  `f({a b})`, and `f({}, x, {a b})` parse. This is one rule; every argument accepts one
  independently.
- The arity rule is unchanged and applies to an argument exactly as it applies to a property value:
  `{a}` is a scalar that the parameter's declared list type coerces, `{a b}` is a list, `{}` is the
  empty list taking its element type from the parameter.
- The type checker already routes every argument through the same seam a typed binding uses, so
  this needs no new plumbing: the parameter type is the expected type, and the empty list and the
  scalar coercion both resolve there.
- A call that cannot be checked — an undefined callee, a wrong argument count — reports its own
  diagnostic and nothing more. An empty list among its arguments is not separately reported as
  owing an element type, because the call is the thing to fix.
- **Scope: arguments only.** A `ValuesBracedExpression` is still not a `value_list_item_expression`,
  so a list is still not an item of a list — see "Not in this change".

### `never` enters the type system, not the source surface

- `Primitive::Never` joins `Primitive::Void` as a type inference assigns and source cannot write.
  Two rules carry it: it satisfies every expected type, and joining it with anything yields the
  other side.
- It is added **here** because this change already needed both rules and wrote them as special
  cases over a stand-in shape. Making the stand-in a real type is what lets the compensating
  machinery — a map of lists owing an element type, an end-of-analysis sweep, and a discharge
  obligation at every site that can conclude a binding — not exist at all.
- The primitive **set is unchanged at eight names**. `never` is absent from the grammar, from
  `builtin_type`, from completions and from syntax highlighting, so a user may declare
  `type never` exactly as they may declare `type void`.
- No value has bottom type, so it never reaches the interpreter, the FFI, or a runtime type test.

### `void` leaves the source surface

- `void` is removed from `primitive_type` in the grammar, from `builtin_type`, and from the editor's
  primitive completions. The NX primitive set becomes eight names: `string`, `int`, `int32`,
  `int64`, `float32`, `float64`, `boolean`, `object`.
- `void` in type position becomes an ordinary named type reference, resolved by exactly the rules
  that already govern `bool`, `float`, and `f64` — a user declaration of that name if one exists,
  and an unresolved type otherwise.
- The internal unit type is unchanged. `Type::Void` keeps its inference sites and keeps rendering as
  `void` in diagnostics. It becomes a type the system infers but an author cannot write, which is
  what it already was in practice.
- Nothing to migrate: no NX source in the repository names it.

## Capabilities

### Modified Capabilities

- `braced-value-sequences`: the "one or more" constraint becomes "zero or more" for
  `ValuesBracedExpression`, and the arity-to-type rule gains the zero-item case — list-valued, with
  the bottom type as its element, and a diagnostic at an unannotated binding whose type it fixes. A
  call argument joins the positions that accept the braced rule, at every arity, while an item of a
  braced value still does not.
- `unbraced-literal-forms`: the requirement *First-party NX-syntax value output round-trips* cites
  the empty list as an example of a value with no source spelling. It has one now, so the example is
  replaced and the empty list moves from "report a failure" to "emit `{}`".
- `primitive-type-names`: the primitive set drops from nine names to eight, and the requirement on
  first-party listings of those names follows it. Two new requirements state that the unit type and
  the bottom type are inference-internal and have no source spelling, so their absence from the set
  is a deliberate property rather than an omission.

## Impact

**Grammar** — `crates/nx-syntax/grammar.js`: `values_braced_expression` wraps its `choice` in
`optional`, `call_expression` takes its arguments from a new hidden `_call_argument` rule that is
`choice(value_expression, values_braced_expression)`, and `void` leaves `primitive_type`.
Regenerating the parser with those edits produces a conflict set identical to today's, so neither
admitting the empty form nor admitting the brace as an argument costs new ambiguity. Parser fixtures
cover `{}` in each accepting position, braced arguments at each arity and in each position, and the
positions that still reject a brace.

**Type checking** — `crates/nx-types`: a `Primitive::Never` variant, one arm making it satisfy every
expected type, one arm making it the identity of the join, and a zero-item braced expression
inferring `Array(Never)`; `builtin_type` stops resolving `void`. Arrays are already covariant and any
array already satisfies `object`, so those two arms reach every site an empty list can be written at
without a site-by-site change — see design D10. `infer_call` already checks each argument through
the same seam a typed binding uses, so admitting the brace as an argument needs no inference change
either. What remains is one diagnostic, reported at the two places a binding's type can be fixed by
an empty list: an unannotated value binding and an unannotated function return.

**Evaluation** — `crates/nx-interpreter`: body content that is present and produces no values binds
the empty list rather than leaving the content property unbound. Without this, `<List>{}</List>`
type checks and then loses the value: the property falls through to null, which fails to coerce at
a non-nullable list field and reads back as null at a nullable one.

**Formatting** — `crates/nx-cli/src/format.rs`: `format_property_value` emits `{}` where it
currently returns `unspellable_empty_list()`, and that error and its test are removed. The
own-value path emits `{}` for an empty list too, and takes the same nested-list guard, since a run
of values one per line cannot spell either case.

**Editor tooling** — `crates/nx-language-service`: `PRIMITIVE_TYPE_COMPLETIONS` drops `void`. So do
the two primitive alternations in `src/vscode/syntaxes/nx.tmLanguage.json`, which otherwise colour
a user type that takes the freed name as a primitive, and the primitive list in `README.md`.

**Code generation** — `crates/nx-codegen` and `crates/nx-cli/src/typegen`: the primitive-name-to-host
mappings still contain `void`, which must no longer be reachable from a source type reference now
that a user declaration may take the name.

**Documentation** — `nx-grammar.md` and `nx-grammar-spec.md` both list the primitive names and the
braced-expression grammar; both change.

**No migration.** The empty-list half is purely additive to what parses. The `void` half removes a
spelling that no NX source in the repository uses.

## Sequencing

The empty-list half was filed by `replace-enums-with-unions` task 6.5, which makes the gap
observable but does not close it; nothing there blocks this.

The two halves are independent and can land in either order — they share a theme (aligning what NX
source can spell with what its type system holds) and the two files they both touch, not a
dependency. See design D7.

## Not in this change

Inference joins `Type::Void` into result types at three sites, and `common_supertype` has no case
for it, so it falls through to `object`. A condition-list match with no `else` therefore infers
`object` rather than its arms' type. That is a real defect, it is adjacent to the reasoning here,
and it is **not** fixed here — it changes inferred types for existing programs and deserves its own
change. See design's Open Questions.

Not in this change is admitting a braced value as an *item* of another braced value. A
`ValuesBracedExpression` is not a `value_list_item_expression`, so `items={{"a" "b"}}` and
`items={{}}` stay syntax errors at every arity, and so does a braced item inside a braced argument:
`f({{a} b})`.

That is a decision rather than a deferral. At arity one the brace is a scalar, so a nested brace
would collapse: `{{"a" "b"}}` would mean a scalar holding a `string[]`, the outer brace a no-op, and
`{{}}` would mean exactly what `{}` means. A `string[][]` with two rows could be written and one
with a single row could not — silently, since the collapsed value still binds at a `string[]` site.
Nesting cannot be made regular without giving up the arity rule, which is what lets `{name}` be an
expression escape everywhere else.

One consequence is that a list held directly in a list has no *literal* spelling, so first-party
formatting reports such a value rather than emitting `{{}}` — the `unbraced-literal-forms` delta
says so, and the guard the formatter used to get from the empty-list check is now stated on its own
terms. Nested lists are still reachable: a host can hand one across the FFI boundary, and a
value-position `for` with a multi-item body constructs one (`let xs:string[][] = {for y in ys {y
y}}` type checks and evaluates today, before this change). So `T[][]` is a type the system holds
and that no *literal* can write, which is why rendering one has nowhere to go. Closing that would
take a distinct list literal, whose brackets could be list-valued at every arity because they would
carry no escape meaning to overload. That is its own change.

Neither is host-identifier escaping in code generation. A user type whose name collides with a host
keyword is emitted unescaped: `export type class = { ... }` generates `public sealed class class`
today, before this change. Freeing `void` adds one name to that existing set — the reference does
resolve to the declaration rather than to the host's `void`, which is the part this change is
responsible for and which is verified — but what the name then *renders* as is the pre-existing
defect, and fixing it means escaping every host keyword in both backends.
