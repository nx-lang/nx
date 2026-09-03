## 0. Order

Groups are executed in this order, not the order they are written:

`2.1` → `4.x` → `5.x` → `6.x` → `7.x` → `8.x` → `3.x` → `2.2`–`2.5` → `9.x` → `10.x`

Task 3.3 requires task 1.1's recorded output to be byte-identical after the source migration, and
that only holds once the union path generates what the enum path generated — group 7. Task 1.2
measured the gap: today the two spellings produce different C#, TypeScript, JavaScript, IR, and
schema output. So the representation is unified first with the `enum` keyword still parsing and
lowering to a `UnionDef`; group 3 then becomes a pure keyword swap with genuinely zero observable
change, which is what makes 3.3 a real check rather than a formality; and the keyword is removed
last. The tree stays green throughout. Agreed with the author on 2026-08-30.

## 0a. Soundness fixes made while unifying

Collapsing two nominal kinds into one surfaced two places where a union was matched **by name
alone**, which an enum was never subject to because `EnumType` equality compared its member list.
Both had to be closed, or every existing enum would have regressed:

- `crates/nx-types/src/ty.rs` and `crates/nx-types/src/infer.rs` each carried a
  `(Type::UnionCase, Type::Union)` compatibility rule reading `case.union == union.name`. A local
  `Fit.stretch` therefore satisfied a foreign `Fit`. Both now also require that the union actually
  declares the case. The `infer.rs` copy shadows the `ty.rs` one, so fixing only the latter changed
  nothing — worth knowing before touching either again.
- The contextual-name path resolved a case by looking the union's *name* up again, which finds
  whatever this module binds to that spelling rather than the type that was resolved. It now
  searches the resolved type's own cases, and only accepts a declaration reached by that name when
  it is the same type. This is what `nominal-type-identity`'s task 2.3a asks for, arrived at early
  because the collapse forced it.

These close the same-name/different-case-names hole for unions, which is half of RF6. The other
half — a same-named union whose case declares a payload field of a different type — is still open,
because `UnionType` equality covers case names and not payload field types. That stays
`nominal-type-identity`'s.

## 1. Pin the baseline

- [x] 1.1 Record the generated IR, JavaScript, TypeScript, C#, and schema output for every first-party
  `enum` declaration in the corpus at `HEAD`; verify the recording reproduces from a clean build
- [x] 1.2 Add a failing test asserting an `enum` declaration and the equivalent `type` declaration
  produce byte-identical IR, JS, TS, C#, and schema output; verify it fails today because the `type`
  form produces `$type` maps where the `enum` form produces bare strings
- [x] 1.3 Add a failing round-trip test formatting an empty qualified record that is not a union case
  (`<div data={<foo.bar />} />`); verify it fails by rendering the bare last segment — this is RF2 in
  `contextual-literal-binding`'s `review.md`. It stays failing after task 5.4 alone, because an
  empty record then routes to the nested-element path and loses the property name; task group 6 is
  what makes it pass. Asserted on the value rather than through a source round-trip: the interpreter
  cannot construct a record imported under a module alias (`RecordTypeNotFound`), so
  `<div data={<foo.bar />} />` has no end-to-end spelling today. That is a separate pre-existing gap;
  the defect this task pins lives entirely in `crates/nx-cli/src/format.rs`
- [x] 1.4 Add a failing test asserting a fieldless case of a base-less union serializes as a bare
  string in JSON and MessagePack; verify it fails with the current `$type` map
- [x] 1.5 Add a failing test asserting a record-valued property round-trips, and a second asserting
  two properties of the same record type stay distinguishable; verify the first fails to type check
  with `missing-content-property` and `missing-property`, and the second produces two identical
  sibling elements — this is RF5 in `contextual-literal-binding`'s `review.md`
- [x] 1.6 Add a failing test asserting a list-valued property round-trips; verify it fails by
  emitting the property name as an element tag (`<items>`), and record what an empty list currently
  produces (`items="..."`) as the input to task 6.4

## 2. Grammar

