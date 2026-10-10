## 1. Package

- [x] 1.1 Create `packages/viewer` (`@nx-lang/viewer`) on the pattern of `packages/value-view`: the `<nx-viewer>` element with `tree`, `text`, `stale`, `ghosts` and `selection`, a shadow root, light and dark styles, and `defineNxViewerElement(registry)`; verify with the scenarios of `One element shows a source tree in any host`
- [x] 1.2 Add fixtures that compute source trees through `@nx-lang/sdk-wasm` at test time for a set of sources (the question-flow program, the agent example and one per role), listed in a committed JSON file of sources rather than of trees, which would be large and go stale; verify the tests read them
- [x] 1.3 Implement one renderer per role, the card for elements with declaration order and ghost rows, values, logic with the operator list, handlers, declarations, references and comments; verify with the scenarios of `Elements read as cards`, `Values have a style and lose their quotes`, `Omitted defaults show as ghosts`, `Logic reads as sentences`, `Handlers read as events` and `Declarations, references and comments read in place`
- [x] 1.4 Add the source toggle, sliced by byte offsets, and the details strips; verify with `The source of a card` and `Text after an emoji`
- [x] 1.5 Add hover from the declaration table and reference expansion; verify with the scenarios of `Hover and peek explain what a node refers to`
- [x] 1.6 Add selection, the `nx-select` event and `nodeAtSpan`; verify with the scenarios of `A selection is shared by key`

## 2. Coverage

- [x] 2.1 Add a coverage test that computes the source tree of every `.nx` file in the repository through `@nx-lang/sdk-wasm`, renders it in jsdom with the details strips open, and checks that every node's key appears exactly once; verify it passes, and that removing the `comment` renderer makes it fail naming a file and a key
- [x] 2.2 Record the render time of the question-flow program in the test output; verify it is under a second in jsdom (about 200 ms measured)

## 3. Playground

- [x] 3.1 Add the Edit and Read switch to the source pane, mount `<nx-viewer>` with the worker's `sourceTree`, and carry the selection both ways; verify with the scenarios of `The source pane reads as well as edits`
- [x] 3.2 Document the package in its README with a plain page and React example, and the Read view in the playground README; verify the examples run

## 4. Specs

- [x] 4.1 Run `openspec validate add-viewer --strict`; verify it passes
