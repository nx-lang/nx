# @nx-lang/sdk-node

Node-only native SDK access for NX host, compiler, diagnostics, reusable program artifacts, NX IR
generation, and `root()` evaluation. This package is backed by napi-rs / N-API and delegates to the
same Rust `nx-api` host model used by the CLI and .NET SDK.

This package is separate from `@nx-lang/ir-runtime` under `runtime/typescript`. Use
`@nx-lang/sdk-node` when Node needs to build, validate, generate IR from, or evaluate NX source.
Use the pure TypeScript IR runtime when JavaScript only needs to execute an already persisted NX IR
JSON document.

## Support Posture

- Node only; browser and WASM hosts are not supported by this package.
- Initial local source builds require Node 22 or newer. Release validation should cover supported
  Node LTS majors, starting with Node 22 and Node 24.
- Native source builds currently use the host platform's Rust toolchain and produce one local
  `.node` addon.
- Future publishing should use napi-rs prebuilds for `linux-x64`, `osx-arm64`, and `win-x64` first,
  matching the initial .NET SDK native runtime identifiers.

## Local Source Consumption

This package is a member of the repository's root pnpm workspace, alongside
`@nx-lang/language-protocol`, which it depends on. Install once at the repository root (pnpm is
provided through corepack):

```bash
corepack enable
pnpm install
```

Build the native addon and TypeScript wrapper:

```bash
pnpm run --dir bindings/node build
```

Run the Node tests:

```bash
pnpm run --dir bindings/node test
```

For native-only rebuilds:

```bash
pnpm run --dir bindings/node build:native
```

The native build script compiles `nx-sdk-node-native` through Cargo and copies the resulting dynamic
library to `bindings/node/native/nx_sdk_node.node`, where the ESM wrapper loads it.

## Importing

```ts
import {
  NxEvaluationError,
  NxLibraryRegistry,
  NxProgramArtifact,
  NxWorkspace,
  evaluateJsonFromSource,
  generateNxIrFromSource
} from "@nx-lang/sdk-node";
```

## Workspaces

```ts
const registry = new NxLibraryRegistry();
const buildContext = registry.createBuildContext();
const workspace = new NxWorkspace([
  {
    identity: "app/main.nx",
    source: `import { answer } from "../shared/value.nx"
let root(): int = { answer() }`
  },
  {
    identity: "shared/value.nx",
    source: "export let answer(): int = { 42 }"
  }
]);

const diagnostics = workspace.validate(buildContext);
```

Workspace module identities use NX logical path normalization. Duplicate normalized identities such
as `lib/config.nx` and `lib/./config.nx` are rejected with structured diagnostics.

## Language Snapshots

`NxLanguageSnapshot` answers editor queries — hover, completions, diagnostics, and document
symbols — over a set of in-memory documents addressed by logical URI. Results are the
`@nx-lang/language-protocol` shapes, re-exported from this package; positions count UTF-16 code
units, the way JavaScript strings do.

```ts
import { NxLanguageSnapshot } from "@nx-lang/sdk-node";

const snapshot = new NxLanguageSnapshot([
  {
    uri: "nx://tenant/form.nx",
    version: 1,
    source: `type Mode = light | dark
let <Panel mode:Mode title:string /> = <div />
<Panel mode=light title="Hello" />
`
  }
]);

try {
  const hover = snapshot.hover("nx://tenant/form.nx", { line: 2, character: 3 });
  console.log(hover?.contents); // ```nx\nlet <Panel mode:Mode title:string />\n```
  console.log(hover?.range);    // { start: { line: 2, character: 1 }, end: { line: 2, character: 6 }, startByte: 73, endByte: 78 }

  // Inside `mode=`: the property under the cursor is offered; `title` is already supplied.
  const completions = snapshot.completions("nx://tenant/form.nx", { line: 2, character: 7 });
  console.log(completions.items.map((item) => item.label)); // [ "mode" ]

  const report = snapshot.diagnostics();
  console.log(report.documents[0].diagnostics.length); // 0
} finally {
  snapshot.dispose();
}
```

A snapshot is immutable: build a new one when a document changes. Analysis runs on the first query
and is cached for the snapshot's lifetime, so several queries against unchanged text cost one
analysis.

**Build contexts.** Libraries loaded through an `NxLibraryRegistry` are visible to a snapshot only
when it is constructed with a build context from that registry:

```ts
const registry = new NxLibraryRegistry();
registry.loadFromDirectory("/srv/nx/ui");
const buildContext = registry.createBuildContext();

const snapshot = new NxLanguageSnapshot(documents, { buildContext });
```

Without one, a name that only a library declares is unresolved — hover says nothing about it and
diagnostics report the import as missing, exactly as the compiler would without that context.

## Program Artifacts

```ts
const artifact = NxProgramArtifact.buildWorkspace(workspace, {
  buildContext,
  entryIdentity: "app/main.nx"
});

const ir = artifact.generateNxIr();
const fingerprint = ir.metadata.programFingerprint; // decimal string, safe for cache comparisons
const jsonValue = artifact.evaluateJson();
const messagePackBytes = artifact.evaluateBytes();
const jsonBytes = artifact.evaluateBytes({ outputFormat: "json" });
```

Source convenience APIs build and dispose a short-lived artifact for simple workflows:

```ts
const value = evaluateJsonFromSource("let root() = { 42 }");
const generated = generateNxIrFromSource("let root() = { 42 }");
```

Only `root()` evaluation is exposed initially. Named entrypoint requests throw an
`NxEvaluationError` with an `unsupported-entrypoint` diagnostic rather than performing
JavaScript-side declaration lookup.

## Diagnostics and Errors

Validation returns `NxDiagnostic[]` as data. Build, IR generation, and evaluation failures throw
`NxEvaluationError` and preserve the same diagnostics array:

```ts
try {
  evaluateJsonFromSource("let root(): int = { \"oops\" }");
} catch (error) {
  if (error instanceof NxEvaluationError) {
    console.log(error.diagnostics);
  }
}
```

Native addon load or ABI problems throw `NxNativeError` with local build guidance. Operations on a
disposed resource throw `NxDisposedResourceError`.

## Resource Lifecycle

Long-lived Node services should dispose native resources explicitly:

```ts
const registry = new NxLibraryRegistry();
const buildContext = registry.createBuildContext();
const artifact = NxProgramArtifact.buildSource("let root() = { 42 }", { buildContext });

try {
  console.log(artifact.evaluateJson());
} finally {
  artifact.dispose();
  buildContext.dispose();
  registry.dispose();
}
```

The native addon uses JavaScript object lifetime as a backstop, but explicit disposal is the
supported lifecycle for server-side reuse. Program artifacts remain usable after the build context
used to create them has been disposed.

## Future Distribution

The package metadata already declares napi-rs package naming and platform triples. Published npm
distribution should add Node 22+ prebuilt native artifacts produced from the same NX source
revision as the TypeScript wrapper, validate active LTS Node majors, and document how each platform
package maps to the local source-built workflow.
