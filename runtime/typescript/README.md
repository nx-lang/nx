# @nx-lang/ir-runtime

Evaluates persisted NX IR in JavaScript, in a browser or under Node, with no compiler and no NX
checkout. The IR is the schema 5 image `@nx-lang/sdk-wasm`, `@nx-lang/sdk-node`, the .NET SDK
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

One module never needs a resolver: the NX prelude, `@nx/prelude.nx`, which holds the declarations
every NX module sees without an import — `Range` today. This package ships the compiled prelude of
the compiler release it was built with, and linking serves it for that identity whenever the host's
resolver returns nothing for it, at any depth of the link. A host that wants its own prelude returns
a prepared module for that identity and linking uses that instead. The built-in prelude is prepared
at most once and reused across programs, so nothing is decoded for a program that never reaches it.

An artifact whose module table names only itself — or names only itself and the prelude — is a
program on its own: `prepareNxIrProgram` prepares and links it in one call, and a prepared module of
that shape can be passed to the evaluation APIs directly. A prepared module that names other modules
must be linked first, and the evaluation APIs say so.

`prepareNxIrModule` validates the whole image before it answers: the magic and schema version, the
recorded length against the bytes given, every section, offset array and string range, the string
blob's encoding, the runtime ABI and required features, and every entry of every table against its
layout, so every index the image holds is inside the table it names. What fails is an
`NxIrRuntimeError` naming the problem, and an image from another schema is refused naming both
versions, before anything is evaluated. A truncated or altered image is refused the same way, never
with an exception from inside the reader. `tryPrepareNxIrModule` and `tryLinkNxIrProgram` return
the same as a result instead of throwing.

## Components, instances and dispatch

`initializeComponent` normalizes the props, materializes the state, renders the body and returns
the rendered output, the initial state and an *instance*: an immutable value the host holds and
hands back to `dispatchComponentActions`, which runs a batch and returns the rendered output, the
effects, the next state and the next instance. Every handler in a lifecycle's rendered output is a
record `{ $type: "ActionHandler", action, token }`; the token is how the host names that handler in
a batch, and it is valid only for the instance returned with it. This is the `Counter` of
`examples/nx/component.nx`:

```ts
const { rendered, instance } = initializeComponent(program, "Counter", { step: 2 });
// rendered.children[1].onTapped is { $type: "ActionHandler", action: "Button.Tapped", token: "h1-1" }
const next = dispatchComponentActions(program, instance, [
  { $type: "ActionHandlerInvocation", token: "h1-1", action: { $type: "Button.Tapped" } },
]);
// next.state.count is 2; next.rendered carries tokens h2-1 and h2-2; next.effects is []
const reset = dispatchComponentActions(program, next.instance, [
  { $type: "ActionHandlerInvocation", token: "h2-2", action: { $type: "Button.Tapped" } },
]);
// reset.state.count is 0 and reset.effects is [{ $type: "Reset" }], for the host to route
```

A batch entry is either an action record the component emits, which runs the handler the parent
bound under `on<Emit>` and is a no-op when none was bound, or an `ActionHandlerInvocation` naming a
handler of the instance's most recent output by token, with the action to feed it. Entries run in
order. A handler the component's own body bound reads the state live, so two patches in one batch
compound, and every `<Component>.Update` it returns patches the state; every other returned record
is an effect. A handler bound anywhere else, at the root or in a parent whose content the component
renders, sees only what it captured, and everything it returns is an effect. A failure throws
`NxIrRuntimeError` before anything is returned, and the instance the call was given is unchanged
and still usable. Tokens are `h<generation>-<n>`: the generation is `1` at initialization and one
more after every dispatch, and the number is the handler's position in a walk that visits lists in
order and object keys in sorted order, the same walk the Rust runtime does, so the two runtimes
hand out the same tokens for the same program.

A host that initializes a child from a parent's rendered descriptor passes the descriptor's fields
as props and the parent's instance as `options.parent`:

```ts
const page = initializeComponent(program, "Page");
const { $type, ...props } = page.rendered.children[1];
const searchBox = initializeComponent(program, "SearchBox", props, { parent: page.instance });
const submitted = dispatchComponentActions(program, searchBox.instance, [
  { $type: "SearchSubmitted", searchString: "docs" },
]);
// submitted.effects is [{ $type: "DoSearch", search: "docs" }], the parent's handler's result
```

