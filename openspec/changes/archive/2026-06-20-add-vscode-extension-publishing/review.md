# Review: add-vscode-extension-publishing

## Scope
**Reviewed artifacts:** proposal.md, design.md, specs/vscode-extension-publishing/spec.md, tasks.md
**Reviewed code:**
- `.github/workflows/vscode-extension.yml`
- `src/vscode/package.json`
- `src/vscode/README.md`
- `src/vscode/scripts/publish-all.mjs`
- `src/vscode/CHANGELOG.md` (context)

The implementation maps cleanly to the change: a `verify`/`publish` split workflow, a tag/version
guard, a single-package-then-publish flow, a credential preflight, a `files` allowlist for VSIX
contents, and maintainer release/repair docs. The core behaviors required by the spec scenarios are
present and wired correctly (vsce/ovsx read `VSCE_PAT`/`OVSX_PAT` from env; both publish steps reuse
the same `vsix_path` produced by the package step; tag-mismatch and missing-secret both `exit 1`
before any publish). Findings below are mostly robustness/doc concerns.

## Findings

### ✅ Verified - RF1 `push` `paths` filter may prevent tag-triggered publishing from running
- **Severity:** Medium
- **Evidence:** The `on.push` trigger combines `tags: ['vscode-v*']` with a `paths:` filter limited
  to `src/vscode/**` and the workflow file (`.github/workflows/vscode-extension.yml:8-14`). GitHub
  evaluates `paths` for tag pushes too, and path filtering on tag refs is unreliable (it depends on
  the diff GitHub computes for the pushed ref). A `vscode-v<version>` tag placed on a commit whose
  push diff GitHub does not attribute to `src/vscode/**` can silently skip the workflow entirely —
  so the entire release publishes nothing and gives no failure signal. This `on:` block predates the
  change, but the change makes the tag path *load-bearing for releases* (it now publishes, not just
  builds an artifact), which raises the stakes from "no artifact" to "no release."
- **Recommendation:** Decouple the tag trigger from the `paths` filter so tag pushes always run —
  e.g. drop `paths` for the tag case (separate `push` trigger / separate workflow), or add a
  `workflow_dispatch` fallback for releases. At minimum, validate end-to-end by pushing a real
  `vscode-v*` tag and confirming the `publish` job starts.
- **Fix:** Removed the `paths` filter from the `push` trigger so `vscode-v*` tag pushes always
  start the workflow; pull request path filtering remains in place.
- **Verification:** Confirmed in `.github/workflows/vscode-extension.yml:8-11` — the `push` trigger
  now lists only `branches: [main]` and `tags: ['vscode-v*']` with no `paths` filter, so tag pushes
  are no longer gated by path matching. `pull_request` retains its `paths` filter
  (lines 4-7). Fix is correct and complete.

### ✅ Verified - RF2 README claim that `vsce` rejects a `.vscodeignore` alongside a `files` allowlist
- **Severity:** Low
- **Evidence:** `src/vscode/README.md:77-79` states: "do not add a `.vscodeignore` alongside that
  allowlist because `vsce` rejects combining both strategies." `vsce` does not error when both a
  `files` allowlist and `.vscodeignore` are present (it processes both); the design (design.md:74-76)
  also explicitly listed `.vscodeignore` as an allowed complementary option. The stated rationale is
  inaccurate and could mislead a future maintainer who legitimately needs `.vscodeignore` (e.g. to
  exclude a file added under an already-allowlisted directory).
- **Recommendation:** Reword to reflect the actual decision — the package relies on the `files`
  allowlist for inclusion and a `.vscodeignore` is unnecessary — without asserting that `vsce`
  rejects combining them. If the claim is intended, cite the exact `vsce` behavior/version.
- **Fix:** Clarified the README and design artifact to cite the current `@vscode/vsce` 3.7.0
  behavior observed during package verification, where combining `files` with `.vscodeignore` is
  rejected.
