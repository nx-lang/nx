## Why

A server that compiles tenant-authored NX against a set of shared libraries cannot use
`@nx-lang/sdk-wasm` today. Libraries — multi-module directories that import one another, analyzed
once into a `LibraryRegistry` and reused across builds — exist only on the filesystem-backed path,
which `@nx-lang/sdk-node` exposes and which needs a per-platform native addon that is not published.
The wasm SDK's registry is always empty, and its implicit imports name single workspace modules, so
a library of thirty modules that imports a sibling library has no faithful in-memory spelling.

ReachMe is the first consumer blocked on this. Its API compiles each tenant's `chat-link.nx`
against two built-in libraries (`chat-link`, which imports `../question-flow`, and the 29-module
`question-flow`) and is moving from the unpublished native SDK to `@nx-lang/sdk-wasm` so it can
consume NX from npm, drop Rust from its build, and compile untrusted source inside a sandbox whose
traps do not take down the process. It also wants authors to stop writing an import line that
names a server file path. A server-side consumer also needs non-throwing validation that reports
warnings, which the wasm SDK only offers as a thrown error on failure.

## What Changes

- **Libraries load from in-memory modules.** A library registry accepts a library as a logical root
  identity, an optional version, and a set of modules (library-relative identity and source text),
  with the same analysis, dependency-closure, cycle and snapshot-reuse semantics a directory-loaded
  library has today. Imports between libraries (`import "../question-flow"` inside `chat-link`)
  resolve against the logical roots of loaded libraries rather than the filesystem.
- **The wasm SDK exposes a library registry and reusable build contexts.** A host loads its shared
  libraries once, then builds any number of workspaces against a build context from that registry
  without re-analyzing the libraries.
- **Implicit imports may name a library.** The implicit-import list accepts a loaded library's root
  identity as well as a workspace module identity, with exactly the semantics of a written wildcard
  import of that library.
- **Library modules emit NX IR.** A program built against loaded libraries can emit an image for
  each library module it links against, under a stable identity carrying the library's version, so
  a host can ship and prepare library images independently of the entry module.
- **The wasm SDK validates a workspace without building it.** Validation returns every diagnostic
  (errors, warnings, info, hints) for the submitted modules as data, without throwing, with the same
  shape the Node SDK's workspace validation returns.
- The Node SDK gains the same in-memory library loading, so both SDKs keep their parity guarantee:
  byte-identical IR and equal diagnostics for the same inputs.
- No change to directory-loaded libraries, the NX language, or the NX IR format.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `library-registry`: Registries load libraries from in-memory modules under a logical root, with
  the same semantics as directory-loaded libraries.
- `workspace-programs`: Implicit imports may name a loaded library root; workspace validation is
  available as non-throwing data through both SDKs.
- `sdk-wasm`: The wasm SDK exposes library registries, reusable build contexts, workspace
  validation, and emission of library-module IR.
- `sdk-node`: The Node SDK loads in-memory libraries with results identical to the wasm SDK.

## Impact

- `crates/nx-api`: `LibraryRegistry` keys libraries by a logical root rather than only a
  canonicalized filesystem path; `build_library_artifact_from_sources` becomes reachable from a
  public in-memory load; `ProgramBuildContext` implicit imports resolve library roots.
- `bindings/wasm` (native crate, ABI, TypeScript wrapper, README, parity tests): new registry and
  validation operations; ABI version bump.
- `bindings/node` (native and TypeScript): in-memory library load for parity.
- Released in the next minor version on the unified release track; `@nx-lang/sdk-wasm` consumers
  must update together with the module's ABI version.
