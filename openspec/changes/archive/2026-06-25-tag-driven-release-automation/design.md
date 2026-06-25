## Context

The current pipeline builds verified artifacts on PRs and `main`, publishes packages from successful
`main` runs, and has a `release.yml` workflow that uploads build artifacts to an already-published
GitHub Release. Versioning is currently coupled to Nerdbank.GitVersioning through `version.json`,
`dotnet nbgv`, and scripts that read NBGV package variables.

The target model makes GitHub Releases the reviewed release boundary: tags create draft releases with
attached artifacts, and publishing the release triggers external registry writes from those attached
assets. Pull request and `main` builds remain useful for testing, but they do not write to package or
extension registries.

## Goals / Non-Goals

**Goals:**

- Make PR and `main` builds artifact-only for NuGet, npm editor assets, and VSIX outputs.
- Provide low-friction PR testing through uploaded artifacts and exact install commands.
- Use `v*` tags for compiler/runtime and editor-assets releases.
- Use `vscode-v*` tags for VS Code extension releases.
- Create draft GitHub Releases from tag builds and publish external registries only after a human
  publishes the release.
- Replace NBGV with MinVer CLI and explicit package-version staging.
- Keep production registry credentials out of build and PR workflows.

**Non-Goals:**

- Publishing PR builds to GitHub Packages, npm, NuGet.org, Marketplace, or Open VSX.
- Adding Rust crate or Rust CLI publication.
- Changing package identities such as `NxLang.Runtime`, `@nx-lang/language`, or `nx-language`.
- Introducing a separate public prerelease/insiders extension channel.

## Decisions

### Use GitHub Release assets as the production publish input

Tag workflows will build and verify artifacts, create or update a draft GitHub Release, and attach the
verified artifacts plus a small manifest/checksum file. Publish workflows will trigger on
`release.published`, validate the tag pattern and release assets, then publish the attached files.

Alternative considered: keep publishing from Actions artifacts. That keeps the current shape closer,
but it makes the human-reviewed GitHub Release less authoritative and requires publish workflows to
find the correct historical run.

### Keep package and VS Code release tracks separate

The package release track accepts only tags matching `^v\d+\.\d+\.\d+$`. The VS Code release track
accepts only tags matching `^vscode-v\d+\.\d+\.\d+$`. Workflows should use both trigger filters and
runtime regex guards because broad tag patterns can overlap in surprising ways.

Alternative considered: one release workflow that branches internally by tag. A shared helper script is
fine, but separate workflow entry points make permissions, artifacts, and failure modes easier to read.

### Replace NBGV with a small MinVer wrapper

Add MinVer CLI as a local .NET tool and remove NBGV from `.config/dotnet-tools.json`, `version.json`,
and CI scripts. Centralize version calculation in a small repository script, for example
`tools/versions/Get-ReleaseVersion.ps1`, that can emit:

- `PACKAGE_VERSION` for NuGet/MSBuild.
- `NPM_PACKAGE_VERSION` for `@nx-lang/language`.
- `VSCODE_EXTENSION_VERSION` for VSIX packaging.

For stable tag releases, the wrapper uses the relevant tag prefix (`v` or `vscode-v`). For PR and
`main` artifacts, it may combine MinVer output with GitHub run metadata to guarantee unique NuGet/npm
prerelease versions. For VSIX artifacts, it must keep `major.minor.patch` because Marketplace/VSIX
versions do not support SemVer prerelease suffixes.

Alternative considered: use MinVer MSBuild package everywhere. The CLI-plus-explicit-MSBuild-property
approach keeps version calculation visible in CI and avoids adding package references solely for
versioning.

### Make PR comments a trusted metadata workflow

Build workflows should upload tested artifacts and a small instruction/metadata artifact containing
the PR number, workflow run ID, artifact names, package versions, and suggested install commands. A
separate trusted workflow, triggered by `workflow_run`, can post or update a PR comment without
checking out or executing pull request code.

Alternative considered: post comments directly from `pull_request` jobs. That is simpler for same-repo
PRs but does not work reliably for forks with read-only tokens. `pull_request_target` can comment, but
it is easy to misuse by checking out and running untrusted code.

### Retire preview package registry publishing

The existing `preview` package publishing path should be removed from the supported workflow and docs.
PR testing uses downloaded artifacts. Production publish workflows keep the `production` environment
and registry-specific authentication.

Alternative considered: use GitHub Packages for preview NuGet/npm packages. That improves package
manager ergonomics but adds authentication, cleanup, and security policy work that is unnecessary for
the requested PR testing path.

## Risks / Trade-offs

- MinVer may not by itself produce the exact unique PR versions each ecosystem needs -> Mitigate with
  a thin wrapper that applies explicit package-version overrides, and stop to discuss if the wrapper
  becomes more complex than the NBGV setup it replaces.
- GitHub Release assets can drift if re-uploaded after a draft is reviewed -> Mitigate by publishing a
  manifest/checksum file and re-validating assets before registry writes.
- PR comment automation needs write permission while PR code is untrusted -> Mitigate with a
  `workflow_run` comment workflow that reads metadata artifacts and never executes PR code.
- VSIX versions cannot include prerelease suffixes -> Mitigate by treating PR VSIX artifacts as
  downloaded test builds and using `code --install-extension --force` in generated commands.
- Removing preview registry publishing may reduce package-manager-native test ergonomics -> Mitigate
  with exact `gh run download` commands and direct local package install commands.

## Migration Plan

1. Add MinVer CLI as the versioning tool and create a shared version wrapper script.
2. Update package and editor-assets scripts to consume explicit version environment variables or
   wrapper output instead of `dotnet nbgv`.
3. Update PR and `main` build workflows to produce verified `.nupkg`, `.snupkg`, npm `.tgz`, and VSIX
   artifacts without registry credentials.
4. Add metadata/instruction artifacts and a safe PR comment workflow for artifact testing commands.
5. Add tag-driven package and VS Code draft release creation with release asset manifests.
6. Update publish workflows to trigger on `release.published`, validate release assets, and publish
   from GitHub Release assets.
7. Remove or retire preview registry publishing inputs, variables, secrets, and documentation.
8. Update deployment setup and runbooks for tag creation, draft release review, release publication,
   PR artifact testing, repair, and rollback.

Rollback is straightforward before a release is published: revert the workflow changes and delete any
draft releases created by test tags. After a release is published, registry immutability still requires
the existing roll-forward posture with a higher fixed version.

## Open Questions

- Should release tags allow prerelease suffixes such as `v1.2.3-rc.1`, or should the first
  implementation intentionally support stable `major.minor.patch` tags only?
- Should the old `package-publish.yml` and `vscode-extension-publish.yml` names be preserved for
  continuity, or should new tag/release-specific workflow names replace them?
- Should draft releases include generated release notes immediately, or should maintainers own release
  notes before publishing?
