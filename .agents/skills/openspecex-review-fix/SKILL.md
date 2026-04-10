---
name: openspecex-review-fix
description: Fix issues discovered by a review. Use when the user asks to fix review findings for an OpenSpec change.
license: MIT
compatibility: Requires openspec CLI.
metadata:
  author: openspecex
  version: "0.1.0"
  argument-hint: "[fix-notes]"
---

Read `review.md` for the current OpenSpec change, fix issues that are high confidence, and make notes for ambiguous or risky items.

**Input**: Extra instructions for the fixes (optional), and optionally the name of the OpenSpec change.

**Steps**

1. **Select the change**

   - Infer the change name from the conversation context if the user mentioned a change
   - Auto-select if only one active change exists
   - If ambiguous, run `openspec list --json` to get available changes and use the **AskUserQuestion tool** to let the user select

   Always announce: "Using change: <name>" and how to override (e.g., `/openspecex:review-fix <other>`).

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
   Then read `<changeDir>/review.md`. If `review.md` does not exist, stop and tell the user there is no review report to fix.

3. **Make Fixes**

   Use `review.md` as the primary work list.
   Treat findings marked `🔴 Open` as the default work list.

   Fix findings when all of these are true:

   - You agree that the finding is something that should be changed.
   - You have high confidence in the best change to address it.

4. **Update review.md**

   Preserve the existing text in the report, but update it to reflect what was handled.

   Use 4-state prefix format for findings:
   - `🔴 Open` - for new/unaddressed findings
   - `🟡 Fixed` - for findings that are fixed, but not verified
   - `✅ Verified` - for fixes that have been fully verified by the AI agent that reported the issue
   - `✅ Resolved` - for fixes that are resolved in some other way than fixing (e.g. deferred or not a bug)

   When you fix a finding:
   - Change its prefix from Open to Fixed, following the format above. Do NOT make the status Verified yet; the AI agent that reported the issue must verify it.
   - Append `- **Fix:** ` followed by a short note with the fix description.

   When you skip a finding:
   - Leave it open.
   - Append `- **Status:** ` followed by a short note explaining why it was not addressed and any recommendations.

5. **Present Results to the User**

   Report:

   - total count of items fixed and list of those items
   - total count of items left open and list of those items
