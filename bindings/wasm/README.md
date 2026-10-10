# @nx-lang/sdk-wasm

Browser and Node access to the NX compiler, NX IR generation, and the language service, through one
WebAssembly module. No server, no native addon.

This package is separate from `@nx-lang/ir-runtime` under `runtime/typescript`: use `@nx-lang/sdk-wasm`
when JavaScript needs to *compile* NX source or answer editor queries over it, and the IR runtime when
JavaScript only needs to *execute* an already persisted NX IR image. It also evaluates a program's
`root()` to NX text, the form `nxlang run` prints, for showing a value to a person. Filesystem-backed
workflows — library registries loaded from disk, workspace directories, `root()` evaluation to JSON
values — belong to [`@nx-lang/sdk-node`](../node/README.md); this package sees only the source text
it is handed, including the sources of the libraries a host loads into a registry.

## Scope

| Provided                                                   | Not provided                                 |
| ---------------------------------------------------------- | -------------------------------------------- |
| Build a program artifact from in-memory modules            | Libraries and workspaces read from directories |
| Library registries loaded from in-memory modules           | Evaluating NX to JSON values (use the IR runtime) |
| Validate a workspace, answering diagnostics as data        |                                              |
| Emit one NX IR artifact per module, library modules too    |                                              |
| Evaluate `root` to annotated NX text                       |                                              |
| JSON Schema and documentation for a function or a type     | Deriving schemas from an NX IR image         |
| Hover, completions, diagnostics, document symbols          | Threads, streaming, incremental analysis     |
| An in-process `NxLanguageService` over host documents      |                                              |

Everything here answers over in-memory documents. Nothing needs a file to exist.

## Toolchain

The module targets `wasm32-wasip1`, whose standard library bundles wasi-libc, so tree-sitter's C
runtime and the NX grammar link with no shims. Building it needs:

- The `wasm32-wasip1` rustup target — listed in `rust-toolchain.toml`, so `rustup` installs it with
  the toolchain.
- `clang`, to compile the C sources for that target.
- The pinned wasi-sdk `wasi-sysroot`, which `scripts/fetch-wasi-sysroot.mjs` downloads into a
  gitignored `.cache/` under the repository, verifies by SHA-256, and reuses afterwards.

`pnpm --filter @nx-lang/sdk-wasm build` runs all of that: the fetch, cargo for the wasm target under
the `wasm-release` profile, and a copy of the module to `dist/nx.wasm`. There is no emscripten, no
wasm-bindgen and no wasm-pack; the ABI is JSON in and JSON out over a hand-written loader, except
the NX IR calls, which carry an image's bytes (see *ABI* below).

```bash
pnpm install
pnpm --filter @nx-lang/sdk-wasm build
pnpm --filter @nx-lang/sdk-wasm test
```

## Loading: compile once, host many

The two halves of loading are separate on purpose. `compileNxModule` produces a `WebAssembly.Module`
— the expensive, cacheable half — and `createNxHost` instantiates it, which takes tens of
milliseconds. A caller keeps the module and creates hosts from it as often as it needs to.

```ts
import { compileNxModule, createNxHost } from "@nx-lang/sdk-wasm";

// In a browser: stream the module straight from the network.
const module = await compileNxModule(fetch(nxModuleUrl));
const host = createNxHost(module);

const artifact = host.buildProgramArtifact(source, { fileName: "input.nx" });
try {
  const [{ bytes, metadata }] = artifact.generateNxIr();
  console.log(metadata.identity, metadata.fingerprint, bytes.byteLength);
} finally {
  artifact.dispose();
}
```

`buildProgramArtifact` is the one-module case of a workspace build; see *Workspace builds* below.
A build that reports an error throws `NxEvaluationError`. One that succeeds can still have
warnings, such as a doc link that names nothing; `artifact.diagnostics()` returns them, with info and
hints, in the same shape.

`compileNxModule` also accepts the module's bytes or an already-compiled `WebAssembly.Module`, so a
caller that has one already can pass it through without a special case.

Two entry points are selected by the package's `exports` conditions, and both expose the same
`createNxHost`:

- the default entry supplies WASI through `@bjorn3/browser_wasi_shim`;
- the `node` entry supplies it through `node:wasi`, and adds `loadNxModule()` for callers reading the
  module from the filesystem. Node marks `node:wasi` experimental and warns on import; the package's
  own tests and the playground's example check run with `--no-warnings`.

