# @nx-lang/ir-runtime

Evaluates persisted NX IR in JavaScript, in a browser or under Node, with no compiler and no NX
checkout. The IR is the schema 3 image `@nx-lang/sdk-wasm`, `@nx-lang/sdk-node`, the .NET SDK
and the `nxlang` CLI emit for one module of a compiled program: a binary a runtime reads in place.
This package links images by name and turns them into values.

Use it when JavaScript only needs to *execute* an already compiled program. When JavaScript needs
to *compile* NX source or answer editor queries over it, use `@nx-lang/sdk-wasm`, which emits the
artifacts this package runs.

## Prepare, link, evaluate

An artifact carries one module and a table naming the modules it references. A host prepares each
module once and links an entry module against the prepared modules a resolver supplies:

```ts
import { createNxHost } from "@nx-lang/sdk-wasm";
import { evaluateFunction, linkNxIrProgram, prepareNxIrModule } from "@nx-lang/ir-runtime";

// The catalog ships with the host as bytes and is prepared once per page.
const catalog = prepareNxIrModule(catalogBytes);

// Each snippet's image names the catalog and the version it was compiled against. Here the SDK
// emits it; a host that stores images hands over the bytes it stored.
const artifact = host.buildWorkspaceArtifact({
  modules: [{ identity: "input.nx", source }],
  entry: "input.nx",
  implicitImports: ["drawnui.nx"],
});
const [{ bytes }] = artifact.generateNxIr();
const snippet = prepareNxIrModule(bytes);
const program = linkNxIrProgram(snippet, {
  resolve: (identity) => (identity === catalog.identity ? catalog : undefined),
});
const root = evaluateFunction(program, "root");
```

`prepareNxIrModule` takes a `Uint8Array` or an `ArrayBuffer`. The tables are read as 32-bit cells
through typed-array views over the bytes given, so nothing is copied and no table is decoded up
front; a string is decoded the first time it is named. One exception: a `Uint8Array` whose byte
offset is not a multiple of four is copied first, since a `Uint32Array` needs alignment. The SDKs
answer with fresh buffers, so a host that passes their bytes through pays no copy.

Linking checks, before anything is evaluated, that the resolver supplies every module the entry
names, that each one's version equals the version the entry recorded, and that every declaration
the entry references exists in the resolved module. A mismatch is an `NxIrRuntimeError` naming the
module and, for a version, both versions. A host that wants a snippet compiled against catalog
version 9 to run on catalog 10 passes `allowVersionMismatch: true`; the declaration check still
applies, so a control the newer catalog dropped is reported by name rather than failing midway
through drawing. Linking never copies a prepared module, so one prepared catalog serves any number
of programs.

An artifact whose module table names only itself is a program on its own: `prepareNxIrProgram`
prepares and links it in one call, and a prepared module of that shape can be passed to the
evaluation APIs directly. A prepared module that names other modules must be linked first, and the
evaluation APIs say so.

`prepareNxIrModule` validates the whole image before it answers: the magic and schema version, the
recorded length against the bytes given, every section, offset array and string range, the string
blob's encoding, the runtime ABI and required features, and every entry of every table against its
layout, so every index the image holds is inside the table it names. What fails is an
`NxIrRuntimeError` naming the problem, and an image from another schema is refused naming both
versions, before anything is evaluated. A truncated or altered image is refused the same way, never
with an exception from inside the reader. `tryPrepareNxIrModule` and `tryLinkNxIrProgram` return
the same as a result instead of throwing.

## Diagnostics

A runtime diagnostic names the declaration the failing expression belongs to, as
`identity::name`. When the artifact carries its debug section, which the CLI writes and the SDKs
emit on request, the diagnostic also carries the expression's span in the module's source. The
runtime never reads a source file.

## Exports

| Export | What it does |
| --- | --- |
| `prepareNxIrModule`, `tryPrepareNxIrModule` | Validate and index one artifact. |
| `linkNxIrProgram`, `tryLinkNxIrProgram` | Link a prepared entry module against resolved modules. |
| `prepareNxIrProgram`, `tryPrepareNxIrProgram` | Prepare and link a self-contained artifact. |
| `evaluateFunction` | Evaluate a function entrypoint by name with arguments. |
| `constructComponentDescriptor`, `initializeComponent`, `evaluateComponent` | Build a component's descriptor, initialize its state, and evaluate it. |
| `normalizeComponentState`, `applyComponentStatePatch` | Bring component state into its declared shape and apply a patch. |
| `applyUpdate`, `mergeUpdates`, `diffRecords`, `changedFields` | Record update arithmetic over host-held values. |
| `NX_IR_SCHEMA_VERSION`, `NX_IR_RUNTIME_ABI` | The schema and ABI this runtime accepts. |
| `nodeKinds`, `typeKinds`, `constantKinds`, `declarationKinds` | The kind numbers of the schema. |
| `NxIrRuntimeError` | Thrown for an artifact the runtime cannot run, with its diagnostics. |

The opened image (`NxIrImage`) and the prepared types (`NxPreparedModule`, `NxPreparedProgram`,
`PreparedDeclaration`) are exported so a host can read the program it runs. The image's layout is
documented in `docs/nx-ir-format.md`; `nxlang ir explain`, or `explainNxIr` from either SDK,
renders one as text.

## Versions

Install this package and `@nx-lang/sdk-wasm` at the same release version. The two are released
together from one tag, and the artifacts the SDK emits at that version satisfy this runtime's
format, schema and feature checks. The repository's tests run every SDK's emitted IR and the
conformance corpus through this runtime on every build.

## Layout

| Path | What it holds |
| --- | --- |
| `src/index.ts` | The whole runtime |
| `test/runtime.test.ts` | Preparation, linking and boundary tests over images written by the test |
| `test/corpus.test.mjs` | Evaluates every image of `specs/ir-conformance` against the interpreter's results, and refuses every truncation and cell overwrite of them |
| `test/emitted-ir.test.mjs` | Compiles NX through the CLI and runs the emitted IR, comparing with the native evaluator |
