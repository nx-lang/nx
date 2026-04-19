## 1. Update specs and generation contract

- [x] 1.1 Update `openspec/specs/cli-code-generation/spec.md` to replace MessagePack `[Union(...)]` polymorphism requirements with canonical `$type`-map requirements.
- [x] 1.2 Update any related docs/comments in codegen guidance that still describe `Union` as the polymorphic wire contract.
- [x] 1.3 Add or update codegen-level tests asserting generated C# polymorphic DTOs align with `$type` map semantics rather than `Union` envelopes.

## 2. Implement C# generator and runtime polymorphism changes

- [x] 2.1 Modify `crates/nx-cli` C# generation so abstract polymorphic NX record/action families no longer rely on MessagePack `Union` metadata.
- [x] 2.2 Implement/adjust managed MessagePack polymorphic serialization support in `bindings/dotnet` to resolve concrete types from `$type` map payloads.
- [x] 2.3 Ensure typed serialization outputs map keys and discriminator values matching canonical raw `NxValue` MessagePack payloads.

## 3. Add conformance and migration coverage

- [x] 3.1 Add round-trip tests for raw->typed and typed->raw MessagePack polymorphic records, including nested/collection cases.
- [x] 3.2 Add regression tests proving concrete record/action payloads include `$type` and do not use MessagePack `Union` envelope encoding.
- [x] 3.3 Document migration impact for consumers currently depending on `Union` payloads and include upgrade guidance in relevant docs/changelog notes.
