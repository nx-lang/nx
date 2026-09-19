## 1. Grammar and validation

- [x] 1.1 Add `applied_type` and `type_argument` to `crates/nx-syntax/grammar.js` as an alternative in the `type` rule (design D2), run `pnpm generate` in `crates/nx-syntax`, and add the new kinds to `syntax_kind.rs`; verify `tree-sitter generate` reports no new conflicts and the existing parser tests still pass
- [x] 1.2 Add parser tests beside the "Component type parameters" banner in `crates/nx-syntax/tests/parser_tests.rs` for `<Range T=int/>` as a field type, a `let` annotation, an alias target, under `?` and `[]`, nested, with a suffixed argument, with zero arguments, with a qualified tag (`<Range.Update T=int/>`), and next to a `<function …/>: T` type; verify each tree has an `applied_type` node with the expected `name` and `arguments` fields
- [x] 1.3 In `validation.rs`, admit `Name:type` in a `RECORD_DEFINITION` and run the placement, reserved-name, default and modifier checks for it with "field" wording; verify with parser tests that a record type parameter is accepted, that each misuse in the spec is rejected with `invalid-type-parameter`, and that an `action`, emitted action, state group, function and function type still reject it
- [x] 1.4 Highlight the applied type's tag as a type and its argument names as type parameters in `queries/highlights.scm`; verify with a case in `crates/nx-syntax/tests/query_tests.rs`

## 2. HIR

- [x] 2.1 Add `ast::TypeRef::Applied { name, args }`, lower `applied_type` to it, and extend `spell_type_ref` and the suffix-aware spellers; verify with unit tests in `ast/types.rs` that `<Range T=int/>`, `<Range T=int/>[]?` and a nested applied type round-trip through lowering and spelling
- [x] 2.2 Add `RecordDef.type_params`, fill it with `lower_type_parameters`, reject a field that shares a type parameter's name, and carry the parameters on `EffectiveRecordShape`; verify with lowering tests modelled on `test_lower_component_type_parameters_are_separate_from_props` that parameters are never in `properties`
- [x] 2.3 Reject a generic record that is `abstract` or has an `extends` clause in `validate_record_definition`; verify with tests in `records.rs` that each is a diagnostic naming the record
- [x] 2.4 Pass the target's type parameters through `predeclare_update_record` so `R.Update` declares them, and leave the `R.Property` union unchanged; verify with a lowering test that `Range.Update` has `type_params == [T]` and `Range.Property` has cases `start` and `end` only
- [x] 2.5 Add `type_params` to `InterfaceItemKind::Record` in `prepared.rs` and populate it; verify with a prepared-module test that an exported generic record's interface item carries its parameters
- [x] 2.6 Extend `erase_type_parameters` to recurse into an applied type's arguments; verify with a test beside `erase_type_parameters_reaches_inside_a_function_type`
- [x] 2.7 Make `remove_property_entries` also drop a `RecordLiteralProperty` whose value expression was consumed; verify with a test beside `remove_property_entries_drops_plain_entries_by_value_expression`

## 3. Type checker

- [x] 3.1 Add `args: Vec<Type>` to `NamedType`, include it in equality, hashing and `Display` (applied spelling), and make `substitute_parameters`/`find_parameter` recurse into it; verify with unit tests in `ty.rs` that two instantiations are unequal, that neither is compatible with the other, and that both are compatible with `object`
- [x] 3.2 Resolve `TypeRef::Applied` in `type_from_type_ref_in` (design D4) with diagnostics for an unknown argument, a duplicate argument, a missing parameter, a tag that is not a generic record, and an unresolved argument; report a bare generic record name as a missing argument that shows the applied form; verify with a new `crates/nx-types/tests/record_type_parameters.rs` covering every scenario under "An applied type names one instantiation" and "A malformed applied type is rejected by name"
- [x] 3.3 Check a generic record's own fields and defaults under a rigid type-parameter scope; verify the "parameter-typed field rejects a concrete default" scenario and that `T[]`, `T?` and a function type over `T` are accepted
- [x] 3.4 Factor the resolve-arguments / swap-scope / build-spec / restore sequence out of `check_element_bindings_against_component` and use it from both `infer_record_literal` and `check_element_bindings_against_record`, producing a `NamedType` with `args`, reporting an unbound parameter once per element and skipping the bindings it types; verify every scenario under "Constructing a generic record binds every type argument", each written once in a single file and once across a library boundary, and verify `component_type_parameters.rs` still passes unchanged
- [x] 3.5 Substitute arguments on field access and require equal `args` in `record_type_satisfies_expected`; verify the scenarios under "Applied types are distinct per argument and invariant"
- [x] 3.6 Copy `args` from `R` to `R.Update` in `infer_intrinsic_call` and require equal applied types in `merge`; verify the scenarios under "A generic record's derived companions follow its type parameters" in `record_type_parameters.rs`
- [x] 3.7 Make `nominal_named_type` and `resolve_named_type` unable to return an argument-less type for a generic record, and add a test asserting that every spelling of bare `Range` in a type position is a diagnostic

