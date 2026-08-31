## Context

The load-bearing facts about the current implementation:

- `EnumDef { name, visibility, members: Vec<EnumMember>, span }` and
  `UnionDef { name, visibility, base: Option<Name>, cases: Vec<UnionCaseDef>, span }` differ only in
  that a union may have a base and its cases may have fields. `EnumMember { name, span }` carries no
  ordinal and no backing value, so an enum is exactly a base-less union of fieldless cases.
- The duplication runs the full depth of the compiler: `Type::Enum` (25 sites) beside `Type::Union`
  (27) and `Type::UnionCase` (18); `enum_defs` beside `union_defs` in `crates/nx-types/src/infer.rs`
  (26 lines); `CodegenDeclarationKind::Enum` beside the union declaration kind;
  `CodegenExpressionKind::EnumMember` beside `CodegenExpressionKind::UnionCase`;
  `Value::EnumValue` (27 sites) beside `Value::Record`.
- The two runtime representations are not equivalent in quality. `Value::EnumValue` is a distinct
  variant and is unambiguous. A payloadless union case is `Value::Record { type_name: "U.c",
  fields: {} }`, which is byte-identical to an empty qualified record, so
  `payloadless_union_case` in `crates/nx-cli/src/format.rs:227` must guess — and guesses wrong for
  any empty qualified record, silently rewriting the value.
- The split is published, not internal. `crates/nx-cli/src/typegen/languages/csharp.rs:229` emits an
  NX enum as a CLR `enum` with a string wire-format converter; `:309` emits a union as
  `[JsonPolymorphic]` `abstract class` plus one `sealed class` per case. TypeScript mirrors this at
  `crates/nx-cli/src/typegen/languages/typescript.rs:284` and `:381`. `bindings/dotnet`'s
  `NxPolymorphicMessagePackFormatter` mediates the `$type` shape.
- `union_case: seq('|', name, ...)` in `crates/nx-syntax/grammar.js:188` requires a leading `|` on
  every case including the first. `enum_member_list` makes it optional. That difference is why
  `enum Status = a | b` reads well and `type Status = | a | b` reads heavier.

## Goals / Non-Goals

**Goals:**

- One declaration form for a closed set of named alternatives, and one runtime representation for a
  member of such a set.
- Generated and serialized output for a constant union is byte-identical to what the same
  declaration produced as an `enum`, so no host consumer of an existing enum is disturbed.
- The formatter's union-case heuristic is deleted rather than corrected.

**Non-Goals:**

- Backing values, explicit discriminants, or ordinals on cases. Adding them later is what would
  justify a second declaration form; this change asserts NX does not need one now.
- Changing how payload cases are declared, constructed, matched, or serialized.
- Nominal identity across module boundaries. That is `nominal-type-identity`, which should follow.
- Fixing the formatter's separate defect of dropping a property name when rendering a record-valued
  property in element form. That is independent of enums and unions.

## Decisions

### D1: `enum` is removed rather than kept as an alias

After unification the two keywords would mean exactly the same thing. No language in the survey
carries two synonymous forms: the ML family (OCaml, Haskell, Elm, PureScript, ReScript, Gleam) has
no `enum`; Rust, Swift, and Scala 3 use `enum` *as* the ADT keyword; F#, Kotlin, Java, and C# carry
both only because a platform enum exists at the ABI level, and F#'s two forms are distinguished
precisely by backing values, which NX has no notion of. Keeping `enum` as a synonym would also
contradict the one-spelling-per-concept rule the project adopted for primitive type names one commit
ago (`crates/nx-types/src/ty.rs`, "there are no aliases or synonyms").

`type` is the surviving keyword because NX already spells records, aliases, and unions with it.

