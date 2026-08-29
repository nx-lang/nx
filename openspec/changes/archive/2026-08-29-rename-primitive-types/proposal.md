## Why

NX's primitive names were assembled from two different language families and agree with neither.
The numeric names are Rust's (`i32`, `f64`), the boolean is C's (`bool`), and six spellings cover
four numeric types because two of them are aliases: `int` is a display-preserving synonym for
`i64`, and `float` for `f64`. Because the synonyms are
display-preserving, the same type produces two different diagnostics — `expects float` from one
declaration and `expects f64` from another — and the same schema has two textual forms that
tools, docs, and codegen all have to handle.

The alias is also actively misleading. `int` means 64-bit in NX, but 32-bit in C#, Java, C, Go, and
Kotlin. NX's primary codegen target is TypeScript, where a 64-bit integer becomes `number` and is
exact only to 2^53. Dart is the closest precedent for the resulting hazard — its `int` is 64-bit
natively and 2^53 on the web, a divergence it has had to document permanently. In NX the exposure is
total rather than theoretical: across every `.nx` file in the repository, `int` appears 76 times (67
in type position) and `i32`, `i64`, `f32`, `f64`, and `float` appear zero times. The only numeric
spelling anyone writes is the one whose width is wrong for most readers' priors.

That argument is against the *alias*, not against the name. `int` is the spelling every author
reaches for, and the 76 uses across the repository are evidence that the language needs a concise
default integer. So `int` comes back as a type in its own right, defined by the one property the
alias lacked: a **range that is the same on every backend**. `int` is exact over ±(2^53−1) — chosen
because it is the widest range C# `long`, JavaScript `number`, and Rust `i64` all represent exactly
and cheaply at the same time.

That choice moves the implementation freedom from semantics to codegen. Go's `int` and GopherJS's
32-bit web `int` differ in what a program *means*; NX's `int` does not, because each backend is free
to pick storage only among slots that hold the whole specified range, which makes the choice
unobservable. It also disposes of the Dart hazard directly: the range where a JavaScript `number`
stops being exact is precisely the range `int` stops at, so the web backend has nothing to
approximate. `int64` keeps the full 64-bit range for the cases that genuinely need it — hashes,
snowflake ids, bit masks — which are rare and, on JavaScript, expensive.

`bool` is a smaller problem of the same kind. It is not misleading, but it is the one primitive
whose spelling diverges from TypeScript for no reason NX can point at. NX's reference language is
TypeScript: it is what NX's users write when they are not writing NX, and it is what NX generates.
`string`, `void`, and `object` already match it; the numerics are forced off it because TypeScript
has a single `number` and NX must carry widths to C# and MessagePack. `bool` is forced off it by
nothing at all — `crates/nx-cli/src/typegen/languages/typescript.rs` renames it to `boolean` on the
way out, the only entry in that table that is a pure spelling translation carrying no semantic
content. Nothing outside the systems-language family agrees with `bool` either: ANSI SQL specifies
`BOOLEAN`, and where dialects offer `bool` at all it is an alias bolted onto that name, never the
canonical one.

Now is the moment because the cost is near zero and the change is subtractive. All 67 type-position
uses of `int` and all 27 of `bool` are in examples, samples, and test fixtures, and no `.nx` file
uses a width-suffixed name. Removing the
aliases also removes the machinery built to support them: the `CanonicalPrimitive` projection and the
hand-written `PartialEq`/`Eq`/`Hash` impls exist solely so that two spellings compare as one type, and
they all go away. Every schema written from here on raises the price.

## What Changes

- **BREAKING** Rename the width-suffixed primitives to word+width form: `i32` → `int32`,
  `i64` → `int64`, `f32` → `float32`, `f64` → `float64`.
- **BREAKING** Remove the `int` and `float` *aliases* entirely. There is exactly one spelling per
  primitive type. The alias machinery is deleted along with them: the `CanonicalPrimitive`
  indirection, the `canonical()` projection, and the hand-written `PartialEq`/`Eq`/`Hash` impls that
  exist only to serve them; `Primitive` returns to a derived `PartialEq`, `Eq`, and `Hash`.
