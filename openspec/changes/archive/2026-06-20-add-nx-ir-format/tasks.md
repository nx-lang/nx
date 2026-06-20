## 1. IR Model And Emission

- [x] 1.1 Add an NX IR Rust module or crate to the workspace with public DTOs for program metadata, modules, declarations, references, types, schemas, expressions, diagnostics, and source provenance.
- [x] 1.2 Define IR version constants, runtime ABI metadata, required feature flags, and stable reference identifiers for modules, declarations, expressions, props, and state slots.
- [x] 1.3 Define the v1 expression schema for literals, slots, top-level references, calls, unary/binary operations, `if`, match-style `if is`, `let`, blocks, arrays, loops, index/member access, records, unions, enums, intrinsic elements, and component descriptors.
- [x] 1.4 Define component IR records for effective props, declared state, default expressions, content fields, abstract/external/concrete flags, descriptor construction, and component body expressions.
- [x] 1.5 Implement `ProgramArtifact` to NX IR lowering using resolved module-qualified references and existing semantic/type metadata.
- [x] 1.6 Implement unsupported-construct diagnostics for IR emission, including action-handler cases that cannot be represented by v1 IR.
- [x] 1.7 Implement deterministic JSON serialization for NX IR with stable ordering for all cache-significant collections.
- [x] 1.8 Add Rust golden tests for IR metadata, deterministic output, module-qualified references, expression encoding, component state metadata, canonical value metadata, and source provenance.

## 2. Public Emission Surfaces

- [x] 2.1 Add artifact-first Rust API entry points that emit NX IR JSON plus structured metadata from a successful `ProgramArtifact`.
- [x] 2.2 Add executable-generation API plumbing so IR emission is exposed separately from existing JavaScript and TypeScript source emission.
- [x] 2.3 Add native FFI entry points and refresh the generated C header for artifact-first IR emission.
- [x] 2.4 Add managed .NET DTOs and `NxRuntime`/`NxProgramArtifact` APIs for artifact-first IR emission.
- [x] 2.5 Add managed source/build-context convenience APIs that build a transient `NxProgramArtifact` before emitting IR.
- [x] 2.6 Add `nxlang codegen --target nx-ir` CLI support that writes one `.nxir.json` artifact and does not write generated runtime/source files for that request.
- [x] 2.7 Add Rust CLI tests for source-file IR output, workspace IR output with `--entry`, static diagnostic rejection, and source-format rejection.
- [x] 2.8 Add .NET binding tests for successful IR emission, diagnostic surfacing, and source convenience overloads.

## 3. TypeScript IR Runtime Preparation

- [x] 3.1 Add TypeScript runtime source, package/build configuration, and test harness integration in the repository's existing JavaScript/TypeScript tooling layout.
- [x] 3.2 Define public TypeScript types for NX IR input, prepared programs, diagnostics, results, canonical values, component descriptors, component state, and runtime options.
- [x] 3.3 Implement IR JSON parsing and preparation with validation for format identifier, schema version, runtime ABI, required features, structural references, and unknown operation tags.
- [x] 3.4 Implement prepared lookup tables for modules, declarations, expressions, function entrypoints, component entrypoints, schemas, and source provenance.
- [x] 3.5 Implement schema normalization and validation helpers for primitives, arrays, nullable values, records, unions, enums, component props, component state, and state patches.
- [x] 3.6 Implement prepared default evaluators for record fields, component props, and component state fields.

## 4. TypeScript Function Evaluation

- [x] 4.1 Implement the TypeScript IR evaluator execution context with slot-based local bindings, module-qualified declaration lookup, recursion/resource limit hooks, and diagnostic reporting.
- [x] 4.2 Implement literal, slot/reference, unary, binary, call, `let`, block, array, index, and member expression evaluation.
- [x] 4.3 Implement eager `if`, match-style `if is`, and `for` expression evaluation with authored-order semantics.
- [x] 4.4 Implement record, union case, enum member, and intrinsic element evaluation using canonical NX value shapes.
- [x] 4.5 Implement lossless handling or explicit rejection paths for numeric values that cannot safely round-trip through JavaScript numbers.
- [x] 4.6 Add TypeScript runtime tests for function evaluation parity against the native interpreter across primitives, arithmetic, conditionals, match, loops, arrays, records, unions, enums, member access, and cross-module calls.

## 5. TypeScript Component And State Runtime

- [x] 5.1 Implement atomic component descriptor construction with prop/content normalization and inherited default application.
- [x] 5.2 Implement component initialization that normalizes props, materializes initial state defaults, evaluates the component body, and returns rendered output plus host-owned state.
- [x] 5.3 Implement explicit-state component evaluation that normalizes supplied props and state and renders without replacing supplied state fields with defaults.
- [x] 5.4 Implement complete state validation/normalization and host-owned state patch application with all-or-nothing diagnostics.
- [x] 5.5 Implement runtime boundary diagnostics for missing required props/state, unknown fields, invalid enum members, type mismatches, unsupported features, and malformed IR.
- [x] 5.6 Add TypeScript runtime tests for descriptor construction, inherited defaults, initialization, explicit state evaluation, state patch success/failure, invalid boundary values, and conditional content based on state.

## 6. End-To-End Verification

- [x] 6.1 Add cross-runtime parity tests that emit IR from NX source or `ProgramArtifact`, execute it through the TypeScript runtime, and compare outputs with native interpreter evaluation.
- [x] 6.2 Add parity coverage comparing IR runtime descriptor/evaluation behavior with existing generated JavaScript behavior where both support the same component scenarios.
- [x] 6.3 Add tests proving existing JavaScript/TypeScript source codegen output remains unchanged unless IR output is explicitly requested.
- [x] 6.4 Add documentation or reference notes for the v1 `.nxir.json` format, TypeScript runtime API, CLI usage, and non-goals such as reactivity and NX-owned reducers.
- [x] 6.5 Run the relevant Rust test suite for IR emission, executable generation, CLI behavior, and interpreter parity.
- [x] 6.6 Run the TypeScript runtime test suite.
- [x] 6.7 Run the .NET binding test suite covering managed IR APIs.
