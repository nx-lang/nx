## 1. Disclosure model and public API

- [x] 1.1 Add composition-owner classification, including content under loops/conditions/sequences
  and exclusion of property-value subtrees; verify with nested source-tree fixtures and fallback cases.
- [x] 1.2 Implement explicit-open, pinned, suppressed, and selection-driven transitions; verify table
  tests cover selection within/across owners, close-selected-property, pin, unpin, and explicit-open cases.
- [x] 1.3 Expose compact, detailsState, NX_DETAILS_EVENT, and typed event detail with pre-definition
  property upgrade; verify defaults, restoration, event payloads, equivalent assignments, and no echo.

## 2. Compact source rendering

- [x] 2.1 Render stable composition headers with icon/kind/id/title or property count; verify pane
  resizing does not switch row structure and summaries do not evaluate expressions or duplicate nodes.
- [x] 2.2 Render open properties using existing readers and preserve nested compound values,
  comments, and logic; verify fully disclosed compact source coverage alongside existing reading tests.
- [x] 2.3 Add Details and Pin controls and status text; verify Star/Circle comparison, explicit-open
  persistence, unpin semantics, intentional closure, and read-only inspection through DOM tests.
- [x] 2.4 Separate content-child folding from details, retain run/declaration folding, and omit leaf
  chevrons; verify ancestor-collapse selection, properties on folded containers, and whole-outline commands.

## 3. Selection, editors, and accessibility

- [x] 3.1 Integrate selection-owner following and reveal of hidden properties/ancestors without event
  loops; verify host assignment, unchanged-selection closure, nested value selection, and arrow navigation.
- [x] 3.2 Keep existing property editors, unwritten-property affordances, item/row menus, and Show in
  Source reachable in compact mode; verify real source-edit requests and exact source ranges.
- [x] 3.3 Preserve live fields/caret across updates and settle/validate edits before hiding them;
  verify valid commit, invalid-draft retention, Escape cancellation, live changes, and source undo/redo.
- [x] 3.4 Implement native keyboard-operable controls, labels, expanded/pressed relationships,
  announcements, and focus restoration; verify keyboard-only comparison and cursor keys inside inputs.
- [x] 3.5 Anchor acted-on headers during disclosure and selection-following layout changes; verify
  in a browser with a large nested document, offscreen reveal, and viewport-boundary cases.

## 4. Source update reconciliation

- [x] 4.1 Retain state during temporary tree absence, prune invalid owners on complete updates, and
  notify the host of normalized state; verify no-op refreshes and removal of stale keys.
- [x] 4.2 Extend existing list-edit mapping to disclosure choices and pins; verify move/insert/delete
  with duplicate-looking items, undo/redo, and no transfer to an item inheriting a positional key.
- [x] 4.3 Detect ambiguous list membership edits in Source and clear affected subtree choices while
  preserving unaffected state; verify insertion, deletion, and reorder cases against previous tree/text.

## 5. Playground integration

- [x] 5.1 Enable compact presentation in ReadingView and wire detailsState/event handling into host
  state; verify NX on both engines uses the real tree and edits without requiring a running preview.
- [x] 5.2 Preserve disclosure state before selection restoration across Source/Visual and engine
  switches; verify unchanged round trips, suppressed details, no-op refreshes, and scroll restoration.
- [x] 5.3 Reset choices on preset/share/language changes and label commands Collapse outline and
  Expand outline; verify reset behavior, retained pins across outline commands, and unchanged share data.

## 6. Documentation and acceptance

- [x] 6.1 Document the new viewer API and playground interactions, link the prototype to the final
  behavior, and describe explicit-open versus pin semantics; verify all examples match the implemented API.
- [x] 6.2 Run viewer and playground tests/builds using their package scripts and record results;
  verify regular-mode coverage and existing editing, selection, folding, and undo regressions pass.
- [x] 6.3 Exercise finding, adjusting, and comparing siblings in drawing and large nested examples
  using pointer and keyboard at wide/narrow widths; record browser evidence for stable headers,
  visible focus, readable controls, and preserved active editing.
