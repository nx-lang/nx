## Context

`crates/nx-cli` currently emits three enum-specific support types for every generated C# enum: a
wire-format mapper, a `JsonConverter<TEnum>`, and an `IMessagePackFormatter<TEnum>`. The managed
binding repeats the same pattern for `NxSeverity`. The duplicated serializer plumbing increases
generated output size, spreads the same error handling logic across codegen and runtime sources, and
creates two places to maintain identical behavior.

The change is cross-cutting because it touches the generated C# surface in `crates/nx-cli`, the
public managed runtime assembly in `bindings/dotnet`, runtime tests, and the .NET binding docs. It
also has a compatibility wrinkle: generated enum support will now depend on `NxLang.Runtime`
instead of being self-contained.

Generated CLR enum member names cannot be used as the source of truth for NX wire names. The
generator converts snake_case NX members to PascalCase CLR members, and the sanitization logic is
lossy for punctuation and separator-heavy names. The design therefore needs a shared serializer
implementation without losing the explicit authored NX member spelling.

## Goals / Non-Goals

**Goals:**
- Eliminate per-enum generated JSON converter and MessagePack formatter boilerplate.
- Reuse the same enum serializer implementation for generated C# enums and managed binding enums
  such as `NxSeverity`.
- Preserve explicit, lossless mapping between CLR enum members and authored NX member strings across
  both serializers.
- Keep raw `NxValue` enum workflows unchanged so typed DTO enum strings and canonical raw enum
  payloads remain distinct.

**Non-Goals:**
- Changing record, action, or external-component DTO serialization patterns.
- Replacing the canonical raw `"$enum"` plus `"$member"` payload shape used by schema-free runtime
  APIs.
- Inferring NX wire strings from CLR enum member names at runtime.
- Keeping generated C# enum support fully self-contained without a `NxLang.Runtime` reference.

## Decisions

### Expose public generic enum serializer helpers from `NxLang.Nx.Serialization`

`NxLang.Runtime` will add a small public helper surface:

- `INxEnumWireFormat<TEnum>`
- `NxEnumJsonConverter<TEnum, TWire>`
- `NxEnumMessagePackFormatter<TEnum, TWire>`

`TEnum` will be constrained to `struct, Enum`, and `TWire` will provide static `Format` and `Parse`
operations through the wire-format interface. The generic helpers will centralize JSON token
validation, MessagePack token validation, authored string writing, and exception translation.

Rationale:
- Generated enums can reference a closed generic converter/formatter type directly from attributes.
- The runtime can use the same helpers for `NxSeverity`, removing its bespoke converter/formatter
  classes.
- The implementation stays reflection-free and AOT-friendly because the mapping stays compile-time
  bound through generic type parameters.

Alternatives considered:
- Shared helper methods plus generated per-enum converter/formatter classes: reduces method bodies
  slightly but keeps two generated types per enum and does not materially simplify output.
- Reflection-based helpers that inspect enum member attributes: avoids a generated mapper type but
  adds runtime reflection, introduces trimming/AOT risk, and still requires per-member metadata.
- A generated shared support file instead of runtime helpers: preserves self-contained generated code
  but duplicates the same helper implementation across outputs and leaves `NxSeverity` on a separate
  path.

### Keep one generated per-enum wire-format mapping type

Each generated enum will keep a dedicated mapping type such as `PaymentProviderWireFormat`, but it
will become the only enum-specific support type emitted for serialization. The type will implement
`INxEnumWireFormat<TEnum>` and expose explicit static `Format` and `Parse` methods backed by switch
expressions or equivalent exhaustive mappings.

Rationale:
- The explicit mapping preserves authored NX spellings exactly.
- The generator already has the enum member list at compile time, so switches remain simple,
  predictable, and testable.
- The mapping type gives the shared helpers a stable contract without depending on naming
  conventions.

Alternatives considered:
- Converting CLR member names back to snake_case at runtime: rejected because sanitization is lossy
  and not every authored NX spelling is recoverable from the CLR member name.
- Emitting per-member string attributes and teaching the generic helpers to reflect over them:
  rejected because it replaces code duplication with reflection and still leaves explicit metadata on
  every enum member.

### Generated C# enum output will reference runtime helpers explicitly

For generated enums, the generator will emit:

- `using NxLang.Nx.Serialization;`
- `[JsonConverter(typeof(NxEnumJsonConverter<TEnum, TWire>))]`
- `[MessagePackFormatter(typeof(NxEnumMessagePackFormatter<TEnum, TWire>))]`
- the enum-specific wire-format mapping type

It will stop emitting `{EnumName}JsonConverter` and `{EnumName}MessagePackFormatter`.

Rationale:
- The generated code change is visible and testable.
- The dependency is narrow and isolated to enum serialization support.
- The wire format stays unchanged for both JSON and MessagePack typed DTO workflows.

Alternatives considered:
- Fully qualifying the helper type names in attributes instead of adding a `using`: viable, but
  noisier in generated output and offers no functional benefit.

### Migrate `NxSeverity` to the shared helper infrastructure

`NxSeverity` will adopt the same public generic converter/formatter types plus its own wire-format
mapping type. This keeps the managed binding on the same implementation path as generated enums and
provides an internal proof point for the runtime helper API.

Rationale:
- Reduces duplicate serializer code in the runtime itself.
- Validates that the helper surface works for both generated and hand-written enums.

## Risks / Trade-offs

- [Generated C# enums now depend on `NxLang.Runtime`] → Document the dependency in the proposal,
  binding README, and generated-code tests so the change is explicit and intentional.
- [New public runtime helper surface becomes a compatibility commitment] → Keep the surface minimal,
  focused on enum serialization only, and covered by tests that validate both generated usage and
  `NxSeverity`.
- [Generic helper attributes could fail if formatter/converter construction rules differ across
  serializers] → Use closed generic types with public parameterless construction and cover both
  `System.Text.Json` and MessagePack round trips in tests.
- [A future enum naming edge case could bypass convention-based recovery] → Preserve explicit
  generated mappings and avoid convention-based parsing entirely.

## Migration Plan

1. Add the public enum helper surface to `bindings/dotnet/src/NxLang.Runtime/Serialization`.
2. Migrate `NxSeverity` to use the shared helpers and remove its dedicated converter/formatter
   classes.
3. Update C# code generation to emit shared helper attributes plus a single per-enum mapping type.
4. Refresh generator tests, managed runtime tests, and README guidance to reflect the new dependency
   and emitted output shape.
5. If the change needs to be rolled back, restore per-enum generated converter/formatter emission
   while keeping the typed enum wire format unchanged.

## Open Questions

- None currently. The main compatibility trade-off is intentional and covered by the proposal.
