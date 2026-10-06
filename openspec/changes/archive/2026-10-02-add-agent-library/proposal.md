## Why

NX is about to carry types that more than one host needs and that no host should own: the
declaration of an AI agent, its reference documents and its tools. ReachMe is the first consumer. Its
own libraries must import these types and extend them (`RecordSearchTool extends Tool`,
`ChatToolContext extends ToolContext`, a flow step with `agent?: Agent`), and the NX host package of
`add-agent-host-package` must read values of them. NX has no way to ship such a library today. The
only NX source the compiler carries is the prelude, which holds one type, is in scope everywhere and
cannot be imported or left out. A host library, on the other hand, is supplied by the host under a
root the host picks, so two hosts would name the same agent types differently and every tool (the
CLI, the language server, `typegen`, the playground) would have to be handed the sources separately.

This change adds the missing middle: a **standard library**, written in NX, carried by the compiler
like the prelude, but imported by name like a library. It then adds the first one, `@nx/agent`.

## What Changes

**Standard libraries (the mechanism).**

- A standard library is NX source carried inside the compiler and analyzed once per process, as the
  prelude is. Unlike the prelude it is not in scope by default: a module imports it with
  `import "@nx/<name>"`, in any import form, or a host lists `@nx/<name>` as an implicit import.
- `@nx/<name>` is a new kind of import path. It is not relative to the importing module, resolves the
  same way from a single source, a workspace module at any depth, a directory library and an
  in-memory library, and never reads a file.
- Every `LibraryRegistry` and every `ProgramBuildContext` sees the standard libraries with nothing
  loaded by the host. A host does not load a standard library and cannot: the root is reserved. A library that imports one records it as an ordinary dependency.
- The whole `@nx/` root is reserved. A workspace module or a host-loaded library under it is refused.
  Today only the identity `@nx/prelude.nx` is refused.
- In NX IR a standard library module is an ordinary linked module. The compiler emits its image on
  request, as it does for any library module. No runtime carries a standard library's image, so the
  IR runtimes do not change.
- Its module-table version is derived from the library's source, so any edit produces a new version
  and an image never links against a different revision by accident.
- Each standard library has a stability, `unstable` or `stable`. An unstable library may change in
  any NX release, patch releases included, with no deprecation period.
- `nxlang typegen` accepts `@nx/<name>` as its input, and generated TypeScript that references a
  standard library type imports it from that library's published package instead of from a package
  name guessed from a directory.
- The language service resolves, completes and hovers standard library declarations once a document
  imports the library, with no build context, and labels them as standard library declarations.

**The `agent` library (`@nx/agent`, unstable).**

- One module, `@nx/agent/agent.nx`, exporting thirteen types: `Agent`, `Document`, `AgentLimits`,
  `Tool` (abstract), `FunctionTool`, `WebSearchTool`, `ToolContext` (abstract), `Connection`
  (abstract), `HttpConnection`, `HttpMethod`, `HttpParam`, `HttpArguments`, `HttpTool`. Every type
  and field carries a `///` doc comment.
- `Agent.instructions` and `Document.text` are content properties, so both are written as an element
  body and take typed text (`<Agent:markdown ...>`).
- `Agent.model` is a string NX does not interpret. `HttpConnection` has no `auth`; the name is
  reserved.
- `FunctionTool.function` is typed `<function ... />: object*`, a function of any parameters and
  any result, and `HttpTool.arguments` is typed `<function ... />: HttpArguments`, a function of
  any parameters that returns `HttpArguments`. Both come from
  `add-function-reference-type`, so NX checks that an arguments function returns `HttpArguments`.
- `HttpArguments.body` is typed `object`, the top type NX already has, so no generic record and no
  new type is needed.
- The library holds types only: no functions, no components, no evaluation behavior. What a tool
  does is the host's business.

Nothing is removed and no existing program changes meaning. The one **BREAKING** edge: an import
path whose first segment is `@nx` used to be read as a relative directory and now names a standard
library, and a host library root or workspace identity under `@nx/` is now refused.

