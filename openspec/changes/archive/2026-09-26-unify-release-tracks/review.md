# Review: unify-release-tracks

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/package-release-automation/spec.md,
specs/vscode-extension-publishing/spec.md (against openspec/specs/* on `main`)  
**Reviewed code:** .github/workflows/release.yml, .github/workflows/package-publish.yml,
.github/workflows/build.yml, .github/workflows/vscode-extension.yml, deleted
.github/workflows/vscode-release.yml and .github/workflows/vscode-extension-publish.yml (from `main`),
tools/versions/Get-ReleaseVersion.ps1, CONTRIBUTING.md, docs/deployment.md, docs/deployment-setup.md,
src/vscode/CONTRIBUTING.md; cross-checked src/vscode/scripts/*.mjs, scripts/publish-packages.mjs,
tools/packaging/Test-NuGetPackageVersionAvailable.ps1, src/vscode/CHANGELOG.md  

Checks run: `openspec validate unify-release-tracks --strict` (valid), `actionlint` on the changed
workflows (clean), `Get-ReleaseVersion.ps1` for `v0.5.0` (every version 0.5.0), `vscode-v0.1.0`
(resolves to `main`, not a release), `main`, and a pull request (VSIX = core of the preview version).
Tasks 5.1 and 5.2 are intentionally left for the maintainer and are not findings.

## Findings

### ✅ Verified - RF1 The credentials scenario now says a missing extension secret stops publication to "any registry", but NuGet and npm publish in an independent job
- **Severity:** Medium
- **Evidence:** `openspec/changes/unify-release-tracks/specs/vscode-extension-publishing/spec.md:125`
  changes "MUST fail before publishing to either registry" to "MUST fail before publishing to any
  registry". Now that the workflow publishes to four registries, "any" includes NuGet and npm. But
  the design puts them in a separate job on purpose: `publish-packages`
  (`.github/workflows/package-publish.yml:63`) needs only `prepare`, and the credential check lives
  only in `publish-extension` (`package-publish.yml:293`). A missing `OVSX_PAT` or `AZURE_*` secret
  fails the extension job while NuGet and npm still publish. Archiving the change would write a
  requirement into the main spec that the implementation doesn't meet.
- **Recommendation:** Scope the scenario to the extension registries, for example "THEN the
  extension publish job MUST fail before publishing to the Visual Studio Marketplace or Open VSX".
  Keep the scenario heading as it is.
- **Fix:** The scenario now says "THEN the extension publish job MUST fail before publishing to the Visual Studio Marketplace or Open VSX"; the heading is unchanged. The proposal notes the scoping.
- **Verification:** Confirmed. `specs/vscode-extension-publishing/spec.md:125` now scopes the failure to the extension publish job and the Marketplace/Open VSX; the heading is unchanged. `proposal.md` mentions the scoping, and `openspec validate --strict` passes.

### ✅ Verified - RF2 proposal.md describes the spec deltas differently from how they are written
- **Severity:** Low
- **Evidence:** `openspec/changes/unify-release-tracks/proposal.md:41` says "VS Code tag does not
  create package release" is removed, but the delta keeps it as a MODIFIED scenario for the retired
  tag (`specs/package-release-automation/spec.md:88`). `proposal.md:43-47` says "VS Code extension
  versioned release trigger" changes and only its "Package release tag does not create VS Code
  release" scenario is removed. The delta actually removes that whole requirement and adds a new one,
  "VS Code extension is released with the NX release".
- **Recommendation:** Update the Modified Capabilities bullets to match the deltas: the
  `vscode-v*` scenario is kept and reworded, and the versioned-trigger requirement is replaced by
  the new ADDED requirement.
- **Fix:** The Modified Capabilities bullets in `proposal.md` now match the deltas: the `vscode-v*` scenario is kept and reworded, and "VS Code extension versioned release trigger" is removed with its scenario and replaced by the ADDED "VS Code extension is released with the NX release". They also mention the RF1 credentials scoping.
- **Verification:** Confirmed. `proposal.md:38-49` now matches the deltas: the `vscode-v*` scenario is kept and reworded, and the versioned-trigger requirement is removed with its scenario and replaced by the ADDED requirement.

### ✅ Verified - RF3 The unified runbook has no step for the extension's changelog, so the first unified release would ship without a 0.5.0 entry
- **Severity:** Low
- **Evidence:** `docs/deployment.md:55` ("Merge the release change to `main`") is now the only
  release runbook. It never mentions `src/vscode/CHANGELOG.md`. That step exists only in
  `src/vscode/CONTRIBUTING.md:102`, which a maintainer releasing the SDK may not read.
  `src/vscode/CHANGELOG.md:5` still has `0.1.0` as its newest entry. Both registries show this file
  as the extension's changelog, so the 0.1.0 to 0.5.0 jump (proposal "Users" impact) would go out
  without an entry or an explanation.
- **Recommendation:** Add a step to "Publish A Release" in `docs/deployment.md`: update
  `src/vscode/CHANGELOG.md` with a section for the release version. Mention in the first unified
  entry that the extension's version now follows the NX release version.
- **Fix:** Step 1 of "Publish A Release" in `docs/deployment.md` now says to include a section for the release version in `src/vscode/CHANGELOG.md`, which both registries show. The 0.5.0 entry itself belongs to the release change, not this one.
- **Verification:** Confirmed. Step 1 of "Publish A Release" (`docs/deployment.md:55-56`) now requires a `src/vscode/CHANGELOG.md` section for the release version. Writing the 0.5.0 entry itself is reasonably left to the release change.

### ✅ Verified - RF4 The release-asset download and manifest/checksum validation are duplicated between the two publish jobs
- **Severity:** Low
- **Evidence:** `.github/workflows/package-publish.yml:79-121` (`publish-packages`) and
  `package-publish.yml:244-276` (`publish-extension`) have the same download step and the same
  manifest/schema/tag/version and `sha256sum -c` block. Only the per-kind checks after them differ.
  Any change to the manifest format now has to be made in both places, which is the drift the design
  names as a reason to fold the workflows together.
- **Recommendation:** Move the shared part into a script, for example
  `tools/packaging/Test-ReleaseAssets.ps1 -Path <dir> -Tag -Version`, or a local composite action
  that downloads and validates. Each job then calls it and keeps only its own NuGet/npm or VSIX
  checks.
- **Fix:** The shared checks moved into `tools/packaging/Test-ReleaseAssets.ps1 -Path -Tag -Version`, which both publish jobs call before their own NuGet/npm or VSIX checks. The move also fixes a bug: the inline `sha256sum -c` never failed the step, because PowerShell doesn't stop on a native command's exit code and later `node`/`unzip` calls reset `$LASTEXITCODE`. The script now throws on a mismatch. Tested locally with a matching set, a wrong tag and a tampered asset. The download step stays in each job; it is a single `gh release download`.
- **Verification:** Confirmed. Both jobs call `tools/packaging/Test-ReleaseAssets.ps1` after checkout and before their own checks (`package-publish.yml`). I ran it locally: a matching set passes; a wrong tag, a tampered asset and a missing asset each throw. The added `$LASTEXITCODE` check is correct: a pwsh step only fails on the exit code of its last native command, so the old inline `sha256sum -c` could not fail the step.

### ✅ Verified - RF5 The VSIX job is a near-copy of the VS Code Extension workflow's job, and the target list now lives in three places
- **Severity:** Low
- **Evidence:** The `vsix` job in `.github/workflows/release.yml:234-317` repeats the
  `vscode-extension.yml` package job almost step for step, and has the same matrix
  (`release.yml:243-247`, `vscode-extension.yml:28-33`). The draft-release job hard-codes the
  targets a third time (`release.yml:378`). If a target is added to a matrix but not to the
  hard-coded check, its VSIX is attached and published without being required. If the two matrices
  drift apart, PR builds stop testing what releases ship.
- **Recommendation:** Share one VSIX build, for example as a `workflow_call` reusable workflow or a
  composite action that both workflows use with the target matrix defined once. Or at least have the
  draft-release check read the expected targets from the same place as the matrix.
- **Status:** Left open. Sharing one VSIX build between `vscode-extension.yml` (PR and `main` builds, with its own triggers, path filters and artifact names) and `release.yml` means a `workflow_call` refactor of the PR workflow, which the design lists as a non-goal ("the PR/`main` artifact builds stay as they are"). It can't be verified without CI runs. Recommend a follow-up change that extracts a reusable VSIX workflow with the target matrix as its one source, and has the draft-release check read the targets from it.
- **Fix:** Done in this change, at the maintainer's request. A new reusable workflow,
  `.github/workflows/vscode-vsix.yml`, holds the target matrix and the build steps once, with actions
  pinned to SHAs. `vscode-extension.yml` calls it for pull requests and `main`, and `release.yml`'s
  `vsix` job calls it with `release_tag`, which it checks out and stages as the VSIX version. Both
  now upload `vscode-vsix-<target>`, so `pr-artifact-comment.yml` and the runbooks need no change.
  The draft-release check no longer hard-codes targets: it downloads the artifacts without merging
  them and requires `nx-language-<target>-<version>.vsix` in each `vscode-vsix-<target>` directory.
  It was tested against a mock layout: a wrong-version VSIX fails, and a matching set passes.
  `design.md`, `proposal.md` and task 3.1 describe the shared workflow. `vscode-extension.yml`'s
  pull request paths also cover `vscode-vsix.yml` and `tools/versions/**`, which it now depends on.
  `actionlint` is clean on every workflow.
- **Verification:** Confirmed. `.github/workflows/vscode-vsix.yml` is now the only place the target matrix is defined. `vscode-extension.yml:27-28` and `release.yml:219-223` both call it, and the `vscode-vsix-<target>` artifact names are unchanged, so `pr-artifact-comment.yml`, which matches on the caller's "VS Code Extension" run and on artifact names, still works. Inside a called workflow the `github` context is the caller's, so `github.event.pull_request.number` and the auto-detected context are the same as in the old PR job. `ref: ''` makes `actions/checkout` use its default ref, as the old job did without a `ref`. A release build checks out and versions from `release_tag` with `fetch-depth: 0`. `download-artifact@v5` with `pattern` and no `merge-multiple` extracts each artifact into `<path>/<artifact-name>/`, which is the layout the check at `release.yml:284-294` reads. I replayed that check against mock layouts: a full set passes; a wrong-version VSIX, an empty artifact directory and no artifacts at all each throw. A target whose matrix leg fails never reaches the check, because `draft-release` needs `vsix`, and a failed leg fails that job. `actionlint` is clean on every workflow and `openspec validate --strict` passes. Side effect, not a finding: the PR check names become `package / Package (<target>)`. `main` has no branch protection or rulesets, so no required check depends on the old names.

### ✅ Verified - RF6 The moved VSIX job uses movable action tags and inherits `contents: write`
- **Severity:** Low
- **Evidence:** `.github/workflows/release.yml:257-313` uses `actions/checkout@v4`,
  `pnpm/action-setup@v4`, `actions/setup-node@v4`, `actions/upload-artifact@v4` and
  `dtolnay/rust-toolchain@stable`. It also inherits the workflow-level `contents: write`
  (`release.yml:25-26`) while it runs `pnpm install` and a Rust build. The same steps in
  `publish-extension` (`package-publish.yml:222-236`) and the other build jobs in `release.yml`
  (`:80`, `:93`, `:146`) are pinned to SHAs. The job was copied as-is from the deleted
  `vscode-release.yml`. The `editor-assets` job (`release.yml:179`, `:184`) has the same pre-existing
  gap.
- **Recommendation:** Pin the actions to the SHAs already used elsewhere in the repository. Give the
  `vsix` job (and the other build jobs) `permissions: contents: read`, so only `draft-release`
  writes.
- **Fix:** The `vsix` and `editor-assets` jobs now pin `actions/checkout`, `pnpm/action-setup`, `actions/setup-node` and `actions/upload-artifact` to the SHAs used elsewhere in the repository, and `dtolnay/rust-toolchain` to the current `stable` commit (`6bed076`). The workflow default is now `contents: read`; only `draft-release` keeps `contents: write` (with `actions: read`). The `setup-rust-node` composite action and `vscode-extension.yml` still use tags, which predates this change.
- **Verification:** Confirmed. Every action in `release.yml` except the local `setup-rust-node` is pinned to a SHA. The `dtolnay/rust-toolchain` SHA `6bed076` is the current head of `stable`, and its `action.yml` defaults `toolchain` to `stable`, so pinning keeps the same toolchain. The workflow default is `contents: read`; only `draft-release` has `contents: write` and `actions: read`. `actionlint` is clean.

### ✅ Verified - RF7 `release.yml` still wakes on `vscode-v*` tags, only to skip them through `should_release` gating on every job
- **Severity:** Low
- **Evidence:** The trigger `tags: ['v*']` (`release.yml:10`) matches `vscode-v*`. So the validate job
  keeps a retired-track branch (`release.yml:46-54`) and a `should_release` output, and six jobs
  repeat `if: ${{ needs.validate.outputs.should_release == 'true' }}` (`release.yml:71`, `:105`,
  `:158`, `:210`, `:238`, `:328`). Task 3.1 asked to keep the skip, but with one track left, all of
  that plumbing exists only to ignore a tag nobody should push.
- **Recommendation:** Narrow the trigger to `'v[0-9]+.[0-9]+.[0-9]+'`, which GitHub's filter syntax
  supports, so a stray `vscode-v*` tag starts no run. Then drop the skip branch, the `should_release`
  output and the six `if:` conditions. The validate job's regex still rejects a bad
  `workflow_dispatch` tag. Update the design's "keeps skipping it" wording to match.
- **Fix:** The trigger is now `'v[0-9]+.[0-9]+.[0-9]+'`, so a `vscode-v*` tag starts no run. The skip branch, the `should_release` output and the six `if:` gates are gone, and the validate regex still rejects a bad `workflow_dispatch` tag. `design.md` (decision and risk) and task 3.1 describe the narrowed trigger; task 4.2's grep expectation no longer names the skip. `actionlint` is clean.
- **Verification:** Confirmed. The trigger is `v[0-9]+.[0-9]+.[0-9]+` (`release.yml:11`), which GitHub's filter syntax supports and which does not match `vscode-v*`. No `should_release` or `should_publish` remains in `.github/workflows/`. The validate regex still rejects a bad dispatch tag, and `design.md:66`/`:92-93` and task 3.1 describe the narrowed trigger. `design.md:13-15` still describes the `v*` skip, but it is in the Context section, which describes the state before the change.

### ✅ Verified - RF8 The local repair commands in the runbook publish only the first of the three VSIX files
- **Severity:** Low
- **Evidence:** `docs/deployment.md:159-160` runs `pnpm run publish:vsce -- nx-language-*.vsix` (and
  the same for `publish:ovsx`). `src/vscode/scripts/publish-vsix.mjs:9` destructures a single
  `vsixPath` and silently ignores any further arguments. Every release carries three VSIX files, so
  this "emergency repair" leaves two platforms unpublished and reports no error. The bug predates this
  change, but the change rewrote this section of the runbook.
- **Recommendation:** Loop in the doc, as the workflow does
  (`for vsix in nx-language-*.vsix; do pnpm run publish:vsce -- "$vsix"; done`). Or have
  `publish-vsix.mjs` accept several paths, or reject extra arguments.
- **Fix:** The local repair in `docs/deployment.md` now loops over `nx-language-*.vsix` and calls `pnpm -C src/vscode run publish:vsce` and `publish:ovsx` once per file, with an absolute path.
- **Verification:** Confirmed. `docs/deployment.md:162-169` loops over the VSIX files and passes each one, with an absolute path, to `pnpm -C src/vscode run publish:vsce`/`publish:ovsx`. `publish-vsix.mjs` falls back to the absolute path when `join(packageRoot, path)` does not exist, so each file is published.

## Questions
- RF7 goes against task 3.1's "Keep the validate job's skip of `vscode-v*` tags". Is keeping a run
  for stray `vscode-v*` pushes intentional, for example so it is visible in the Actions tab, or can
  the trigger be narrowed?
  - **Answer:** Narrowed. The Actions tab doesn't need to show a run for a retired tag, and the
    runbook already says the track is retired. Task 3.1 is updated to match.

## Summary
- The implementation matches the design. One tag builds NuGet, npm and the three VSIX files into one
  draft titled `NX <version>`. `package-publish.yml` publishes them in two independent,
  idempotent jobs in `production`. `-Track` is gone and the version script behaves correctly on
  tag, PR, `main` and `vscode-v*` refs. The retired workflows are deleted and their references
  removed. `openspec validate --strict` and `actionlint` pass.
- The one finding that affects correctness is RF1, a spec scenario the two-job design deliberately
  does not satisfy. The rest are documentation gaps (RF2, RF3, RF8) and maintainability or hardening
  improvements (RF4 to RF7). None block the first unified release.
