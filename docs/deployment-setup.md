# Deployment Setup

This checklist covers the one-time setup for publishing NX packages and VS Code extension artifacts
from GitHub Actions. The ongoing release runbook lives in [deployment.md](deployment.md).

## GitHub Environment

Create one GitHub environment:

- `production`: used only by workflows that publish already-reviewed GitHub Release assets to
  NuGet.org, npm, the Visual Studio Marketplace, and Open VSX.

Recommended protection:

- Add required reviewers before enabling public registry writes.
- Restrict deployments to protected release branches or tags as appropriate for the repository.
- Keep pull request and `main` build workflows artifact-only; they do not need production secrets.
- Audit environment deployment history after each published release.

## Registry Ownership

Set up ownership before enabling publication:

- NuGet.org: reserve or own `NxLang.Sdk`.
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

The package publish job requests GitHub OIDC with `id-token: write` only after a package GitHub
Release is published and its release assets are validated.

Visual Studio Marketplace and Open VSX publishing currently use production environment token secrets
in `.github/workflows/vscode-extension-publish.yml`.

## Secrets And Variables

Production environment secrets:

- `NUGET_USER`: NuGet.org account or organization owner used by NuGet trusted publishing.
- `NUGET_API_KEY`: fallback NuGet.org API key when trusted publishing is unavailable.
- `VSCE_PAT`: Visual Studio Marketplace token for publisher `nx-lang`.
- `OVSX_PAT`: Open VSX token for namespace `nx-lang`.

No preview NuGet, preview npm, or pull request registry credentials are required. Pull request
testing uses workflow artifacts and PR comments with download/install commands.

The Publish packages workflow accepts `release_tag` for manual repair. Set it to a published package
GitHub Release tag such as `v1.2.3`; the workflow downloads and republishes the attached release
assets without rebuilding package contents.

The Publish VS Code extension workflow accepts `release_tag` for manual repair. Set it to a
published VS Code GitHub Release tag such as `vscode-v1.2.3`; the workflow validates and republishes
the attached VSIX assets without rebuilding package contents.

Rust tool publication for `nxlang`, `nx-lsp`, and Rust crates is not part of this deployment setup
yet; no crates.io token or Rust binary-release credential is required for this release pipeline.

Never commit registry tokens or write them into tracked configuration files.

## Versioning Setup

The repository uses the restored local .NET tool `minver-cli` through
`tools/versions/Get-ReleaseVersion.ps1`.

Supported release tag formats:

- Package releases: `v<major>.<minor>.<patch>`, for example `v1.2.3`.
- VS Code extension releases: `vscode-v<major>.<minor>.<patch>`, for example `vscode-v1.2.3`.

The first implementation intentionally supports stable `major.minor.patch` release tags only. Pull
request and `main` package artifacts receive unique prerelease versions, while VSIX artifacts use
registry-valid `major.minor.patch` versions and are tested by direct VSIX installation.

## First Enablement

1. Confirm PR workflows upload `deployables-Complete`, `editor-assets-package`, and `vscode-vsix-*`
   artifacts without public registry credentials.
2. Confirm the trusted PR artifact comment workflow posts download/install commands without checking
   out or executing pull request code.
3. Push a test package tag in a disposable repository or dry-run branch and confirm `release.yml`
   creates a draft package GitHub Release with `.nupkg`, `.snupkg`, `.tgz`, manifest, and checksum
   assets.
4. Push a test VS Code tag in a disposable repository or dry-run branch and confirm
   `vscode-release.yml` creates a draft VS Code GitHub Release with VSIX, manifest, and checksum
   assets.
5. Enable `production` after release asset validation, package inspection, smoke tests, and publish
   workflow validation pass.
6. Keep `NUGET_API_KEY` empty when NuGet trusted publishing is working. Production npm publishing
   uses trusted publishing only; do not configure an npm publish token for CI.
