## Why

NX IR can be run in JavaScript by `@nx-lang/ir-runtime`, but a Rust host has only `nx-interpreter`,
which evaluates HIR and therefore links the whole compiler: tree-sitter, lowering and the type
checker. A Rust host that already holds compiled images — the first one planned is a Skia renderer
written in Rust — should be able to run them with a small dependency that knows nothing about NX
source. The image reader that such a runtime needs already exists in Rust, but it lives inside
`nx-codegen`, which depends on the compiler.

## What Changes

- Add a leaf crate, `nx-ir`, holding the NX IR format: the schema, ABI and required-feature
  constants, the kind tables, the artifact model, the image writer, the validating in-place reader
  and the explainer. `nx-codegen` keeps the emitter and depends on `nx-ir`; the CLI and the bindings
  import the format from `nx-ir`.
- Add a crate, `nx-ir-runtime`, depending only on `nx-ir` and `nx-value`, with a Rust API that
  mirrors `@nx-lang/ir-runtime`:
  - prepare a module image, link an entry module against prepared modules through a host resolver,
    and prepare a self-contained image in one call;
  - evaluate a function entrypoint, and call the function a `Function` record names;
  - construct a component descriptor, initialize a component, evaluate it from explicit state, and
    dispatch an action batch against an instance;
  - normalize and patch component state, and the update helpers `apply`, `merge`, `diff` and
    `changed`;
  - call-depth and range-length limits, and diagnostics carrying the same `nx-ir-*` codes, the
    declaration as `identity::name`, and a span when the image has its debug section.
- Values cross the API as `nx_value::NxValue`.
- A component instance is plain data that can be serialized and handed back to a later dispatch
  against the same program.
- The crate carries the compiled prelude image and supplies it during linking, as the TypeScript
  runtime does. The existing prelude generator writes the Rust runtime's image beside the
  TypeScript one, and the staleness test covers both.
- Tests: the conformance corpus (entrypoints and lifecycles, with and without debug sections), the
  truncation and cell-overwrite damage runs, and a differential run that compiles NX source, emits
  IR, and compares this runtime with `nx-interpreter`.

Not in this change: language bindings over the new runtime, publishing to crates.io, a CLI command
that runs an image, benchmarks, and the Skia integration itself. Retiring `nx-interpreter` in favour
of this runtime is the separate change `retire-hir-interpreter`, which depends on this one.

## Capabilities

### New Capabilities

- `rust-ir-runtime`: A Rust crate that prepares, links and evaluates NX IR images without the NX
  compiler, with the same observable results and diagnostics as the TypeScript IR runtime.

### Modified Capabilities

- None. The Rust runtime's copy of the prelude image is specified in `rust-ir-runtime`.

## Impact

- New crates `crates/nx-ir` and `crates/nx-ir-runtime`, added to the workspace.
- `crates/nx-codegen`: `ir_image.rs`, `ir_explain.rs`, the format constants, the `kinds` module and
  the artifact model move to `nx-ir`; `ir.rs` keeps the emitter; `prelude_image_tests.rs` writes a
  second output.
- `crates/nx-cli`, `crates/nx-ffi`, `bindings/node/native`, `bindings/wasm/native`: import
  `NxIrImage`, `explain_nx_ir_image` and `ExplainError` from `nx-ir`. No behavior change.
- `docs/nx-ir-format.md` and `crates/README.md`: name the Rust runtime and the new crate layout.
- No change to the IR format, the corpus, or any published package.
