## 1. Spelling runtime values

- [x] 1.1 In `crates/nx-api`, add a formatter entry that takes a canonical JSON value, converts it with `ActionHandler` records kept as records, and returns annotated `NxValueText` from the formatter `evaluateNx` uses; verify with Rust tests for a rendered component, an invalid value and parity with `evaluateNx` text
- [x] 1.2 Give every `NxValueText` node the JSON pointer of its value, from both entries; verify with the scenario `Pointers into a rendered output`
- [x] 1.3 Export it from `bindings/wasm/native` as `nx_wasm_program_format_value`, add `NxProgramArtifact.formatValue` and `NxValueNode.pointer`, and move the ABI version to 8; verify with the wrapper's tests and the scenario `The ABI version moves with the new export`
- [x] 1.4 Document `formatValue` and `pointer` in the sdk-wasm README; verify the README's examples still type-check

## 2. Actions on values

- [x] 2.1 Add `pointer` to `@nx-lang/value-view`'s `NxValueNode`, the `actions` property, the buttons and the `nx-value-action` event; verify with the scenarios of `A host can attach actions to nodes`
- [x] 2.2 Document `actions` in the value-view README with an example

## 3. Preview in the worker

- [x] 3.1 Add `@nx-lang/previewer` and `@nx-lang/ir-runtime` to the playground, keep the latest build's artifact in `worker/session.ts`, and add the `preview` call to `worker/protocol.ts` with `start`, `dispatch`, `goTo`, `reload` and `stop`; verify with Node tests against the wasm module
- [x] 3.2 Choose the component from `root`'s value or the source's first component, list the source's components, and follow `root`'s props across reloads; verify with the scenarios of `The output pane previews a component` and `New props`
- [x] 3.3 Build snapshots: path steps with branch counts, output, emitted actions and state fields spelled by `formatValue` and cut at 100,000 characters, handlers with their input schemas, and origins as source spans; verify with Node tests over the counter example
- [x] 3.4 Set the runtime limits for every session call; verify with the scenario `A runaway handler`

## 4. Preview in the page

- [x] 4.1 Add the Output and Preview switch, the component picker and the messages for no component, a missing prop and a program that does not compile; verify with the scenarios of `The output pane previews a component`
- [x] 4.2 Add buttons for handlers and the action form; verify with the scenarios of `Handlers in the preview are buttons`
- [x] 4.3 Add the timeline strip with branches, Back and Forward; verify with the scenarios of `The preview keeps a timeline`
- [x] 4.4 Add the state and emitted panel; verify with the scenarios of `The preview shows state and what was emitted`
- [x] 4.5 Reload on each compiled edit and show what the reload did; verify with the scenarios of `Edits hot reload the preview`
- [x] 4.6 Navigate from a record to the expression that built it; verify with `From a rendered label to its expression`
- [x] 4.7 Check the layout at 375 pixels and in both themes; verify with `Preview on a phone` and screenshots

## 5. Example and docs

- [x] 5.1 Add "A counter to click" to `src/examples` with its expected output; verify `pnpm run check-examples` and `Opening the example`
- [x] 5.2 Describe Preview in the playground's README

## 6. Specs

- [x] 6.1 Run `openspec validate add-preview-pane --strict`; verify it passes
