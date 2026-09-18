## 1. NX IR: the action-handler kind and component emits

- [x] 1.1 Add `kinds::node::ACTION_HANDLER = 19` (with its `NAMES` entry) and
  `NX_IR_REQUIRED_FEATURE_ACTION_HANDLERS_V1` to `crates/nx-codegen/src/ir.rs`, the matching
  `CodegenExpressionKind::ActionHandler` (component, emit, action and owner references, body) in
  `model.rs`, its emission as `[19, ref, str, ref, slot, ref?, node]` with the `action` slot
  allocated like a `let`, and list the feature from `required_features` only when a module contains
  the kind; verify `cargo build -p nx-codegen` passes with every exhaustive match updated and
  `visit_expressions` descends into the body
- [x] 1.2 Append `emits` (`[[str, ref]...]`, inherited included, declaration order, references
  naming the emit's declaring module) to the component declaration layout in `ir.rs`, and add the
  operand to the layout table in `ir_image.rs`; verify an `ir_tests.rs` case over explained text
  shows `component <Counter emits { Reset ValueChanged { value:int } } />` listing both, with
  `ValueChanged` referencing the inline `Counter.ValueChanged` record, and a component with no
  emits listing none
- [x] 1.3 Add the node's layout to the `ir_image.rs` table and render it and the `emits` block in
  `ir_explain.rs` (one line per emit, `<name> = <module>:<action>`; the handler as
  `handler <module>:<Component>.<Emit> action@<slot> [owner <Component>] => <body>` or a close
  equivalent); verify the corpus's damaged-image tests still refuse or accept every cell overwrite
  once the corpus carries a handler program
- [x] 1.4 Replace the `ActionHandler` diagnostic arm in `build_expression` with a builder that
  pushes a scope, inserts `action`, builds the body, resolves the component and action references
  from the handler's module identities, and carries the owner reference; verify `ir_tests.rs`
  cases over explained text match the two spec scenarios (owned `Counter` handler referencing the
  `count` state slot with the feature listed; ownerless root handler referencing its `action` slot)
- [x] 1.5 Add the `CodegenExpressionKind::ActionHandler` arm to `emit.rs` that reports
  `codegen-unsupported-construct` with the existing message and emits nothing; verify
  `component_action_handler_bindings_fail_before_emission` in `tests.rs` still passes for the
  JavaScript target and a new `ir_tests.rs` case emits an image for the same source
- [x] 1.6 Update `crates/nx-ffi/tests/ffi_smoke.rs` and
  `bindings/dotnet/tests/NxLang.Sdk.Tests/NxEndToEndTests.cs` so their NX IR cases with a handler
  expect an image whose explained text names the handler, while their JS program-module cases keep
  expecting the diagnostic; verify `cargo test --workspace` and `dotnet test bindings/dotnet/NxLang.sln`
  pass

## 2. Conformance corpus

- [x] 2.1 Add `specs/ir-conformance/handlers` with a program that covers node kind 19 and a
  component with emits: the `Counter` from `examples/nx/component.nx`, handlers inside a `for`
  loop whose variable shadows a state field (NX has no `let` expression, so the loop is the
  shadowing binding, per D8), one on a descriptor prop declared out of sorted order, a `Page` whose
  content child binds a `Page`-owned handler inside a `Stack`, and a `root` function binding a
  handler outside any component; add its coverage to `manifest.json`; verify the coverage check
  passes and the size budget holds
- [x] 2.2 Extend `program.json` with `lifecycles` (module, component, optional props, ordered
  batches of literal entries with literal tokens) and `ir_corpus_tests.rs` to initialize and
  dispatch each through `nx-api`, recording under `identity::Component` in `results.json` the
  initial rendered output and, per batch, the rendered output and effects; document the fields in
  `specs/ir-conformance/README.md`; verify `NX_UPDATE_CORPUS=1 cargo test -p nx-codegen --lib
  ir_corpus` writes them and a second run without the variable passes
- [x] 2.3 Regenerate every corpus program that declares a component for the new layout and review
  the explained-text diff, which should show only the added `emits` lines; verify
  `cargo test -p nx-codegen` passes

## 3. TypeScript IR runtime: handler values and canonical output

- [x] 3.1 Register kind 19 in `nodeKinds`, its layout in the reader's validation, `emits` on the
  prepared component declaration, and the `action-handlers-v1` feature in
  `runtime/typescript/src/index.ts`; evaluate the node to an internal `$nxKind: "actionHandler"`
  value capturing `frame.slice()`; verify a runtime test prepares an image with the feature and
  rejects one naming an unknown feature
- [x] 3.2 Add `canonicalizeRendered` (arrays in order, object keys sorted, handlers to
  `{ $type: "ActionHandler", action }` plus `token` under a generation, returning the collected
  handler table) and apply it at the exit of `evaluateFunction`, `constructComponentDescriptor`,
  and `evaluateComponent` without tokens; verify tests show a descriptor's `onTapped` as a
  token-less `ActionHandler` record and no `$nxKind` key in any output
- [x] 3.3 Split handler properties from declared props wherever descriptor properties are
  normalized (`evalComponentDescriptor`, and the props a host supplies to `initializeComponent`,
  `evaluateComponent` and `constructComponentDescriptor`; D5 explains why component-typed host
  input is not a site) using the target's `emits`, attach them without normalization,
  fail with `nx-ir-boundary-field` for a handler property naming no emit and `nx-ir-type` for a
  handler bound to a different emit; verify tests cover a carried `onReset`, a rejected `onNope`,
  and that the body of an initialized component cannot read `onTapped`

## 4. TypeScript IR runtime: instances and dispatch

- [x] 4.1 Add the opaque `NxComponentInstance` (component, reference, props with handler props,
  state, handler table, generation), return it from `initializeComponent` alongside the existing
  `rendered` and `state`, and assign tokens with generation 1; verify a test reads `h1-1` from
  `Counter`'s rendered `onTapped` and that existing initialization tests pass unchanged
- [x] 4.2 Add `options.parent` to `initializeComponent`: every `ActionHandler` record in the props
  at any depth is replaced by the parent's handler under its token, an unknown token fails with
  `nx-ir-handler-token`, and such a record with no parent fails with `nx-ir-boundary-field`;
  verify tests initialize `SearchBox` from `Page`'s rendered output and a content child's handler
  survives into the child's rendered output with a new token
- [x] 4.3 Implement `dispatchComponentActions(program, instance, batch, options?)` per design D7:
  token lookup, action type check and host-input normalization, owned-handler live state by slot,
  result normalization, update routing through the state patch logic, effects, parent-bound
  handler entries via `on<Emit>`, no-op for unbound emits, one re-render with generation + 1, and
  a new instance; verify runtime tests cover every scenario of the dispatch requirement (patch,
  live state twice, shadowed `let`, non-update effects, emitted action to parent handler, parent
  update as effect, unowned content-child handler, unbound no-op, empty batch retokenizes, stale
  token, wrong action, unknown emit, atomic failure, invalid update aborts)
- [x] 4.4 Confirm dispatch leaves the supplied instance untouched by freezing instance objects and
  re-dispatching against the original after a failed batch in a test; verify the test observes the
  original state
- [x] 4.5 Run the corpus lifecycles in `runtime/typescript/test/corpus.test.mjs` through
  `initializeComponent` and `dispatchComponentActions`, comparing each recorded value with
  `stableJson`; verify `pnpm -r test` passes and a deliberately reordered token walk fails the
  comparison
- [x] 4.6 Extend `runtime/typescript/test/emitted-ir.test.mjs` with a component that binds a
  handler, so the emitted-IR path through the real CLI prepares and initializes a program with the
  feature; run the wasm and Node SDK suites unchanged (`bindings/wasm`, `bindings/node`) and
  `pnpm run check-examples` in `sites/playground` with the handler-related entries dropped from its
  skip list; verify all pass

## 5. Documentation

- [x] 5.1 Update `docs/nx-ir-format.md`: kind 19 in the node table, `emits` in the component
  declaration layout, `action-handlers-v1` under required features, the `ActionHandler` value and
  the instance and dispatch APIs in the runtime section, the corpus lifecycle fields, and rewrite
  the "Non-goals" paragraph that says schema 3 assigns no kind to a handler; verify the document
  names every operand the node carries and the worked example's cells still match
- [x] 5.2 Update `runtime/typescript/README.md` with the instance shape, the parent option, the
  dispatch API, the batch entry forms, and the token rule; verify the example dispatches the
  `Counter` from `examples/nx/component.nx`

## 6. TypeScript IR runtime: re-initialization and handler identity

- [x] 6.1 Add `options.state` to `initializeComponent` in `runtime/typescript/src/index.ts`,
  validated as a complete state the way `evaluateComponent` validates its `state` argument and
  used in place of the initial state, with tokens assigned as for any initialization; verify
  runtime tests initialize `SearchBox` with `{ query: "docs" }` and read it back rendered, and
  refuse a mistyped and a missing field with a diagnostic naming the field
- [x] 6.2 Pin handler identity across the parent route with a runtime test against the corpus's
  `handlers` image: initialize `Stack` from `Page`'s rendered content with `Page` as parent and
  assert the handler under `Stack`'s token is the same value as under `Page`'s; document the
  `state` option and the identity in `runtime/typescript/README.md` and in the runtime section of
  `docs/nx-ir-format.md`; verify `pnpm test` in `runtime/typescript` passes

## 7. DrawnUI catalog: events as emits

- [x] 7.1 Extend `sites/playground/scripts/generate-catalog.mjs` per design D9: an optional
  function-typed member returning `void` (or a union with it) is an event, declared as an emit on
  the component for its declaring class with payload fields for the primitive parameters after
  `sender`, dropped parameters recorded in `omitted.json` with the parameter and its type,
  non-event functions omitted as before, and `catalog-meta.json` carrying each renderable
  control's events with parameter names in order (inherited included); regenerate `catalog/`;
  verify the diff adds emits and removes only the event entries from the omitted list, a second
  run is identical, the catalog compiles alone (`src/compile/catalog.test.mjs`), and a snippet
  binding `<SkiaButton onTapped=... />` inside a component compiles while `onNope` fails naming
  the property
- [x] 7.2 Rewrite the "Properties with no NX expression" section of
  `sites/playground/docs/CATALOG.md`: the event rule, the dropped-parameter table (event,
  parameter, type), `ContextMenu`'s return value, and the two non-event functions that stay
  omitted; verify every dropped parameter in `omitted.json` appears in the table

## 8. Playground renderer: instances and dispatch

- [x] 8.1 Add `sites/playground/src/render/instances.ts`, plain TypeScript with no React: create
  an instance from a descriptor under an optional parent, re-initialize with new fields and the
  held state, dispatch a token found in an instance's output against the ancestor that owns the
  handler (D12), route each effect to the parent-bound handler's owner by emit or collect it as a
  host effect naming the instance, and surface a failed dispatch without changing any instance;
  add `instances.test.mjs` compiling snippets through the wasm host against the catalog (as
  `evaluate.test.mjs` does) covering: a tap patches state, two taps read live state, an emit
  patches the parent, a content-child handler patches its owner while the child keeps its state,
  a parent redraw keeps the child's state, an unbound emit is a host effect, a failed dispatch
  leaves every instance, and a token-less handler is reported; verify `pnpm test` passes
- [x] 8.2 In `src/render/values.ts` and `DrawnTree.tsx`: `coerceProps` turns a tokened
  `ActionHandler` under `on<Emit>` into the DrawnUI callback `<Emit>` building the action from the
  metadata's parameter names and the action name the record carries (`ContextMenu` returning
  `true`), reports a token-less one once per drawing, and leaves other objects as before;
  `drawValue` visits the instance tree from 8.1 for an authored descriptor, keyed by the
  descriptor's position, and draws its output under a context that dispatches, so a parent's
  redraw re-initializes it with the held state; verify the type check passes and the existing
  render tests still pass