*Alternative — keep `enum` as a constrained form* that rejects payload cases and `extends`. This
earns its keyword: it is an assertion the author wants enforced, not a synonym, and it makes the
host mapping syntactically stable, since a union cannot silently stop generating a C# `enum`. It was
rejected because the guarantee is cheap to lose and expensive to carry: adding a payload case is a
deliberate source edit, the resulting host-shape change surfaces as a compile error in generated C#
at every call site, and the cost is a second concept in the language, in the grammar, in
diagnostics, and in the docs, whose only power is to forbid something.

*Alternative — keep `enum` as a deprecated alias for a release.* There is no backward-compatibility
obligation (`AGENTS.md`), and a keyword removal is the change that becomes impractical once source
exists outside this repository.

### D2: A single-case union still requires the leading `|`

Making the leading `|` optional is what keeps migration to a literal `enum` → `type` substitution.
But `type_definition` is `'type' name '=' type` — a type alias — so with the leading `|` optional,
`type A = B` would be ambiguous between an alias to `B` and a single-case union with case `B`. Two
or more cases are unambiguous, because an alias's right-hand side is a single `$.type` and cannot
contain `|`.

So the leading `|` is optional only for a list of two or more cases, and required for a single-case
union. This is exactly F#'s resolution of the same ambiguity with type abbreviations, and it costs
nothing in practice: a single-case union is rare, and every declaration being migrated from `enum`
has at least two cases or is already suspect.

*Alternative — keep the leading `|` mandatory everywhere.* Zero grammar risk, and the migration
becomes `enum X = a | b` → `type X = | a | b`. Rejected because the added `|` is pure noise on the
most common declaration shape in the language, and it is the one thing that would make the removal
feel like a downgrade.

### D3: Constant-ness is a property of the declaration, not of the value

A case is **constant** when it declares no fields *and* its union declares no base. Everything else
is a **payload** case, including a fieldless case in a union that extends an abstract base — such a
case carries the base's fields at runtime, verified: `type Shape extends Base = | circle` evaluates
to `{"$type":"Shape.circle","color":"red"}`.

Constant-ness is therefore per-case with a union-level input, not per-union. A union with mixed
cases genuinely has two wire shapes, which is the point: the cost of the record representation lands
only on cases that need it.

*Alternative — make scalar shape a per-union property* (a union is scalar only if all its cases are
constant). It removes the mixed shape and keeps each union's host mapping uniform. Rejected on two
counts: it leaves a fieldless case in a mixed union as an empty dotted record, so the formatter
defect this change exists to dissolve would survive; and adding one payload case would reclassify
every other case's wire form, which is a far worse silent break than the `extends Base` case, where
the base genuinely gives every case fields.

### D4: The host mapping keys off constant-ness, and the .NET reader gains a scalar alternative

A **constant union** — all cases constant — generates the host language's closed constant type. In
C# that is a CLR `enum` with its authored-string wire format, from either generator. In TypeScript
it is two different things, because the two emitters have two different jobs, and that difference is
deliberate:

- **CLI type generation** emits a *type surface* — a module describing NX values to a host that
  receives them over the wire. Its output contains no runtime values at all: every declaration is an
  `interface` or a `type`, and the whole module erases. A constant union is the union of the
  authored string literals, `"light" | "dark"`, which is TypeScript's idiomatic closed set for
  exactly this role.
- **Executable codegen** emits *running code*. Its modules already carry functions and values, so a
  constant union is a frozen `as const` object plus a type derived from it, giving the emitted code
  named case values to reference.

The two are not in tension, because the derived type of the `as const` object *is* the string
literal union: `typeof ThemeMode[keyof typeof ThemeMode]` evaluates to `"light" | "dark"`. Assignment
from a bare string, narrowing, exhaustiveness, and error messages are identical under both — the sole
difference is whether the module also exports a value. So the question is never which type to emit;
it is only whether a types-only module should stop being types-only, and for the CLI generator the
answer is no.

This is also the reversible choice. Adding the value object to the type surface later is a
non-breaking addition; removing one after consumers import it is not. The trigger to revisit is a
TypeScript host that needs the cases at *runtime* — enumerating them for a property inspector or a
dropdown, or validating untrusted input at a trust boundary — because a hand-maintained case list
silently rots when NX gains a case. Until such a consumer exists, the union keeps the option open.

