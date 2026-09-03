## Context

See proposal.md — Why. The constraints that shape the approach:

- **`RhsExpression` is deliberately narrow.** `crates/nx-syntax/grammar.js:405` admits an element, a
  literal, or a braced expression, and nothing else. That narrowness is load-bearing: it is what
  makes an unbraced property value unambiguous without any type information. Everything below exists
  to add one form to that list without weakening the property.
- **The checker is infer-then-compare, not bidirectional.** `crates/nx-types/src/infer.rs` infers a
  type for an expression from the expression alone, then compares it against the expected type at
  the binding site via `check_typed_binding`. There are exactly three such sites relevant here:
  element and component properties (~1602), property and field defaults (~1826), and match patterns
  (~581). A contextual literal has no type derivable from the expression alone, so it is the first
  construct in NX whose type comes *only* from the site.
- **A bare name in pattern position already parses.** `pattern` (grammar.js:919) admits
  `qualified_name`, which admits a single identifier, and it lowers to `Expr::Ident`. So pattern
  position is the one place where this change alters the meaning of source that compiles today.
  Property value position is new syntax and cannot regress anything.
- **`-` is an operator, not part of a number.** `int_literal`, `real_literal`, and `hex_literal`
  (grammar.js:530-532) are unsigned, and `prefix_unary_expression` (~490) is reachable only from
  `value_expression`. That is why `x: float64 = -1.0` is rejected as an invalid definition before
  type checking runs, in record position as well as component position.
- **Coercion at typed binding sites already exists.** `type_satisfies_expected_with_coercion` and the
  scalar-to-list rule in `braced-value-sequences` mean the expected type is already massaged before
  comparison. Contextual resolution slots in ahead of that, not beside it.

## Goals / Non-Goals

**Goals:**
- Add two forms to `RhsExpression` — a contextual name and a signed numeric literal — while
  preserving the invariant that an unbraced value is a literal and never an expression.
- Make first-party NX-syntax value output re-parseable, which neither form achieves alone.
- Resolve a bare name through a single entry point shared by all three binding sites, so the rule
  cannot drift between them.
- Keep the source spelling invisible downstream: after type checking, nothing can tell a contextual
  literal from the qualified form.
- Make every failure a diagnostic that names the expected type and its members.

**Non-Goals:**
- Full bidirectional type checking. The deferral is scoped to one node kind at three known sites.
- Integer-literal widening at float-typed sites. It is numeric coercion rather than grammar or name
  resolution, and the formatter is made not to depend on it.
- Out-of-range integer literals. `<div>{9223372036854775808}</div>` prints `null` today — an integer
  literal that exceeds its type is silently swallowed with no diagnostic, and negating it fails at
  runtime with *Type mismatch in negation: expected number, got null* rather than at compile time.
  That is a pre-existing literal-parsing bug, not an unbraced-forms question, and it needs its own
  change. Noted here because D7's folding is a prerequisite for fixing the `int64` minimum
  specifically: `-9223372036854775808` can only be range-checked correctly if the sign is folded
  before the magnitude is checked.
- Any change to serialization, IR, or the wire format. `NX_IR_SCHEMA_VERSION` is not bumped.
- Any new resolution behavior inside braced expressions. Names there stay variables.
- A codemod rewriting existing `{Fit.cover}` occurrences. Both forms stay legal; see Open Questions.

## Decisions

### D1: A distinct HIR node, not `Expr::Ident`

A contextual literal lowers to a new `Expr` variant carrying the name and its span, not to
`Expr::Ident`.

*Alternative considered — reuse `Expr::Ident` and special-case it at the binding sites.* Rejected
because `Ident` is resolved against lexical scope during inference, well before any binding site is
reached. A bare `cover` with no `cover` in scope would report an unresolved-name error that the
binding site never gets a chance to replace, and a bare `cover` *with* a same-named variable in
scope would silently resolve to the variable — the exact ambiguity the strict split exists to
prevent. The two must be different nodes because they have different resolution rules.