- [x] 2.1 Make the leading `|` optional in `union_case_list` for a list of two or more cases, keeping
  it required for a single case (design D2); verify `type A = B` still parses as a type alias and
  `type A = | B` as a single-case union. Two existing tests asserted the old rule and were replaced:
  `test_parse_union_definition_requires_leading_pipe`, and the invalid fixture
  `union-missing-leading-pipe.nx`, which is now `valid/union-without-leading-pipe.nx`.
  `UNION_DEFINITION_SYNTAX` in `validation.rs` was restated to match
- [x] 2.2 Remove `enum_definition`, `enum_member_list`, and `enum_member` from
  `crates/nx-syntax/grammar.js`, and remove `enum` from the declaration choice. This retires task
  1.2's parity tests, which compared the two spellings and become vacuous with one keyword left.
  They are repurposed rather than deleted: they now assert that D2's two case-list spellings
  (`= a | b` and `= | a | b`) generate identical C#, TypeScript, JavaScript, IR, and schema — the
  invariant that remains worth guarding
- [x] 2.3 Reserve `enum` in declaration position and emit the replacement diagnostic in
  `crates/nx-syntax/src/validation.rs` (design D5); verify `enum Fit = fill | cover` reports the
  `type Fit = fill | cover` form to write. Implemented as a **source-level** scan rather than a
  parse-tree check, because it has to fire regardless of where the parse gave up: with `enum` out of
  the grammar, `enum Fit = fill | cover` followed by another declaration reports two errors on the
  *next* line and never mentions the keyword. The new `removed-enum-keyword` diagnostic points at
  the keyword and names the form to write. It is appended after the parse errors rather than sorted
  by position, so the actionable one prints last — a legibility wart, not a correctness one. A
  source-level scan cannot tell a declaration from the same words quoted in prose, so it asked the
  tree where the keyword *cannot* be a declaration: comments, string literals, and element text
  content are collected up front and skipped (review RF6, which caught false positives in raw text,
  plain text content, and block comments)
- [x] 2.4 Regenerate `parser.c`, `grammar.json`, and `node-types.json`, and drop the removed kinds
  from `syntax_kind.rs`; verify the parser builds and every existing union fixture parses unchanged.
  `SyntaxKind::ENUM` outlived the first pass — the grammar stopped producing it, but the variant,
  its `"enum"` mapping, and its `is_keyword` membership stayed, so the kind claimed a keyword the
  language no longer has. Removed in the fix pass (review FP2)
- [x] 2.5 Add fixtures: a multi-case union without a leading pipe, a single-case union with one, a
  type alias that is not a union, and an invalid fixture for the `enum` keyword; verify each
  round-trips through the parser snapshot tests. The first three landed with task 2.1 in
  `valid/union-without-leading-pipe.nx`; the fourth is `invalid/removed-enum-keyword.nx`, covered by
  `test_parse_removed_enum_keyword_names_the_replacement` and by `test_parse_all_invalid_fixtures`

## 3. Migrate first-party sources

- [x] 3.1 Rewrite the 7 `enum` declarations in `.nx` files (`examples/nx/types.nx` and four
  `crates/nx-syntax/tests/fixtures` files) as `type` declarations; verify the corpus still analyzes
  clean. `fixtures/valid/enum-definition.nx` is renamed `constant-union-definition.nx`. The example
  corpus reports the same three diagnostics as at `HEAD` — `complex.nx:70`, `function.nx`'s missing
  library, and `types.nx:58`'s unimplemented member access — none introduced here
- [x] 3.2 Rewrite the 54 NX-syntax `enum` declarations embedded in Rust test sources; verify the
  suite still passes with the grammar change in place. 55 rewritten; two are deliberately left as
  `enum`, the inputs to task 1.2's parity tests, which exist to compare the two spellings and would
  become vacuous if migrated. Those two must go when task 2.2 removes the keyword — see 2.2