Note that C# and TypeScript differing here is localization rather than asymmetry: each language gets
its own idiomatic closed set. What C# does have and TypeScript does not is runtime enumeration, via
`Enum.GetValues`; that gap is the thing the trigger above would close.

A union with any payload case generates the existing
polymorphic hierarchy. Within that hierarchy a constant
case's wire form is a bare string, so generated C# exposes it as a singleton instance of its case
type and `NxPolymorphicMessagePackFormatter` accepts a bare string where it currently requires a
`$type` map. TypeScript expresses this natively as `"circle" | Shape_Square`.

This is the part of the change with real cost, and it is what the OCaml precedent does not have to
pay: OCaml represents constant constructors as immediates invisibly, whereas NX's value model is
observable to the formatter, JSON, MessagePack, FFI, and two generated languages. Deserialization in
both hosts is type-directed, so the reader always knows which union it is reading and can accept
either shape; that is what makes the mixed shape implementable rather than merely desirable.

### D5: The `enum` keyword stays reserved for a diagnostic

`enum` remains a recognized token in declaration position and produces an error naming the `type`
form to write. It is the word most authors will reach for, and a parse error at the keyword teaches
nothing. This costs one token and one diagnostic, not a concept.

### D6: The `enum-values` capability keeps its path

The keyword goes away; the concept does not. A closed set of named constants with a bare-string wire
contract recovered through the target type is exactly what a constant union is, and it is what the
C# and TypeScript generators still emit. The capability's requirements are removed and re-added under
names that say "constant case", rather than moved to `discriminated-unions`, because moving
requirements across capabilities loses their history for no behavioral gain. The capability's
`## Purpose` is updated in place after archive.

`record-type-inheritance` is deliberately **not** in this change even though its
*Only abstract records may act as base records* requirement lists "enums" among the kinds that
cannot be a base. Unions still cannot be a base — that is what the clause protects, and
`discriminated-unions` already covers it with *Union cannot be extended after declaration* — so the
word is stale terminology rather than a behavioral error, and copying a thirteen-scenario
requirement to fix one word costs more risk than it removes.

### D7: The value formatter emits every property in property position

