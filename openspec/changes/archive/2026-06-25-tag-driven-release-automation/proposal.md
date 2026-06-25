## Why

The current release pipeline splits build artifacts, GitHub Releases, and registry publication in a way
that makes the release source of truth unclear. Moving to tag-driven draft releases creates a human
review gate, keeps `main` and pull request builds focused on verified artifacts, and gives testers a
safe artifact-based path that works without registry credentials.

## What Changes

- **BREAKING**: Stop production package and VS Code extension publication from successful `main`
  builds. `main` builds will produce CI artifacts only.
- **BREAKING**: Remove preview/test package-registry publishing from the supported PR testing path.
  Pull requests will expose tested package artifacts and install commands instead of publishing to
  preview feeds.
- Add two tag-driven release tracks:
  - `v*` tags build compiler/runtime and editor-assets artifacts, then create a draft GitHub Release.
  - `vscode-v*` tags build VS Code extension artifacts, then create a draft GitHub Release.
- Publish external registries only when a human publishes the corresponding GitHub Release.
- Publish from GitHub Release assets rather than rebuilding or consuming transient workflow artifacts.
- Replace Nerdbank.GitVersioning/`nbgv` usage with MinVer CLI-driven version calculation so package
  versions are derived from tags and CI source context through one simpler versioning path.
- Always upload tested PR artifacts for package and extension changes:
  `.nupkg`, `.snupkg`, npm `.tgz`, and `.vsix`.
- Add PR comments with exact commands for downloading and installing tested artifacts.
- Update release and deployment documentation to describe tag creation, draft release review,
  release publication, artifact testing, repair, and versioning rules.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `package-release-automation`: Main and PR builds become artifact-only, preview registry publishing is
  removed, compiler/editor-assets releases are created from `v*` tags as draft GitHub Releases,
  production package registries publish only from published GitHub Release assets, and package
  versions are derived with MinVer instead of NBGV.
- `vscode-extension-publishing`: VS Code extension releases are created from `vscode-v*` tags as draft
  GitHub Releases, Marketplace/Open VSX publishing happens only after the release is published, and
  PR extension builds expose tested VSIX artifacts plus install commands.
- `editor-assets`: The `@nx-lang/language` npm package uses the shared MinVer-derived release version
  and participates in artifact-only PR testing and `v*` package releases.

## Impact

- Affected workflows:
  `.github/workflows/build.yml`, `.github/workflows/package-publish.yml`,
  `.github/workflows/release.yml`, `.github/workflows/vscode-extension.yml`, and
  `.github/workflows/vscode-extension-publish.yml`.
- Affected versioning files/scripts: `version.json`, `.config/dotnet-tools.json`,
  `tools/variables/*`, `src/vscode/scripts/stage-vsix-version.mjs`, and
  `src/vscode/scripts/package-language.mjs`.
- Affected documentation: `docs/deployment.md`, `docs/deployment-setup.md`, and VS Code extension
  publishing documentation.
- External systems: GitHub Releases, NuGet.org, npm, Visual Studio Marketplace, and Open VSX.
- Maintainer workflow changes: maintainers create release tags, review draft GitHub Releases, publish
  the release to trigger registry publication, and test PR builds by downloading CI artifacts instead
  of consuming preview registries.
