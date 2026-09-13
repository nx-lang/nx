# Review: map-backed-update-records

## Scope

**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md`, `specs/dotnet-binding/spec.md`,
`specs/cli-code-generation/spec.md`

**Reviewed code:**

- SDK (new): `NxUpdateRecord.cs`, `NxUpdate.cs`, `NxUpdateSchema.cs`, `NxField.cs`, `NxProperty.cs`,
  `NxValueEquality.cs`, `Serialization/NxUpdateRecordJsonConverter.cs`,
  `Serialization/NxUpdateRecordMessagePackFormatter.cs`
- SDK (changed/removed): `NxOptional.cs`, deleted `Serialization/NxOptionalSerialization.cs`
- Typegen: `crates/nx-cli/src/typegen/languages/csharp.rs`, `typegen/model.rs`, `typegen.rs` (tests)
- Fixture and tests: `Generated/UpdateRecords.g.cs`, `Generated/update-records.nx`,
  `NxUpdateRecordTests.cs`, `NxPropertyReferenceTests.cs`
- Docs: `docs/add-update-actions-followsup.md`

**Verification run during review:** `cargo fmt --all --check` clean; `cargo test --workspace` green
(40 test binaries, no failures); `dotnet test bindings/dotnet/NxLang.sln` green (127 passed). The
findings below were reproduced against the emitter and the SDK as they stand, not inferred.

## Findings

### ✅ Verified - RF1 A record field named `changed`, `isSet`, `unset`, or `diff` makes the generated update companion fail to compile

- **Severity:** High
- **Evidence:** Before this change the companion introduced exactly one member of its own
  (`NxType`), and `emit_update` guarded it with a rename loop. The new shape adds four more
  members — `IsSet(<T>_property)`, `Unset(<T>_property)`, `Changed()`, and `Diff(...)`
  ([csharp.rs:354](crates/nx-cli/src/typegen/languages/csharp.rs#L354),
  [csharp.rs:358](crates/nx-cli/src/typegen/languages/csharp.rs#L358),
  [csharp.rs:362](crates/nx-cli/src/typegen/languages/csharp.rs#L362),
  [csharp.rs:370](crates/nx-cli/src/typegen/languages/csharp.rs#L370)) — but `reserve_member`
  ([csharp.rs:266](crates/nx-cli/src/typegen/languages/csharp.rs#L266)) still protects only
  `NxType` and `FieldSchema`. Generating `export type Form = { changed:bool isSet:bool diff:string
  unset:bool }` and compiling the output against the SDK produces
  `error CS0102: The type 'Form_update' already contains a definition for 'Changed'` (and the same
  for `IsSet`, `Diff`, `Unset`). Field names like `changed` are ordinary in the domain this change
  is aimed at (forms, dirty state), so this is reachable from plain NX source.
  Two more names degrade rather than break: fields named `fields` or `schema` emit properties that
  hide `NxUpdateRecord.Fields` / `NxUpdateRecord.Schema`, producing `warning CS0108` and shadowing
  the base members a host needs for generic access.
- **Recommendation:** Feed every member the companion introduces through `reserve_member`, not just
  the two: derive `IsSet`/`Unset`/`Changed`/`Diff` names the same way, and include the base class's
  public surface (`Fields`, `Schema`, `IsSet`, `Unset`, `ChangedNames`, `Merge`) in the reserved
  set so hiding cannot happen either. Add a typegen test over a record whose fields are named after
  those members.
- **Fix:** `emit_update` now reserves every member it introduces (`NxType`, `FieldSchema`, `IsSet`,
  `Unset`, `Changed`, `Diff`) through `reserve_member`, so a colliding field keeps its accessor
  name and the helper steps aside (`Changed_()`, `IsSet_(...)`, `Diff_(...)`). An accessor that
  lands on a non-generic base member (`Fields`, `Schema`, `IsSet`, `Unset`, `ChangedNames`, and
  `Apply` on `NxUpdate<T>`) is emitted `public new`; the generic base members (`Get`, `Set`,
  `Merge`, `Diff`) are not hidden by a property, which the compiler confirms by rejecting `new`
  there, so they stay callable. Generated bodies reach the base through `base.` and the base type
  name (`NxUpdate<Clash>.Diff<Clash_update>(...)`). Covered by the typegen test
  `csharp_update_companion_steps_around_field_names_it_would_collide_with`, and the fixture now
  declares `Clash` with all seven names so the .NET test project compiles the output and
  `UpdateRecord_WithFieldsNamedAfterItsMembers_KeepsBothReachable` exercises it.
- **Verification:** Confirmed. The original repro (`changed isSet unset diff`) now compiles clean,
  and the fixture's `Clash` carries all seven names with `dotnet test` reporting 0 warnings and 0
  errors. I also generated and compiled the names the fixture does *not* cover — fields named
  `merge`, `apply`, `get`, `set`, `of`, `properties`, `nxType_` — and they build with no warning:
  `Apply` correctly takes `public new`, the table's mapper steps aside to `Of_`, and the claim that
  the generic base members (`Get`, `Set`, `Merge`, `Diff`) are not hidden by a same-named property
  holds in the compiler, so no `new` is needed there. `Clash_update.Diff` (the accessor) coexisting
  with `Diff_` (the wrapper) also produces no CS0108, confirming the arity reasoning. The hidden
  base members stay correct on the serialization paths: the converter, the formatter, `Merge`, and
  `Apply` all bind through `NxUpdateRecord`/`NxUpdate<T>`, not the derived type, so a `Fields` or
  `Schema` accessor cannot shadow them.

### ✅ Verified - RF2 The `<T>Properties` key table has no name-collision check, so a declaration of that name silently emits a duplicate type

- **Severity:** Medium
- **Evidence:** `emit_property_table` names the table
  `sanitize_csharp_identifier(format!("{}Properties", target_name))`
  ([csharp.rs:458](crates/nx-cli/src/typegen/languages/csharp.rs#L458)) with no check against the
  module's declarations. Generating `export type User = { name:string }` beside
  `export type UserProperties = { x:string }` emits both `public sealed class UserProperties` and
  `public static class UserProperties` into one namespace — `error CS0101`, with no warning. The
  `_update` and `_property` companions already have a documented warn-and-skip rule for exactly this
  ("Update companion name collision warns and skips" in the `cli-code-generation` spec); the new
  `Properties` suffix introduces a third generated name that skips it.
- **Recommendation:** Apply the existing companion collision rule to `<T>Properties`: warn naming
  the table and the conflicting declaration, and omit the table (falling back to the plain
  name→type schema and the `NxUpdateRecord` base for that companion). Cover it with a typegen test
  alongside the existing collision test.
- **Fix:** `update_plain_target` returns `None` when an exported declaration owns the table's name,
  so the companion falls back to `NxUpdateRecord` with the name→type schema, and
  `collect_warnings` reports the omission naming the table, the target, and the companion. Covered
  by `csharp_property_key_table_yields_to_an_explicit_declaration`; the rule is added to the
  `cli-code-generation` delta.
- **Verification:** Confirmed. Re-running the original repro (`User` beside `UserProperties`) now
  prints `Warning: Generated C# property-key table 'UserProperties' for 'User' was omitted because
  it conflicts with exported declaration 'UserProperties'; 'User_update' derives from
  NxUpdateRecord without keys, Apply, or Diff.`, emits `User_update : NxUpdateRecord`, and compiles
  clean where it previously produced CS0101. The delta carries a matching
  "Key table name collision warns and skips" scenario.

### ✅ Verified - RF3 `Diff` misses a change when a polymorphic field switches to a sibling type with equal fields

- **Severity:** Medium
- **Evidence:** `NxValueEquality.ValuesEqual` falls through to comparing two records by
  `JsonSerializer.SerializeToUtf8Bytes(value, value.GetType())`
  ([NxValueEquality.cs:77-79](bindings/dotnet/src/NxLang.Sdk/NxValueEquality.cs#L77-L79)).
  Serializing at the *runtime* type bypasses the `[JsonPolymorphic]` metadata that lives on the
  abstract base, so no `$type` discriminator is written and the runtime type is erased from the
  comparison. The TypeScript `nxValuesEqual`
  ([runtime.rs:457](crates/nx-codegen/src/runtime.rs#L457)) compares plain objects key by key, and
  `$type` is one of those keys, so it reports the change. Reproduced end to end: for
  `export abstract type Shape = { id:string }`, `export type Circle extends Shape = { }`,
  `export type Square extends Shape = { }`, `export type Doc = { shape:Shape }`, the call
  `Doc_update.Diff(new Doc { Shape = new Circle { Id = "1" } }, new Doc { Shape = new Square { Id = "1" } })`
  returns `diff fields: 0`. Design and the `dotnet-binding` delta both commit to diff matching
  `nxDiffRecords` "with the same results for the same inputs", and a diff that silently drops a
  field is the data-loss failure mode D7 was written to avoid.
- **Recommendation:** Compare records' runtime types before their content (`left.GetType() !=
  right.GetType() => false`), or serialize through the declared base so the discriminator is
  written. Add a diff test over a record with a polymorphic field.
- **Fix:** `NxValueEquality.ValuesEqual` compares runtime types before falling through to the
  structural comparison. The fixture gained `Shape`/`Circle`/`Square`/`Doc`, and
  `Diff_ReportsAPolymorphicFieldThatChangesType` covers the Circle→Square case and the equal case.
- **Verification:** Confirmed independently. My standalone repro program, which reported
  `diff fields: 0` before, now reports `diff fields: 1 (shape)`. The type check sits after the
  `IConvertible` branch, so it cannot regress a boxed-primitive comparison that `Equals` already
  decided.

### ✅ Verified - RF4 An external component's state does have an instantiable plain type, so its companion is denied `Apply`/`Diff` on a rationale that does not hold

- **Severity:** Low
- **Evidence:** D3 and the `cli-code-generation` delta both justify the untyped base for a
  component with "no generated plain type carries the component's state". For an *external*
  component that is not true: `export external component <Ticker step:int /> = { state { count:int
  = 0 } }` emits `public sealed class Ticker_state { public long Count { get; set; } }`, whose
  fields are exactly `Ticker_update`'s. `update_plain_target`
  ([csharp.rs:397](crates/nx-cli/src/typegen/languages/csharp.rs#L397)) returns `None` for any
  component, so `Ticker_update : NxUpdateRecord` gets no key table and no `Apply`/`Diff` even
  though `NxUpdate<Ticker_state>` would work. The check itself is right to reject the props
  contract under the component's own name — it is the stated reason that is inaccurate, and the
  capability left on the table is real given two-way binding is the next change.
- **Recommendation:** Either target `<Name>_state` for external components with declared state and
  emit `<Name>_stateProperties`, or keep the current behavior and correct the wording in `design.md`
  D3 and the spec scenario to say the *state* companion is deliberately not bound to `<Name>_state`,
  with the reason.
- **Fix:** Took the first option. `update_plain_target_candidate` binds an external component's
  companion to `<Name>_state` when that state record is emitted, so `Ticker_update : NxUpdate<Ticker_state>`
  gets `Ticker_stateProperties`, `Apply`, and `Diff`; a non-external component still takes the
  untyped base. Covered by `csharp_external_component_update_companion_applies_to_the_state_record`
  and, with `Ticker` added to the fixture, `ExternalComponentUpdate_AppliesToTheStateRecord`.
  Design D3 and the `cli-code-generation` delta now say so.
- **Verification:** Confirmed. The fixture emits `Ticker_update : NxUpdate<Ticker_state>` with
  `Ticker_stateProperties`, `Apply`, and `Diff`, while the non-external `Counter_update` still
  derives from `NxUpdateRecord` — so the change is scoped to the case where a plain type actually
  exists. `Ticker_stateProperties` is emitted exactly once (the `_state` record does not also
  produce a table of its own), and the candidate requires an `ExternalState` declaration whose
  `component_name` matches, so a hand-written type named `<Name>_state` cannot be mistaken for it.

### ✅ Verified - RF5 The MessagePack key ordering that D6 makes a goal is never asserted across more than one field

- **Severity:** Low
- **Evidence:** D6 commits to byte-identical MessagePack output, and task 1.3 says to verify by
  "comparing against bytes captured from the current implementation before it is deleted" — no such
  comparison or captured fixture landed. The only MessagePack ordering assertion,
  `UpdateRecord_MessagePack_OmitsUnsetAndKeepsNull`
  ([NxUpdateRecordTests.cs:216](bindings/dotnet/tests/NxLang.Sdk.Tests/NxUpdateRecordTests.cs#L216)),
  sets a single field, so it cannot distinguish ordinal order from declared order; the new
  multi-field ordering test covers JSON only
  (`UpdateRecord_Json_WritesTypeThenFieldsInOrdinalKeyOrder`). A future edit to
  `NxUpdateSchema.FieldsInWireOrder` would silently change the wire bytes. Separately,
  `Assert.IsNotAssignableFrom<NxUpdate<object>>(merged)` in
  `ComponentUpdate_WithNoPlainType_MergesAndReportsChanges` is vacuous — `NxUpdate<object>` is
  unrelated to every generated companion, so the assertion passes for `User_update` too and does not
  test what its comment implies.
- **Recommendation:** Assert the MessagePack key sequence for a multi-field patch (e.g.
  `Form_update` with all three fields set, expecting `$type, drafts, pending, sortBy`), and replace
  the vacuous assertion with one that checks the base actually chosen, such as
  `Assert.IsAssignableFrom<NxUpdateRecord>` plus a check that `Counter_update` is not an
  `NxUpdate<>` of anything.
- **Fix:** `UpdateRecord_MessagePack_WritesTypeThenFieldsInOrdinalKeyOrder` pins the exact bytes
  the pre-change reflection formatter wrote for a two-field `User_update` and a three-field
  `Form_update` (captured from it before deletion, during task 1.3) and asserts the
  `$type, drafts, pending, sortBy` key sequence. The vacuous assertion is replaced by
  `Assert.Equal(typeof(NxUpdateRecord), typeof(Counter_update).BaseType)`.
- **Verification:** Confirmed. I decoded the pinned hex by hand: the `User_update` bytes are a
  3-entry map `$type`/`User.Update`, `email`/`x@y`, `name`/`Ada`, and the `Form_update` bytes a
  4-entry map `$type`, `drafts` (a 1-element array holding a nested `User.Update` map), `pending`
  (nil), `sortBy` (`email`) — ordinal order in both, matching the rule the deleted reflection
  formatter documented. The replacement base-class assertion is now meaningful, and its twin covers
  `Ticker_update`. One cosmetic slip: the sentence added to D6 in `design.md` runs past the file's
  line-wrap width.

### ✅ Verified - RF6 An unknown field key throws but a wrong `$type` is accepted silently

- **Severity:** Low
- **Evidence:** Both readers skip `$type` without comparing it to `Schema.NxType`
  ([NxUpdateRecordJsonConverter.cs:54-57](bindings/dotnet/src/NxLang.Sdk/Serialization/NxUpdateRecordJsonConverter.cs#L54-L57),
  [NxUpdateRecordMessagePackFormatter.cs:173-176](bindings/dotnet/src/NxLang.Sdk/Serialization/NxUpdateRecordMessagePackFormatter.cs#L173-L176)).
  D7's argument for rejecting an unknown key — "both sides generate from the same NX source, and
  silently dropping a field of a patch is a data-loss bug that surfaces far from its cause" —
  applies at least as strongly to a patch whose discriminator names a different record: a
  `Form.Update` payload whose keys happen to be a subset of `User`'s would deserialize as a
  `User_update` without complaint. The schema now makes this checkable for the first time, at the
  same place the key check already happens.
- **Recommendation:** Compare the read `$type` against `Schema.NxType` and throw naming both, or
  record in design.md why the discriminator is deliberately not validated.
- **Fix:** Both readers compare the `$type` they read with `Schema.NxType` and throw naming both
  (a non-string discriminator reads as `null` and fails the same check). Covered by
  `UpdateRecord_Json_WithAnotherRecordsDiscriminator_Throws` and its MessagePack twin; D7 now
  states the rule.
- **Verification:** Confirmed. Both readers compare against `Schema.NxType` and both messages name
  the read discriminator and the expected one. The JSON path is safe on a non-string `$type`: it
  throws before the `continue`, so no unskipped composite value can desynchronize the reader. The
  `dotnet-binding` delta carries the matching scenario, and the native-runtime interop tests still
  pass, so nothing in the repository writes a discriminator the SDK now rejects.

## Questions

- Is the ordinal key order (`$type` first because `$` sorts below every letter) a contract the
  native decoder relies on, or purely an artifact of the old reflection formatter that the new one
  reproduces? If it is only an artifact, declared order would be cheaper and more legible — but
  changing it later breaks the byte-identity D6 promises, so it is worth stating either way.
  - **Answer:** An artifact. Nothing in the repository reads a patch by key position, and the
    interpreter's raw-value tests only assert key sets. D6 keeps it so the bytes a host recorded
    against the old SDK stay identical, and the new byte-pinning test (RF5) makes any later change
    to the order a deliberate one. Recorded in D6.
- Should `Merge` get a generated wrapper the way `Diff` does? `User_update.Merge(a, b)` resolves
  today through static-member inheritance (the tests use both spellings), so this is legibility
  only, not a gap.
  - **Answer:** No. `Diff` needs the wrapper because its result type cannot be inferred from its
    arguments; `Merge` infers it, and a wrapper would be one more member to keep clear of field
    names (RF1).

## Verification (2026-09-12 18:05)

All six findings were re-checked against the working tree and are verified fixed; none were
reopened and no new findings were opened. Suites re-run during verification: `cargo fmt --all
--check` clean, `cargo test --workspace` green (40 test binaries, no failures), `dotnet test
bindings/dotnet/NxLang.sln` green (133 passed, up from 127, with 0 warnings and 0 errors), and
`openspec validate map-backed-update-records` reports the change valid. Each fix was additionally
exercised outside its own tests, by re-running the repros that produced the original findings and by
probing the cases the new tests do not cover.

### Checked and left alone: two pre-existing typegen behaviors, not this change's

Both surfaced while probing RF1's collision guard, and both reproduce on the **plain record** class
that this change does not touch, so neither is attributable here and neither is opened as a finding:

- Two NX fields whose names differ only by a trailing underscore (`changed` and `changed_`) both
  sanitize to the same C# member, so `public sealed class Adv`, `Adv_property`, and the companion
  each emit duplicate members (CS0102). `sanitize_csharp_member_name` is the cause.
- A field named `toString`, `equals`, or `getHashCode` emits a property that hides the `object`
  member of that name (CS0108) — on the plain record as much as on the companion, and as it did
  before this change.

Worth a follow-up against typegen's member-naming generally, rather than against this change.

## Summary

The implementation matches the design closely, the wire format is genuinely preserved, and all 21
tasks have landed with real code behind them: reflection over generated members is gone from both
serialization paths, the schema/key-table split is clean, and `NxOptional<T>` is correctly demoted
to an in-memory value. Test coverage for the new surface is good, and every artifact scenario in
both deltas has a test behind it.

All six findings raised by the review have been fixed and verified. RF1 (the blocker) was resolved
the right way round: the field accessor keeps the record's name, since that is the front door a host
writes, and every member the companion introduces steps around it, with `new` where an accessor
genuinely hides a base member — verified past the cases the new tests cover. RF2 folds the key table
into the collision rule the other companions already follow, RF3 restores parity with
`nxDiffRecords` for polymorphic fields, and RF4 turned a wrong rationale into a real capability by
binding an external component's companion to its `<Name>_state` record. RF5 and RF6 closed the
coverage and symmetry gaps, with the MessagePack bytes now pinned exactly. Artifacts were kept in
step: design D3, D6, and D7 and both spec deltas record the new rules, and the change validates.
The change looks ready to archive.