- [x] 3.3 Confirm task 1.1's recorded output is unchanged after the migration; verify byte identity
  rather than a spot check. Measured by recording the pinned corpus in both spellings from the same
  binary and diffing:
  - **Generated C# and TypeScript type definitions: byte-identical.** No difference at all.
  - **Generated JavaScript and TypeScript: identical except the `nx-fingerprint` comment**, which
    hashes the source text — and the source text is what the migration changes.
  - **NX IR: identical except the fingerprint, the embedded source text, and twelve case-span start
    offsets**, each shifted by exactly 2. That shift is inherent rather than incidental: an
    `enum_member` node spans just the identifier, while a `union_case` node spans its leading `| `
    too. Byte identity therefore cannot hold for the IR, and the guarantee that does hold — and the
    one a consumer can observe — is that the generated C#, TypeScript, and JavaScript are unchanged

## 4. One declaration form in HIR and types

- [x] 4.1 Remove `EnumDef`, `EnumMember`, and the enum item variant from `crates/nx-hir`; verify
  lowering produces a `UnionDef` for every closed-set declaration
- [x] 4.2 Add `is_constant` to the union case model, defined as no declared fields and no union base
  (design D3); verify it is derivable from the declaration with no evaluation
- [x] 4.3 Remove `Type::Enum` and `EnumType` from `crates/nx-types/src/ty.rs`; verify equality,
  compatibility, and display handle the single nominal kind
- [x] 4.4 Collapse the `enum_defs` and `union_defs` resolution paths in `crates/nx-types/src/infer.rs`
  into one; verify contextual name resolution consults a single nominal set. The surviving path keeps
  the *enum* path's nameability discipline, not the union path's: `nominal_is_nameable_here` compares
  the resolved type against the declaration reached under that name here, rather than only checking
  that the name is bound. Taking the union path's weaker check would have regressed every enum, since
  rejecting a same-named local declaration with different members is what RF1 of
  `contextual-literal-binding` closed. This incidentally closes the same-name/different-case-names
  half of RF6; the payload-field-type half stays open for `nominal-type-identity`, because
  `UnionType` equality covers case names and not payload types
- [x] 4.5 Update declaration-kind enumerations in `crates/nx-hir`, `crates/nx-api`,
  `crates/nx-language-service`, and `crates/nx-ffi`; verify no consumer still branches on an enum
  kind

## 5. Constant cases as scalar runtime values

- [x] 5.1 Replace `Value::EnumValue` with a single constant-case value kind carrying union and case
  name; verify all 27 sites compile and the value displays as `Union.case`
- [x] 5.2 Make `build_union_case_value` in `crates/nx-interpreter/src/interpreter.rs` produce that
  scalar for a constant case and a record for a payload case; verify a fieldless case of a union with
  an abstract base is still a record carrying the base's fields and defaults
- [x] 5.3 Update `SerializedValue` round-tripping, `runtime_type_of`, comparison in
  `crates/nx-interpreter/src/eval/logical.rs`, and host conversion in `crates/nx-api/src/value.rs`;
  verify task 1.4's test passes
- [x] 5.4 Delete `payloadless_union_case` and both its call sites in `crates/nx-cli/src/format.rs`,
  rendering every record in element form; verify a constant case still formats bare and that task
  1.3's test now fails on the dropped property name rather than the bare last segment, which task
  group 6 resolves
- [x] 5.5 Confirm no first-party consumer infers union-case-ness from a dotted name or an empty field
  map; verify by searching for the removed heuristic's shape

## 6. Property-position value formatting

- [x] 6.1 Rewrite `format_record_with_name` in `crates/nx-cli/src/format.rs` so every field is emitted
  in property position as `key=<value>` (design D7); verify the simple/complex distinction governs
  line layout only and no longer selects between property syntax and body-content syntax
- [x] 6.2 Emit a record-valued property as an unbraced element (`home=<Address city="Boston" />`) and
  a list-valued property as a braced sequence (`items={<Item /> <Item />}`); verify both spellings
  parse and type check, and that two properties of the same record type remain distinguishable
- [x] 6.3 Delete `format_nested_element` and `format_nested_record`; verify no remaining path emits a
  property name in element-tag position and none drops it
