# NX Language Enhancements Found by the NX UI Proposal

**Status:** Findings, not decisions
**Source:** Expressing the [NX UI MVP object model](NX-Drawn-UI-MVP-Object-Model-Proposal.md) in NX instead of TypeScript
**Verified against:** `nxlang` built from the `rename-primitive-types` working tree (`cargo build -p nx-cli`), August 21, 2026

Rewriting the proposal's object model into NX was the first time NX had been pointed at a
non-trivial external schema, so it doubles as a stress test of the language. This file records what
that surfaced, with a stable identity per finding so later work can refer to `NXE7` rather than
re-deriving it.

Every finding here was reproduced against the toolchain in this repository, not inferred from the
grammar. Each entry gives the reproducer and the compiler's verbatim message. Findings are numbered
`NXE1`…`NXE18` and the numbers are permanent; a finding that is fixed, rejected, or re-rated keeps
its number and changes status. `Open` means outstanding as written, `Revised` means the finding
stands but its assessment has changed, `Informational` means no enhancement is being asked for,
`Deferred` means the enhancement is wanted but deliberately out of scope for the MVP, and
`Accepted as-is` means the behavior is fine and is recorded only so the choice stays visible.

Related documents: the proposal's **Appendix B** states the same gaps from the object model's point
of view; [drawn-ui-proposal-review.md](drawn-ui-proposal-review.md) tracks findings about the
proposal itself under `RF*` numbers. The two series are independent.

## What already fits

Worth stating before the gaps, because it shaped the rest: `external component` is almost exactly a
generative-UI catalog entry — a component signature with no NX body, rendered by a host.
`abstract external component` plus `extends` expresses the proposal's `UiCommonProps` /
`GraphicsCommonProps` / `ShapeProps` bundles with no adjustment, and those bundles were designed
before anyone knew the construct existed. The `content` property is the other good fit: the wire
format's `children` becomes a declared, typed property, so "accepts many children"
(`content children: Element[]?`) and "accepts exactly one" (`content child: Element`) are different
signatures rather than a prose rule.

Discriminated unions also model `Transform` and `ImageSource` better than the TypeScript original
did, replacing a hand-written `type` / `kind` field with a real discriminator.

## Summary

