## Context

See proposal.md for motivation. Today `nx-api` owns both `ProgramArtifact` construction and
interpreter-backed evaluation. `nx-codegen` consumes `nx_api::ProgramArtifact` and imports
`ResolvedItemKind`, `RuntimeModuleId` and related types from `nx-interpreter`; `nx-language-service`
imports only foundation items from `nx-api` (workspace, library registry, standard libraries,
diagnostic spans and program visualizers). Node and WASM bindings compose `nx-api`, `nx-codegen`
and `nx-language-service` themselves. `nx-ffi` and the CLI call both `nx-api` and `nx-codegen`, and
`nx-graphics-skia`'s `preview` binary chains `nx-api` → `nx-codegen` → `nx-ir-runtime`, which is the
build-emit-run sequence a facade should own.

`nx-ir-runtime` is already independent of the compiler, and `pnpm run verify:crates` proves it by
building a host from the published packages alone. That property must survive this refactor.

The direction is to retire the HIR interpreter in favor of the IR runtime. Interpreter retirement
(`retire-hir-interpreter`) is planned as a separate change and is not a dependency of this one.

## Goals / Non-Goals

**Goals:** Establish an acyclic foundation/service/facade structure; make `nx-api` able to depend on
code generation and the IR runtime, so the retirement can switch evaluation inside `nx-api` alone;
take the resolved-program types out of the interpreter; let external adapters reach existing
capabilities through `nx-api`.

**Non-Goals:** Change an evaluator, remove the interpreter, change serialization or arithmetic,
implement CLI IR execution, add source-evaluation conveniences or a curated facade API, add runtime
caches, change publishing policy, or redesign editor and compiler APIs. Command-specific type
generation and documentation logic need not move into libraries in this change. Preserving Rust
import paths is not a goal (no backward compatibility is needed yet); preserving behavior is.

## Decisions

### 1. Extract a shared `nx-program` foundation

Move the implementation of these existing `nx-api` modules into the new crate:

| Module | Responsibility |
| --- | --- |
| `artifacts` | Programs, libraries, registry, build context, standard-library loading and prelude |
| `workspace` | Logical source modules, directory loading and identity rules |
| `source_graph` | Source providers and module graph construction |
| `diagnostics` | Shared binding-friendly diagnostic types and source mapping |
| `schema` | Program type/function schema queries |
| `program_visualizers` | Shared declaration and visualizer inspection |

Move `nx-interpreter/src/resolved_program.rs` into `nx-program` as the single definition of the
resolved types. `nx-interpreter` depends on `nx-program` and imports them from there; it does not
re-export them. Every consumer (`nx-codegen`, CLI, tests) changes its imports to `nx_program::` or
`nx_api::`. A single public path per type keeps the next change, which deletes the interpreter, from
having to chase leftover re-exports. Keep the type names; renaming `RuntimeModuleId` and friends is
not worth the churn here.

`artifacts.rs` (about 9k lines) is the bulk of the move. Move it whole; splitting it into smaller
modules is a separate cleanup and would make the move harder to review.

Move the embedded standard-library sources (`std/`) and `prelude.nx` with their loader so the
`include_str!` paths stay relative; preserve the source bytes, library identities and computed
fingerprints. Audit all `crate::` and `pub(crate)` references at the boundary, including
`ProgramArtifact`'s `source_map` and `version_map` fields. Prefer narrow accessors or shared
functions in the foundation over making previously private storage public.

`nx-program` may depend on syntax, HIR, types, diagnostics and canonical value crates; it must not
depend on the interpreter, code generator, language service, IR runtime or `nx-api`. Schema and
visualizer inspection move with program building because lower tooling (the language service)
consumes them. Leaving them in `nx-api` would keep the language service above the facade.

Naming: `nx-program` describes what the crate produces (a `ProgramArtifact`) and what the other
crates consume. Alternatives considered: `nx-compiler` (overstates it; lowering and checking stay in
`nx-hir`/`nx-types`, emission in `nx-codegen`) and `nx-workspace` (too narrow; libraries and schema
are not workspace concepts).

### 2. Make `nx-api` the external composition layer

The intended normal dependency graph is:

```text
CLI   bindings (Node, WASM, FFI)   graphics preview          nx-lsp
  \             |                    /                          |
   +----------- nx-api -------------+                           |
      /       |          |             \          \             |
nx-codegen  nx-language-service  nx-interpreter  nx-ir-runtime  |
      \       |          |                                      |
       +---- nx-program -+                     nx-language-service
```

No crate below `nx-api` depends on it, by normal or dev dependency. `nx-ir-runtime` has no edge to
`nx-program` or any compiler crate. `nx-codegen` keeps `nx-ir-runtime` as a dev-dependency for its
execution tests. Other low-level dependencies are omitted above.

Keep the existing flat `nx-api` exports: the foundation types (`ProgramArtifact`, `NxWorkspace`,
`NxDiagnostic`, schema and visualizer types), which every external call takes or returns, plus
evaluation, components, NX text and value conversion. Add namespaces as whole-crate re-exports:
`pub use nx_codegen as codegen;`, `pub use nx_language_service as language_service;`,
`pub use nx_ir_runtime as ir_runtime;`. This is deliberately an aggregator, not a curated facade:
wrappers would add code with no translation to do, and a curated build-emit-run API is better
designed once the retirement shows which calls the adapters share. Continue using the interpreter
for every operation that currently uses it; exposing the IR runtime does not switch those operations.

