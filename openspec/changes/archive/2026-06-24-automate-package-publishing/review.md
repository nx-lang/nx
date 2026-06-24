# Review: automate-package-publishing

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/package-release-automation/spec.md, specs/vscode-extension-publishing/spec.md, specs/editor-assets/spec.md, specs/dotnet-binding/spec.md

**Reviewed code:**
- [.github/workflows/build.yml](../../../.github/workflows/build.yml)
- [.github/workflows/vscode-extension.yml](../../../.github/workflows/vscode-extension.yml)
- [src/vscode/scripts/stage-vsix-version.mjs](../../../src/vscode/scripts/stage-vsix-version.mjs)
- [src/vscode/scripts/check-vsix-version.mjs](../../../src/vscode/scripts/check-vsix-version.mjs)
- [src/vscode/package.json](../../../src/vscode/package.json)
- [tools/packaging/Test-NxRuntimePackage.ps1](../../../tools/packaging/Test-NxRuntimePackage.ps1)
- [tools/packaging/Test-NuGetPackageVersionAvailable.ps1](../../../tools/packaging/Test-NuGetPackageVersionAvailable.ps1)
- [bindings/dotnet/src/NxLang.Runtime/NxLang.Runtime.csproj](../../../bindings/dotnet/src/NxLang.Runtime/NxLang.Runtime.csproj)
- [docs/deployment-setup.md](../../../docs/deployment-setup.md), [docs/deployment.md](../../../docs/deployment.md)
- [bindings/dotnet/README.md](../../../bindings/dotnet/README.md), [src/vscode/README.md](../../../src/vscode/README.md)

## Findings

