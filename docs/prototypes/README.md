# Visual UX interaction prototype

This is PR #71's prototype of a compact outline with Details and Pin buttons, kept as history. The
production design that replaced it is the inline inspector in `packages/viewer` (see
[Rows and inspectors](../../packages/viewer/README.md#rows-and-inspectors)), where selecting a row opens
its inspector, a pin keeps it open, and there is no Details button.

Open [visual-ux.html](visual-ux.html) directly in a browser. It is a self-contained HTML file with no
build step, network dependencies, or changes to the NX viewer. Alternatively, from the repository root:

```sh
python3 -m http.server 4178 --directory docs/prototypes
```

Then open <http://localhost:4178/visual-ux.html>.

The prototype explores suggestions 1–3 and part of suggestion 6 from
[Visual UX improvements](../visual-ux-improvements.md). It uses sample data and an SVG preview to make
the interactions reviewable before defining the production OpenSpec change.

## Interactions to try

1. Select Star, then Circle. Only the current selection's temporary details stay open.
2. Select Star, pin its properties, then select Circle. Both property sheets remain open.
3. Use Details to explicitly open another element's properties. These stay open across selection
   changes. Closing Details also removes that element's pin.
4. Open Drawing's details, then collapse and expand its children using the chevron. Child disclosure
   is independent of property disclosure. Collapsing an ancestor of the selected element moves
   selection to that ancestor. Descendant pins and explicit property expansion are retained.
5. Collapse the outline and click Star in Preview. Its ancestors expand, and its row is selected and
   revealed. Selecting a row also highlights the corresponding preview object.
6. Edit Star's coordinates or radius, or change Text's content. The sample preview updates on field
   change. Text, numeric, and color fields are editable; code-styled values are informational.
7. Switch to Card collection to try nested navigation and comparison with 18 cards. Reset or switching
   examples clears selection, disclosure choices, pins, and sample edits.
8. Use Tab to reach selection, child disclosure, Details, Pin, and inputs. Buttons respond to Enter
   and Space; preview objects can also be selected with Enter or Space.

## Deliberate prototype limits

- The sample data is independent of NX parsing, source editing, and runtime evaluation. Edits are
  local to this page and are not saved. There is no undo history.
- Summaries are mostly illustrative and do not recompute from edited fields. Path selection bounds
  are illustrative; card height is constrained to keep the example usable.
- The view redraws when selection or disclosure changes. Stable scrolling, editor continuity, and
  keyboard navigation beyond native controls need further design and implementation.
- Pins and disclosure choices use sample IDs. Mapping them across real source edits, distinguishing
  source declarations from rendered instances, and persistence across Visual/Source switches remain
  production decisions.
- Nested compound-property editors, source navigation, and full accessibility validation remain
  outside this prototype.

## Suggested process

Review the interaction with both examples before creating the OpenSpec change. Decide whether
selection should automatically open properties, whether explicit disclosure should persist separately
from pins, and what should happen when a selected descendant is collapsed.

Then capture those decisions in a proposal, design, scenarios, and implementation tasks. The complete
implementation should use the real viewer/source-tree pipeline and cover selection, folding,
editing, undo, keyboard behavior, accessibility, source changes, and host state persistence.