## Language snapshots

```ts
const uri = "nx://tenant/form.nx";
const snapshot = host.createLanguageSnapshot([{ uri, version: 1, source }]);
try {
  const hover = snapshot.hover(uri, { line: 2, character: 3 });
  const completions = snapshot.completions(uri, { line: 2, character: 7 });
  const report = snapshot.diagnostics();
  const symbols = snapshot.documentSymbols(uri);
  const tree = snapshot.sourceTree(uri); // unstable
} finally {
  snapshot.dispose();
}
```

A snapshot is immutable: build a new one when a document changes. Analysis runs on the first query
and is cached for the snapshot's lifetime, so several queries against unchanged text cost one
analysis. Positions count UTF-16 code units, the way JavaScript strings and browser editors count,
and results are the `@nx-lang/language-protocol` shapes, re-exported here.

`sourceTree(uri)` answers every piece of a document as a typed node, with the declarations the nodes
refer to; the protocol's README describes the answer. It is **unstable**: its shape may change in any
release until a later one commits to it.

`createLanguageService(host, options)` implements the protocol's `NxLanguageService` over these
snapshots, with a bounded cache of analyses. A host whose context declarations live in a catalog
passes the catalog as a document of its own and names it as an implicit import:

```ts
const service = createLanguageService(host, {
  documents: [{ uri: "nx://host/drawnui.nx", identity: "drawnui.nx", source: catalog }],
  implicitImports: ["drawnui.nx"]
});
```

Every queried document then sees the catalog's exported declarations without an import line, and
because its text is analyzed as written, every answer is already in its own coordinates. A
diagnostic inside the catalog is reported against the catalog's URI, never the visitor's. The
answering and the cache are `@nx-lang/language-core`, the same code the HTTP handler runs.

## Workspace builds

A program can span several in-memory modules. The build takes them with an entry and, optionally,
identities every other module imports implicitly, as if it began with `import "<identity>"`:

```ts
const artifact = host.buildWorkspaceArtifact({
  modules: [
    { identity: "drawnui.nx", source: catalog, version: "9" },
    { identity: "input.nx", source }
  ],
  entry: "input.nx",
  implicitImports: ["drawnui.nx"]
});
try {
  const [snippet] = artifact.generateNxIr();
  // snippet.bytes names drawnui.nx at version "9" in its module table and carries none of it.
} finally {
  artifact.dispose();
}
```

An implicit import behaves exactly as the written wildcard import would, which means it sees what
the module exports; a catalog declares its controls with `export`. A build failure is an
`NxEvaluationError` whose diagnostics each carry a label against the identity of the module they
belong to, in that module's own lines and bytes.

`generateNxIr(options)` emits one image per requested module, the entry alone by
default: `{ modules: [] }` emits every module of the program, entry first, and `{ modules:
["drawnui.nx"] }` emits the catalog on its own. Each artifact's module table records the version
each module was given in `modules[].version`, or `""` for one given none. The debug section, spans and source
text, is left out unless `{ debug: true }` is passed; with it, the artifact differs from the one
without only in that section. The metadata beside each artifact names the module, its fingerprint,
the schema and ABI, the required features, and the module's function and component entrypoints by
name.

## Libraries

A server that compiles many tenants' NX against the same libraries loads them once per host into a
registry, then builds and validates every tenant's workspace against a build context from it. A
library is a logical root, an optional version, and its modules named relative to the root; one
library imports another by a relative path to its root, exactly as a directory library would:

```ts
const registry = host.createLibraryRegistry();
const libraryWarnings = registry.loadLibraries([
  { root: "libraries/chat-link", modules: [{ identity: "ChatLinkConfig.nx", source: chatLinkSource }] },
  { root: "libraries/question-flow", version: "3", modules: questionFlowModules }
]);
const context = registry.createBuildContext({
  implicitImports: ["libraries/chat-link", "libraries/question-flow"]
});

const diagnostics = host.validateWorkspace({ modules: tenantModules, buildContext: context });
const artifact = host.buildWorkspaceArtifact({
  modules: tenantModules,
  entry: "chat-link.nx",
  buildContext: context
});
```

