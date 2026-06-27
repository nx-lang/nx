## 1. SDK Naming and Package Setup

- [x] 1.1 Rename the managed .NET project/package/assembly from `NxLang.Runtime` to `NxLang.Sdk`.
- [x] 1.2 Rename managed test projects, solution entries, package metadata, and package asset paths that refer to `NxLang.Runtime`.
- [x] 1.3 Remove or avoid any `NxLang.Runtime` compatibility package, alias project, or forwarding assembly.
- [x] 1.4 Create the `bindings/node` package scaffold for `@nx-lang/sdk-node` with package metadata, TypeScript config, test config, and README stub.
- [x] 1.5 Add a napi-rs native crate for the Node SDK and register it in the Cargo workspace.
- [x] 1.6 Add package scripts for local native builds, TypeScript builds, tests, and clean rebuilds.
- [x] 1.7 Configure package exports and generated TypeScript declarations for ESM consumption from Node.

## 2. Native Binding Layer

- [x] 2.1 Implement napi-rs conversion helpers for source strings, `Buffer` or `Uint8Array` bytes, output formats, JSON payloads, and diagnostics.
- [x] 2.2 Implement native `NxWorkspace` and workspace module creation over the existing Rust workspace model.
- [x] 2.3 Implement native `NxLibraryRegistry` and `NxProgramBuildContext` wrappers over the existing Rust host APIs.
- [x] 2.4 Implement native `NxProgramArtifact` creation from source and from workspace plus explicit entry identity.
- [x] 2.5 Implement native resource disposal and disposed-resource checks for registries, build contexts, and program artifacts.

## 3. TypeScript Public API

- [x] 3.1 Add TypeScript wrapper classes for `NxWorkspace`, `NxLibraryRegistry`, `NxProgramBuildContext`, and `NxProgramArtifact`.
- [x] 3.2 Add public TypeScript types for workspace modules, diagnostics, spans, labels, IR generation results, evaluation options, and output formats.
- [x] 3.3 Add typed `NxEvaluationError`, `NxNativeError`, and disposed-resource error classes that preserve structured diagnostic data.
- [x] 3.4 Hide all raw native handles from the public API while still supporting deterministic `dispose()` and `[Symbol.dispose]` where available.

## 4. Host Operations

- [x] 4.1 Expose workspace validation against a supplied build context and return structured diagnostics as data.
- [x] 4.2 Expose source and workspace program artifact build APIs with structured diagnostic errors on build failure.
- [x] 4.3 Expose deterministic NX IR generation from `NxProgramArtifact` and source convenience APIs.
- [x] 4.4 Expose `root()` evaluation from source and program artifacts to normalized JSON values.
- [x] 4.5 Expose byte evaluation from source and program artifacts using Node-compatible binary results.
- [x] 4.6 Ensure unsupported named-entrypoint requests fail explicitly without JavaScript-side global declaration lookup.

## 5. Documentation and Packaging

- [x] 5.1 Document the Node SDK package purpose, Node-only support posture, and distinction from the pure TypeScript NX IR runtime.
- [x] 5.2 Document local source consumption, including dependency installation, native build steps, imports, and test execution.
- [x] 5.3 Document workspace validation, workspace build, IR generation, JSON evaluation, byte evaluation, diagnostics, and disposal examples.
- [x] 5.4 Add package metadata or documentation for the future napi-rs npm/prebuild distribution path and supported platform targets.
- [x] 5.5 Update .NET SDK documentation and examples to use `NxLang.Sdk` and remove `NxLang.Runtime` package/project references.

## 6. Tests and Verification

- [x] 6.1 Update managed .NET test references and package assertions for `NxLang.Sdk`.
- [x] 6.2 Add Node tests for valid workspace validation and invalid workspace diagnostic aggregation.
- [x] 6.3 Add Node tests for duplicate normalized module identities and missing workspace entry identities.
- [x] 6.4 Add Node tests for source and workspace program artifact builds against build contexts.
- [x] 6.5 Add Node tests for deterministic NX IR JSON generation and metadata shape.
- [x] 6.6 Add Node tests for JSON evaluation, byte evaluation, missing `root()` diagnostics, and invalid source diagnostics.
- [x] 6.7 Add parity tests or fixtures comparing Node behavior with existing Rust, .NET SDK, or CLI behavior for representative inputs.
- [x] 6.8 Run Rust, Node, and relevant existing binding tests needed to verify the new package and shared host behavior.
