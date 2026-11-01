## 1. Baseline

- [ ] 1.1 Record current Rust, CLI, LSP, .NET, Node, WASM and playground test results and the built `.wasm` size; verify pre-existing failures are identified before moving code and the commands and results are recorded in the PR.
- [ ] 1.2 List the tests that mix program building with evaluation or code generation (design.md decision 4) and the direct crate dependencies each adapter actually imports; verify each test has a destination layer and each adapter dependency is marked keep, route through `nx-api`, or remove.

## 2. Extract the foundation

- [ ] 2.1 Add the `nx-program` workspace crate, move the resolved-program types there, make `nx-interpreter` import them from `nx-program` without re-exporting them, and update every consumer's imports; verify interpreter tests pass and serde representations are unchanged.
- [ ] 2.2 Move workspace, source graph, diagnostics, artifact/library construction, standard libraries, `std/` sources and `prelude.nx` into `nx-program`, keeping the flat `nx-api` exports by re-export; verify foundation tests pass and library identities, sources, fingerprints and diagnostics are unchanged.
- [ ] 2.3 Move schema and program-visualizer inspection into `nx-program`, adding only the cross-crate accessors the split needs; verify schema and visualizer fixtures pass without changed expectations and `nx-program` depends on none of the interpreter, codegen, language service, IR runtime or `nx-api`.
- [ ] 2.4 Move evaluation-dependent tests out of the extracted modules into `nx-api` integration tests; verify the relocation list from 1.2 accounts for every moved case and no evaluation coverage is dropped.

## 3. Retarget lower services

- [ ] 3.1 Change `nx-codegen` to import from `nx-program` and remove its dependencies on `nx-api` and `nx-interpreter`; move interpreter-parity tests into `nx-api` integration tests; verify codegen and relocated tests pass and emitted IR/JavaScript goldens are unchanged.
- [ ] 3.2 Change `nx-language-service` to import from `nx-program` and remove its dependency on `nx-api`; verify its tests pass with unchanged editor results.
- [ ] 3.3 Verify `nx-api` evaluation, components, value conversion and NX text still use the interpreter with the moved types; verify existing evaluation, limit, diagnostic, lifecycle, snapshot and formatting fixtures pass unchanged.

## 4. Establish the external facade

- [ ] 4.1 Add `nx-api` dependencies on `nx-codegen`, `nx-language-service` and `nx-ir-runtime` with `codegen`, `language_service` and `ir_runtime` whole-crate re-exports; verify an `nx-api` integration test builds a program, emits IR, runs it through `ir_runtime`, and evaluates the same program through the flat interpreter-backed API.
- [ ] 4.2 Route C/.NET FFI, Node and WASM service calls through `nx-api` and remove direct dependencies they no longer need (including unused ones such as `nx-ffi`'s); verify the adapters build and their public names, ABI, disposal, error and payload assertions are unchanged.
- [ ] 4.3 Route CLI and `nx-graphics-skia` `preview` service calls through `nx-api`, keeping justified command-specific low-level dependencies, and leave `nx-lsp` on `nx-language-service`; verify CLI, LSP and graphics preview suites pass and `nxlang run` still uses the interpreter with unchanged output and exit behavior.

## 5. Verify and document the architecture

- [ ] 5.1 Add a Cargo-metadata check to the normal repository checks for the edges Cargo does not reject: dev-dependencies on `nx-api` from crates below it, and any edge between `nx-program` and `nx-ir-runtime`; verify it passes on the new graph and fails when one such edge is added locally.
- [ ] 5.2 Run `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets`, `pnpm run verify:crates`, and the affected .NET, Node, WASM and playground suites; verify results match the baseline and investigate any changed artifact, snapshot, diagnostic or output rather than accepting it.
- [ ] 5.3 Compare the built `.wasm` size with the baseline; verify every existing native and WASM target still builds and record any material size change and its cause in the PR.
- [ ] 5.4 Update `crates/README.md` (replacing its stale phase notes and dependency graph with the ownership table and layering from design.md) and affected crate READMEs; verify any facade usage example matches the `nx-api` integration test.
- [ ] 5.5 Run `openspec validate refactor-tooling-api --strict` and review the implementation against the scope exclusions; verify no execution path switches to the IR runtime and the interpreter is not removed.
