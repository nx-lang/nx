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

Install package dependencies:

```bash
cd bindings/node
npm install
```

Build the native addon and TypeScript wrapper:

```bash
npm run build
```

Run the Node tests:

```bash
npm test
```

For native-only rebuilds:

```bash
npm run build:native
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

## Program Artifacts

```ts
const artifact = NxProgramArtifact.buildWorkspace(workspace, {
  buildContext,
  entryIdentity: "app/main.nx"
});

const ir = artifact.generateNxIr();
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
