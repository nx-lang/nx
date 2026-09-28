## Context

`LibraryRegistry` (`crates/nx-api/src/artifacts.rs`) keys loaded libraries by canonicalized
`PathBuf` and loads them by walking a directory; `ProgramBuildContext` carries `visible_roots` of
the same type and already has an `implicit_imports` list, which today names workspace module
identities only. Analysis of a library's sources already goes through
`build_library_artifact_from_sources`, so the filesystem is involved only in discovering files and
in keying roots.

The wasm SDK's native crate (`bindings/wasm/native/src/lib.rs`) constructs an empty
`LibraryRegistry` per call and exposes no registry to JavaScript. Its README scopes "library
registries loaded from disk" to the Node SDK, which is `private` and unpublished because it needs a
native prebuild matrix.

The consumer driving this is ReachMe's API. It compiles each tenant's `chat-link.nx` against two
built-in libraries it owns (`chat-link`: one module importing `../question-flow`; `question-flow`:
29 modules), validates first to report diagnostics to the author, then builds, emits IR, and
persists the images for a Cloudflare Durable Object that evaluates them with `@nx-lang/ir-runtime`.
It will run the wasm SDK in a Node worker-thread pool, one host per thread, compiling many tenants
per host.

## Goals / Non-Goals

**Goals:**
- A library can be loaded from memory with semantics indistinguishable from a directory load.
- Libraries are analyzed once per host and reused across builds.
- Tenant source needs no import line for host-provided libraries.
- Library modules have stable, tenant-independent IR images a runtime can prepare once.
- Validation with warnings is available as data in the wasm SDK.
- Node and wasm SDKs stay at parity.

**Non-Goals:**
- Publishing `@nx-lang/sdk-node` or building its prebuild matrix.
- JSON evaluation in the wasm SDK. Hosts evaluate emitted IR with `@nx-lang/ir-runtime`, which
  keeps one evaluator for build-time and run-time values.
- Remote (Git/HTTP) library resolution.
- Any language or IR format change.

## Decisions

### D1. Libraries are keyed by a logical root, and directory loads derive one
The registry's key is the library root. An in-memory load supplies it as a normalized logical
identity (always relative, such as `libraries/question-flow`); a directory load keys by the
canonical path as before, so existing behavior and diagnostics are unchanged. The key stays a
`PathBuf` rather than a new type: the two spaces cannot collide because a canonical path is always
absolute, and every consumer that reads `root_path` (the resolved program, codegen provenance,
diagnostics) already handles a relative root, as the prelude's reserved root shows. A module of an
in-memory library is named `<root>/<module>`. Relative library imports from such a module resolve
lexically by joining against its logical location and normalizing with the workspace identity rules
(including the root-escape check), then looking up a loaded root; a module read from disk still
resolves through the filesystem.

Because `<root>/<module>` is also a valid workspace identity, a logical root reserves its whole
namespace: a workspace module whose identity equals a library module's or lies under the root of a
library the build context can see or the program links fails validation and the build with
`workspace-module-in-library-root`. Otherwise the program would hold two modules under one identity,
and untrusted workspace text could stand in for a shared library's module inside that program.
Reserving the prefix rather than only the library's current module identities also rules out a
collision when a library later gains modules. Directory roots are absolute, so they reserve nothing a
workspace can name.

*Alternative*: a virtual filesystem under the existing path-keyed registry. Rejected: it keeps the
filesystem as the model and leaks host path spellings (`/virtual/...`) into diagnostics and IR
identities.

### D2. Loading order is dependency-first, with a batch convenience
A single load resolves its library imports against roots already loaded, so a dependency must be
loaded first; a missing one is a diagnostic, not a deferred resolution. The SDKs also accept a
list of libraries in one call and load it in dependency order, so hosts need not sort. Reloading a
root with identical content and version is a no-op; different content is refused (roots are
immutable for a registry's life, which is what makes snapshot reuse sound).

### D3. Implicit imports resolve library roots after workspace modules
The implicit-import list is resolved once per build: each identity is looked up among workspace
modules and loaded library roots; a workspace module whose identity is also exactly a visible
library's root is an ambiguity diagnostic; a miss in both keeps the existing unknown-identity
diagnostic. Only an exact root match competes with a workspace module: the suffix matching a written
import of a library uses (a directory library's absolute path ending in the identity) does not, so a
host that implicitly imports a workspace module keeps building when a directory library with a
similar path is visible, as it did before. A resolved library root behaves as a written wildcard
import of that library, reusing the existing library import path.

### D4. Library module image identity is `<root>/<module>` with the library version
Library module images use the identity `<library root>/<library-relative module identity>`
(for example `libraries/question-flow/QuestionFlow.nx`) and the version supplied at load. Entry
images record these identities and versions in their module tables as today.

Three changes make a library module's image the same bytes whichever tenant program emitted it,
and make it safe to cache by fingerprint:

- A library's module sources join the program's source map, and its version the program's version
  map, so a library module's fingerprint hashes its identity and its actual source text (it hashed
  the identity alone before) and its module-table entry carries the library's version.
- A library module's image carries every declaration of the module, derived update records and
  property unions included, as the prelude's image already did. Selecting only the derived
  declarations the tenant program reached made the library image depend on the tenant.
- The library's own diagnostics now render against its source text, with lines and columns.

Workspace validation, and the diagnostics a failed workspace build throws, report what concerns the
submitted modules: the build request's own diagnostics, every workspace module's, and every linked
library's except an in-memory library's warnings, info and hints. Those are about text the
workspace's author cannot edit and would repeat in every tenant's result, so they are left out and
answered once instead, by the in-memory load that loaded the library
(`LibraryArtifact::api_diagnostics`, and the SDKs' `loadLibraries`/`loadLibrary` return value); an
in-memory library's errors stay, since they fail the build. A directory-loaded library keeps its
existing behavior: its load answers with no warnings, so validation (the Node SDK, the C FFI and the
language service) still reports all of its diagnostics. A failed build renders its diagnostics against
the program's full source map, library and prelude sources included, so it reports exactly what
validation of the same input reports.

These apply to directory-loaded libraries too: their images gain unreferenced derived declarations
and their fingerprints change, which no runtime checks at link time (linking compares versions).

### D5. Wasm ABI additions
New operations: create/dispose registry; load libraries (JSON descriptor in; the libraries' own
warnings out on success, diagnostics out on failure);
create/dispose build context (registry handle, visible roots, implicit imports); validate
workspace (build context handle, modules, implicit imports → diagnostics JSON). Workspace build
accepts an optional build context handle. Implicit imports a build or validation request names,
even an empty list, replace the context's; omitted, the context's apply. Handles follow the existing crashed-host invalidation.
The ABI version is bumped; the wrapper refuses a mismatched module as today.

## Risks / Trade-offs

- [Root identity change touches every registry lookup] → Directory loads derive the same keys as
  before; the existing library-registry and directory-library IR tests run unchanged as the
  regression net, plus a new in-memory-vs-directory byte-equality test.
- [Registries grow a long-lived host's memory] → Libraries are loaded once and immutable; hosts
  that need to reclaim memory replace the host, which the SDK already supports cheaply.
- [Ordering errors when a host loads libraries one at a time] → The batch load sorts; single
  loads report the missing dependency by name.

## Migration Plan

Additive for callers: existing SDK calls keep their behavior. Released in the next minor on the
unified release track; the wasm ABI bump means `@nx-lang/sdk-wasm` and its `nx.wasm` ship together
as they already do.

## Open Questions

- None blocking. Whether directory loads should also accept an explicit logical root (so a CLI and a
  server produce identical image identities for the same library) can follow separately.
