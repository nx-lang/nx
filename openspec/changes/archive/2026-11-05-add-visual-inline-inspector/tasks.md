## 1. Language and library

- [x] 1.1 Derive `F.Props` (with `.Property` and `.Update`) for element-style functions in the compiler; tests for props order, defaults and rejection for paren-style functions
- [x] 1.2 `@nx/visualizer`: add `label` to `PropertyVisualizer`, `inline` to `ElementVisualizer`, and `Target` and `properties` to `FunctionVisualizer`; doc comments
- [x] 1.3 Visualizer declarations: bind `Target` to `F.Props` for function visualizers; resolve `label` and `inline` like editor hints
- [x] 1.4 Source tree: carry per-property `label` and `inline` for records, actions, components and element functions; protocol types
- [x] 1.5 `@nx/graphics`: visualizers with labels, in design-tool order, for every element and helper; element icons only; a test that every property resolves a label

## 2. Viewer

- [x] 2.1 Remove `compact`, the Details button, `opened` and the status line; trim `compact.ts` to pins and suppression; update state tests
- [x] 2.2 Rows for composition elements: icon, kind and one value that names the element (title, else text content, else id), cut and faded; a click on it opens its field; derived labels
- [x] 2.3 `if`/`for` among rows: lines start at the sibling icons, held rows level with siblings, quiet braces; `State.Update` values read with the action icon
- [x] 2.4 Inspector: the doc's first paragraph as a quiet line; one field to a line, labels beside values in a column up to 40%; lists and unfolded element values under their label; only set properties and the title, with a last `Show N unset` / `Hide unset` line forgotten on close; booleans as checkboxes
- [x] 2.5 Fields reuse the in-place editors, choices, menus and invalid-entry notes
- [x] 2.6 Colors in an inspector in their parts: swatch (opens the picker), six-digit hex and opacity percent edited in place, dragging `%` scrubs opacity
- [x] 2.7 Braced values keep quiet braces; references to a color `let` lead with its swatch and offer Go to; other expressions read as written and open in Source
- [x] 2.8 Short element values read their set properties on their field's line as editable, scrubbable fields, dropping the kind the property's type implies; `inline=true|false` overrides; other element values unfold into a nested group
- [x] 2.9 Scrubbing on labels (threshold, step rate, modifiers, touch); delete steppers and their layout rules
- [x] 2.10 Selection opens, click/Enter on the selected row closes and reopens, pins; anchoring and off-screen reveal
- [x] 2.11 Hover cards: first after 1 s, then at once while warm; none under a held button or after a press or scroll until the pointer moves on; a row's card on its icon; placed just past what they describe; an element's card titled by its name after a kind chip; no browser tooltips that repeat a card
- [x] 2.12 Keyboard and accessibility: tree roles, Tab into inspector, pin button, focus return, announcements
- [x] 2.13 Coverage test over the repository with every inspector open; unit tests for rows, labels, inspectors, colors, inline values, scrubbing, hover cards and state

## 3. Playground

- [x] 3.1 Host state: `detailsState` with pins/suppression across Visual/Source and engines; reset on new document
- [x] 3.2 Examples: visualizers for the settings, survey and other examples so their rows and inspectors read well
- [x] 3.3 Browser checks (adapt `check-compact-browser.mjs`): selection, close/reopen, pins, scrubbing with one undo step, invalid entry, anchoring; refresh the graphics walkthrough fixtures
- [x] 3.4 Docs: viewer README and the website's Visual and visualizer pages

## 4. Wrap-up

- [x] 4.1 `openspec validate add-visual-inline-inspector --strict`; independent review loop until high-confidence findings are fixed and verified
- [x] 4.2 Screenshots for Bret (/mnt/project-files/reviews/pr-72/)
- [x] 4.3 Close PR #71