`loadLibraries` loads a list in dependency order, whatever order it is given in; `loadLibrary`
loads one whose dependencies are already loaded. A library whose analysis reports errors, or that
imports a root nothing loaded, is not retained, and the call throws `NxEvaluationError` with the
diagnostics. A library that loads answers with its own warnings, info and hints, rendered against
its source; this is the one place a host sees them, since validation and builds of a workspace leave
them out. A root is immutable for the registry's life: loading it again with the same modules and
version does nothing, and loading it with different ones throws naming the root. A build context
holds the libraries it sees, so it stays usable after its registry is disposed; one created with
`visibleRoots` sees only those libraries and what they depend on, and a visible root that is not a
loaded library throws naming it.

An implicit import may name a loaded library's root as well as a workspace module, with the meaning
of a written wildcard import of the library, so tenant source uses the libraries' declarations with
no import line. An identity that is both a workspace module and exactly a library's root is refused
as ambiguous. `implicitImports` given to a build or validation replaces the context's own, so
`implicitImports: []` builds without them.

A library's root names its modules, so it reserves its whole namespace: a workspace module whose
identity lies under a visible or linked library's root, such as `libraries/question-flow/Step.nx`,
fails validation and the build with `workspace-module-in-library-root` rather than standing in for
the library's module.

`validateWorkspace` analyzes a workspace without building it and answers with every diagnostic of
the submitted modules — errors, warnings, info and hints — as data, never throwing for NX
diagnostics. An in-memory library's own warnings are not among them; its errors, which fail a
build, are. It is
what an editor or an authoring API reports to the person writing the source; a build then throws
only for what validation already said, with the same diagnostics.

Every module of a library a program links is a module of the program: `generateNxIr({ modules:
[] })` emits an image for each, named `<root>/<module>` (for example
`libraries/question-flow/QuestionFlow.nx`) and recording the library's version. A library module's
image is the same bytes whichever tenant's program emitted it — it carries every declaration a using
image could reach, derived ones included — so a runtime prepares it once with `@nx-lang/ir-runtime`
and links every tenant's entry image against it:

```ts
const prepared = new Map(libraryImages.map((image) => [image.identity, prepareNxIrModule(image.bytes)]));
const program = linkNxIrProgram(prepareNxIrModule(entry.bytes), {
  resolve: (identity) => prepared.get(identity)
});
```

## Standard libraries

A standard library is NX source the module itself carries, imported by a reserved name:
`import "@nx/agent"`. Nothing is loaded for it. Every entry point that analyzes or builds NX resolves
it with no registry, build context or option, and a `loadLibrary` whose root lies under `@nx/` is
refused with `library-root-reserved`. A path under `@nx/` that names no standard library is
reported as `unknown-standard-library`, listing those that exist.

`@nx/agent` is the first: product-neutral types for declaring an AI agent, its documents and its
tools. It is **unstable**, so its declarations may change incompatibly in any release, including a
patch release; a host gets a change only by moving its pin of the NX packages.

```ts
const registry = host.createLibraryRegistry();
// A host library that names an agent type writes the import itself: a module of an implicitly
// imported library receives no implicit imports.
registry.loadLibrary({
  root: "libraries/chat-link",
  modules: [{ identity: "ChatLink.nx", source: 'import "@nx/agent"\nexport type AssistantConfig = { agent?:Agent }' }]
});
// Naming the standard library's root as an implicit import puts it in scope for tenant source, so
// an authored config needs no import line.
const context = registry.createBuildContext({ implicitImports: ["libraries/chat-link", "@nx/agent"] });
const artifact = host.buildWorkspaceArtifact({ modules: tenantModules, entry: "main.nx", buildContext: context });

// The library's image is emitted like any library module's and stored with the program's others;
// no runtime carries it.
const images = artifact.generateNxIr({ modules: [] });
const library = prepareNxIrModule(images.find((image) => image.identity === "@nx/agent/agent.nx")!.bytes);
const version = library.version; // 16 hex digits derived from the library's source
```

A program links `@nx/agent/agent.nx` only if something it uses imports the library, and the image
is the same bytes whichever program emitted it. Its version is read from that image; a host that
needs it before any tenant compiles builds the one-line source `import "@nx/agent"` and emits the
library image from it. The library image lists the required feature `function-reference-type-v1`,
so the runtime that links it must be a release that supports that feature.