## 4. Library interface

- [x] 4.1 Carry record `type_params` in the `nx-api` record interface entry, and make `type_to_type_ref` produce `TypeRef::Applied` for a `NamedType` with arguments and `TypeRef::Name` for `Type::Parameter`; verify with an `artifacts.rs` test that a library exporting `Range` and a function returning `<Range T=int/>` is consumed by an importing module that applies and constructs it

## 5. Below the checker

- [x] 5.1 Treat `TypeRef::Applied` as its record name in the interpreter's `runtime_type_from_type_ref`, and erase a generic record's own parameters before coercing its fields; verify with a new `crates/nx-interpreter/tests/record_type_parameters.rs` that a constructed `Range` has fields `start` and `end` and no `T`, that field access and `apply` work, and that two instantiations with equal fields are equal values
- [x] 5.2 Resolve an applied type to `CodegenTypeRef::Nominal` and a record's parameter-typed field to `object` in `crates/nx-codegen/src/builder.rs`, for the record and its update record; verify with `ir_tests.rs` cases for each scenario in the `nx-ir-format` delta, including that `NX_IR_SCHEMA_VERSION` is still 4 and no `T` appears in the explained IR text
- [x] 5.3 Add a `specs/ir-conformance/` program that constructs a generic record, reads a field and applies an update; verify the interpreter, the generated executable code and the TypeScript IR runtime agree on its output, with no change to `runtime/typescript/src`
- [x] 5.4 Fix the top-level-value binding cycle the field-default path exposes: a record construction that omits a defaulted field binds the declaring module's top-level values so the default can name one, and the value being constructed is one of them; track the values in flight in `ExecutionContext` and leave them out of the nested pass, and verify with `crates/nx-interpreter/tests/edge_cases.rs` that the construction terminates, that a default still sees the other top-level values, that a default naming the value it fills is reported as an undefined variable, and that a genuine failure in a value the nested pass reached is still reported by the outer pass

## 6. Code generation

- [x] 6.1 Fill `ExportedRecord.type_params` for plain generic records in `typegen/model.rs` and expose each record's parameter order to the emitters
- [x] 6.2 TypeScript typegen: declare `Name<T, …>` with no default for a generic record, render an applied type as `Name<A, …>` in declaration order, and keep the update companion erased; verify with tests in `crates/nx-cli/src/typegen.rs` for each TypeScript scenario in the `cli-code-generation` delta
- [x] 6.3 C# typegen: stop erasing in `emit_record` when the record is a plain generic record, declare `Name<T, …>`, render an applied type as `Name<A, …>`, and keep the update companion erased; verify with tests for each C# scenario in the delta, including `Range<object>` inside a generic component contract
- [x] 6.4 Executable TypeScript (`crates/nx-codegen/src/emit.rs`): generic interface for the record, the parameter's own name in its fields, the instantiation for an applied type; verify with a case in `crates/nx-codegen/src/tests.rs` beside the `SkiaLayoutProps<TItem = unknown>` test and by type-checking the emitted module
- [x] 6.5 Round-trip one generated generic C# record through `System.Text.Json` and MessagePack in the .NET binding tests; rebuild `nx-ffi` first, then verify `dotnet test bindings/dotnet/NxLang.sln` passes
- [x] 6.7 Make a generic record's update companion generic in both languages: `update_companion_type_params` in `typegen/model.rs`, `Range_update<T>` with `start?: T` in TypeScript, and in C# `Range_update<T> : NxUpdate<Range<T>>` with a `RangeProperties<T>` key table, a `NxUpdateRecordJsonConverterFactory` in the SDK (an attribute cannot name `NxUpdateRecordJsonConverter<Range_update<T>>` — CS0416) and an emitted open generic formatter shim for MessagePack; verify a component's state companion still erases, a non-generic companion still names its converter directly, and the wire is unchanged
- [x] 6.8 Regenerate `UpdateRecords.g.cs` and add .NET tests that diff and apply at `Range<long>`, round-trip a patch through both serializers and apply it afterwards — the case an erased companion fails with `InvalidCastException` — and keep two instantiations apart; verify `dotnet test bindings/dotnet/NxLang.sln` passes and the fixture drift test still matches
- [x] 6.6 Resolve an applied type's parameter order through a dependency library's imported types as well as the local export graph (`ExportedTypeGraph::record_type_params`, `ImportedTypeKind::Record`), expand an imported alias whose target is an applied type in C#, and render the arguments in source order rather than dropping them where the declaration cannot be reached; verify with a two-library typegen test that both languages emit the instantiation rather than an open generic

