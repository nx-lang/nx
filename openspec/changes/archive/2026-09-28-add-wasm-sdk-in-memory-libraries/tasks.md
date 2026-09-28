## 1. Registry model

- [x] 1.1 Introduce a normalized logical library root as the `LibraryRegistry` key, derived from the canonical path for directory loads; verify existing `nx-api` library-registry and directory-library IR tests pass unchanged
- [x] 1.2 Add an in-memory library load (root, optional version, library-relative modules) over `build_library_artifact_from_sources`; verify a unit test loads a multi-module library with no filesystem access
- [x] 1.3 Resolve relative library imports against loaded logical roots with the workspace root-escape rules; verify tests for `../question-flow` resolution, missing-dependency diagnostics with no partial snapshot, and cycle rejection
- [x] 1.4 Refuse reloading a root with different content or version and no-op an identical reload; verify both cases in tests
- [x] 1.5 Add a batch load that orders libraries by dependency; verify a test loading `chat-link` before `question-flow` in the input list succeeds

## 2. Build context and implicit imports

- [x] 2.1 Resolve implicit-import identities against workspace modules then loaded library roots, with an ambiguity diagnostic on collision; verify tests for library-root implicit import, collision, and unknown identity
- [x] 2.2 Verify an implicitly imported library produces the same program (fingerprint and emitted IR) as written wildcard imports of it
- [x] 2.3 Emit library module images under `<root>/<module>` with the library version, from the library snapshot; verify two tenant programs emit byte-identical `libraries/question-flow/QuestionFlow.nx` images
- [x] 2.4 Verify in-memory and directory loads of the same sources emit byte-identical IR and equal diagnostics

## 3. Wasm SDK

- [x] 3.1 Add native ABI operations for registry create/dispose, library load (single and batch), build context create/dispose, and workspace validation; bump the ABI version; verify the wrapper refuses the previous module version
- [x] 3.2 Accept an optional build context in workspace builds; verify builds against a context reuse the loaded snapshots (no re-analysis, observable through a load counter in tests)
- [x] 3.3 Expose `NxLibraryRegistry`, build contexts and `validateWorkspace` in the TypeScript wrapper with disposed-resource and crashed-host behavior; verify lifecycle tests
- [x] 3.4 Update the wasm README scope table and examples (registry, implicit library imports, validation, library module emission)

## 4. Node SDK parity

- [x] 4.1 Add in-memory library loading (single and batch) to the Node SDK native binding and wrapper; verify tests
- [x] 4.2 Extend the parity tests to an in-memory `chat-link` + `question-flow` style fixture: byte-identical entry and library images and equal validation diagnostics across Node and wasm

## 5. Runtime round trip and release

- [x] 5.1 Add a test that prepares the emitted library images once with `@nx-lang/ir-runtime`, links two different entry images against them, and evaluates both entrypoints
- [x] 5.2 Run `cargo test`, the wasm and node package tests, and `pnpm run verify:package` for `@nx-lang/sdk-wasm`; all pass