## Declaration schemas

A host that hands an NX function to a system that speaks JSON Schema — a language model's tool
interface, an MCP client — asks the artifact for it while it compiles. `functionSchema` answers with
JSON Schema (draft 2020-12) for the function's arguments and for its result, its `///` documentation,
and one entry per parameter:

```ts
const artifact = host.buildWorkspaceArtifact({
  modules: [{ identity: "tools.nx", source }],
  entry: "tools.nx"
});
try {
  const tool = artifact.functionSchema({ name: "findPlans" });
  tool.description;  // "Finds the plans that fit a team.\n\nPlans are sorted by price."
  tool.summary;      // "Finds the plans that fit a team."
  tool.inputSchema;  // { $schema, type: "object", properties: { teamSize: { type: "integer", … } }, … }
  tool.outputSchema; // { $schema, type: "array", items: { $ref: "#/$defs/Plan" }, $defs: { Plan: … } }
  tool.parameters;   // [{ name: "teamSize", type: "int", required: true, description: "…" }, …]

  const images = artifact.generateNxIr(); // the image a runtime calls the function from
  store(tool, images);
} finally {
  artifact.dispose();
}
```

with `tools.nx`:

```nx
/// A plan a team can buy.
type Plan = {
  name:string   /// The plan's display name.
}

/// Finds the plans that fit a team.
///
/// Plans are sorted by price.
let findPlans(
  teamSize:int,   /// Number of people who need a seat.
  maxMonthlyPrice?:int
): Plan* = { <Plan name="Team" /> }
```

These two queries are the supported way to read a program's types and documentation. The NX IR
image holds neither — it erases generic arguments, records no alias targets, and carries no doc
comments — so a host that only executes images, with `@nx-lang/ir-runtime`, derives the schemas
when it compiles and stores them beside the images. Doc comments and these queries first ship
together in release 0.6.0, the minimum version for both.

A function is named by the identity of its module and its name, the pair a `Function` record
carries, so a host holding one from an evaluated value asks for its schema directly. The module
defaults to the entry; a library module is named `<root>/<module>`, `@nx/agent/agent.nx` for the
agent library. Any function qualifies, exported or not. `typeSchema(reference, { direction })`
answers for a record, action, union, type alias, `<Target>.Update` or `<Target>.Property`.

| Option | Query | Meaning |
| ------ | ----- | ------- |
| `hostSuppliedTypes` | `functionSchema` | Types whose parameters the host fills in itself, such as `{ module: "@nx/agent/agent.nx", name: "ToolContext" }`. A parameter declared with one, with a record extending one, or with a type alias of either, is left out of `inputSchema` and its entry names the type as `hostSupplied`. A parameter whose type holds one without being one, under `+` or as a field of a record at any depth, stays in `inputSchema` and its entry lists the types it holds as `hostSuppliedWithin`. A type the program does not declare matches nothing. An entry's `typeRef` names the record or union the parameter is declared with, and through a type alias what the alias denotes. |
| `direction` | `typeSchema` | `output` (the default) describes a value a runtime returns: every record carries `$type`. `input` describes a value a host supplies: `$type` is asked for only where a runtime needs it to choose a shape, at an abstract record or a payload union case. |

A schema describes the canonical JSON encoding and nothing wider: a value valid against
`inputSchema` is accepted as the function's arguments by a runtime running the image from the same
artifact, and a value the function returns is valid against `outputSchema`; the package's tests
hold both against `@nx-lang/ir-runtime`. Objects are closed (`additionalProperties: false`), an
optional field or parameter is left out of `required` rather than made nullable, and `object` is
`{ "not": { "type": "null" } }`, any value but `null`. Records, unions and applied generic records are `$defs` entries the document refers to by
`$ref`, so a recursive type is described once. The mapping for every type form is on the website's
*Declaration schemas* reference page.

A type with no JSON form is reported, never approximated, and the answer is still returned: the
affected schema is absent and `diagnostics` says why, while the other side is answered.

| Code | Meaning |
| ---- | ------- |
| `schema-inexpressible-type` | A parameter, field or result has a function type, a function reference type (`FunctionTool.function`), a component type, markup, an abstract record nothing in the program extends, or a result the checker could not infer. Labeled at the member. |
| `schema-ambiguous-discriminator` | Two shapes at one abstract-record site share a `$type`, which a runtime refuses as ambiguous. |
| `schema-unknown-declaration` | The reference names nothing in the program. Thrown as `NxEvaluationError`, not answered. |

