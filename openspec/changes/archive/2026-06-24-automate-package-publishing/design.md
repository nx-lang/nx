## Context

The repository already has most package production mechanics:

- `.github/workflows/build.yml` builds and tests on Linux, macOS, and Windows, stages native
  `nx_ffi` assets, assembles a complete `NxLang.Runtime` NuGet package, smoke-tests it on each
  supported RID, and uploads `deployables-Complete`.
- The same workflow produces and verifies the `@nx-lang/language` npm tarball.
- `.github/workflows/vscode-extension.yml` verifies VS Code extension packaging on PR/main and
  publishes platform-specific VSIX packages from `vscode-v<version>` tags.
- `.github/workflows/release.yml` can attach artifacts from a selected successful build to a GitHub
  Release, but package registry writes now live in the Publish packages workflow.

The remaining gap is release operation shape. Package registries are more constrained than SaaS
deployments: public package versions are immutable, untrusted PRs must not receive write tokens, and
the VS Code Marketplace's pre-release channel still requires monotonically distinct
`major.minor.patch` extension versions. The desired workflow should feel like the SaaS
preview/production environments while respecting package-registry safety.

## Goals / Non-Goals

**Goals:**

- Provide one documented CI release path for NuGet, VS Code Marketplace/Open VSX, and
  `@nx-lang/language`.
- Use GitHub environments named `preview` and `production` so package publishing can be protected
  and audited like other deployment workflows.
- Make PR builds produce preview artifacts, and allow trusted preview/test registry publishing where
  it is useful and safe.
- Make trusted `main` builds capable of publishing production packages automatically after package
  verification, smoke tests, version checks, and environment gates pass.
- Prefer OIDC/trusted publishing for registries that support it, with token secrets documented as
  fallbacks or for registries that still require PATs.
- Fix in-repo metadata defects discovered during release readiness review.
- Add `docs/deployment-setup.md` as the maintainer-facing one-time setup checklist for GitHub
  environments, secrets, trusted publisher policies, and registry ownership.
- Add `docs/deployment.md` as the ongoing deployment runbook for routine release workflows, including
  how to publish new releases, inspect artifacts, repair partial publishes, and handle rollback or
  unlist/deprecate scenarios.

**Non-Goals:**

- Building a new package registry or private feed service.
- Publishing public packages from arbitrary fork PRs.
- Changing the managed runtime API, native FFI ABI, editor language grammar, or LSP protocol.
- Solving stable 1.0 release governance beyond documenting environment gates and version policy.

## Decisions

### Use a Package Release Workflow With Environment Gates

Create or refactor CI so package publication happens in a dedicated Publish packages workflow whose
jobs reference explicit GitHub environments. The Build workflow remains responsible for package
assembly, verification, smoke tests, and artifact upload:

- `preview`: PR and non-main trusted branch outputs. Default behavior is artifact upload only.
  Optional preview registry publishing may target GitHub Packages because it can use the
  repository-scoped `GITHUB_TOKEN` and avoids leaking public registry credentials to PR contexts.
  Manual preview publishing should take an explicit successful workflow run ID when publishing a
  previously validated PR or branch package, then download and publish that run's verified artifacts
  instead of rebuilding package contents.
- `production`: trusted `main` publishing to public registries after a successful Build workflow run
  completes all required packaging and smoke tests. The Publish packages workflow can run without
  approval while NX is early-stage and later gain required reviewers without redesigning the workflow.

Alternative considered: keep tag/GitHub Release publishing as the only production path. That is
safe, but it does not match the desired "merge to main starts publication" workflow and leaves
normal release automation split across workflows.

Alternative considered: keep preview and production package publishing in `.github/workflows/build.yml`
and skip build jobs when reusing artifacts. That works, but it makes the Build workflow carry
deployment-only inputs and conditions. A separate Publish packages workflow keeps PR CI artifact-only
and gives publishing its own permissions, environment gates, and audit trail.

### Publish Built Artifacts, Not Rebuilt Packages

Publish packages workflow jobs should consume the verified artifacts from the build/package jobs:

- NuGet: `deployables-Complete/NxLang.Runtime.*.nupkg` after `Test-NxRuntimePackage.ps1` and
  `SmokeTest-NxRuntimePackage.ps1`.
- npm editor assets: verified `editor-assets-package/*.tgz`.
- VS Code: platform-specific VSIX artifacts produced by the package job for each target.

The release workflow must not implicitly rebuild a package at publish time because that can drift from
the inspected artifact.

Alternative considered: run `dotnet pack`, `npm pack`, or `vsce publish` directly in publish jobs.
That is simpler but weakens traceability and can publish different bits from the tested artifacts.

### Prefer Trusted Publishing Where Available

Use OIDC-based trusted publishing for:

- NuGet.org `NxLang.Runtime`, through `NuGet/login` when available for the package owner.
- npm `@nx-lang/language`, through npm trusted publishing with the production workflow and
  environment configured as the trusted publisher.