### ✅ Verified - RF1 Preview-publish toggle documented as `PUBLISH_PREVIEW_PACKAGES` is never read by the workflow
- **Severity:** Medium
- **Evidence:** [docs/deployment-setup.md:59](../../../docs/deployment-setup.md#L59) and [docs/deployment.md:78](../../../docs/deployment.md#L78) tell maintainers to set `PUBLISH_PREVIEW_PACKAGES=true` to enable trusted preview publishing. The actual `publish-preview` job is gated only on `github.event_name == 'workflow_dispatch' && inputs.publish_preview` ([.github/workflows/build.yml:206](../../../.github/workflows/build.yml#L206)). `PUBLISH_PREVIEW_PACKAGES` is referenced nowhere in any workflow, so following the documented setup has no effect, and the real toggle (a manual-dispatch boolean input) is undocumented.
- **Recommendation:** Pick one mechanism. Either read `vars.PUBLISH_PREVIEW_PACKAGES` in the job `if:` (so the documented variable works), or update both docs to describe the `workflow_dispatch` `publish_preview` input as the actual toggle.
- **Fix:** The preview publish job now accepts either the manual `publish_preview` input or `vars.PUBLISH_PREVIEW_PACKAGES == 'true'`, and the setup/runbook docs describe both toggles.
- **Verification:** Confirmed. [build.yml:206](../../../.github/workflows/build.yml#L206) gates on `inputs.publish_preview || vars.PUBLISH_PREVIEW_PACKAGES == 'true'`; [docs/deployment-setup.md:58-59](../../../docs/deployment-setup.md#L58) and [docs/deployment.md:81-82](../../../docs/deployment.md#L81) document both the `publish_preview` dispatch input and the `PUBLISH_PREVIEW_PACKAGES` variable. Toggle and docs now agree.

### ✅ Verified - RF2 Tag-based VS Code publishing was removed but is still triggered and still documented as a repair path
- **Severity:** Medium
- **Evidence:** `vscode-v*` tag pushes still trigger the workflow ([.github/workflows/vscode-extension.yml:18-19](../../../.github/workflows/vscode-extension.yml#L18)) and the `package` job still runs the tag/version match check ([.github/workflows/vscode-extension.yml:72-82](../../../.github/workflows/vscode-extension.yml#L72)). But the `publish` job only runs for `push` to `refs/heads/main` or a `workflow_dispatch` repair; a tag push has ref `refs/tags/vscode-v*`, so it builds VSIX artifacts that are never published. Meanwhile [src/vscode/README.md](../../../src/vscode/README.md) still states "Manual tag or explicit-artifact publishing remains a repair path," which no longer holds.
- **Recommendation:** Decide whether tag publishing is supported. If not, drop the `vscode-v*` tag trigger and the now-dead tag-match step, and fix the README to describe only the `workflow_dispatch` `repair_run_id` path. If yes, extend the `publish` job `if:` to cover tag refs.
- **Fix:** Removed the `vscode-v*` tag trigger and dead tag/version check, and updated the VS Code README to describe `workflow_dispatch` with `repair_run_id` as the manual repair path.
- **Verification:** Confirmed. The `push:` trigger in [vscode-extension.yml:17-18](../../../.github/workflows/vscode-extension.yml#L17) is now `branches: [ main ]` only (no `tags:`), and the "Check tag matches extension version" step is gone — `Stage CI extension version` runs unconditionally. No remaining `vscode-v*` or tag-ref references in the workflow; README repair path matches.

### ✅ Verified - RF3 npm production publish has no pre-publish auth gate, unlike the NuGet path
- **Severity:** Low
- **Evidence:** The NuGet path explicitly detects trusted-publishing vs. fallback and errors before any registry write when neither is configured ([.github/workflows/build.yml:305-313](../../../.github/workflows/build.yml#L305)). The npm production step just runs `npm publish "$package" --access public` with `NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}` ([.github/workflows/build.yml:333-338](../../../.github/workflows/build.yml#L333)). If neither npm trusted publishing nor `NPM_TOKEN` is configured, it fails deep inside `npm publish` rather than failing fast with a clear message. The token-fallback requirement in [specs/package-release-automation/spec.md:64-68](specs/package-release-automation/spec.md#L64) ("CI SHALL fail before publication if the token is missing") and the symmetry implied by task 3.5 are only partially met. (Also note OIDC trusted publishing needs npm ≥ 11.5.1, which is not pinned/verified here.)
- **Recommendation:** Add an npm auth preflight mirroring the NuGet one (fail clearly when neither trusted publishing nor `NPM_TOKEN` is available), and consider asserting the npm version that supports OIDC.
- **Fix:** Updated production npm publishing to require trusted publishing with npm >= 11.5.1 and removed the `NPM_TOKEN` fallback from the workflow and documentation.
- **Verification:** Confirmed and consistent. The "Check npm trusted publishing support" step ([build.yml:329-345](../../../.github/workflows/build.yml#L329)) fails fast with a clear error when npm < 11.5.1, and the publish step no longer sets `NODE_AUTH_TOKEN`/`NPM_TOKEN`. The trusted-publishing-only decision was propagated cleanly to the artifacts: editor-assets spec scenarios now read "npm trusted publishing is configured" / "without an `NPM_TOKEN` fallback" ([specs/editor-assets/spec.md:19,34-37](specs/editor-assets/spec.md#L19)), tasks 1.5/3.5 updated, and [docs/deployment-setup.md:70-71](../../../docs/deployment-setup.md#L70) state production npm uses trusted publishing only. No spec↔impl divergence remains.

### ✅ Verified - RF4 New `build.yml` publish steps mix SHA-pinned and floating action references
- **Severity:** Low
- **Evidence:** The surrounding `build.yml` convention pins actions to a commit SHA (e.g. `actions/checkout@08c6903…`, `actions/download-artifact@634f93cb…`). The new publish jobs introduce floating tags `actions/setup-node@v4` ([.github/workflows/build.yml:329](../../../.github/workflows/build.yml#L329)) and `NuGet/login@v1` ([.github/workflows/build.yml:315](../../../.github/workflows/build.yml#L315)) — exactly the steps that hold `id-token: write` and registry push rights.
- **Recommendation:** Pin `setup-node` and `NuGet/login` to commit SHAs to match the rest of the file and reduce supply-chain risk on the credential-bearing jobs.
- **Fix:** Pinned production `NuGet/login` and `actions/setup-node` references in `build.yml` to the current `v1` and `v4` tag SHAs.
- **Verification:** Confirmed. `NuGet/login@ebc737b6fc418a6ca0073cf116ec8dc156d8b81e # v1` ([build.yml:315](../../../.github/workflows/build.yml#L315)) and `actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020 # v4` ([build.yml:327](../../../.github/workflows/build.yml#L327)) are now SHA-pinned, matching the file's convention on the credential-bearing job.

### ✅ Verified - RF5 Preview npm `.npmrc` auth line is malformed for typical registry URLs
- **Severity:** Low
- **Evidence:** [.github/workflows/build.yml:240](../../../.github/workflows/build.yml#L240) builds the auth line as `"${PREVIEW_NPM_REGISTRY#https:}/:_authToken=${NODE_AUTH_TOKEN}"`. For a registry URL with a trailing slash or path (e.g. `https://npm.pkg.github.com/`), stripping `https:` yields `//npm.pkg.github.com/` and appending `/:_authToken` produces `//npm.pkg.github.com//:_authToken=…` (double slash), which npm will not match to the publish registry, breaking auth.
- **Recommendation:** Normalize the trailing slash before composing the `_authToken` key (or use `npm config set //host/:_authToken`), and test against a real preview feed URL.
- **Fix:** Normalized the preview npm registry URL before writing the `.npmrc` auth key so trailing slashes and registry paths produce npm-compatible `//host/path/:_authToken` entries.
- **Verification:** Confirmed. [build.yml:251-254](../../../.github/workflows/build.yml#L251) strips the trailing slash (`%/`) and `https://`/`http://` scheme before composing `//${registry_key}/:_authToken=`, yielding a single-slash, scheme-less key for both bare hosts and pathed registries.

### ✅ Verified - RF6 Hard version pre-checks defeat the `--skip-duplicate` idempotent-retry behavior
- **Severity:** Low
- **Evidence:** Production NuGet/npm pushes use idempotent flags (`dotnet nuget push --skip-duplicate`, [.github/workflows/build.yml:325](../../../.github/workflows/build.yml#L325)), but the preceding `Check NuGet version` / `Check npm version` steps throw hard when the version already exists ([Test-NuGetPackageVersionAvailable.ps1:57-58](../../../tools/packaging/Test-NuGetPackageVersionAvailable.ps1#L57), [.github/workflows/build.yml:291-296](../../../.github/workflows/build.yml#L291)). A legitimate re-run of an already-succeeded `main` build therefore fails at the check step and never reaches the idempotent push. The spec allows "fail OR skip as an idempotent retry," but the design goal of repairable/idempotent retries ([design.md:147-158](design.md#L147)) is not achieved.
- **Recommendation:** Either let the check warn-and-continue (so `--skip-duplicate` handles the retry) or document that re-running a published `main` build is expected to fail the version check.
- **Fix:** NuGet duplicate detection now warns and continues so `--skip-duplicate` handles reruns, while npm duplicate detection marks the package as already published and skips the npm publish step.
- **Verification:** Confirmed. [Test-NuGetPackageVersionAvailable.ps1:57-59](../../../tools/packaging/Test-NuGetPackageVersionAvailable.ps1#L57) now `Write-Warning`s and returns instead of throwing, letting `--skip-duplicate` handle the retry. The npm check sets `exists=true` and `exit 0` on a duplicate ([build.yml:292-300](../../../.github/workflows/build.yml#L292)), and the editor-assets publish step is gated on `steps.npm-version.outputs.exists != 'true'` ([build.yml:346](../../../.github/workflows/build.yml#L346)). Re-running a published `main` build no longer fails the version checks.

### ✅ Verified - RF7 `check-vsix-version.mjs` may reject the first-ever publish
- **Severity:** Low
- **Evidence:** [src/vscode/scripts/check-vsix-version.mjs:33-39](../../../src/vscode/scripts/check-vsix-version.mjs#L33) only treats a `vsce show` failure as "not published yet" when `error.status === 1` and stderr matches `/not found|does not exist/i`; otherwise it rethrows and fails the publish. If `vsce show <id> --json` returns a different exit code or message for an unknown extension, the very first production publish (and Open VSX, via the `/not found|404/` branch at line 47) would be blocked. This path was not exercised — task 5.4 only verified local packaging.
- **Recommendation:** Verify the actual `vsce show` / `ovsx get` exit codes and messages for a non-existent extension, and broaden the "not yet published" detection accordingly before enabling production credentials.
- **Fix:** Broadened the not-yet-published detection to consider stdout and stderr across non-zero exit codes for common Marketplace/Open VSX missing-extension messages.
- **Verification:** Confirmed. The new `isNotPublishedError` helper ([check-vsix-version.mjs:28-32](../../../src/vscode/scripts/check-vsix-version.mjs#L28)) matches any non-zero exit and scans combined stdout+stderr against `not found|does not exist|doesn't exist|could not find|no extension|404`. The "already exists" guard still propagates correctly: a plain `throw new Error(...)` has no `.status`/`.stdout`, so the helper returns `false` and the duplicate error is not misclassified as "not published". Detection is now robust to CLI message/exit-code variation; the residual caveat is that exact CLI output was still not exercised against live registries (task 5.4 scope).

### ✅ Verified - RF8 Generated `artifacts/` build output is not gitignored
- **Severity:** Low
- **Evidence:** `git check-ignore artifacts/` reports it is not ignored, and it currently contains generated `nx-runtime` native assets (the csproj's default `NxRuntimeNativePackageAssetsRoot` is `artifacts\nx-runtime\`). It is easy to accidentally `git add` this build output.
- **Recommendation:** Add `artifacts/` (or the specific generated subpath) to `.gitignore`.
- **Fix:** Added `/artifacts/` to `.gitignore` as repository-local package staging output.
- **Verification:** Confirmed. `.gitignore:50` contains `/artifacts/` and `git check-ignore artifacts/` now reports it as ignored.

## Questions
- Should production publish run unattended on every `main` merge during beta, or should the `production` environment require manual approval from the start? (design.md Open Questions; affects whether RF6 re-run behavior matters in practice.)
- Is tag-based VS Code publishing intended to remain supported (RF2), or is `workflow_dispatch` repair the only sanctioned repair path?

## Summary
- The core mechanics are solid: PR builds stay credential-free and upload artifacts, production jobs consume verified `deployables-Complete` / `editor-assets-package` / `vscode-vsix-*` artifacts without repacking, environment gating (`preview`/`production`) is wired correctly, fork PRs never reach publish jobs, metadata URLs are fixed, and metadata/version verification scripts were added. Tasks and specs are substantially implemented.
- No high-severity correctness defects found. The two most actionable issues are doc↔implementation drift on the preview toggle (RF1) and the silently-removed-but-still-documented tag publish path (RF2). The remaining findings are robustness/consistency hardening (npm auth gate, action pinning, npmrc formatting, idempotent retry, first-publish detection, gitignore) worth addressing before enabling production credentials, per task 5.5.
- **Verification pass (2026-06-22):** All 8 findings (RF1–RF8) verified as fixed. RF3's trusted-publishing-only decision was propagated consistently across the workflow, editor-assets spec, tasks 1.5/3.5, and both deployment docs — no spec↔implementation divergence remains. The only residual caveat is RF7's untested-against-live-registries note, which is within the pre-credential review scope (task 5.5). No findings reopened; no new findings.

## New Findings Discovered During 2026-06-23 18:59 Review

**Scope of this pass:** Staged changes only (`git diff --cached`): the new `preview_artifact_run_id`
manual-dispatch input and artifact-reuse flow in [.github/workflows/build.yml](../../../.github/workflows/build.yml),
the `ls *.tgz | head` → `find … -print -quit` hardening, and the `docs/deployment-setup.md` /
`docs/deployment.md` updates describing the reuse flow. This implements spec scenario "Manual preview
publish reuses verified artifacts" ([specs/package-release-automation/spec.md:23-29](specs/package-release-automation/spec.md#L23)) and task 3.6.

**Assessment:** The core mechanic is correct. The job-skip gating
([build.yml:34](../../../.github/workflows/build.yml#L34), [build.yml:165](../../../.github/workflows/build.yml#L165))
is the exact negation of the publish-preview gate, so when a run ID is supplied and publishing is
enabled, `build`/`package`/`smoke-test-package`/`editor-assets` are all skipped and `publish-preview`
still runs via `always()` plus the `inputs.preview_artifact_run_id != ''` clause
([build.yml:213](../../../.github/workflows/build.yml#L213)). Cross-run download is correctly wired
with `actions: read`, `run-id`, and `github-token`. The fallback (no run ID) path still requires the
package jobs to succeed. Docs match the implementation. Findings below are robustness/consistency only.

### ✅ Verified - RF9 Preview npm publish still uses a shell glob instead of the hardened `find` selection used everywhere else in this change
- **Severity:** Low
- **Evidence:** This change replaced `ls dist/*.tgz | head -n 1` with `find … -name '*.tgz' -type f -print -quit` in three places ([build.yml:197](../../../.github/workflows/build.yml#L197), [build.yml:324](../../../.github/workflows/build.yml#L324), [build.yml:381](../../../.github/workflows/build.yml#L381)) for robustness, but the preview npm publish still passes a raw glob: `npm publish "${{ runner.temp }}"/editor-assets/*.tgz` ([build.yml:286](../../../.github/workflows/build.yml#L286)). If the editor-assets artifact ever contains more than one `.tgz`, the glob expands to multiple args and `npm publish` will either fail or publish an unintended file, whereas the production path was just hardened against exactly this. Note also that `find … -print -quit` returns the first directory-order match (not sorted like `ls`), so multi-tarball selection is now nondeterministic-but-consistent — fine for the single-tarball reality, but the preview path doesn't even share that behavior.
- **Recommendation:** Select the tarball with the same `find … -print -quit` pattern into a variable and pass the quoted variable to `npm publish`, matching the other three call sites.
- **Fix:** Updated the preview npm publish step to select the tarball with `find … -print -quit` and pass the quoted package path to `npm publish`.
- **Verification:** Confirmed. [build.yml:303-304](../../../.github/workflows/build.yml#L303) now sets `package=$(find "${{ runner.temp }}/editor-assets" -maxdepth 1 -name '*.tgz' -type f -print -quit)` and runs `npm publish "$package" …`, matching the three production/editor-assets call sites. No raw `*.tgz` glob remains in a publish argument.

### ✅ Verified - RF10 Source-run validation makes three redundant `gh api` calls for the same run
- **Severity:** Low
- **Evidence:** The "Validate source artifact run" step calls `gh api repos/…/actions/runs/${PREVIEW_ARTIFACT_RUN_ID}` three separate times to extract `.status`, `.conclusion`, and `.head_sha` ([build.yml:227-229](../../../.github/workflows/build.yml#L227)). Each call refetches the full run object.
- **Recommendation:** Fetch once into a variable and read all three fields with a single `--jq`/`jq` pass (or `--jq '[.status,.conclusion,.head_sha] | @tsv'`). Minor, but reduces API calls and avoids the three-way TOCTOU window.
- **Fix:** Changed source-run validation to fetch the run object once and read status, conclusion, head SHA, workflow path, and source repository from one `gh api --jq` call.
- **Verification:** Confirmed. The run object is now fetched a single time via `gh api … --jq '[.status, .conclusion, .head_sha, .path, .head_repository.full_name] | @tsv'` and split with `IFS=$'\t' read` ([build.yml:227-230](../../../.github/workflows/build.yml#L227)). A null `conclusion` yields an empty TSV field, so the `!= "success"` check still fails closed. (The separate `…/artifacts` call at [build.yml:247](../../../.github/workflows/build.yml#L247) is a distinct resource for RF11, not a duplicate of the run fetch.)

### ✅ Verified - RF11 Source-run validation does not confirm the run is the Build workflow or that the expected artifacts exist
- **Severity:** Low
- **Evidence:** Validation only checks `status == completed` and `conclusion == success` ([build.yml:231-234](../../../.github/workflows/build.yml#L231)). A maintainer who pastes the ID of any successful run (e.g. the VS Code extension workflow, or a Build run from before these artifacts existed) passes validation and then fails later at the `download-artifact` step with a less obvious "artifact not found" error. The spec only requires validating successful completion ([spec.md:26](specs/package-release-automation/spec.md#L26)), so this is hardening, not a spec gap.
- **Recommendation:** Optionally also assert `.path == '.github/workflows/build.yml'` (or `.name`) from the run object, and/or surface a clearer error when `deployables-Complete` / `editor-assets-package` are absent from the source run.
- **Fix:** Added validation that the source run path is `.github/workflows/build.yml` and that the run exposes both `deployables-Complete` and `editor-assets-package` before any download step runs.
- **Verification:** Confirmed. [build.yml:237-240](../../../.github/workflows/build.yml#L237) rejects runs whose `.path != .github/workflows/build.yml`, and [build.yml:247-251](../../../.github/workflows/build.yml#L247) lists the run's artifacts (`--paginate`) and fails with a clear `::error::` unless both `deployables-Complete` and `editor-assets-package` are present (exact-line `grep -Fxq`). A wrong-but-successful run ID, or a run whose artifacts expired, now fails at validation with an actionable message instead of at download.

### ✅ Verified - RF12 Preview reuse can publish artifacts built from an untrusted run; fetched `head_sha` is logged but not used as a trust gate
- **Severity:** Low
- **Evidence:** `head_sha` is fetched and echoed for the operator ([build.yml:229,236](../../../.github/workflows/build.yml#L236)) but never checked against a trusted ref/branch. Because preview reuse accepts any successful Build run ID — including a fork PR's run, whose `deployables-Complete`/`editor-assets-package` were built from untrusted code — a maintainer could publish fork-built artifacts to the preview feed. This is mitigated by being manual-dispatch + `preview` environment-gated + preview-scoped credentials, and the spec/design intentionally allow reuse, so it is acceptable; but task 5.5 explicitly asks to review fork-PR/trusted-preview behavior before enabling credentials.
- **Recommendation:** Either document this caveat in the runbook (operator must confirm the source run is from a trusted ref) or, for stronger safety, verify `head_branch`/`event` of the source run (e.g. reject runs whose `head_repository.full_name` differs from the base repo) before publishing.
- **Fix:** Added a source repository gate that rejects preview artifact runs whose `head_repository.full_name` does not match the current repository, and documented that preview source runs must come from this repository.
- **Verification:** Confirmed. [build.yml:242-245](../../../.github/workflows/build.yml#L242) fails when `head_repository != ${GITHUB_REPOSITORY}`, so a fork-PR run (whose `head_repository` is the fork) is rejected before any download/publish. [docs/deployment.md:87](../../../docs/deployment.md#L87) documents that the source run must be a successful Build run from this repository with package artifacts. This narrows reuse to same-repo runs — the intended trusted-source tradeoff. Note: the gate relies on `head_repository.full_name`; this is the right safety posture for the `preview` environment, and production publishing is unaffected (separate push-to-`main` path).

## Questions
- None new. (Prior questions on unattended `main` publishing and tag-based VS Code publishing still stand above.)

## Summary (2026-06-23 pass)
- The `preview_artifact_run_id` artifact-reuse flow is correctly implemented and matches both the spec scenario and the docs: job-skip gating is the precise negation of the publish gate, cross-run artifact download has the right permissions/inputs, and the success-of-source-run validation is present.
- No correctness defects or regressions found in the staged diff. Four Low-severity hardening/consistency items: the preview npm publish was not hardened alongside the other three tarball lookups (RF9), redundant API calls in validation (RF10), validation doesn't confirm workflow identity/artifact presence (RF11), and reuse has no trusted-ref gate beyond environment protection (RF12 — relevant to task 5.5).
- **Verification pass (2026-06-23 19:05):** All four findings (RF9–RF12) verified as fixed and marked ✅ Verified. The preview npm publish now uses the hardened `find … -print -quit` selection (RF9); source-run validation fetches the run object once via a single `@tsv` `gh api` call (RF10); validation now rejects runs that aren't from `build.yml` or are missing the `deployables-Complete`/`editor-assets-package` artifacts, failing with clear errors before download (RF11); and a `head_repository == ${GITHUB_REPOSITORY}` gate plus a runbook note now block reuse of fork/untrusted runs (RF12). No findings reopened; no new findings. All 12 findings (RF1–RF12) are now resolved.

## Post-Review Refactor Note

- **2026-06-23:** NuGet/editor-assets preview and production publishing moved out of
  `.github/workflows/build.yml` into `.github/workflows/package-publish.yml`. The Build workflow is now
  artifact-only again; Publish packages validates source Build runs, downloads the verified artifacts,
  and publishes through `preview` or `production`. The earlier `publish_preview`,
  `preview_artifact_run_id`, and `PUBLISH_PREVIEW_PACKAGES` Build-workflow toggle path was removed in
  favor of explicit Publish packages workflow dispatch inputs (`target_environment`, `artifact_run_id`)
  and automatic production release from successful `main` Build runs.

## New Findings Discovered During 2026-06-23 19:49 Review

**Scope of this pass:** The dedicated **Publish packages** workflow refactor (publishing now runs in a
"release" workflow instead of the Build workflow). Reviewed: the new
[.github/workflows/package-publish.yml](../../../.github/workflows/package-publish.yml) (`workflow_run`
auto-production trigger + `workflow_dispatch` preview/production with `artifact_run_id`); the
[build.yml](../../../.github/workflows/build.yml) diff that strips the publish jobs and toggles back to
artifact-only; the [release.yml](../../../.github/workflows/release.yml) diff that removes registry
publishing (task 3.8); and the matching [docs/deployment.md](../../../docs/deployment.md) /
[docs/deployment-setup.md](../../../docs/deployment-setup.md) updates. Implements spec scenarios
"Production environment publishing" and "Manual preview publish reuses verified artifacts"
([specs/package-release-automation/spec.md:23-38](specs/package-release-automation/spec.md#L23)) and
tasks 3.7/3.8.

**Assessment:** The refactor is sound. The `workflow_run.workflows: [🏭 Build]` reference exactly
matches `build.yml`'s top-level `name:` ([build.yml:1](../../../.github/workflows/build.yml#L1)), so
the auto-trigger will actually fire. Fork/untrusted safety is enforced both at the job `if`
(`workflow_run.event == 'push' && head_branch == 'main' && head_repository.full_name == github.repository`,
[package-publish.yml:32](../../../.github/workflows/package-publish.yml#L32)) and re-validated inside
`prepare` (run path is `build.yml`, repo matches, production requires `head_branch == main`, both
artifacts present). Cross-run download has `actions: read` + `run-id` + `github-token` in both publish
jobs, and the `preview`/`production` environment split, OIDC/`id-token: write` scoping, and idempotent
NuGet/npm version handling carried over intact. Behavior vs. the old in-Build publish job is actually
slightly safer: the workflow only triggers when the *entire* Build run concludes `success`. Findings
below are leftovers/clarity only — no correctness defects.

### ✅ Verified - RF13 GitHub Release workflow still downloads `editor-assets-package` but no longer uses it
- **Severity:** Low
- **Evidence:** Task 3.8 removed the npm/NuGet publish steps from
  [release.yml](../../../.github/workflows/release.yml), but the "Download editor-assets package" step
  ([release.yml:58-64](../../../.github/workflows/release.yml#L58)) was left in place. Its only former
  consumer was the deleted "Publish editor-assets package" npm step. The surviving "Upload artifacts to
  release" step iterates only `${{ runner.temp }}/deployables`
  ([release.yml:72](../../../.github/workflows/release.yml#L72)), so the `editor-assets` tarball is now
  downloaded into `${{ runner.temp }}/editor-assets` and silently discarded — wasted work, and
  misleading to a reader who assumes the tarball gets attached to the Release.
- **Recommendation:** Either delete the orphaned editor-assets download step, or — if the npm tarball
  should be a GitHub Release asset — extend the upload step to also iterate
  `${{ runner.temp }}/editor-assets`. Decide intent and make the download and upload consistent.
- **Fix:** Deleted the orphaned `editor-assets-package` download from `release.yml`, leaving the
  GitHub Release workflow focused on deployable release assets while package registry publication stays
  in `package-publish.yml`.
- **Verification:** Confirmed. [release.yml](../../../.github/workflows/release.yml) now contains a
  single download step (`deployables-Complete` → `${{ runner.temp }}/deployables`,
  [release.yml:50-56](../../../.github/workflows/release.yml#L50)); the `editor-assets-package` download
  is gone and `editor-assets` appears nowhere in the file. The "Upload artifacts to release" step
  ([release.yml:58-67](../../../.github/workflows/release.yml#L58)) iterates only `/deployables`, so
  every downloaded artifact is now consumed. No wasted download remains.

### ✅ Verified - RF14 Removing `ship_run_id` drops the manual re-ship path for GitHub Release assets
- **Severity:** Low
- **Evidence:** The refactor removed `release.yml`'s `workflow_dispatch` / `ship_run_id` input and the
  branch that shipped a specific run's artifacts to a Release. `release.yml` now triggers only on
  `release: published` and always resolves the run from the tag
  ([release.yml:3-5,35-44](../../../.github/workflows/release.yml#L3)). Registry repair is fully covered
  by `package-publish.yml` dispatch (`target_environment` + `artifact_run_id`), but if the Release
  *asset upload* itself fails or needs to be redone from a different run, there is no longer a manual
  dispatch path — a maintainer must re-publish the Release. This is plausibly intended, but it is a
  capability reduction not called out in the runbook.
- **Recommendation:** Confirm this removal is intentional. If GitHub Release asset re-upload should
  remain repairable, restore a minimal `workflow_dispatch` with a run-id input on `release.yml`;
  otherwise note in `docs/deployment.md` that re-uploading Release assets means re-publishing the
  Release.
- **Fix:** Documented that failed GitHub Release asset upload should be repaired by uploading the same
  Build workflow artifacts to the GitHub Release, while package registry repair remains in the Package
  release workflow.
- **Verification:** Confirmed. The recommendation's "document it" option was taken:
  [docs/deployment.md:71-72](../../../docs/deployment.md#L71) now states "If GitHub Release asset upload
  fails, upload the same Build workflow artifacts to the GitHub Release; registry repair still belongs
  in the Publish packages workflow." `release.yml` remains `release: published`-only (no
  `workflow_dispatch`/`ship_run_id`), which is the intended scope, and the runbook now records the
  manual asset re-upload path. The capability gap is closed at the documentation level as recommended.

### ✅ Verified - RF15 "Release Model" wording implies VSIX flows through the Publish packages workflow
- **Severity:** Low
- **Evidence:** [docs/deployment.md:8-11](../../../docs/deployment.md#L8) now reads "the Publish packages
  workflow publishes those already-verified artifacts," but VS Code extension (VSIX) publishing was
  intentionally kept in `.github/workflows/vscode-extension.yml` (task 4.2), not the Publish packages
  workflow. The release-steps section does distinguish "VS Code extension workflow artifacts"
  separately, so this is a wording/completeness nit rather than a contradiction, but an operator could
  read the model paragraph as "everything, including VSIX, ships from package-publish.yml."
- **Recommendation:** Tighten the Release Model paragraph to scope the Publish packages workflow to
  NuGet + editor-assets and add one sentence that VSIX publishing runs from the VS Code extension
  workflow.
- **Fix:** Updated the Release Model wording to scope Publish packages to NuGet and editor-assets
  artifacts and explicitly state that VSIX publishing remains in the VS Code extension workflow.
- **Verification:** Confirmed. [docs/deployment.md:8-10](../../../docs/deployment.md#L8) now reads "The
  Build workflow proves the NuGet and editor-assets package bits; the Publish packages workflow publishes
  those already-verified artifacts. VSIX publishing remains in the VS Code extension workflow." The
  Publish packages workflow is now explicitly scoped to NuGet + editor-assets and VSIX is called out as a
  separate path, removing the "everything ships from package-publish.yml" misreading. Consistent with
  task 4.2 (VSIX stays in `vscode-extension.yml`).

## Questions
- None new. (Standing questions on unattended `main` production publishing and tag-based VS Code
  publishing remain.) RF14 raises one decision: is the loss of `ship_run_id` GitHub-Release re-ship
  intended?

## Summary (2026-06-23 19:49 pass — Publish packages workflow refactor)
- Moving publishing into the dedicated `package-publish.yml` "release" workflow is correctly
  implemented: the `workflow_run` name reference matches the Build workflow, fork/untrusted runs are
  blocked at both the job gate and in-job validation, cross-run artifact download is wired with the
  right permissions, production requires a `main` source run, and the `preview`/`production` split plus
  idempotent version handling carried over from the prior (verified) passes. `build.yml` is cleanly
  back to artifact-only and `release.yml`'s registry publishing was removed (task 3.8).
- No correctness defects or regressions found. Three Low-severity leftovers: `release.yml` still
  downloads the now-unused editor-assets tarball (RF13), the `ship_run_id` manual re-ship path was
  dropped without a runbook note (RF14), and the "Release Model" doc wording can imply VSIX flows
  through package-publish.yml (RF15). All are cleanup/clarity, safe to address before enabling
  production credentials (task 5.5).
- **Verification pass (2026-06-23 19:52):** All three findings (RF13–RF15) verified as fixed and marked
  ✅ Verified. The orphaned editor-assets download was removed from `release.yml` (RF13); the GitHub
  Release asset re-upload repair path is now documented in `docs/deployment.md` (RF14, "document it"
  option taken); and the Release Model paragraph now scopes Publish packages to NuGet + editor-assets and
  names VSIX as a separate VS Code-extension-workflow path (RF15). No findings reopened; no new findings.
  All 15 findings (RF1–RF15) are now resolved.
