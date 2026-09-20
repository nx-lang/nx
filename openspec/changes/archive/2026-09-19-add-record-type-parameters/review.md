# Review: add-record-type-parameters

## Scope

**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md` (44/44 marked complete),
`specs/record-type-parameters/spec.md`, `specs/cli-code-generation/spec.md`,
`specs/nx-ir-format/spec.md`

**Reviewed code:** the full working tree against `HEAD` (64 modified files + 5 new files).
Focus areas:

- Grammar and validation: `crates/nx-syntax/grammar.js`, `src/validation.rs`, `src/syntax_kind.rs`,
  `queries/highlights.scm`, `tests/parser_tests.rs`
- HIR: `crates/nx-hir/src/ast/types.rs`, `src/lower.rs`, `src/records.rs`, `src/components.rs`,
  `src/prepared.rs`, `src/lib.rs`
- Checker: `crates/nx-types/src/infer.rs`, `src/ty.rs`, `src/semantics.rs`,
  `tests/record_type_parameters.rs`
- Below the checker: `crates/nx-interpreter/src/interpreter.rs`, `src/context.rs`,
  `crates/nx-codegen/src/builder.rs`, `src/emit.rs`, `src/model.rs`, `src/ir_tests.rs`,
  `specs/ir-conformance/generic-records/`, `runtime/typescript/test/emitted-ir.test.mjs`
- Codegen surfaces: `crates/nx-cli/src/typegen.rs`, `typegen/model.rs`,
  `typegen/languages/{csharp,typescript}.rs`
- Library interface: `crates/nx-api/src/artifacts.rs`
- Language service: `crates/nx-language-service/src/{lib,positions,hover}.rs`
- .NET: `bindings/dotnet/tests/NxLang.Sdk.Tests/NxGenericRecordTests.cs`, `Generated/UpdateRecords.g.cs`
- Clippy cleanup and the CI gate: `.github/workflows/build.yml`, `crates/nx-ffi/src/lib.rs`, and the
  mechanical lint fixes across the workspace

**Verification run during this review (all green):** `cargo test --workspace`,
`cargo clippy --workspace --all-targets -- -D warnings` (exit 0), `pnpm -r build`, `pnpm -r test`,
`dotnet test bindings/dotnet/NxLang.sln` (145 passed).

Overall the change is well built: the grammar, HIR, checker and erasure layers line up with the
design, the diagnostics are worded carefully, and the test suite is broad. The findings below are
holes the existing tests do not reach, found by running the code rather than by reading it.

## Findings

### ✅ Verified - RF1 `diff` does not compare type arguments, so two different instantiations diff without a diagnostic and the resulting value carries the wrong field types

- **Severity:** High
- **Evidence:** `crates/nx-types/src/infer.rs:2475` still gates `diff` on
  `before.is_same_declaration_as(&after)`, which by design ignores `NamedType::args`
  (`crates/nx-types/src/ty.rs:990`). `apply` (`infer.rs:2429`) and `merge` (`infer.rs:2455`) were
  both tightened to full `NamedType` equality in this change; `diff` was not. The result type then
  copies the *first* operand's arguments via `carry_type_arguments` (`infer.rs:2500`).

  Reproduced end to end (checker reports zero errors, interpreter produces the value):

  ```nx
  type Range = { T:type start:T end:T }
  let a() = {<Range T=int start={1} end={5} />}
  let b() = {<Range T=string start="x" end="y" />}
  let bad():<Range T=int/> = {apply(a(), diff(a(), b()))}
  ```

  `check_str` → `errors: []`; evaluating `bad()` →
  `Record { type_name: "Range", fields: {"start": String("x"), "end": String("y")} }`.

  A value annotated `<Range T=int/>` holds two strings. This contradicts the
  `record-type-parameters` requirement "Applied types are distinct per argument and invariant" and
  the intrinsics requirement that `diff` produce `<R.Update T=A/>` for a value of `<R T=A/>`.
- **Recommendation:** Change the `diff` guard to `before != after` (the same comparison `merge`
  now uses) and spell both operands with `Type::Named(...)` in the message, as `apply` and `merge`
  do. Add a case to `record_type_parameters.rs::intrinsics_carry_the_argument` asserting
  `diff(<Range T=int/>, <Range T=string/>)` is rejected. Consider whether `changed` needs the same
  treatment — `changed(diff(a, b))` is currently also accepted.
- **Fix:** The `diff` guard is now `before != after`, the same full `NamedType` comparison `merge`
  uses, and the message spells both operands with `Type::Named(...)` so it reads
  `'<Range T=string/>' is not '<Range T=int/>'`. For a non-generic record the two comparisons agree,
  so nothing else changed. `intrinsics_carry_the_argument` gained the rejection case. `changed`
  needs no equivalent: it takes one update and has nothing to compare it against — the example that
  looked accepted was `diff`'s result being accepted.
- **Verification:** ✅ Confirmed. `infer.rs` now gates `diff` on `before != after` and spells both
  operands. Re-ran the original repro: `diff(<Range T=int/>, <Range T=string/>)` reports
  `Intrinsic 'diff' takes two records of one type: '<Range T=string/>' is not '<Range T=int/>'`,
  where it previously reported nothing. `diff(a, a)` and a non-generic `diff` are both still clean,
  so the tightening costs nothing for the pre-existing cases (for a non-generic record the two
  comparisons are identical, as the fix says). The rejection case is in
  `intrinsics_carry_the_argument`. The note on `changed` is right: it takes one update, so there is
  nothing to compare — the accepted `changed(diff(a, b))` in the original evidence was `diff`'s
  acceptance, which is now gone.

### ✅ Verified - RF2 An applied type over a record imported from another library generates uncompilable C# and TypeScript

- **Severity:** High
- **Evidence:** Both emitters look the record's parameter order up in the *current* library's
  export graph — `graph.resolve_record(name)` in
  `crates/nx-cli/src/typegen/languages/typescript.rs:754` and
  `crates/nx-cli/src/typegen/languages/csharp.rs:1179`. A record imported from a dependency library
  is not in that graph (it lives in `imported_types_by_visible_name`), so `order` is empty,
  `rendered` is empty, and both emitters fall through to `return base` — the bare record name with
  no type arguments. `csharp_imported_alias_target_type` (`csharp.rs:1375`) does the same for an
  imported alias whose target is an applied type, dropping the arguments outright.

  Reproduced with a two-library fixture (library `ranges` exports
  `type Range = { T:type start:T end:T }` and `type IntRange = <Range T=int/>`; library `app`
  imports it and declares `type Slider = { week:<Range T=int/> alias:IntRange }`):

  ```csharp
  public global::Test.Ranges.Range Week { get; set; } = default!;   // CS0305
  public global::Test.Ranges.Range Alias { get; set; } = default!;  // CS0305
  ```
  ```ts
  export interface Slider extends NxRecord<"Slider"> {
    week: Range;   // TS2314: Generic type 'Range<T>' requires 1 type argument(s)
    alias: IntRange;
  }
  ```

  The same-library, cross-module case works correctly (`Range<long>` / `Range<number>`), so the gap
  is specifically the dependency-library boundary. The proposal states "A generic record crosses a
  library boundary with its type parameters, so an importing module can apply and construct it",
  and the checker honours that (`artifacts.rs` tests pass) — only codegen does not.
- **Recommendation:** Resolve the parameter order through the imported-type lookup as well as the
  local graph in all three emitters (`typescript.rs::ts_type`, `csharp.rs::csharp_type_inner`,
  `csharp.rs::csharp_imported_alias_target_type`), carrying `type_params` on `ImportedType` the way
  `ExportedRecord` does. Separately, make the `rendered.is_empty() → base` fallback not silently
  emit an open generic: when the record is known to have parameters but none resolved, that is a
  bug, not a valid rendering — emit a typegen warning or erase to `object`/`unknown` deliberately.
  Add a cross-library typegen test for both languages (none exists today).
- **Fix:** `ImportedTypeKind::Record` now carries the record's `type_params`, read from the
  dependency's `InterfaceItemKind::Record`, and the new `ExportedTypeGraph::record_type_params`
  answers from the local export graph or the imported entry. `ts_type`, `csharp_type_inner` and
  `csharp_imported_alias_target_type` all go through it; the last one now renders the instantiation
  rather than dropping the arguments, because C# has no type aliases and expands the target at
  every use. The fallback is no longer an open generic: where the declaration cannot be reached at
  all, the arguments render in source order, since a checked program wrote them against a
  declaration that does exist and a bare name is CS0305/TS2314. A two-library typegen test
  (`an_applied_type_over_a_record_imported_from_a_dependency_keeps_its_arguments`) covers both
  languages, including the imported alias. Design D7 and a new task 6.6 record the scope question:
  the dependency boundary is in scope, as the proposal's last bullet promises.
- **Verification:** ✅ Confirmed, and it holds beyond what the new test covers. Re-ran the original
  two-library repro with a wildcard `import "../ranges"` (the test uses a selective import) and
  with extra shapes: C# emits `global::Test.Ranges.Range<long> Week`,
  `global::Test.Ranges.Pair<string, long> P` (declaration order, though the source wrote
  `TValue` first), `global::Test.Ranges.Range<global::Test.Ranges.Range<double>>[]? Nested`, and
  expands the imported alias to `Range<long>`; TypeScript emits `week: Range<number>`,
  `p: Pair<string, number>`, `nested: Range<Range<number>>[] | null` and imports `IntRange` by
  name, which the dependency's own file resolves. A qualified import alias
  (`import { Range as Rng.Range }`) also resolves, emitting `Rng_Range<number>` / `Range<long>`.
  `ExportedTypeGraph::record_type_params` returning `Option` — `None` for unreachable, `Some([])`
  for reached-and-non-generic — is the right shape for distinguishing the fallback from a plain
  record, and the fallback now renders source order rather than an open generic.

### ✅ Verified - RF3 Applied-type and missing-argument diagnostics are reported at file offset 0 everywhere except record and union-case field types

- **Severity:** Medium
- **Evidence:** `InferenceContext::type_ref_span` (`crates/nx-types/src/infer.rs:345`) is
  initialised to `TextSpan::new(0, 0)` (`infer.rs:386`) and is only ever assigned in two places —
  `validate_local_record_defaults` (`infer.rs:5045`) and `validate_local_union_defaults`
  (`infer.rs:5074`). Every other type position resolves its `TypeRef` with the span still at 0.
  `require_type_arguments` (`infer.rs:3517`) and `applied_type` (`infer.rs:3551`) both read it.

  Observed spans from `check_str`:

  | source | label range |
  | --- | --- |
  | `type Slider = { range:<Range U=int/> }` | `54..74` ✅ |
  | `let r:<Range U=int/> = …` | `0..0` ❌ |
  | `let f(r:<Range U=int/>): int = 1` | `0..0` ❌ |
  | `external component <Slider range:<Range U=int/> />` | `0..0` ❌ |
  | `let r:Range = …` | `0..0` ❌ |

  In the CLI this underlines the first character of the file; in the editor it puts the squiggle on
  an unrelated declaration. `crates/nx-types/tests/record_type_parameters.rs` only asserts on
  message text, and `a_bare_generic_record_name_is_a_diagnostic_in_every_type_position` walks
  exactly these positions without checking where the diagnostic landed — which is why this is not
  caught.
- **Recommendation:** Set `type_ref_span` around every conversion whose caller knows a span: value
  definitions (`ValueDef::span` / the annotation node), function parameters and return types,
  component props and state fields. A guard test asserting the label range is inside the annotation
  for each of the positions already listed in
  `a_bare_generic_record_name_is_a_diagnostic_in_every_type_position` would lock it down. Consider
  also asserting `type_ref_span != 0..0` in debug builds so a new caller cannot silently regress.
- **Fix:** Added `type_from_type_ref_at` / `type_from_type_ref_in_at`, which scope
  `type_ref_span` around one conversion, and used them at every position that knows a span: value
  definitions, function parameters and return types, component props (only where this module
  declared the prop — an inherited one's span belongs to the base's file) and state fields, and the
  two record/union field loops that were doing it by hand. Each of the table's rows now underlines
  its own annotation. Fixing the spans exposed that a function's annotations were resolved three
  times and reported three times; `infer_function` now reuses the parameter types it already
  resolved instead of re-resolving them through `bind_function_signature`, and the signature
  pre-pass resolves quietly (`type_from_type_ref_in_quietly`) because the pass that knows the span
  reports the same problem. The new
  `a_diagnostic_about_a_type_reference_lands_on_the_declaration_that_wrote_it` asserts both the
  count and that every primary label falls inside the declaration under test. The suggested
  debug-build assertion that `type_ref_span != 0..0` was not added: plenty of legitimate
  conversions (an alias target, a foreign signature) have no span to give, so it would fire on
  correct code; the placement test covers the same ground without that risk.
- **Verification:** ✅ Confirmed for every position the finding enumerated. Re-ran the evidence
  table: record field `54..74`, `let` annotation `38..94`, function parameter `44..60`, function
  return `38..102`, component prop `65..85`, component state field `72..88`, bare name in a `let`
  `38..85`, union payload `61..77`. No `0..0` remains among them. Also checked positions the table
  did not list — an `action` field, an emitted action's field, an element function's parameter, an
  alias target and a nested applied type inside a function type — all land on their own
  declaration. Regressions checked: an alias cycle and the pre-existing record-default mismatch are
  still reported exactly once.

  Two notes, neither a reopen. First, for a value definition the label is the whole declaration
  rather than just the annotation, so "underlines its own annotation" in the fix note is loose —
  but `ValueDef::span` is one of the two options the recommendation named, and it is a large
  improvement over offset 0. Second, the `type_from_type_ref_in_quietly` approach of truncating
  `self.diagnostics` is blunt but safe here: the only diagnostics a type-reference conversion
  raises are about the reference, and `infer_function` re-resolves every local annotation with a
  span, so nothing is lost — verified by the alias-cycle case still reporting.

  The fix does not reach *use sites* of a declaration whose type reference is malformed; those
  still re-resolve and re-report, at offset 0 where the caller has no span. That is a separate
  mechanism from the one this finding identified, so it is filed as RF10 rather than reopening
  this.

### ✅ Verified - RF4 A malformed applied type in a record field or component state field is reported twice

- **Severity:** Medium
- **Evidence:** `validate_local_record_defaults` (`crates/nx-types/src/infer.rs:5046`) now resolves
  every field's `TypeRef` whether or not it has a default, and it iterates
  `self.module.raw_module().items()` — which includes the synthesized `<Target>.Update` records
  that `predeclare_update_record` (`crates/nx-hir/src/lower.rs:731`) built by copying the target's
  field types. Each malformed field type is therefore resolved once for the declaration and once
  for its companion.

  ```
  type Range = { T:type start:T end:T }
  type Slider = { range:<Range U=int/> }
  ```
  → 4 errors, each of the two messages printed twice at the same span. Same for a component state
  field (`state { s:<Range U=int/> }`). A union payload field, which has no `.Update` companion,
  correctly reports once. The pre-existing "Default value for record property …" diagnostic reports
  once, so this duplication is new.
- **Recommendation:** Skip records that have an `update_target()` in the resolve-for-diagnostics
  loop — their field types are copies whose problems belong to the target — while still resolving
  them where the shape is actually needed. Add a regression test asserting exactly one diagnostic
  for `type Slider = { range:<Range U=int/> }`.
- **Fix:** `validate_local_record_defaults` skips a record with an `update_target()`. Its fields
  are copies of its target's with the defaults stripped, so the loop had nothing of its own to
  check and only re-reported the target's field-type problems. `type Slider = { range:<Range U=int/> }`
  and the component-state case now report each message once; the count is asserted by the new
  placement test in RF3.
- **Verification:** ✅ Confirmed. `validate_local_record_defaults` skips a record with an
  `update_target()`, and the skip is safe: an `action` declaration and an emitted action are
  themselves `Item::Record` with no update target, so their field types are still resolved (checked
  — both report once, on their own field). `type Slider = { range:<Range U=int/> }` and the
  component-state case now report 2 diagnostics (one per distinct message) where they reported 4.
  As with RF3, duplication arising from a *use site* re-resolving the declaration's type reference
  is a different mechanism and is filed as RF10.

### ✅ Verified - RF5 The generated C# update companion of a generic record is typed `NxUpdate<Range<object>>` and cannot be used with any real instantiation

- **Severity:** Medium
- **Evidence:** `bindings/dotnet/tests/NxLang.Sdk.Tests/Generated/UpdateRecords.g.cs` now contains:

  ```csharp
  public sealed class Range_update : NxUpdate<Range<object>>
  public static Range_update Diff(Range<object> before, Range<object> after) => …
  public static readonly NxProperty<Range<object>, object> Start = …
  ```

  C# classes are invariant, so a host holding a `Range<long>` — the type `Schedule.Week` actually
  uses — cannot call `Range_update.Diff`, cannot pass it to `RangeProperties.Of`, and cannot apply
  the patch. Every other generated companion in the suite is usable
  (`Doc_update.Diff(circle, square)`, `Ticker_update.Diff(state, applied)`,
  `User_update.Diff(user, partial)` in `NxUpdateRecordTests.cs`); this one is reachable only for the
  `Range<object>` instantiation, which nothing produces.

  The erasure itself matches the `cli-code-generation` requirement ("SHALL erase the record's type
  parameters … to `object` in C# … with no generic parameter"), but that requirement was written by
  analogy to a component's companion, where the patched type (`Ticker_state`) is concrete. Here the
  patched type is the generic record itself, so erasure produces a *different* closed type rather
  than an erased view of the same one. `NxGenericRecordTests.cs` only round-trips `Range<T>` and
  `Schedule` through the two serializers and never touches `Range_update`, so nothing catches it.
- **Recommendation:** Decide the intended C# shape before archiving — either declare the companion
  generic (`Range_update<T> : NxUpdate<Range<T>>`, keeping the wire format parameter-free) or state
  explicitly in the spec and in `bindings/dotnet/README.md` that a generic record's update companion
  is not usable from C# and why. Either way add a .NET test that exercises `Range_update` the way
  `NxUpdateRecordTests` exercises the others; that test is what would have surfaced this.
- **Status:** Still open — it needs the author's decision to change a written requirement — but no
  longer an open question. Prototyping both candidate shapes against the real SDK settled it:
  **the companion should follow the record and be generic** (`Range_update<T> : NxUpdate<Range<T>>`).
  Three things the finding did not have:
  - **The erasure is not merely awkward, it fails at runtime on the main use case.** A JSON round
    trip of today's `Range_update` stores `end` as a `JsonElement`, because the schema's
    `NxField.ValueType` for a parameter-typed field is `typeof(object)`. Applying that patch to a
    `Range<long>` throws `InvalidCastException` in `NxProperty<TRecord, TValue>.SetValue`'s
    `(TValue)value!`. So "receive a patch, apply it" — the reason the companion exists — does not
    work for any instantiation a program produces. (MessagePack happens to survive, because its
    `object` deserialization yields a real `long`; JSON does not.)
  - **Why the component analogy fails.** A component's companion erases to `object` on *both*
    sides: `Ticker_state.Sel` is `object?` and so is the patch field, so a `JsonElement` flows
    through without a cast. A generic record's fields are `T`, so an erased patch and the record it
    patches disagree, and the disagreement is a cast failure rather than a loss of precision. The
    requirement inherited the component rule by analogy and the analogy does not hold.
  - **It contradicts D7, which already decided this.** D7 stops erasing the record itself in C#
    "because a host names the concrete instantiation at its own deserialization site". That is
    exactly as true of the companion: the host writes `Range_update<long>` where it writes
    `Range<long>`.

  Cost, measured rather than estimated (prototype in the session scratchpad, `protoA/`):
  - The wire format does not change. The prototype emits
    `{"$type":"Range.Update","end":9,"endInclusive":true}` — byte-identical to today, no type
    arguments anywhere — and round-trips through both serializers with the typed read
    (`patch.End.Value` is a `long`) and the wire-apply both correct.
  - `[JsonConverter(typeof(NxUpdateRecordJsonConverter<Range_update<T>>))]` does not compile
    (CS0416, "an attribute argument cannot use type parameters" — verified). The fix is one
    non-generic `NxUpdateRecordJsonConverterFactory : JsonConverterFactory` in the SDK, about ten
    lines, reusable by every companion.
  - MessagePack does work through the attribute, contrary to what the CS0416 result suggests:
    `[MessagePackFormatter(typeof(Range_updateFormatter<>))]` with a small generated shim
    `Range_updateFormatter<T> : IMessagePackFormatter<Range_update<T>>` delegating to
    `NxUpdateRecordMessagePackFormatter<Range_update<T>>`. MessagePack's attribute resolver closes
    the open generic itself when the arity matches.
  - Typegen emits, for a generic record only: the companion as `Range_update<T>`, `RangeProperties`
    as a generic static class `RangeProperties<T>` (`NxProperty<Range<T>, T>`), the formatter shim,
    and the two attributes. Non-generic records are untouched.

  A third shape was prototyped and rejected (`proto/`): keep the companion non-generic and make
  `Apply`/`Diff` generic *methods* over `Range<T>`. It compiles, keeps both attributes unchanged and
  makes `Diff`/`Apply` reachable at a real instantiation — but the schema stays one non-generic
  thing, so `ValueType` stays `object` and the JSON wire-apply still throws. It fixes the signature
  and not the defect underneath it.

  TypeScript should follow for symmetry, but it is not the same problem and should not be described
  as one: a TS `Range_update` is a plain structural interface with `start?: unknown`, so erasure
  there costs precision, not usability. (An earlier version of this note said TypeScript "has the
  same problem"; that was wrong.)

  Either way the finding's test request stands: `NxGenericRecordTests.cs` never touches
  `Range_update`, so nothing in the suite would notice this shape changing. The test to add is a
  JSON round trip followed by an apply to a `Range<long>` — the case that fails today.
- **Fix:** The companion now follows the record in both languages, as the status note
  recommended. `update_companion_type_params` (`typegen/model.rs`) answers with the record's
  parameters for a plain generic record and with none for anything else, so a component's state
  companion still erases — there the patched type is concrete and the analogy that produced the
  erasure rule does hold.
  - **TypeScript:** `export interface Range_update<T>` with `start?: T`. No `= unknown` default,
    for the reason `ts_generic_declaration` already gives for the record.
  - **C#:** `Range_update<T> : NxUpdate<Range<T>>` with `Start` typed `NxOptional<T>`,
    `Diff(Range<T>, Range<T>)` returning `Range_update<T>`, and the key table as
    `RangeProperties<T>` holding `NxProperty<Range<T>, T>`.
  - **The two C# mechanics.** A generic companion cannot name its own converter in an attribute
    (CS0416), so it names a new non-generic `NxUpdateRecordJsonConverterFactory` in the SDK.
    MessagePack's attribute resolver does take an open generic but closes it over the *annotated
    type's* arguments, so typegen emits a shim `Range_updateFormatter<T>` of matching arity that
    delegates to `NxUpdateRecordMessagePackFormatter<Range_update<T>>`; the emitter also brings
    `MessagePack.Formatters` into scope, and only when a generic companion is present. A
    non-generic companion is byte-for-byte what it was.
  - **The wire is unchanged.** A patch is still `{"$type":"Range.Update","end":9,…}` with no type
    argument, which the new .NET tests assert directly.

  Tests: `the_update_companion_of_a_generic_record_carries_the_parameter` (rewritten from the
  erasure test), `a_generic_update_companion_names_a_converter_factory_and_a_formatter_shim`, and
  `a_non_generic_update_companion_keeps_naming_its_converter_directly` in `typegen.rs`; three .NET
  tests in `NxGenericRecordTests.cs`, including
  `GenericUpdateCompanion_RoundTripsThroughBothSerializersAndStillApplies` — the JSON
  round-trip-then-apply that threw `InvalidCastException` under the erased shape. `UpdateRecords.g.cs`
  regenerated. Artifacts updated: the `cli-code-generation` requirement and its scenarios, D7,
  the proposal's companion and typegen bullets and its Impact list (including the SDK addition),
  and tasks 6.7 and 6.8.
- **Verification:** ✅ Confirmed, and the central claims were re-derived independently rather than
  taken from the note. `dotnet test` is 148 passing (was 145). Beyond the three added tests:
  - **The wire really is unchanged.** Serializing a `Range_update<long>` gives
    `{"$type":"Range.Update","end":9}`; the MessagePack bytes decode to the same map with string
    keys and no type argument; and serializing through a static `object` gives the same JSON, so
    the attribute-named factory engages even when the static type is erased at the call site.
  - **The two-parameter case compiles and runs**, which the new typegen tests only assert as text.
    Generated C# for `Pair` into the .NET fixture, compiled and ran it: `Pair_update<string, long>`
    diffs, round-trips through both serializers and applies, with JSON
    `{"$type":"Pair.Update","value":2}`. So `[MessagePackFormatter(typeof(Pair_updateFormatter<,>))]`
    does close a two-arity open generic, not just the one-arity case the fixture covers. The
    fixture and generated file were restored afterwards and
    `checked_in_dotnet_update_record_fixture_matches_typegen_output` passes.
  - **The shapes the tests do not reach hold up.** A nested instantiation
    (`Range_update<Range<long>>`) round-trips and applies; `RangeProperties<long>.Of(...)` returns
    the right key; a null companion round-trips through the formatter shim (`WriteNil` then `null`).
  - **The carve-out is real.** A component's state companion still erases in both languages —
    `Picker_update` with `sel?: unknown | null` in TypeScript and `NxOptional<object?> Sel` in C#,
    with no generic parameter — matching the new spec scenario. A non-generic companion still names
    its converter and formatter directly and does not pull in `MessagePack.Formatters`.
  - `NxUpdateRecordJsonConverterFactory.CanConvert` mirrors the
    `where TRecord : NxUpdateRecord, new()` constraint on the converter it builds, so the
    `MakeGenericType` cannot throw on a type the factory accepted.

  The `cli-code-generation` requirement is rewritten coherently, including the sentence that keeps
  a component's state companion erased and the reason. The status note's diagnosis is borne out by
  the code: `update_companion_type_params` returns the record's parameters only for a plain,
  non-abstract, non-contract record, which is exactly the case where the patched type is generic.

  One gap, filed as RF11: the two prose documents this change wrote about the companion — the NX
  reference and the .NET README — still describe the erased shape.

### ✅ Verified - RF6 The comment on the `APPLIED_TYPE` lowering arm describes behavior the code does not have

- **Severity:** Low
- **Evidence:** `crates/nx-hir/src/lower.rs:1832` says "The tag goes through the same bare-name
  resolution a written type name does, so `<Update T=int/>` inside a component means that
  component's update record." The call is `self.resolve_bare_property_type(name.text())`
  (`lower.rs:2509`), which only rewrites the bare name `Property` (`PROPERTY_UNION_SUFFIX`).
  Rewriting a bare `Update` is `resolve_bare_update_tag` (`lower.rs:2529`), which is not called
  here. No behavior depends on this today — a component's update record is never generic — but the
  comment will mislead the next reader.
- **Recommendation:** Correct the comment to say the tag goes through bare *property-union*
  resolution, or call `resolve_bare_update_tag` as well if the described behavior is wanted.
- **Fix:** Corrected the comment to say the tag goes through bare *property-union* resolution,
  and to say explicitly that a bare `Update` tag is not rewritten here — that is
  `resolve_bare_update_tag`, which belongs to element lowering, and a component's update record has
  no type parameters to apply. Behavior is unchanged, as the finding notes nothing depends on it.
- **Verification:** ✅ Confirmed. The comment now says the tag goes through bare-name resolution
  "which inside a component rewrites a bare `Property` to that component's property union", and
  states explicitly that a bare `Update` tag is not rewritten here because that is
  `resolve_bare_update_tag`. That matches `resolve_bare_property_type`. Comment-only; no behavior
  change.

### ✅ Verified - RF7 `is_visible_type_name` gained `Element`, which also changes component type-argument resolution, with no test and no task line

- **Severity:** Low
- **Evidence:** `crates/nx-types/src/infer.rs:3047` and `:3058` add `nx_syntax::BUILTIN_TYPE_NAMES`
  to `is_visible_type_name` / `visible_type_names`. `is_visible_type_name` is read by the new
  `applied_type` unresolved-argument check (`infer.rs:3620`) *and* by the pre-existing
  `resolve_type_argument` (`infer.rs:2970`), so `<List TItem=Element …/>` at a component use site
  now resolves where it previously reported "is not a visible type". That is plausibly a fix, but it
  is a behavior change outside this change's specs and tasks, and no test in the tree covers
  `Element` as a type argument (`grep '=Element'` finds nothing).
- **Recommendation:** Add a test for `Element` as a type argument on both a component and a generic
  record, and note the change in the proposal's Impact section so archiving records it.
- **Fix:** Added `a_builtin_type_name_is_a_type_argument`, which checks `Element` as an argument
  on a generic record and on a component use site in one clean program, and recorded the behavior
  change in the proposal's Impact section, naming the component use site as the surface that also
  moves.
- **Verification:** ✅ Confirmed. `a_builtin_type_name_is_a_type_argument` covers `Element` as an
  argument on a generic record (`<Box T=Element/>`, both in an annotation and at a construction
  site) and on a component use site (`<Holder TItem=Element .../>`) in one clean program, so it
  pins both surfaces the predicate serves. The proposal's Impact section records the change and
  names the component use site as the surface that also moves.

### ✅ Verified - RF8 The interpreter's top-level-value binding cycle fix is real but undocumented scope, and it swallows errors in nested passes

- **Severity:** Low
- **Evidence:** `crates/nx-interpreter/src/context.rs:100` adds `binding_values` and
  `crates/nx-interpreter/src/interpreter.rs:1446` rewrites `bind_module_values` around it, with four
  new tests in `crates/nx-interpreter/tests/edge_cases.rs:520-584`. The test comment says the old
  behavior "used to be a stack overflow with no diagnostic" for
  `type R = { a:int b:boolean = false }` + `let r = <R a={1} />` — a pre-existing bug unrelated to
  generic records. It appears in no task, no proposal bullet and no design decision. Separately,
  `Err(_) if nested => continue` (`interpreter.rs:1468`) discards a genuine runtime error raised
  during a nested pass; the justification given is that the outer pass will report it, which holds
  for the values the outer pass reaches but is not obviously total.
- **Recommendation:** Add a proposal/design line (or a task under §5) recording the fix so the
  archived change explains why `context.rs` changed. Consider asserting in a test that a real
  evaluation failure inside a field default still surfaces from the outer pass.
- **Fix:** Recorded the fix as task 5.4 and in the proposal's Impact section, saying why it lands
  in this change (the generic-record tests are what first reached it). Added
  `test_a_failure_in_a_value_bound_by_a_nested_pass_is_reported_by_the_outer_pass`, which makes a
  nested pass reach a value that fails, has the nested pass swallow its error, and asserts the
  outer pass still reports it — the totality argument the `Err(_) if nested => continue` arm rests
  on.
- **Verification:** ✅ Confirmed. Task 5.4 and the proposal's Impact section record the fix and say
  why it lands here. `test_a_failure_in_a_value_bound_by_a_nested_pass_is_reported_by_the_outer_pass`
  makes a nested pass reach a value that fails and asserts the outer pass still reports it, which is
  the totality argument the `Err(_) if nested => continue` arm rests on.

### ✅ Verified - RF9 `docs/scratch-highlighting.nx` is an untracked leftover that says it should have been deleted

- **Severity:** Low
- **Evidence:** The file's own header reads "Scratch file for visually checking NX syntax
  highlighting. Not real UI, and not meant to be kept — delete when done testing." Its content is
  about highlight queries from an earlier change, not this one. It is untracked, so a `git add -A`
  before committing this change would sweep it in.
- **Recommendation:** Delete it, or move it under a gitignored scratch path, before archiving.
- **Fix:** Moved out of the tree. It is untracked and has no git history, so rather than delete it
  outright it was moved to this session's scratchpad
  (`scratchpad/scratch-highlighting.nx`) in case it is still wanted; `git status` no longer shows
  it under `docs/`.
- **Verification:** ✅ Confirmed. `git status` no longer lists anything under `docs/` but the two
  intended documentation edits, and the file is preserved at
  `scratchpad/scratch-highlighting.nx` in the fix session's scratchpad.

## New Findings Discovered During 2026-09-18 22:41 Verification

### ✅ Verified - RF10 A use site re-resolves the declaration's type reference and re-reports it, at offset 0 where the caller has no span

- **Severity:** Low
- **Evidence:** RF3 and RF4 fixed the *declaration* passes; the use-site paths were not touched.
  `applied_type` and `require_type_arguments` report unconditionally, so every path that resolves a
  declaration's field or prop `TypeRef` again reports again — and the thirteen remaining spanless
  callers of `type_from_type_ref` / `type_from_type_ref_in` in `crates/nx-types/src/infer.rs` do so
  at whatever `type_ref_span` happens to hold, which at the top of a use-site check is `0..0`.

  Reproduced in a single file (spans from `check_str`):

  ```
  type Range = { T:type start:T end:T }
  type S = { r:<Range U=int/> }
  let s = <S r={1} />
  let x = {s.r}
  ```
  → 6 diagnostics: the correct pair at `49..65` (the field), plus two pairs at `0..0` — one from
  building the construction's binding spec (`infer.rs:3250`/`3260`) and one from the field read
  (`infer.rs:1826`/`1833`).

  ```
  type Range = { T:type start:T end:T }
  external component <W range:<Range U=int/> />
  let root() = <W range={1} />
  ```
  → 4 diagnostics: the correct pair at `60..80`, plus a pair at `0..0` from the use site.

  Two more shapes: a component extending a base declared in the same file re-reports the inherited
  prop, correctly placed but duplicated; and a component extending a base in *another module of the
  same library* re-reports it at `0..0` of the deriving file — the `else` branch at `infer.rs:879`,
  which deliberately drops the span for an inherited prop but still lets the conversion report.

  The well-placed diagnostic is always present, so no problem is hidden; the cost is a squiggle on
  the first character of an unrelated file and two to three copies of each message.
- **Recommendation:** Resolve a declaration's field or prop type *quietly* at a use site —
  `type_from_type_ref_in_quietly`, the helper RF3's fix already added — since the pass that owns
  the declaration reports it with a span. This is safe across a library boundary too: a library
  whose declaration has a malformed type reference fails its own build (verified — the registry
  refuses to load it), so a consumer never resolves a broken one. The inherited-prop `else` branch
  at `infer.rs:879` should use the same helper rather than falling through to a spanless reporting
  conversion. A test asserting one diagnostic per message for the two repros above would lock it
  down.
- **Fix:** Every use-site conversion of a *declaration's* type reference now goes through
  `type_from_type_ref_in_quietly`: the record and union field reads (`infer.rs:1826`/`1833`,
  `1923`, `1970`), the record-literal field expectation, the element binding specs for a record,
  a union case and its base shape, a function's return type at a call site, a foreign alias's
  target, and an imported value's annotation. The pass that owns the declaration still reports,
  with a span. Two things beyond the recommendation:
  - The inherited-prop branch at `infer.rs:879` was keyed on the wrong thing. It asked whether the
    *module* declared the prop, so a base in the same file was reported twice — correctly placed,
    but duplicated. It now asks whether *this component* declared it (`component.props`), which
    covers the same-file and cross-module shapes with one rule; a prop a derived component
    redeclares still gets its own span. Verified on a two-module library: the diagnostic appears
    once, in `base.nx` where it was written, and no longer at offset 0 of `derived.nx`.
  - `resolve_type_argument` (`infer.rs:2995`) was left reporting, but now with the span it already
    had. The argument there is a name the *use site* wrote, not a declaration's, so a generic
    record named without its own arguments is a real error at that element — it was simply landing
    at offset 0.

  `type_from_type_ref` had no callers left and was removed. Added
  `a_declarations_type_reference_is_reported_once_however_often_a_use_site_resolves_it`, which
  asserts an exact count of one per message for RF10's two repros plus the inherited-prop and
  union-case shapes.
- **Verification:** ✅ Confirmed. Re-ran all four repros: the record field + construct + read case
  is 2 diagnostics (was 6), the component prop + use case 2 (was 4), the same-file inheritance case
  2 (was 4, correctly placed but doubled), and a union case + construct 2 — one per message, each
  on the declaration that wrote it. The cross-module shape is fixed too: on a two-module library the
  diagnostic now appears once, in `base.nx` at the prop, and no longer at offset 0 of the deriving
  file.

  The `resolve_type_argument` carve-out is right and worth keeping: `<C TItem=Range .../>` at a use
  site still reports the missing argument, now at the binding rather than at offset 0, because that
  name is the use site's own.

  Checked that quieting loses nothing: a bad alias reached only through a field, only through a
  `let`, a bad return type reached only through a call, a field type never constructed, and a union
  case field all still report exactly once. The one silent case — an alias declared and never used
  — is pre-existing and not specific to applied types: an alias's target is resolved on use, and
  `type R = { x:Nope }` is equally silent today.

## New Findings Discovered During 2026-09-19 08:22 Verification

### ✅ Verified - RF11 Two documents this change wrote still describe the erased update companion

- **Severity:** Low
- **Evidence:** RF5's fix made a generic record's companion generic in both languages and updated
  the `cli-code-generation` requirement, D7, the proposal and the tasks. Two prose documents that
  this same change added were not updated and now state the opposite of what typegen emits:
  - `docs/src/content/docs/reference/syntax/types.md:436` — "The update companion is erased in both
    languages and declares no generic parameter." It is not: TypeScript emits
    `Range_update<T>` with `start?: T` and C# emits `Range_update<T> : NxUpdate<Range<T>>`.
  - `bindings/dotnet/README.md:670` — "The record's `<Name>_update` companion is erased, declares no
    generic parameter, and patches `Range<object>`." All three clauses are now wrong, and this is
    the host-facing document where a .NET developer would go to learn the companion's shape — the
    one place that would tell them `Range_update<long>` exists.

  The README's **AOT note** immediately below is also now incomplete rather than wrong. It says each
  *record* instantiation must be named to a source-generated resolver; with a generic companion the
  same is true of each *companion* instantiation, and the new
  `NxUpdateRecordJsonConverterFactory` reaches the closed converter through `MakeGenericType` plus
  `Activator.CreateInstance`, which is exactly the reflection an AOT or trimmed publish cannot see.
  Nothing breaks today — neither project sets `IsTrimmable`, `PublishTrimmed` or `PublishAot`, and
  all 148 .NET tests pass — so this is a documentation gap, not a build one.
- **Recommendation:** Update both sentences to the generic shape, and add one clause to the AOT
  note covering the companion instantiations and the factory's reflection. Task 8.1 and task 8.3
  are the tasks these two documents belong to; neither was revisited when 6.7 and 6.8 landed.
- **Fix:** Both sentences now describe the generic shape, and the AOT note gained the companion
  clause.
  - `types.md`: the update companion "follows the record: `Range_update<T>` in both languages, so a
    patch of a `Range<long>` is typed as one and applies to it", with the wire called out as
    unaffected and the component-state exception kept.
  - `bindings/dotnet/README.md`: replaced the one wrong sentence with the host-facing shape a .NET
    developer needs — a compiling `Diff`/`Apply` example at `Range<long>`, the derivation
    (`NxUpdate<Range<T>>`), the key table (`RangeProperties<T>`), the accessor types
    (`NxOptional<T>`), the serialized form, and the component-state exception. Added a paragraph on
    why a generic companion names `NxUpdateRecordJsonConverterFactory` and a generated
    `<Name>_updateFormatter<T>` rather than its own converter (CS0416), noting that nothing is
    needed at the call site.
  - The AOT note now says each *companion* instantiation needs its own
    `[JsonSerializable]`/generated formatter entry separately from the record's, and that the
    factory closes its converter through `MakeGenericType` + `Activator.CreateInstance`, which a
    trimmed or AOT publish cannot see — so those instantiations should stay rooted.

  The README example was compiled and run against the checked-in `UpdateRecords.g.cs` rather than
  written from the emitter: it prints `end=9 applied.End=9 applied.Start=1` and
  `{"$type":"Range.Update","end":9}`, which is the JSON the README quotes. Swept the rest of
  `docs/` and the README for other companion-and-erasure prose; the only two matches left are the
  component-state sentences, which are still correct. Tasks 8.1 and 8.3 updated to name the
  companion so a later reader sees the documents were revisited.
- **Verification:** ✅ Confirmed. Both sentences now describe the generic shape and the AOT note
  carries the companion clause.
  - I compiled the README's `Diff`/`Apply` snippet **verbatim** against the checked-in
    `UpdateRecords.g.cs` rather than trusting the note: it builds, `patch.End.Value` is a `long`
    (not `object`), `patch.Apply(before)` gives `Start = 1, End = 9`, and
    `JsonSerializer.Serialize(patch)` is exactly the `{"$type":"Range.Update","end":9}` the README
    quotes. The three surrounding claims check out too — `typeof(Range_update<long>).BaseType` is
    `NxUpdate<Range<long>>`, `Start` is `NxOptional<long>`, and
    `RangeProperties<long>.Of(Range_property.End).Name` is `"end"`.
  - `types.md`'s "Below the checker" paragraph is accurate on all three points: the companion is
    `Range_update<T>` in both languages, the wire is unchanged, and only a component's state
    companion erases.
  - The sweep claim holds. Grepping `docs/`, both READMEs and the rest of the repo's Markdown for
    companion-plus-erasure prose leaves exactly two matches, and both are the component-state
    sentences, which I confirmed are still true (`Picker_update` erases to `sel?: unknown | null`
    and `NxOptional<object?>`). The main `cli-code-generation` spec's surviving erasure scenario is
    also the component one.
  - Tasks 8.1 and 8.3 were updated to name the companion, so the revisit is on the record.

  One observation, not a finding: tasks 6.2 and 6.3 still read "keep the update companion erased".
  That is the accurate history — they were carried out, then superseded by 6.7, which says so — and
  `tasks.md` is a log rather than a statement of current behavior. Worth leaving as is unless the
  archived tasks are read as documentation.

  Re-ran the full suite after the documentation edits: `cargo test --workspace`,
  `cargo fmt --all --check`, `pnpm -r build` (which builds the docs site), `dotnet test`
  (148 passed) and `openspec validate --strict` all pass.

## Questions

- RF5: is the erased-to-`Range<object>` companion the intended C# surface, or did the
  `cli-code-generation` requirement inherit the component rule without anyone working through what
  it means when the patched type is itself generic? **Answered: it inherited the rule.** The
  analogy does not hold — a component erases to `object` on both sides of the patch, a generic
  record does not — and the erased companion fails at runtime, not just at the signature. The
  recommendation is a generic companion; see RF5's status note for the measured cost. **Decided and
  implemented:** the companion is generic in both languages and the requirement was rewritten.
- RF2: was cross-*library* codegen (as opposed to cross-module within one library) considered in
  scope? Design D7 speaks only of "another module of the library", while the proposal's last bullet
  promises the library boundary works. If it is out of scope, it should be a stated Non-Goal and the
  emitters should fail loudly rather than emit an open generic. **Answered by the fix:** it is in
  scope — the proposal's promise is the one that stands, D7 now says how the dependency boundary
  resolves, and task 6.6 records the work.

## Summary

- The core of the change — grammar, validation, `TypeRef::Applied`, `NamedType::args` with
  argument-aware equality, the shared use-site machinery, erasure below the checker, and the IR
  schema staying at 4 — is implemented as designed and well covered. All 44 tasks' verification
  commands pass, including the new `cargo clippy --workspace --all-targets -- -D warnings` gate.
- Nine findings were raised: two High (RF1, a soundness hole where `diff` ignores type arguments;
  RF2, cross-library applied types generate uncompilable C# and TypeScript), three Medium (RF3,
  diagnostics at offset 0 outside record fields; RF4, duplicate diagnostics via the synthesized
  `.Update` companion; RF5, an unusable C# update companion for generic records), and four Low.
- Eight were fixed and all eight are now verified, each by re-running the original reproduction:
  RF1, RF2, RF3, RF4, RF6, RF7, RF8, RF9. RF2 and RF3 were checked beyond the cases the fixes'
  own tests cover — wildcard and qualified imports, nested and multi-parameter applied types, and
  six type positions the RF3 evidence table did not list — and all hold.
- All ten original findings are fixed and verified. RF5 was the one open design question:
  prototyping showed the erased companion fails a JSON round-trip-then-apply at runtime rather than
  merely being awkward, so the companion now follows the record and is generic in both languages,
  and the `cli-code-generation` requirement was rewritten to match. Verification re-derived that
  independently — the wire is byte-identical, a two-parameter companion compiles and round-trips,
  and a component's state companion still erases.
- RF11, raised during the second verification pass, is fixed and verified: both documents now
  describe the generic companion, the AOT note covers the companion instantiations and the
  converter factory's reflection, and the README's C# example was compiled verbatim against the
  checked-in generated code rather than read from the emitter.
- **No findings remain open.** All eleven are verified.
- Verification after the RF5, RF10 and RF11 fixes (all green): `cargo test --workspace`,
  `cargo clippy --workspace --all-targets -- -D warnings` (exit 0), `cargo fmt --all --check`,
  `pnpm -r build`, `pnpm -r test`, `dotnet test bindings/dotnet/NxLang.sln` (148 passed),
  `openspec validate add-record-type-parameters --strict`, and the .NET fixture drift test.
- The common thread in the High and Medium findings is coverage shape: the new tests assert on
  diagnostic *messages* and on single-file typegen output, so gaps in diagnostic *placement*,
  diagnostic *count*, and *cross-library* emission all pass unnoticed. Adding assertions on those
  three dimensions would be worth more than any additional scenario.
