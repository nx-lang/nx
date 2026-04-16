## 1. C# Generator Output

- [x] 1.1 Update the C# generator header/import output so generated DTO files include the JSON serialization namespace needed for dual annotations.
- [x] 1.2 Emit `JsonPropertyName` metadata alongside existing MessagePack `Key` metadata for generated record, action, discriminator, and external-component-state members.
- [x] 1.3 Emit JSON polymorphism metadata for generated abstract C# record and action roots while preserving the current MessagePack union behavior and intermediate abstract inheritance rules.

## 2. Generator Coverage

- [x] 2.1 Extend concrete C# codegen tests to assert dual MessagePack/JSON member annotations for generated records and actions.
- [x] 2.2 Extend abstract hierarchy C# codegen tests to assert `$type`-based JSON polymorphism metadata on roots and no conflicting discriminator redeclaration on intermediate abstract types.
- [x] 2.3 Extend external component companion-state C# codegen tests to assert dual annotations without introducing a `$type` discriminator.

## 3. Verification

- [x] 3.1 Run the relevant Rust codegen test suite for `nx-cli` and fix any generated-output regressions introduced by the dual-annotation change.