## Evaluating root to NX text

`artifact.evaluateNx()` runs the entry module's `root` and returns `{ text, nodes }`. `text` is the
value spelled in NX, the text `nxlang run` prints for the same program, and the package's parity
tests hold the two to it. `nodes` says what each part of the text is, for a viewer such as
`@nx-lang/value-view`:

```ts
const artifact = host.buildProgramArtifact(`type User = { id:string name:string }
<User id="1" name="Ada" />`);
try {
  const { text, nodes } = artifact.evaluateNx();
  console.log(text); // <User id="1" name="Ada" />
  for (const node of nodes) {
    console.log(node.role, node.type, text.slice(node.start, node.end));
  }
  // record User <User id="1" name="Ada" />
  // property string id="1"
  // property string name="Ada"
} finally {
  artifact.dispose();
}
```

Each node has:

| Field         | Meaning                                                                          |
| ------------- | -------------------------------------------------------------------------------- |
| `start`, `end` | UTF-16 offsets into `text`, so `text.slice(start, end)` is the node                |
| `parent`      | Index of the enclosing node, absent at the top; parents come before children     |
| `role`        | `record`, `property`, `sequence`, `case`, `scalar`, `function` or `empty`         |
| `type`        | The type in NX: `User`, `string`, `Status`, `User*`; a property's declared type   |
| `name`        | A property's name, or a function value's                                          |
| `optional`    | `true` for a property declared optional, as `subtitle?:string` is                  |
| `count`       | A sequence's length                                                               |
| `declaration` | Where the entry module declares the record, component, case, property or function, if it does |

A number, string, boolean or `{}` written directly as a property's value has no node of its own; the
property's node describes it. A sequence's type is its items' common type with `*`, or `object*`.

