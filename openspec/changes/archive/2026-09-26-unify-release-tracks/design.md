## Context

Two tag-driven tracks exist today, each with a validate job, a build matrix, a draft-release job and
a publish workflow:

| | Package track | Extension track |
|---|---|---|
| Tag | `v<x.y.z>` | `vscode-v<x.y.z>` |
| Draft release | `release.yml`: NuGet `.nupkg`/`.snupkg`, one `.tgz` per npm package, manifest, checksums; titled `NX packages <version>` | `vscode-release.yml`: one VSIX per target (linux-x64, darwin-arm64, win32-x64), manifest, checksums; titled `NX VS Code <version>` |
| Publish | `package-publish.yml`: NuGet (trusted publishing) and npm (trusted publishing, provenance) | `vscode-extension-publish.yml`: Marketplace (`azure/login` as `nx-vscode-publisher`, `vsce --azure-credential`) and Open VSX (`OVSX_PAT`), both with `--skip-duplicate` |

Both publish workflows run in the `production` environment, which the Marketplace identity's
federated credential trusts (`repo:nx-lang/nx:environment:production`). `release.yml` triggers on
`v*`, which also matches `vscode-v*`, so its validate job skips those tags explicitly, and every
later job is gated on the skip.
`Get-ReleaseVersion.ps1` takes `-Track package|vscode`, and derives the VSIX version from the tag
only on the `vscode` track. `vscode-extension.yml` builds VSIX files for pull requests and `main`,
and `pr-artifact-comment.yml` posts their install commands; neither depends on the tracks.

This change should follow `launch-nxlang-website`, which publishes the first extension release,
`vscode-v0.1.0`, on the extension track, and whose delta to "VS Code extension publishing
credentials" this change's delta builds on. Archive that change first.

## Goals / Non-Goals

**Goals:**
- One tag, one version, one draft release and one publication for every artifact.
- Keep each registry repairable on its own from the release's assets.
- Reuse the jobs that already work: the VSIX matrix, the Marketplace and Open VSX steps, the release
  asset validation.

**Non-Goals:**
- Changing what is built. The VSIX targets, the npm package set, the NuGet package and the PR/`main`
  artifacts stay as they are; the PR/`main` VSIX build only moves into the shared workflow.
- Publishing `nxlang` or Rust crates.
- A release notes generator. Notes stay the short review instructions they are today.

## Decisions

### Fold the extension jobs into the package workflows

The VSIX build moves into a reusable workflow, `vscode-vsix.yml`, which holds the target matrix and
the build steps once. `vscode-extension.yml` calls it for pull requests and `main`, and `release.yml`
calls it with the release tag, which it checks out and stages as the VSIX version. The
draft-release job downloads one `vscode-vsix-<target>` artifact per target, requires each to hold
`nx-language-<target>-<version>.vsix`, lists them in the one manifest and checksum file, and titles
the release `NX <version>`. It learns the targets from the artifacts, so adding a target to the
matrix is the only edit. `vscode-release.yml` is deleted.

`package-publish.yml` gains a second publish job, `publish-extension`, beside the existing package
job. Both need the same `prepare` job, both run in `production`, and each downloads and validates the
release assets it publishes. The extension job carries the Marketplace and Open VSX steps from
`vscode-extension-publish.yml` (credential check, `azure/login`, `pnpm run publish:vsce`,
`pnpm run publish:ovsx`), which is then deleted.

*Why two jobs rather than one:* a registry outage on one side must not stop the other, and a rerun of
the failed job repairs only its registries. Both jobs are idempotent (`--skip-duplicate` for NuGet,
the Marketplace and Open VSX; `publish-packages.mjs` skips an npm version that already exists), so a
whole-workflow repair through `release_tag` is also safe.

*Alternative:* keep the two workflows and have `release.yml` call `vscode-release.yml` as a reusable
workflow. It keeps two files that must agree on the manifest and the release title, for no gain.
Sharing only the VSIX build, which pull requests and releases both need, avoids that while keeping the
target list in one place.

### One version, from the tag

`Get-ReleaseVersion.ps1` drops `-Track`. On a `v<x.y.z>` tag, the package version, the npm version
and the VSIX version are all `x.y.z`. On pull requests and `main`, the VSIX version stays the core of
the preview version (`0.5.1` for `0.5.1-pr.12.3`), because the Marketplace and VS Code accept only
`major.minor.patch`. A `vscode-v*` tag is no longer a release tag: the script treats it as an
unrecognized ref, and `release.yml` triggers only on `v<x.y.z>` tags, so a stray push starts no run.

*The extension's version jumps* from 0.1.0 to the first unified version. Registries accept any higher
version, and at 0.x no one relies on the extension's own numbering.

### One title, one "Latest"

The draft is titled `NX <version>`, short enough that GitHub's release list shows the version. Each
published release is GitHub's "Latest" as usual, and now covers everything, so the extension and the
packages no longer compete for it.

### Keep the Marketplace identity and its check as they are

The publish job keeps `environment: production`, so the federated credential needs no change.
`marketplace-identity.yml` stays: it is the setup and pre-release check for the identity, not part
of a track.

## Risks / Trade-offs

- **[A release rebuilds the VSIX files even when only the SDK changed]** → Accepted. The extension
  updates quietly in VS Code, and the version then says which NX it understands.
- **[The release workflow gets longer: the VSIX matrix builds the language server on three
  platforms]** → The matrix runs in parallel with the native SDK and npm jobs; the draft waits for
  the slowest, which today is the Windows VSIX at about four minutes.
- **[A failure in one publish job leaves a half-published release]** → The failed job is rerun
  alone, or the workflow is dispatched with `release_tag`; each registry skips what it already has.
- **[Someone pushes a `vscode-v*` tag from habit]** → `release.yml`'s trigger doesn't match it
  and nothing else triggers on it. The runbook says the track is retired.

## Migration Plan

1. Publish `vscode-v0.1.0` on the old track, as `launch-nxlang-website` does, and archive that change.
2. Land this change. The first `v*` tag after it (for example `v0.5.0`) releases the packages and the
   extension together. Check the draft for the three VSIX files at the release version beside the
   package artifacts.
3. Publish the release, and confirm npm, NuGet, the Marketplace and Open VSX each report the version.

Rollback: restore `vscode-release.yml` and `vscode-extension-publish.yml` from history and the
`-Track` parameter. No registry state depends on the tracks.
