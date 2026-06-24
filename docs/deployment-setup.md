# Deployment Setup

This checklist covers the one-time setup for publishing NX packages from GitHub Actions. The
ongoing release runbook lives in [deployment.md](deployment.md).

## GitHub Environments

Create two GitHub environments:

- `preview`: used for trusted preview/test feed publishing. Keep public registry credentials out of
  this environment. Optional feeds can use repository-scoped `GITHUB_TOKEN` or preview-only tokens.
- `production`: used for NuGet.org, npm, Visual Studio Marketplace, and Open VSX publication. Start
  with required reviewers if automatic main publishing should wait for human approval.

Recommended protection:

- Restrict `production` deployments to `main`.
- Add required reviewers for `production` before enabling public registry writes.
- Keep fork pull requests artifact-only; do not expose environment secrets to untrusted PRs.
- Audit environment deployment history after each release.

## Registry Ownership

Set up ownership before enabling publication:

- NuGet.org: reserve or own `NxLang.Runtime`.
- npm: own the `@nx-lang` scope and the `@nx-lang/language` package.
- Visual Studio Marketplace: own publisher `nx-lang` and extension `nx-language`.
- Open VSX: own namespace `nx-lang` and extension `nx-language`.

## Trusted Publishing

Prefer trusted publishing where the registry supports it:

- NuGet.org: create a trusted publishing policy for repository `nx-lang/nx`, workflow file
  `package-publish.yml`, and the `production` environment. Set `NUGET_USER` as a production
  environment secret for the NuGet owner account used by `NuGet/login`.
- npm: create a trusted publisher for `@nx-lang/language` that matches repository `nx-lang/nx`,
  workflow `.github/workflows/package-publish.yml`, and environment `production`.

The production workflow requests GitHub OIDC with `id-token: write` only in publish jobs.

## Secrets And Variables

Production environment secrets:

- `NUGET_USER`: NuGet.org account or organization owner used by NuGet trusted publishing.
- `NUGET_API_KEY`: fallback NuGet.org API key when trusted publishing is unavailable.
- `VSCE_PAT`: Visual Studio Marketplace token for publisher `nx-lang`.
- `OVSX_PAT`: Open VSX token for namespace `nx-lang`.

Preview environment secrets or variables:

- `PREVIEW_NUGET_SOURCE`: optional NuGet-compatible preview feed URL.
- `PREVIEW_NUGET_API_KEY`: optional preview feed API key.
- `PREVIEW_NPM_REGISTRY`: optional npm-compatible preview registry URL.
- `PREVIEW_NPM_TOKEN`: optional preview npm token.

The Publish packages workflow accepts `artifact_run_id`. Set it to a successful Build workflow run ID
when publishing preview packages from already-verified PR or branch artifacts. This lets the preview
publish job download and publish the same `deployables-Complete` and `editor-assets-package` artifacts
instead of rebuilding package contents.

Never commit registry tokens or write them into tracked configuration files.

## First Enablement

1. Confirm PR workflows upload `deployables-Complete`, `editor-assets-package`, and VSIX artifacts
   without public registry credentials.
2. Enable `preview` only after the preview feed and variables are configured.
3. Enable `production` after package version checks, package inspection, smoke tests, and Package
   release workflow validation pass.
4. Keep `NUGET_API_KEY` empty when NuGet trusted publishing is working. Production npm publishing
   uses trusted publishing only; do not configure an npm publish token for CI.
5. Run the first production publish with required reviewers enabled.
