# NX Rust crates

The compiler, tooling services and external API share a program foundation. Dependency arrows
below mean “depends on”; low-level syntax/type/value edges are omitted for clarity.

```text
CLI / bindings / graphics preview -> nx-api
nx-api -> nx-codegen, nx-language-service, nx-program, nx-ir-runtime
nx-codegen, nx-language-service -> nx-program
nx-lsp -> nx-language-service
nx-ir-runtime -> nx-ir, nx-value
```

## Ownership

| Crate | Responsibility |
| --- | --- |
| `nx-diagnostics` | Internal compiler diagnostics and labels |
| `nx-syntax` | tree-sitter parsing and concrete syntax trees |
| `nx-hir` | Lowering, declarations and semantic model |
| `nx-types` | Type checking and inference |
| `nx-program` | Workspaces, source graphs, libraries and standard libraries, program construction, resolved-program model, tooling diagnostics, schema and visualizer inspection |
| `nx-codegen` | Resolved-program lowering to executable JavaScript/TypeScript and NX IR |
| `nx-language-service` | Editor analysis, completions, hover, diagnostics and source inspection |
| `nx-api` | External Rust entry point: flat foundation/formatting exports and `codegen`, `language_service`, `ir_runtime` namespaces |
| `nx-ir` | Compiled image format, reader/writer and explainer |
| `nx-value` | Canonical host values and their JSON/MessagePack encodings |
| `nx-ir-runtime` | Preparing, linking and executing compiled images; component instance trees |
| `nx-cli`, `nx-ffi`, Node/WASM native bindings | CLI and transport adapters over `nx-api` |
| `nx-lsp` | LSP protocol adapter directly over `nx-language-service` |
| `nx-graphics`, `nx-graphics-skia` | Graphics model and renderer; native preview composes compiler/runtime through `nx-api` |

`nx-program` depends on compiler foundations, with no dependency on an execution engine, codegen,
the language service or `nx-api`. Services depend on that shared foundation. No crate below the
facade may depend on `nx-api`, even through a test-only dependency. Cross-service and IR-runtime
parity tests therefore live in `nx-api/tests/`; pure construction tests stay in `nx-program`.

The facade exposes the existing services as whole-crate namespaces. Execution uses emitted IR through `nx-api::ir_runtime`.

The LSP server stays directly on `nx-language-service` so building it does not bring in execution
engines. A host that only executes compiled images uses `nx-ir-runtime` directly and builds no
compiler or facade. `nx-ir-runtime` and `nx-program` must not depend on each other.

## Building and checking

Use the Rust toolchain pinned by the repository and install the pnpm workspace (Node 22.18 or
newer) for tests that execute generated JavaScript or check generated TypeScript.

```sh
pnpm install
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
pnpm run check:crate-layering
pnpm run verify:crates
```

Cargo rejects normal dependency cycles. `check:crate-layering` checks the remaining boundaries
using Cargo metadata, including dev, build, optional and target-specific declarations, and tests
that injected forbidden edges fail. CI runs this check alongside formatting and Clippy.

## Published runtime crates

Only `nx-ir`, `nx-value` and `nx-ir-runtime` are published to crates.io, at each NX release's
version. All compiler/tooling crates remain `publish = false`. The runtime crates build natively,
for `wasm32-wasip1` and for `wasm32-unknown-emscripten`. `pnpm run verify:crates` builds a host from
the packaged crates alone, proving it does not require the compiler.

See [nx-ir-runtime/README.md](nx-ir-runtime/README.md),
[../docs/nx-ir-format.md](../docs/nx-ir-format.md) and [../docs/deployment.md](../docs/deployment.md).
