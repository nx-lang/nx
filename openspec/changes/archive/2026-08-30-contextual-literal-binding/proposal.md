## Why

Naming a value of a closed type in NX source costs more than the value itself. `Fit` has three
members and exactly one of them can appear at a `Fit`-typed prop, yet the source has to say
`fit={Fit.cover}` — braces because `RhsExpression` admits only an element, a literal, or a braced
expression, and a qualifier because a bare name in value position means a variable. Eleven
characters of the fourteen carry no information the type system does not already hold.

The qualifier is worse than verbose; it is a dependency. Writing `Fit.cover` means `Fit` must be
in scope, so every module that sets an enum-valued prop imports the enum type alongside the
component. That is where [NXE14](../../../docs/drawn-ui-proposal-nx-enhancements.md) bites: an
enum member is unreachable through a wildcard import alias (`ui.TextVariant.h2` — *Member access not
yet implemented*), so a module needing both aliased components and one enum member from the same
library must switch everything it takes from that library to the selective import form. The
proposal's §8.1 example imports its two catalogs in two different styles for exactly this reason.
A value that never names its type never triggers this.

Source also disagrees with the wire today. `openspec/specs/enum-values/spec.md` already requires an
enum member to serialize as "the bare authored member string", with the consumer recovering the
declaring type from the target context — so the JSON says `"cover"` while the source insists the
type must be respelled at every use. The information needed to resolve a bare name at a typed site
is the same information the deserializer is already required to use. NX simply declines to use it
on the source side.

Now is the moment because the NX UI catalog is the first NX schema large enough for the cost to be
measured rather than asserted — 29% of source characters in the §8.1 worked example, most of it
this and [NXE8](../../../docs/drawn-ui-proposal-nx-enhancements.md) — and because the type system
side is already built. The checker has the expected type in hand at all three binding sites
(`crates/nx-types/src/infer.rs`: element properties at ~1602, property defaults at ~1826, match
patterns at ~581). Nothing needs to be inferred that is not already known; what is missing is a
node that waits for it.

## What Changes

- **Add contextual literal binding.** An unquoted single identifier becomes a legal property value,
  resolved against the declared type of the site it binds to rather than against lexical scope:

  ```nx
  <Img fit=cover />              // today: fit={Fit.cover}
  <Card background=none />       // today: background={Paint.none}
  ```

  It resolves against enum members and payloadless union cases. Nothing else.

- **Grammar delta**: `RhsExpression` gains `ContextualName ::= Identifier`. **A single identifier
  only, never a `QualifiedName`.** This preserves NX's existing invariant that *an unbraced RHS is
  a literal and never an expression* — admitting `fit=Fit.cover` would also admit `fit=obj.field`,
  and the invariant would be gone. `{Fit.cover}` stays legal and stays the only way to name a
  member inside an expression.

- **Applies at three positions**, the ones where the expected type is already known and the syntax
  is unbraced: element and component property values, property defaults
  (`a: Alignment = start`), and match patterns (`if fit is { cover => ... }`).

- **Strict split between bare and quoted.** A bare name resolves **only** against the closed
  nominal set — enum members and payloadless union cases. A quoted string resolves **only** as
  string data. There is no fallback between the two tiers in NX source. Once a union may carry a
  string alternative alongside payloadless cases, `fill=none` is the case and `fill="none"` is a
  color whose text is `none`; both stay spellable, and adding a case to a union can never
  retroactively reinterpret an existing quoted value. The closed-set-then-fallback rule sketched in
  NXE2 still governs the **JSON deserialization boundary**, where no bare/quoted distinction
  exists; design.md records why the two rules differ and why that is not a contradiction.

