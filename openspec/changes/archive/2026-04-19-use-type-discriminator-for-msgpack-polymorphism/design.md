## Context

NX currently has two MessagePack polymorphic record representations:

- Canonical/raw `NxValue` values encode records as maps and use `$type` as a string discriminator.
- Generated C# polymorphic DTOs use MessagePack `[Union(...)]` metadata for abstract roots.

This divergence creates interoperability friction when hosts mix raw runtime payloads and typed DTO
workflows. The proposal for this change is to standardize on one MessagePack record shape for
polymorphism: map + `$type` discriminator + authored fields.

Constraints:

- Preserve existing canonical `NxValue` behavior as the source-of-truth shape.
- Keep JSON behavior based on `$type` stable.
- Update generated C# output and managed serialization pathways without introducing parallel
  polymorphic contracts.

## Goals / Non-Goals

**Goals:**

- Ensure MessagePack polymorphic records use the same `$type` discriminator map shape across raw and
  typed workflows.
- Remove generated C# dependence on MessagePack `Union` polymorphism for NX record/action families.
- Add clear conformance coverage proving raw/typed cross-round-trip consistency.
- Keep the implementation simple by treating this as a direct wire-format replacement without
  backward-compatibility shims.

**Non-Goals:**

- Changing enum serialization shape (already standardized as bare authored strings).
- Changing JSON discriminator behavior for polymorphic records.
- Supporting legacy `Union` polymorphic payloads via dual-decoding paths.

## Decisions

1. **Canonical contract remains `NxValue` map + `$type`**
   - Rationale: This is already the raw runtime contract and is language-agnostic.
   - Alternative considered: Keep `Union` for C# typed workflows and document divergence.
   - Rejected because divergence preserves the current mismatch and complicates host integrations.

2. **C# generated polymorphism stops emitting MessagePack `[Union(...)]` metadata for NX records**
   - Rationale: `Union` implies a different wire contract than map + `$type`.
   - Alternative considered: Keep `[Union]` and add adapter-only runtime paths.
   - Rejected because generated contracts still suggest a non-canonical wire shape.

3. **Managed typed MessagePack polymorphic handling resolves concrete type from `$type` key**
   - Rationale: Aligns typed deserialization with raw runtime payloads and JSON discriminator model.
   - Alternative considered: emit synthetic discriminator members on generated DTOs.
   - Rejected due to prior direction to avoid generated `$type` data members and avoid field-name
     collisions.

4. **Conformance tests target both directions (raw->typed and typed->raw)**
   - Rationale: Prevents regressions where one direction silently drifts.
   - Alternative considered: testing only typed output.
   - Rejected because the problem is specifically contract parity across representations.

## Risks / Trade-offs

- **[Breaking change for Union-based consumers]** Existing consumers of generated C# `Union`
  MessagePack payloads may fail after switching to `$type` map contract.
  → **Mitigation:** Accept the break for this phase, document the new contract clearly, and keep
  implementation focused on the canonical shape only.

- **[Deserializer complexity]** Managed polymorphic deserialization from `$type` can require custom
  resolver logic or generated converters.
  → **Mitigation:** Centralize logic in shared runtime support and keep generated code declarative.

- **[Incomplete parity across edge cases]** Nested polymorphic records or arrays may still diverge.
  → **Mitigation:** Add targeted tests for nested records and mixed collection payloads.

## Migration Plan

1. Update CLI C# codegen spec and implementation to stop emitting MessagePack `Union` metadata for
   NX polymorphic record families and adopt `$type`-driven behavior.
2. Add/adjust managed runtime serialization support for typed polymorphic records keyed by `$type`.
3. Expand tests for raw/typed MessagePack parity in `bindings/dotnet` and any relevant Rust FFI/API
   integration suites.
4. Update docs/changelog guidance to call out the wire-format change and direct cutover to the new
   contract.

## Open Questions

- Which generated/runtime layer should own polymorphic type registry lookup for `$type` values:
  generated static maps, runtime resolver APIs, or both?