Keep token-based publishing support documented as a fallback:

- `NUGET_API_KEY` or scoped API key fallback if NuGet trusted publishing is unavailable.
- `VSCE_PAT` and `OVSX_PAT` remain required unless those registries add supported keyless CI
  publishing.

Alternative considered: keep only long-lived API keys. That is compatible with the current workflow
but creates rotation burden and larger blast radius than OIDC policies.

### Derive CI Package Versions Consistently

NuGet and editor-assets already use Nerdbank.GitVersioning-derived package versions. VS Code
extension packaging should gain an equivalent CI-controlled version path so `main` can publish unique
versions without committing a package.json edit for every merge.

The implementation should stage or patch the VS Code manifest during packaging, leaving the checked-in
manifest as the development manifest. Version checks should fail before publishing if the computed
VSIX version is already present in a target registry or if the version does not satisfy registry
constraints.

Alternative considered: require manual version bumps and tags for VS Code forever. That remains a
useful repair path, but it blocks fully automated main publishing.

### Keep Public Preview Publishing Conservative

PR builds should always package and upload artifacts. They should not publish to public
NuGet/npm/Marketplace registries by default because public versions are immutable and fork PRs cannot
safely receive secrets. If preview registry publishing is enabled, it should use GitHub Packages or
another internal/test feed under the `preview` environment and only for trusted contexts.

Alternative considered: publish every PR to public prerelease channels. That creates package-version
noise, consumes immutable versions, and increases credential exposure risk.

### Split One-Time Setup From Ongoing Deployment Operations

Add `docs/deployment-setup.md` for one-time CI and registry setup:

- Recommended workflow model and rationale.
- GitHub environments to create and suggested protection rules.
- Registry ownership/publisher setup.
- Required secrets, variables, and trusted publisher policies.

Add `docs/deployment.md` as the day-to-day runbook:

- How PR preview builds, trusted preview publishes, and production main publishes work across the
  Build and Publish packages workflows.
- How to publish preview packages from an already-successful Build workflow run without rebuilding
  package artifacts.
- How to publish a new release and what checks should be reviewed.
- How to find and inspect package artifacts for NuGet, npm editor assets, and VSIX packages.
- How to run manual repair publishing with the same artifacts after partial registry failure.
- How to respond when a published package needs a higher-version fix, unlisting, deprecation, or
  documentation update.

Alternative considered: leave setup scattered across package READMEs. Package-specific READMEs are
still useful, but the environment setup and recurring release operations both cross package
boundaries.

## Risks / Trade-offs

- Public package versions are immutable -> CI must verify uniqueness or use skip-duplicate only for
  idempotent retries, never as a substitute for versioning.
- Trusted publishing availability can vary by registry/account -> Keep token fallback documented and
  feature-gate OIDC paths.
- Automatic main publishing can ship accidental changes -> Gate on all verification jobs and use the
  `production` environment so required reviewers can be enabled later without workflow redesign.
- VS Code extension versioning differs from npm/NuGet prerelease semantics -> Add a dedicated VSIX
  version staging/check step instead of reusing `0.1.238-beta` directly.
- Preview registry packages can clutter feeds -> Default PR behavior to artifacts only and make
  preview feed publishing opt-in for trusted contexts.
- Partial registry outages can produce split releases -> Publish from the same artifacts, make each
  registry step idempotent where possible, and document repair commands.

## Migration Plan

1. Fix known metadata defects in package projects/manifests.
2. Add `docs/deployment-setup.md` with the recommended environments, secrets, trusted publishing
   policies, and registry setup.
3. Add `docs/deployment.md` with the ongoing runbook for publishing new releases, checking package
   artifacts, repairing partial releases, and handling rollback/unlist scenarios.
4. Update workflows so build/package jobs publish reusable artifacts for downstream publish jobs.
5. Add preview environment behavior that always uploads artifacts and optionally publishes to
   preview/test feeds from trusted contexts.
6. Add production environment publish jobs for NuGet, npm editor assets, and VSIX artifacts from
   verified `main` builds.
7. Preserve manual dispatch or tag-based repair paths for one-off re-publishing of already-built
   artifacts.
8. Validate by running package verification locally and by dry-running CI paths without secrets
   before enabling production publishing.

Rollback is registry-specific: source changes can be reverted, but already-published package
versions cannot be overwritten. Recovery should publish a fixed higher version and, where the
registry supports it, unlist or deprecate the bad version.

## Open Questions

- Should production publish immediately on every `main` merge during the beta period, or should the
  `production` environment require manual approval from the start?
- Which preview feed should be enabled first: GitHub Packages for NuGet/npm, VSIX artifacts only, or
  a separate public prerelease channel?
- Should stable VS Code releases use even/odd minor version channel conventions, or should NX defer
  formal pre-release channel policy until it has external users?
