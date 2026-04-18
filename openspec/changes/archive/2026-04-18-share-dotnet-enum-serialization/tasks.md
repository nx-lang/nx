## 1. Runtime Enum Helper Infrastructure

- [x] 1.1 Add public `INxEnumWireFormat<TEnum>`, `NxEnumJsonConverter<TEnum, TWire>`, and `NxEnumMessagePackFormatter<TEnum, TWire>` under `bindings/dotnet/src/NxLang.Runtime/Serialization`.
- [x] 1.2 Migrate `NxSeverity` to the shared enum helper infrastructure and remove its dedicated converter and formatter types.
- [x] 1.3 Add or update managed runtime tests to cover shared-helper-based JSON and MessagePack enum round trips plus helper type availability.

## 2. C# Code Generation

- [x] 2.1 Update the C# emitter in `crates/nx-cli` to import `NxLang.Nx.Serialization` when generated output contains enums and to annotate enums with the shared helper types.
- [x] 2.2 Change generated enum support so each enum emits only an explicit wire-format mapping type that implements `INxEnumWireFormat<TEnum>` and preserves authored NX member strings.
- [x] 2.3 Refresh C# code generation tests to assert the shared helper attributes, retained authored string mappings, and absence of per-enum converter and formatter types.

## 3. Documentation And Verification

- [x] 3.1 Update `bindings/dotnet/README.md` to document the `NxLang.Runtime` reference requirement for generated C# enums and describe the shared helper model.
- [x] 3.2 Run the relevant Rust and .NET test suites for C# generation and enum serialization, and address any resulting failures.
