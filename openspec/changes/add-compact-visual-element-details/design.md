## Context

See `proposal.md` for motivation. The reference interaction is `docs/prototypes/visual-ux.html`, with
limits documented in `docs/prototypes/README.md`.

`packages/viewer/src/source.ts` currently chooses line or group layout by measuring available width.
Group folding hides both properties and content. `index.ts` owns selection, folds, editing sessions,
scroll anchoring, and source-key remapping after list edits. `ReadingView.tsx` feeds source trees to
the viewer and forwards events; `App.tsx` keeps folds and selection across view switches and resets
them when a document changes. Hosts can temporarily clear the tree during updates.

## Goals / Non-Goals

**Goals:**
- Give composition elements a stable navigation row while reusing the existing property readers,
  editors, menus, source ranges, and edit queue.
- Make disclosure state explicit and host-restorable, with deterministic selection and focus behavior.
- Preserve source coverage and existing reading behavior outside the compact presentation.

**Non-Goals:**
- Runtime evaluation, new preview picking or rendered-instance mapping, a docked inspector, new NX
  syntax, custom property grouping, gradient editors, or a new summary-expression language.
- Reimplementing the prototype's SVG renderer or copying its sample data into the product.
- Persisting view state in shares, source text, browser storage, or across page reloads.

## Decisions

### 1. An opt-in presentation on the existing viewer

Add a boolean `compact` property, default false, with pre-definition upgrade support alongside the
other element properties. The playground sets it true for NX Visual on both engines. No additional
user-facing mode switch is introduced. This keeps the general-purpose reading usable while making
the intended authoring experience the playground default. A separate viewer would duplicate editing,
selection, and source mapping.

A composition element is an element outside a non-content property-value subtree. Content children,
including those inside conditions, loops, and sequences, qualify. Records used as paint, stroke, or
other property values retain the existing nested reader rather than becoming outline siblings.
Elements whose syntax cannot be represented losslessly keep their existing as-written fallback.
Comments and logic remain in source order.

Representable composition elements always use a header and optional inline property rows, regardless
of width. Headers show the existing icon, kind, id, and literal title when available; otherwise a
quiet property count. Never synthesize evaluated summaries. Long identity text truncates visually
with its full accessible text available; property values wrap within the details area. A title or id
shown in the header is not duplicated as a second keyed source node in the details.

### 2. Child folding and detail disclosure are separate

Reuse `folded` and `nx-fold` for child-content disclosure on compact composition elements. Leaf
elements have no child chevron. Properties remain visible when their owner folds its children.
Existing run/declaration folding and default supporting-declaration folds remain in effect.
`collapseAll`/`expandAll` affect child and declaration folds, not details or pins; the playground labels
these commands Collapse outline and Expand outline.

Collapsing an element over a selected descendant selects that element, without immediately reopening
its children. A selected property of that element remains selected when only its content is hidden.
Hidden descendants retain their disclosure choices. Selection into a folded descendant from a host
or source navigation reveals the necessary ancestors using the existing selection contract.

### 3. A small disclosure state model