Every `ActionHandler` record among the props, at any depth, is replaced by the handler the parent
holds under its token; a token the parent does not hold is a diagnostic, and such a record with no
parent is an unknown field. A handler property is never in the child's scope: its body cannot read
`onSearchSubmitted`. The handler substituted is the parent's own value, not a copy: the handler a
child's table holds under one token is the same object the parent's table holds under another, so
a host holding both instances can find the instance whose body created a handler by looking for it
in each ancestor's table.

There is no "update props" operation. A host that has to re-render an instance whose props changed
initializes again with the new props and `options.state` set to the state the instance holds:

```ts
const kept = initializeComponent(program, "SearchBox", { placeholder: "Find" }, { state: instance.state });
```

The state is validated as a complete state for the component, as `evaluateComponent` validates
its state argument, and the new instance's tokens start at generation 1 again.

## Limits

Every evaluation API takes runtime options:

| Option | Default | What it does |
| --- | --- | --- |
| `maxCallDepth` | The runtime's own | The deepest chain of calls one evaluation may make. |
| `maxRangeLength` | `1_000_000` | The most integers one range may hold when a `for` iterates it. |

A range makes an enormous loop one token long — `for i in 0..2000000` is four tokens — so the count
is checked before the body runs at all, and a range above the limit fails with
`nx-ir-resource-limit` naming the limit. The default matches the NX interpreter's operation budget, so
a program that runs under `nxlang` runs here. A host with a legitimately larger loop raises the
limit:

```ts
evaluateFunction(program, "rows", [], { maxRangeLength: 5_000_000 });
```

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
| `evaluateFunction` | Evaluate a function entrypoint by name with positional arguments. The arguments may stop before trailing parameters that are optional or have a default; the function fills those itself. |
| `constructComponentDescriptor`, `initializeComponent`, `evaluateComponent` | Build a component's descriptor, initialize it into an instance, and evaluate it from explicit state. |
| `dispatchComponentActions` | Run a batch of actions and handler invocations against an instance. |
| `callFunction` | Call the function a `{ $type: "Function", module, name }` record names — a rendered template, say — with arguments keyed by parameter name; an argument the function does not declare is dropped, a parameter it declares and the arguments lack is a diagnostic naming it. |
| `normalizeComponentState`, `applyComponentStatePatch` | Bring component state into its declared shape and apply a patch. |
| `applyUpdate`, `mergeUpdates`, `diffRecords`, `changedFields` | Record update arithmetic over host-held values. |
| `float32Text` | The canonical text of a `float32` carried as a `number`: the shortest digits that round-trip as a `float32`, which is what a `text` node naming `float32` prints. |
| `NX_IR_SCHEMA_VERSION`, `NX_IR_RUNTIME_ABI` | The schema and ABI this runtime accepts. |
| `NX_IR_REQUIRED_FEATURE_*` | The required features this runtime knows, including `ranges-v1` for iteration over a range. An image listing a feature this runtime does not know is refused by name. |
| `NX_PRELUDE_MODULE_IDENTITY`, `NX_DEFAULT_MAX_RANGE_LENGTH` | The prelude's reserved identity, and the default range-length limit. |
| `nodeKinds`, `typeKinds`, `constantKinds`, `declarationKinds` | The kind numbers of the schema. |
| `NxIrRuntimeError` | Thrown for an artifact the runtime cannot run, with its diagnostics. |

The opened image (`NxIrImage`), the prepared types (`NxPreparedModule`, `NxPreparedProgram`,
`PreparedDeclaration`) and the instance (`NxComponentInstance`) are exported so a host can read the
program it runs and type what it holds; the instance's fields are the runtime's, not an API. The image's layout is
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
| `test/corpus.test.mjs` | Evaluates every image of `specs/ir-conformance` against the interpreter's results, drives every lifecycle it names, and refuses every truncation and cell overwrite of them |
| `test/emitted-ir.test.mjs` | Compiles NX through the CLI and runs the emitted IR, comparing with the native evaluator |
