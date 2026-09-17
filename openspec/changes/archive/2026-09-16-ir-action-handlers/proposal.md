## Why

`add-update-actions` finished the language half of interaction: handler bodies are type checked,
`<Update ... />` patches a component's state, and the Rust runtime dispatches a handler by token,
re-renders, and returns the next snapshot. None of that reaches a browser. The playground and the
DrawnUI fiddle run compiled NX IR through the TypeScript IR runtime, and that path stops at the
compiler: `nx-codegen` refuses any program that binds a handler ("action-handler codegen is not
supported by this non-reactive executable target"), schema 3 assigns no node kind to one, and the
TypeScript runtime has no handler value and no dispatch entry. `add-update-actions` named this gap
as the separate `ir-action-handlers` work and deferred it; every interactive NX example in both
hosts is blocked on it.

The playground is the browser host this is for, and with the compiler and runtime halves done it
still draws every handler as an inert record: its DrawnUI catalog omits every event, its renderer
initializes an authored component once and never dispatches, and its editor tells the visitor that
authored interaction is unsupported. Landing the renderer in the same change keeps the whole path
testable together: a tap in the playground exercises the catalog, the compiler, the image, the
runtime and the composition of instances, and the change ends with a working system rather than an
API waiting for its first caller.

## What Changes

- NX IR schema 3 gains node kind 19, `actionHandler`, carrying what the lowered AST already holds:
  a reference to the component and the name of the emit the handler answers, a reference to the
  action record it accepts, the slot its `action` binding occupies, an optional reference to the
  owner component whose state it may patch, and the body node. A module that contains one lists a
  new required feature, `action-handlers-v1`, so a runtime that predates handlers refuses the
  module by name rather than by an unknown kind number.
- Component declarations gain a trailing `emits` list: each emit's name and a reference to the
  action record it carries, inherited emits included, so a runtime can validate a dispatched action
  against the contract and recognize a descriptor's handler properties (`on<Emit>`) without
  consulting source. The schema version stays `3`: no schema 3 image has been published in any
  encoding, so the layout is amended in place. Every conformance corpus image that declares a
  component is regenerated, and the reviewed explained-text diff is the record of what changed.
- **BREAKING (test expectations only):** the NX IR target no longer fails on a handler binding.
  The executable TypeScript and JavaScript targets keep rejecting handlers with the same
  diagnostic, enforced at emission instead of at IR build, so `executable-code-generation`'s
  requirement is unchanged. The .NET and FFI tests that expect `GenerateNxIr` to fail move to
  expecting an image whose explained text shows the handler.
- The image reader in both languages validates the new layouts, `nxlang ir explain` renders the
  node and the emits, and the conformance corpus gains a program that binds handlers, which the
  corpus's kind-coverage check requires.
- The TypeScript IR runtime evaluates a handler node to a handler value that captures the frame by
  value, renders it as `{ $type: "ActionHandler", action, token }` with the same token scheme as
  the Rust runtime (`h<generation>-<n>`, assigned in the same walk order), and keeps handler-valued
  properties on a component descriptor beside the normalized props rather than rejecting them as
  unknown fields.
- `initializeComponent` returns, beside the rendered output and the initial state, an opaque
  instance the host passes back. A host that initializes a child from a parent's rendered output
  passes the parent's instance too, and every token-bearing `ActionHandler` record in the props is
  resolved through it, which is the one route by which a parent-bound handler reaches a child
  instance. The handler substituted is the parent's own handler value, so a host that holds both
  instances can find the token the parent's table holds it under. Initialization also accepts a
  `state` object to use in place of the initial state, validated as a complete state, so a host
  can re-render an instance with new props without losing the state it holds.
- The TypeScript IR runtime gains `dispatchComponentActions`, mirroring the Rust dispatch: a batch
  of action records and handler invocations against an instance; owned handlers read state live,
  their update records patch the working state in order, every other result is an effect, the body
  re-renders once, and the whole batch fails atomically.
- The conformance corpus gains lifecycle scenarios: a component to initialize and the batches to
  dispatch against it, with the rendered output (tokens included) and effects the interpreter
  produces recorded beside the function results. The TypeScript runtime's corpus test runs them, so
  dispatch parity is checked where every other parity already is.
- The DrawnUI catalog declares every DrawnUI event as an emit on the component for the class that
  declares it: `Tapped` and `ContextMenu` on `SkiaControl`, `Toggled { value:boolean }` on
  `SkiaToggle`, `TextChanged { text:string }` on `SkiaEditor`, `LinkTapped { url:string }` on
  `SkiaRichLabel`, and so on, inherited by every control below. A callback parameter with a
  primitive NX type becomes a payload field under its own name; a parameter typed as an engine
  object is dropped and recorded. The generated metadata records each event's parameter names, so
  the renderer can build the action from a callback's arguments. The 43 omitted event handlers
  leave `docs/CATALOG.md`; functions that are not events (`ItemTemplate`, `ProcessJson`) stay
  omitted.
- The playground renderer draws every use of an authored component as an instance. An instance
  tree keyed by position in the drawing holds the runtime instances, re-initializes one with the
  state it held whenever its parent redraws, and turns each `ActionHandler` record on a control
  into the DrawnUI callback of the same event, which dispatches one batch and redraws. Results go
  where the language says they go: an update patches the state of the instance whose body bound
  the handler, an action a component emits reaches the handler its parent bound through the token
  the parent's rendered output carries, and anything left over is a host effect listed in the
  diagnostics pane with the instance that produced it. A dispatch that fails is reported there and
  leaves the drawing as it was. A handler bound outside any component is reported as inert, since
  pure evaluation carries no token.
- The examples' capability vocabulary distinguishes what NX lacks (animation, list virtualization,
  code-behind) from what NX now has and a port does not use yet (event handlers, component state),
  and the in-source notes stop saying NX has no handlers. The two examples blocked on event handlers
  alone, Text and Shapes, are ported in full and become complete. The rest keep their tags; their
  upgrades are porting work with judgment per example, and most of their readouts need a
  number-to-string conversion NX does not have.