- **A bare name that misses the closed set is an error, not a string.** The diagnostic names the
  candidate members, offers an edit-distance suggestion, and — when the expected type admits
  strings — suggests the quoted form: *`'white' is not a case of 'Paint'; for a string value write
  `fill="white"`*.

- **This supersedes the `RhsExpression` relaxation proposed in NXE8.** NXE8 asked for bare
  *member access* in default position (`a: Alignment = Alignment.start`). That is explicitly **not**
  adopted: it would let `a = foo.bar` through and break the unbraced-is-a-literal invariant.
  Contextual names deliver the same ergonomic win — `a: Alignment = start` — without it.

- **Admit signed numeric literals unbraced**, which is NXE8's other half. `-1.0` is not a literal
  today: `-` exists only as `prefix_unary_expression` (`crates/nx-syntax/grammar.js:490`), which
  lives in `value_expression` and not in `rhs_expression`, so a negative default is an expression
  and needs braces. Verified against `nxlang` — both of these are rejected before type checking
  ever runs:

  ```
  external component <C x: float64 = -1.0 />   // error: Invalid component definition
  type Opts = { x: float64 = -1.0 }            // error: Invalid record definition
  ```

  **Match patterns have the same hole.** `Pattern ::= Literal | QualifiedName` and literals are
  unsigned, so `if n is { -1 => "neg one" ... }` is also a parse error today. Same gap, same fix.

  The sign becomes part of the literal in those two positions — the ones that grammatically require
  a literal. **Neither tokenization nor expression syntax changes.** The lexer must not absorb `-`
  into a number, and `value_expression` must not gain a signed-literal production, or `a-1` would
  stop meaning subtraction. `{-90 + currentRotation}` already parses correctly and evaluates to
  `-45`; there is no gap there to fix. `!` and every other prefix operator stay expressions.

- **Fold negation of a numeric literal during lowering, in every position.** Today
  `crates/nx-hir/src/lower.rs` does no folding, so without this the change would create two lowered
  shapes for one meaning: `= -1.0` a negative literal, `= {-1.0}` a `UnaryOp` wrapping a positive
  one. Folding `-` applied directly to a numeric literal everywhere collapses them, and makes the
  grammar asymmetry above invisible below the parser. `{-n}` stays a unary operation.

- **Fix the value formatter's attribute output, which does not round-trip for any scalar type.**
  `format_attribute_value` in `crates/nx-cli/src/format.rs` quotes everything it emits. Verified:

  ```
  $ nxlang run fmt.nx
  <Box fit="Fit.cover" flag="true" neg="-1" w="1.5" />
  ```

  Feeding that output back in produces four errors — *expects Fit, found string*, *expects boolean,
  found string*, and two of *expects float64, found string*. Numbers, booleans, and null are already
  legal unbraced today and are quoted anyway; the enum needs the bare form this change adds; and the
  negative float needs the signed literal form, which is why the two halves belong in one change
  rather than two. The float must also be spelled `-1.0` rather than `-1`, since an integer literal
  does not widen at a `float64` site.

- **Formatting and display split by context.** `enum-values` currently requires first-party
  formatting or display of a member to use `DealStage.pending_review`. That stays true for hover,
  diagnostics, and value display, where the type name is the point. Source-position formatting
  emits the bare form.

- **Completion at property-value position.** Dropping the type name removes the `Fit.` prefix that
  made members discoverable by typing. The language service gains contextual member completion
  after `=` inside an opening tag, driven by the property's declared type.

- **Not a breaking change.** Every existing spelling keeps working; `{Fit.cover}` is still how a
  member is named inside an expression, and is still required there.

- **Deliberately deferred.**
  - **Swift-style leading dot inside braces** — `fit={isWide ? .cover : .contain}`. The one place
    the feature does not reach is a conditional value, where names revert to being variables. The
    leading dot would close that gap unambiguously, since no NX expression begins with `.`. Held
    pending evidence that the gap hurts in real catalogs; adding a second spelling costs more than
    the gap does until then.
  - **Integer-literal widening at float sites** — NXE2's change 2 also proposed that `w=120` bind at
    a `float64` site, and it is the remaining half of NXE8. That is numeric coercion, not name
    resolution and not grammar, so it is the one NXE8 item this change leaves open: after this
    change `x: float64 = -1.0` works and `x: float64 = -1` still does not. The formatter is made to
    emit `-1.0` rather than depend on widening.
  - **A primitive alternative in union position** (NXE2 change 3) and **bare serialization of
    payloadless cases** (NXE2 change 1). This change is source-only and does not alter any wire
    format. The strict split is designed to compose with both; design.md records the seam.

- **Out of scope: out-of-range integer literals.** `<div>{9223372036854775808}</div>` prints `null` —
  an integer literal exceeding its type is silently swallowed with no diagnostic, and negating it
  fails at *runtime* with *Type mismatch in negation: expected number, got null*. So
  `a: int64 = {-9223372036854775808}` cannot currently be written at all. This is a pre-existing
  literal-parsing bug rather than an unbraced-forms question and needs its own change; it is
  recorded here because the folding above is a prerequisite for fixing the `int64` minimum, which
  can only be range-checked correctly if the sign is folded before the magnitude is checked.

- **Out of scope: enum member casing.** The convention question raised by NXE17 — snake_case house
  style versus the camelCase of a mirrored CSS/SVG vocabulary — is independent. Contextual
  resolution is casing-agnostic: `fit=cover` and `fit=spaceBetween` resolve identically. Every enum
  in this change keeps its current casing, and the convention change is its own proposal.

## Capabilities

### New Capabilities
- `unbraced-literal-forms`: what may be written as a value without braces — the
  unbraced-is-a-literal invariant, the signed numeric literal form, and the bare contextual name
  with the positions it is legal in, its resolution rule against the expected type, the strict
  bare/quoted split and its relationship to the deserialization boundary, and the diagnostics for an
  unresolved bare name — plus the requirement that first-party NX-syntax value output emit those
  forms and round-trip. Named for the invariant rather than for the headline feature, because the
  invariant is what later changes to unbraced syntax will need to look up.

### Modified Capabilities
- `enum-values`: an enum member becomes referenceable without naming its enum type; the
  formatting-and-display requirement splits by context, so source-position output is bare while
  hover, diagnostics, and value display stay qualified.
- `discriminated-unions`: a payloadless union case becomes referenceable by contextual name at a
  typed site, and a bare name in match-pattern position resolves as a case of the scrutinee's union
  rather than as an unresolved identifier.
- `editor-language-service`: completions gain a property-value context, offering the members of the
  property's declared type after `=`.

## Impact

**Grammar and parsing**
- `crates/nx-syntax/grammar.js` — `rhs_expression` (line ~405) gains two alternatives, a
  contextual name and a signed numeric literal; the tree-sitter parser is regenerated once for
  both. Conflict risk is low: the contextual name is exactly one token and cannot extend across the
  following property name, an `if` fragment, `>` or `/>`; and a leading `-` after `=` is
  unambiguous because `markup_identifier` cannot begin with `-` and an unbraced value is never an
  expression context. `prefix_unary_expression` (~490) and the numeric literal token rules (~530)
  are not modified.
- `pattern` (line ~919) already admits a bare `qualified_name`, so bare patterns parse today and
  only their *resolution* changes.
- `crates/nx-syntax/src/{ast.rs,syntax_kind.rs}`, `node-types.json`.
- `nx-grammar.md` (`RhsExpression`, `Pattern`) and `nx-grammar-spec.md` (a new AST node plus a
  resolution paragraph beside the existing MemberAccess disambiguation at ~768).

**HIR and lowering**
- `crates/nx-hir/src/ast/expr.rs` — a new `Expr` variant for an unresolved contextual name.
  Reusing `Expr::Ident` is not viable: `Ident` resolves against lexical scope and would report an
  unresolved-name error before the expected type is ever consulted.
- `crates/nx-hir/src/lower.rs` — lowering of `rhs_expression` and of pattern expressions. A signed
  numeric literal folds to a negative `Literal` during lowering and needs no HIR node of its own,
  which is the whole difference in cost between the two halves of this change.

**Type system**
- `crates/nx-types/src/infer.rs` — the new node has no context-free type, so inference must defer
  it to the three `check_typed_binding` sites that already hold the expected type (~1602 element
  properties, ~1826 property defaults, ~581 match patterns). This is the one genuinely new shape in
  the checker, which is otherwise infer-then-compare; design.md covers the alternatives.
- Resolution must strip nullability (`Fit?` accepts `cover`) and reject every expected type that is
  not an enum or a union.

**Runtime and interpreter**
- `crates/nx-interpreter/` — a resolved contextual name evaluates to the same `Value::EnumValue` or
  union case value as the qualified form. No new runtime value kind.

**Serialization**
- None. The wire form of an enum member is already the bare authored string, and this change does
  not touch union case tagging.

**Tooling**
- `crates/nx-cli/src/format.rs` — `format_attribute_value` (~214) quotes every scalar it emits, so
  the whole attribute path is rewritten, not just the enum arm at ~224. Its test at ~305 covers only
  the non-attribute path and needs siblings.
- `crates/nx-language-service/src/lib.rs` — `completions` (~334) gains the property-value context;
  hover and go-to-definition must follow a bare name to its member declaration; rename must rewrite
  bare occurrences.
- `src/vscode/` — TextMate scoping for a bare value token, which is lexically distinguishable from
  a quoted one without type information.

**Docs and sources**
- `docs/drawn-ui-proposal-nx-enhancements.md` — NXE2 change 2 is written as the quoted form
  `fit="cover"` and needs amending to the bare form with the strict-split rationale; NXE8's
  `RhsExpression` relaxation is superseded and must say so.
- `docs/NX-Drawn-UI-MVP-Object-Model-Proposal.md` — the §8.1 worked example and Appendix A defaults
  (`units: GradientUnits = {GradientUnits.objectBoundingBox}`, `background: Paint = {Paint.none}`).
- `examples/nx/**` and `crates/nx-syntax/tests/fixtures/**` — new fixtures for the bare form and for
  signed literals; no existing fixture requires rewriting, since nothing is being removed. Existing
  braced negative defaults such as `= {-1.0}` stay valid and are left alone.
