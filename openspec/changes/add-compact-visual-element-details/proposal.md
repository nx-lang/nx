## Why

Expanded properties in Visual push sibling elements out of view and mix navigation with inspection.
The interactive prototype demonstrates how compact element rows and focused inline details can keep
the composition readable while retaining NX's ability to compare properties in context.

## What Changes

- Add an opt-in compact presentation to `nx-viewer`, and enable it in the playground's NX Visual view.
  Composition elements have stable identity rows; nested property values retain their existing readers.
- Separate child folding from property disclosure. Selecting an element reveals its inline properties;
  selecting another closes only temporary details.
- Provide explicit Details disclosure and Pin controls. Explicitly opened details persist across
  selection changes; pins keep details open for comparison. Closing details also clears their pin.
- Integrate disclosure with source selection, property editing, undo, keyboard controls, scroll
  anchoring, and host-managed state across Visual/Source and engine switches.
- Keep rendering source-based, with no new evaluator, preview-instance mapping, or inspector pane.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `viewer`: Compact composition rows, independent disclosure, selection-following properties, pins,
  state events, editing continuity, and accessibility.
- `fiddle-nx-visual-view`: Enable compact Visual and preserve its disclosure state alongside folds
  and selection throughout the existing editor lifecycle.

## Impact

- `packages/viewer`: Rendering, styles, selection/folding, state API, editing integration, and tests.
- `sites/playground`: `ReadingView`, app state/reset handling, toolbar wording, and integration tests.
- Viewer and playground documentation, including the design discussion and prototype under `docs/`.
- No changes to NX syntax, source-tree protocol, evaluation, compiler output, or runtime previews.
  Other viewer hosts retain the existing reading unless they opt in to compact presentation.