- [x] 6.4 Make `format_value` fail explicitly for a value with no NX spelling — an empty list and a
  `Value::ActionHandler` — instead of emitting `items="..."` or a synthetic `<ActionHandler />`;
  verify the caller reports the failure rather than printing output that does not read back
- [x] 6.5 File the empty-list spelling gap as a separate language question (design D7, Open
  Questions); verify it is recorded outside this change rather than worked around in the formatter.
  Filed as the change `empty-list-spelling`, with a `braced-value-sequences` delta stating that a
  spelling must exist without fixing which one; `openspec validate empty-list-spelling --strict`
  passes
- [x] 6.6 Confirm tasks 1.3, 1.5, and 1.6 all pass, and that 1.3 passes only with this group and task
  5.4 both in place

## 7. Code generation

> Landed while unifying the representation (the acceptance guard, task 1.2, now passes):
> `CodegenDeclarationKind::Enum`, `CodegenExpressionKind::EnumMember`, and
> `SchemaDeclarationKind::Enum` are gone; `CodegenUnionCase` and `NxIrUnionCase` carry `is_constant`;
> a constant union emits the frozen value object and `nxEnumSchema`; a constant case emits a bare
> string in a mixed union and a reference into the frozen object in a constant one; C# and TypeScript
> typegen key off constant-ness. Still open in this group: the mixed-union C# **singleton** for a
> constant case (7.4), and 7.6's corpus-wide byte-identity check.


- [x] 7.1 Remove `CodegenDeclarationKind::Enum`, `CodegenExpressionKind::EnumMember`, and
  `SchemaDeclarationKind::Enum` from `crates/nx-codegen`; verify union declaration emission produces
  the frozen-const output for a constant union that enum emission produced before
- [x] 7.2 Emit a constant case as a bare string and a payload case as a `$type` object in
  `emit_union_case_object`; verify a mixed union emits both shapes
- [x] 7.3 Update NX IR emission to one union-case construct; verify the IR carries enough to
  reproduce both wire shapes and that task 1.1's IR baseline is unchanged for constant unions.
  The second clause cannot hold and was dropped: removing `NxIrDeclarationKind::Enum` and
  `NxIrExpressionOp::EnumMember` *is* a change to the IR, so a constant union now serializes as
  `"tag": "union"` with `isConstant` on each case rather than as `"tag": "enum"` with `members`.
  What is verified instead, and is the guarantee that matters, is that the **generated output** for
  every constant union — C#, TypeScript, JavaScript — is byte-identical to the task 1.1 baseline.
  `isConstant` is what carries the wire shape through, so both shapes remain reproducible from the IR
- [x] 7.4 Update `emit_union`/`emit_union_case` in `crates/nx-cli/src/typegen/languages/csharp.rs` so
  a constant union generates a CLR `enum` and a constant case in a mixed union generates a singleton
  (design D4); verify generated C# compiles for both shapes. A constant union generates the CLR
  `enum` and its wire format, byte-identical to the task 1.1 baseline for all five first-party enum
  shapes; a constant case of a mixed union carries `[NxConstantCase]` and a
  `public static readonly ... Instance`. Verified by compiling the generated C# against the SDK and
  running it: all four shapes round-trip through both serializers, and a fieldless case of a *based*
  union is correctly not constant
- [x] 7.5 Update `emit_union` in `crates/nx-cli/src/typegen/languages/typescript.rs` so a constant
  union generates the `as const` object and a constant case in a mixed union contributes a string
  literal to the union type; verify generated TypeScript type-checks for both shapes.
  **The `as const` half of this task is wrong and was not done.** It conflates this generator with
  executable codegen (task 7.7), where the `as const` object already exists and belongs. CLI type
  generation emits a pure type surface — no runtime values in the module at all — and TypeScript's
  idiomatic closed set for that role is the union of the authored string literals, which is what an
  enum generated here and what a constant union generates now. The two forms are the same *type*
  (`typeof X[keyof typeof X]` is the literal union), so nothing is lost: the only difference is
  whether a types-only module exports a value. This was reconsidered on its merits with backward
  compatibility set aside, and the union was kept — see design D4 for the rationale and the trigger
  to revisit. What this task actually required, and what was done, is the constant-case string
  literal in a mixed union; the constant union keeps its string-literal-union output unchanged
