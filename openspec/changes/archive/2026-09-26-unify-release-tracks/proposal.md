## Why

NX releases its packages (NuGet, npm, including the `@nx-lang/language` grammar) from `v*` tags and
the VS Code extension from `vscode-v*` tags, each with its own version, draft release and publish
workflow. But the extension is not a separate product: its language server, `nx-lsp`, is built from
the same Rust crates as the runtime, and a language change such as the occurrence-suffix change
touches the parser, the runtime, the grammar and the language server at once. Two tracks let the
editor and the SDKs disagree about what is valid NX, split the grammar (on the package track) from
the extension that uses it, and compete for GitHub's single "Latest" release. While NX is 0.x and
changing quickly, one release should carry all of it.

## What Changes

- **One tag releases everything.** A `v<major>.<minor>.<patch>` tag builds the NuGet package, every
  npm package and the per-platform VSIX files, and attaches them all to one draft GitHub Release,
  titled `NX <version>`.
- **One version.** The extension takes the package version. The first unified release moves it from
  0.1.0 to the next package version (for example 0.5.0), and from then on "extension 0.5.0" means it
  understands the same NX as SDK 0.5.0.
- **One publish.** Publishing that GitHub Release publishes to npm, NuGet, the Visual Studio
  Marketplace (as the `nx-vscode-publisher` managed identity) and Open VSX. Each registry is still
  repaired on its own from the release's assets, through the publish workflow's `release_tag` input.
- **BREAKING (release process): the `vscode-v*` track is retired.** `vscode-release.yml` and
  `vscode-extension-publish.yml` fold into `release.yml` and `package-publish.yml`. A `vscode-v*` tag
  no longer does anything. The existing `vscode-v0.1.0` tag and release stay as history.
- **`Get-ReleaseVersion.ps1` loses its `-Track` parameter.** The VSIX version on a tag is the release
  version; on pull requests and `main` it stays the core of the preview version, as today.
- **Docs.** `docs/deployment.md` describes one release runbook, and `docs/deployment-setup.md` and
  `src/vscode/CONTRIBUTING.md` drop the separate extension track.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `package-release-automation`: the package release track also builds and publishes the VS Code
  extension. "Release publishing is split into explicit package and extension actions" becomes one
  release track; "VS Code tag does not create package release" now says the retired `vscode-v*` tag
  builds and releases nothing; the draft release and production publishing scenarios cover the VSIX
  files and both extension registries.
- `vscode-extension-publishing`: the extension is released from the package `v*` tag at the package
  version, not from `vscode-v*` tags. "VS Code extension versioned release trigger" is removed,
  with its "Package release tag does not create VS Code release" scenario, and replaced by "VS Code
  extension is released with the NX release". "VS Code extension release documentation" changes
  accordingly, "The extension is published for NX users" no longer names the retired track, and
  "VS Code extension publishing credentials" scopes a missing extension credential to the extension
  publish job, which leaves NuGet and npm publishing unaffected.

## Impact

- **Workflows**: a new reusable workflow, `vscode-vsix.yml`, builds the VSIX files, and both
  `vscode-extension.yml` (PR and `main` builds) and `release.yml` call it; `package-publish.yml`
  gains the Marketplace (OIDC `azure/login`, `vsce --azure-credential`) and Open VSX steps;
  `vscode-release.yml` and `vscode-extension-publish.yml` are deleted. `pr-artifact-comment.yml` and
  `marketplace-identity.yml` are unchanged.
- **Scripts**: `tools/versions/Get-ReleaseVersion.ps1` (one track), and the release manifest, which
  lists every artifact of the one release.
- **Secrets**: none added or removed. The `production` environment already holds
  `AZURE_CLIENT_ID`, `AZURE_TENANT_ID` and `OVSX_PAT` next to the package publishing settings.
- **Users**: the extension's version number jumps once. Nothing else changes for someone installing
  it.
