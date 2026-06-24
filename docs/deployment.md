# Deployment Runbook

This runbook covers day-to-day package publishing. One-time environment and registry setup is in
[deployment-setup.md](deployment-setup.md).

## Release Model

Pull requests build, verify, and upload package artifacts without public registry credentials. The
Build workflow proves the NuGet and editor-assets package bits; the Publish packages workflow publishes
those already-verified artifacts. VSIX publishing remains in the VS Code extension workflow. Trusted
preview publishing is optional and uses the `preview` environment. Successful trusted `main` Build runs
trigger production package publishing through the `production` environment.

Public package versions are immutable. If a bad package is published, roll forward with a higher
version and unlist or deprecate the bad version where the registry supports it.

## Publish A New Release

1. Merge the release change to `main`.
2. Confirm the Build workflow completed package assembly, package inspection, and RID smoke tests.
3. Inspect uploaded artifacts:
   - `deployables-Complete`: verified `NxLang.Runtime.*.nupkg`.
   - `editor-assets-package`: verified `nx-lang-language-*.tgz`.
   - VS Code extension workflow artifacts: one verified `.vsix` per platform target.
4. Confirm the Publish packages workflow started from the successful `main` Build run.
5. Approve the `production` environment deployment if reviewers are required.
6. Confirm package publication in NuGet.org, npm, Visual Studio Marketplace, and Open VSX.

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
gh workflow run package-publish.yml --ref main -f target_environment=production -f artifact_run_id=<build-run-id>
```

For a local emergency repair, publish the same tarball with an interactive maintainer session rather
than a CI `NPM_TOKEN`.

VS Code registry repair from `src/vscode`:

```bash
pnpm run publish:vsce -- nx-language-*.vsix
pnpm run publish:ovsx -- nx-language-*.vsix
```

Do not rebuild package contents for a repair publish unless the fix requires a new higher version.
If GitHub Release asset upload fails, upload the same Build workflow artifacts to the GitHub Release;
registry repair still belongs in the Publish packages workflow.

## Higher-Version Fixes

When a published artifact is bad:

1. Fix the source issue.
2. Let versioning produce a higher package version.
3. Publish the fixed version through `main` and the `production` environment.
4. Unlist or deprecate the bad NuGet/npm version where useful.
5. Update release notes or documentation to steer users to the fixed version.

## Preview Publishing

Preview publishing is off by default. Run the Publish packages workflow manually after configuring
preview feed variables. The workflow publishes artifacts from a successful PR or branch Build workflow
run:

1. Let the PR or branch Build workflow complete successfully.
2. Copy the source run ID from the run URL, or run `gh run list --workflow build.yml`.
   The source run must be a successful Build workflow run from this repository with package artifacts.
3. Run the Publish packages workflow manually on a ref that contains the publish workflow file.
4. Set `target_environment=preview`.
5. Set `artifact_run_id` to the successful source run ID.
6. Approve the `preview` environment deployment if reviewers are required.
7. Confirm the preview NuGet feed and npm-compatible registry received the expected package versions.

The Publish packages workflow validates that the source run completed successfully, downloads
`deployables-Complete` and `editor-assets-package` from that run, and publishes those exact artifacts.
Preview packages must use credentials scoped to preview feeds and must never use production registry
credentials.
