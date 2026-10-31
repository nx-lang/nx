## Context

NX has no way to say that a number must lie in a range. Authors write it in a doc comment
(`/// Between 0 and 1.`), and nothing reads the comment: the checker accepts `opacity=1.5`, the
Visual view offers steppers that run past 1, the declaration schema a model reads says
`"type": "number"`, and a function that cares has to test its argument itself. The DrawnUI review
listed this as NXE9 (`opacity` in 0..1, `fontWeight` in 1..1000, `columnSpan >= 1`), and
`add-agent-host-package` (decision 14) reserved the shape a constrained type must take at the host
boundary.

Three pieces already exist to build on:

- **Range expressions.** `0..1` and `0..=1` build the prelude's `Range`; `specs/future.md` already
  imagines the same spelling in type position ("range types") and asks that integer overflow and
  range checks share one runtime check.
- **Editor hints.** `@nx/visualizer`'s `min`, `max` and `step` bound the Visual number editor, and
  the docs say plainly that they are not constraints. A constraint is the missing half.
- **Boundary checks.** Both IR runtimes already refuse a host value of the wrong type with an
  `nx-ir-boundary-*` code naming the argument.

## Goals / Non-Goals

**Goals**

- One syntax for constraining a type's values, usable wherever a type is written, readable by the
  less technical people and the AI agents who read and write NX most.
- The checker rejects a literal or constant that breaks a constraint, where it is written.
- A value the checker cannot prove is checked when the program runs, at the site it enters a
  constrained type, so a function never has to test its own arguments.
- Tooling reads constraints with no extra declaration: hover, the Visual number editor, JSON Schema.
- Both IR runtimes enforce constraints identically, at the host boundary and inside the program.

**Non-goals (this change)**

- Constraints on strings (length, pattern) and sequences (item count). The syntax is designed for
  them (decision 3); they are the next change.
- Arbitrary predicates (`where isEmail(value)`, `where value % 2 == 0`), cross-field record
  invariants, and bounds that name constants (`where value <= MaxColumns`).
- Proving constraints through arithmetic (Liquid Haskell, SPARK). The checker reasons about literals
  and about constrained types flowing into each other, nothing more.
- The tree-walking interpreter. It is being retired; it gets the checker's errors but no runtime
  checks.
- Generated C# and TypeScript host types. They keep the base type; constraints reach hosts through
  schemas and the runtime.

## How other languages do it

| Language | Spelling | Checked | Notes |
| --- | --- | --- | --- |
| Pascal, Modula | `type Percent = 0..100;` | runtime, optional | Integers and enums only. The original range type. |
| Ada / SPARK | `subtype Ratio is Float range 0.0 .. 1.0;` plus `with Static_Predicate => …` / `Dynamic_Predicate` | compile time for constants, runtime on every assignment (`Constraint_Error`); SPARK proves absence | Closest model to this design: a subtype is the base type plus a check, operations yield the base type, conversion into the subtype checks. |
| Nim | `range[0..10]`, `range[0.0..1.0]` | runtime, compile-time for constants | A distinct generic type. |
| Raku | `subset Ratio of Num where 0 <= * <= 1;` and `sub f(Int $n where $n > 0)` | runtime | `where` plus an expression; the closest spelling to this proposal. |
| Rust | nightly *pattern types*: `pattern_type!(u32 is 1..)` | compile time | Experimental; the stable language uses newtypes (`NonZeroU32`) and constructors. |
| Swift, Kotlin, Go, TypeScript, C# (language) | none | — | Newtypes with checked constructors, `require`, or property wrappers. C# has `[Range(0, 1)]` in DataAnnotations: metadata a validator reads, invisible to the compiler. |
| Zod, Valibot, ArkType (TypeScript libraries) | `z.number().min(0).max(1)`, ArkType `"0 <= number <= 1"` | runtime | Schema values, not types; ArkType shows how readable a comparison-chain spelling is. |
| Pydantic | `Annotated[float, Field(ge=0, le=1)]` | runtime | Keyword vocabulary mirrors JSON Schema. |
| Pkl | `ratio: Float(isBetween(0, 1))`, `name: String(length <= 10)` | evaluation time | Constraints as a parenthesized list of boolean expressions about `this`. |
| CUE | `ratio: float & >=0 & <=1` | evaluation time | Constraints are types; unification intersects them. |
| JSON Schema, XSD, OpenAPI | `minimum`, `exclusiveMaximum`, `pattern`, `minItems`; XSD facets | validator | The vocabulary every host already speaks, and what this design maps to. |
| Liquid Haskell, F\*, Dafny | `{v:Float | 0 <= v && v <= 1}`, `x:int{x >= 0}`, `type R = r: real | 0.0 <= r <= 1.0` | proved statically | Full refinement types; needs an SMT solver and proof effort. |

