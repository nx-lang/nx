## 1. Crate Setup

- [x] 1.1 Add `crates/nx-codegen` to the Cargo workspace with dependencies on `nx-api`, `nx-hir`, `nx-types`, `nx-diagnostics`, `nx-interpreter` for the shared resolved-program model, and shared utility crates as needed.
- [x] 1.2 Define public options and result types for executable code generation, including target language, output format, generated files, warnings, and diagnostics.
- [x] 1.3 Add crate-level tests and fixture helpers for building `ProgramArtifact` inputs from inline source and temporary workspace/library layouts.
- [x] 1.4 Add public read-only `nx-api` accessors for `ProgramArtifact` source text lookup and source entry iteration.

## 2. CodegenProgram Model

- [x] 2.1 Define `CodegenProgram`, `CodegenModule`, entrypoint metadata, module provenance, and module-qualified declaration/reference model types.
- [x] 2.2 Define codegen expression/declaration model types for the supported eager subset: literals, identifiers, calls, conditionals, blocks, arrays, loops, member/index access, records, unions/enums, and element expressions that serialize to `NxValue`.
- [x] 2.3 Preserve source-map inputs, source spans, inferred type metadata, runtime module IDs, local definition IDs, import references, and program fingerprint in the model where available.
- [x] 2.4 Add diagnostics for unsupported constructs or missing semantic data so codegen can fail before emitting incomplete output.

## 3. ProgramArtifact Builder

- [x] 3.1 Implement `ProgramArtifact` to `CodegenProgram` construction using `ResolvedProgram` modules, entrypoints, import tables, preserved `LoweredModule`s, and `ModuleArtifact` type environments.
- [x] 3.2 Resolve local, peer, and imported item references into module-qualified codegen references without rescanning visible names during emission.
- [x] 3.3 Map supported HIR expressions and top-level declarations into the `CodegenProgram` model with deterministic generated names.
- [x] 3.4 Reject artifacts with static error diagnostics, missing lowered modules, missing type environments, or unsupported executable constructs with actionable diagnostics.

## 4. Runtime Helpers

- [x] 4.1 Add host-neutral TypeScript runtime helper source for canonical `NxValue` values, record discriminators, union case discriminators, enum bare strings, element-like records, and runtime errors.
- [x] 4.2 Add JavaScript runtime helper output equivalent to the TypeScript helper surface and free of TypeScript-only syntax.
- [x] 4.3 Ensure runtime helpers expose no public reactive signal, subscription, invalidation, dependency-graph, component lifecycle, dispatch, or action-handler APIs in this change.
- [x] 4.4 Add runtime helper unit tests covering value construction, enum encoding, record/union discriminators, element-like record shapes, and error helpers.

## 5. Shared TypeScript/JavaScript Emitter

- [x] 5.1 Implement shared module/file planning for generated program modules, runtime helper imports, support files, and entrypoint exports.
- [x] 5.2 Implement one emitter pipeline with a TypeScript mode that includes type-only syntax and a JavaScript mode that omits type-only syntax.
- [x] 5.3 Emit readable source for supported declarations and expressions using stable names derived from NX declarations where possible.
- [x] 5.4 Emit coherent imports for cross-module and imported-library references represented by `CodegenProgram`.
- [x] 5.5 Add snapshot tests for primitive root functions, function calls, conditionals, loops, records, unions/enums, element-like records, and cross-module imports in both target modes.

## 6. JavaScript Execution And Target Agreement

- [x] 6.1 Verify JavaScript output is executable ESM and does not require a TypeScript runtime compiler.
- [x] 6.2 Verify JavaScript output covers the same supported eager subset as TypeScript while omitting type-only syntax.
- [x] 6.3 Add JavaScript snapshot tests proving emitted files are ESM-compatible and contain no TypeScript-only syntax.
- [x] 6.4 Add target-agreement tests proving generated TypeScript and JavaScript produce equivalent serialized `NxValue` payloads for supported programs.

## 7. CLI And Public API

- [x] 7.1 Add public `nx-codegen` APIs for building `CodegenProgram` and emitting TypeScript or JavaScript generated files from a `ProgramArtifact`.
- [x] 7.2 Add `nxlang codegen` CLI parsing for executable TypeScript and JavaScript targets, input selection, output paths, and diagnostic reporting.
- [x] 7.3 Wire `nxlang codegen` through the existing source/workspace analysis pipeline to build `ProgramArtifact`s before invoking `nx-codegen`.
- [x] 7.4 Rename the existing `nx-cli` internal `codegen` module to a type-generation-specific name such as `typegen` so `codegen` consistently refers to executable generation.
- [x] 7.5 Add `nxlang typegen` as the primary types-only CLI surface for TypeScript and C# DTO/type generation without emitting executable NX behavior.
- [x] 7.6 Remove or replace the old `nxlang generate` command surface so types-only generation is exposed through `nxlang typegen`.
- [x] 7.7 Verify `nxlang typegen --language typescript` and `nxlang typegen --language csharp` preserve the existing DTO/type generation semantics.
- [x] 7.8 Update README.md, bindings/dotnet/README.md, and relevant CLI documentation to use `nxlang typegen` for types-only generation and `nxlang codegen` for executable generation.

## 8. Parity And Validation

- [x] 8.1 Add interpreter parity tests comparing generated output execution with interpreter evaluation for primitives, arithmetic, conditionals, arrays, loops, and function calls.
- [x] 8.2 Add interpreter parity tests comparing serialized `NxValue` payloads for records, discriminated unions, enum values, element-like records, and cross-module imports.
- [x] 8.3 Add negative tests proving unsupported constructs and invalid artifacts return diagnostics and emit no executable output.
- [x] 8.4 Add negative tests proving component lifecycle, dispatch, and action-handler codegen are rejected until a later component/reactivity change supports them.
- [x] 8.5 Add deterministic output tests for module ordering, declaration ordering, import ordering, generated identifiers, and record/property ordering.
- [x] 8.6 Add tests proving `nx-codegen` can retrieve preserved source text from `ProgramArtifact` through the public `nx-api` accessors and produce source-map metadata.
- [x] 8.7 Run Rust workspace tests and targeted CLI/codegen tests for the completed implementation.

## 9. Direct Record Objects And Strong TypeScript

- [x] 9.1 Update the OpenSpec proposal, design, and executable-code-generation spec so records and enums are emitted as plain JavaScript data with TypeScript-only strong typing layered on top.
- [x] 9.2 Extend the `CodegenProgram` model and builder to preserve record field type metadata and enough record declaration metadata to emit strongly typed TypeScript records in deterministic field order.
- [x] 9.3 Replace normal record helper emission with direct object literals that write `$type` and fields in emitter-controlled order without runtime sorting.
- [x] 9.4 Replace enum helper emission with TypeScript `as const` value objects plus `typeof Enum[keyof typeof Enum]` derived types, and JavaScript frozen value objects containing plain member strings.
- [x] 9.5 Remove `nxRecord`, `nxEnum`, and `nxArray` from generated runtime helpers and imports, keeping helpers only for element-like records and runtime errors.
- [x] 9.6 Update parity tests so object property order is not semantic, using parsed value comparison instead of byte-for-byte JSON string comparison.
- [x] 9.7 Update snapshots/documentation-adjacent tests and run targeted `nx-codegen` plus workspace test suites.
