---
name: openspecex-review
description: Review the implementation for an OpenSpec change and write findings to review.md. Use when the user asks to review an OpenSpec change.
license: MIT
compatibility: Requires openspec CLI.
metadata:
  author: openspecex
  version: "0.1.0"
  argument-hint: "[review-notes]"
---

Review the implementation for one OpenSpec change and write the findings to `review.md` in the change directory.

For stronger review independence, prefer running this with an AI agent different from the one that implemented the change.

**Input**: Extra instructions for the review (optional), and optionally the name of the change to review.

**Steps**

1. **Select the change**

   - Infer the change name from the conversation context if the user mentioned a change
   - Auto-select if only one active change exists
   - If ambiguous, run `openspec list --json` to get available changes and use the **AskUserQuestion tool** to let the user select

   Always announce: "Using change: <name>" and how to override (e.g., `/openspecex:review <other>`).

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

3. **Review Code Changes**

   Review the implementation according to these principles:

   - Start with files in the working tree and staged diff if they exist.
   - If instructions are provided in the input review-notes, use those to modify the scope of the review.
   - Do not make code changes in this skill.

   Review for issues like:

   - Behavioral bugs or regressions
   - Code that's duplicated and would be better shared
   - Code that could be improved or simpler if implemented differently
   - Requirement or scenario gaps
   - Task completion mismatches
   - Missing or weak test coverage for risky paths
   - Design divergence that affects correctness or maintainability

   Write the report to:

   ```text
   <changeDir>/review.md
   ```

   If there's an existing `review.md` file:
   - If the user's intent is already clear, follow it. Otherwise, ask whether they want to verify previously fixed findings or run another review pass.
   - If they want to verify previously fixed findings, stop this skill and tell the user to run `openspecex-review-verify` instead.
   - If they want another review pass, preserve the existing report and append only new findings under a header like `## New Findings Discovered During <local date/time in YYYY-MM-DD HH:MM format> Review`.
   - Continue `RF` numbering from the highest existing finding ID.

   ```markdown
   # Review: <change-name>

   ## Scope
   **Reviewed artifacts:** <list>  
   **Reviewed code:** <files covered>  

   ## Findings

   ### 🔴 Open - RF1 <one sentence issue>
   - **Severity:** <Low/Medium/High>
   - **Evidence:** <why this is a problem; use path:line references when available>
   - **Recommendation:** <specific fix>

   ### 🔴 Open - RF2 <one sentence issue>
   ...

   ## Questions
   - <question or "None">

   ## Summary
   - <short assessment>
   ```
   Rules for the report:

   - Use 4-state prefix format for findings:
     - `🔴 Open` - for new/unaddressed findings
     - `🟡 Fixed` - for findings that are fixed, but not verified
     - `✅ Verified` - for fixes that have been fully verified by the AI agent that reported the issue
     - `✅ Resolved` - for fixes that are resolved in some other way than fixing (e.g. deferred or not a bug)
   - Use stable finding IDs: `RF1`, `RF2`, `RF3`, ...
   - Keep findings actionable and concise.

4. **Present Results to the User**

   After writing `review.md`, summarize:

   - which change was reviewed
   - where `review.md` was written
   - how many findings were opened
   - any important unresolved questions