**Dependencies on the other changes in this set.** This change depends on
`add-function-reference-type`, which has landed, for the function reference type, `<function ... />: R`. `add-agent-host-package` depends on this
change for the types it reads, and its package `@nx-lang/agent` exports the TypeScript types that
`nxlang typegen @nx/agent` generates, which is where a host library's generated code imports them
from.
`add-declaration-schema-export` exports the doc comments written here as tool descriptions.
`add-ir-runtime-evaluation-budget` is independent. In ReachMe, `add-agent-tool-loop` adopts these
types, bumps the NX pin, lists `@nx/agent` as an implicit import, writes `import "@nx/agent"` in its
own library modules, stores the `@nx/agent/agent.nx` image in its runtime bundle and adds
`function-reference-type-v1` to the features its flow runtime supports, and `add-agent-function-tools`, `add-agent-http-tools` and
`add-question-flow-agent-steps` build on that.

## Capabilities

### New Capabilities

- `standard-libraries`: NX libraries carried by the compiler and imported as `@nx/<name>`: where they
  are available, the reserved root, how they are versioned in NX IR, how their images are emitted,
  and what `unstable` means.
- `agent-library`: the declarations of `@nx/agent`, their shapes and the constructs they rely on.

### Modified Capabilities

- `module-imports`: adds the `@nx/<name>` standard library import path and its resolution rule.
- `library-registry`: adds that every registry resolves standard libraries without a load and
  refuses a host library under the reserved root.
- `cli-code-generation`: adds `typegen` from a standard library and the fixed package target for
  references to a standard library's types.
- `editor-language-service`: adds resolution, completion and hover for standard library
  declarations.
- `sdk-wasm`: adds that builds, validation, implicit imports and IR emission reach standard
  libraries with no registry, at parity with the Node SDK.

## Impact

- **`crates/nx-hir`**: the reserved-root constant is renamed from prelude-specific to the `@nx/`
  root; the FNV-1a hasher NX writes into artifacts moves here so the module fingerprint and a
  standard library's version share it.
- **`crates/nx-api`**: `agent.nx` embedded beside `prelude.nx`; a process-wide table of standard
  libraries built through `build_library_artifact_from_sources`; import normalization that leaves an
  `@nx/` path alone; registry and build-context lookups that answer for standard roots; the reserved
  root checks in `source_graph.rs` and in in-memory library preparation; the implicit-import check.
- **`crates/nx-codegen`**: `module_version` answers the content-derived version for a standard
  library module. This change adds no node kind, feature or schema version of its own. The
  `@nx/agent/agent.nx` image lists `function-reference-type-v1`, the required feature
  `add-function-reference-type` adds, because two of its fields are typed by a function reference type.
- **`crates/nx-cli`**: `typegen` input and the TypeScript and C# dependency targets.
- **`crates/nx-language-service`**, **`crates/nx-lsp`**: indexing of standard libraries a document
  imports, and the hover label.
- **`bindings/wasm`**, **`bindings/node`**, **`bindings/dotnet`**: no new API and no ABI change. They
  gain tests, and the READMEs gain a section.
- **Runtimes** (`runtime/typescript`, `crates/nx-ir-runtime`): unchanged. A test links and evaluates
  an image that uses `@nx/agent`.
- **Docs**: `nx-grammar.md` (import path note), the modules reference and tour pages, a new website
  page for the agent library, `docs/nx-ir-format.md`.
- **Release**: the library ships inside the compiler in every package that carries it
  (`@nx-lang/sdk-wasm`, `@nx-lang/sdk-node`, the CLI, the VS Code extension, the .NET SDK). No new
  package is published by this change.
- **Other active changes**: `add-go-to-definition` excludes locations in the prelude and in
  libraries, which covers standard libraries without edits. `retire-hir-interpreter` removes the
  interpreter that today rejects a subtype declared in another module at an abstract-typed sequence
  field (see design.md), so the agent library's evaluation tests run on the IR runtimes.