- [x] 7.6 Confirm task 1.2's byte-identity test passes for every constant union in the corpus. Task
  1.2's test established this and was then repurposed by task 2.2, since with one keyword left there
  are no longer two spellings to compare. The corpus-wide confirmation is task 3.3's measurement: all
  five first-party enum shapes generate byte-identical C#, TypeScript, and JavaScript

## 8. Host bindings

- [x] 8.1 Make `NxPolymorphicMessagePackFormatter` accept a bare string as the wire form of a constant
  case and emit one for that case; verify a mixed union round-trips through MessagePack
- [x] 8.2 Apply the same alternative to the `[JsonPolymorphic]` path; verify a mixed union round-trips
  through `System.Text.Json`. Not achievable as a `[JsonPolymorphic]` *alternative*: System.Text.Json
  refuses a custom converter on a type that also carries polymorphism metadata
  (`NotSupportedException: The converter for derived type '…' does not support metadata writes or
  reads`), and reading two wire shapes needs a converter. A union with a constant case beside a
  payload case therefore registers its cases with a new `[NxUnionCase]` attribute, which both
  serializers read, and carries `NxPolymorphicJsonConverter<T>`; a union with one wire shape keeps
  `[JsonPolymorphic]` and `[JsonDerivedType]` untouched
- [x] 8.3 Update `bindings/dotnet/tests/NxLang.Sdk.Tests/NxUnionSerializationTests.cs` to cover a
  mixed union, and verify an unknown bare string is rejected through the existing error path
- [x] 8.4 Verify `runtime/typescript` handles the bare-string case form at a union-typed boundary,
  including rejecting an unknown case

## 9. Tooling and documentation

- [x] 9.1 Remove `enum` from `src/vscode/syntaxes/nx.tmLanguage.json` and the snippets, and verify the
  grammar suite covers a multi-case union without a leading pipe. The snippets file never mentioned
  `enum`. Two findings beyond the removal: the TextMate grammar did not recognise D2's bare-first-case
  form at all — its union rule required `|` or end-of-line straight after `=`, so
  `type Color = red | green | blue` fell through to the alias rule and its pipes were scoped only by
  the generic operator fallback — and that gap is now closed by a second union pattern. The scope
  `variable.other.enummember.nx` is deliberately kept: it is the conventional TextMate scope for a
  member of a closed set, and themes colour by it. Grammar suite: 82 passing
- [x] 9.2 Update the 17 non-archive documentation occurrences in `docs/src/content/docs`,
  `nx-grammar.md`, and `nx-grammar-spec.md`, presenting one declaration form; verify no page still
  shows `enum` as NX syntax. The only surviving mention is the generated **C#** `enum`, which is
  accurate. `docs/nx-ir-format.md` was updated too though it is outside the listed scope: this change
  removes the IR's enum constructs, so it had gone stale. Two statements were wrong rather than
  merely dated and are corrected: `nx-grammar.md` claimed the leading `|` is required and that
  `type Result = Success | Failure` is invalid, which D2 reverses.

  Extended on request beyond the listed scope to the two drawn-UI proposals, which carried 32
  declarations between them: `docs/NX-Drawn-UI-MVP-Object-Model-Proposal.md` and
  `docs/drawn-ui-proposal-nx-enhancements.md`. Four of their claims were stale rather than merely
  worded in the old vocabulary, and are corrected — see the report. Every migrated NX block was
  parse-checked; Appendix A analyzes with zero diagnostics. `docs/drawn-ui-proposal-review.md` and
  `…-Proposal-original.md` were checked and left alone: their `enum` mentions all describe A2UI,
  DrawnUI, or JSON Schema validation, not NX's removed keyword
