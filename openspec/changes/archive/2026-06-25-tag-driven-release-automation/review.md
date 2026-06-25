# Review: tag-driven-release-automation

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/package-release-automation/spec.md,
specs/vscode-extension-publishing/spec.md, specs/editor-assets/spec.md

**Reviewed code:**
- `tools/versions/Get-ReleaseVersion.ps1`
- `.config/dotnet-tools.json`, `Directory.Build.props`, deleted `version.json`
- `.github/workflows/build.yml`, `.github/workflows/release.yml`, `.github/workflows/vscode-release.yml`
- `.github/workflows/package-publish.yml`, `.github/workflows/vscode-extension.yml`,
  `.github/workflows/vscode-extension-publish.yml`, `.github/workflows/pr-artifact-comment.yml`
- `src/vscode/scripts/package-language.mjs`, `src/vscode/scripts/stage-vsix-version.mjs`,
  `src/vscode/scripts/publish-vsix.mjs`
- `tools/packaging/Test-NuGetPackageVersionAvailable.ps1`
- `docs/deployment.md`, `CONTRIBUTING.md`

## Findings

### ✅ Verified - RF1 CONTRIBUTING.md still documents the removed NBGV / auto-publish release model
- **Severity:** Medium
- **Evidence:** [CONTRIBUTING.md:46-60](CONTRIBUTING.md#L46-L60) instructs maintainers to `nbgv tag`
  and links to Nerdbank.GitVersioning, then describes `release.yml` as a workflow that triggers on a
  *published* GitHub Release, "finds the most recent build.yml run", uploads `deployables`, and pushes
  to nuget.org when `NUGET_API_KEY` is set. NBGV has been removed (`.config/dotnet-tools.json`,
  `version.json` deleted) and `release.yml` is now a **tag-triggered draft-release** workflow
  ([release.yml:3-14](.github/workflows/release.yml#L3-L14)). The documented commands and flow no longer
  exist, directly contradicting the new model the change introduces. Tasks 6.1–6.5 updated
  `docs/deployment*.md` but missed this file.
- **Recommendation:** Rewrite the CONTRIBUTING.md "Releases" section to point at the new `v*` /
  `vscode-v*` tag → draft-release → publish flow (and link to `docs/deployment.md`), removing all
  `nbgv` references.
- **Fix:** Rewrote the CONTRIBUTING.md Releases section to document package `v*` and VS Code
  `vscode-v*` tags, draft GitHub Release review, publish-triggered production workflows, and links to
  the deployment setup/runbook docs.
- **Verification:** Confirmed [CONTRIBUTING.md:44-69](CONTRIBUTING.md#L44-L69). The Releases section
  now describes the tag-driven, artifact-only model with `git tag v1.2.3` / `git tag vscode-v1.2.3`
  examples, draft-release review, publish-to-trigger production, and links to `docs/deployment.md` and
  `docs/deployment-setup.md`. No `nbgv` reference remains anywhere in the repo.

### ✅ Verified - RF2 Forked-PR builds never receive an artifact comment
- **Severity:** Medium
- **Evidence:** [pr-artifact-comment.yml:32-41](.github/workflows/pr-artifact-comment.yml#L32-L41)
  derives the PR number only from `workflow_run.pull_requests[0]` and a runs API fallback, then exits
  early ("nothing to comment") when both are empty. For pull requests from forks, `pull_requests` is
  empty, so the workflow bails **before** reading the uploaded metadata artifact — even though that
  metadata already carries a validated `pullRequest` field
  ([build.yml:251](.github/workflows/build.yml#L251),
  [vscode-extension.yml:163](.github/workflows/vscode-extension.yml#L163)). The whole reason design.md
  chose the `workflow_run` pattern over `pull_request` was fork support
  ([design.md:74-82](openspec/changes/tag-driven-release-automation/design.md#L74-L82)), so forks
  silently getting no comment defeats that goal.
- **Recommendation:** Download/validate the metadata artifact first, then fall back to its
  `pullRequest` value when the event/API yields no PR number (the run_id is trusted, so the artifact's
  PR number can be trusted after the existing schema/repository/runId checks pass).
- **Fix:** Reordered PR comment handling so the metadata artifact is downloaded and validated before
  requiring a PR number, then uses the validated `pullRequest` metadata as the fallback for forked PR
  workflow runs.
- **Verification:** Confirmed [pr-artifact-comment.yml:34-99](.github/workflows/pr-artifact-comment.yml#L34-L99).
  The early bail-out was removed; the workflow now downloads/validates the metadata (schema +
  `repository` + `runId` against the trusted `workflow_run` id) before deriving the PR number, then
  falls back to `metadata_pr` when the event/API yields none (line 94-95) and cross-checks when both
  exist. Because `run_id` comes from the trusted event, trusting the artifact's `pullRequest` is safe.
  Fork PRs will now receive a comment.

### ✅ Verified - RF3 Package-publish has no npm trusted-publishing fallback or version guard, unlike NuGet
- **Severity:** Low
- **Evidence:** [package-publish.yml:182-225](.github/workflows/package-publish.yml#L182-L225) gives
  NuGet two auth paths (trusted publishing `NUGET_USER` **or** `NUGET_API_KEY` fallback), but npm is
  hard-gated: the "Check npm trusted publishing support" step fails the job when the runner's bundled
  npm is `< 11.5.1`, with no `npm install -g npm@latest` step and no token fallback. Because NuGet is
  pushed first, an old-npm runner fails *after* NuGet has already published, forcing a manual repair.
- **Recommendation:** Pin/upgrade npm explicitly (e.g. `npm install -g npm@latest` before the check)
  or document the runner npm requirement, so the publish job cannot half-complete due to a toolchain
  version drift.
- **Fix:** Added an explicit npm upgrade step and moved the trusted-publishing version check before
  NuGet authentication/publish steps, so npm toolchain drift fails before any package registry write.
- **Verification:** Confirmed both parts. [package-publish.yml:101-104](.github/workflows/package-publish.yml#L101-L104)
  adds an `npm install -g npm@latest` step after `setup-node`, and the "Check npm trusted publishing
  support" gate ([package-publish.yml:186-195](.github/workflows/package-publish.yml#L186-L195)) now
  runs before "Check NuGet authentication", "NuGet login", and "Publish NuGet package" (lines 196-224).
  An old/incompatible npm now fails the job before any NuGet write, eliminating the half-complete path.

### ✅ Verified - RF4 Dead/incorrect PR-number fallback env var in the version helper
- **Severity:** Low
- **Evidence:** [Get-ReleaseVersion.ps1:144-146](tools/versions/Get-ReleaseVersion.ps1#L144-L146)
  falls back to `$env:GITHUB_EVENT_NUMBER` for the PR number, but GitHub Actions does not set that
  variable (the PR number lives in the event payload / `github.event.pull_request.number`). It is
  harmless today only because every workflow passes `-PullRequestNumber` explicitly, but the fallback
  is effectively dead code and would silently yield `0` if ever relied upon.
- **Recommendation:** Remove the fallback or read the PR number from `$GITHUB_EVENT_PATH` so the helper
  behaves correctly when invoked without an explicit `-PullRequestNumber`.
- **Fix:** Replaced the nonexistent `$GITHUB_EVENT_NUMBER` fallback with `$GITHUB_EVENT_PATH` parsing
  for pull request events and made pull request version calculation fail when no PR number can be
  determined.
- **Verification:** Confirmed [Get-ReleaseVersion.ps1:104-124](tools/versions/Get-ReleaseVersion.ps1#L104-L124)
  adds `Get-PullRequestNumberFromEvent`, which reads `pull_request.number` (or `event.number` for PR
  events) from `$GITHUB_EVENT_PATH`, and that it is now the fallback at line 166-168. The
  `pull_request` context throws when no PR number is resolvable
  ([Get-ReleaseVersion.ps1:170-173](tools/versions/Get-ReleaseVersion.ps1#L170-L173)) instead of
  silently producing `0`. The bogus `$env:GITHUB_EVENT_NUMBER` reference is gone.

## Questions
- RF2 was fixed for forked PRs by falling back to validated metadata, so no scope decision is needed
  for this fix pass.

## Summary
The implementation is thorough and closely matches the specs: MinVer replaces NBGV cleanly, build/PR
workflows are artifact-only with no registry credentials, tag tracks are separated with both trigger
filters and runtime regex guards, draft releases carry manifest+checksum assets, and publish workflows
re-validate assets and preserve idempotent (NuGet `--skip-duplicate`, npm/VSIX existence-check) retries
from GitHub Release assets. No correctness-blocking bugs were found in the release/publish paths. All
four reported findings (RF1–RF4) have been fixed and verified. No new issues were found during
verification. The change is ready to archive.
