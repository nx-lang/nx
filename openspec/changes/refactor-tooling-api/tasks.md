## 1. Inventory and baseline

- [ ] 1.1 Inventory `nx-api` exports, resolved-type consumers, lower-crate upward dependencies, embedded source paths, and tests mixing building with execution; verify the inventory maps each item to the owner and public path specified in design.md.
- [ ] 1.2 Record current Rust, CLI, LSP, .NET, Node, WASM and playground test results, generated-artifact/snapshot fixtures, and representative WASM size/build measurements; verify baseline failures are identified before moving code and commands/results are recorded in the PR.

## 2. Extract the foundation

- [ ] 2.1 Add the `nx-program` workspace crate, move the resolved-program types there, and re-export those exact types from `nx-interpreter`; verify interpreter tests pass and a compile check accepts the same value through both public paths.
- [ ] 2.2 Move workspace, source graph, diagnostics, artifact/library construction, standard libraries and embedded resources into `nx-program`, preserving their existing `nx-api` exports; verify foundation tests pass and existing library identities, sources, fingerprints and diagnostics are unchanged.
- [ ] 2.3 Move schema and program-visualizer inspection into `nx-program`, retaining old facade exports and adding narrow cross-crate accessors where required; verify schema/visualizer fixtures pass without changed expectations and `nx-program` has none of the forbidden service dependencies.
- [ ] 2.4 Relocate program/API tests according to the layer they exercise, including execution-dependent tests formerly embedded in extracted modules; verify a test relocation inventory accounts for the moved cases and no source-execution coverage is dropped.

## 3. Retarget lower services

- [ ] 3.1 Change `nx-codegen` implementation imports to `nx-program` and remove its dependency on `nx-api` and resolved types owned by the interpreter; move native-evaluation parity tests into facade integration tests where necessary; verify codegen and relocated tests pass and emitted IR/JavaScript goldens are unchanged.
- [ ] 3.2 Change `nx-language-service` to consume foundation workspace, diagnostic and inspection APIs; verify its tests pass with unchanged editor results and no dependency on `nx-api`.
- [ ] 3.3 Verify retained `nx-api` evaluation, components, value conversion and NX text consume the extracted types while continuing to use the interpreter; verify existing evaluation, limit, diagnostic, lifecycle, snapshot and formatting fixtures pass unchanged.

## 4. Establish the external facade

- [ ] 4.1 Add `nx-api` dependencies and namespaced re-exports for codegen, language service and IR runtime while preserving legacy exports; verify an external-style integration test compiles and exercises source-to-IR emission, legacy evaluation, an editor operation and direct IR execution through the facade.
- [ ] 4.2 Route C/.NET FFI, Node and WASM service calls through the facade and remove redundant direct service dependencies; verify the adapters build and their existing public names, ABI, disposal, error and payload assertions remain unchanged.
- [ ] 4.3 Route existing CLI and LSP service calls through the facade, retaining justified command-specific low-level dependencies; verify CLI/LSP suites pass and `nxlang run` still uses the interpreter with unchanged output and exit behavior.

## 5. Verify and document the architecture

- [ ] 5.1 Add a Cargo-metadata dependency check to normal repository checks for foundation exclusions, lower-service independence from `nx-api` including test-only upward edges, and IR-runtime compiler independence; verify it passes on the intended graph and rejects representative forbidden edges in check-level test data.
- [ ] 5.2 Run `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets`, and affected .NET/Node/WASM/playground suites using repository workflows; verify results against the baseline and investigate any changed artifact, snapshot, diagnostic or output expectation rather than accepting behavior changes.
- [ ] 5.3 Check existing native and WASM target builds and compare representative WASM size/build measurements; verify facade aggregation preserves target support and record any material size/build change and its cause.
- [ ] 5.4 Update `crates/README.md`, affected crate READMEs and binding architecture guidance with the ownership table, dependency direction and facade usage examples; verify examples compile or match the facade integration test and interpreter retirement is explicitly a separate follow-up.
- [ ] 5.5 Run `openspec validate refactor-tooling-api --strict` and review the final implementation against the scope exclusions; verify validation passes, no interpreter removal or execution switch is included, and the existing `retire-hir-interpreter` artifacts remain untouched.
