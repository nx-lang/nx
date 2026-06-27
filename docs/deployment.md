# Deployment Runbook

This runbook covers day-to-day package and VS Code extension publishing. One-time environment and
registry setup is in [deployment-setup.md](deployment-setup.md).

## Release Model

Pull requests and `main` builds are artifact-only. They build, verify, and upload NuGet, npm
editor-assets, and VSIX artifacts without public registry credentials.

Production publishing has two reviewed release tracks:

- Package releases use tags like `v1.2.3`. The tag workflow creates a draft GitHub Release with
  verified `NxLang.Sdk` `.nupkg` and `.snupkg` assets, the `@nx-lang/language` npm tarball, a
  release manifest, and checksums.
- VS Code extension releases use tags like `vscode-v1.2.3`. The tag workflow creates a draft GitHub
  Release with verified VSIX assets, a release manifest, and checksums.

Publishing the GitHub Release is the production gate. Published package releases trigger
`package-publish.yml`; published VS Code releases trigger `vscode-extension-publish.yml`. Those
workflows validate the release assets and publish the attached files without rebuilding package
contents.

Rust tooling publication for `nxlang`, `nx-lsp`, and Rust crates is not part of this release
pipeline yet.

## Versioning Rules

Version calculation is centralized in `tools/versions/Get-ReleaseVersion.ps1` and uses the local
MinVer CLI tool.

- `v<major>.<minor>.<patch>` package tags produce stable NuGet and npm versions with no prerelease
  suffix.
- `vscode-v<major>.<minor>.<patch>` tags produce registry-valid VSIX versions.
- Pull request package artifacts use unique prerelease versions such as
  `0.1.0-pr.<pr>.<run>.<attempt>`.
- `main` package artifacts use CI prerelease versions such as `0.1.0-ci.<run>.<attempt>`.
- VSIX test artifacts use `major.minor.patch` because VSIX registries do not accept SemVer
  prerelease suffixes.

Only stable `major.minor.patch` release tags are supported in this implementation.

## Publish A Package Release

1. Merge the release change to `main`.
2. Create and push a package release tag:
   ```bash
   git tag v1.2.3
   git push origin v1.2.3
   ```
3. Wait for the Package release workflow to finish.
4. Open the draft GitHub Release for `v1.2.3`.
5. Inspect the attached `.nupkg`, `.snupkg`, npm `.tgz`, `release-manifest.json`, and
   `release-checksums.txt` assets.
6. Confirm the manifest tag, version, commit, artifact names, and checksums match the intended
   release.
7. Publish the GitHub Release.
8. Approve the `production` environment deployment if reviewers are required.
9. Confirm publication on NuGet.org and npm.

## Publish A VS Code Extension Release

1. Merge the extension release change to `main`.
2. Create and push a VS Code extension release tag:
   ```bash
   git tag vscode-v1.2.3
   git push origin vscode-v1.2.3
   ```
3. Wait for the VS Code extension release workflow to finish.
4. Open the draft GitHub Release for `vscode-v1.2.3`.
5. Inspect the attached VSIX assets, `release-manifest.json`, and `release-checksums.txt`.
6. Confirm every VSIX contains publisher `nx-lang`, extension `nx-language`, and version `1.2.3`.
7. Publish the GitHub Release.
8. Approve the `production` environment deployment if reviewers are required.
9. Confirm publication in the Visual Studio Marketplace and Open VSX.

## Pull Request Artifact Testing

The trusted PR artifact comment workflow posts commands after successful PR artifact builds. Use the
specific workflow run ID from the comment so the downloaded files match the verified build.

NuGet SDK package test:

```bash
gh run download <run-id> -R nx-lang/nx -n deployables-Complete -D nx-package-artifacts
dotnet new console -n nx-sdk-test
dotnet add nx-sdk-test/nx-sdk-test.csproj package NxLang.Sdk --version <package-version> --source "$(pwd)/nx-package-artifacts"
```

npm editor-assets package test:

```bash
gh run download <run-id> -R nx-lang/nx -n editor-assets-package -D nx-editor-assets
mkdir nx-editor-assets-test
cd nx-editor-assets-test
npm init -y
pnpm add ../nx-editor-assets/*.tgz
```

VSIX test:

```bash
gh run download <run-id> -R nx-lang/nx -p 'vscode-vsix-*' -D nx-vsix-artifacts
find nx-vsix-artifacts -name '*.vsix' -type f -print0 | xargs -0 -I{} code --install-extension '{}' --force
```

## Artifact Inspection

Use workflow or GitHub Release artifacts rather than rebuilding locally:

```bash
unzip -l NxLang.Sdk.*.nupkg
unzip -l NxLang.Sdk.*.snupkg
tar -tf nx-lang-language-*.tgz
unzip -l nx-language-*.vsix
sha256sum -c release-checksums.txt
```

For the SDK package, `tools/packaging/Test-NxSdkPackage.ps1` verifies metadata and native SDK
assets. For editor assets, run `pnpm run verify:package` and `pnpm run smoke:package` from
`src/vscode`.

## Repair A Partial Publish

Repair uses the same GitHub Release assets that were already reviewed and partially published.

Package registry repair:

```bash
gh workflow run package-publish.yml --ref main -f release_tag=v1.2.3
```

VS Code registry repair:

```bash
gh workflow run vscode-extension-publish.yml --ref main -f release_tag=vscode-v1.2.3
```

The package publish workflow validates the release assets before registry writes and uses
idempotent duplicate-version behavior where supported. The VSIX publish script checks each registry
separately and skips an already-published extension version, so a repair can fill in a missing
Marketplace or Open VSX publication.

For a local emergency repair from already-downloaded assets:

```bash
dotnet nuget push NxLang.Sdk.*.nupkg --source https://api.nuget.org/v3/index.json --api-key "$NUGET_API_KEY" --skip-duplicate
pnpm run publish:vsce -- nx-language-*.vsix
pnpm run publish:ovsx -- nx-language-*.vsix
```

Do not rebuild package contents for a repair publish unless the fix requires a new higher version.

## Higher-Version Fixes

Public registry versions are immutable. When a published artifact is bad:

1. Fix the source issue.
2. Publish a higher version through the appropriate tag-driven release track.
3. Unlist or deprecate the bad NuGet, npm, or extension version where useful.
4. Update release notes or documentation to steer users to the fixed version.