- [x] 9.3 Remove the stale mention of enums from `openspec/specs/record-type-inheritance/spec.md`'s
  base-record clause as an editorial follow-up (design D6); verify no behavior changes
- [x] 9.4 Update `openspec/specs/enum-values/spec.md`'s `## Purpose` after archive to describe
  constant union cases (design D6). Done now rather than after archive — it is the same edit and the
  delta does not touch `## Purpose`. The Purpose says so explicitly, noting that the requirements
  below it are restated in constant-case terms when this change archives

## 10. Verification

- [x] 10.1 Run the full workspace suite, the .NET binding tests, and the VS Code grammar suite; verify
  all three pass with no pre-existing test modified except for the `enum` → `type` migration.
  **The .NET step must be `cargo build --release -p nx-ffi && dotnet test bindings/dotnet/NxLang.sln`.**
  `NxLang.Sdk.targets` stages `target/release/libnx_ffi.so` and only warns when it is absent, so a
  bare `dotnet test` silently runs the managed tests against whatever native library was last built
  — which is how this step first reported green while eight tests were in fact failing.
  **All four green**: Rust workspace 1259, .NET 96, VS Code grammar 82, TypeScript runtime 12.
  **This condition is too strong and needs relaxing.** Some pre-existing tests assert behaviour this
  change deliberately alters, and no migration can leave them untouched: `eval.rs`'s
  *eval_source_returns_fieldless_union_case_as_type_map* and
  `simple_functions.rs`'s *test_fieldless_union_case_shorthand_constructs_record_value* both pin the
  record wire shape of a fieldless case in a base-less union, which the proposal states as
  **BREAKING**; and the contextual-literal diagnostics name "an enum or union", which is now one
  kind. The condition should be: no pre-existing test modified except for the `enum` → `type`
  migration, the wire-shape change the proposal declares, and diagnostic wording that names the
  removed keyword
- [x] 10.2 Walk every scenario in the nine delta specs and confirm each has a corresponding test.
  There are **ten** delta specs, not nine, carrying 118 scenarios between them. Most are carried
  forward unchanged from the promoted specs and keep their existing tests; the walk concentrated on
  the scenarios this change adds or alters. Two had no covering test and now do:
  *A fieldless case of a union with a base keeps the `$type` map*, the D3 boundary at runtime rather
  than in typegen — `test_fieldless_case_of_a_based_union_keeps_the_record_shape`; and
  *Declaration completions do not offer the removed enum keyword* —
  `declaration_completions_do_not_offer_the_removed_enum_keyword`. `enum` was already absent from
  `KEYWORD_COMPLETIONS`, so that one pins existing behaviour rather than changing it.

  **Two scenarios are not testable and should be read as prose, not as claims a test backs.**
  *Generated output for a constant union is unchanged from the enum form* and
  *Constant-union output is unchanged from the enum contract* both compare against a form that no
  longer exists — nothing can construct the enum side once the keyword is gone. What stands behind
  them is task 3.3's recorded measurement against the `HEAD` baseline, which is a one-time artifact
  rather than a standing test. *Documentation explains scalar choices and payload cases with one
  form* is likewise a documentation scenario with no automated check.
