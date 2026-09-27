## 1. Before starting

- [x] 1.1 Confirm `vscode-v0.1.0` is published to both registries and `launch-nxlang-website` is
  archived, so this change's delta to "VS Code extension publishing credentials" applies to the
  archived text. Verify `openspec validate unify-release-tracks --strict` still passes after the
  archive.

## 2. One version

- [x] 2.1 Remove `-Track` from `tools/versions/Get-ReleaseVersion.ps1`: a `v<x.y.z>` tag is the only
  release tag, the VSIX version on it is the release version, and a `vscode-v*` ref is treated as not
  a release tag. Update its callers in `build.yml`, `release.yml`, `vscode-extension.yml`,
  `src/vscode/scripts/package-language.mjs` and `stage-vsix-version.mjs`. Verify by running the
  script for a `v0.5.0` tag ref (every version is `0.5.0`), a pull request ref (VSIX version is the
  core of the preview version), `main`, and a `vscode-v0.1.0` ref (not treated as a release).

## 3. One draft release

- [x] 3.1 Move the VSIX matrix job from `vscode-release.yml` and `vscode-extension.yml` into a
  reusable workflow, `vscode-vsix.yml`, that both `vscode-extension.yml` and `release.yml` call,
  staging the release version with the updated script on a release tag. Have the draft-release job
  download the VSIX artifacts, require one at the release version per built target, list them
  in the one manifest and checksum file, and title the release `NX <version>` with notes that name
  every artifact kind. Narrow the tag trigger to `v<x.y.z>`, so a `vscode-v*` tag starts no run,
  and drop the validate job's skip and its gating. Delete `vscode-release.yml`.
  Verify with `actionlint` (or a YAML parse) and a `workflow_dispatch` dry run on a disposable tag in
  a fork, or by reading the job graph if no fork is available.
- [x] 3.2 Update `docs/deployment.md` for the one release: the runbook creates and inspects one
  draft, which now includes the VSIX files; drop the `vscode-v*` runbook. Verify against the
  "Maintainer follows the release runbook" scenario.

## 4. One publication

- [x] 4.1 Add a `publish-extension` job to `package-publish.yml`, needing `prepare`, in `production`,
  with `id-token: write`: download and validate the release's VSIX assets, check `AZURE_CLIENT_ID`,
  `AZURE_TENANT_ID` and `OVSX_PAT`, sign in with `azure/login`, and run `publish:vsce` and
  `publish:ovsx` for each VSIX. Remove the `prepare` job's skip of extension releases. Delete
  `vscode-extension-publish.yml`. Verify with a YAML parse and by checking that both publish jobs
  appear in `prepare`'s dependents.
- [x] 4.2 Update `docs/deployment-setup.md` (the extension secrets belong to the publish workflow, and
  the Marketplace identity section no longer names a separate workflow) and `src/vscode/CONTRIBUTING.md`
  (the extension ships with each `v*` release at the package version; the `vscode-v*` sections go).
  Verify `git grep -n "vscode-v" -- ':!openspec/changes/archive'` finds only history notes and the
  `vscode-vsix-*` artifact names of pull request builds.

## 5. First unified release

- [ ] 5.1 Tag the next release (for example `v0.5.0`; a maintainer pushes it). Verify the draft
  titled `NX 0.5.0` holds the NuGet package and symbols, every npm tarball, the three VSIX files at
  `0.5.0` with their platform's language server, the manifest and the checksums, and that
  `sha256sum -c` passes.
- [ ] 5.2 Publish the release. Verify npm, NuGet, the Marketplace and Open VSX each report `0.5.0`
  (for the extension, all three platforms), and that the release is GitHub's latest.
