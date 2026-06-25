## 1. Versioning Migration

- [x] 1.1 Add MinVer CLI as a restored local .NET tool and remove NBGV from local tool configuration.
- [x] 1.2 Remove `version.json` and any CI/script assumptions that call `dotnet nbgv`.
- [x] 1.3 Add a shared version helper that validates package tags, VS Code tags, PR builds, and `main` builds.
- [x] 1.4 Make the version helper emit NuGet/MSBuild, npm, and VSIX version values needed by workflows.
- [x] 1.5 Ensure PR and `main` package versions are unique prerelease versions and release tag versions are stable.
- [x] 1.6 Update `src/vscode/scripts/package-language.mjs` to use MinVer-derived or explicit npm package versions.
- [x] 1.7 Update `src/vscode/scripts/stage-vsix-version.mjs` to use MinVer-derived or explicit VSIX versions without NBGV fallback.
- [x] 1.8 Update NuGet packing commands to pass explicit MinVer-derived `Version`/`PackageVersion` properties.

## 2. Artifact-Only CI Builds

- [x] 2.1 Update the Build workflow so PR and `main` runs build, verify, and upload `.nupkg`, `.snupkg`, and npm `.tgz` artifacts without registry credentials.
- [x] 2.2 Update the Build workflow to upload package artifact metadata and test-command data for pull request runs.
- [x] 2.3 Update the VS Code Extension workflow so PR and `main` runs build, verify, and upload VSIX artifacts without Marketplace or Open VSX credentials.
- [x] 2.4 Update the VS Code Extension workflow to upload VSIX artifact metadata and test-command data for pull request runs.
- [x] 2.5 Remove automatic production publishing triggers from successful `main` Build workflow runs.
- [x] 2.6 Remove automatic production publishing triggers from successful `main` VS Code Extension workflow runs.
- [x] 2.7 Remove preview package-registry publish inputs, jobs, variables, and secrets from the supported workflow path.

## 3. Pull Request Artifact Comments

- [x] 3.1 Add a trusted PR artifact comment workflow triggered from completed artifact-producing workflows.
- [x] 3.2 Make the comment workflow read only workflow metadata/artifacts and avoid checking out or executing pull request code.
- [x] 3.3 Generate NuGet test commands that download the package artifacts and install from the downloaded local source.
- [x] 3.4 Generate npm test commands that download the editor-assets `.tgz` and install from the downloaded tarball.
- [x] 3.5 Generate VSIX test commands that download VSIX artifacts and install with `code --install-extension --force`.
- [x] 3.6 Make PR comments idempotent by updating a previous bot comment instead of creating duplicate comments for every run.

## 4. Tag-Driven Draft Releases

- [x] 4.1 Add or update the package release workflow so valid `v<major>.<minor>.<patch>` tags build verified package artifacts.
- [x] 4.2 Add runtime guards so `vscode-v*` tags cannot run the package release track.
- [x] 4.3 Create or update a draft GitHub Release for each valid package release tag.
- [x] 4.4 Attach NuGet, symbol, npm editor-assets, manifest, and checksum assets to the package draft release.
- [x] 4.5 Add or update the VS Code release workflow so valid `vscode-v<major>.<minor>.<patch>` tags build verified VSIX artifacts.
- [x] 4.6 Add runtime guards so package `v*` tags cannot run the VS Code release track.
- [x] 4.7 Create or update a draft GitHub Release for each valid VS Code release tag.
- [x] 4.8 Attach VSIX, manifest, and checksum assets to the VS Code draft release.

## 5. Published Release Publishing

- [x] 5.1 Update package publishing to trigger from `release.published` events for valid package release tags.
- [x] 5.2 Validate package release assets, manifest data, tag format, and artifact versions before registry writes.
- [x] 5.3 Publish NuGet packages from GitHub Release assets using production-scoped authentication.
- [x] 5.4 Publish `@nx-lang/language` from the GitHub Release npm tarball using production-scoped authentication.
- [x] 5.5 Preserve idempotent duplicate-version behavior for package registry retries.
- [x] 5.6 Update VS Code extension publishing to trigger from `release.published` events for valid `vscode-v*` tags.
- [x] 5.7 Validate VS Code release assets, manifest data, tag format, and VSIX versions before registry writes.
- [x] 5.8 Publish VSIX assets to Visual Studio Marketplace and Open VSX from GitHub Release assets.
- [x] 5.9 Preserve per-registry idempotent skip behavior for already-published VSIX versions.
- [x] 5.10 Keep or replace manual repair paths so maintainers can republish from an explicit GitHub Release asset set.

## 6. Documentation

- [x] 6.1 Update `docs/deployment-setup.md` to remove preview registry setup and document production environment credentials.
- [x] 6.2 Update `docs/deployment.md` with tag creation, draft release review, release publication, registry confirmation, and repair steps.
- [x] 6.3 Document pull request artifact testing commands for NuGet, npm editor assets, and VSIX files.
- [x] 6.4 Update VS Code extension publishing documentation to describe `vscode-v*` releases and VSIX artifact testing.
- [x] 6.5 Document MinVer-based versioning rules and any tag-format limitations selected during implementation.

## 7. Verification

- [x] 7.1 Run `openspec validate tag-driven-release-automation --strict`.
- [x] 7.2 Run local version helper checks for package tags, VS Code tags, PR-like metadata, and `main`-like metadata.
- [x] 7.3 Run package generation and verification for NuGet runtime artifacts.
- [x] 7.4 Run editor-assets package generation, verification, and smoke tests.
- [x] 7.5 Run VS Code extension packaging and verification for at least one target locally.
- [x] 7.6 Inspect workflow permissions to confirm build/PR jobs do not receive production registry credentials.
