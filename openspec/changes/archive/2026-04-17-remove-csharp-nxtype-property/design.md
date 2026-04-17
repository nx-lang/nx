## Context

The C# generator currently emits a synthetic `__NxType` property mapped to wire key `$type` on
generated records and actions. That behavior came from earlier work that wanted the DTO surface to
mirror NX-authored JSON more directly, but the current serializer setup no longer needs it:

- `System.Text.Json` polymorphism is already declared at abstract roots through
  `[JsonPolymorphic(TypeDiscriminatorPropertyName = "$type")]` and `[JsonDerivedType(...)]`.
- MessagePack polymorphism already routes through `[Union(...)]` tags rather than an inner `$type`
  data member.
- External component state contracts already prove the generator can emit dual-annotated DTOs
  without a discriminator member.

The remaining generated property now behaves as synthetic payload data rather than serializer
metadata. It can also collide conceptually with real user fields such as `nx_type`, even though the
generator currently avoids that with the `__NxType` identifier.

## Goals / Non-Goals

**Goals:**
- Remove the synthetic discriminator property from generated C# records and actions.
- Keep generated C# hierarchies usable with both `System.Text.Json` and MessagePack.
- Preserve abstract-root polymorphism metadata so abstract-family serialization and deserialization
  still work.
- Update generator tests to lock in the discriminator-free output.

**Non-Goals:**
- Changing NX runtime payload semantics outside generated C# DTOs.
- Reworking TypeScript generation or runtime JSON contracts.
- Introducing custom JSON converters or custom MessagePack formatters for records/actions.

## Decisions

### 1. Stop generating any C# data member mapped to `$type`

The C# emitter will stop producing `__NxType` on concrete DTOs and stop producing abstract
discriminator members that concrete types override. Generated records and actions will expose only
their declared NX fields.

Rationale:
- Concrete DTOs do not need a `$type` property for serializer correctness.
- Removing the property simplifies the generated API and avoids synthetic data members in host code.
- It also removes the last remaining reason to reserve a language-specific property name that can
  shadow user intent.

Alternative considered:
- Keep `__NxType` on concrete root records/actions only.

Why rejected:
- That still forces a synthetic wire field into otherwise plain DTOs and preserves the API churn
  the user wants to eliminate.

### 2. Keep polymorphism metadata only where the serializers actually use it

Abstract record and action roots will continue to emit:

- `[JsonPolymorphic(TypeDiscriminatorPropertyName = "$type")]`
- `[JsonDerivedType(...)]` registrations for concrete descendants
- `[Union(...)]` registrations for MessagePack

No `$type`-mapped property will be generated alongside that metadata.
When an abstract C# root has no concrete exported descendants in the generated graph, the emitter
will omit `[JsonPolymorphic]`/`[JsonDerivedType(...)]` entirely, emit a short generated-source
comment explaining why, and surface a warning instead.

Rationale:
- This keeps abstract-family dispatch working for both supported serializers.
- It aligns the generated C# surface with serializer-native behavior instead of trying to model the
  NX wire shape as ordinary DTO fields.
- It avoids generating an invalid STJ polymorphism contract, because `[JsonPolymorphic]` without
  any registered derived types throws at runtime.

Alternative considered:
- Remove the JSON polymorphism attributes together with `__NxType`.

Why rejected:
- That would break abstract-family `System.Text.Json` polymorphism, which still depends on those
  attributes.

### 3. Treat direct concrete serialization without `$type` as the intended contract

After this change, serializing a concrete generated C# type directly will no longer emit a `$type`
field unless the serializer is operating through an abstract polymorphic contract that adds one.

Rationale:
- That is the natural behavior of the supported serializer metadata.
- It matches the requested simplification and avoids pretending that a concrete DTO property is
  required for serializer support.

Alternative considered:
- Preserve NX-authored JSON shape exactly for all concrete DTO serializations.

Why rejected:
- Doing that would require keeping the synthetic property or introducing a separate custom
  converter/codec layer, which is outside the scope of generated DTOs.

## Risks / Trade-offs

- [Generated C# DTO API becomes breaking for consumers that referenced `__NxType`] → Mitigation:
  document the break in the proposal/spec delta and update tests to assert the new contract.
- [Direct concrete JSON serialization no longer includes `$type`] → Mitigation: make the new spec
  explicit that discriminator emission is metadata-driven for abstract hierarchies only.
- [A hidden serializer dependency on the old property exists outside current tests] → Mitigation:
  run focused generator tests and keep the change localized to C# record/action emission.

## Migration Plan

1. Remove discriminator-member emission from the C# record/action generator path.
2. Update generator tests and CLI integration assertions to expect no `__NxType`/`$type` member on
   generated record/action DTOs.
3. Regenerate any downstream C# DTOs and remove consumer references to the old synthetic property.

Rollback is straightforward: restore the old emitter behavior and regenerate the affected outputs.

## Open Questions

No blocking questions. If consumers later need exact NX JSON map shapes for direct concrete DTO
serialization, that should be handled as a separate converter/codec feature rather than by
reintroducing a synthetic generated property.
