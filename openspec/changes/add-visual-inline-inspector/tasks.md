## 1. Language and library

- [x] 1.1 Derive `F.Props` (with `.Property` and `.Update`) for element-style functions in the compiler; tests for props order, defaults and rejection for paren-style functions
- [x] 1.2 `@nx/visualizer`: add `label`, `icon`, `short`, `group` to `PropertyVisualizer`; `summary` to `ElementVisualizer`; `Target`, `summary` and `properties` to `FunctionVisualizer`; doc comments
- [x] 1.3 Visualizer declarations: bind `Target` to `F.Props` for function visualizers; resolve the new hints like editor hints; `visualizer-hint` warnings for a bad `short` and repeated summary items; verify bare property names inside `summary={…}`
- [x] 1.4 Source tree: carry `summary` and per-property `label`, `icon`, `short`, `group` for records, actions, components and element functions; protocol types
- [x] 1.5 `@nx/graphics`: visualizers with labels, groups, icons and summaries for every element and helper; a test that every property resolves a label and group

## 2. Viewer

- [x] 2.1 Remove `compact`, the Details button, `opened` and the status line; trim `compact.ts` to pins and suppression; update state tests
- [x] 2.2 Row renderer for composition elements (icon, kind, id, title, summary) with whole-item overflow and `+N`; derived labels
- [x] 2.3 Inspector renderer: groups, two-column grid, leads (icon/short/caption), unset dashed fields, booleans, foldable groups per kind, tooltips
- [x] 2.4 Fields reuse the in-place editors, choices, colors, menus and invalid-entry notes; references as tokens with swatches; nested element values as unfolding groups
- [x] 2.5 Scrubbing on leads (threshold, step rate, modifiers, touch); delete steppers and their layout rules
- [x] 2.6 Selection opens, click/Enter on the selected row closes and reopens, pins; anchoring and off-screen reveal
- [x] 2.7 Keyboard and accessibility: tree roles, Tab into inspector, pin button, focus return, announcements
- [x] 2.8 Coverage test over the repository with every inspector open; unit tests for summaries, labels, groups, scrubbing and state

## 3. Playground

- [x] 3.1 Host state: `detailsState` with pins/suppression across Visual/Source and engines; reset on new document
- [x] 3.2 Examples: visualizers for the settings, survey and other examples so their rows and inspectors read well
- [x] 3.3 Browser checks (adapt `check-compact-browser.mjs`): selection, close/reopen, pins, scrubbing with one undo step, invalid entry, anchoring
- [x] 3.4 Docs: viewer README and the website's Visual and visualizer pages

## 4. Wrap-up

- [x] 4.1 `openspec validate add-visual-inline-inspector --strict`; independent review loop until high-confidence findings are fixed and verified
- [ ] 4.2 Screenshots for Bret; close PR #71
- [x] 4.3 Labels beside values, one field to a line; lists and unfolded element values under their label; no groups (Bret, 2026-11-03): remove `group` from `PropertyVisualizer`, the source tree, `@nx/graphics` and the inspector
- [x] 4.4 `if`/`for` among rows: lines start at the sibling icons, held rows level with siblings, quiet braces; State.Update reads with the action icon; UI text keeps its color where the highlighter reads a word as a keyword (Bret, 2026-11-03)
- [x] 4.5 One value names a row (Bret, 2026-11-03): title, else text content, else id, with no label; a click on it opens the inspector with that field ready to edit; remove `summary` and `short` from `@nx/visualizer`, the compiler's braced name lists and checks, the source tree, `@nx/graphics`, the examples and the docs
- [x] 4.6 No property icons (Bret, 2026-11-04): remove `icon` from `PropertyVisualizer`, the source tree, the viewer's field leads, `@nx/graphics`, the examples and the docs; element icons stay
- [x] 4.7 Inspector shows what the element sets and its title; a last `Show N unset` line reveals the rest, `Hide unset` hides them, forgotten on close (Bret, 2026-11-04)
- [x] 4.8 Colors in an inspector read in their parts, as in Figma (Bret, 2026-11-04): swatch, six-digit hex and opacity percent; the swatch opens the color picker, the hex and the opacity edit in place, dragging the `%` scrubs the opacity
- [x] 4.9 Short element values read their set properties on their field's line, as editable, scrubbable fields; the kind their property's type implies is dropped; `ElementVisualizer inline=true|false` overrides (Bret, 2026-11-04)
- [x] 4.10 Braced values keep their braces, quiet (Bret, 2026-11-04); a reference to a color leads with its swatch and its menu offers Go to, as the spec already said
- [x] 4.11 Quieter hover cards (Bret, 2026-11-04): first after 1 s, then at once while warm; none under a held button, after a press or a scroll until the pointer moves on; a row's card on its icon, not its name or value; the type's doc as a quiet line over the inspector; no browser tooltips that repeat a card
- [x] 4.12 Hover cards start just past what they describe (8 px past its right end, or 12 px past the pointer on a wide anchor such as a doc line), so they do not cover the labels below; slide left only to stay inside the box (Bret, 2026-11-04)
