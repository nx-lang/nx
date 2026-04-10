---
name: openspecex-review-verify
description: Verify fixes for findings in review.md and update the report with verification results. Use when the user asks to verify review fixes for an OpenSpec change.
license: MIT
compatibility: Requires openspec CLI.
metadata:
  author: openspecex
  version: "0.1.0"
  argument-hint: "[verify-notes]"
---

Verify the review fixes for one OpenSpec change, updating `review.md` in the change directory.

For stronger review independence and continuity, prefer running this with the same AI agent that performed the original review, and use a reviewer different from the agent that implemented the change.

**Input**: Extra instructions for the verification (optional), and optionally the name of the change to verify.

**Steps**

1. **Select the change**

   - Infer the change name from the conversation context if the user mentioned a change
   - Auto-select if only one active change exists
   - If ambiguous, run `openspec list --json` to get available changes and use the **AskUserQuestion tool** to let the user select

   Always announce: "Using change: <name>" and how to override (e.g., `/openspecex:review-verify <other>`).

2. **Load OpenSpec Context**

   Run:

   ```bash
   openspec status --change "<name>" --json
   openspec instructions apply --change "<name>" --json
   ```

   Notes:

   - `openspec status` is for schema and artifact availability, not task completion.
   - `openspec instructions apply` provides the change directory, context files, and task progress.
   - These commands may print a status line before the JSON payload. Treat the trailing JSON object as the real result.
   - `contextFiles.specs` may be a glob such as `specs/**/*.md`; expand it and read the matching files.

   Read every available context file returned by `openspec instructions apply`, including proposal, design, specs, and tasks when present.
   Then read `<changeDir>/review.md`. If `review.md` does not exist, stop and tell the user there is no review report to verify.

3. **Verify Fixes**

   - Use 4-state prefix format for findings in `review.md`:
     - `🔴 Open` - for new/unaddressed findings
     - `🟡 Fixed` - for findings that are fixed, but not verified
     - `✅ Verified` - for fixes that have been fully verified by the AI agent that reported the issue
     - `✅ Resolved` - for fixes that are resolved in some other way than fixing (e.g. deferred or not a bug)
   - Verify every finding marked as fixed (`🟡 Fixed`) in `review.md`, ensuring the fix is correct and complete.
   - Update `review.md` with verification results for each fixed finding:
     - Append `- **Verification:** ` followed by a short note describing the result
     - If the fix is satisfactory, update the prefix to `✅ Verified`
     - If the fix is incomplete or incorrect, change the prefix back to `🔴 Open` and explain what is still wrong in the `**Verification:**` note
   - If any new issues are found during verification, add them to `review.md` with a header like `## New Findings Discovered During <local date/time in YYYY-MM-DD HH:MM format> Verification`. Number them sequentially after the existing findings and use the same format.

4. **Present Results to the User**

   Report:

   - total count and list of findings verified as fixed
   - total count and list of findings reopened
   - total count and list of new findings added
