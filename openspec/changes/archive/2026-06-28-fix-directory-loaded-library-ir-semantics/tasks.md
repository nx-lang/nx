## 1. Regression Coverage

- [x] 1.1 Add a minimal directory-loaded library graph fixture with `chat-link`, `question-flow`,
  `QuestionFlow`, and `FlowStep` style cross-library nominal type references.
- [x] 1.2 Add a native Rust regression test that builds a program artifact from the fixture through
  `LibraryRegistry` and proves validation, JSON evaluation, and IR generation use the same library
  semantics.
- [x] 1.3 Add a Node SDK Vitest regression that loads the fixture through
  `NxLibraryRegistry.loadFromDirectory`, builds an artifact, evaluates JSON, and calls
  `generateNxIr()`.

## 2. Semantic Binding Preservation

- [x] 2.1 Choose the binding snapshot location: extend `ModuleArtifact` or add a compact
  codegen-facing binding map owned by `ProgramArtifact`.
- [x] 2.2 Capture prepared semantic binding targets for source-provider modules during analysis,
  including value, type, and element namespaces.
- [x] 2.3 Capture prepared semantic binding targets for loaded library modules, including
  same-library peer declarations and declarations imported from dependency libraries.
- [x] 2.4 Ensure `ProgramBuildContext` and selected `ProgramArtifact` instances carry the full
  loaded-library dependency closure and its semantic binding data without re-reading source files.

## 3. IR Generation

- [x] 3.1 Update `build_codegen_program` type-reference resolution to use preserved semantic
  bindings instead of a partial visible-import reconstruction.
- [x] 3.2 Ensure nominal type references emitted for loaded library declarations resolve to the
  correct module-qualified record, union, enum, or type-alias declaration.
- [x] 3.3 Preserve structured `codegen-missing-semantic-data` diagnostics for genuinely incomplete
  artifacts and continue rejecting partial IR output.
- [x] 3.4 Assert generated IR for the regression fixture contains module-qualified references for
  both `QuestionFlow` and `FlowStep`.

## 4. Node SDK Metadata

- [x] 4.1 Change `NxIrMetadata.programFingerprint` declarations from `number` to `string` in the
  Node SDK source and any tracked generated declaration output.
- [x] 4.2 Ensure Node SDK IR parsing preserves `programFingerprint` as the decimal string emitted by
  the native IR generator.
- [x] 4.3 Update Node SDK README examples and tests to compare program fingerprints as strings.

## 5. Verification

- [x] 5.1 Run the relevant Rust API/codegen tests that cover library registry artifact creation,
  resolved program construction, and NX IR generation.
- [x] 5.2 Run the Node SDK typecheck and Vitest suite.
- [x] 5.3 Run `openspec status --change "fix-directory-loaded-library-ir-semantics"` and confirm the
  change is ready for implementation.
