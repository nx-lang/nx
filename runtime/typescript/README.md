# @nx-lang/ir-runtime

Evaluates persisted NX IR in JavaScript, in a browser or under Node, with no compiler and no NX
checkout. The IR is the JSON document `@nx-lang/sdk-wasm`, `@nx-lang/sdk-node`, the .NET SDK and
the `nx` CLI emit for a compiled program; this package turns it into values.

Use it when JavaScript only needs to *execute* an already compiled program. When JavaScript needs
to *compile* NX source or answer editor queries over it, use `@nx-lang/sdk-wasm`, which emits the
IR this package runs.

```ts
import { evaluateFunction, prepareNxIrProgram } from "@nx-lang/ir-runtime";

const program = prepareNxIrProgram(irJson); // a string, or the parsed document
const root = evaluateFunction(program, "root", []);
```

`prepareNxIrProgram` checks the document's format, schema version, runtime ABI and required
features against what this runtime supports and throws `NxIrRuntimeError` when they disagree, so a
document from a newer compiler fails at preparation rather than partway through evaluation.
`tryPrepareNxIrProgram` returns the same as a result instead of throwing. A prepared program is
reusable: prepare once, evaluate as often as needed.

## Exports

| Export | What it does |
| --- | --- |
| `prepareNxIrProgram`, `tryPrepareNxIrProgram` | Validate and index an IR document. |
| `evaluateFunction` | Evaluate a public function entrypoint by name with arguments. |
| `constructComponentDescriptor`, `initializeComponent`, `evaluateComponent` | Build a component's descriptor, initialize its state, and evaluate it. |
| `normalizeComponentState`, `applyComponentStatePatch` | Bring component state into its declared shape and apply a patch. |
| `applyUpdate`, `mergeUpdates`, `diffRecords`, `changedFields` | Record update arithmetic over prepared programs. |
| `NX_IR_FORMAT_ID`, `NX_IR_SCHEMA_VERSION`, `NX_IR_RUNTIME_ABI` | The document format, schema and ABI this runtime accepts. |
| `NxIrRuntimeError` | Thrown for a document the runtime cannot run, with its diagnostics. |

The IR document types (`NxIrProgram`, `NxIrModule`, `NxIrDeclaration`, and the rest) are exported
so a host can read the program it runs.

## Versions

Install this package and `@nx-lang/sdk-wasm` at the same release version. The two are released
together from one tag, and the IR the SDK emits at that version satisfies this runtime's format,
schema and feature checks. The repository's tests run every SDK's emitted IR through this runtime
on every build.

## Layout

| Path | What it holds |
| --- | --- |
| `src/index.ts` | The whole runtime |
| `test/runtime.test.ts` | Unit tests over hand-written IR |
| `test/emitted-ir.test.mjs` | Compiles NX through the CLI and runs the emitted IR, comparing with the native evaluator |
