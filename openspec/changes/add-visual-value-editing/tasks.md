## 1. The source edit in the language service

- [x] 1.1 Add `crates/nx-language-service/src/source_edit.rs`: request types (`setValue` with a string, number, boolean or case value; `setText`; `remove`), the answer (`edits`, optional `refused`), and key resolution through the source tree of the document as sent; refuse `unknownNode` for a missing key; verify with unit tests
- [x] 1.2 `setValue` for a literal or bare case that is a whole value (an attribute's value, an item of a braced sequence, a parameterless `let`'s value), replacing only the target's text and keeping braces: strings quoted and escaped, numbers kept as given when they lex as a numeric literal, booleans, bare case names; refuse `notEditable` for operands, arguments, patterns and defaults, and `invalidValue` for what cannot be spelled; verify with tests for each scenario in `source-edits`
- [x] 1.3 `setText` for a one-line `text` node outside typed and raw bodies, written as it is and kept only when it reads back as one run of that text; refuse runs over several lines, typed bodies, and empty or multi-line text or text that would not read back; verify with tests
- [x] 1.4 `remove` for an attribute, with its separating whitespace, or its whole line when it stands alone; verify on one-line and one-attribute-per-line elements, first, middle and last attributes, and with comments beside it
- [x] 1.5 A round-trip test over the repository's `.nx` files: for every editable literal, setting it to itself answers no edits, and setting it to another value of its kind yields a document that parses with the same keys

## 2. Exposing it to TypeScript hosts

- [x] 2.1 `bindings/wasm`: a `nx_wasm_snapshot_source_edit` export and `sourceEdit` on the snapshot in `host.ts`; raise the ABI version; `bindings/node`: the same on the native snapshot and `index.ts`
- [x] 2.2 `packages/language-protocol`: `SourceEditRequest`, `SourceEdit`, `SourceEditValue`, `TextEdit`, `SourceEditAnswer`, `SourceEditRefusal`; `sourceEdit` in the query names, requests, answers and `NxLanguageService`; mark it unstable in the docs
- [x] 2.3 `language-core` (answer and service), `language-http` (request validation), `language-client` (method); verify with each package's tests, including a refusal round-tripping through HTTP as an answer, not an error
- [x] 2.4 Parity: the wasm SDK's language-service tests ask for source edits over their corpus through TypeScript and Rust and compare after JSON round-trip

## 3. Editing in the viewer

- [x] 3.1 Add the `editable` property (honored when set before definition), the `nx-edit` and `nx-open` events and their detail types, exported from the package; verify that nothing changes when `editable` is not set, with the existing tests unchanged
- [x] 3.2 Decide what is editable and with which editor, from the tree: the value's place, its property's declared type through the element's declaration and alias chain, `uitext`, `format=id`, `format=color`, the union's constant cases; verify with unit tests over fixtures
- [x] 3.3 The text field (strings, `uitext`, ids, text runs) in the value's box, decoding a string literal's escapes for its content (a text run is shown as written); commit, cancel, Tab and Shift+Tab; growth and wrapping within the reading; verify "Editing a string", "Cancelling" and "Nothing moves"
- [x] 3.4 The number field: arrow and Shift+arrow steps, scrubbing by dragging the property's name with one commit on release, invalid entries kept and marked; verify with tests using faked pointer events
- [x] 3.5 Boolean toggles (Space and a click on a selected unit's check), the case popover (constant cases, docs, current marked, type to filter, arrows and Enter), the color popover (hex field and the browser's color picker); verify "Toggling" and "Picking a case"
- [x] 3.6 Selection and opening: click on a value of the selected unit, double-click, Enter and F2, first value of a line; hover cards close when an editor opens; `nx-open` for values that cannot be edited; Ctrl/Cmd+Z and Y left to the host
- [x] 3.7 Reset and Clear: Delete and Backspace on a selected row whose property has a default or is optional, the quiet button on the selected row, selection moving to the element; verify "Resetting" and "A required property"
- [x] 3.8 The edited value shown muted until the next tree, then the tree as it is; the selection kept by key; verify "The host refuses"
- [x] 3.9 Styles: editors in the reading's faces and theme tokens, no borders or chrome at rest, a faint hover mark on editable values of the selected unit; light and dark

## 4. The playground

- [x] 4.1 The worker answers `sourceEdit`; the editor handle applies text edits with `executeEdits` as one undo step
- [x] 4.2 `ReadingView` sets `editable`, handles `nx-edit` in order (queued while one is in flight), asks for the source tree at once after an edit, and forwards Ctrl/Cmd+Z and redo to the editor; `App` switches to Source at the node for `nx-open`
- [x] 4.3 Verify the playground scenarios with its tests, and by screenshots in the playground: editing a string, picking a case, toggling, resetting, undo from Visual, in light and dark and at 375 pixels wide

## 5. Docs and checks

- [x] 5.1 Document editing, `editable`, `nx-edit` and `nx-open` in `packages/viewer/README.md`, and `sourceEdit` in the language packages' READMEs; add the changes to `src/vscode/CHANGELOG.md`'s next release and the package changelogs
- [x] 5.2 Run `cargo test`, `pnpm -r build` and `pnpm -r test`, and `openspec validate add-visual-value-editing --strict`, all passing

## 6. Revisions after trying it (Bret, 2026-10-21)

- [x] 6.1 Popover placement: place the popover by its value before bringing its current option into view, and scroll only its list, so the reading no longer jumps and the popover lands by its value
- [x] 6.2 A click on an editable value edits it (caret where the click fell; booleans toggle, cases and colors open their popover) and selects its unit; a click on a name selects; Enter and F2 still open with the text selected; a double-click's second click does not toggle again (replaces "select first" in 3.5 and 3.6)
- [x] 6.3 Hover cues on every editable value (text cursor or hand, faint mark, a chevron after a case) and an input look for an open field: background, focus ring and its own selection color (revises 3.9)
- [x] 6.4 Remove the row button and Backspace; Delete, or Cmd+Backspace on a Mac, removes a property with a default or an optional one (revises 3.7)
- [x] 6.5 Playground: Undo and Redo buttons in the NX title bar for both views, disabled when there is nothing to undo or redo, tooltips naming the shortcuts; Collapse all and Expand all as icons on a narrow screen
- [x] 6.6 The "Out of date" badge floats over the reading instead of taking a line, and the badge and the muting wait half a second, so an edit's brief staleness neither flashes nor moves the reading