## 7. Language service

- [x] 7.1 Classify a `Name:type` definition in a record as a type parameter rather than a record field in `positions.rs`, and show the same hover a component type parameter gets; verify by extending the `GENERIC` smoke test in `crates/nx-language-service/src/lib.rs` with a generic record, including that field completion at a construction site does not offer `T` as a missing field and that a hover inside `<Range T=int/>` does not panic

## 8. Documentation

- [x] 8.1 Add a "Generic records" section to `docs/src/content/docs/reference/syntax/types.md` covering declaration, the applied type, construction, invariance, aliases for non-name arguments, the companions, and the inheritance exclusion, with `Range`, `Pair` and `Page` examples; cross-link it from "Type parameters" in `functions.md`. The "Below the checker" paragraph says the update companion is generic in both languages and that only a component's state companion erases
- [x] 8.2 Update `nx-grammar.md` and `nx-grammar-spec.md` with `AppliedType`, `TypeArgument`, and the `type` keyword as a property type — which also records the component type-parameter syntax both files are missing; verify the EBNF matches `grammar.js`
- [x] 8.3 Note in the typegen documentation (`bindings/dotnet/README.md`) that an AOT serializer needs each generic instantiation named, covering the update companions as well as the records, and that `NxUpdateRecordJsonConverterFactory` reaches its converter by reflection; document the generic companion's C# shape with a `Diff`/`Apply` example verified by compiling and running it
- [x] 8.4 Add `examples/nx/generic-records.nx` and verify `nx check` accepts it

## 9. Verification

- [x] 9.1 Run `cargo test --workspace`, `cargo clippy --workspace`, the TypeScript runtime tests and `dotnet test`, and verify all pass
- [x] 9.2 Run `openspec validate add-record-type-parameters --strict` and verify it passes

## 10. Clippy cleanup and gate (folded in from `fix-clippy-diagnostics`)

- [x] 10.1 Replace the five `approx_constant` literals in `nx-cli`, `nx-hir`, and `nx-value` with `std::f64::consts` values or different test digits, and verify `cargo clippy -p nx-cli -p nx-hir -p nx-value --all-targets` reports no `approx_constant`
- [x] 10.2 Delete `crates/nx-syntax/benches/parse_benchmark.rs`, whose undeclared `criterion` dependency is the `E0432`, and verify no `[[bench]]` entry or `criterion` dev-dependency is left behind and `Cargo.lock` is unchanged
- [x] 10.3 Mark the raw-pointer `extern "C"` entry points in `crates/nx-ffi/src/lib.rs` as `unsafe` with `# Safety` sections, wrap Rust-side callers in `unsafe` blocks, and verify `cargo clippy -p nx-ffi --all-targets -- -D warnings` and `cargo test -p nx-ffi` pass
- [x] 10.4 Clean `nx-codegen` and verify `cargo clippy -p nx-codegen --all-targets -- -D warnings` passes and `cargo test -p nx-codegen` still passes
- [x] 10.5 Clean `nx-cli`, including its tests, and verify `cargo clippy -p nx-cli --all-targets -- -D warnings` and `cargo test -p nx-cli` pass
- [x] 10.6 Clean `nx-hir` and verify `cargo clippy -p nx-hir --all-targets -- -D warnings` and `cargo test -p nx-hir` pass
- [x] 10.7 Clean `nx-api`, `nx-types`, and `nx-syntax` and verify `cargo clippy -p nx-api -p nx-types -p nx-syntax --all-targets -- -D warnings` and their tests pass
- [x] 10.8 Clean `nx-diagnostics`, `nx-interpreter`, `nx-language-service`, and `bindings/node/native` and verify `cargo clippy --workspace --all-targets -- -D warnings` exits 0
- [x] 10.9 Add a `cargo clippy --workspace --all-targets -- -D warnings` step to the `rust` job in `.github/workflows/build.yml` after the format check
- [x] 10.10 Run `cargo test --workspace`, `pnpm -r build`, and `pnpm -r test` and verify all pass, confirming the cleanup changed no behavior
