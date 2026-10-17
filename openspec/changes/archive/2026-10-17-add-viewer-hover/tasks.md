## 1. What a card says

- [x] 1.1 Add a module in `packages/viewer/src/` that, given the tree and a node, answers the card's signature and doc in the editor's shapes: a declaration (`component Shape extends Node`, `type`, `action`, an element function), a property (`(property) name?: Type = default` and its doc), a union case (`(case) Union.case` and its doc), a reference or call (`name: Type` and its declaration's doc), or nothing; verify with unit tests over fixtures in `test/fixtures/sources.json` for each, a property inherited from a base, an optional one, and a literal and a comment that answer nothing
- [x] 1.2 Add the built-in Markdown renderer: paragraphs, inline and fenced code, emphasis, lists, and links only to `http:`/`https:` with `rel="noopener noreferrer"` and a new tab, building nodes and text only; verify with tests that render each construct, and that raw HTML (`<img src=x onerror=…>`) stays text and a `javascript:` link is not a link

## 2. Showing it

- [x] 2.1 In `src/source.ts`, mark hover targets as the reading draws them: an element's name and icon (its declaration), a row's name and a line's `name=` (the property), an id or title shown without its name (its property), and values (a case, a reference or a call); drop the icon's `title`; verify with tests that the marks sit on those spans and not on a unit's room, and that existing reading tests still pass
- [x] 2.2 In `src/index.ts` and `src/styles.ts`, show the card: open after 500 ms at rest on a target, keep it open while the pointer moves onto it (300 ms grace), close on leaving both, scroll, Escape, or a text or tree change; place it below its target or above where there is no room, clamped to the element's box, in the reading's faces and scheme; never change the selection, folds or scroll; verify with tests using mock timers and faked geometry for the scenarios "An element's declaration", "Moving onto the card" and "Nothing to say"
- [x] 2.3 Add the `i` key for the selected unit's card, its text sent to the announcer, Escape closing it; verify with the "From the keyboard" scenario test
- [x] 2.4 Add the `markdown` property (a function from a doc's text to a `Node`), honored when set before the element is defined like the others; verify with the "A host's Markdown" scenario test

## 3. Docs and checks

- [x] 3.1 Document hover, the `i` key and `markdown` in `packages/viewer/README.md`, and add the viewer's hover to `src/vscode/CHANGELOG.md`'s next release; verify `pnpm --filter @nx-lang/viewer test`
- [x] 3.2 Check it in the playground's Visual view on the survey example (an element, a property, a case, an id), by screenshot, and that the playground's tests pass
- [x] 3.3 Run `pnpm -r test` and `openspec validate add-viewer-hover --strict`, all passing