Deleting `payloadless_union_case` (D3's consequence) is not by itself enough to make the round-trip
requirement pass, because it moves an empty qualified record out of attribute position and into
`format_nested_element`, whose `Value::Record` arm renders the value's *type* name and discards the
*property* name entirely. `<div data={<foo.bar />} />` stops formatting as `<div data=bar />` and
starts formatting as `<div><foo.bar /></div>` — a different wrong value. The formatter therefore has
to be fixed in this change, not after it.

The rule is that a field is always emitted as `key=<value>`, never as element body content and never
as an element named after the property. NX has no property-element syntax: an element body binds to
the single field marked `is_content` (`EffectiveRecordShape::content_property`), so a property name
written in body position has nowhere to go. `rhs_expression` already admits `$.element`, so a record
value needs no braces (`home=<Address city="Boston" />`); a list needs them
(`items={<Item /> <Item />}`). Both spellings type check today. The simple/complex split survives
only as a line-layout decision.

*Alternative — teach the formatter which field is the content property, so a UI tree keeps its
markup shape (`<div><span /></div>`).* Rejected on two grounds. First, the formatter holds only a
`Value`, and content-property-ness is schema information no `Value` carries; recovering it means
resolving a `type_name` string back to a declaration, which is precisely the name-based nominal
resolution `nominal-type-identity` exists to remove. Second, the body form is lossy by construction
— without the schema there is no way to tell which field a body bound to — so legibility and
round-tripping are in genuine conflict here, and `unbraced-literal-forms` chooses round-tripping. A
schema-aware pretty mode can be layered on a correct default later; it cannot be the default.

Two values have no NX spelling at all, and the fix exposes both rather than creating them. An empty
list has none — `items={}` is a syntax error, and today it is classified simple and emitted as
`items="..."`, a string. `Value::ActionHandler` has none — it prints a synthetic `<ActionHandler
component="..." ... />` that is not a real element. Both should fail loudly instead of emitting
output that cannot read back.

## Risks / Trade-offs

- **The wire shape of a fieldless case in a mixed or base-less union changes**, from
  `{"$type":"U.c"}` to `"c"`. This is the only externally visible behavior change for existing
  programs. → It is stated as **BREAKING** in the proposal, and the `runtime-output-format` delta
  pins both shapes with scenarios. Constant unions — the overwhelming majority of what was written
  as `enum` — are unaffected.
- **A grammar change to `union_case_list` risks parser conflicts** with `type_definition`. → D2
  keeps the two disjoint by case count; the single-case form is unchanged from today. The invalid
  fixtures should gain a case asserting `type A = B` still lowers as an alias.
- **The migration is mechanical but wide**: 7 `enum` declarations across 5 `.nx` files, 54 in Rust
  test sources, 17 in non-archive docs. → The byte-identical output test below makes the migration
  self-verifying rather than eyeballed.
- **Removing `enum` is a source-breaking change with no deprecation window.** → Accepted
  deliberately; D5's diagnostic is the mitigation, and no NX source exists outside this repository.
- **The formatter rewrite changes the shape of all nested value output, not just union cases.** →
  It is sequenced immediately after the runtime change (step 4a below) so a regression is
  attributable, and D7's rule is uniform rather than case-by-case, so there is one behavior to
  review rather than one per value kind.
- **`nominal-type-identity` is already written against the pre-unification model.** → Its D5 and task
  group 6 become unnecessary and should be deleted rather than reworked; with constant cases
  represented as scalars there is no runtime union-case discriminator left to add. That change should
  be updated after this one lands, not before.

## Migration Plan

Sequenced so each step is independently verifiable:

1. Grammar: optional leading `|` for multi-case lists, `enum` reserved for the D5 diagnostic,
   `enum_definition` removed. Regenerate the parser. Existing union fixtures must be unchanged.
2. Migrate every first-party `enum` declaration to `type`, with `enum` still lowering, and record the
   generated IR, JS, TS, C#, and schema output as the baseline.
3. HIR and types: `EnumDef`, `EnumMember`, `Type::Enum`, `EnumType`, and the `enum_defs` path are
   removed; one declaration form and one nominal kind remain.
4. Runtime: constant cases become scalar values; `build_union_case_value` decides shape by
   constant-ness; `payloadless_union_case` is deleted.
5. Formatter: every field is emitted in property position (D7). Steps 4 and 5 must land together —
   the round-trip test is red between them.
6. Codegen and typegen: union emission covers both shapes; the C# and TypeScript mappings key off
   constant-ness; the .NET polymorphic reader accepts a bare string.
7. Docs, examples, VS Code grammar and snippets.

The acceptance guard for steps 2–6 is that the output recorded in step 2 is **byte-identical** at the
end: rewriting an `enum` as a `type` must change nothing a consumer can observe.

## Open Questions

- Whether the diagnostic for `enum` should offer an automatic fix-it in the language service, or only
  name the replacement in the message. The message is required; the fix-it can follow.
- How an empty list should be spelled in NX source. `items={}` does not parse, and
  `braced-value-sequences` does not specify one, so a formatted empty list has no correct output.
  D7 makes the formatter fail rather than lie about it; the language question is separate from this
  change and should be filed on its own.
- Whether `Value::ActionHandler` should ever be formattable as NX source. It has no source spelling
  today; the round-trip requirement should say so explicitly rather than leave it looking covered.
- Whether `nx-value` and the FFI boundary need a distinct tag for a constant case, or whether the
  bare string plus the target schema is sufficient there as it is everywhere else. The existing enum
  contract says the latter; this change assumes it continues to hold and does not add a tag.
