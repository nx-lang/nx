## Context

`crates/nx-types/src/ty.rs` defines nine primitives, two of which are aliases, and their names come
from two unrelated language families — the numerics from Rust, the boolean from C:

```
i32   i64   int(=i64)   f32   f64   float(=f64)   string   bool   void
```

`Int` and `Float` are display-preserving synonyms that compare and hash equal. Making that work
costs a private `CanonicalPrimitive` enum, a `canonical()` projection over it, and hand-written
`PartialEq`, `Eq`, and `Hash` impls — roughly 37 lines whose only purpose is to make two spellings
of one type behave like one type. `object` appears in the grammar's `PrimitiveType` production but
has no `Primitive` variant; it is handled elsewhere in the type model.

Two consequences shape this design. Because the synonyms are display-preserving, a declaration
written `float` reports *"expects float"* and one written `f64` reports *"expects f64"* for the
identical type — the language has no canonical rendering. And inference has already picked a side:
`Literal::Int` infers `Type::int()` and `Literal::Float` infers `Type::float()`, so the compiler
says *"found float"* and never *"found f64"*, regardless of how the declaration was spelled.

Numeric compatibility is category-wide and width-blind: `is_compatible_with` returns true for any
integer against any integer and any float against any float, in both directions. Integers are not
compatible with floats, which is the behavior recorded as NXE8 in
`docs/drawn-ui-proposal-nx-enhancements.md`. None of that changes here.

## Goals / Non-Goals

**Goals:**
- Exactly one spelling per primitive type, with no aliases and no display-preserving behavior.
- Type names whose width is readable without prior knowledge of any particular language's
  conventions.
- One reference language. NX matches TypeScript except where it is forced to differ, and the places
  it is forced are identifiable rather than a matter of taste.
- **Delete, do not redirect.** The alias machinery is removed rather than re-pointed at the new
  names, and no compatibility surface of any kind is introduced — no transitional aliases, no
  deprecation warnings, no dual-name acceptance, no IR upgrade path. `int` returns as a type of its
  own, which is the opposite of an alias: it has its own range and its own diagnostics.
- **A concise default integer whose meaning does not vary by backend.** `int` is defined by a range
  every backend represents exactly, so implementation freedom lives in codegen rather than in
  semantics.
- Keep observable semantics stable apart from the new default integer. Widths of the named types,
  compatibility rules, and host type mappings are exactly as they are today; literal binding is the
  one deliberate exception, because an integer literal now infers `int` rather than the type
  formerly spelled `i64`.

**Non-Goals:**
- Changing float or integer-to-float semantics. Integer-to-float compatibility (NXE8) stays as it is.
- Enforcing `int`'s range or checked arithmetic at runtime. Specified here, deferred to the
  bounds-check change that also covers user-declared ranges (`1..10`).
- Carrying `int64` as a JavaScript `bigint`. Intended direction, deferred to its own change.
- Bringing back `float`. `float64` remains the only default-float spelling.
- Adding `int8`, `int16`, unsigned types, or a decimal type.
- Renaming IR literal *kind* tags. See Decisions.
- Resolving the `object` grammar/`Primitive` mismatch. Pre-existing and orthogonal.

## Decisions

### Word+width names over Kotlin-style word names

`int32 / int64 / float32 / float64`, rejecting `int / long / float / double`.

The Kotlin-style set is tempting because `int` = 32 would match nearly every reader's prior from
Java, C#, C, and Go, fixing the exact hazard that motivates this change. It was rejected because it
trades one unreadable name for another: `float` = 32-bit and `double` = 64-bit is a historical
accident from C that no reader can derive and everyone must memorize. Word+width is the only scheme
in which **no name requires prior knowledge to read correctly**, which matters disproportionately for
NX because its schemas are read by models and by non-specialists as often as by systems programmers.

### TypeScript is the reference language, and the exception is identified

Every naming decision below is decided by one rule: **match TypeScript except where NX is forced to
differ.** TypeScript is what NX users write when they are not writing NX, and what NX generates
alongside C#.

