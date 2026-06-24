## 1. Metadata And Documentation

- [x] 1.1 Fix `NxLang.Runtime` NuGet metadata URLs to use `https://github.com/nx-lang/nx`.
- [x] 1.2 Add or update package metadata verification so stale repository/project URLs are caught before NuGet publication.
- [x] 1.3 Create `docs/deployment-setup.md` covering one-time `preview` and `production` GitHub environment setup, registry ownership, protection rules, trusted publishing policies, and token fallback secrets.
- [x] 1.4 Create `docs/deployment.md` as the ongoing release runbook for publishing new releases, inspecting package artifacts, repairing partial publishes, and handling higher-version fixes or unlist/deprecate actions.
- [x] 1.5 Document required GitHub secrets and variables, including `VSCE_PAT`, `OVSX_PAT`, `NUGET_USER`, `NUGET_API_KEY` fallback, production npm trusted publishing, and any preview-feed toggles.
- [x] 1.6 Link `docs/deployment-setup.md` and `docs/deployment.md` from the VS Code extension and .NET binding release documentation.

## 2. Versioning And Package Artifact Plumbing

- [x] 2.1 Add CI-safe VS Code extension version staging so publish builds can produce a unique `major.minor.patch` VSIX version without committing a manifest edit for every main merge.
- [x] 2.2 Update VS Code package scripts or workflow steps so `package:verify` and publish jobs use the same staged extension manifest/version.
- [x] 2.3 Add pre-publish version checks or idempotent duplicate handling for NuGet, npm editor assets, and VSIX registry publication.
- [x] 2.4 Ensure publish jobs consume verified `.nupkg`, `.tgz`, and `.vsix` artifacts from package jobs rather than rebuilding package contents.

## 3. NuGet And Editor Assets CI Publishing

- [x] 3.1 Update `.github/workflows/build.yml` so PR builds continue producing verified NuGet and editor-assets artifacts without public registry credentials.
- [x] 3.2 Add optional `preview` environment publishing for trusted contexts to preview/test NuGet and npm-compatible feeds, defaulting to artifact upload only.
- [x] 3.3 Add `production` environment publishing from trusted `main` builds after package assembly, package inspection, and RID smoke tests pass.
- [x] 3.4 Implement NuGet.org trusted publishing through GitHub OIDC when available, with `NUGET_API_KEY` fallback behavior documented and gated.
- [x] 3.5 Implement npm trusted publishing for `@nx-lang/language` without a production `NPM_TOKEN` fallback.
- [x] 3.6 Add a manual preview publish path that reuses artifacts from a successful Build workflow run instead of rebuilding package contents.
- [x] 3.7 Move NuGet and editor-assets preview/production publishing from the Build workflow into a dedicated Publish packages workflow.
- [x] 3.8 Remove legacy NuGet/npm registry publishing from the GitHub Release workflow so Publish packages is the only package registry publishing path.

## 4. VS Code Extension CI Publishing

- [x] 4.1 Update `.github/workflows/vscode-extension.yml` so PR builds package and upload VSIX artifacts for inspection without public registry credentials.
- [x] 4.2 Change the primary production VS Code extension publish path from tag-only to trusted `main` builds targeting the `production` environment.
- [x] 4.3 Preserve manual dispatch or explicit-artifact repair publishing for partial registry failures.
- [x] 4.4 Keep platform-specific VSIX targets publishing the same verified artifacts to both the Visual Studio Marketplace and Open VSX.
- [x] 4.5 Fail before VSIX publication when `VSCE_PAT` or `OVSX_PAT` is required but unavailable in the target environment.

## 5. Verification

- [x] 5.1 Run OpenSpec validation for `automate-package-publishing`.
- [x] 5.2 Run .NET build/tests and `tools/packaging/Test-NxRuntimePackage.ps1` against a complete package artifact where CI artifacts are available.
- [x] 5.3 Run editor-assets package, verify, and smoke tests from `src/vscode`.
- [x] 5.4 Run VS Code extension package verification for at least the local host target.
- [x] 5.5 Review generated workflow behavior for fork PRs, trusted preview contexts, and `main` production publishing before enabling production credentials.
