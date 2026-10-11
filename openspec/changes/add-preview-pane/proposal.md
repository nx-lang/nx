## Why

The playground answers "what does `root` return?", and since `add-viewer` it also answers "does
this say what I meant?". It cannot yet answer the previewer's question, "does it behave right?".
A component's output holds action handlers, which have no NX spelling, so the output pane can only
say that the value cannot be shown, and nothing in the page can press a button, step back or try
another answer.

`add-preview-session` gave us the engine: `@nx-lang/previewer` runs one component headless on the
TypeScript IR runtime, keeps every render as a tick in a tree, dispatches by handler pointer, hot
reloads, and reports where each record of the output came from. This change puts it in the
playground, the last of the four lanes in the NX Viewer and Previewer design, so a visitor can
write a component and use it on the same page.

## What Changes

- The playground's output pane gains an Output and Preview switch, as the source pane has Edit and
  Read. Preview runs a component of the visitor's source in a preview session inside the
  playground's worker and shows its current output, its state and what it emitted.
- Preview runs the component `root` returns, with the props of that element, so the source is
  where props are written. When `root` returns no component element, Preview runs the first
  component the source declares with its default props. A picker chooses among the source's
  components.
- Each action handler in the output gets a button. Pressing it dispatches the handler's action.
  An action with fields opens a small form, built from the action's input schema.
- A timeline strip shows the run: one step per tick on the current path, a branch marker where a
  step has other continuations, and Back and Forward. Choosing a step goes back to it without
  running anything, and dispatching from an earlier step adds a branch.
- Editing hot reloads the preview: the state is kept when the new program accepts it, the run is
  replayed when it does not, and the last good program is kept when the edit breaks it. The pane
  says which happened.
- In Preview, a record's type name leads to the element expression in the source that built it,
  from the session's value origins.
- `@nx-lang/sdk-wasm`: a program artifact spells a runtime value, in canonical JSON as the IR
  runtimes return it, as annotated NX text (`formatValue`), with the same formatter `evaluateNx`
  uses. An action handler spells as the record the runtimes give it,
  `<ActionHandler action="Button.Tapped" token="h1-1" />`. Every node of an annotated text carries
  the JSON pointer of its value. The ABI version moves to 8.
- `@nx-lang/value-view`: a host can attach actions to nodes. The element shows each as a button
  on the node's first line and reports a press with an `nx-value-action` event.
- A new example, "A counter to click", ends with a component element so it previews at once.

Not in this change: the pane shell and chat rail from lane 4 stage 1, saving or sharing a run as a
scenario, selection from the source to the preview, a props form (no props schema exists until
`T.Property` means props for components), and viewer hints.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `playground`: the output pane previews a component, with action buttons, a timeline, state and
  emitted actions, hot reload and navigation by origin; a new example.
- `sdk-wasm`: program artifacts spell runtime values as annotated NX text, nodes carry pointers,
  ABI 8.
- `value-view`: nodes can carry host actions.

## Impact

- `crates/nx-api`: a formatter entry that takes canonical JSON and spells `ActionHandler` records,
  and node pointers in `NxValueText`.
- `bindings/wasm`: the module export and `NxProgramArtifact.formatValue`, `NxValueNode.pointer`,
  ABI 8.
- `packages/value-view`: the `actions` property and the `nx-value-action` event.
- `sites/playground`: depends on `@nx-lang/previewer` and `@nx-lang/ir-runtime`; preview calls on
  the worker channel; the Preview view, timeline, state panel and action form; the new example.
- No change to the language, the IR format, the IR runtimes or `@nx-lang/previewer`.