Three families fall out:

1. **Metadata** (C#, Pydantic, JSON Schema): easy, but a second language beside the type, and the
   compiler never checks it.
2. **Subtypes with checks** (Pascal, Ada, Nim, Raku, Pkl, CUE): the constraint is part of the type;
   constants are checked when compiled and everything else when run. This is the family NX fits:
   it keeps the check in the language without asking authors for proofs.
3. **Proved refinements** (Liquid Haskell, F\*, Dafny, SPARK): strongest, but needs proofs NX's
   audience will not write. Family 2 leaves room to grow towards it.

## Decisions

### 1. Syntax: `Type where value <condition>`

```nx
type Ratio = float32 where value in 0.0..=1.0
type Weight = int where value in 1..=1000
type Count = int where value >= 1
type Unit = float64 where value >= 0.0 && value < 1.0

type Paint = {
  opacity: float64 where value in 0..=1 = 1
  layer: Ratio
}

let fade(amount: Ratio, steps: int where value >= 1) = { … }
```

A type may be followed by `where` and a condition about `value`, the value being constrained. This
is Bret's candidate, kept nearly as written, because:

- **It reads as a sentence.** "opacity: a float64 where the value is in 0 through 1." A reader who
  has never seen the feature can say what it means; an AI agent will write it correctly from one
  example. `float32(0..1)` (Pkl) and `float & >=0 & <=1` (CUE) both need the feature explained.
- **The condition is NX's own expression syntax.** `in`, `..=`, `>=` and `&&` mean what they mean
  everywhere else (`in` as in `for`, the range operators as in `range-expressions`), so there is
  nothing new to learn, and the Visual view renders them with the operator symbols and words it
  already uses (`≥`, "through", "and").
- **`where` grows.** Strings, sequences and, later, general predicates are more conditions in the
  same clause (decision 3). A dedicated range form (`float32 in 0..=1`, `type Rating = 1..=5`) has
  nowhere to grow, and the bare range type leaves its base implicit: is `0..=1` an `int`, a
  `float32` or a `float64`?
- **`value` over `it`, `this` or `_`.** `value` says what it is to a non-programmer, matches the
  word the docs already use, and does not suggest an object (`this`). It is contextual: it means
  the constrained value only inside a `where` clause, and stays an ordinary name elsewhere.

Alternatives considered: Ada's `float32 range 0.0..1.0` (a second keyword that does only ranges);
Rust's `float32 is 0.0..=1.0` (`is` already reads as a pattern test in NX's `if … is`); attributes
`@range(0, 1)` (metadata the checker does not own, the C# failure mode); a parameter list
`float32(min=0 max=1)` (a second vocabulary beside the expression syntax, and close to element
syntax).

**Defaults.** A default follows the whole type: `opacity: float64 where value in 0..=1 = 1`. The
`=` after the constraint reads less well than elsewhere, which is one reason the style guide
recommends naming a constrained type that is used more than once (`opacity: Ratio = 1`). The
alternative, a constraint after the default (`opacity: float64 = 1 where …`), would put part of
the type outside the type, and would not work for a `let` annotation or a return type.

### 2. What a condition can say (this change)

A condition is one or more terms joined by `&&`, each term one of:

| Term | Meaning | JSON Schema |
| --- | --- | --- |
| `value in a..b` | `a ≤ value < b` | `minimum`, `exclusiveMaximum` |
| `value in a..=b` | `a ≤ value ≤ b` | `minimum`, `maximum` |
| `value >= a`, `value > a` | lower bound | `minimum`, `exclusiveMinimum` |
| `value <= b`, `value < b` | upper bound | `maximum`, `exclusiveMaximum` |

The bounds are numeric literals, optionally signed. The base must be a numeric primitive (`int`,
`int32`, `int64`, `float32`, `float64`) or an alias of one, constrained or not. A bound is checked
as a literal written at the base type (so `0.5` is rejected for an `int` base and `3000000000` for
an `int32` one), and the terms together must admit at least one value. Anything else, `||`, `!=`,
a call, a name other than `value`, is rejected with a diagnostic naming what is supported, not a
parse error, so the grammar can stay the same when more is supported.

The terms intersect, so every constrained numeric type is exactly one interval with inclusive or
exclusive ends. That makes everything below simple: containment is two comparisons, the editor
gets a `min` and a `max`, and the schema gets at most two keywords.

### 3. Where a constraint can appear

Anywhere a type is written: record, action and union-case fields, component props and state,
function parameters and results, `let` annotations, and aliases. It binds to the base type it
follows and so comes after any occurrence suffix; a constraint on a type that has one is rejected
in this change (`string+ where value.count >= 2` is the planned item-count form), with a fix-it
naming the item type: `type Positive = int where value > 0` then `Positive+`. Constrained types
are rejected as type arguments (`<Range T=Ratio/>`) and inside function types, since the IR erases
both and neither runtime could check them.

Planned next, with no new syntax: `value.length` and `value matches "…"` for strings (the pattern
dialect needs choosing across Rust, JavaScript and .NET), `value.count` for sequences, constant
names as bounds, and general pure predicates checked only at run time.

### 4. A constrained type is its base type plus a check

There is no new kind of value: a `Ratio` is a `float32`, encoded, compared, hashed and sent to a
host exactly as one. In the checker:

- A constrained type is structural. `type Ratio = float32 where value in 0.0..=1.0` is an alias,
  and an inline `float32 where value in 0.0..=1.0` is the same type.
- A constrained type satisfies its base, and every type its base widens to, with no check.
- `A` satisfies `B where …` with no check when `A`'s base widens to `B`'s and `A`'s interval lies
  inside `B`'s. So a `Ratio` passes to a parameter declared `float64 where value >= 0` freely.
- An alias of a constrained type may add terms (`type Low = Ratio where value <= 0.5`), which
  intersect.
- Arithmetic, as in Ada, yields the base type: `ratio * 2` is a `float32`, so a result is checked
  again when it enters a constrained site.

### 5. Checking: at compile time when constant, otherwise when the value enters the site

A value bound to a constrained site (a property, a field, an argument, a default, a `let`
annotation, a returned value, an item of a sequence whose item type is constrained) is checked:

- **A literal or constant expression** is checked by the type checker. One outside the interval
  is an error, `constraint-violation`, on the value: "1.5 is outside `0.0..=1.0`, the range of
  `opacity`". Defaults are checked the same way where they are declared.
- **A value of a type that is already inside the interval** (decision 4) needs nothing.
- **Any other value** is checked when the program runs, where it enters the site. The emitter
  inserts the check; the author writes nothing. This is the third motivation: `fade(x, 3)` checks
  `x` once, at the call, and the body of `fade` can rely on `amount`.

A failed check inside a program is the error `nx-ir-constraint`, naming the site and the interval.
A value a host passes in that breaks a constraint is refused at the boundary with
`nx-ir-boundary-constraint`, naming the argument, which is the shape `add-agent-host-package`
decision 14 asked for: a model that sends 11 where the parameter admits 1 to 10 gets
`invalid-input` and can correct it. A default computed from another field that misses its own
constraint names no argument, as that decision's fourth condition says.

`NaN` is in no interval, so it fails every numeric constraint.

The alternative, refusing to compile until the author proves or tests the value, is what Liquid
Haskell and SPARK do; it would make every arithmetic result need a `clamp` or an `if`. Implicit
checks are what Ada, Pascal, Nim and Pkl do, and the checker's literal errors catch the mistakes an
author or agent most often makes. A later change can add a warning for a check that will always
fail.

### 6. IR: one type kind, one node kind, one feature

- A type entry `[7, base, low, high, flags]`: `low` and `high` are constant-table indexes or `-1`
  for an open end, and `flags` bit 0 makes the low end exclusive and bit 1 the high end. Constants
  are written at the base type. The prelude and the IR reader treat every other type rule as they
  would for `base`.
- A node `[28, operand, type, site]`: evaluates `operand`, fails with `nx-ir-constraint` unless the
  value lies in `type`'s interval, and yields it. `site` is a string-table index naming what was
  being bound, for the message. It costs one operation, as a comparison does.
- A module that carries either lists the required feature `constraints-v1`, so a runtime that
  predates constraints refuses it by name.
- Parameters, record fields, component props and state fields carry their declared types already;
  with the constrained type in the table, the boundary check needs no new metadata.

The schema version does not change: the feature gate is how this IR adds kinds (ranges and
occurrences did the same).

### 7. Tooling

- **Diagnostics**: `constraint-violation` (a constant outside its site's interval),
  `invalid-constraint` (a term this change does not support, a non-numeric base, a bound of the
  wrong type, a constraint on a type with an occurrence, as a type argument or in a function
  type), and `empty-constraint` (terms that admit no value, such as `value > 5 && value < 2`).
- **Hover and signature help** spell the type with its constraint, as written, and through an
  alias show the alias's constraint.
- **Source tree**: a `SourceProperty` (and an alias's declaration) carries `bounds`: `min`, `max`
  and whether each is exclusive. This is there whether or not the request asks for visualizers.
- **Visual editor**: a number's editor keeps within the intersection of its constraint and its
  `min`/`max` hints, so a constraint alone gives steppers that stop at the ends and the note
  `At most 1`. A hint can narrow the range offered (a constraint of `0..=1000`, steppers of
  `0..=100`) but not widen it: the editor keeps within the intersection. An exclusive end reads
  `Above 0` / `Below 1`, and an integer with an exclusive end steps as its inclusive neighbour. A
  float with both ends and no `step` hint steps by the largest power of ten not above a tenth of
  its span (`0.1` for `0..=1`), since the default step of 1 would cross the whole range. (The
  slider was dropped from the Visual editor in favour of steppers on 2026-10-22; a constraint with
  two ends is what a slider would need, so this change keeps that door open.)
- **Schemas**: decision 2's table. Because the runtimes refuse what the schema now excludes, the
  existing requirement that a schema is never wider than the runtime holds.
- **Style guide**: name a constrained type used in more than one place, write bounds with the
  base type's literal form (`0.0..=1.0` for a float), and keep a doc comment for what the range
  means, not for what it is.

### 8. Grammar

`type` gains an optional `where_clause` after its suffixes: `'where' condition`. `where` and `value`
are keywords only there. The condition is parsed as a restricted expression, `value` followed by
`in` and a range of two signed literals or by a comparison operator and one signed literal, joined
by `&&`, which keeps a field list unambiguous without braces: a field list is whitespace-separated,
and the condition ends at the first token that cannot continue it. A broader condition (a call, `||`)
parses as a general `value_expression` inside the clause so that the checker can reject it by name.

## Risks / Trade-offs

- **Implicit runtime checks can surprise.** A computed value that is usually in range fails in
  production rather than at compile time. Mitigation: the error names the site and the interval,
  the checker catches every constant, and this is the trade every language in family 2 makes.
- **`value` is contextual.** A declaration named `value` cannot be read inside a `where` clause.
  No bound in this change can name a declaration, so nothing is lost now; when bounds can name
  constants, `value` stays reserved there.
- **Float bounds at `float32`.** `0.1` is not exact in a `float32`; the bound is converted as a
  literal at the base type is, and values are compared at the base type, so `0.1` written as a
  value and `0.1` written as a bound agree.
- **Two sources of `min`/`max`.** Hints and constraints both bound the editor. Keeping hints as
  "narrow what is offered" and constraints as "what is allowed" keeps the two distinct, and the
  docs say so.
- **Interpreter drift.** Programs run by the interpreter skip runtime checks. It is being retired;
  `specs/future.md` notes the gap.

## Open questions

1. Is `value` the subject word Bret wants (vs. `it`)? The design assumes `value`.
2. Is `opacity: float64 where value in 0..=1 = 1` acceptable as the inline-with-default form, given
   the style guide's advice to name the type?
3. Should integer overflow checking (`specs/future.md`, "Bounds checks are specified but not
   enforced") ride on the same `[28 …]` node in a follow-up? The node is shaped so it can.