The rule earns its keep by having a boundary rather than being a preference. NX is forced off
TypeScript in exactly one place. TypeScript has a single `number`; NX generates C# and MessagePack
and must carry widths, so it cannot collapse four numeric types into one name. That is the whole of
the exception, and `string`, `void`, `object`, and the boolean all fall outside it.

### Word+width names over Rust-style `f64`

Once forced off `number`, NX must invent numeric names, and `int32 / int64 / float32 / float64` is
the best available scheme rather than a borrowing from any one language.

First, internal consistency: `f64` is the only abbreviation among NX's primitives, which otherwise
spell out `string`, `void`, and `object`. `AGENTS.md` legislates against abbreviation-flavored
naming elsewhere ("treat 'UI' as a word").

Second, readability without priors. `f64` belongs to the systems-language family and is confined in
practice to Rust and Zig; a reader who has not written either cannot decode it. Word+width is the
only scheme in which no name requires prior knowledge, which matters disproportionately for NX
because its schemas are read by models and by non-specialists as often as by systems programmers.

Third, the precedent for width-in-the-name is strongest exactly where NX sits. Protobuf
(`int32`, `int64`), Cap'n Proto (`Int32`, `Float64`), and FlatBuffers (`int32`, `float64`) all put
the width in the name because their values cross a boundary into another language. This is cited as
evidence about naming under that constraint, not as a claim that NX is an IDL — NX is a language for
writing applications and UI, and it happens to share this one constraint.

Dart supplies the sharpest form of the argument. Its ordinary code uses `int` and `double`, but
`dart:typed_data` and `dart:ffi` use `Int32`, `Float64List`, and `ffi.Int64` — width names appear
precisely where a value crosses a boundary and its representation matters. NX has no "ordinary code"
tier where the width is a private detail; every declaration is a boundary declaration.

### `boolean`, not `bool`

`bool` is the one primitive that diverges from TypeScript for no reason NX can point at. The
forced-difference exception does not reach it: a boolean is one bit wide in every target, so there
is no width to carry and nothing to disambiguate. `bool` diverges because Rust and C spell it that
way, which is the same accident that produced `f64`.

`crates/nx-cli/src/typegen/languages/typescript.rs` makes the point mechanically. Its mapping table
has four kinds of entry: names that pass through unchanged (`string`, `void`), numerics that collapse
to `number` for a real reason, `object` → `unknown` for a semantic reason, and `bool` → `boolean`,
which is a pure spelling translation carrying no semantic content whatsoever. It is the only rename
in the table that exists solely because the two languages picked different words for the same thing.

The surrounding evidence is one-sided once the systems-language family is set aside. ANSI SQL:1999
specifies `BOOLEAN`; PostgreSQL, DB2, Snowflake, Redshift, and DuckDB all name it `BOOLEAN` and offer
`bool` only as an alias bolted on afterward. In no dialect is `bool` the canonical name. The
JSON-descended schema languages NX shares a wire format with — JSON Schema, OpenAPI, Avro, GraphQL,
TypeSpec, Smithy — all say `boolean`. The languages that say `bool` are C, C++, Rust, Go, C#, and the
binary IDLs, which is the family NX has just decided it is not drawing its names from. Choosing
`bool` while removing the `int` alias would mean adopting the *alias* form as the one canonical
name, which is the same mistake in the opposite direction.

The friendliness argument for `boolean` — that spelled-out names read as less "techy" to
non-specialist authors — is real but marginal, and should not be leaned on. Excel, the most
successful computing environment ever built for non-programmers, does not expose a boolean type name
at all: its users write the values `TRUE` and `FALSE` and never name the type. Non-technical authors
write values and markup, not type declarations. TypeScript consistency carries this decision on its
own.

### `int` is defined by range, not by storage width

