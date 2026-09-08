# Deployment Runbook

This runbook covers day-to-day package and VS Code extension publishing, and the playground site.
One-time environment, registry and hosting setup is in [deployment-setup.md](deployment-setup.md).

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

## Playground Site

The NX Playground at `https://nxlang.org/playground` is the `sites/playground` workspace member,
built and run as one Docker image on Railway behind Cloudflare. It is not part of the tag-driven
release tracks above.

### Deploy

`.github/workflows/deploy-playground.yml` deploys every push to `main` that touches
`sites/playground/`, a package the site depends on, the Rust crates or the workspace manifests
(the workflow's `paths` list). It is the same shape as the other Railway-hosted sites: the runner
builds and tests the workspace, then uploads the checkout with `railway up` under the project
token in the `production` GitHub environment. Railway builds `sites/playground/Dockerfile` from
the repository root, polls `/playground/api/health` on the new container, and switches traffic
only once it answers `200`; the job waits for the deployment to report success, then checks the
health endpoint and the gallery through Cloudflare. A failed check, build or health check leaves
the previous deployment serving, and the job summary names the deployment to roll back to.
Nothing else deploys the service: the Railway GitHub App is not installed and the service has no
repository source.

To redeploy without a change, run the workflow from the Actions tab. To deploy something that is
not on `main`, from the repository root with the Railway CLI linked to the project:

```bash
railway up --service playground --environment production --ci
```

### Verify

```bash
curl -sI https://nxlang.org/                          # 302 to /playground
curl -sI https://nxlang.org/playground                # 200, cache-control: no-cache
curl -s  https://nxlang.org/playground/api/health     # {"ok":true}
curl -sI https://nxlang.org/playground/assets/<hashed asset>   # cf-cache-status: HIT on the second request
```

Then open an example (`https://nxlang.org/playground/shapes`), edit it, and confirm the drawing
follows and hover answers. Railway's deployment log shows `NX playground listening on ...` on
start; a line beginning `watchdog:` means the main thread stopped answering and the process ended
itself, after which the restart policy (`ALWAYS`, no retry budget) started a fresh one. Repeated
`watchdog:` lines mean something is provoking the hang and are worth reading the request log for.

### Roll back

Railway keeps previous deployments. In the service's deployment list, choose the last good one and
**Redeploy**; it becomes live once its health check passes. Nothing on Cloudflare needs to change.
The immutable asset cache is safe across a rollback because the shell is never cached and names
the assets of whichever build is live.

### Change the hosting configuration

The service's build, health check and restart settings are declared in `.railway/railway.ts`.
Edit the file, then from the repository root with the CLI linked to the `nxlang` project:

```bash
railway config plan      # read-only preview of what would change on Railway
railway config apply     # applies it, after showing the plan once more
```

Do not change those settings in the dashboard; the next apply would revert them.

### Change the edge

Every Cloudflare record and rule the site depends on, with its value, is listed in
[deployment-setup.md](deployment-setup.md#playground-site-hosting). Change them there first, then
in the dashboard, so the doc stays the record.
