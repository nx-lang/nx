# Deployment Runbook

This runbook covers day-to-day package publishing. One-time environment and registry setup is in
[deployment-setup.md](deployment-setup.md).

## Release Model

Pull requests build, verify, and upload package artifacts without public registry credentials.
Trusted preview publishing is optional and uses the `preview` environment. Trusted `main` builds can
publish production packages through the `production` environment after all package checks pass.

Public package versions are immutable. If a bad package is published, roll forward with a higher
version and unlist or deprecate the bad version where the registry supports it.

## Publish A New Release

1. Merge the release change to `main`.
2. Confirm the Build workflow completed package assembly, package inspection, and RID smoke tests.
3. Inspect uploaded artifacts:
   - `deployables-Complete`: verified `NxLang.Runtime.*.nupkg`.
   - `editor-assets-package`: verified `nx-lang-language-*.tgz`.
   - VS Code extension workflow artifacts: one verified `.vsix` per platform target.
4. Approve the `production` environment deployment if reviewers are required.
5. Confirm package publication in NuGet.org, npm, Visual Studio Marketplace, and Open VSX.

## Artifact Inspection

Use the workflow artifacts from the successful run rather than rebuilding locally:

```bash
unzip -l NxLang.Runtime.*.nupkg
tar -tf nx-lang-language-*.tgz
unzip -l nx-language-*.vsix
```

For the runtime package, `tools/packaging/Test-NxRuntimePackage.ps1` verifies metadata and native
runtime assets. For editor assets, run `pnpm run verify:package` and `pnpm run smoke:package` from
`src/vscode`.

## Repair A Partial Publish

If one registry publish succeeds and another fails, download the artifacts from the successful
workflow run and republish the same artifact.

NuGet fallback repair:

```bash
dotnet nuget push NxLang.Runtime.*.nupkg --source https://api.nuget.org/v3/index.json --api-key "$NUGET_API_KEY" --skip-duplicate
```

npm trusted-publishing repair:

```bash
gh workflow run build.yml --ref main
```

For a local emergency repair, publish the same tarball with an interactive maintainer session rather
than a CI `NPM_TOKEN`.

VS Code registry repair from `src/vscode`:

```bash
pnpm run publish:vsce -- nx-language-*.vsix
pnpm run publish:ovsx -- nx-language-*.vsix
```

Do not rebuild package contents for a repair publish unless the fix requires a new higher version.

## Higher-Version Fixes

When a published artifact is bad:

1. Fix the source issue.
2. Let versioning produce a higher package version.
3. Publish the fixed version through `main` and the `production` environment.
4. Unlist or deprecate the bad NuGet/npm version where useful.
5. Update release notes or documentation to steer users to the fixed version.

## Preview Publishing

Preview publishing is off by default. Run the Build workflow manually with `publish_preview`
enabled, or set `PUBLISH_PREVIEW_PACKAGES=true` in the `preview` environment, after configuring
preview feed variables. Preview packages must use credentials scoped to preview feeds and must never
use production registry credentials.