`evaluateNx()` throws `NxEvaluationError` when there is no `root` (code `no-root`), when evaluating
it fails at run time (`runtime-error`, labeled at the expression that failed, or at the entry
module's call into the module where it failed), and when the value has no NX spelling
(`nx-text-unspellable`): one holding an action handler, or a sequence directly inside a sequence.
The artifact stays usable either way.

Calls nest at most 200 deep when the module evaluates, a fifth of the interpreter's default, because
a browser keeps the module's frames on a native stack the module cannot size: in a Chromium worker
it ran out between 325 and 517 calls. Runaway recursion therefore ends in a `runtime-error` naming
the limit rather than a trapped host.

## The image

`NxGeneratedNxIr.bytes` is the artifact as an NX IR image: a binary of 32-bit cells that
`@nx-lang/ir-runtime` reads in place, copied out of the module's memory so it outlives the artifact
and the host. It is byte for byte what the Node SDK, the .NET SDK and the CLI emit for the same
input, and no debug section unless asked for. `host.explainNxIr(bytes)` renders an image as the
text `nxlang ir explain` prints, with every table index resolved, and throws `NxEvaluationError`
for bytes that are not an image this build reads. The layout is documented in
`docs/nx-ir-format.md`.

## A trap ends the host; the caller replaces it

The module targets `wasm32-wasip1`, where a panic aborts rather than unwinds. A panic or an
out-of-bounds access inside the module therefore ends the instance and its memory — there is no
`catch_unwind` that could hide it, and the request that trapped is lost.

The SDK makes that explicit rather than papering over it:

- the call that trapped throws `NxHostCrashedError`, naming the operation;
- every later call on that host throws the same error without entering the module, and `host.crashed`
  is `true`;
- `dispose()` on resources from a crashed host is tolerated, so cleanup paths never mask the crash;
- the caller recovers by calling `createNxHost` again on the compiled module it already has. That
  costs an instantiation, not a download or a recompilation.

```ts
try {
  return compile(host, source);
} catch (error) {
  if (error instanceof NxHostCrashedError) {
    host = createNxHost(module); // same module; no fetch
  }
  throw error;
}
```

The playground does exactly this inside its worker, so a compiler crash costs one request and the
editor stays live.

## Errors

| Error                      | Thrown when                                                          |
| -------------------------- | -------------------------------------------------------------------- |
| `NxEvaluationError`        | NX reports diagnostics: source that does not compile, a `root` that is missing, fails or has no NX spelling, an unparseable snapshot URI, duplicate identities, a library that cannot be loaded, a schema query for a declaration the program does not have. Carries `diagnostics`. |
| `NxDisposedResourceError`  | An operation is attempted on a disposed artifact, snapshot, registry, build context or host. Disposing twice is allowed. |
| `NxHostCrashedError`       | The module trapped, and on every later call to that host. Carries `operation`. |
| `NxWasmError`              | The module's ABI version is not the loader's, or it answered in a shape the loader cannot read. |

The names and shapes match `@nx-lang/sdk-node`, so code can move between the two bindings.

## ABI

The module exports `nx_wasm_abi_version`, which the loader checks before any other call and refuses
when it disagrees, naming both versions. The current version is 7. Version 4 added library
registries (`nx_wasm_registry_new`, `nx_wasm_registry_load`, `nx_wasm_registry_free`), build
contexts (`nx_wasm_build_context_new`, `nx_wasm_build_context_free`) and
`nx_wasm_workspace_validate`, and gave `nx_wasm_workspace_build` a build-context handle argument
(null for none); 5 added `nx_wasm_program_diagnostics`; 6 added `nx_wasm_program_function_schema`
and `nx_wasm_program_type_schema`; 7 added `nx_wasm_snapshot_source_tree`.

| Exports | Purpose |
| ------- | ------- |
| `nx_wasm_abi_version`, `nx_wasm_alloc`, `nx_wasm_free`, `nx_wasm_result_free` | The version check and the module's memory |
| `nx_wasm_program_build`, `nx_wasm_workspace_build`, `nx_wasm_workspace_validate` | Building and validating programs |
| `nx_wasm_program_nx_ir`, `nx_wasm_program_evaluate_nx`, `nx_wasm_program_diagnostics`, `nx_wasm_program_function_schema`, `nx_wasm_program_type_schema`, `nx_wasm_program_free` | A program artifact's operations |
| `nx_wasm_registry_new`, `nx_wasm_registry_load`, `nx_wasm_registry_free`, `nx_wasm_build_context_new`, `nx_wasm_build_context_free` | Library registries and build contexts |
| `nx_wasm_snapshot_new`, `nx_wasm_snapshot_hover`, `nx_wasm_snapshot_completions`, `nx_wasm_snapshot_diagnostics`, `nx_wasm_snapshot_document_symbols`, `nx_wasm_snapshot_source_tree`, `nx_wasm_snapshot_free` | Language snapshots |
| `nx_wasm_ir_explain` | Explaining an image |

Arguments cross as UTF-8 JSON in buffers from `nx_wasm_alloc`, or as an image's bytes for
`nx_wasm_ir_explain`; every operation answers with a pointer to a `{ status, ptr, len }` record
whose payload is UTF-8 JSON, except that `nx_wasm_program_nx_ir` answers with an NX IR bundle (a
`u32` header length, a JSON header `[{ identity, metadata, offset, length }]`, padding to four
bytes, then the images), released through `nx_wasm_result_free` before the call returns. Handles
to artifacts, snapshots, registries and build contexts are opaque to the loader, and a build
context is refused by any host but the one that created it.

The Rust side is `bindings/wasm/native` (crate `nx-sdk-wasm-native`), over `nx-api`, `nx-codegen` and
`nx-language-service`. Its NX IR, diagnostic and schema payloads are the ones the Node binding serializes, and
the package's parity tests compare the two bindings' answers on every test run, so a divergence fails
the build rather than reaching a site.

## Layout

| Path            | What it holds                                                       |
| --------------- | ------------------------------------------------------------------- |
| `native/`       | The Rust crate and its ABI                                          |
| `src/`          | The loader, the two WASI entry points, the language service, errors and types |
| `scripts/`      | The wasm build and clean scripts                                    |
| `test/`         | Loader, SDK, language service, trap, entry-point, schema agreement and Node-SDK parity tests |
| `test/fixtures/schema/` | The schema corpus: one NX source per mapping, with the documents the export answers and argument cases |
| `dist/nx.wasm`  | The built module (gitignored; produced by `pnpm run build`)         |
