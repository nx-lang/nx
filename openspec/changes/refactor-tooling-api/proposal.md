## Why

`nx-api` mixes shared program-building foundations with external evaluation and inspection APIs.
Code generation and the language service depend on those foundations, preventing `nx-api` from
serving as an external facade above them. Separate these responsibilities before changing execution
engines so architecture and behavior can be reviewed independently.

## What Changes

- Extract workspaces, source graphs, libraries, standard libraries, program construction, shared
  program inspection, and binding-friendly diagnostics into `nx-program`.
- Move resolved-program types from `nx-interpreter` into `nx-program`; retain re-exports at their
  existing public paths so all consumers use the same types.
- Make `nx-codegen`, `nx-language-service`, and `nx-interpreter` depend on the shared foundation
  rather than on the external facade.
- Position `nx-api` above the foundation, code generation, language service, interpreter, and IR
  runtime. Expose existing tooling through delegated functions and namespaced re-exports.
- Route existing binding and CLI calls through the facade where they invoke those capabilities;
  retain low-level dependencies where a command implements functionality not yet owned by a library.
- Preserve every existing capability and its behavior, including interpreter-backed source
  evaluation, component snapshots, resource defaults, NX text, diagnostics, generated artifacts,
  editor results, and binding signatures.
- Document the layering and enforce the dependency boundary with a focused architecture check.

## Capabilities

### New Capabilities

None. This change reorganizes and exposes existing implementations; it introduces no language or
runtime behavior.

### Modified Capabilities

None. Existing requirements remain in force. `.openspec.yaml` sets `skip_specs: true` because this
is an architecture refactoring without spec-level behavior changes.

## Impact

- New `crates/nx-program`; internal ownership and imports change across the Rust workspace.
- `nx-api` remains the external Rust entry point, preserving existing exports while adding access
  to existing code-generation, language-service, and IR-runtime surfaces.
- CLI, LSP, C/.NET FFI, Node, and WASM adapters are checked and updated where appropriate.
- Tests that currently depend upward on `nx-api` move to the facade's integration-test layer when
  necessary; semantic expectations do not change.
- Interpreter retirement is a separate follow-up. This proposal does not modify
  `retire-hir-interpreter`, switch any execution path, add CLI IR execution, introduce execution
  caches, redesign SDK APIs, or add LSP previews or compile-time execution.