- [x] 10.2a Align every MODIFIED block with the spec `contextual-literal-binding` promoted on
  2026-08-30, since a MODIFIED requirement replaces the whole block and archive refuses to drop
  scenarios. `openspec validate replace-enums-with-unions --strict` now passes.

  **The prescribed remedy does not work.** This task called for REMOVED plus ADDED rather than
  MODIFIED; OpenSpec rejects that outright — *"Requirement present in both ADDED and REMOVED"* — so a
  requirement cannot be restated by removing and re-adding it under the same name. RENAMED renames
  requirements, not scenarios, and the problem here is entirely in scenario names.

  What worked is the validator's own advice: keep each requirement MODIFIED, and keep every scenario
  under the name the promoted spec uses, restating the body. Five scenarios were renamed by this
  change's delta and had to be renamed back: `Bare name resolves to an enum member at an enum-typed
  property`, `Bare name resolves to a payloadless union case`, `A lexical binding of the same name
  does not shadow the member`, `Quoted string at an enum-typed property is rejected`, and `Unknown
  member suggests a near match`. Their bodies now describe constant cases; their titles still say
  "enum" and "member", because OpenSpec offers no way to rename a scenario without archive reading it
  as a deletion. That is a cosmetic wart in the promoted spec, worth filing against OpenSpec rather
  than worked around here.

  `editor-language-service` needed no restructuring — its two omitted scenarios were simply missing
  from the block and are now present, restated in union terms, alongside a new scenario asserting
  that declaration completions no longer offer `enum`.
- [x] 10.3 Verify the example corpus produces identical diagnostics counts before and after, and that
  the only IR, JS, TS, C#, and schema differences are the intended wire-shape change for fieldless
  cases of base-less and mixed unions. Diagnostic counts are **identical** for all thirteen example
  files, compared against `HEAD` with the pre-migration corpus. The output differences are task 3.3's
  measurement, and reduce to three: the union spelling now generating what the enum spelling
  generated, the declared wire-shape change for constant cases, and the IR's one-construct change
- [x] 10.4 Update `review.md` in `contextual-literal-binding` to mark RF2 and RF5 resolved here,
  pointing at the covering tests from tasks 1.3, 1.5, and 1.6. Both entries now carry a
  **Fixed by** note naming the covering test. The Review-fix summary also records that RF6 was
  closed in part here rather than waiting for `nominal-type-identity`: the unified resolution path
  forced it, because leaving the name-only union matching in place would have regressed every enum
- [x] 10.5 Update `nominal-type-identity` to remove everything the constant-case representation makes
  unnecessary. Removed: `design.md`'s D5 and its `Value`-variant risk (D6 renumbered to D5, and the
  migration plan from five steps to four); its Goals line *Runtime values distinguish a union case
  from a record*; `proposal.md`'s payloadless-`Value::Record` paragraph, its runtime-values bullet,
  and its `crates/nx-interpreter` `Value` line; `tasks.md`'s task 1.3 and all of task group 6; and
  the *Runtime values distinguish a union case from a record* requirement together with the whole
  `unbraced-literal-forms` delta, which existed only to carry it. Group 6's slot is kept as a
  tombstone rather than renumbered, so the cross-references in groups 7–9 still resolve. The three
  later additions are handled too: tasks 1.1 and 1.1a are collapsed into one union-only collision
  test, `EnumType` is out of task 2.1's origin-field list, and the *identical shape* scenario has
  lost its enum clause. Task 2.3a is **marked done rather than surviving unchanged** — the unified
  resolution path in this change had to fix exactly what it describes. Its remaining scope is origin
  only
- [x] 10.5a Retarget `nominal-type-identity`'s deltas onto the capabilities as this change leaves
  them. Its `enum-values` delta now MODIFIES *Constant cases are referenceable without naming the
  union type* — the requirement this change ADDs in place of *Enum members are referenceable without
  naming the enum type* — carrying forward all three of that requirement's scenarios plus the
  wildcard-alias one, with the import guard reversed as that change intends. The remaining enum
  vocabulary is restated: of the 64 occurrences, the only ones left are the `enum-values` capability
  path, which D6 deliberately keeps, and accurate references to `enum_defs` and to this change by
  name. The motivating example in its `proposal.md` is rewritten — it used `enum Fit = stretch |
  squish`, a *differently* shaped local type, which this change now rejects on its own; it uses an
  identically shaped local union, which is the case that is still open. `openspec validate
  nominal-type-identity --strict` passes
- [x] 10.6 Correct `nominal-type-identity`'s RF2 references, which predate this change: its
  `proposal.md` Impact line and its task 9.4 both claim RF2 is resolved there; RF2 and RF5 are
  resolved here, and only RF1's deferred half and RF3 remain that change's to close. Both corrected.
  One addition beyond the task: RF6 is now split across the two changes rather than wholly that
  change's, because its same-name/different-case-names half closed here — see task 4.4