- **Reintroduce `int` as a distinct primitive type**, not as an alias. `int` is exact over
  ±(2^53−1) on every backend — the widest range C# `long`, JavaScript `number`, and Rust `i64` all
  represent exactly — and it is the **default integer type**: what an integer literal infers, and
  what NX sources use unless a declaration has a specific reason to name a width. `Primitive::Int`
  is a variant of its own, unequal to `Primitive::Int64`, so no alias machinery returns with it.
  `float` does not come back; `float64` is the only default-float spelling.
- Integer promotion follows the rank order `int32` < `int` < `int64`.
- `int` generates C# `long` (its range does not fit a C# `int`) and TypeScript `number`.
- **BREAKING** No compatibility surface is introduced: no transitional aliases, no deprecation
  warnings, no dual-name acceptance, and no IR upgrade path. The old names simply stop being types.
- Integer literals infer as `int` and float literals as `float64`, and diagnostics render those
  names — a mismatch now reads *"expects float64, found int"*.
- **BREAKING** Rename `bool` to `boolean`, matching TypeScript, ANSI SQL, and NX's own IR literal
  tag. `bool` stops being a type name; there is no alias.
- `string`, `void`, and `object` are unchanged — all three already match TypeScript.
- Correct the language service primitive completion list, which currently offers `long` and `double`
  — neither of which is or was an NX type — and omits every width-suffixed name.
- **BREAKING** Bump `NX_IR_SCHEMA_VERSION` from `1` to `2` so IR emitted before this change is
  rejected at load rather than misread. `int` joins the same unreleased schema version 2 and needs
  no further bump.
- **Deliberately deferred.** Two things this change specifies but does not enforce, each its own
  later change:
  - **Bounds checks.** `int`'s range and checked (non-wrapping) arithmetic are specified here but
    not enforced; `crates/nx-interpreter/src/eval/arithmetic.rs` still wraps. Deferred so that one
    bounds-check mechanism can serve both `int` and the user-declared ranges (`1..10`) that are
    wanted next.
  - **`int64` on JavaScript.** `int64` keeps generating TypeScript `number`, which is exact only to
    2^53−1. Carrying it as `bigint` is the intended direction, deferred to its own change. This gap
    is precisely why `int` rather than `int64` is the default.
- **Deliberately not fixed here.** A name in type position that resolves to no type is still
  accepted silently: an undeclared lowercase name is an intrinsic element type, which is how
  `item: div` works, so after this change `v:long` and `v:decimal` are indistinguishable from
  element tags. Reporting them needs an element registry or a did-you-mean rule, both of which are
  their own change. See design.md.
- **What changes semantically.** Integer literals previously inferred the type spelled `i64`; they
  now infer `int`, a distinct type with a narrower specified range, and integer promotion gains a
  middle rank (`int32` < `int` < `int64`). `int`'s range is not yet enforced at runtime — that is
  the deferred bounds-check work above — so no existing program's evaluation changes today; what
  changes is the type that is inferred, rendered in diagnostics, and carried into codegen.
- **What does not change.** The widths of the named types, boolean semantics, integer/float
  compatibility rules, and the TypeScript, C#, and MessagePack type mappings all keep their current
  behavior. In particular NX `boolean` still generates C# `bool` and TypeScript `boolean`, exactly
  as NX `bool` does today.

## Capabilities

### New Capabilities
- `primitive-type-names`: The canonical set of NX primitive type names, the prohibition on aliases,
  the specified range of `int` and its independence from the evaluating backend, and the names used
  when rendering inferred and declared types in diagnostics.

### Modified Capabilities
None. Eight existing specs use `int` inside scenario text
(`record-type-inheritance`, `content-properties`, `cli-code-generation`,
`record-construction-validation`, `source-analysis-pipeline`, `discriminated-unions`,
`braced-value-sequences`, `executable-code-generation`) and six use `bool`
(`component-syntax`, `discriminated-unions`, `cli-code-generation`, `external-components`,
`component-contract-inheritance`, `runtime-output-format`), but every one of those is an illustrative
type name in a `WHEN` clause rather than a requirement about the type system. The requirements are
unchanged; the example text needs a mechanical rewrite, which is tracked in tasks rather than as a
delta spec.