Expose `detailsState` with `opened: readonly string[]`, `pinned: readonly string[]`, and optional
`suppressed: string` (the currently selected composition owner's key). Undefined means empty choices.
Expose `NX_DETAILS_EVENT = "nx-details"`, carrying the complete normalized state and resulting
selection. Programmatic state assignment does not echo events. Reader actions emit once after
selection has settled; assigning equivalent state is a no-op. Selection continues to use `nx-select`;
disclosure never masquerades as an edit or a fold.

Resolve the selection's composition owner by walking through any property-value subtree to its
composition element. The visible details formula is:

```text
pinned(owner) OR opened(owner) OR
  (selectedCompositionOwner == owner AND suppressed != owner)
```

| Action | State transition |
| --- | --- |
| Select another composition owner | Clear suppression; temporary details follow the new owner |
| Move selection within the same owner | Keep suppression unless revealing an explicitly targeted hidden property |
| Open Details | Add owner to opened; clear its suppression; keep selection |
| Close Details | Remove owner from opened and pinned; suppress it if selected |
| Pin | Add owner to pinned; clear its suppression; keep selection |
| Unpin | Remove only its pin; existing explicit-open and selection reasons still apply |

Pinning does not implicitly create an explicit-open entry. Explicit disclosure preserves intentional
workspace setup; pinning communicates comparison intent and can later support pin management without
changing selection. The alternative of making Pin the only persistent-open action is simpler, but
would discard the prototype's explicit-open behavior. Status text distinguishes Following selection,
Kept open, and Pinned, with Pinned taking precedence.

Closing details containing the selected property moves selection to the owner and suppresses its
automatic details. A host explicitly selecting a hidden property clears suppression for its owner
and reveals the property, including any nested value folds. Reassigning the same selection alone
does not reopen intentionally closed details. Opening properties never changes child folds.

### 4. State follows the document lifecycle

Keep `detailsState` in the playground host alongside folds. Restore it before applying selection to
a newly mounted viewer. Source/Visual switches and engine switches preserve choices for the same
document; a preset/share/language change resets them. The state API is also usable by other hosts.

Retain state through a temporary `tree = undefined`. With a complete replacement tree, retain valid
element keys and prune invalid ones. Reuse existing list-edit mapping for viewer-initiated reorder,
insert, delete, and their undo/redo selection restoration. Never keep a pin on a different list item
just because it inherited a positional key. If arbitrary Source edits cannot be mapped unambiguously,
clear choices for the affected list subtree rather than guessing; unchanged regions keep their state.
Carry the previous tree and text through the host update long enough to detect that ambiguity.
Deleted state is not resurrected by undo; the normal undo-selected property is revealed temporarily.
Emit normalized state when reconciliation changes choices so the host cannot restore stale entries.

### 5. Integrate with the existing editing and navigation paths

Derive compact layout and disclosure in the source renderer; keep interaction state in the viewer.
The renderer must not copy source nodes into summaries, duplicate keyed DOM, or evaluate expressions.
Use existing property rows and editors for explicit and unwritten properties, menus, source opening,
live updates, diagnostics, and undo. The unwritten-property affordance belongs within open details;
an element's item actions remain reachable on its compact header.

Expose `finishEditing(): boolean` so the host can settle editing before hiding Visual.
Before an action hides an active field, use its normal commit/validation path. An invalid draft keeps
its editor visible and reports the problem rather than disappearing. Escape retains existing cancel
behavior. Disclosure changes do not enter the source undo stack or re-run the snippet. Live edits and
tree refreshes preserve the active field, caret, and selection without opening unrelated details.

Do not use the prototype's full redraw on every click as the production strategy. Reconcile the
affected details or restore editor state through existing mechanisms. Anchor the acted-on header
during local disclosure and selection-following transitions; if the prior header was offscreen,
reveal the new selection with minimal scrolling. Avoid repeated selection/render/event cycles.

### 6. Native controls with explicit keyboard and focus rules

Use real buttons for child disclosure, Details, and Pin, with element-specific accessible names,
expanded/control relationships, pressed pin state, visible keyboard focus, and no reliance on hover.
Preserve existing outline arrow navigation: left/right first collapse/expand child content, then
navigate; they do not toggle properties. Details and Pin are reachable with Tab and activated with
Enter/Space. Do not intercept cursor keys inside fields. Return focus to the corresponding control
after disclosure, or to the owner header if a removed details area held focus. Announce selection
and pin/disclosure changes concisely. Read-only source still allows inspection and pinning.

## Risks / Trade-offs

- [Two persistent-open mechanisms can confuse users] → Distinguish their status and labels; verify
  explicit-open/unpin cases in the prototype and implementation acceptance checks.
- [Selection-driven expansion shifts siblings] → Anchor the selected header and test large nested
  documents, property editing, and repeated keyboard navigation in a real browser.
- [Positional source keys can transfer state to the wrong item] → Map known viewer edits; clear
  ambiguous list-subtree state after arbitrary source changes and test duplicate-looking items.
- [Compact layout could hide syntax or duplicate title/id nodes] → Keep the as-written fallback;
  run existing coverage tests in regular mode and fully disclosed compact mode.
- [Rendering changes disrupt live editors or cause event loops] → Test active fields, stale-tree
  refresh, host echo, undo, and no-op updates before enabling compact mode in the playground.

## Migration Plan

Implement and test the viewer opt-in first, then wire host state and enable it in the playground.
There is no stored-data migration. Retain the prototype as a design reference and update its README
to point to the implemented interaction. Rolling back the playground opt-in returns to the existing
reading; the source and undo history are independent of this view state.
