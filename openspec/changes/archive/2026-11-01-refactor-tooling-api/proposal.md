## Why

`nx-api` mixes shared program-building foundations with interpreter-backed evaluation. Code
generation and the language service depend on it for the foundations alone, so `nx-api` cannot
depend on them in turn: Cargo forbids the cycle. That has two costs:

- `nx-api` cannot evaluate through `nx-codegen` and `nx-ir-runtime`, which is exactly what retiring
  the HIR interpreter needs. Today the retirement would have to rewire codegen, the language service
  and every adapter at the same time as it changes evaluation.
- The resolved-program types live in `nx-interpreter`, so `nx-codegen` and `nx-api` depend on the
  interpreter for data types it does not execute, and the interpreter cannot be deleted.

Separating the foundation now lets the retirement become a change inside `nx-api`'s evaluation
functions, reviewed on its own.

## What Changes

- Extract workspaces, source graphs, libraries, standard libraries, program construction, shared
  program inspection (schema and visualizers), and binding-friendly diagnostics into a new
  `nx-program` crate.
- Move the resolved-program types from `nx-interpreter` into `nx-program`, and update every import.
  No re-exports are kept at the old interpreter paths (no backward compatibility is needed yet).
- Make `nx-codegen`, `nx-language-service`, and `nx-interpreter` depend on `nx-program` instead of
  `nx-api`. `nx-codegen` and `nx-language-service` no longer depend on `nx-interpreter` at all.
- Make `nx-api` the external Rust entry point above `nx-program`, `nx-codegen`,
  `nx-language-service`, `nx-interpreter`, and `nx-ir-runtime`. It re-exports the foundation types
  flat (they are the vocabulary of every external call) and the services as namespaces
  (`nx_api::codegen`, `nx_api::language_service`, `nx_api::ir_runtime`).
- Route the Node, WASM, C/.NET FFI, CLI and native graphics preview adapters through `nx-api`, and
  drop their direct dependencies on lower crates where nothing needs them. `nx-lsp` stays on
  `nx-language-service` directly: it is that service's protocol adapter, and going through `nx-api`
  would build the interpreter, code generator and IR runtime into the server for nothing.
- Keep behavior unchanged: interpreter-backed evaluation, component snapshots, resource defaults,
  NX text, diagnostics, generated artifacts, editor results and binding signatures.
- Record the layering as a spec and check the edges Cargo cannot reject on its own.

## Capabilities

### New Capabilities

- `rust-crate-layering`: which crate owns program building, which crate is the external entry point,
  and which dependency directions are allowed.

### Modified Capabilities

- `executable-code-generation`: the source-text accessors codegen uses come from `nx-program`
  instead of `nx-api`.

## Impact

- New `crates/nx-program`, holding about 13k lines (plus tests) moved from `nx-api` and
  `nx-interpreter`, including the embedded standard-library sources and `prelude.nx`.
- `nx-api` keeps its existing flat exports for evaluation, components, NX text and value conversion,
  and gains the service namespaces. Imports of resolved types change from `nx_interpreter::` to
  `nx_program::` (or `nx_api::`).
- CLI, C/.NET FFI, Node, WASM and `nx-graphics-skia`'s `preview` feature change their Cargo
  dependencies and imports; their public names, ABI and payloads do not change.
- Tests that call evaluation or codegen from inside the extracted modules move to `nx-api`
  integration tests; semantic expectations do not change.
- Out of scope: switching any execution path to the IR runtime, removing the interpreter, CLI IR
  execution, execution caches, a curated facade API, SDK redesign, and LSP previews. Interpreter
  retirement (`retire-hir-interpreter`, planned) follows as a separate change on top of this one.