## Impact

**Grammar and parsing**
- `crates/nx-syntax/grammar.js` — the `primitive_type` rule; the tree-sitter parser is regenerated.
- `crates/nx-syntax/src/lib.rs`, `src/ast.rs`.
- `nx-grammar.md` (the `PrimitiveType` production) and `nx-grammar-spec.md`.

**Type system**
- `crates/nx-types/src/ty.rs` — the `Primitive` enum (including `Bool` → `Boolean` and
  `Type::bool()` → `Type::boolean()`), `as_str`, `is_integer`, `is_float`, and the
  `canonical()` / `CanonicalPrimitive` machinery that exists solely to make the aliases compare
  equal. Removing the aliases lets the custom `PartialEq`/`Hash` impls go and restores a derived
  `PartialEq`.
- `crates/nx-types/src/infer.rs` — `Literal::Int` and `Literal::Float` currently infer `Type::int()`
  and `Type::float()`.
- `crates/nx-types/src/semantics.rs` — the `"bool"` name lookup at line 108.
- `crates/nx-hir/src/ast/types.rs`, `src/lower.rs` (`"bool" => TypeTag::Boolean` at line 48),
  `src/scope.rs`.

**Code generation and runtime**
- `crates/nx-codegen/src/{emit.rs,builder.rs,ir.rs}` — several `match` arms list all the primitive
  names, including the `"bool"` arms in `emit.rs` at ~2249, ~2624, and ~3768, and
  `Primitive::Bool => "boolean"` at ~2657.
- `crates/nx-cli/src/typegen/languages/{csharp.rs,typescript.rs}` and `typegen/model.rs`. The C#
  mapping currently pairs `"i64" | "int"` and `"f64" | "float"`; each collapses to one arm. The
  match *keys* become `boolean` while the emitted C# text stays `bool`, and
  `typescript.rs:702`'s `"bool" => "boolean"` becomes a pass-through.
- `crates/nx-interpreter/src/{interpreter.rs,value.rs}` and `src/eval/logical.rs` — roughly a dozen
  `expected: "bool"` diagnostic strings and `Value::type_name`.
- `runtime/typescript/src/index.ts` — the primitive switch at ~852 including `case "bool"` at ~867,
  and the integer test at ~1335.

**IR**
- The IR carries primitive type names as strings (`{"kind":"primitive","name":"int"}`), so
  `NX_IR_SCHEMA_VERSION` is bumped in `crates/nx-codegen/src/ir.rs` and
  `runtime/typescript/src/index.ts`. The TypeScript runtime already rejects any mismatched version
  outright, so no reader changes are needed and no upgrade path is written.
- IR *literal kind* tags (`{"kind":"int"|"float"}` on literal values) are a separate namespace from
  type names and stay as they are — design.md records why.

**Tooling**
- `crates/nx-language-service/src/lib.rs:40` — `PRIMITIVE_TYPE_COMPLETIONS`.

**Sources to update**
- The 67 `:int` type-position occurrences across `examples/nx/**` and
  `crates/nx-syntax/tests/fixtures/**` stay `int`, since `int` is the default integer type and every
  one of them is an ordinary count, id, age, or coordinate.
- 27 `bool` occurrences across `examples/nx/**`, `src/vscode/samples/**`, and
  `crates/nx-syntax/tests/fixtures/valid/**`.
- Scenario text in the specs listed under Modified Capabilities, taking care that `bool` in
  *expected C# output* (`public bool Enabled { get; set; }`) is correct and must not be rewritten.
- Rust test files under `crates/nx-interpreter/tests/` and `crates/nx-codegen/src/tests.rs`.
- `docs/NX-Drawn-UI-MVP-Object-Model-Proposal.md` and
  `docs/drawn-ui-proposal-nx-enhancements.md`, which use `f64` throughout.
