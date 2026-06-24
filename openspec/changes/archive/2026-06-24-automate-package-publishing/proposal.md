## Why

NX can already build the publishable artifacts, but the release path still depends on a mix of
manual steps, tag-only VS Code publishing, and incomplete deployment setup documentation. This
change makes package publishing repeatable from CI while keeping the workflow safe for immutable
public package registries.

## What Changes

- Add a CI release model that separates PR preview validation, trusted preview/test package
  publishing, and production package publishing.
- Allow `NxLang.Runtime` NuGet packages to be published by CI from the complete multi-OS package
  artifact after package verification and smoke tests pass.
- Allow the VS Code extension to be published by CI without requiring a maintainer to hand-run the
  local publish scripts, while preserving verified VSIX artifact publishing.
- Support production publishing from trusted `main` builds when package versions are unique and all
  required registry credentials or trusted-publishing policies are configured.
- Prefer keyless/OIDC trusted publishing where the registries support it, with token-based secrets
  documented as fallback paths.
- Fix release metadata that can be corrected in-repo, including stale repository/project URLs in
  NuGet metadata.
- Add `docs/deployment-setup.md` documenting one-time GitHub environment, registry, and
  secret/policy setup.
- Add `docs/deployment.md` documenting the ongoing package publishing runbook: how to publish new
  releases, inspect artifacts, repair partial publishes, and respond to bad releases.

## Capabilities

### New Capabilities

- `package-release-automation`: Cross-package CI release orchestration, environment policy, registry
  authentication setup, and deployment documentation for NX package outputs.

### Modified Capabilities

- `vscode-extension-publishing`: Replace tag-only publishing as the primary CI contract with a
  main/environments release workflow that can publish verified VSIX artifacts from trusted builds.
- `editor-assets`: Clarify how the reusable editor-assets npm tarball participates in preview/test
  and production publishing.
- `dotnet-binding`: Require correct public package metadata and CI publication of the complete
  verified `NxLang.Runtime` package artifact.

## Impact

- GitHub Actions workflows under `.github/workflows/`.
- Package metadata in `bindings/dotnet/src/NxLang.Runtime/NxLang.Runtime.csproj`.
- VS Code extension publishing scripts and workflow behavior under `src/vscode/` as needed.
- Release documentation in `docs/deployment-setup.md`, `docs/deployment.md`, and any affected
  package README release sections.
- GitHub repository setup: environments, environment secrets, trusted publishing policies, and
  package registry ownership.
