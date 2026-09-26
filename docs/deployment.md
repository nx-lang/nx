# Deployment Runbook

This runbook covers day-to-day package and VS Code extension publishing, and the website and
playground.
One-time environment, registry and hosting setup is in [deployment-setup.md](deployment-setup.md).

## Release Model

Pull requests and `main` builds are artifact-only. They build, verify, and upload NuGet, npm
(editor assets and the workspace packages), and VSIX artifacts without public registry credentials.

Production publishing has two reviewed release tracks:

- Package releases use tags like `v1.2.3`. The tag workflow creates a draft GitHub Release with
  verified `NxLang.Sdk` `.nupkg` and `.snupkg` assets, one npm tarball per package — the
  `@nx-lang/language` editor assets and the workspace packages `@nx-lang/language-protocol`,
  `@nx-lang/language-core`, `@nx-lang/language-client`, `@nx-lang/ir-runtime`, `@nx-lang/sdk-wasm`
  (with the WebAssembly module inside) and `@nx-lang/monaco` — a release manifest, and checksums.
  Every npm package carries the tag's version, and a workspace package's dependency on another is
  pinned to that version.
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
5. Inspect the attached `.nupkg`, `.snupkg`, npm `.tgz` files, `release-manifest.json`, and
   `release-checksums.txt` assets.
6. Confirm the manifest tag, version, commit, artifact names, and checksums match the intended
   release.
7. Publish the GitHub Release.
8. Approve the `production` environment deployment if reviewers are required.
9. Confirm publication on NuGet.org and npm: `npm view @nx-lang/sdk-wasm version` and the same
   for each package should answer with the tag's version.

The publish job publishes the npm tarballs in dependency order (`scripts/publish-packages.mjs`), so
a consumer installing a just-published package finds its `@nx-lang/*` dependencies on the registry
already, and skips any version the registry has.

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

npm workspace packages test (the tarballs depend on each other by version, so add them together):

```bash
gh run download <run-id> -R nx-lang/nx -n npm-packages -D nx-npm-packages
mkdir nx-npm-test
cd nx-npm-test
npm init -y
pnpm add ../nx-npm-packages/*.tgz
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
tar -tf nx-lang-sdk-wasm-*.tgz | grep nx.wasm
unzip -l nx-language-*.vsix
sha256sum -c release-checksums.txt
```

For the SDK package, `tools/packaging/Test-NxSdkPackage.ps1` verifies metadata and native SDK
assets. For editor assets, run `pnpm run verify:package` and `pnpm run smoke:package` from
`src/vscode`. For the workspace packages, `pnpm run verify:packages` at the repository root packs
each one and installs it into a scratch project, and `node scripts/pack-packages.mjs <version>
<dir>` packs them all at one version and checks the packed manifests.

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
node scripts/publish-packages.mjs <directory-with-the-release-tgz-files> --version 1.2.3
pnpm run publish:vsce -- nx-language-*.vsix
pnpm run publish:ovsx -- nx-language-*.vsix
```

The npm script publishes in dependency order and skips versions the registry already has, so it
can be pointed at the whole set of downloaded `.tgz` files after a partial publish.

Do not rebuild package contents for a repair publish unless the fix requires a new higher version.

## Higher-Version Fixes

Public registry versions are immutable. When a published artifact is bad:

1. Fix the source issue.
2. Publish a higher version through the appropriate tag-driven release track.
3. Unlist or deprecate the bad NuGet, npm, or extension version where useful.
4. Update release notes or documentation to steer users to the fixed version.

## Website And Playground

`https://nxlang.org` is two Cloudflare Workers that serve static files, with no origin server:

| Worker | Serves | Built from | Deployed by |
|---|---|---|---|
| `nxlang-website` | everything except `/playground` | `sites/website` | `.github/workflows/deploy-website.yml` |
| `nxlang-playground` | `/playground` and everything under it | `sites/playground` | `.github/workflows/deploy-playground.yml` |

Each Worker's `wrangler.jsonc` sits beside its site and declares its routes. The playground's routes
are more specific than the website's `nxlang.org/*`, so Cloudflare sends `/playground` requests to
it. Neither is part of the tag-driven release tracks above.

### Deploy

Both workflows deploy every push to `main` that touches their site, and either can be run by hand
from the Actions tab. Each authenticates with `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` in
the `production` GitHub environment.

- **Website.** Installs only the site's dependencies, runs `astro build`, which fails on a broken
  internal link, then `wrangler deploy`. It needs no Rust. The docs' code-block check, which does,
  runs in `build.yml` on every pull request, so a page reaches `main` already checked. The smoke test
  fetches `/` and a docs page and looks for the commit in their `nx-build` meta tag.
- **Playground.** Builds and tests the whole workspace, the WebAssembly compiler included, keeps
  `sites/playground/dist` as an artifact, and deploys that with `wrangler deploy`. The smoke test
  checks that the shell names this build's entry script and that the compiler module answers.

A Workers deploy is atomic: the previous version keeps serving until every file of the new one is
uploaded. A failed build or test stops the job before the deploy.

To deploy by hand, from the site's folder after building it, with a token in the environment:

```bash
cd sites/website            # or sites/playground
npx wrangler@4.141.0 deploy
```

### Verify

```bash
curl -sI https://nxlang.org/                                   # 200, the landing page
curl -sI https://nxlang.org/language-tour/types/               # 200
curl -sI https://nxlang.org/nope                               # 404, the site's not-found page
curl -sI https://nxlang.org/playground                         # 200, cache-control: no-cache
curl -sI https://nxlang.org/playground/shapes                  # 200, the same shell
curl -sI https://nxlang.org/playground/assets/nx-<hash>.wasm   # 200, immutable for a year
curl -sI https://nxlang.org/playground/assets/missing.js       # 404, not the shell
```

Then open the playground, choose an example and edit it: the result follows and hover answers.

### Roll back

From the site's folder, with a token in the environment:

```bash
npx wrangler@4.141.0 deployments list    # the recent versions
npx wrangler@4.141.0 rollback            # back to the previous version, or name one
```

A rollback takes effect at once. The playground's immutable assets are safe across it, because the
shell is never cached and names the assets of whichever version is live.

### Change the hosting configuration

Routes, the static-assets settings and the playground's Worker script are in each site's
`wrangler.jsonc` and deploy with it. The cache headers are in `sites/playground/_headers` and
`sites/website/public/_headers`. Zone settings, DNS records and the redirect rule are in the dashboard
and recorded in [deployment-setup.md](deployment-setup.md#website-and-playground-hosting). Change
them there first, then in the dashboard, so the doc stays the record.