- [x] 8.3 In `src/render/useNxDrawing.ts` and `src/editor/EditorView.tsx`: carry host effects and
  dispatch failures out of the drawing, list the most recent effects (action and instance) and
  any failure in the diagnostics pane, clear both on recompile, and replace the "Authored
  interaction is not supported yet" notice with one sentence saying handlers and state run inside
  components and unhandled actions are listed below; verify in the browser (Playwright, per the
  memory note) that the Counter snippet from the spec redraws on a tap, a second tap compounds,
  an unbound emit is listed, and the Text example's readouts update
- [x] 8.4 Route `scripts/expand-components.mjs` through `instances.ts`, so every authored
  component in every example is initialized under its parent with handler properties resolved;
  verify `pnpm run check-examples` passes for all examples, including the two ported in 9.2

## 9. Examples

- [x] 9.1 In `src/examples/types.ts`, word `event-handlers` and `component-state` as what the port
  does not use yet and the other three as what NX lacks, keeping the vocabulary; update the
  README paragraph that describes the vocabulary; verify the check's vocabulary is unchanged and
  the gallery chips read correctly for a static, a reduced, and a mixed example
- [x] 9.2 Port Text and Shapes in full: the page becomes a component with state; Text's span tap
  and `SkiaRichLabel` link tap drive the two readouts (no time stamp; the tap reads "link
  tapped"); Shapes' context-menu card reports the taken request in a label with the browser menu
  suppressed; mark both `complete` in `examples.json` with no capabilities and no note; verify
  both compile, the check passes, and the readouts update in the browser
- [x] 9.3 Reword the in-source notes of the remaining static and reduced examples so none says NX
  has no event handlers or component state, keeping each note at the point the behavior would
  appear and naming the same capability; verify `grep -n "NX has no event handlers\|no component
  state" src/examples/nx` finds nothing

## 10. Playground documentation

- [x] 10.1 Rewrite the "What it does not do" section of `sites/playground/README.md`: what runs
  where (handlers and state inside components, the root evaluated once, effects listed), the
  pattern of a page component, the remaining gaps by capability, and the number-to-string
  conversion the remaining readouts need; verify the README no longer says authored interaction is
  unsupported and `openspec validate ir-action-handlers` passes
