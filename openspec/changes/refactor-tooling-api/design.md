## Context

See proposal.md for motivation. Today `nx-api` owns both `ProgramArtifact` construction and
interpreter-backed evaluation. `nx-codegen` consumes `nx_api::ProgramArtifact` and imports resolved
types from `nx-interpreter`; `nx-language-service` imports workspace, analysis, diagnostic and
visualizer facilities from `nx-api`. Node and WASM bindings consequently compose several Rust
crates themselves. `nx-ffi` and the CLI also call both `nx-api` and `nx-codegen`.

`nx-ir-runtime` is already independent of the compiler. That property must survive this refactor.
The existing retirement proposal is unimplemented and is not a dependency of this change.

## Goals / Non-Goals

**Goals:** Establish an acyclic foundation/service/facade structure; preserve Rust type identity
and existing public import paths; let external adapters reach existing capabilities through
`nx-api`; make later execution-engine changes independent of program ownership.

**Non-Goals:** Change an evaluator, remove APIs, change serialization or arithmetic, implement CLI
IR execution, introduce source-evaluation conveniences beyond those already present, add runtime
caches, change publishing policy, or redesign editor and compiler APIs. Command-specific type
generation and documentation logic need not move into libraries in this change.

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
resolved types. `nx-interpreter` depends on `nx-program` and re-exports its formerly owned public
types. `nx-api` re-exports every existing public foundation symbol at its existing path. Do not
copy types into two crates or convert between parallel representations.

Move supporting embedded sources/resources with their loaders or update their include paths;
preserve the source bytes, library identities and computed fingerprints. Audit all `crate::` and
`pub(crate)` references at the boundary. Prefer narrow accessors or shared functions in the
foundation over exposing previously private storage wholesale. This includes source maps, version
maps and diagnostic line-index utilities used by retained API code and emitters.

`nx-program` may depend on syntax, HIR, types, diagnostics and canonical value crates; it must not
depend on the interpreter, code generator, language service, IR runtime or external facade.
Schema and visualizer inspection move with program building because lower tooling consumes them.
An alternative is leaving them in the facade and duplicating lower-level queries, which would
leave upward dependencies or introduce unnecessary duplication.

### 2. Make `nx-api` the external composition layer

The intended normal dependency graph is:

```text
CLI / bindings / LSP adapter
             |
           nx-api
        /     |      |          |                \
nx-program nx-codegen nx-language-service nx-interpreter nx-ir-runtime
    ^          |             |                 |
    +----------+-------------+-----------------+
```

The IR runtime retains its current compiler independence; there is no runtime-to-foundation edge.
Other low-level dependencies are omitted above. No lower implementation crate imports `nx-api`.

Keep the existing flat `nx-api` exports, including evaluation, components, text and value
conversion. Add namespaced re-exports of existing services, for example `nx_api::codegen`,
`nx_api::language_service`, and `nx_api::ir_runtime`. Use re-exports instead of wrappers when no
translation is required. Preserve existing function options and result types rather than designing
a common abstraction over dissimilar services. Continue using the interpreter for every operation
that currently uses it; merely exposing the IR runtime does not switch those operations.

An alternative is a new facade crate above the current `nx-api`. That avoids renaming the current
foundation but leaves its execution responsibilities mixed with it and introduces a second external
entry point. Extracting the foundation makes the existing name reflect its intended role.

The facade aggregates dependencies by default for this change. Consumers needing a lightweight
runtime or compiler subset can still use the underlying crates directly. Feature-gating the facade
is deferred unless an existing build target requires it to preserve buildability; any such gating
must keep existing default exports available and be recorded in implementation notes.

### 3. Migrate adapters without demanding one-crate purity

Route Node, WASM, C/.NET FFI and CLI calls to existing codegen/editor capabilities through the
facade namespaces. Route the LSP adapter's existing language-service access through the facade.
Remove redundant direct service dependencies once imports are migrated. Keep adapter-specific
transport, ownership, error translation and protocol handling where they are today.

Low-level imports needed to implement command-specific tooling can remain. The objective is a
usable external entry point, not wrapping every syntax node or relocating every CLI command.
External bindings must keep their names, ABI, exceptions, disposal behavior, payloads and defaults.

### 4. Keep tests at the layer they exercise

Pure program-building and inspection tests move with the implementation. Tests involving source
execution or code generation move to `nx-api` integration tests if their present placement would
introduce an upward dependency. Inventory tests that currently mix building and execution inside
`artifacts.rs`, as well as `nx-codegen` parity tests that call native evaluation.

Do not add a dev-dependency on the facade to a lower crate to recover those tests. Shared fixture
data can stay in a neutral location; test helpers must not become a second public API.
Interpreter unit/integration tests remain and use re-exported resolved types as needed.

### 5. Validate structural and behavioral preservation separately

Add a focused dependency-boundary check using Cargo metadata that runs with normal repository
checks. Verify normal dependencies and inspect dev-dependencies for upward facade edges; avoid
treating platform-disabled dependencies as absent. Assert the foundation exclusions, the lack of
facade dependencies in lower services, and the IR runtime's compiler independence. Compile a small
external-style test against the facade namespaces and legacy exports, including use of a resolved
type through both its old interpreter path and new foundation path.

Capture the existing suite baseline before moving code. Existing semantic assertions, emitted
artifact goldens, snapshots and diagnostics must remain unchanged. Explain any test-count movement
with a relocation inventory; do not update behavior expectations to make the refactor pass.
Run representative source-to-IR emission, legacy evaluation and editor operations through the
facade. Check native and WebAssembly builds because the facade adds dependency reachability.

## Risks / Trade-offs

- [A lower-layer test recreates an upward dependency] → Move cross-service tests into the facade
  integration layer and check the dependency graph including test-only edges.
- [Moving embedded source or resolved types changes fingerprints or snapshot bytes] → Keep exact
  source data, type fields and serde representations; compare existing artifacts and lifecycle
  fixtures before/after and retain compatibility regression coverage.
- [The facade increases compile time or WASM output size] → Record current sizes/build baselines,
  inspect adapter builds after aggregation, and avoid pulling unrelated optional tooling into
  targets that cannot build it. A runtime-only consumer can continue to use `nx-ir-runtime`.
- [A broad extraction hides behavior changes] → Keep commits mechanical, preserve assertions and
  review code moves separately from the namespace/adaptor changes.
- [Shared private helpers become accidental API] → Add only the accessors required by the split;
  document their ownership and avoid forwarding implementation-only helpers from `nx-api`.

## Migration Plan

1. Inventory public exports, dependencies, fixtures and target builds; record the passing baseline.
2. Extract the foundation and resolved types, retain old re-exports, and retarget the interpreter.
3. Retarget codegen and language service; relocate cross-service tests before adding facade edges.
4. Add facade namespaces, migrate adapters, and remove redundant direct dependencies.
5. Run structural checks and all affected suites; update crate and binding architecture docs.

Each phase should build before the next begins. No persisted-data migration is required. Rollback
reverts the refactor commits; there is no runtime cutover to undo. After this lands, rebase and
rescope `retire-hir-interpreter` as a separate change; leave its current artifacts untouched here.