| ID | Kind | Area | Status | Summary |
|---|---|---|---|---|
| [NXE1](#nxe1) | Language gap | Types | Revised | No map type — one MVP site NX would not author; real post-MVP. Carries a `V[K]` syntax sketch |
| [NXE2](#nxe2) | Language gap | Types | Revised | Two findings under one number; the cheap half was mis-filed. Carries a design sketch |
| [NXE3](#nxe3) | Language gap | Serialization | Partly resolved | Records and payload cases carry a `$type` tag; the key is fixed. Constant cases now serialize bare |
| [NXE4](#nxe4) | Language gap | Types | Open | Nullable is the only way to say "optional" |
| [NXE5](#nxe5) | Language gap | Components | Open | A derived component cannot override an inherited default |
| [NXE6](#nxe6) | Language gap | Defaults | Open | A default cannot reference a sibling property |
| [NXE7](#nxe7) | Language gap | Sequences | Open | There is no empty-sequence literal, in any position |
| [NXE8](#nxe8) | Language gap | Defaults | Partly resolved | Qualified names in defaults still need braces; integer literals do not widen to `float64` |
| [NXE9](#nxe9) | Language gap | Types | Open | No refinement, range, or pattern constraints |
| [NXE10](#nxe10) | Language gap | Modules | Open | Name qualification is exactly one segment deep |
| [NXE11](#nxe11) | Language gap | Elements | Informational | No flat, ID-addressed authoring form |
| [NXE18](#nxe18) | Language gap | Types | Deferred | No way to declare a value's textual form, so string-encoded scalars stay opaque |
| [NXE12](#nxe12) | Toolchain bug | Components | Open | Prop defaults on an imported external component are not applied at call sites |
| [NXE13](#nxe13) | Toolchain bug | Components | Open | Props inherited from an imported abstract external base are invisible at call sites |
| [NXE14](#nxe14) | Toolchain bug | Modules | Open | A union case cannot be reached through a wildcard import alias |
| [NXE15](#nxe15) | Docs drift | Grammar | Open | `external component` and `abstract component` are absent from the grammar documents |
| [NXE16](#nxe16) | Docs drift | Docs | Open | Published docs and a shipped example use syntax that does not parse |
| [NXE17](#nxe17) | Convention | Unions | Accepted as-is | The `snake_case` case-name convention conflicts with camelCase wire vocabularies |

---

## Language gaps

<a id="nxe1"></a>
### NXE1 — No map type, and no generics to build one

**Status:** Revised, August 20, 2026 — impact downgraded after checking where the model actually
needs a map. The gap is real; its MVP cost is not.

**Impact:** Low for the MVP. Real for the post-MVP catalogs the proposal defers in §11.

NX has records, sequences, unions, and enums. It has no map/dictionary type, and generic type
parameters are a post-1.0 roadmap item ([nx-planning-future.md](../nx-planning-future.md), "Version
1.2: Generic Type System"), so `Record<ElementId, Element>` cannot be written or approximated.

**Where the model needs one.** Exactly one site: `NxUiDocument.elements`, the flat
`ElementId → Element` map in §4.1. Nothing else in either catalog is map-typed. `props` looks like a
second candidate and is not — in NX those are declared properties on the `external component`, which
fits the model better than the JSON open object does.

**Why that site costs little.** NX source is the nested authoring form and carries no element IDs at
all ([NXE11](#nxe11)); the flat map is what a compiler emits. NX is therefore never asked to write
`elements`. The only reason to want `map<K, V>` today would be for the NX declarations to serve as
the normative schema of the wire envelope — and they cannot, because [NXE9](#nxe9) leaves JSON
Schema as the normative validator regardless. Appendix A's `elements: Element[]` is a lossy stand-in
(no key uniqueness, no O(1) lookup, `id` pushed onto the element) for a field NX has no occasion to
author.

**Where it becomes real.** Map-typed *properties* inside catalogs, all of which the proposal's §11
defers rather than rules out: a state or data model (A2UI's `dataModel` is an open JSON object),
themes and design tokens, localization string tables, and per-element extension metadata under a
reserved namespace. Each is a map a catalog would have to declare, and none has a workaround as
cheap as `Element[]`.

**Superseded claim.** The first version of this finding called it "the only finding that changes the
model rather than its notation." That is wrong: the model is unchanged, and the one affected field
is not one NX would author. [NXE2](#nxe2) is the finding with real MVP cost.

**See also:** [NXE11](#nxe11), the authoring-side consequence; [NXE9](#nxe9), which is why NX is not
the normative validator either way.

#### Possible enhancement: `V[K]` as a core indexed type

Recorded August 20, 2026. A sketch, not a decision.

Spell a map as a postfix index suffix carrying its key type, making it a core language type rather
than something generics supply later:

```nx
type DesignTokens = Color[string]
type Typography   = TypographySpec[TextVariant]

type Theme = {
  tokens: Color[string]
  overrides: Color[string]?
}
```

**Why postfix, and why core.** NX has never needed `Sequence<Color>` — `Color[]` is core syntax
because sequences are core, and the [sequences reference](src/content/docs/reference/concepts/sequences-and-objects.md)
says so outright: "Sequences are the primary collection type in NX." If maps are judged equally core,
the same reasoning gives them the same treatment, and generics are left to do what generics are
for — user-defined abstractions — rather than re-spelling a built-in.

The framing that makes it more than an analogy: under this syntax `[]` is not "the sequence suffix"
but *the index suffix with its key type elided*. A sequence is the case where the index is ordinal.

```nx
Color[]              // Color indexed by position
Color[string]        // Color indexed by string
Color[TextVariant]   // Color indexed by a constant union case
```

One concept, one bracket, two spellings. D reached the same place with `int64[string]`.

**Composition needs no new rules.** Suffixes already compose in source order
([nx-grammar.md](../nx-grammar.md): "`string?[]` means a list of nullable strings, while `string[]?`
means a nullable list of strings"), and an index suffix falls straight into that:

| Written | Means |
|---|---|
| `Color[string]?` | nullable map |
| `Color?[string]` | map to nullable `Color` |
| `Color[string][]` | sequence of maps |
| `Color[][string]` | map of sequences |

**Key types should be restricted** to `string`, the integer primitives, and constant unions. Not a
representational limit — a round-tripping one. JSON objects have string keys only, so an
unrestricted key type produces a declaration that silently cannot serialize. Constant-union keys stay
safe because NX serializes a constant case verbatim, which is what makes `TypographySpec[TextVariant]`
a realistic theme declaration rather than a curiosity.

**No new literal syntax.** Reuse the braced sequence literal with pair elements:

```nx
let tokens: Color[string] = { ("accent" "#5B5BD6") ("surface" "#ffffff") }
```

**Two prerequisites, both already open findings.**

- [NXE3](#nxe3) — a map must serialize as a plain JSON object (`{"accent": "#5B5BD6"}`), never as a
  `$type`-tagged record or an entries array. A map that inherits the universal `$type` tag cannot
  describe an external schema, which is most of the reason to add one.
- [NXE7](#nxe7) — `{}` must mean the empty map, and it is currently a syntax error. This matters more
  for maps than for sequences: an empty map is a very common initial value, and shipping a type whose
  most common starting value is unwritable would be a poor first impression.

**Costs, stated rather than glossed.** Deeply nested containers read worse in postfix than in angle
brackets: `Color[][string]` against `map<string, Color[]>` is not a close contest for a reader
meeting either for the first time. And `V[K]` is unfamiliar outside D, so the first encounter is a
guess for most people, even though it is learned in seconds.

**Rejected alternative: `map<K, V>`.** More legible on first sight, and it would ride on the generics
syntax the roadmap already commits to ([nx-planning-future.md](../nx-planning-future.md), "Version
1.2: Generic Type System"). Rejected because it only avoids duplicate spellings if generics never
also provide a `Map`, which is a strange thing to design around; because it makes a core type wait on
a post-1.0 feature; and because it parses worse — `Color[` followed by `]` or a type is a one-token
decision, while `Color[string][]` has no counterpart to the `>>` token-splitting problem in
`map<string, map<string, Color>>`, in a language where `<` already opens an element.

<a id="nxe2"></a>
### NXE2 — No literal types, so no scalar-or-structured unions

**Impact:** High, and mostly measured in tokens.

NX union cases are named, so a union cannot offer a bare scalar as one alternative and a record or
another shape as the next. Three MVP types depend on exactly that:

| Model type | JSON form | NX form |
|---|---|---|
| `Length` | `120`, `"auto"`, `"50%"` | three-case union; `<Length.px value={120.0} />` |
| `Insets`, `CornerRadii` | `20` (all sides) or a per-side record | record only; all four sides written out |
| `Paint \| "none"` | `"#fff"` or `"none"` | union cases `Paint.solid` and `Paint.none` |

`Paint` is genuinely better as an NX union — `"none"` stops being bolted onto each use site. The
other two are pure loss: `"padding": 20` becomes a four-field record, and that shorthand is not
cosmetic. It is a large share of the token cost of a generated layout, which is the proposal's own
§12.10 open question.

String literal union types are already on the roadmap as a 1.1 feature
([nx-planning-future.md](../nx-planning-future.md), "String Literal Union Types"). If they arrive,
`Length` and `Insets` should be revisited.

**Possible enhancement:** literal types in union position, plus the ability to mix a primitive
alternative with a record alternative in one union.

**Status:** Revised, August 20, 2026 — the finding stands, but it is two findings sharing one
number, and the cheaper half is the one the proposal actually needs. Everything below was checked
against `nxlang` at the `rename-primitive-types` working tree.

#### Two findings, one number

**(A) Untagged unions** — a union alternative that is a bare scalar. A type-system change.
**(B) Concise literal surface** — writing `20` or `"cover"` and getting a nominal type. A
binding-site change with no type-system implications.

The finding above is written as (A), but its impact line — "mostly measured in tokens" — is
describing (B). The distinction matters because (B) is cheap and general and (A) is expensive and
narrow. NX's tagged union already models `Length` *correctly*; what costs tokens is writing it and
serializing it.

#### What NX already has

NX has string literal types today. A union whose cases carry no payload is one:

```nx
type Fit = fill | contain | cover
```

generates `export type Fit = "fill" | "contain" | "cover"`, and
[enum-values/spec.md](../openspec/specs/enum-values/spec.md) requires the case to serialize as
"the bare authored case name", with the consumer recovering identity from the target type. So
`Fit` on the wire is `"cover"` — untagged, exactly the JSON Schema `enum` shape.

**The asymmetry this section was written about is gone.** It described two declaration forms —
`enum Fit = ...` and `type Fit = | ...` — carrying identical information and serializing
differently, the second as `{"$type": "Length.auto"}`. `replace-enums-with-unions` removed the
`enum` keyword and made the union form serialize bare, so there is now one declaration form and one
encoding. Which form the author reached for is no longer a question.

What remains is the parser's constraint on what a case can *be*: `type UnionName [extends
AbstractRecord] = caseName | payloadCase { prop:type }`. `type L = string | int`, `| "auto" | int`,
`F | Px`, and `| px float64` are all still rejected. A case is a name, optionally with fields — it
is never a bare scalar.

#### Precedent

| Language | Literal types? | Scalar-in-union? | How it gets the concise form |
|---|---|---|---|
| TypeScript | yes, plus template literals | yes, untagged, structural narrowing | the type system does it directly |
| Scala 3 | yes (singleton types) | yes (`A \| B`), erased, needs type tests | a nominal language that added both |
| Rust | no | no — variants always named | `serde(untagged)`, try-each-in-order at the boundary |
| Swift | no | no | `ExpressibleByIntegerLiteral`; implicit member expr `.cover` |
| C# / XAML | no | no | `TypeConverter`, implicit operators, `IParsable<T>` |
| GraphQL | enums only | **no, by design** — unions are object-only | custom scalars: string on the wire, server parses |
| Protobuf | no | no — `oneof` always tagged | canonical string JSON on well-known types (`"3s"`) |
| CSS Houdini | — | — | `@property { syntax: "<length-percentage>" }` |

Two clusters. TypeScript and Scala put it in the type system. Everyone else — including both schema
languages, GraphQL and Protobuf — keeps named cases and solves conciseness at the serialization
boundary. NX is a schema language, so the second cluster is the relevant precedent set.

Rust is a warning rather than a model: `#[serde(untagged)]` attempts each variant in order and keeps
the first that succeeds, which produces famously unusable diagnostics. Whatever NX does, it should
not be try-in-order.

#### Recommended changes

**1. Constant union cases serialize bare.** ✅ **Implemented**, as the
`replace-enums-with-unions` change. `Length.auto` is `"auto"` rather than `{"$type": "Length.auto"}`,
and deserialization stays unambiguous: at a `Length`-typed field a bare string can only be such a
case. This gets `Paint | "none"` to the exact JSON the proposal wants.

**Shipped narrower than sketched, and deliberately.** The rule is not "payloadless" but *constant*:
a case that declares no fields **in a union that declares no base**. A fieldless case of a union
that extends an abstract base is not constant, because it carries the base's fields at runtime —
`type UiEvent extends EventBase = clicked { x: int } | closed` evaluates `closed` to
`{"$type": "UiEvent.closed", "source": "ui"}`, and there is nothing bare about a value with fields
in it. Reading "payloadless" literally would have dropped those fields. Nothing in this proposal's
Appendix A declares a base on a closed-set type, so every one of them is constant and gets the bare
form.

**2. Contextual literal binding at typed sites.** ✅ **Implemented**, as the
`contextual-literal-binding` change. Where the expected type is known — and in NX it always is, at
props, fields, annotated `let`s, and match patterns — a bare name resolves against it:

```nx
<Img fit=cover />          // was: fit={Fit.cover}
```

Swift's implicit member expression, or C#'s target-typed `new`. Source-only, applies to every
closed set in every NX program, and it makes source agree with serialization — the wire already said
`"cover"` while the source insisted on `{Fit.cover}`.

**Shipped unquoted rather than quoted.** This sketch originally spelled it `fit="cover"`. The bare
form was chosen instead, under a **strict split**: a bare name resolves *only* against the closed
nominal set — the cases of the union that types the site — and a quoted string resolves *only* as
string data, with no fallback between the two in NX source. That keeps both spellable once change 3
lands (`fill=none` is the case, `fill="none"` is a colour whose text is `none`), and it removes the
"unspellable `none`" cost the amended rule below concedes. It also closes a schema-evolution hazard
the quoted form has: under fallback, adding a payloadless case named `currentColor` to a union
would silently reinterpret every existing `fill="currentColor"` from string data into that case,
changing the serialized shape of documents nobody edited.

The closed-set-then-fallback rule below still governs the **JSON deserialization boundary**, where
no bare/quoted distinction exists. The two rules differ because source carries a lexical signal that
JSON does not.

**Numerics were not part of it.** Applied to numerics this would also subsume [NXE8](#nxe8), but
integer-literal widening is numeric coercion rather than name resolution and remains open; see NXE8
for what did and did not ship.

**3. One primitive alternative per union, partitioned by JSON kind.** The only real type-system
addition, under a rule that keeps it decidable:

```nx
type Length =
  | float64                     // number
  | auto                    // string
  | percent { value: float64 }  // object
```

Partition the cases by JSON kind — number, string, boolean, object — and reject the *declaration* if
two cases collide. Resolution is then a single switch on the token kind, never try-in-order, and the
error surfaces once at the declaration rather than at every value.

**4. Positional payloads on union cases** — `| px(float64)`, Rust's tuple variant. Nice-to-have. It
shortens construction but does not reach a bare `120`, so it solves nothing on its own.

#### Two problems found by writing the use sites

**`| float64` already parses, as a case named `float64`.** The declaration in change 3 compiles clean today
and generates `interface LengthFloat64 extends NxRecord<"Length.float64"> {}` — silently wrong, no
diagnostic. Bare lowercase identifiers in case position are case names, and NX's primitive type
names are lowercase identifiers. The collision set is closed and tiny (`string int int32 int64 float32
float64 boolean void object`), so the fix is a rule, not syntax: *a union case may not be named after a
primitive type, and a bare primitive name in case position is a type alternative*. The marked form
`| (float64)` is the alternative if named types should be admissible as alternatives later.

**The string slot needs two tiers.** Strict one-alternative-per-kind breaks on the first real type.
`Length` wants `120`, `"auto"`, and `"50%"` — one number and *two* strings. `Paint` wants
`"#5B5BD6"`, `"none"`, and gradient objects. Amended rule:

- **Across kinds:** at most one alternative per JSON kind, where all payloadless cases collectively
  count as *one* string alternative.
- **Within the string kind:** payloadless case names form a closed set and resolve first; **one**
  open-ended string alternative may follow as a fallback.

Still deterministic and O(1) — a set lookup with a fallback. The only cost is that a color literally
named `"none"` becomes unspellable, which is the real CSS situation and why `none` is a `Paint` case
to begin with.

#### Worked example

The proposal's §8.1 document, with changes 1–3 applied:

```nx
<ui.Card width=360 padding=20>
  <ui.VStack gap=12>
    <ui.Text: variant=h2>NX UI</ui.Text>
    <gfx.Drawing height=140 viewBox=<ViewBox x=0 y=0 width=320 height=140 /> >
      <gfx.Rect
        width=320 height=140 rx=12
        fill=<Paint.linearGradient
                x1=0 y1=0 x2=1 y2=1
                stops={ <GradientStop offset=0 color="#5B5BD6" />
                        <GradientStop offset=1 color="#14B8A6" /> } /> />
      <gfx.Path
        data="M20 105 C90 15 210 125 300 35"
        fill=none
        stroke=<Stroke paint="white" width=5 lineCap=round /> />
      <gfx.Circle cx=300 cy=35 r=7 fill="white" />
    </gfx.Drawing>
    <ui.Text: variant=caption>Portable layout above; portable drawing below.</ui.Text>
  </ui.VStack>
</ui.Card>
```

956 → 760 characters against the version in §8.1, 21% off the source — markup only, imports
excluded, and measured against §8.1 as current NX spells it, with literals and single elements
unbraced. What is left is what changes 1–3 buy. On the wire a representative element goes from 161
to 56 bytes:

```json
{"$type":"Box","width":{"$type":"Length.px","value":120},"height":{"$type":"Length.auto"},"padding":{"$type":"Insets","top":20,"right":20,"bottom":20,"left":20}}
{"$type":"Box","width":120,"height":"auto","padding":20}
```

Two lines exercise the whole rule between them. `fill=none` is bare, so it resolves only against
the closed set, where `none` is a payloadless `Paint` case. `paint="white"` is quoted, so it is
never a case name and can only be the open `Color` alternative. Same type, two spellings, two
resolution paths, both decided at compile time, and neither able to shadow the other — this is the
strict split that shipped, not the closed-set-then-fallback rule the sketch above started from.
That fallback rule still governs the JSON deserialization boundary, where no bare/quoted
distinction exists.

The `TextVariant` import also disappears from the document header, because change 2 removes the need
to name the union type at all. That incidentally routes around [NXE14](#nxe14) for every
closed-set-valued prop.

#### Scope: percentages are out

An earlier version of this sketch spelled the third `Length` case as a refined string,
`| string matching "^-?[0-9.]+%$"`. That is wrong, and [NXE18](#nxe18) says why: a refinement
validates, it does not convert, so the `%` would be left for every consumer to strip separately.
`Length` keeps `percent { value: float64 }` for the MVP. `120` and `"auto"` are the high-frequency
cases and changes 1–3 deliver both.

#### On the typeconverter alternative

The XAML route — every value is a string, converted by the host — works in NX today with no language
change at all: `type Color = string` in Appendix A is exactly a typeconverter with no converter, and
`<Box width="50%" />` against a string alias type-checks clean. Its advantage is real and proven at
scale in CSS, XAML, and HTML: tooling drives completion from the type.

Two verified costs. `width="fifty bananas"` also type-checks clean — no compile-time signal, which
is the thing NX otherwise offers over JSON Schema. And `width={120}` is *rejected* at a
string-typed prop, so numbers must be quoted and `"padding": 20` becomes `"padding": "20"`,
collapsing a distinction JSON already has in a format that is counting tokens.

The deeper objection is where the metadata lives. In XAML the converter is imperative C# in a
separate assembly, reachable only by a .NET host with reflection; VS Code's HTML completion has the
same problem and solves it with a hand-maintained data file duplicating the spec. For a catalog
whose consumer is a model reading the `.nx` file, a value grammar outside the schema is
disqualifying — the catalog stops being self-describing. Houdini's
`@property { syntax: "<length-percentage>" }` is the same idea done right, because the grammar is in
the stylesheet.

So the typeconverter route is not an alternative to this finding; it is [NXE9](#nxe9) (constrained
string types, declared in NX) for validation and [NXE18](#nxe18) for conversion. The dividing line
is in NXE18.

<a id="nxe3"></a>
### NXE3 — Every record and union case carries a `$type` tag, and the key is fixed

**Impact:** High for any token-sensitive format.

NX serialization gives every record a `$type` discriminator, not only polymorphic ones. Confirmed
by generating types for the proposal's `nx/core` library:

```ts
// generated _nx.ts
export interface NxRecord<TType extends string = string> {
  $type: TType;
}

// generated types.ts
export interface Point extends NxRecord<"Point"> { x: number; y: number; }
export interface PaintSolid extends NxRecord<"Paint.solid"> { color: Color; }
```

So a `Point` is `{"$type": "Point", "x": 10, "y": 20}` and a solid fill is
`{"$type": "Paint.solid", "color": "#fff"}` rather than `"#fff"`. The key is `"$type"` in both the
TypeScript and MessagePack paths
([`NxPolymorphicMessagePackFormatter.cs`](../bindings/dotnet/src/NxLang.Sdk/Serialization/NxPolymorphicMessagePackFormatter.cs)
hard-codes `private const string DiscriminatorKey = "$type";`) and is not configurable.

**Impact on the proposal:** the NX-native encoding is materially heavier than the hand-written JSON
in the proposal's §8. The conclusion recorded there is that NX source and NX serialization are
separable decisions — adopting the language does not oblige the wire format to adopt its encoding.

**Possible enhancement:** omit `$type` for non-polymorphic records whose type is statically known
from the field they occupy; and make the discriminator key configurable per library or per
declaration.

<a id="nxe4"></a>
### NXE4 — Nullable is the only way to say "optional"

**Impact:** Low, but it touches nearly every declaration.

NX has no distinction between "property absent" and "property present and null". A property with no
meaningful default is therefore written `T?`, and the consumer resolves null. Most of the `?` marks
in the proposal's §6 and §7 exist for this reason, not because null is a meaningful value there.

**Possible enhancement:** an explicit optional marker distinct from nullable, or a documented rule
that null round-trips as omission.

<a id="nxe5"></a>
### NXE5 — A derived component cannot override an inherited default

**Impact:** Medium.

Redeclaring an inherited property is rejected as a duplicate. From
[`component-contract-inheritance/spec.md`](../openspec/specs/component-contract-inheritance/spec.md):

> **Scenario: Duplicate inherited prop name is rejected** — analysis SHALL reject `NxSearchUi`
> because `placeholder` duplicates an inherited component prop

**Impact on the proposal:** §6.1 wants `fill` to default to `Paint.none` on `Line` and `Polyline`
and to black on closed shapes and `Path` — a per-component default over a shared property. Not
expressible. `fill` is nullable on `ShapeCommon` and the per-component default becomes renderer
prose that the type system cannot check.

**Possible enhancement:** allow a derived component to restate an inherited property for the sole
purpose of narrowing its default, while still rejecting a changed type or a second content marker.

<a id="nxe6"></a>
### NXE6 — A default cannot reference a sibling property

**Impact:** Low.

A property default is a literal or a constant expression and cannot read another property of the
same declaration. Component `state` defaults *can* read props — see
[`component-syntax/spec.md`](../openspec/specs/component-syntax/spec.md), "Initialization applies
prop and state defaults once" — but property defaults cannot.

**Impact on the proposal:** two "defaults to its neighbour" relationships are lost.
`Transform.scale.y` defaulting to `x` (uniform scale) and `nx.graphics.Rect.ry` defaulting to `rx`
both become nullable, with the relationship demoted to a comment.

**Possible enhancement:** allow a property default to reference an earlier-declared property in the
same signature, resolved in declaration order — the rule `state` already uses.

<a id="nxe7"></a>
### NXE7 — There is no empty-sequence literal, in any position

**Impact:** Medium, and broader than it first appears.

The braced form is the only sequence literal, and it must contain at least one item. Bracket-list
literals do not parse anywhere — not in a `let`, not in a `for`, not in a default.

```nx
let nums: int32[] = {1 2 3}      // OK
let nums: int32[] = {}           // error: Expected identifier here
let nums: int32[] = [1, 2, 3]    // error: Syntax error
let nums = for f in ["a", "b"] { f }   // error: Syntax error
```

**Impact on the proposal:** every `T[]` property whose natural default is "empty" is written `T[]?`
instead — `transform`, `shadows`, `dashArray`, `rows`, and both `children` properties. The nullable
workaround also drags in [NXE4](#nxe4): the reader cannot tell "no shadows" from "shadows not
specified".

**Possible enhancement:** accept `{}` as the empty sequence. Separately, decide whether bracket-list
literals are meant to exist at all — see [NXE16](#nxe16), because the published docs say they do.

<a id="nxe8"></a>
### NXE8 — Qualified names in defaults need braces, and integer literals do not widen to `float64`

**Status:** Partly resolved, by the `contextual-literal-binding` change. The aggregate-noise half of
this finding is gone.

**Impact:** Low. What remains touches only defaults written in qualified form, which authors now
have no reason to write.

An unbraced value position takes a literal, and a bare name at a typed site resolves against the
declared type, so a negative number and a case name are both fine unbraced. Only a *qualified* name
still has to be wrapped:

```nx
external component <C x: float64 = -1.0 />                // OK — signed literals are literals
external component <C a: Alignment = start />             // OK — resolves against Alignment
external component <C a: Alignment = Alignment.start />   // error: Invalid component definition
external component <C a: Alignment = {Alignment.start} /> // OK
```

Separately, integer literals do not widen:

```nx
type Insets = { top: float64 = 0 }
// error: Default value for record property 'top' expects float64, found int
```

**Impact on the proposal:** Appendix A writes its closed-set defaults bare (`lineCap: LineCap = butt`),
which is now correct and reads better than the braced form this finding originally required. What is
left is that every `float64` default must be written `0.0` / `1.0` / `4.0`; that is pure ceremony in a
schema listing, and the `float64 = 0` error is the kind of thing that will hit every newcomer once.

**Possible enhancement:** widen integer literals to float64 in a float64-typed position; and admit bare
member access and prefix-negated literals as `RhsExpression`.

**Status:** partly resolved by the `contextual-literal-binding` change.

- ✅ **Prefix-negated literals shipped.** `x: float64 = -1.0` and `<C x=-1.5 />` now parse.
  `SignedNumericLiteral` was added to `RhsExpression` *and* to `Pattern`, which had the same hole —
  `if n is { -1 => ... }` was a parse error too. Tokenization is unchanged and `-` remains a prefix
  operator in expressions, so `a-1` is still subtraction; lowering folds `-` applied directly to a
  numeric literal in every position, so `-1.0`, `{-1.0}`, and the `-90` in `{-90 + rotation}` share
  one lowered representation.
- ❌ **Bare member access was deliberately not adopted.** Admitting `a = Alignment.start` unbraced
  would also admit `a = obj.field`, breaking the invariant that an unbraced value is a literal and
  never an expression. Contextual literal binding ([NXE2](#nxe2) change 2) delivers the same
  ergonomic win — `a: Alignment = start` — without it.
- ⏳ **Integer widening remains open, and now reaches further.** `x: float64 = -1` still errors
  while `x: float64 = -1.0` works. That is numeric coercion rather than grammar or name resolution,
  and it is the one item of this finding still outstanding. It used to bite only record and union
  case defaults, because a component's prop and state defaults were not type checked at all; they
  are now, so `external component <C x: float64 = 0 />` is rejected where it used to be silently
  accepted. The rule did not change — the set of places it is enforced did, and the bare
  closed-set defaults Appendix A writes (`lineCap: LineCap = butt`) are now verified rather than
  merely parsed.

A related pre-existing bug surfaced while implementing the above and is **not** fixed: an integer
literal that exceeds its type is silently swallowed to `null` with no diagnostic — `{9223372036854775808}`
evaluates to `null`, and negating it fails at runtime with *Type mismatch in negation: expected
number, got null*, so `int64`'s minimum cannot currently be written. It needs its own change; the
folding above is a prerequisite, since the sign must be folded before the magnitude is range-checked.

<a id="nxe9"></a>
### NXE9 — No refinement, range, or pattern constraints

**Impact:** Medium. It decides what the NX listing *is*.

The model carries constraints that NX's type system cannot see: `ElementId`'s
`^[A-Za-z_][A-Za-z0-9_.-]{0,63}$`, `opacity` in 0..1, `fontWeight` in 1..1000, `columnSpan >= 1`,
"at least two gradient stops in non-decreasing offset order", "width and height must be positive".

**Impact on the proposal:** all of it stays in the JSON Schema, which means the schema — not the NX
declarations — remains the normative validator. The NX listing is the shape; the schema is the
contract. That split is workable but it is a real limit on how much NX can own.

**Possible enhancement:** annotations on type aliases and properties that survive into generated
schemas (`@pattern`, `@range`, `@minItems`), even if the NX type checker only checks them for
literals.

<a id="nxe10"></a>
### NXE10 — Name qualification is exactly one segment deep

**Impact:** Medium.

Both import forms cap qualification at one prefix segment:

```nx
import "../ui" as nx.ui
// error: Syntax error (the wildcard alias is a single Identifier)

import { VStack as nx.ui.VStack } from "../ui"
// error: Selective import alias 'nx.ui.VStack' must contain exactly one dot
```

**Impact on the proposal:** the catalog type names are `nx.ui.VStack` and `nx.graphics.Rect`, which
are not reachable as NX prefixes. NX source uses a one-segment local alias (`ui.VStack`), and the
mapping from alias to real catalog ID lives in the document's `catalogs` list. Workable, but it
means NX cannot mirror a reverse-DNS namespace, which most catalog and package ecosystems use.

**Possible enhancement:** allow multi-segment aliases in both import forms.

<a id="nxe11"></a>
### NXE11 — No flat, ID-addressed authoring form

**Impact:** Medium. Follows from [NXE1](#nxe1) but is worth its own identity.

NX elements nest. There is no way to write a flat map of `id → element` with `children` as ID
references, so NX cannot express the proposal's canonical interchange form at all.

**Impact on the proposal:** this settles a question §3 had left open. NX is the *authoring* syntax,
and lowering to the flat map is a compilation step. The flat map remains the interchange and
validation form. That is a coherent outcome rather than a problem — but it does mean an NX-authored
document cannot be streamed or patched by ID without going through the compiler first.

**Possible enhancement:** none needed for the proposal. Listed so that a future streaming or
JSON-Patch design does not assume NX source can express element identity directly.

<a id="nxe18"></a>
### NXE18 — No way to declare a value's textual form, so string-encoded scalars stay opaque

**Impact:** Medium, and deliberately deferred. Recorded because [NXE2](#nxe2) and [NXE9](#nxe9) both
stop at this line and neither one can cross it.

**The principle.** *A string-typed value is fine when the host treats it as opaque. It is a problem
when the host must compute with it.*

`Color` is opaque. Appendix A declares `type Color = string` with the comment "CSS Color syntax
subset accepted by the host", and nothing in layout does arithmetic on `"#5B5BD6"` — it reaches the
renderer and becomes a paint. Validation is the whole job there, so [NXE9](#nxe9) is genuinely
sufficient.

`Length` is computed. `"50%"` has to become `0.5 × parentWidth` during layout, so the number must
come out of the string. A validator is structurally the wrong tool: the question is not whether the
string is well-formed, it is what `float64` it denotes.

That line, not "text versus choice", is what decides when a string-typed value is acceptable.

#### What a refinement leaves undone

| Layer | `percent { value: float64 }` | a refined string `"^-?[0-9.]+%$"` |
|---|---|---|
| NX source | `<Length.percent value=50.0 />` | `width="50%"` |
| Type check | `float64`, checked | string shape, checked |
| JSON | `{"$type":"Length.percent","value":50}` | `"50%"` |
| Generated TS | `{ value: number }` | `string` |
| Host | reads `.value` | **calls `parseFloat` itself** |

The `%` is stripped by whoever needs the number, which is every consumer, separately. This is
GraphQL custom scalars exactly, and its documented cost: each client re-implements the parse, and
codegen cannot help because the generated type is `string`.

#### Possible enhancement: a bidirectional format template on a union case

```nx
type Length =
  | float64
  | auto
  | percent { value: float64 } as "{value}%"
```

Serialize: emit `"50%"`. Deserialize: match the literal segments and parse the holes at their
declared types. Generated TypeScript gets `{ value: number }` while the wire stays `"50%"`.
Decidable if the template is required to have no two adjacent holes.

This is the XAML typeconverter idea, relocated: the converter belongs to a *union case* and is
declared *in NX*, rather than being attached to a type and written in C#. That keeps the catalog
self-describing, which is the property that matters when the consumer is a model.

#### Why it is not part of NXE2

The simple version does not generalize to units. A CSS-style `Length` is `50%`, `20px`, `1.5em`,
`100vw`, and one templated case per union buys exactly one unit. Making the unit a hole —

```nx
| dimension { value: float64 unit: LengthUnit } as "{value}{unit}"
```

— has adjacent holes, so it is ambiguous, and it needs case names spelled `%` and `vw`, which are
not identifiers. That runs straight into [NXE17](#nxe17), the case-spelling-versus-wire-vocabulary
conflict. A real dimension type therefore needs templates *plus* explicit wire spellings on union
cases, which is a much larger feature than it looks and should not ride along on NXE2.

---

## Toolchain bugs

These three are implementation gaps rather than language limits. Together, [NXE12](#nxe12) and
[NXE13](#nxe13) make the proposal's three-library layout uncheckable today: each library type-checks
on its own, but a document that imports two catalogs cannot be checked against them. A
single-library variant of the same catalog plus the same document checks end to end.

<a id="nxe12"></a>
### NXE12 — Prop defaults on an imported external component are not applied at call sites

**Reproducer:**

```nx
// lib/a.nx
export abstract external component <Base fill: string? width: float64 = 1.0 />
export external component <Rect extends Base x: float64 = 0.0 y: float64 />

// app/main.nx
import "../lib" as lib
let r = <lib.Rect y={5.0} />
```

```
error: Element 'lib.Rect' requires property 'x'
```

`x` is declared `= 0.0`. The identical declarations in a single library check cleanly, so this is
specific to the library boundary.

<a id="nxe13"></a>
### NXE13 — Props inherited from an imported abstract external base are invisible at call sites

**Reproducer:** same two libraries as [NXE12](#nxe12).

```nx
import "../lib" as lib
let r = <lib.Rect x={0.0} y={5.0} width={2.0} fill={"red"} />
```

```
error: Element 'lib.Rect' has no property 'width'
error: Element 'lib.Rect' has no property 'fill'
```

Both are declared on `Base`, which `Rect` extends.
[`component-contract-inheritance/spec.md`](../openspec/specs/component-contract-inheritance/spec.md)
requires the effective prop set to include the whole base chain, and its "Imported abstract
component can be extended" scenario covers the cross-library case for `extends` itself — so the
declaration side works and only the invocation side is missing.

<a id="nxe14"></a>
### NXE14 — A union case cannot be reached through a wildcard import alias

**Reproducer:**

```nx
import "../ui" as ui
// ui.TextVariant       resolves
// ui.TextVariant.h2    error: Member access not yet implemented
```

The third qualifying segment is unsupported. The workaround is the selective import form, which
brings the union in unqualified:

```nx
import { Card as ui.Card, Text as ui.Text, TextVariant } from "../ui"
```

A wildcard import of the same library cannot be added alongside it, because importing one library
path twice in a file is an error — so a module that needs both aliased components and a union case
from one library must use the selective form for everything it takes from that library. The
proposal's §8.1 example does exactly this, and imports its two catalogs in two different styles as a
result.

---

## Documentation drift

<a id="nxe15"></a>
### NXE15 — `external component` and `abstract component` are absent from the grammar documents

[nx-grammar.md](../nx-grammar.md) defines `ComponentDefinition` with no `abstract` or `external`
modifier, and [nx-grammar-spec.md](../nx-grammar-spec.md) — the machine-readable version described
as being "used for AI code generation" — does not mention `external` at all. Both modifiers are
specified in [`openspec/specs/component-syntax/spec.md`](../openspec/specs/component-syntax/spec.md)
and implemented.

This is the finding with the widest blast radius outside this proposal: `external component` is the
construct that makes NX suitable for host-rendered catalogs, and an agent or a person working from
the grammar documents would never learn it exists. The parser's own error message already knows the
full form:

```
note: Expected: [abstract] [external] component <Name [extends BaseComponent] prop:type
      emits { ActionName { prop:type } ActionType } /> [= { state { prop:type } [<Element />] }]
```

**Fix:** update `ComponentDefinition` in both grammar documents to match.

<a id="nxe16"></a>
### NXE16 — Published docs and a shipped example use syntax that does not parse

Three forms appear in the documentation site but are rejected by the compiler:

| Form | Where | Result |
|---|---|---|
| `let empty: string[] = []` and `[1, 2, 3]` | [sequences-and-objects.md:15](src/content/docs/reference/concepts/sequences-and-objects.md), `examples/nx/complex.nx:80` | `error: Syntax error` |
| `type <User id:string name:string/>` | [sequences-and-objects.md:45-46](src/content/docs/reference/concepts/sequences-and-objects.md) | `error: Syntax error` |
| `type NameLookup = (string, User)[]` | [types.md:76](src/content/docs/language-tour/types.md) | `error: Syntax error` |

The element-style record declaration and tuple types are both listed as future features
([nx-planning-future.md](../nx-planning-future.md), "Version 1.5: Tuple Types"), so the docs are
describing an intended language rather than the current one. Bracket-list literals are the more
confusing case, because they are neither in the EBNF nor implemented, yet they appear in a shipped
example.

`examples/nx/complex.nx` also fails independently at line 70 on `<:>completed</>`.

**Fix:** decide per form whether it is planned or abandoned, then either implement it or correct the
docs and the example. See [NXE7](#nxe7) for the sequence-literal question specifically.

---

## Convention

<a id="nxe17"></a>
### NXE17 — The `snake_case` case-name convention conflicts with camelCase wire vocabularies

NX documents `snake_case` as the case-name convention and serializes case names verbatim, so
`pending_review` stays `"pending_review"` on the wire. That is the right default when NX owns both
ends.

It is the wrong default when NX describes someone else's vocabulary. The proposal's closed-set
values — `spaceBetween`, `objectBoundingBox`, `scaleDown`, `userSpaceOnUse` — come from CSS and SVG
and cannot be renamed. camelCase case names are legal NX identifiers, so Appendix A keeps the wire
spelling and diverges from the convention deliberately.

**Not a defect.** Recorded so the choice is visible, and so the style guidance can acknowledge that
a schema mirroring an external vocabulary should follow that vocabulary rather than NX house style.
