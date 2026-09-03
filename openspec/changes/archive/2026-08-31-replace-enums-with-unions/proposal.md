## Why

NX has two declaration forms for one concept. `enum Fit = fill | contain | cover` and
`type Fit = fill | contain | cover` describe the same thing — a closed set of named constants —
but they are separate keywords, separate HIR nodes (`EnumDef` / `UnionDef`), separate types
(`Type::Enum` / `Type::Union`), separate runtime values (`Value::EnumValue` / `Value::Record`), and
separate host mappings. `EnumMember` carries only a name and a span: no ordinal, no backing value,
no payload. An NX enum is *exactly* a union whose cases have no payload.

The duplication costs correctness, not only code. A payloadless union case is an empty
`Value::Record` with a dotted type name — byte-identical to an empty qualified record — so
first-party formatting renders `<div data={<foo.bar />} />` as `<div data=bar />`, a different value
that does not read back. An enum member has none of this trouble, because it is its own value kind.
One concept, two representations, and the second one is lossy. This is RF2 in
`contextual-literal-binding`'s `review.md`, and it dissolves once there is a single representation
rather than needing a discriminator bolted onto records.

No language in the ML family carries both forms. OCaml, Haskell, Elm, PureScript, ReScript, and
Gleam have no `enum`; Rust, Swift, and Scala 3 use `enum` *as* the algebraic-data-type keyword.
The languages that do carry both — F#, Kotlin, Java, C# — do so because a platform enum exists at
the ABI level, and the distinguishing feature is a backing value. NX has no backing values, so it
has no reason for the second form. Keeping `enum` as a synonym for `type` would also contradict the
one-spelling principle the project adopted one commit ago for primitive type names.

The language is early and carries no backward-compatibility obligation, and removing a keyword is
the least reversible kind of change once source exists in the wild. Now is when it is cheap.

## What Changes

- The `enum` declaration form is removed from the language. A closed set of constants is written
  `type Fit = fill | contain | cover`. **BREAKING** to NX source.
- The leading `|` in a union case list becomes optional, matching what `enum` already allowed and
  what OCaml and F# allow for single-line unions, so migration is exactly `enum` → `type`.
- `enum` remains a recognized token that produces a targeted diagnostic naming the `type` form to
  write instead, rather than an unhelpful parse error.
- Two terms are defined for the shapes a union case can take. A **constant case** declares no fields
  *and* belongs to a union that declares no base. A **constant union** is a union all of whose cases
  are constant — this is what an `enum` used to be, and it is now a structural property rather than
  a syntactic one.
- A constant case evaluates to a scalar runtime value carrying its union and case name, replacing
  both `Value::EnumValue` and the empty-record representation of a payloadless case. Every other
  case keeps the `$type` record representation. **BREAKING** to the wire shape of payloadless cases
  in non-constant unions, which change from `{"$type":"Shape.circle"}` to `"circle"`.
- Host and generated-code mappings key off constant-ness rather than off which keyword was written:
  a constant union generates each host language's idiomatic closed set — a C# `enum` from either
  generator, and in TypeScript a union of the authored string literals from CLI type generation
  (which stays a pure type surface) or an `as const` value object from executable codegen (which
  emits runtime code); a union with any payload case generates the existing polymorphic class
  hierarchy, with its constant cases carried as bare strings. This is what an NX enum generates
  today in each case, so the change is invisible to generated output.
- The .NET polymorphic serializer accepts a bare string as the wire form of a constant case, and
  generated C# gives such a case a singleton instance.
- The formatter's `payloadless_union_case` heuristic is deleted rather than corrected. With constant
  cases represented as scalars, an empty record is always a record.
- The value formatter emits every field in property position (`home=<Address city="Boston" />`,
  `items={<Item /> <Item />}`) rather than as element body content or as an element named after the
  property. This is RF5 in `contextual-literal-binding`'s `review.md`, and it has to land here:
  deleting the heuristic alone moves an empty qualified record from attribute position into the
  nested-element path, which discards the property name, so `<div data={<foo.bar />} />` would format
  to `<div><foo.bar /></div>` — still a different value. Fixing one without the other leaves the
  round-trip requirement red.

