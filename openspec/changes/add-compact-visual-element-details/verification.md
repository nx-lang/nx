# Implementation verification

Implemented on `codex/compact-visual-element-details`, 2026-11-02.

## Automated checks

| Check | Result |
| --- | --- |
| `pnpm --filter @nx-lang/viewer test` | 243 tests passed |
| `pnpm --filter @nx-lang/playground test` | 156 tests passed; all 26 example outputs matched |
| `pnpm --filter @nx-lang/playground typecheck` | Passed |
| `pnpm --filter @nx-lang/playground build` | Passed; Vite reported a chunk-size advisory |
| `openspec validate add-compact-visual-element-details --strict` | Passed |
| `git diff --check` | Passed |

The viewer coverage suite now renders the repository corpus in regular and fully disclosed compact
presentation, both with and without forced wrapping, checking every source key appears exactly once.
Focused tests cover state transitions, pre-definition property upgrades, host restoration without
event echo, nested property ownership, independent folds, invalid drafts, missing trees, known item
moves/deletion, ambiguous duplicate items, and source comments during reconciliation.

## Browser acceptance

The repeatable `test:compact-browser` script launches the actual playground and uses Chromium against
the gradients and survey examples. It verifies:

1. Selecting Star and Circle, with Star pinned, keeps both details visible.
2. Keyboard disclosure exposes the expected state and retains control focus.
3. Editing the pinned Star changes actual NX source while Circle remains inspected.
4. An invalid draft prevents closing its details or leaving Visual; Escape cancels it.
5. Source/Visual restoration retains pins, and Undo reverts source edits independently of pins.
6. Drawing child disclosure leaves its properties visible and retains descendant pins.
7. Expanding the survey outline exposes a larger real document; selecting a visible lower element
   preserves its header position within a 3px tolerance.
8. Pin controls retain visible keyboard focus at 1000px and 720px viewport widths.
9. Choosing another example resets pins and explicit property openings.
10. The exercised pages emit no JavaScript errors.

Run with an installed Chromium executable:

```sh
CHROME_PATH=/path/to/chromium pnpm --filter @nx-lang/playground test:compact-browser
```

`NX_COMPACT_SCREENSHOT=/absolute/path.png` optionally saves a browser screenshot. The development
playground was also inspected in dark mode with the real gradient drawing and simultaneous details.

This checkout has one NX playground backend and no engine/language selector. Cross-engine host
restoration uses the shared viewer state API; tests cover restoring that API and replacing its tree.
The separate DrawnUI application can opt into compact presentation when it consumes this viewer.

## Implementation notes

The reading stays mounted while Source is visible, preserving its previous tree for identity
reconciliation and its place. The source-tree hook still requests nothing while Source is active.
Host document resets remount the reading and clear its choices.

`finishEditing()` was added to let the host validate and settle an editor before hiding Visual;
the design, delta spec, and API documentation include that integration.

The standalone HTML prototype remains available as design reference. Production rendering and editing
use the existing source-tree, editor, and undo pipelines.