*Alternative considered — resolve during lowering.* Rejected: lowering is file-local and does not
have type information (`source-analysis-pipeline`: "Raw lowering remains file-local and does not
require prepared visibility"). The expected type is not known until type analysis.

### D2: Deferred resolution at the existing `check_typed_binding` sites

Inference assigns the new node a "pending" marker rather than a type. Each of the three binding
sites, which already hold the expected type, calls one shared `resolve_contextual_name(name,
expected)` before its normal compare step. That function is the only place the resolution rule
lives.

The expected type is normalized before lookup, in this order: strip nullability, then strip one
list level if the site is list-typed (consistent with the existing scalar-to-list coercion), then
require the result to be an enum or a discriminated union. Anything else is a diagnostic.

*Alternative considered — thread an `expected: Option<&Type>` parameter through inference generally.*
That is real bidirectional checking, and it is the right long-term shape, but it touches every
`infer_expr` arm and would make this change a checker rewrite. The narrow deferral gets identical
observable behavior for the three positions the specs cover. If NX later wants bidirectional
checking for other reasons, `resolve_contextual_name` becomes the leaf of it rather than being
thrown away.

*Consequence to accept:* a pending node that reaches any site *other* than the three must produce a
clear diagnostic rather than a panic or a silent `Type::Error`. The grammar should make this
unreachable, but the checker must not depend on the grammar being right.

### D3: One identifier, never a qualified name

The grammar production is a single `identifier`, not `qualified_name`. This is the whole reason the
feature is safe: `fit=Fit.cover` and `fit=obj.field` are lexically indistinguishable, so admitting
the first admits the second, and an unbraced property value stops being a literal.

The cost is that `fit=Fit.cover` — which authors will try, since it reads naturally — is a parse
error. That is mitigated by the diagnostic, which offers both `fit=cover` and `fit={Fit.cover}`, and
it is the correct trade: a syntax error at the moment of typing is cheaper than an ambiguity in the
grammar forever.

Two lexical adjacencies to get right in the tree-sitter rule:
- `true`, `false`, and `null` are identifier-shaped and must keep resolving as `bool_literal` and
  `null_literal`, so the literal alternative must win. An enum member named `true` is unspellable
  bare; it is reachable as `{E.true}` if it is even a legal member name.
- Keywords (`if`, `else`, `for`, `is`, `content`) are not identifiers and cannot appear here, which
  is what keeps a bare value from swallowing a following property-list fragment.

### D4: Strict split in source, closed-set-then-fallback at the boundary

In NX source, bare resolves only nominally and quoted resolves only as string data, with no fallback
between tiers. At the JSON deserialization boundary the fallback rule still applies, because JSON
has no bare/quoted distinction — every string arrives quoted, and a `Paint`-typed field holding
`"none"` must check the closed set of payloadless cases before the open string alternative.

These are not in conflict; they are the same rule under different information. Source carries a
lexical signal that JSON does not, so source can decide by spelling what JSON must decide by
lookup order. Writing it down matters because the alternative — mirroring JSON's ambiguity in
source — was the version originally sketched in NXE2 change 2, and it has a defect the strict split
does not: under fallback, adding a payloadless case named `currentColor` to a union silently
reinterprets every existing `fill="currentColor"` from string data into that case, changing the
serialized shape of documents nobody edited. Under the strict split a quoted value is string data
when written and stays string data forever.

The observable consequence today, before any union carries a string alternative, is that
`fit="cover"` at an enum-typed prop stays a type error. That is a deliberate refusal of NXE2's
quoted form, not an oversight.

### D5: Nominal-first in pattern position, with a diagnostic when it displaces a binding

Pattern position is the one place with existing meaning: `if state is { idle => ... }` currently
compares the scrutinee against a variable named `idle`. After this change, when the scrutinee's type
is an enum or union and `idle` is one of its cases, the pattern resolves nominally instead.

Nominal-first is the right precedence — it is what every other match-style language does, and
variable-first would make the feature useless in exactly the position where enums are most often
matched. But a precedence change that alters the meaning of compiling source must not be silent, so
the checker reports a diagnostic whenever nominal resolution displaces a visible lexical binding of
the same name. The author can rename the variable or use the qualified pattern; either way they are
told.

*Alternative considered — variable-first, nominal as fallback.* Rejected: it makes the meaning of a
pattern depend on what happens to be in scope, which is the ambiguity D1 and D4 both exist to avoid.

### D6: Signed numeric literals are grammar, not lexing

`-1.0` is not a literal today. `-` exists only as `prefix_unary_expression` (grammar.js:490), which
is reachable from `value_expression` and not from `rhs_expression`, so a negative default is an
expression and needs braces. The fix adds `"-" (int | real | hex)` as an alternative in unbraced
value position and folds it to a negative literal during lowering.

*Alternative considered — absorb the sign into the numeric literal token in the lexer.* Rejected
outright: `a-1` would lex as `a` followed by `-1` and stop meaning subtraction, in every expression
in the language. Every language that has tried a signed numeric token has walked it back.

Doing it in the grammar and only where a literal is grammatically required is safe for the same
reason the contextual name is safe: those positions are not expression contexts, so a leading `-`
there cannot be a binary operator. `markup_identifier` cannot begin with `-`, so the token is
unambiguous against a following property name; patterns are comma-separated and `=>`-terminated, so
it is unambiguous there too.

**Match patterns are the second position, and they are broken today for the same reason.**
`Pattern ::= Literal | QualifiedName` and literals are unsigned, so
`if n is { -1 => "neg one" ... }` is a parse error — verified against `nxlang`. It is the same gap
as the RHS one and gets the same fix.

**Expression positions get no grammar change.** `{-90 + currentRotation}` already parses correctly
and evaluates to `-45`; `-` is a prefix operator at `prec.right(130)` and binds tighter than `+`.
There is no gap to fix, and introducing a signed-literal production into `value_expression` would
create a genuine ambiguity — `a -1` against `a - 1` — that does not exist in the literal-only
positions. The asymmetry is not arbitrary: the grammar changes exactly where a literal is required
and nowhere a binary operator can appear.

Scoped to `-` and to numerics. `!true` stays an expression and stays braced — nobody writes it as a
default, `false` exists, and admitting one prefix operator invites the rest.

#### Precedent

The asymmetry is the mainstream convention rather than an NX quirk.

| Language | Numeric token | Expression `-1` | Pattern `-1` |
|---|---|---|---|
| C, Java, Go | unsigned | unary minus | n/a |
| C# | unsigned | unary minus | signed literal |
| Rust | unsigned | unary minus | `-? LITERAL` in the pattern grammar |
| Python | unsigned | `UnaryOp(USub, …)` | `signed_number: NUMBER \| '-' NUMBER` |
| Haskell | unsigned | unary minus, notoriously | — |
| JSON, TOML, YAML | **signed** | — | — |
| Lisp, Clojure | **signed** (reader) | — | — |

The rule underneath the table: **the sign belongs in the token exactly when the grammar has no
infix minus in that position.** The data languages have no infix operators anywhere, so they take
signed numbers. The programming languages all have infix minus in expressions, so they keep `-` an
operator there — Haskell is the cautionary tale for doing otherwise. Rust and Python, which added
pattern matching later, both put signed literals back in for patterns specifically, because patterns
have no infix operators either.

NX's unbraced value position is structurally a JSON value position, and its pattern position is
Rust's and Python's. This design draws the same line in both.

### D7: Fold negation of a numeric literal during lowering, everywhere

Without this, the change would introduce a wart it is meant to remove: `= -1.0` would lower to a
negative `Literal` while `= {-1.0}` kept lowering to `UnaryOp { Neg, Literal(1.0) }`
(`crates/nx-hir/src/lower.rs:1013`, which does no folding today), giving two lowered shapes for one
meaning. Folding `-` applied directly to a numeric literal in every position collapses them, and it
is what makes the grammar asymmetry above invisible below the parser: the unbraced form, the braced
form, and `-90` inside a larger expression all arrive at the same representation.

Folding applies only to a literal operand. `{-n}` stays a unary operation.

*Alternative considered — fold only in the new unbraced and pattern positions.* Rejected: it keeps
the two shapes, so every consumer of lowered output has to handle both, and the pattern position
would still need a negative literal to compare against.

Folding is also what makes the minimum value of a signed type spellable, and the languages that
skipped it all carry a scar for it: C never fixed the wart, so `limits.h` still defines `INT_MIN`
as `(-2147483647 - 1)`, and Java and C# each bolted on a targeted rule saying the max-magnitude
literal is legal only as the operand of unary minus. Folding is strictly better than that carve-out
because it handles the general case with no exception in the spec. Go reaches the same place through
arbitrary-precision untyped constants, which NX cannot adopt now that `int` has a specified range.

This is bundled with the contextual name rather than shipped separately because they edit the same
production, share one tree-sitter regeneration and one fixture corpus pass, and — as D8 shows —
neither one alone is enough to make the value formatter round-trip.

### D8: Rendering splits by context, and the value formatter's attribute path is a bug fix

`enum-values` requires first-party display to use `Type.member`. That stays right for hover,
diagnostics, and value display, where naming the type is the point. It is wrong for NX source
output, where the qualified form is now not even the canonical spelling.

The defect is wider than the enum arm. `format_attribute_value` quotes every scalar it emits, so a
formatted record comes back as `<Box fit="Fit.cover" flag="true" neg="-1" w="1.5" />`, and re-parsing
that yields four type errors — *expects Fit, found string*, *expects boolean, found string*, and two
of *expects float64, found string*. Numbers, booleans, and null have been legal unbraced all along
and were quoted anyway; only the enum needed new syntax.

That makes the round-trip fix the reason the two halves of this change belong together. Emitting
`fit=cover` needs the contextual name; emitting `neg=-1.0` needs the signed literal; either one
alone leaves the formatter still broken. The float must also be spelled `-1.0` rather than `-1`,
because an integer literal does not widen at a `float64` site — the formatter carries that itself
rather than waiting on the widening work this change defers.

The requirement is stated as round-tripping rather than as a list of arms, so it stays true as new
value kinds are added. The test at ~305 covers only the non-attribute path and needs siblings.

## Risks / Trade-offs

- **Pattern-position meaning change (D5).** → Nominal-first is specified, and the displacement
  diagnostic makes every affected site visible at compile time. The repository has no such
  collision today; the check exists for downstream code.
- **A value no longer names its type, hurting readability of unfamiliar code.** → Hover,
  go-to-definition, and completion on the bare name all resolve to the member declaration. This is
  the same trade CSS, HTML, and Swift's implicit member expressions already make, and NX keeps the
  qualified form available for cases where the author wants it visible.
- **Two spellings for one member (`cover` and `{Fit.cover}`).** → They live in different grammatical
  positions — unbraced literal versus inside an expression — so they are not interchangeable
  alternatives so much as one form per context. The formatter canonicalizes source position to the
  bare form, which keeps the repository consistent without banning either.
- **The conditional cliff.** `fit={isWide ? .cover : .contain}` is not available; inside braces the
  author must write `Fit.cover` and import `Fit`, which is exactly the cost this change removes
  everywhere else. → Accepted and deferred rather than mitigated; the leading-dot form closes it
  unambiguously if evidence says it hurts. Recorded in proposal.md.
- **Cascading diagnostics when the expected type is unknown.** A bare name on a misspelled property
  or an unknown component has no type to resolve against. → The unknown-property or unknown-element
  diagnostic is reported alone; the pending node is discarded without a second error. Covered by a
  test.
- **Tree-sitter regeneration risk.** Adding an identifier alternative to `rhs_expression` is a small
  edit with a large generated diff. → The existing fixture corpus under
  `crates/nx-syntax/tests/fixtures/` is the regression net, and no existing fixture changes meaning.

## Migration Plan

Additive. Every existing spelling keeps working, no IR schema version changes, and no serialized
output changes shape. Rollout is a single change with no staging:

1. Grammar (both forms), HIR node, and lowering. Signed literals are complete at this step, since
   they fold to an existing `Literal` and need no checker work; contextual names parse and lower but
   are still rejected by the checker.
2. `resolve_contextual_name` plus the three binding sites, with diagnostics.
3. Formatter, language service, and docs.

Rollback is reverting the change; nothing persisted on disk or on the wire depends on it.

## Open Questions

- **Should the formatter rewrite existing `{Fit.cover}` occurrences in source position to the bare
  form?** The spec requires only that formatter *output* for a value use the bare form; whether
  formatting an existing file normalizes already-written braced member accesses is a codemod
  question. It can be answered after the feature ships without changing the specs, the approach, or
  the task breakdown, and the answer depends on how noisy the resulting diff is across the NX UI
  catalog.
- **Should the displacement diagnostic in D5 be a warning or an error?** Specified as a diagnostic;
  the severity can be tightened to an error later if the collision turns out to be always a mistake.
  Nothing in the resolution rule depends on which it is.
