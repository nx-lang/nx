## Context

The C# generator under `crates/nx-cli/src/codegen/languages/csharp.rs` currently emits DTO classes
that are shaped for MessagePack only. Generated records, actions, and synthesized
external-component-state contracts use `[MessagePackObject]`, `[Key(...)]`, and `[Union(...)]`,
but do not emit `System.Text.Json` metadata. In practice, hosts that want to use the same
generated DTOs with JSON today need to maintain hand-written wrappers or patch generated files
after codegen.

This change follows the new runtime-output work that allows hosts to receive JSON directly from the
runtime. Once hosts can receive JSON directly, the generated C# DTO surface should be able to bind
those payloads with the same type definitions already used for MessagePack-based host calls.

## Goals / Non-Goals

**Goals:**
- Make generated C# DTO classes usable with both MessagePack and `System.Text.Json` from the same
  generated source.
- Preserve the existing MessagePack contract and generated type names.
- Keep JSON member names aligned with NX wire names, including `$type`.
- Support JSON polymorphic deserialization for generated abstract record and action families.
- Cover the new output contract with generator-focused tests.

**Non-Goals:**
- Changing runtime bindings, runtime payload formats, or the MessagePack input contract.
- Reworking non-C# generators.
- Defining a new repository-wide JSON policy for standalone generated enums.
- Emitting custom JSON converters when attribute-based metadata is sufficient for DTO classes.

## Decisions

### Emit dual serializer metadata on generated DTO members

Generated C# records, actions, and external-component-state companion contracts will keep their
existing MessagePack annotations and add `System.Text.Json.Serialization` metadata on the same
members. Each generated DTO field will continue to use `[Key("<nx-name>")]` and will also emit
`[JsonPropertyName("<nx-name>")]`.

Rationale:
- This keeps the MessagePack contract unchanged while making JSON property binding explicit.
- It matches the hand-written DTO style already used in the .NET runtime tests.
- It avoids forcing consumers to depend on global `JsonSerializerOptions.PropertyNamingPolicy` or
  custom naming converters.

Alternatives considered:
- Rely on default JSON property-name inference from generated C# property names. Rejected because
  sanitized C# identifiers do not always match the NX wire key, especially for `$type`.
- Generate separate MessagePack and JSON DTO surfaces. Rejected because it duplicates model types
  and defeats the purpose of a shared generated contract.

### Use `$type` as the shared JSON polymorphism discriminator

For abstract generated record and action roots, the generator will emit JSON polymorphism metadata
that uses `$type` as the discriminator property and registers each concrete descendant by its NX
declaration name. The generated discriminator property will also receive `JsonPropertyName("$type")`
so the same member definition remains valid for both serializers.

Rationale:
- NX payloads already use `$type` as the runtime discriminator for concrete records and actions.
- Reusing the same discriminator avoids inventing a JSON-only shape or requiring a translation
  layer between MessagePack and JSON DTO workflows.
- Registering concrete descendants at the abstract root mirrors the generator’s existing
  MessagePack `[Union(...)]` behavior.

Alternatives considered:
- Use a JSON-only discriminator such as `type`. Rejected because it would diverge from the existing
  NX payload contract.
- Depend on a custom `JsonConverter` per hierarchy. Rejected because attribute-based metadata is
  simpler to generate and easier for consumers to inspect.

### Keep companion state contracts plain and discriminator-free

Generated external-component-state companion contracts will remain plain DTO classes without a
`$type` discriminator. The generator will add JSON property metadata to their declared state fields
but will continue to omit any discriminator member.

Rationale:
- Companion state contracts model opaque host-owned state snapshots, not runtime-polymorphic
  record/action payloads.
- The current generated naming and field selection behavior is still correct; only the serializer
  metadata needs to expand.

Alternatives considered:
- Add a synthetic discriminator for all generated DTO classes. Rejected because state companions do
  not need polymorphic dispatch and adding `$type` would change their wire shape.

### Validate behavior through generated-output tests

The existing Rust codegen tests will be extended to assert the generated C# source contains the
expected JSON annotations and JSON polymorphism metadata alongside the existing MessagePack
annotations. Tests will focus on concrete records/actions, abstract hierarchies, and companion
state contracts.

Rationale:
- The generator’s public contract is the emitted source, so string-based output tests are the
  fastest way to lock down behavior.
- These tests already exist for the MessagePack/discriminator surface and can be extended without
  introducing a new test harness.

## Risks / Trade-offs

- [Serializer attribute coupling] Generated DTOs will become more explicitly tied to
  `System.Text.Json.Serialization` in addition to MessagePack. → Mitigation: keep the change scoped
  to generated C# output and avoid introducing runtime dependencies outside the generated source.
- [Framework metadata differences] JSON polymorphism attributes need to match the .NET target used
  by generated consumers. → Mitigation: generate metadata compatible with the repository’s supported
  .NET 10 workflow and cover it in generator tests.
- [Over-scoping enums] Expanding the change to standalone generated enums could force a broader JSON
  policy decision than the user requested. → Mitigation: keep this change focused on DTO classes and
  leave enum JSON policy unchanged.

## Migration Plan

1. Update the C# generator to emit the additional JSON serialization attributes and hierarchy
   metadata.
2. Extend generator tests to assert dual-annotated output for the supported DTO shapes.
3. Regenerate or re-run codegen in consuming workflows as needed; no runtime migration layer is
   required because the MessagePack contract remains intact.

Rollback is straightforward: revert the generator change and regenerate affected outputs.

## Open Questions

- None at proposal time. The main scope boundary is explicit: dual-annotated DTO classes are in
  scope; changing standalone generated enum JSON behavior is not.