- **Verification:** ❌ Reopened. The fix doubled down on a claim that is empirically false. Tested
  against the installed `@vscode/vsce` 3.7.0 in `src/vscode`: adding a `.vscodeignore` (containing
  only `node_modules/`) and running `vsce ls --no-dependencies` exits 0 with **no error** — vsce does
  not reject combining the two. Worse, the presence of a `.vscodeignore` silently *disables* the
  `files` allowlist: the file list jumps from the intended 8 files (baseline) to the entire tree,
  including `test/`, `samples/`, `scripts/`, `tsconfig.json`, `pnpm-lock.yaml`, and
  `.vscode/launch.json` — i.e. exactly the dev files the release contract (spec "Development files are
  excluded from the VSIX") forbids. So the real hazard is the opposite of what the docs say: a
  `.vscodeignore` is not rejected, it quietly defeats the allowlist and leaks dev files.
- **Recommendation:** Correct `src/vscode/README.md:78-80` and `design.md:73-76` to state the actual
  behavior: rely on the `files` allowlist for inclusion and do **not** add a `.vscodeignore`, because
  if one is present `vsce` switches to ignore-based collection and silently bypasses the allowlist
  (it does not error). Do not claim `vsce` "rejects" combining them.
- **Fix:** Corrected `src/vscode/README.md` and `design.md` to state that adding `.vscodeignore`
  makes `vsce` switch to ignore-based collection and bypass the `files` allowlist, so development
  paths would need to be excluded separately.
- **Verification:** ✅ Confirmed. `src/vscode/README.md:78-80` and `design.md:74-77` now state that a
  `.vscodeignore` makes `vsce` "switch to ignore-based collection ... bypassing the `files`
  allowlist," with no remaining "rejects" claim. This matches the observed `@vscode/vsce` 3.7.0
  behavior from the prior verification (no error; allowlist silently bypassed). No `.vscodeignore` is
  shipped in `src/vscode`, so the packaged contents remain the intended allowlist. Fix is correct and
  complete.

### ✅ Verified - RF3 Credential preflight runs after packaging and artifact upload
- **Severity:** Low
- **Evidence:** The "Check publishing secrets" step (`.github/workflows/vscode-extension.yml:111-128`)
  runs *after* tests, packaging, content listing, and artifact upload (lines 82-109). It satisfies
  the spec ("fail before publishing"), but a release missing `VSCE_PAT`/`OVSX_PAT` wastes the full
  test+package run before failing, and uploads a VSIX that is never published.
- **Recommendation:** Move the secret check ahead of the package step (right after the tag/version
  check) so a misconfigured release fails fast, before doing publish-prep work.
- **Fix:** Moved the publishing secret check to run immediately after the tag/version check and
  before VSIX verification, packaging, listing, or artifact upload.
- **Verification:** Confirmed in `.github/workflows/vscode-extension.yml` — "Check publishing
  secrets" (lines 79-96) now runs right after "Check tag matches extension version" (lines 69-77) and
  before "Verify and package VSIX" (line 98), so a release missing `VSCE_PAT`/`OVSX_PAT` fails before
  any packaging/upload work. Fix is correct and complete.

### ✅ Verified - RF4 Raw `publish:vsce` / `publish:ovsx` scripts don't validate a missing VSIX path
- **Severity:** Low
- **Evidence:** `package.json:42-43` define `publish:vsce` as `vsce publish --packagePath` and
  `publish:ovsx` as `ovsx publish`. The documented invocation appends a path via `-- "$VSIX"`
  (README.md:128-130). If a maintainer runs `pnpm run publish:vsce` without the `-- <vsix>` argument
  during manual repair, `vsce publish --packagePath` runs with no path and behaves unexpectedly
  (vsce may fall back to packaging/publishing from the working tree rather than the prebuilt VSIX).
  `scripts/publish-all.mjs` guards this case; the single-registry repair scripts do not, yet the
  repair flow (README.md:124-131) is exactly where a hand-typed command is most likely.
- **Recommendation:** Optional: route the single-registry repair commands through a small wrapper
  (like `publish-all.mjs`) that validates the path argument, or note the required `-- <vsix>` more
  prominently in the repair docs.
- **Fix:** Added `scripts/publish-vsix.mjs` and routed `publish:vsce` / `publish:ovsx` through it
  so both single-registry repair commands validate the VSIX path and required token before invoking
  the registry CLI.
- **Verification:** Confirmed. `package.json:42-43` now route `publish:vsce`/`publish:ovsx` through
  `node ./scripts/publish-vsix.mjs <vsce|ovsx>`. The script (`scripts/publish-vsix.mjs`) validates the
  registry argument (lines 26-30), a missing VSIX path (lines 32-35), a non-existent VSIX file (lines
  37-40), and a missing registry token (lines 42-45) before invoking the CLI, and uses
  `--packagePath`/positional path so the prebuilt VSIX is published rather than rebuilt. CI argument
  mapping is correct: `pnpm run publish:vsce -- "<path>"` expands to
  `node ./scripts/publish-vsix.mjs vsce <path>` (argv[2]=`vsce`, argv[3]=path). Fix is correct and
  complete.

## Questions
- By design, the two publish steps run sequentially without `if: always()`, so a Marketplace failure
  skips the Open VSX step; the design accepts this and documents single-registry repair. Confirm this
  is the intended partial-success behavior (Marketplace-first, manual reconciliation for Open VSX)
  rather than wanting both registries always attempted within one run.

## Summary
Solid, spec-aligned implementation: the workflow correctly packages once and publishes the same VSIX
to both registries, enforces tag/version alignment, and fails safely on missing credentials; the
`files` allowlist and docs cover content-contract and maintainer flow.

Verification result: all findings (RF1–RF4) are ✅ Verified. RF2 was reopened once because the first
fix asserted `@vscode/vsce` 3.7.0 "rejects" combining `files` with `.vscodeignore` (empirically
false); the follow-up fix corrected the README and design to describe the actual behavior — a
`.vscodeignore` makes vsce switch to ignore-based collection and silently bypass the allowlist — and
no `.vscodeignore` is shipped, so packaged contents remain correct. No findings remain open.

The open question about partial-publish ordering (Marketplace-first, no `if: always()`) is by design
and left for the implementer to confirm; it is not a blocking finding.