- `docs/nx-ir-format.md`, the runtime package README, the format's non-goals, the playground's
  README, `docs/CATALOG.md`, and the editor's notice are updated; the IR is still eager and
  non-reactive, it now simply carries handlers as data.

Out of scope: the DrawnUI fiddle, which lives in its own repository and waits on the published
packages; the presets and playground-independent parts of a "share" flow; porting the remaining
static examples; a number-to-string conversion in NX. Also out of scope: invoking a handler rendered
by pure evaluation (`evaluateFunction`, `evaluateComponent`), which carries no token in the Rust
runtime either; a batch entry that applies a component's own update record directly, which neither
runtime accepts today; and a public Rust route for a parent-bound handler to reach a child
instance, which `nx-api` does not offer (its input decoder refuses `ActionHandler` records).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `nx-ir-format`: the node table gains the action-handler kind with its required feature,
  component declarations carry `emits`, the corpus gains lifecycle scenarios, and the "unsupported
  action handler is rejected" scenario now says IR emission succeeds while the executable targets
  still fail, as `executable-code-generation` requires.
- `typescript-ir-runtime`: handler values and their rendered encoding, handler properties on
  descriptors, the instance returned by initialization, the parent route for handler props and the
  identity of what it substitutes, initialization with a supplied state, `dispatchComponentActions`
  with live state, routing, tokens, and atomicity, and corpus lifecycle checks.
- `drawnui-nx-catalog`: DrawnUI events are declared as emits with payloads derived from their
  callback parameters; the exclusion of callbacks narrows to functions that are not events.
- `playground`: authored handlers run in the output pane through an instance tree, handler
  properties become DrawnUI callbacks, unhandled effects and failed dispatches are shown, the
  capability wording separates landed capabilities from missing ones, and the example check
  expands components the way the renderer does.

## Impact

- `crates/nx-codegen/src/ir.rs`, `ir_image.rs`, `ir_explain.rs`, `model.rs`, `builder.rs`,
  `emit.rs`: the new kind, its layout in the reader's validation table, its explained form, the
  feature, `emits` on component declarations, the handler's `action` slot, and the executable-target
  rejection moved to the emitter. `ir_tests.rs` gains shape tests over explained text; `tests.rs`
  keeps the executable rejection test.
- `specs/ir-conformance`: a new program binding handlers, regenerated images and explained text
  for every program that declares a component, `manifest.json` coverage, and lifecycle scenarios
  in `program.json` and `results.json`. `crates/nx-codegen/src/ir_corpus_tests.rs` records the
  lifecycle results through `nx-api`.
- `crates/nx-ffi/tests/ffi_smoke.rs` and `bindings/dotnet/tests/NxLang.Sdk.Tests/NxEndToEndTests.cs`:
  the NX IR cases flip from expecting a diagnostic to expecting an image.
- `runtime/typescript/src/index.ts`: the kind and feature in the reader and validators, `emits`
  on the prepared component declaration, handler evaluation, canonicalization with tokens,
  descriptor handler properties, the instance shape, the parent route, the `state` option, and
  `dispatchComponentActions`. `runtime/typescript/test/runtime.test.ts` grows dispatch and
  re-initialization cases; `test/corpus.test.mjs` runs the lifecycle scenarios;
  `test/emitted-ir.test.mjs` compiles a program with a handler.
- `sites/playground/scripts/generate-catalog.mjs` and the generated `catalog/skia.nx`,
  `catalog/catalog-meta.json` and `catalog/omitted.json`: events as emits, parameter names in the
  metadata, dropped parameters recorded.
- `sites/playground/src/render/instances.ts` (new, with a test): the instance tree, dispatch by
  token to the instance that owns the handler, effect routing, and host effects.
  `src/render/values.ts` and `DrawnTree.tsx`: handler records to callbacks, instances drawn by a
  React component. `src/render/useNxDrawing.ts` and `src/editor/EditorView.tsx`: effects and
  dispatch failures in the diagnostics pane, the notice replaced. `scripts/expand-components.mjs`
  and `scripts/check-examples.mjs`: expansion through the instance tree.
- `sites/playground/src/examples/types.ts`, `examples.json`, `nx/text.nx`, `nx/shapes.nx`, and the
  notes in the other example sources.
- `docs/nx-ir-format.md`, `runtime/typescript/README.md`, `specs/ir-conformance/README.md`,
  `sites/playground/README.md`, `sites/playground/docs/CATALOG.md`.
- No change to the Rust interpreter, `nx-api`, the FFI surface, the .NET SDK's runtime bindings,
  the wasm and Node SDKs (they pass images through), the runtime ABI, the language, or the grammar.