## Capabilities

### Modified Capabilities

- `discriminated-unions`: `enum` is removed from the declaration syntax, the leading `|` becomes
  optional, and constant cases and constant unions are defined with their runtime representation.
- `enum-values`: the requirements are restated in terms of constant union cases. The wire contract —
  the bare authored member string, recovered through the target type — is unchanged; the declaration
  that produces it is what changes.
- `unbraced-literal-forms`: a bare contextual name resolves against one closed nominal set (the
  constant cases of a union) rather than against two (enum members and payloadless union cases);
  and rendered value output binds every field by name in property position, so a formatted value
  reads back as itself.
- `runtime-output-format`: constant cases serialize as bare strings; only non-constant cases
  serialize as `$type` maps.
- `cli-code-generation`: which NX declaration produces a generated C# `enum` is decided by
  constant-ness, and generated unions with mixed cases carry constant cases as bare strings.
- `executable-code-generation`: the `as const` value object is emitted for a constant union.
- `dotnet-binding`: the managed polymorphic wire shape admits a bare string for a constant case
  alongside the `$type` map for every other case.
- `nx-ir-format`: NX IR encodes one nominal-member construct rather than separate enum-member and
  union-case constructs.
- `declaration-visibility`: visibility modifiers apply to the declaration forms that exist.
- `editor-language-service`: completion contexts no longer enumerate `enum` as a declaration kind.

## Impact

- `crates/nx-syntax`: `enum_definition`, `enum_member_list`, and `enum_member` are removed from the
  grammar; `union_case_list` gains an optional leading `|`; `parser.c`, `grammar.json`, and
  `node-types.json` are regenerated; `validation.rs` gains the `enum`-keyword diagnostic.
- `crates/nx-hir`: `EnumDef`, `EnumMember`, and `Item::Enum` are removed; lowering produces a single
  union form.
- `crates/nx-types`: `Type::Enum`, `EnumType`, and the parallel `enum_defs` resolution path are
  removed; contextual resolution consults one nominal kind.
- `crates/nx-interpreter`: one scalar value kind replaces `Value::EnumValue` and the empty-record
  representation of payloadless cases; `build_union_case_value` decides shape by constant-ness.
- `crates/nx-codegen`: `CodegenDeclarationKind::Enum` and `CodegenExpressionKind::EnumMember` are
  removed; union emission covers both shapes; NX IR and schema emission follow.
- `crates/nx-cli`: `payloadless_union_case`, `format_nested_element`, and `format_nested_record` are
  deleted from the formatter, which emits every field in property position; `typegen` decides the C#
  and TypeScript mapping by constant-ness.
- `crates/nx-language-service`, `crates/nx-api`, `crates/nx-ffi`: declaration-kind enumerations and
  value conversion.
- `bindings/dotnet`: `NxPolymorphicMessagePackFormatter` and the JSON polymorphic path accept a bare
  string for a constant case.
- `src/vscode`: TextMate grammar and snippets.
- Examples, fixtures, and documentation: 7 `enum` declarations across 5 `.nx` files, 54 in Rust test
  sources, and 17 in non-archive documentation.

## Sequencing

`contextual-literal-binding` must be archived before these deltas apply: this change modifies the
`unbraced-literal-forms` capability that change creates, and both the `enum-values` requirement
*Enum members are referenceable without naming the enum type* and the `discriminated-unions`
requirement *Payloadless union cases support contextual construction* that it adds.

`nominal-type-identity` should follow this change rather than precede it. Doing this first removes
one of the three nominal type constructors that change has to thread a declaring origin through, and
retires its decision D5 and task group 6 entirely — with constant cases represented as scalars there
is no runtime union-case discriminator left to add.
