# Review: add-function-types

## Scope

**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md` (41/41 done), and all eleven delta
specs (`function-types`, `function-values`, `type-reference-suffixes`, `component-type-parameters`,
`nx-ir-format`, `typescript-ir-runtime`, `drawnui-nx-catalog`, `playground`,
`editor-syntax-highlighting`, `editor-language-service`, and `dotnet-binding`, which arrived with
RF9's second fix).

**Reviewed code:** the whole working tree for this change (101 paths) —
`crates/nx-syntax` (grammar, `syntax_kind.rs`, `validation.rs`, highlights, fixtures),
`crates/nx-hir` (`ast/types.rs`, `lower.rs`, `components.rs`, `declarations.rs`),
`crates/nx-types` (`ty.rs`, `infer.rs`, `semantics.rs`, `check.rs`),
`crates/nx-interpreter` (`value.rs`, `interpreter.rs`, `eval/logical.rs`),
`crates/nx-api` (`value.rs`, `artifacts.rs`),
`crates/nx-codegen` (`ir.rs`, `builder.rs`, `emit.rs`, `ir_explain.rs`, `ir_image.rs`, `model.rs`),
`crates/nx-cli` (`typegen`, `format.rs`), `crates/nx-language-service`,
`runtime/typescript/src/index.ts`, `specs/ir-conformance/function-values/`,
`sites/playground` (catalog generator + regenerated catalog, `render/*`, examples, docs),
`src/vscode/syntaxes` + grammar tests, and the language/grammar/IR documentation.

**Verification run during review:** `cargo test --workspace` (pass), `cargo clippy --workspace
--all-targets` (only pre-existing `nx-value`/`nx-diagnostics` findings, covered by the separate
`fix-clippy-diagnostics` change), `npm test` in `runtime/typescript` (pass), `pnpm test` +
`check-examples` in `sites/playground` (pass, 20 examples), `npm test` in `src/vscode`
(256 passing), `pnpm run typecheck` and `pnpm run build` in `sites/playground` (pass), plus ad-hoc
`nxlang run` / `nxlang codegen` / `nxlang ir explain` probes of the spec scenarios and of both
runtimes on the same emitted image.

Spot checks that passed cleanly and are **not** findings: subset-rule acceptance and rejection with
the parameter named in the diagnostic, contravariant parameters, covariant result, parameter-order
independence, `TItem` substitution and the "TItem was not specified" diagnostic, erasure through a
function type in IR, the `Function` record in JSON output, element invocation of a prop and of a
paren-function parameter, the positional-call rejection, feature-list presence and absence,
`namedCall` explained text, and the docs' NX snippets compiling.

## Findings

### ✅ Verified - RF1 Two function values naming the same declaration compare unequal in the Rust interpreter but equal in the TypeScript runtime

- **Severity:** High
- **Evidence:** `values_equal` in [logical.rs:136-176](crates/nx-interpreter/src/eval/logical.rs#L136-L176)
  gained no `Value::Function` arm, so two function values fall through to `_ => false`. The
  TypeScript runtime's `deepEqual` is `JSON.stringify` equality
  ([index.ts:3254](runtime/typescript/src/index.ts#L3254)) and `FunctionReferenceValue.toJSON()`
  returns the canonical record, so it answers `true`. The same program gives opposite answers:

  ```nx
  external component <Box Flag:boolean? />
  let <A Item:object />: string = "a"
  let f: <function Item:object />: string = {A}
  let g: <function Item:object />: string = {A}
  <Box Flag={f == g} />
  ```

  `nxlang run` → `<Box Flag=false />`; the same image through `prepareNxIrProgram` /
  `evaluateFunction` → `{"$type":"Box","Flag":true}`. This contradicts three statements this change
  itself adds: the `typescript-ir-runtime` delta ("A function value SHALL be equal to another only
  when both name the same declaration"), the `Value::Function` doc comment in
  [value.rs:93-102](crates/nx-interpreter/src/value.rs#L93-L102), and *Function values* in
  [docs/nx-ir-format.md](docs/nx-ir-format.md). `values_equal` is also what match patterns and
  `diff` use, so the divergence is not confined to `==`.
- **Recommendation:** Add the arm to `values_equal`: two `Value::Function` values are equal when
  `module` and `name` both match. Pin it with an interpreter test beside the ones in
  `crates/nx-interpreter/tests/function_values.rs`, and add the same program to the
  `function-values` conformance corpus so the two runtimes stay pinned to each other.
- **Fix:** `values_equal` gained a `Value::Function` arm comparing `module` and `name`
  ([logical.rs](crates/nx-interpreter/src/eval/logical.rs)), with its doc comment extended to say
  so. Pinned by `two_function_values_are_equal_exactly_when_they_name_the_same_declaration` in
  `crates/nx-interpreter/tests/function_values.rs`, by `identical` / `distinct` entrypoints added
  to the `function-values` conformance program (both runtimes now answer `true` / `false`), and by
  a new `emitted-ir.test.mjs` case. The `function-values` delta states the equality rule and
  carries a scenario for it, so it is no longer only in the TypeScript runtime's spec.
- **Verification:** Confirmed. `values_equal` now carries the `Value::Function` arm comparing
  `module` and `name` ([logical.rs:158-167](crates/nx-interpreter/src/eval/logical.rs#L158-L167)).
  Re-ran the original repro: `nxlang run` gives `<Box Flag=true />` and the same emitted image
  through `prepareNxIrProgram` / `evaluateFunction` gives `{"$type":"Box","Flag":true}` — the two
  runtimes agree where they disagreed. The interpreter test passes (10/10 in
  `crates/nx-interpreter/tests/function_values.rs`), the corpus program's `identical` / `distinct`
  entrypoints pass in both the debug and stripped runs of `corpus.test.mjs` with recorded
  `true` / `false`, and `emitted-ir.test.mjs` prints `ok - a top-level let of function type is
  callable and compares by declaration in both runtimes`. The `function-values` delta now states
  the rule for `==`, match patterns and `diff`, with a scenario, so it is no longer only in the
  TypeScript runtime's spec.

### ✅ Verified - RF2 A top-level `let` of function type is invocable in the checker and the interpreter but refused by codegen

- **Severity:** High
- **Evidence:** `build_element_expression` guards the named-call path with `if
  !scope.contains(element.tag.as_str())` and otherwise emits a "missing semantic data" diagnostic
  ([builder.rs:1963-1972](crates/nx-codegen/src/builder.rs#L1963-L1972)). `scope` is the lexical
  scope (parameters, props, loop binders); a top-level `let` is a declaration and is not in it. But
  `InferenceContext::function_typed_value` deliberately accepts one — its doc says "a parameter, a
  prop, a local or top-level `let` of function type" — and the interpreter resolves it too. So:

  ```nx
  external component <Box Label:string? />
  let <Wrap Item:object />: string = "w"
  let F: <function Item:object />: string = {Wrap}
  <Box Label=<F Item="x" /> />
  ```

  `nxlang run` → `<Box Label="w" />`, but `nxlang codegen --target nx-ir` fails with
  `Cannot build codegen program for '...' because function-typed binding 'F' is unavailable`. A
  program that analyses and interprets cleanly cannot be compiled, and the diagnostic reads as an
  internal compiler error rather than a language rule.
- **Recommendation:** In the named-call branch, when the tag is not in `scope`, fall back to
  `resolve_visible_reference` for the callee — the same reference node the `Expr::Ident` path at
  [builder.rs:1129-1159](crates/nx-codegen/src/builder.rs#L1129-L1159) already emits — instead of
  failing. Add a codegen/IR test for the top-level `let` form, and an `emitted-ir.test.mjs` case
  pinning it against the native interpreter. (If a top-level `let` is meant to be excluded instead,
  the checker and interpreter must reject it with an author-facing diagnostic and the
  `function-values` spec should say so.)
- **Fix:** the named-call branch in `build_element_expression` now falls back to
  `resolve_visible_reference` when the tag is not in lexical scope and carries the resolved
  reference on the callee identifier ([builder.rs](crates/nx-codegen/src/builder.rs)); it still
  reports missing semantic data when the name reaches nothing at all. The repro emits and explains
  as `<F Item="x" />`, and both runtimes produce `<Box Label="w" />`. Pinned by
  `a_call_of_a_top_level_let_of_function_type_is_a_named_call_on_a_reference` in `ir_tests.rs` and
  by an `emitted-ir.test.mjs` case comparing the two runtimes. Answering the question below: a
  top-level `let` *is* invocable — the requirement in the `function-values` delta now reads "a
  parameter, a component prop, or a `let`, local or top-level" and has a scenario for it.
- **Verification:** Confirmed. The named-call branch now mirrors the `Expr::Ident` path exactly —
  slot when in lexical scope, `resolve_visible_reference` otherwise, and the missing-semantic-data
  diagnostic only when the name reaches neither
  ([builder.rs:1966-2010](crates/nx-codegen/src/builder.rs#L1966-L2010)). The original repro now
  emits: `nxlang codegen --target nx-ir` succeeds, `ir explain` shows `value F = Wrap` and
  `<Box Label=<F Item="x" /> />`, the module still lists `function-values-v1`, and both runtimes
  produce `<Box Label="w" />`. `a_call_of_a_top_level_let_of_function_type_is_a_named_call_on_a_reference`
  passes in `ir_tests.rs`, and the `emitted-ir.test.mjs` case compares the two runtimes against
  `nativeJson`. The `function-values` requirement now reads "a parameter, a component prop, or a
  `let`, local or top-level" and carries a scenario for the top-level form.

### ✅ Verified - RF3 `cargo fmt --all --check` fails on fifteen files this change touched, which breaks CI

- **Severity:** Medium
- **Evidence:** `.github/workflows/build.yml:230` runs `cargo fmt --all --check`. It currently
  reports 32 diffs across `crates/nx-codegen/src/{builder,emit,ir_explain,ir_tests}.rs`,
  `crates/nx-hir/src/lower.rs`, `crates/nx-interpreter/src/interpreter.rs`,
  `crates/nx-interpreter/tests/function_values.rs`, `crates/nx-syntax/src/{syntax_node,validation}.rs`,
  `crates/nx-syntax/tests/{parser_tests,query_tests}.rs`, `crates/nx-types/src/ty.rs`,
  `crates/nx-types/tests/{function_types,type_checker_tests}.rs` — all files in this change.
- **Recommendation:** Run `cargo fmt --all` before committing.
- **Fix:** `cargo fmt --all` run over the workspace; `cargo fmt --all --check` is clean.
- **Verification:** Confirmed. `cargo fmt --all -- --check` exits 0 with no output.

### ✅ Verified - RF4 The duplicate-nullable rule does not see through nested parentheses

- **Severity:** Low
- **Evidence:** `validate_type_suffix_chain` seeds the current nullable suffix from the base only
  when that base is a `PARENTHESIZED_TYPE` whose enclosed type's *last* child is a `?`
  ([validation.rs:276-283](crates/nx-syntax/src/validation.rs#L276-L283),
  [validation.rs:316-320](crates/nx-syntax/src/validation.rs#L316-L320)). With a second layer of
  parentheses the last child is another `parenthesized_type`, so nothing is seeded:
  `type Twice = (string?)?` is rejected, `type Twice = ((string?))?` is accepted. The
  `type-reference-suffixes` delta says the rejection holds "including across a parenthesis", and
  `nx-grammar.md` repeats "Parentheses add no layer, so `(string?)?` is rejected on the same terms".
- **Recommendation:** Make `trailing_nullable_suffix` recurse through a nested
  `PARENTHESIZED_TYPE` base, and add the nested case to
  `tests/fixtures/invalid/parenthesized-duplicate-nullable.nx`.
- **Fix:** `trailing_nullable_suffix` now recurses through a nested `PARENTHESIZED_TYPE` base
  ([validation.rs](crates/nx-syntax/src/validation.rs)), so `((string?))?` is rejected on the same
  terms as `(string?)?`. The nested case is asserted inline in
  `test_duplicate_nullable_across_a_parenthesis_is_rejected` rather than in the fixture, because
  that test pins the fixture's single diagnostic by source offset; `((string?)[])?` is asserted to
  stay legal beside the existing `(string?)[]?` case.
- **Verification:** Confirmed. `trailing_nullable_suffix` recurses through a nested
  `PARENTHESIZED_TYPE` base ([validation.rs:319-327](crates/nx-syntax/src/validation.rs#L319-L327)),
  and the recursion handles arbitrary depth, not just one extra layer: `((string?))?` and
  `(((string?)))?` are both rejected with the same "Type is already nullable at this layer"
  diagnostic pointing at the outer `?`. Legal forms still pass — `(string?)[]?`, `((string?))[]`,
  `(<function Item:object />: string?)?` and `string?[]?` all compile.

### ✅ Verified - RF5 `docs/nx-ir-format.md` still states that NX has no syntax for a function type

- **Severity:** Low
- **Evidence:** [docs/nx-ir-format.md:137-138](docs/nx-ir-format.md#L137-L138) reads "NX has no
  syntax for a local `let` binding, an index expression or a function type, so none has a kind" —
  two lines under a node table that this change extended, and in the same file that now documents
  type kind 4. Separately, the feature-list paragraph edit left an unwrapped ~150-column line at
  [docs/nx-ir-format.md:365](docs/nx-ir-format.md#L365) in a file otherwise wrapped near 100, and
  the `sites/playground/README.md` edit leaves a similarly over-long line ("…which is how Recycled
  cells and Uneven cells are ported. The gallery can be read as a coverage report").
- **Recommendation:** Drop "or a function type" from the sentence (a function type does have a kind
  now; it simply has no *node* kind, which is worth saying explicitly), and re-wrap both paragraphs.
- **Fix:** the sentence now says that neither a local `let` binding nor an index expression has a
  node kind, and that a function type has a *type* kind but no node kind. The feature-list
  paragraph in `docs/nx-ir-format.md` and the paragraph in `sites/playground/README.md` are
  re-wrapped.
- **Verification:** Confirmed. [docs/nx-ir-format.md:137-140](docs/nx-ir-format.md#L137-L140) now
  says a local `let` binding and an index expression have no node kind, and that a function type
  has a *type* kind but no node kind — which is both accurate and the more useful statement. The
  feature-list paragraph ([docs/nx-ir-format.md:361-369](docs/nx-ir-format.md#L361-L369)) and the
  `sites/playground/README.md` paragraph are re-wrapped; no prose line in either exceeds 100
  columns (the remaining long lines in `nx-ir-format.md` are table rows and hex dumps, as before).

### ✅ Verified - RF6 `namedCall` arguments are emitted alphabetically, not in source order as the spec requires

- **Severity:** Low
- **Evidence:** The `nx-ir-format` delta says a named call carries "the callee expression, then each
  argument as a name and a node, **in source order**"
  ([specs/nx-ir-format/spec.md:58](openspec/changes/add-function-types/specs/nx-ir-format/spec.md)),
  and design D4 repeats it. The named-call branch returns before the element sort at
  [builder.rs:2004-2006](crates/nx-codegen/src/builder.rs#L2004-L2006), but `mapped.properties`
  already arrives sorted, so source order is not preserved. `<Row Zeta="a" Alpha=1 />` explains as
  `<Row Alpha=1 Zeta="a" />`. Nothing misbehaves — binding is by name — but the artifact does not
  match its own spec.
- **Recommendation:** Either preserve source order for the named-call argument list, or amend the
  delta spec and `docs/nx-ir-format.md` to say the arguments are in a stable canonical order, as
  `element` node properties already are. The second is probably right and is the smaller change.
- **Fix:** amended the spec rather than the artifact, as recommended: a `namedCall`'s arguments go
  through the same `properties()` encoder as element properties, which sorts by name. The
  `nx-ir-format` delta and the `namedCall` row in `docs/nx-ir-format.md` now say the arguments are
  sorted by name like any property list.
- **Verification:** Confirmed, as the spec-side fix the finding recommended. The `nx-ir-format`
  delta now says the arguments are "sorted by argument name as a property list is, since binding is
  by name and a stable order keeps the artifact canonical", the `namedCall` row in
  `docs/nx-ir-format.md` says "sorted by name like any property list", and the file's ordering
  paragraph states "Property lists are sorted by property name. Content lists are in source order."
  No artifact change was needed, and `<Row Zeta="a" Alpha=1 />` still explains as
  `<Row Alpha=1 Zeta="a" />` — now documented rather than contradicted.

### ✅ Verified - RF7 A templated control nested inside a template cell silently loses its template

- **Severity:** Low
- **Evidence:** `MaterializeContext.bindTemplate` exists and its doc says "nested lists stay
  templated" ([materialize.ts:22-28](sites/playground/src/render/materialize.ts#L22-L28)), but
  `DrawnTree.tsx` builds `cellContext` without it
  ([DrawnTree.tsx:116-122](sites/playground/src/render/DrawnTree.tsx#L116-L122)) — `bindTemplate`
  is defined on the next line and never attached. Inside a cell, `coerceProps` therefore takes the
  `templateParams !== undefined` branch, calls `bindTemplate?.(…)` on `undefined`, and `continue`s
  ([values.ts:171-180](sites/playground/src/render/values.ts#L171-L180)), so the `ItemTemplate` is
  dropped with no `reportUnknown` / `reportInert` / `reportTemplateFailure` notice. A nested
  templated `SkiaStack` in a cell draws empty and says nothing.
- **Recommendation:** Attach `bindTemplate` to `cellContext` after it is defined (the cycle is fine
  — the object is mutated before any cell binds), or report the drop through `reportInert` so the
  diagnostics pane names it.
- **Fix:** `bindTemplate` is now defined before `cellContext` and carried on it
  ([DrawnTree.tsx](sites/playground/src/render/DrawnTree.tsx)), so a templated control inside a
  cell templates its own cells. Pinned by `a templated control inside a cell keeps its own
  template` in `src/render/materialize.test.mjs`, which checks the nested control's `ItemTemplate`
  is a factory producing an `NxTemplateCell` and that nothing was reported as unknown.
- **Verification:** Confirmed. `bindTemplate` is declared before `cellContext` and carried on it
  ([DrawnTree.tsx:114-126](sites/playground/src/render/DrawnTree.tsx#L114-L126)), so a templated
  control materialized inside a cell binds its own factory. `a templated control inside a cell
  keeps its own template` passes (7/7 in `src/render/materialize.test.mjs`) and asserts the nested
  control's `ItemTemplate` is a factory returning an `NxTemplateCell`, with nothing reported
  unknown or failed. One note, not a defect: the test rebuilds the context the way `DrawnTree`
  does rather than driving `DrawnTree` itself, so it pins `materialize` / `coerceProps` behaviour
  and not the wiring in `DrawnTree.tsx`.

### ✅ Verified - RF8 The "function type in NX spelling" rendering is written three times, with the parenthesize-under-suffix rule written four times, and nothing pins them together

- **Severity:** Low
- **Evidence:** `format_function_type` ([ty.rs:393](crates/nx-types/src/ty.rs#L393)),
  `type_ref_display` ([lib.rs:2201](crates/nx-language-service/src/lib.rs#L2201)) and the
  `kinds::ty::FUNCTION` arm ([ir_explain.rs:219](crates/nx-codegen/src/ir_explain.rs#L219)) each
  build the same string independently; `write_postfix_type`, `qualified_postfix_display`,
  `type_ref_postfix_base` and `ty_under_suffix` each re-implement "parenthesize a function type
  under a suffix". The `function-types` delta makes one spelling a requirement across diagnostics,
  hovers and explained artifacts, so three implementations are three places for it to drift, and no
  test compares any two of them.
- **Recommendation:** They operate on three different representations (`Type`, `ast::TypeRef`, IR
  indices), so full sharing is awkward; at minimum add a small shared spelling helper parameterised
  by a per-part renderer (as `format_function_type` already is), reuse it from the language service,
  and add one test asserting that a single source function type renders identically through
  `Type::to_string`, hover and `ir explain`.
- **Fix:** the spelling now lives once, in `nx_hir::ast::spell_function_type`, parameterised by a
  per-part renderer; `format_function_type` (nx-types), the `kinds::ty::FUNCTION` arm
  (`ir_explain.rs`) and the language service all assemble through it. `type_ref_display` and
  `type_ref_postfix_base` are gone from the language service in favour of
  `nx_hir::ast::spell_type_ref` / `spell_type_ref_under_suffix`, retiring one of the four
  parenthesize-under-suffix copies; the two remaining ones read a `Type` and IR indices and cannot
  share more without threading a renderer through. Two tests compare the assembled spelling with
  the checker's `Type` display — one from the hover
  (`a_hover_spells_a_function_type_the_way_the_checker_does`) and one from the explained artifact
  (in `a_function_typed_prop_is_a_function_type_in_nx_spelling`) — so all three are pinned to one
  another.
- **Verification:** Confirmed, and the consolidation goes further than the finding asked. The
  spelling lives once in `nx_hir::ast::spell_function_type`
  ([types.rs:115-132](crates/nx-hir/src/ast/types.rs#L115-L132)), parameterised by a per-part
  renderer, and all three callers assemble through it: `format_function_type`
  ([ty.rs:396-402](crates/nx-types/src/ty.rs#L396-L402)), the `kinds::ty::FUNCTION` arm
  ([ir_explain.rs:238](crates/nx-codegen/src/ir_explain.rs#L238)) and the language service
  ([lib.rs:2205](crates/nx-language-service/src/lib.rs#L2205)). `type_ref_display` and
  `type_ref_postfix_base` are gone in favour of shared `spell_type_ref` /
  `spell_type_ref_under_suffix`, retiring one of the four parenthesize-under-suffix copies. Both
  parity tests pass and genuinely compare across representations — the hover test and the
  `ir_tests.rs` explained-artifact test each build a `nx_types::Type` and assert the rendered text
  equals its `Display`, and the IR test still asserts the text contains no `=>`.

### ✅ Verified - RF9 C# typegen now reaches a function type and emits a `System.Delegate` that cannot be serialized in either supported format

- **Severity:** Medium (raised from Low: this is a functionality gap, not a typing gap)
- **Evidence:** `TypeRef::Function` mapped to `global::System.Delegate` as an unreachable fallback
  ([csharp.rs:1138](crates/nx-cli/src/typegen/languages/csharp.rs#L1138),
  [csharp.rs:1310](crates/nx-cli/src/typegen/languages/csharp.rs#L1310)). Before this change no NX
  syntax produced a function type; the regenerated `sites/playground/catalog/skia.nx` now declares
  `ItemTemplate: (<function Item:TItem Index:int />: DrawnNode)?`, so any C# contract generated over
  a catalog like it gets a `System.Delegate` member that carries none of the parameter names the
  TypeScript side now carries. The proposal's Impact names only the TypeScript mapping, so this is
  arguably deliberate scope — but the fallback comment no longer describes reality and nothing pins
  the behaviour.
- **Recommendation:** Either map the function type to a named C# delegate (or at least record the
  decision in the design's implementation notes) and add a typegen test, or emit the same
  "unsupported member" diagnostic the C# backend uses elsewhere so the gap is visible.
- **Fix:** taken as the "record the decision and pin it" branch of the recommendation, not a
  mapping change. The design's implementation notes now say why a function-typed member stays
  `global::System.Delegate` — C# has no shape for arguments bound by name without a named delegate
  per function type, and the proposal's Impact names only the TypeScript mapping — and
  `generates_a_csharp_delegate_for_a_function_type` in `crates/nx-cli/src/typegen.rs` pins the
  mapping so the gap is visible. A richer C# mapping remains available as its own change.
- **Verification:** **Reopened.** The documentation and the test landed as described, and
  `cargo test -p nx-cli` passes — but the verification accepted them without establishing that the
  emitted contract *works*, and it does not. `generates_a_csharp_delegate_for_a_function_type`
  asserts only that the string `global::System.Delegate` appears in the output; it never
  round-trips a value. Compiling the generated shape against MessagePack 3.1.7 and
  `System.Text.Json` on net10.0 shows:
  - **MessagePack** — the SDK's *canonical* payload format (`NxOutputFormat.MessagePack = 0`) —
    fails with `FormatterNotRegisteredException: System.Delegate is not registered in resolver:
    MessagePack.Resolvers.StandardResolver`. The resolver builds a formatter for the *containing*
    type, so the failure is type-level: `MessagePackSerializer.Serialize(new Holder())` throws
    **even when the member is null**. Declaring one function-typed prop makes the whole contract
    class — and its generated `_update` record and `NxProperty` accessors — unserializable in the
    canonical format.
  - **System.Text.Json** explicitly blocks the type: `NotSupportedException: Serialization and
    deserialization of 'System.Delegate' instances is not supported`, on both read and write.
    Only `null` round-trips, so the member can never carry a value.

  So this is not a weakly-typed but functional member. It is a member that cannot hold a value in
  either format, plus a containing type that cannot be serialized at all in the default one. The
  design note's reasoning ("a host binds the template through the runtime, not through the
  generated contract") is true of the *template* but does not rescue the record that declares it.

  It is also inconsistent with the `ActionHandler` precedent the design invokes for runtime-only
  values with a public rendering: `ActionHandler` has a real serializable DTO in the SDK —
  `NxActionHandlerRef` (`[MessagePackObject]`, `action` + `token`,
  [NxActionHandlerRef.cs](bindings/dotnet/src/NxLang.Sdk/NxActionHandlerRef.cs)) that a host types
  a property with to read the record. The `Function` record has no counterpart.

  Separately, and correctly out of scope: even with a working C# type a .NET host could only
  *observe* a function value, never call it or pass one back. There is no Rust/FFI `callFunction`
  (an explicit Non-Goal), and `from_nx_value` refuses `Function` records — which is the decoder for
  component props, state and dispatch arguments
  ([component.rs:123](crates/nx-api/src/component.rs#L123),
  [component.rs:191-195](crates/nx-api/src/component.rs#L191-L195)). That part of the gap is
  deliberate and documented; the serialization break is not.
- **Revised recommendation:** mirror the `ActionHandler` precedent — add an `NxFunctionRef` DTO to
  `NxLang.Sdk` (`[MessagePackObject]`, `module` + `name`, matching the canonical record) and map
  `TypeRef::Function` to it in `csharp.rs`, replacing `global::System.Delegate`. That is a small
  change, makes the contract serializable in both formats, and lets a host read which template it
  was handed, while leaving "call it from C#" the Non-Goal it already is. Then widen the typegen
  test to round-trip a `Function` record through both `MessagePackSerializer` and
  `JsonSerializer`, since asserting on the emitted type name is what let this through. If even that
  is out of scope for this change, the C# backend should emit its "unsupported member" diagnostic
  for a function-typed member instead, so a contract that cannot serialize is never generated
  silently.
- **Second fix:** took the revised recommendation. `NxLang.Nx.NxFunctionRef`
  ([NxFunctionRef.cs](bindings/dotnet/src/NxLang.Sdk/NxFunctionRef.cs)) is the rendered `Function`
  record — `[MessagePackObject]`, `module` + `name`, `[JsonPropertyName]` on both — exactly as
  `NxActionHandlerRef` is the rendered `ActionHandler` record, and both C# mapping sites in
  `csharp.rs` now emit `global::NxLang.Nx.NxFunctionRef` instead of `global::System.Delegate`,
  with a comment saying why. Calling a function value from .NET stays the Non-Goal it was.
  - The typegen test no longer asserts on a type name alone: it checks the emitted members
    (`NxFunctionRef? RowTemplate`, `NxFunctionRef HeaderTemplate`) and that `System.Delegate`
    appears nowhere.
  - `NxFunctionValueTests` ([bindings/dotnet](bindings/dotnet/tests/NxLang.Sdk.Tests/NxFunctionValueTests.cs))
    does the round trip the last pass was missing: evaluate a module whose `List` carries
    `ItemTemplate={Row}` into a typed element and read `templates.nx` / `Row`; the same evaluation
    as JSON showing the `Function` record; serialize and deserialize the element through
    `MessagePackSerializer` **and** `JsonSerializer`; and the same two formats with the member
    null, which is the case `System.Delegate` failed at type level. `dotnet test
    bindings/dotnet/NxLang.sln`: 142 passed, 0 failed (138 before, plus these four).
  - The artifacts follow: a `dotnet-binding` delta spec with the three scenarios, task 8.6, the
    proposal's Impact (typegen → C# and the new DTO), a rewritten design note that records the
    serialization failure rather than the old "out of scope" reasoning, and a
    *Function Values in Rendered Output* section in `bindings/dotnet/README.md` beside the handler
    one.
- **Second verification:** Confirmed, and the gap is closed at the root. Independent check, the same
  probe that condemned `System.Delegate`, recompiled against the new shape (MessagePack 3.1.7 +
  `System.Text.Json`, net10.0): STJ deserialize of a `Function` record, STJ round trip, STJ with a
  null member, MessagePack round trip, and **MessagePack with a null member** — the type-level case
  that previously threw `FormatterNotRegisteredException` — all five pass, round-tripping as
  `{"row":{"module":"app/main.nx","name":"Row"}}` and `{"row":null}`.
  - `NxFunctionRef` mirrors `NxActionHandlerRef` field for field
    ([NxFunctionRef.cs](bindings/dotnet/src/NxLang.Sdk/NxFunctionRef.cs)): `[MessagePackObject]`,
    `[Key]` + `[JsonPropertyName]` on `module` and `name`, and a remark that says a host reads
    which function it was handed and cannot call it.
  - Both mapping sites in `csharp.rs` emit `global::NxLang.Nx.NxFunctionRef`
    ([csharp.rs:1143](crates/nx-cli/src/typegen/languages/csharp.rs#L1143),
    [csharp.rs:1315](crates/nx-cli/src/typegen/languages/csharp.rs#L1315)) with a comment naming
    the serialization failure as the reason. Regenerating the sample confirms it: `DataTable`
    declares `global::NxLang.Nx.NxFunctionRef? RowTemplate`, and no `System.Delegate` remains.
  - The typegen test is now `generates_a_csharp_function_reference_for_a_function_type` and asserts
    the emitted *members* plus `!csharp.contains("System.Delegate")`, rather than a type name —
    which is the hole that let the first pass through.
  - `NxFunctionValueTests` drives the real SDK, not a mock: `NxRuntime.Evaluate<T>` through the FFI
    into a typed element, `EvaluateJson` for the `Function` record, both-format round trip, and the
    null-member case carrying a comment that names the type-level resolver behaviour.
    `dotnet test bindings/dotnet/NxLang.sln`: **142 passed, 0 failed** (138 before, plus these
    four). `cargo fmt --all --check` clean, `cargo test -p nx-cli` 204 passed, workspace tests
    green, `openspec validate add-function-types` reports valid.
  - Artifacts check out: the `dotnet-binding` delta adds three scenarios that match what the tests
    do (including "both SHALL succeed when the property is null"), task 8.6 is present and done
    (41/41), the design note now records the serialization failure instead of the old "out of
    scope" reasoning, and `bindings/dotnet/README.md` documents the type and states that a
    `Function` record supplied back as a prop, state, or inside an action is refused — which
    matches `from_nx_value`. One loose end is filed as RF13 below.

### 🔴 Open - RF10 `docs/scratch-highlighting.nx` is a leftover scratch file that says it should be deleted

- **Severity:** Low
- **Evidence:** Untracked file whose own header reads "Scratch file for visually checking NX syntax
  highlighting. Not real UI, and not meant to be kept — delete when done testing." It predates this
  change (it references an earlier highlighting design), but it sits in the working tree alongside
  this change's new files and would be swept in by `git add -A`.
- **Recommendation:** Delete it, or add it to `.gitignore`, before committing.
- **Status:** left open. The file is untracked, predates this change and belongs to the earlier
  highlighting work, so deleting it is the author's call rather than this change's; nothing
  commits it unless `git add -A` sweeps it in. The recommendation stands: `rm
  docs/scratch-highlighting.nx` before committing, or ignore it.
- **Verification:** Still open by agreement — `docs/scratch-highlighting.nx` is present in the
  working tree and still untracked. Nothing in this change depends on it either way; it is a
  one-line cleanup for whoever commits.
- **Verification (2026-09-18 14:35):** Unchanged and still open by agreement — the file is present
  (`docs/scratch-highlighting.nx`, untracked, last touched 2026-09-13) and `git status` still lists
  it under `??`. Not a blocker; `rm` it or ignore it when committing.
- **Status (asked and confirmed):** the author chose to leave it open — the file stays in the
  working tree, untracked, and is deleted or ignored at commit time rather than by this change.

### ✅ Verified - RF11 `examples/nx/templates.nx` declares a type alias it never uses, under a name it also uses as a property

- **Severity:** Low
- **Evidence:** `type FooterTemplate = <function Count:int />: DrawnNode` is declared but
  `DataTable` spells its `FooterTemplate` prop inline as
  `(<function Count:int />: DrawnNode)?`. The file is the language-level teaching example for this
  feature (task 8.4), so a dead alias that shares its name with a property in the same file works
  against it — `ContactRowTemplate` beside it is used and reads correctly.
- **Recommendation:** Use the alias in `DataTable` (`FooterTemplate:FooterTemplate?`) or drop the
  alias; if kept, rename it so it does not shadow a property name in the same file.
- **Fix:** the alias is used and no longer shadows a property name: `type TableFooterTemplate`
  (renamed, beside the `TableFooter` function it types), and `DataTable` declares
  `FooterTemplate:TableFooterTemplate?`. The comment above `DataTable` now points out that a
  property may name an alias or spell the function type inline. `nxlang run
  examples/nx/templates.nx` evaluates as before.
- **Verification:** Confirmed. `examples/nx/templates.nx` now declares
  `type TableFooterTemplate` beside the `TableFooter` function it types, `DataTable` uses it
  (`FooterTemplate:TableFooterTemplate?`), no alias is dead, and no alias shares a name with a
  property. The comment above `DataTable` points out that a property may name an alias or spell the
  function type inline, which is the teaching point. `nxlang run examples/nx/templates.nx`
  evaluates.

### ✅ Verified - RF12 A failing template calls React `setState` once per cell bind, from inside DrawnUI's layout pass

- **Severity:** Low
- **Evidence:** `NxTemplateCell.OnBindingContextChanged` calls `reportTemplateFailure` on every
  failing bind ([templateCell.ts:60-66](sites/playground/src/render/templateCell.ts#L60-L66)), and
  `useNxDrawing` implements it as a `setDrawing` that appends unless the exact string is already
  present ([useNxDrawing.ts:143-152](sites/playground/src/render/useNxDrawing.ts#L143-L152)). The
  string carries the item index, so each of the visible cells produces a distinct entry, and
  `.slice(-EFFECTS_KEPT)` evicts older entries, which lets an already-reported index re-enter and
  re-render later. Scrolling a long list whose template fails therefore re-renders the tree per cell
  bind, during layout. Correctness is fine — the cell is cleared first and left empty — and the
  editor path the task verified shows the index as intended.
- **Recommendation:** Keep a `Set` of reported keys outside React state (as `reportUnknown` and
  `reportInert` already do with `unknown` / `inert`) and only push to state for keys not seen in the
  current session, or cap reporting per template name rather than per index.
- **Fix:** `draw` now keeps a `templateFailures` set beside the `unknown` and `inert` sets, and
  `reportTemplateFailure` returns early for a key already told
  ([useNxDrawing.ts](sites/playground/src/render/useNxDrawing.ts)), so an entry evicted by
  `.slice(-EFFECTS_KEPT)` cannot re-enter and set state again on a later scroll. The state update
  keeps its own `includes` guard.
- **Verification:** Confirmed, as the first of the two options the finding offered. `draw` keeps a
  `templateFailures` set beside the existing `unknown` and `inert` sets and
  `reportTemplateFailure` returns early on a key already told
  ([useNxDrawing.ts](sites/playground/src/render/useNxDrawing.ts)), so a key evicted by
  `.slice(-EFFECTS_KEPT)` can no longer re-enter and set state again on a later scroll. The set is
  per-draw, so it is discarded with the session, which is right. Residual, and inherent to
  reporting per index: the number of *distinct* keys is still unbounded over a long failing list,
  so a first pass down 100 000 failing cells still sets state once per index — one update per cell
  rather than per bind. That is what the finding asked for; capping per template name instead
  remains available if it ever shows up in a profile.

## New Findings Discovered During 2026-09-18 14:23 Verification

### ✅ Verified - RF13 The proposal does not list `dotnet-binding` among the capabilities the change modifies

- **Severity:** Low
- **Evidence:** RF9's second fix added `openspec/changes/add-function-types/specs/dotnet-binding/spec.md`,
  a delta over the existing `dotnet-binding` capability. Every other delta directory in this change
  has a matching bullet in the proposal's **Capabilities** section — `type-reference-suffixes`,
  `component-type-parameters`, `nx-ir-format`, `typescript-ir-runtime`, `drawnui-nx-catalog`,
  `playground`, `editor-syntax-highlighting`, `editor-language-service` under *Modified*, and
  `function-types` / `function-values` under *New*. `dotnet-binding` has none, so the section a
  reader uses to see what the change touches omits a capability it now changes. The Impact section
  does mention the DTO, and `openspec validate add-function-types` passes, so nothing catches it
  mechanically.
- **Recommendation:** Add a bullet under **Modified Capabilities**, e.g. "`dotnet-binding`: a
  function value is read through `NxFunctionRef`, and generated C# types a function-typed member
  with it rather than a delegate."
- **Fix:** added under **Modified Capabilities** in `proposal.md`, in the same one-line form as its
  neighbours: "`dotnet-binding`: a function value is read through `NxFunctionRef`, and generated C#
  types a function-typed member with it rather than a delegate, which neither output format can
  serialize." `openspec validate add-function-types` still reports valid.
- **Verification:** Confirmed, and cross-checked rather than eyeballed: the change's eleven delta
  spec directories and the eleven bullets in the proposal's **Capabilities** section now correspond
  one-to-one — no delta without a bullet, no bullet without a delta. The `dotnet-binding` bullet
  sits under *Modified Capabilities* in its neighbours' one-line form and describes what the delta
  and the code actually do (`NxFunctionRef`, and why not a delegate). `openspec validate
  add-function-types` reports valid at 41/41 tasks. No source file is newer than `proposal.md`, so
  this pass changed documentation only and no suite needed re-running.

## Verification pass (2026-09-18 14:35)

RF13 verifies; RF10 is unchanged and still open by agreement. Nothing else moved — `proposal.md` is
the newest file in the change apart from this report, and no `.rs`, `.ts`, `.cs` or `.nx` file is
newer, so the pass was documentation-only and the suites from the 2026-09-18 14:23 pass still stand.
`openspec validate add-function-types` reports valid at 41/41.

No new findings. All thirteen findings are now settled: twelve fixed and verified, RF10 left to the
author.

## Questions

- **Should a top-level `let` of function type be invocable as an element at all?** RF2 assumes yes,
  because the checker's own doc comment says so and the interpreter agrees, but the `function-values`
  requirement enumerates "a parameter, a component prop, or a local `let`". The answer decides
  whether RF2's fix belongs in codegen or in the checker.
  - **Answered:** yes, it is invocable. The fix went into codegen, and the `function-values`
    requirement now reads "a parameter, a component prop, or a `let`, local or top-level", with a
    scenario for the top-level form.
- **Is the `System.Delegate` mapping (RF9) intentionally out of scope?** The proposal's Impact names
  only `crates/nx-cli/src/typegen` → TypeScript, but the design records no decision about C#.
  - **Answered (superseded):** first taken as out of scope with a design note and a test.
    Verification showed the delegate cannot serialize in either supported format, so the mapping
    changed: a function-typed member is now `NxFunctionRef`, the `Function` record's managed DTO.
    The gap that remains — a .NET host cannot *call* a function value — is the documented Non-Goal.
- I did not re-run the .NET / FFI smoke tests that task 4.4 claims (`bindings/dotnet/NxLang.sln`
  needs a fresh `nx-ffi` build first). Everything below them — `nx-api` rendering, the IR image
  tables, `cargo test --workspace` — passes, and images are passed through unchanged there, so the
  risk looks low, but the claim is unverified in this review.
  - **Answered:** run during this fix pass — `cargo build -p nx-ffi` then `dotnet test
    bindings/dotnet/NxLang.sln`: 138 passed, 0 failed.
  - **Verified:** re-run independently during verification — `cargo build -p nx-ffi` then
    `dotnet test bindings/dotnet/NxLang.sln`: `Failed: 0, Passed: 138, Skipped: 0`, build clean.
    All three answers above are confirmed by the code, the specs and the tests cited in the
    matching findings; no question remains open.

## Verification pass (2026-09-18 03:52)

(First pass.) Eleven of the twelve findings were marked fixed. **Ten verify; RF9 is reopened** — see its
verification note: the documentation and test landed, but the emitted `System.Delegate` cannot be
serialized by MessagePack (the SDK's canonical format) or `System.Text.Json`, and the MessagePack
failure is type-level, so it breaks the whole containing contract even with the member null. That
was established after the first sign-off, by compiling the generated shape against the SDK's own
package versions; the original verification took the test at face value and the test only asserts
on the emitted type name. RF10 stays open by agreement — the scratch file is still in the working
tree, untracked, and is the author's cleanup.

Re-run during verification, all green: `cargo fmt --all -- --check` (clean, exit 0),
`cargo test --workspace`, `cargo test -p nx-interpreter --test function_values` (10),
`cargo test -p nx-cli` (204), `npm test` in `runtime/typescript` (including the two new
`emitted-ir` cases and the `function-values` corpus program with `identical` / `distinct` in both
the debug and stripped runs), `pnpm test` + `check-examples` in `sites/playground` (20 examples,
and `src/render/materialize.test.mjs` 7/7 including the new nested-template case), `npm test` in
`src/vscode` (256 passing), and `dotnet test bindings/dotnet/NxLang.sln` (138). Both original
repros were re-run through the Rust interpreter *and* the TypeScript runtime on the same emitted
image, and the two now agree.

One finding reopened (RF9). One observation that is not a finding: `check_function_satisfies` trips clippy's
`result_large_err` ([ty.rs:364](crates/nx-types/src/ty.rs#L364)) because `FunctionMismatch` carries
two `Type`s. It is a warning on a diagnostic-only path, clippy is not run in CI, the workspace
already carries ten instances of the same lint, and `fix-clippy-diagnostics` is a change of its
own — so it is noted rather than opened. Every other clippy diagnostic in the workspace is
pre-existing (`nx-value` and `nx-hir` test literals near π, and a missing `criterion` dev-dependency
for the `nx-syntax` bench).

## Verification pass (2026-09-18 14:23)

RF9 was fixed rather than deferred (in a separate session), so nothing was filed to
`specs/future.md`. Its second fix verifies: `NxFunctionRef` replaces `System.Delegate` at both
mapping sites, and the serialization probe that condemned the delegate now passes all five cases,
including MessagePack with a null member — the type-level failure that broke the whole containing
contract. Re-run: `dotnet test bindings/dotnet/NxLang.sln` 142 passed / 0 failed (up from 138),
`cargo test -p nx-cli` 204 passed, `cargo fmt --all --check` clean, workspace tests green, and
`openspec validate add-function-types` reports valid at 41/41 tasks.

One new finding, RF13 (Low): the proposal's Capabilities section does not list `dotnet-binding`
although the change now carries a delta spec for it. No other regression surfaced.

## Fix pass 3 (RF13)

RF13 fixed: `dotnet-binding` now has its bullet under **Modified Capabilities** in `proposal.md`,
so every delta directory in the change has a matching entry. `openspec validate
add-function-types` valid. RF10 stays open by the author's decision, asked and confirmed.

## Fix pass 2 (RF9)

RF9 is fixed at the root rather than re-documented: `TypeRef::Function` maps to a new
`NxLang.Nx.NxFunctionRef` DTO instead of `global::System.Delegate`, mirroring `NxActionHandlerRef`,
and four `NxFunctionValueTests` cases round-trip it through MessagePack and `System.Text.Json`
— with a value and with the member null — after reading it out of real rendered output. See RF9's
**Second fix** note. `cargo fmt --all --check` clean, `cargo test -p nx-cli` 204 passed,
`dotnet test bindings/dotnet/NxLang.sln` 142 passed. RF10 remains the author's one-line cleanup.

## Summary

The implementation is thorough and closely matches the proposal and design: one function type with
named parameters threaded through the parser, HIR, checker, interpreter, `nx-api`, IR, both
runtimes, typegen, the catalog generator, the playground renderer, the editors and the docs, with
real tests at each layer and a conformance program covering the end-to-end path. The design's
implementation notes honestly record where it departed from the plan (content splicing in the
TypeScript runtime, interpreter type-parameter erasure, the function-parameter scope fix, the
collection-building idiom), and every test suite I ran is green.

**Fix pass (this session):** eleven of the twelve findings are fixed and await verification; RF10
(the untracked scratch file) is left to the author. Re-run after the fixes: `cargo fmt --all
--check` clean, `cargo test --workspace` green, `npm test` in `runtime/typescript` green (249 ok),
`pnpm test` + `check-examples` + `typecheck` in `sites/playground` green, and `dotnet test
bindings/dotnet/NxLang.sln` green (138). The `function-values` conformance corpus was regenerated
with `NX_UPDATE_CORPUS=1` for the two new entrypoints.

**Verified:** all twelve original findings are now settled — eleven fixed and verified, RF10 open
by agreement. RF9 took two passes. The two findings that mattered are closed at the
root, not papered over — the Rust interpreter and the TypeScript runtime now give the same answer
for function-value equality (RF1) and for calling a top-level `let` of function type (RF2), each
pinned by a test on both sides and by a conformance entrypoint or an `emitted-ir` case that compares
the two runtimes directly. RF3 is mechanically clean. Where the finding offered a choice, the fix
took the branch the review recommended and said so: RF6 amended the spec rather than the artifact,
RF12 took the seen-keys set. RF9 first took the "record and pin" branch, which rested on a false premise —
`System.Delegate` is not a workable C# mapping but an unserializable one — and the second pass
replaced it with `NxFunctionRef`, mirroring the SDK's existing `NxActionHandlerRef`. That is the
right shape, and the probe that condemned the delegate now passes on all five cases. Two fixes went further than asked — RF4 now handles parentheses nested to any depth,
and RF8 retired a duplicated helper from the language service as well as consolidating the spelling.

One item remains, and it does not block the work: **RF10**, an untracked scratch file from earlier
highlighting work that this change neither created nor depends on — `rm
docs/scratch-highlighting.nx` before committing, or ignore it. Everything else is fixed and
verified, and the change is ready to archive.
