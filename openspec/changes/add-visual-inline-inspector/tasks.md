## 1. Language and library

- [ ] 1.1 Derive `F.Props` (with `.Property` and `.Update`) for element-style functions in the compiler; tests for props order, defaults and rejection for paren-style functions
- [ ] 1.2 `@nx/visualizer`: add `label`, `icon`, `short`, `group` to `PropertyVisualizer`; `summary` to `ElementVisualizer`; `Target`, `summary` and `properties` to `FunctionVisualizer`; doc comments
- [ ] 1.3 Visualizer declarations: bind `Target` to `F.Props` for function visualizers; resolve the new hints like editor hints; `visualizer-hint` warnings for a bad `short` and repeated summary items; verify bare property names inside `summary={…}`
- [ ] 1.4 Source tree: carry `summary` and per-property `label`, `icon`, `short`, `group` for records, actions, components and element functions; protocol types
- [ ] 1.5 `@nx/graphics`: visualizers with labels, groups, icons and summaries for every element and helper; a test that every property resolves a label and group

## 2. Viewer

- [ ] 2.1 Remove `compact`, the Details button, `opened` and the status line; trim `compact.ts` to pins and suppression; update state tests
- [ ] 2.2 Row renderer for composition elements (icon, kind, id, title, summary) with whole-item overflow and `+N`; derived labels
- [ ] 2.3 Inspector renderer: groups, two-column grid, leads (icon/short/caption), unset dashed fields, booleans, foldable groups per kind, tooltips
- [ ] 2.4 Fields reuse the in-place editors, choices, colors, menus and invalid-entry notes; references as tokens with swatches; nested element values as unfolding groups
- [ ] 2.5 Scrubbing on leads (threshold, step rate, modifiers, touch); delete steppers and their layout rules
- [ ] 2.6 Selection opens, click/Enter on the selected row closes and reopens, pins; anchoring and off-screen reveal
- [ ] 2.7 Keyboard and accessibility: tree roles, Tab into inspector, pin button, focus return, announcements
- [ ] 2.8 Coverage test over the repository with every inspector open; unit tests for summaries, labels, groups, scrubbing and state

## 3. Playground

- [ ] 3.1 Host state: `detailsState` with pins/suppression across Visual/Source and engines; reset on new document
- [ ] 3.2 Examples: visualizers for the settings, survey and other examples so their rows and inspectors read well
- [ ] 3.3 Browser checks (adapt `check-compact-browser.mjs`): selection, close/reopen, pins, scrubbing with one undo step, invalid entry, anchoring
- [ ] 3.4 Docs: viewer README and the website's Visual and visualizer pages

## 4. Wrap-up

- [ ] 4.1 `openspec validate add-visual-inline-inspector --strict`; independent review loop until high-confidence findings are fixed and verified
- [ ] 4.2 Screenshots for Bret; close PR #71
