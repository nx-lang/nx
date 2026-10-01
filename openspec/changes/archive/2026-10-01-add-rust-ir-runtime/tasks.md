## 1. Extract the format crate

- [x] 1.1 Create `crates/nx-ir` and add it to the workspace; move `ir_image.rs`, `ir_explain.rs`, and from `ir.rs` the schema, ABI and feature constants, `kinds`, `IrItem`, `NxIrArtifact`, `NxIrModuleEntry`, `NxIrDebug` and `NxIrDebugSpans`; verify `cargo build -p nx-ir` succeeds and `cargo tree -p nx-ir` lists no other `nx-*` crate
- [x] 1.2 Make `nx-codegen` depend on `nx-ir`, keep the emitter and `ir_bundle.rs` there, turn `NxIrArtifact::metadata` into an emitter-side function, and move the image tests that need the emitter next to `ir_corpus_tests.rs`; verify `cargo test -p nx-codegen` passes with the corpus images unchanged
- [x] 1.3 Point `nx-cli`, `nx-ffi`, `bindings/node/native` and `bindings/wasm/native` at `nx-ir` for `NxIrImage`, `explain_nx_ir_image` and `ExplainError`, with no re-exports from `nx-codegen`; verify `cargo build --workspace` and the Node and wasm binding test suites pass
- [x] 1.4 Add `NxIrImageBuf` to the reader: it owns the bytes with the layout validation found and rebuilds an `NxIrImage` view without validating again; verify with a test that the rebuilt view answers every accessor identically for each corpus image

## 2. Runtime crate: prepare and link

- [x] 2.1 Create `crates/nx-ir-runtime` depending on `nx-ir`, `nx-value` and `serde`, with the clippy lints of design decision 4 denied in non-test code; verify `cargo tree -p nx-ir-runtime` lists no compiler crate
- [x] 2.2 Implement `NxIrRuntimeError` and the diagnostic type with code, message, declaration and optional span; verify with a unit test that a diagnostic built for a declaration with debug spans carries `identity::name` and the span
- [x] 2.3 Implement `PreparedModule::prepare` over `Arc<[u8]>`: validation, ABI and required-feature checks, declaration index by name, entrypoint tables, lazy per-node decoding into the `Node` enum; verify with tests for a valid image, an unknown feature, a wrong ABI, a wrong schema version, and an operator node without `occurrence-v1`
- [x] 2.4 Implement linking with a host resolver, the version check and its opt-out, the missing-module and missing-declaration checks, and the single-call prepare-and-link for a self-contained image; verify with tests for each `nx-ir-link-*` code and for `nx-ir-unlinked`
- [x] 2.5 Extend `prelude_image_tests.rs` to write `crates/nx-ir-runtime/src/prelude.nxir` and to check it for staleness; embed it and serve it during linking, prepared once; verify the corpus `ranges` program links with no resolver entry for the prelude and that editing `prelude.nx` fails the staleness test

## 3. Values and the boundary

- [x] 3.1 Implement the internal value type and conversion to and from `NxValue` by the rules of design decision 3; verify with round-trip tests covering both integer widths, both float widths, host `null`, and records with `$type`
- [x] 3.2 Implement boundary normalization against IR types: primitives, occurrences, records with discriminators and inheritance, abstract records, unions, enums, property unions, update records, `Function` records and `Range`; verify with source-level tests of the boundary scenarios in `nx-codegen`'s `ir_runtime_tests.rs`, asserting codes and named fields
- [x] 3.3 Implement the canonical text forms, including the shortest round-trip `float32` form, and export `float32_text`; verify against the `conversions` corpus program and the text scenarios of `typescript-ir-runtime`

## 4. Evaluation

- [x] 4.1 Implement the evaluator for literals, slots, references, arithmetic including the explicit `float32` operators, comparison, logic, `concat`, `text`, conditionals, match with every pattern kind, `for`, `forRange`, indexing and member access, with the depth budget on every recursion; verify the `expressions`, `conversions`, `occurrences`, `occurrence-lifting`, `occurrence-patterns` and `ranges` corpus programs
- [x] 4.2 Implement calls, parameter defaults, named calls of function values, function values in output, record and union construction, update records, property references and the four update intrinsics; verify the `records`, `generic-records`, `function-values`, `parameter-defaults` and `two-module` corpus programs
- [x] 4.3 Implement `evaluate_function` and `call_function` with result normalization, the optional-result `null`, and the limits options; verify with tests for a missing entrypoint, a dropped extra argument, a missing parameter, runaway recursion and an oversized range
- [x] 4.4 Export `apply`, `merge`, `diff` and `changed` over `NxValue`; verify with the helper scenarios of the spec

## 5. Components

- [x] 5.1 Implement descriptor construction with content binding by occurrence and handler properties; verify the `components`, `snippet` and `trailing-element` corpus programs
- [x] 5.2 Implement `initialize_component`, `evaluate_component`, `normalize_component_state` and `apply_component_state_patch`, including a parent instance, a supplied state and token assignment; verify with tests for each initialization scenario of the spec
- [x] 5.3 Implement `dispatch_component_actions` with live state reads for owned handlers, effects, atomic failure and fresh tokens; verify the `handlers` corpus lifecycles and the dispatch scenarios of the spec
- [x] 5.4 Make `ComponentInstance` serializable as a flat table of values with the program's image hashes, with `Program::restore_component_instance` validating fully as the only way to build one from a serialized form, and an image-hash-only check in dispatch and child initialization; verify that a restored instance dispatches identically, that restoring bytes from other images, with an altered node index or with an altered state fails with a diagnostic, that an in-memory instance dispatched against another program is refused, and that a thousand-row instance stores its shared list once

## 6. Conformance and differential tests

- [x] 6.1 Add `crates/nx-ir-runtime/tests/corpus.rs` running every corpus entrypoint and lifecycle in both variants with numeric-aware, order-insensitive comparison; verify it reports each program by name, passes, and runs in the existing Rust CI test job
- [x] 6.2 Add the damage tests: every four-byte truncation of every corpus image, and every cell of every corpus image overwritten with `0`, `1`, `0xffffffff` and `0x7ffffff0`, running the entrypoints and lifecycles of whatever prepares; verify no case panics, including under `cargo test --release`
- [x] 6.3 Add the differential test in `nx-codegen` that compiles source, emits IR, runs the Rust runtime and compares with `nx-interpreter` for the programs `runtime/typescript/test/emitted-ir.test.mjs` covers, a source exercising every operator and a source whose dispatches show in the rendered output; verify it passes and that a deliberately wrong operator in the runtime fails it

## 7. Documentation

- [x] 7.1 Write `crates/nx-ir-runtime/README.md` with the prepare, link, evaluate and lifecycle examples in Rust and the limits and diagnostics tables; verify the examples compile as doc tests
- [x] 7.2 Update `crates/README.md` for the two new crates and `docs/nx-ir-format.md` where it names the TypeScript runtime as the only reader or the only holder of the prelude image; verify by reading the changed sections against the code
- [x] 7.3 Run `openspec validate add-rust-ir-runtime --strict` and `cargo clippy --workspace --all-targets`; verify both report no errors