`int` is exact over ±(2^53−1) on every backend. The alternatives were an implementation-defined
width (Go's rule) and a plain 64-bit type.

**Implementation-defined was rejected because NX is not Go.** Go's `int` can vary by platform
because one toolchain compiles the whole program and values never cross an implementation boundary
mid-flight. NX has three backends — the Rust interpreter, generated C#, generated TypeScript — plus
`.nxir.json`, a *persisted, cached, cross-implementation* artifact produced by the Rust toolchain and
consumed by the TypeScript runtime. "Implementation-defined" in that setting does not mean "the
compiler picks the fast one"; it means the same program has different semantics per binding, with
values crossing the boundary at runtime.

The precedents are one-sided. GopherJS makes `int` 32-bit on the web against 64-bit natively, and
documents it under *compatibility caveats*. Dart's `int` is 64-bit natively and a double on the web,
where `2^53` and `2^53+1` collapse to the same value and large arithmetic silently approximates
rather than overflowing — a divergence Dart has had to document permanently
([dart-lang/language#3197](https://github.com/dart-lang/language/issues/3197)). Scala.js supplies
the sharpest lesson in the other direction: strict float semantics were opt-in until 1.9.0 made them
the default, because semantics selected by a compiler flag are not semantics.

**2^53 rather than 64-bit** because it is the only integer range that is exact *and* cheap on every
backend at the same time: a C# `long`, a Rust `i64`, a JavaScript `number`, and a plain JSON number
all hold it without special representation. 64-bit is exact on two of the three and requires
`bigint` on the third, where `JSON.stringify` throws, `1n + 1` throws, `Math.max` throws, and
`1n === 1` is `false`.

So the rule is: **implementation freedom moves from semantics to codegen.** A backend may store
`int` in whatever slot is natural, because every slot that qualifies holds the entire specified
range, which makes the choice unobservable to an NX program.

The cost is that `int` is not the machine's widest integer, which is the objection to answer for
timestamps, ids, and file sizes. 2^53 milliseconds is roughly 285,000 years, 2^53 bytes is 9
petabytes, and JavaScript's own `Date.now()` lives in the same range — so the values that genuinely
exceed it are hashes, snowflake ids, and bit masks, which is exactly what `int64` is for.

### The range is specified now and enforced later

Specifying a range and not checking it is a real gap, and it is taken deliberately rather than
overlooked. `crates/nx-interpreter/src/eval/arithmetic.rs` uses `wrapping_add`, `wrapping_sub`, and
`wrapping_mul` for every integer type today, so nothing enforces any width, `int32` included — the
gap predates `int` and is not widened by it.

Enforcement is deferred because the same mechanism is wanted for user-declared ranges (`1..10`),
and building one bounds-check path that serves both is better than building a numeric one now and
retrofitting it. Cost is not the obstacle: on Node v24 an unchecked add measures ~0.74 ns/op against
~2.51 ns/op for a `Number.isSafeInteger`-guarded add, and `Number.isSafeInteger` is a V8 intrinsic
that beats a hand-written comparison (1.03 ns vs 1.35 ns). At roughly 500K integer ops per frame to
cost 1 ms, that is not a barrier for a UI language.

The check bound is the type's **specified range**, not the storage width — so when it lands, .NET
must throw above 2^53 for an `int` even though its `long` could hold the value. Otherwise `int`
would quietly become an implementation-defined width again, by the back door.

### `int64` stays a JavaScript `number` for now

`int64` generates TypeScript `number`, which is exact only to 2^53−1 and so cannot represent the
whole of `int64`. This is pre-existing — `typescript.rs` has always collapsed all numerics to
`number` — and it is deferred rather than fixed here.

The intended direction is `bigint`, with `BigInt64Array` for arrays. Kotlin/JS is the strongest
signal: it carried `Long` as a custom object with two `number` properties for years and moved to
native `BigInt` in 2.2.20, citing interoperability. Measured, `bigint` arithmetic is ~2.93 ns/op —
comparable to a checked `number` at ~2.51 ns/op — so the cost is ergonomic rather than arithmetic:
`JSON.stringify` throws on a `bigint`, it cannot mix with `number`, and `1n === 1` is `false`.
`BigInt64Array` fixes storage (8 bytes/element against 32 for a boxed `bigint`) but every read
materializes a fresh `BigInt`, so allocation churn remains.

Two consequences worth recording. First, this gap is the reason `int` is the default rather than
`int64` — `int`'s range is exactly what JavaScript represents losslessly. Second, arrays of `int`
lower to `Float64Array` with zero allocation churn while arrays of `int64` cannot, which is an
independent performance reason to keep `int64` rare.

The IR already handles the large-integer case the honest way: integer literals that cannot
round-trip through a JavaScript number are encoded as strings with no numeric field, and the
TypeScript runtime rejects arithmetic that would need a lossy conversion. That machinery stays and
becomes the foundation for the `bigint` change.

### Aliases are deleted, not re-pointed

All three of the languages examined agree that a numeric alias is unnecessary. Kotlin has none. Dart
has none. Swift has `Float32`/`Float64` as `typealias`es but no `Int` alias — `Int` is a distinct
type from `Int64` specifically so that code cannot silently acquire a width dependency — and Swift
canonicalizes aliases away, so `Float64.self` prints `Double`.

NX's display-preserving synonym is therefore unusual even among languages that do have aliases, and
it is the direct cause of the two-diagnostics-for-one-type behavior.

The payoff is subtractive. `Primitive::Float` is gone outright, and `Primitive::Int` survives only
because `int` returns as a type of its own — a variant with its own canonical spelling, not a second
name for `Primitive::Int64`. Either way no two variants share a canonical value, so
`CanonicalPrimitive`, `canonical()`, and the hand-written `PartialEq`, `Eq`, and `Hash` impls all
become identity functions over the enum and are deleted. `Primitive` returns to a
plain `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]`, and equality becomes variant equality
with no projection in the middle. This is the main reason to prefer removal over renaming the
aliases: renaming keeps every line of that machinery alive to serve two spellings that no longer
need to exist.

### Literals infer as `int` and `float64`

`Literal::Int` infers `int` and `Literal::Float` infers `float64`. Diagnostics become
*"expects float64, found int"* where they previously read *"expects f64, found int"* — the same
fact, now stated in one vocabulary on both sides of the message.

An integer literal taking `int` rather than `int64` is what makes `int` the default in practice
rather than only by convention: an author gets the safe, everywhere-exact type without naming it,
and has to write `int64` deliberately to opt into the range that JavaScript cannot represent.

### The IR schema version is bumped and stale IR is rejected

Primitive type names travel in the IR as strings (`{"kind":"primitive","name":"int"}`), so IR
produced before this change describes types that no longer exist. `NX_IR_SCHEMA_VERSION` goes from
`1` to `2` in both `crates/nx-codegen/src/ir.rs` and `runtime/typescript/src/index.ts`.

No upgrade path is written. The TypeScript runtime already rejects any IR whose `schemaVersion` does
not match exactly, and there is no multi-version reader to extend — so stale IR fails loudly at load
with an unsupported-version error, which is the correct and cheapest outcome. The IR is a build
artifact, not stored state; consumers regenerate.

### IR literal kind tags are a separate namespace and stay

The IR carries two unrelated uses of these words. Type shapes are
`{"kind":"primitive","name":"i64"}`, where `name` is a **type name** and must be renamed. Literal
values are `{"kind":"int","value":"42"}`, where `kind` is a **literal category tag** describing the
lexical form of the token, not a type. The literal tag has no width claim to make, and renaming it to
`int64` would be actively wrong, since the same tag serves a literal that binds to an `int32` site.

With `int` reintroduced as a type, the type name `int` and the literal tag `"int"` are once again the
same string. That is accepted for the same reason the `boolean` homonym below is, and it has the same
practical consequence: the sweep goes by field, not by string. Sites in
`runtime/typescript/src/index.ts` around line 590 are literal tags; those around line 852 are type
names. They must not be swept together.

The boolean rename runs the other way and is worth stating plainly. `NxIrLiteral::Boolean` already
serializes as `{"kind":"boolean"}` while `primitive_name` in `crates/nx-codegen/src/ir.rs:1831`
emits `"bool"` — the IR contradicts itself today. Renaming the type to `boolean` resolves that at
the cost of making the type name and the literal tag identical strings — the same near-homonym
`int` has. This is accepted: the two live in different fields of different objects
(`name` on a type shape, `kind` on a literal value), so nothing is ambiguous in the data, and the
homonym concern was always about rewrite and grep safety rather than about the format. It also means
the literal tag needs no edit at all — the fix is entirely on the type-name side.

### `bool` becomes `boolean` only in NX-facing positions

Unlike `int`, `bool` is a real type name in two of NX's own implementation languages. Rust source is
full of `bool`, and generated C# is supposed to contain `bool`. The rewrite is therefore *narrower*
than the numeric one, not wider: only `.nx` source text, grammar literals, NX-facing string literals
(match keys, name lookups, diagnostic `expected:` strings, completion lists), and the `Primitive`
variant change. Rust's own `bool`, the C# text emitted by `csharp.rs`, and TypeScript's `boolean`
output all stay exactly as they are. In `csharp.rs` the match *key* becomes `"boolean"` while the
emitted `text` stays `"bool"`; in `typescript.rs` the arm becomes `"boolean" => "boolean"`, a
pass-through.

### Existing `int` stays `int`

First-party `.nx` sources, fixtures, and spec scenario text keep `int`. Every one of those 67
type-position uses is an ordinary count, id, age, or coordinate — exactly what the default integer
type is for — so the rewrite that this change originally planned (`int` → `int64`) is not performed,
and the sites that had already been rewritten are returned to `int`.

The separation is decidable rather than a judgment call at each site, because the two populations
were never mixed: across every `.nx` file, `int` appeared 76 times and `i64` zero times. So the
sites that read `int64` today are exactly the sites that read `int` before, and returning them is
mechanical. `int64` is kept only where the declaration is *about* the 64-bit width — the C# `long`
mapping test, the capitalized-spelling test, and the numeric-width fixture added for that purpose.

### Nothing reports a former spelling after the rename, and that is left alone

This change was scoped to also report an unresolved type name at its declaration, on the theory that
`external component <B v:long />` type-checking clean was a defect in the resolution path the rename
already edits. Implementation showed it is not a defect but a language rule.

NX has no intrinsic-element registry. `named_type_is_element_like` in `crates/nx-types/src/infer.rs`
treats **any** undeclared name in type position as an element type, which is exactly how
`let <Single content item: div />: div` type-checks. To the resolver `long` and `div` are the same
thing: an element type that happens not to be declared anywhere. So after this change `v:long` is
not an error — it is a property whose type is the element `long`.

A denylist of the removed spellings plus `long` and `double` was built and then removed. It reported
nine names and stayed silent on `decimal`, `uint`, `int8`, and `real`, which is worse than an honest
gap: it looks like a general check while being a lookup table, and it would have to grow forever.

Reporting these properly needs one of two things, each its own change: an intrinsic-element registry
so the resolver can tell `div` from `long`, or a did-you-mean diagnostic that recognises names
authors reach for by mistake and names the NX type to use instead. The second is the better fit —
`long` should not merely fail, it should say *use `int64`* — and it wants the removed spellings to be
part of its table, which is a reason to build it after this rename rather than during it.

The consequence to accept for now: an author who writes `long`, `i64`, or `decimal` after this
change gets no diagnostic at the declaration. They get one at the use site, and it names the element
type rather than a width. `int` is no longer among the affected spellings, since it is a real type
again — which removes the single most likely instance of this trap without addressing the rule
behind it.

One related behavior is worth stating because it is now reachable: `builtin_type` is consulted
before user declarations, so `type int = { … }` does not capture uses of `int` — the primitive wins,
silently. That is uniform across primitives (`type string = { … }` behaves the same way) and is left
as it is; it is pinned by a test so the behavior is at least recorded rather than incidental.

## Risks / Trade-offs

**`int` is a substring of many identifiers, and of `int32`/`int64`** → A naive `sed` in either
direction corrupts `print`, `interface`, `into`, `interpreter`, and `println`, and a careless
`int64` → `int` rewrite also hits Rust's `Type::int64()` and the `"int64"` match keys that must
survive. Every rewrite is word-boundary anchored and restricted to type position — the pattern used
was `(?<!:):int64\b`, whose lookbehind is what keeps `Type::int64()` intact — and the fixture and
example rewrites are verified by re-parsing every `.nx` file rather than by inspection.

**Adding `int` to a regex alternation that already contains `int32`** → In the TextMate grammar and
the tree-sitter rule, a leading `int` alternative could shadow `int32` and `int64`. It does not,
because every alternation is `\b`-anchored: on input `int32` the `int` branch fails its trailing
word boundary and the engine backtracks to `int32`. Verified by parsing a fixture that declares all
five numeric types and by a grammar test asserting each is scoped as a primitive.

**`bool` is a legitimate type name in Rust and in generated C#** → A workspace-wide
`s/\bbool\b/boolean/` would corrupt every Rust `bool` in the compiler and would break C# codegen
by emitting `public boolean Enabled`. This rewrite is not mechanical and must be done site by site
from the enumerated list, with `cargo build` as the proof for the Rust side and a typegen run as the
proof for the C# side. The same trap exists in spec text: `retryable:bool` in a `WHEN` clause is NX
source and changes, while `public bool Retryable { get; set; }` in a `THEN` clause is expected C#
output and must not.

**Tree-sitter parser regeneration** → `crates/nx-syntax/grammar.js` changes, so the generated parser
must be rebuilt and its committed artifacts refreshed. If regeneration is skipped, the Rust type
system accepts the new names while the editor grammar still highlights the old ones.

**Names get longer in the most common position** → `float64` is four characters more than `f64`, and
NX property declarations are dense. This is the deliberate trade: the proposal's own object model has
roughly a hundred `f64` properties, and they become measurably wider. Judged worth it because the
width is the fact the schema exists to convey.

**Two documents in `docs/` use `f64` heavily** → `NX-Drawn-UI-MVP-Object-Model-Proposal.md` and
`drawn-ui-proposal-nx-enhancements.md` contain verified NX listings that will stop parsing. They are
rewritten as part of this change so the "verified against `nxlang`" claim in the enhancements
document stays true.

## Sequencing

One atomic change. Order matters only in that the grammar and type system land before the code and
sources that depend on them: grammar and `Primitive` first, then codegen and runtime, then the
mechanical source rewrites, then docs. Rollback is `git revert` of a single change.

## Open Questions

- Should `int8` and `int16` be added while the naming scheme is being settled? Out of scope here, but
  the chosen scheme extends to them cleanly, whereas the Kotlin-style alternative would have needed
  `byte` and `short`.
- Should `object` gain a `Primitive` variant so the grammar and type model agree? Pre-existing
  inconsistency, deliberately untouched, worth its own change.
- Should `Value::type_name` and the interpreter's `expected:` diagnostic strings be treated as
  canonical type names in their own right, or as an incidental second vocabulary? This change moves
  them to `boolean` on the assumption they are canonical, which is what the diagnostics requirement
  in the spec implies, but they were not audited for other divergences.
- Should an undeclared name in type position stay a valid element type? This is the rule that makes
  `v:long` type-check silently. Deciding it is a prerequisite for the did-you-mean diagnostic and
  probably wants an intrinsic-element registry. Its own change.
- Should `int` and `int64` have distinct runtime carriers? Today both evaluate to `Value::Int(i64)`
  and `Value::type_name` reports `int` for both, so a runtime diagnostic cannot tell them apart.
  This costs nothing while arithmetic wraps and nothing enforces a range, but the `bigint` change
  will need the distinction.
- Should a user-defined type be allowed to take a primitive's name at all, given that the primitive
  silently wins? Uniform across primitives and pre-existing; a duplicate-declaration diagnostic
  would be the fix, and it is its own change.
