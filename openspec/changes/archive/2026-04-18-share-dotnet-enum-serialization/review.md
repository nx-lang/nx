# Review: share-dotnet-enum-serialization

## Scope
**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md`, `specs/dotnet-binding/spec.md`, `specs/cli-code-generation/spec.md`
**Reviewed code:**
- [bindings/dotnet/src/NxLang.Runtime/Serialization/INxEnumWireFormat.cs](bindings/dotnet/src/NxLang.Runtime/Serialization/INxEnumWireFormat.cs)
- [bindings/dotnet/src/NxLang.Runtime/Serialization/NxEnumJsonConverter.cs](bindings/dotnet/src/NxLang.Runtime/Serialization/NxEnumJsonConverter.cs)
- [bindings/dotnet/src/NxLang.Runtime/Serialization/NxEnumMessagePackFormatter.cs](bindings/dotnet/src/NxLang.Runtime/Serialization/NxEnumMessagePackFormatter.cs)
- [bindings/dotnet/src/NxLang.Runtime/Serialization/NxSeverityWireFormat.cs](bindings/dotnet/src/NxLang.Runtime/Serialization/NxSeverityWireFormat.cs)
- [bindings/dotnet/src/NxLang.Runtime/NxSeverity.cs](bindings/dotnet/src/NxLang.Runtime/NxSeverity.cs)
- [bindings/dotnet/tests/NxLang.Runtime.Tests/NxEnumSerializationTests.cs](bindings/dotnet/tests/NxLang.Runtime.Tests/NxEnumSerializationTests.cs)
- [bindings/dotnet/README.md](bindings/dotnet/README.md)
- [crates/nx-cli/src/codegen/languages/csharp.rs](crates/nx-cli/src/codegen/languages/csharp.rs)
- [crates/nx-cli/src/codegen.rs](crates/nx-cli/src/codegen.rs)
- Deleted: `NxSeverityJsonConverter.cs`, `NxSeverityMessagePackFormatter.cs`

Rust codegen tests for both new assertions pass locally.

## Findings

### ✅ Verified - RF1 README "Enum Encoding" section is stale and still attributes the encoding to a "generated JSON converter and generated MessagePack formatter"
- **Severity:** Low
- **Evidence:** [bindings/dotnet/README.md:199-200](bindings/dotnet/README.md#L199-L200) still says "both the generated JSON converter and the generated MessagePack formatter encode the value as the authored member string." After this change, the converter and formatter implementations come from `NxLang.Runtime` (`NxEnumJsonConverter<,>` / `NxEnumMessagePackFormatter<,>`); only the wire-format mapping type is now generated. The proposal's explicit goal was that generated C# enums "no longer be fully self-contained" and that behavior is documented later in the "Generated Types" section, but the "Enum Encoding" paragraph on the same page still implies the old model and never names `NxLang.Nx.Serialization`.
- **Recommendation:** Update the "Enum Encoding" section to describe the shared-helper model: the generated wire-format mapping plus the shared `NxEnumJsonConverter`/`NxEnumMessagePackFormatter` from `NxLang.Runtime`. This also keeps the two README sections internally consistent.
- **Fix:** Updated the "Enum Encoding" section to name the generated wire-format mapping type and the shared `NxEnumJsonConverter<,>` / `NxEnumMessagePackFormatter<,>` helpers from `NxLang.Runtime`.
- **Verification:** Confirmed [bindings/dotnet/README.md:198-201](bindings/dotnet/README.md#L198-L201) now says "the generated enum emits an explicit wire-format mapping type and relies on `NxEnumJsonConverter<TEnum, TWire>` and `NxEnumMessagePackFormatter<TEnum, TWire>` from `NxLang.Runtime` to encode the value as the authored member string." This cleanly describes the shared-helper model and is consistent with the later "Generated Types" section that names `NxLang.Nx.Serialization`. Recommendation satisfied.

### ✅ Verified - RF2 Error-path behavior of the shared helpers is not covered by tests
- **Severity:** Low
- **Evidence:** The new shared helpers have non-trivial error paths:
  - [NxEnumJsonConverter.cs:22-36](bindings/dotnet/src/NxLang.Runtime/Serialization/NxEnumJsonConverter.cs#L22-L36) throws `JsonException` on non-string tokens and translates `FormatException` into `JsonException`.
  - [NxEnumMessagePackFormatter.cs:27-44](bindings/dotnet/src/NxLang.Runtime/Serialization/NxEnumMessagePackFormatter.cs#L27-L44) throws `MessagePackSerializationException` on nil/non-string tokens and translates `FormatException`.
  - [NxEnumSerializationTests.cs](bindings/dotnet/tests/NxLang.Runtime.Tests/NxEnumSerializationTests.cs) exercises only happy-path round trips and the "types are public" check. Task 1.3 only required round-trip plus availability coverage, so this is not a task-level gap, but the newly public runtime surface now owns the wrapping/translation contract for both generated enums and `NxSeverity`. A regression that (for example) stopped wrapping `FormatException` would silently alter the exception type observed by every downstream consumer.
- **Recommendation:** Add small tests that feed a numeric JSON token, a MessagePack nil, and an unknown member string through the shared helpers for both `NxSeverity` and the generated-style test enum, asserting the specific exception types/messages. This pins the public error contract of the new `NxLang.Nx.Serialization` surface.
- **Fix:** Added shared-helper regression tests for numeric JSON tokens, MessagePack nil tokens, and unknown member strings across both `TestDealStage` and `NxSeverity`, including wrapper exception assertions.
- **Verification:** Confirmed four new error-path tests in [NxEnumSerializationTests.cs:110-164](bindings/dotnet/tests/NxLang.Runtime.Tests/NxEnumSerializationTests.cs#L110-L164): `SharedEnumJsonHelpers_RejectNonStringTokens`, `SharedEnumJsonHelpers_RejectUnknownMembers`, `SharedEnumMessagePackHelpers_RejectNilTokens`, and `SharedEnumMessagePackHelpers_RejectUnknownMembers`. They cover numeric JSON tokens, MessagePack nil, and unknown member strings for both `TestDealStage` and `NxSeverity`, and pin exception types (`JsonException`, `MessagePackSerializationException`) and inner `FormatException` wrapping via shared helpers. `dotnet test bindings/dotnet/NxLang.sln --filter "FullyQualifiedName~NxEnumSerializationTests"` reports 8/8 passing. Recommendation satisfied.

### ✅ Resolved - RF3 Minor style inconsistency between `Format` and `Parse` in `NxSeverityWireFormat`
- **Severity:** Low
- **Evidence:** [NxSeverityWireFormat.cs:10-20](bindings/dotnet/src/NxLang.Runtime/Serialization/NxSeverityWireFormat.cs#L10-L20) uses a block-bodied method for `Format` (`{ return value switch { ... }; }`) while [NxSeverityWireFormat.cs:22-32](bindings/dotnet/src/NxLang.Runtime/Serialization/NxSeverityWireFormat.cs#L22-L32) uses an expression-bodied method for `Parse` (`=> value switch { ... };`). Both forms are valid, but the two symmetric methods in a hand-written reference type read better when they match, especially since this file is the in-repo "proof point" for the shared-helper pattern. The generated equivalents emit switch-statement bodies, so this file's style is already divergent from generated output; making the two methods here consistent with each other is the cheapest cleanup.
- **Recommendation:** Pick one form and apply it to both methods (expression body is shorter given the pure switch-expression body).
- **Status:** Already resolved in the current implementation; `NxSeverityWireFormat` now uses expression-bodied switch expressions for both `Format` and `Parse`.
- **Verification:** Confirmed [NxSeverityWireFormat.cs:10-28](bindings/dotnet/src/NxLang.Runtime/Serialization/NxSeverityWireFormat.cs#L10-L28): both `Format` and `Parse` use expression-bodied switch expressions consistently. No action needed.

## Questions
- None.

## Summary
The change cleanly replaces per-enum generated `JsonConverter`/`IMessagePackFormatter` types with a small public generic helper surface in `NxLang.Nx.Serialization`, migrates `NxSeverity` onto that same path, preserves the authored NX member strings via an explicit per-enum wire-format mapping, and narrows the `using` emission in the C# generator to match. All 8 tasks are complete, the two new Rust codegen tests pass, and the design decisions in `design.md` line up with the code. The reviewed follow-up items are now addressed: the README names the shared-helper model, the runtime tests cover shared-helper error paths, and `NxSeverityWireFormat` is already consistent in the current implementation.

## 2026-04-18 Verification
All three findings verified against the current tree. README "Enum Encoding" section (RF1) correctly names the generated wire-format mapping type and the shared converter/formatter helpers from `NxLang.Runtime`. New error-path tests (RF2) cover numeric JSON tokens, MessagePack nil, and unknown member strings for both `TestDealStage` and `NxSeverity`; `dotnet test` reports 8/8 passing. `NxSeverityWireFormat` (RF3) is already consistent. No new findings discovered during verification.