An alternative is a new facade crate above the current `nx-api`. That avoids renaming the current
foundation but leaves its execution responsibilities mixed with it and introduces a second external
entry point. Extracting the foundation makes the existing name reflect its intended role.

The facade aggregates its dependencies without feature gates. The WASM binding already builds
`nx-api`, `nx-codegen` and `nx-language-service`; the new reachable crate is `nx-ir-runtime`, whose
unused code the linker drops. Compare the built `.wasm` size before and after; add a feature gate
only if a target fails to build or the size grows materially.

### 3. Migrate adapters, but keep `nx-lsp` on the language service

Route Node, WASM, C/.NET FFI, CLI and `nx-graphics-skia` preview calls to codegen, editor and IR
runtime capabilities through `nx-api` and its namespaces, and remove direct dependencies on those
crates. Also remove direct dependencies nothing uses; for example `nx-ffi` currently declares
`nx-hir`, `nx-syntax`, `nx-types` and `nx-interpreter` without importing them. Keep
adapter-specific transport, ownership, error translation and protocol handling where they are.

`nx-lsp` keeps depending on `nx-language-service` directly. It is the protocol adapter for that one
service, and routing it through `nx-api` would compile the interpreter, code generator and IR
runtime into the language server with no benefit. Revisit if the server gains previews.

Low-level imports needed to implement command-specific CLI tooling can remain. The objective is a
usable external entry point, not wrapping every syntax node or relocating every CLI command.
External bindings keep their names, ABI, exceptions, disposal behavior, payloads and defaults.

### 4. Keep tests at the layer they exercise

Pure program-building and inspection tests move with the implementation. Tests that evaluate
source or emit code move to `nx-api` integration tests (`crates/nx-api/tests/`) when their current
placement would need an upward dependency. Known cases: tests inside `artifacts.rs` that call
`eval_*` (about 28 call sites), a few in `library_source_tests.rs`, `standard_library_tests.rs` and
`visualizer_declarations_tests.rs`, and the `nx-codegen` tests that compare against the interpreter
(`ir_corpus_tests.rs`, `ir_runtime_tests.rs`, `ir_instance_tree_tests.rs`, `math_tests.rs`,
`color_transport_tests.rs`, `tests.rs`). The parity tests are the largest group; they become
`nx-api` integration tests and go away with the interpreter.

Do not add a dev-dependency on `nx-api` to a lower crate to keep those tests in place. Shared
fixture data can stay in a neutral location; test helpers must not become a second public API.
Interpreter unit and integration tests remain in `nx-interpreter` and import resolved types from
`nx-program`.

### 5. Check the boundary with what Cargo does not already enforce

Cargo rejects cycles among normal dependencies, so once `nx-api` depends on the services and
`nx-interpreter`, `nx-codegen` and `nx-language-service` depend on `nx-program`, any normal edge
from them (or from `nx-program`) back to `nx-api`, and from `nx-program` to those three, fails the
build. `pnpm run verify:crates` already covers the IR runtime's independence. Add a small
Cargo-metadata check, run with the repository's normal checks, for the remaining edges:

- no dev-dependency from a crate below `nx-api` on `nx-api` (Cargo allows dev-dependency cycles);
- no dependency of `nx-program` on `nx-ir-runtime`, and none of `nx-ir-runtime` on `nx-program`.

Behavior: capture the existing suite results before moving code. Existing semantic assertions,
emitted artifact goldens, snapshots and diagnostics must remain unchanged. Explain test-count
movement with a relocation list; do not update behavior expectations to make the refactor pass.
Add one `nx-api` integration test that builds a program, emits IR and runs it on the IR runtime
through the facade namespaces, and evaluates the same program through the flat interpreter-backed
API.

## Risks / Trade-offs

- [A lower-layer test recreates an upward dependency] → Move cross-service tests into `nx-api`
  integration tests; the boundary check covers dev-dependencies.
- [Moving embedded sources or resolved types changes fingerprints or snapshot bytes] → Keep exact
  source data, type fields and serde representations; compare existing artifacts and lifecycle
  fixtures before and after.
- [The facade increases compile time or WASM output size] → Compare `.wasm` size before and after.
  A runtime-only consumer continues to use `nx-ir-runtime` directly.
- [A broad extraction hides behavior changes] → Keep the move commits mechanical (move files, fix
  imports) and separate from the facade and adapter commits.
- [Shared private helpers become accidental API] → Add only the accessors the split requires.
- [Whole-crate namespaces expose more than intended] → Accepted for now; `nx-api` is
  `publish = false`, and the curated API comes after the retirement.

## Migration Plan

1. Record the passing baseline and the `.wasm` size; list tests that mix building with execution.
2. Add `nx-program` with the resolved types; retarget `nx-interpreter` and fix imports.
3. Move the foundation modules; retarget `nx-codegen` and `nx-language-service`; relocate
   cross-service tests before adding the facade's service dependencies.
4. Add the facade namespaces, migrate adapters, and remove unneeded direct dependencies.
5. Add the boundary check, run all affected suites, and update the crate docs.

Each phase should build before the next begins. No persisted-data migration is required. Rollback
reverts the refactor commits; there is no runtime cutover to undo.

## Follow-up

With this in place, interpreter retirement can reimplement `nx-api`'s evaluation and component
functions on `codegen` + `ir_runtime` behind the same flat signatures, then delete `nx-interpreter`
and remove it from `nx-api`. The CLI's direct `nx_interpreter` use in `nxlang run` moves with it.
