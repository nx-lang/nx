## ADDED Requirements

### Requirement: Release versioning uses MinVer
NX SHALL use MinVer CLI as the single CI version source for package release artifacts and preview
artifacts in this release pipeline. The pipeline SHALL remove Nerdbank.GitVersioning/`nbgv` from
package, editor-assets, and CI version calculation paths unless implementation discovers a blocking
MinVer limitation and records it for maintainer review before replacing the behavior.

#### Scenario: Package release tag determines stable package version
- **WHEN** a maintainer pushes a valid compiler package release tag matching `v<major>.<minor>.<patch>`
- **THEN** CI SHALL calculate the package release version from that tag using MinVer CLI
- **AND** the produced NuGet and npm editor-assets artifacts SHALL use the tag version without a
  prerelease suffix

#### Scenario: Pull request package version is unique and prerelease
- **WHEN** a pull request build produces NuGet or npm package artifacts
- **THEN** CI SHALL calculate or override the artifact version so it is unique to the pull request build
- **AND** the version SHALL include a prerelease identifier that prevents it from being mistaken for a
  production release version

#### Scenario: Main package version is CI preview only
- **WHEN** the Build workflow runs on `main`
- **THEN** CI SHALL produce installable artifacts with MinVer-derived or explicitly staged CI preview
  versions
- **AND** those versions SHALL NOT be published to production package registries solely because the
  `main` build succeeded

#### Scenario: NBGV tooling is removed from release versioning
- **WHEN** the release and package workflows calculate versions after this change
- **THEN** they SHALL NOT call `dotnet nbgv` or depend on `version.json`
- **AND** package-generation scripts SHALL read MinVer-derived environment variables or direct MinVer CLI
  output instead

## MODIFIED Requirements

### Requirement: Package publishing uses explicit deployment environments
NX SHALL model package registry publication through explicit GitHub Actions deployment environments so
production publishing can use scoped credentials, protection rules, and audit history. Pull request and
`main` builds SHALL build, verify, upload, and document testable artifacts without publishing to package
registries.

#### Scenario: Pull request package artifact builds
- **WHEN** a pull request build runs for package-related changes
- **THEN** CI SHALL build and verify publishable NuGet package artifacts, including `.nupkg` and
  `.snupkg` files when symbols are produced
- **AND** CI SHALL build and verify the npm editor-assets `.tgz` artifact
- **AND** CI SHALL upload the tested package artifacts for maintainer and reviewer inspection
- **AND** CI SHALL NOT publish to public, preview, or test package registries from the pull request
  build

#### Scenario: Pull request package test commands are posted
- **WHEN** pull request package artifacts are uploaded successfully
- **THEN** CI SHALL provide an automated pull request comment with exact commands to download and test
  the NuGet and npm artifacts
- **AND** the comment workflow SHALL NOT execute untrusted pull request code with write-scoped tokens
- **AND** the commands SHALL use the specific artifact-producing workflow run rather than rebuilding
  locally

#### Scenario: Main package builds are artifact-only
- **WHEN** a trusted `main` Build workflow run completes successfully
- **THEN** CI SHALL upload verified package artifacts for inspection and repair use
- **AND** CI SHALL NOT publish production packages to NuGet.org or npm solely because the `main` build
  succeeded
- **AND** CI SHALL NOT publish package artifacts to preview or test package registries

#### Scenario: Package release tag creates draft release
- **WHEN** a maintainer pushes a valid compiler package release tag matching `v<major>.<minor>.<patch>`
- **THEN** CI SHALL build, verify, and smoke-test the compiler/runtime and editor-assets package
  artifacts from that tag
- **AND** CI SHALL create or update a draft GitHub Release for the tag
- **AND** CI SHALL attach the verified NuGet and npm package artifacts to the draft GitHub Release
- **AND** CI SHALL NOT publish those artifacts to external package registries while the GitHub Release
  remains a draft

#### Scenario: VS Code tag does not create package release
- **WHEN** a maintainer pushes a tag matching `vscode-v<major>.<minor>.<patch>`
- **THEN** the package release track SHALL ignore the tag for compiler/runtime and editor-assets
  publication

#### Scenario: Published package release triggers production publishing
- **WHEN** a human publishes a non-draft GitHub Release for a valid compiler package release tag
- **THEN** the package publish workflow SHALL target the `production` environment
- **AND** CI SHALL publish the attached NuGet and npm editor-assets artifacts to the configured
  production registries
- **AND** CI SHALL use only credentials or trusted-publishing policies scoped to the `production`
  environment

### Requirement: Production publishing uses verified immutable package artifacts
NX SHALL publish only package artifacts that were built, inspected, verified, and attached to the
corresponding GitHub Release before the publish step. Production publishing MUST fail before registry
writes when release asset validation fails or when the package version is not publishable.

#### Scenario: Publish jobs consume GitHub Release assets
- **WHEN** a production package publish job runs after a GitHub Release is published
- **THEN** it SHALL download the NuGet and npm package artifacts attached to that GitHub Release
- **AND** it SHALL validate that the release tag, artifact versions, and expected artifact set match the
  selected package release track
- **AND** it SHALL publish those artifacts without rebuilding package contents in the publish job

#### Scenario: Duplicate or invalid package version blocks publication
- **WHEN** a package artifact has a version that is invalid for its registry or already published to
  the target production registry
- **THEN** CI SHALL fail or skip the duplicate as an idempotent retry before attempting a new public
  version write
- **AND** CI SHALL NOT overwrite an existing public package version

#### Scenario: Partial registry failure is repairable
- **WHEN** one production registry publish succeeds and another registry publish fails
- **THEN** CI SHALL preserve the exact GitHub Release assets used for the successful publish
- **AND** the deployment documentation SHALL describe how to repair the failed registry by publishing
  the same release asset rather than rebuilding

### Requirement: Deployment setup and runbook are documented
NX SHALL include separate deployment documentation for one-time CI/registry setup and ongoing package
publishing operations. Documentation SHALL describe tag-driven draft releases, artifact-only CI/PR
builds, release-publication registry writes, and artifact-based pull request testing.

#### Scenario: Maintainer configures deployment environments
- **WHEN** a maintainer reads `docs/deployment-setup.md`
- **THEN** the document SHALL list the GitHub environments to create
- **AND** it SHALL describe recommended protection rules for `production`
- **AND** it SHALL identify which secrets, variables, and trusted-publishing policies belong to the
  production publishing workflows

#### Scenario: Maintainer configures package registries
- **WHEN** a maintainer reads `docs/deployment-setup.md`
- **THEN** the document SHALL describe setup for NuGet.org, npm, the Visual Studio Marketplace, and
  Open VSX
- **AND** it SHALL explain which registries can use trusted publishing and which require token secrets
- **AND** it SHALL NOT require preview package registry setup for pull request testing

#### Scenario: Maintainer understands tag-driven publishing
- **WHEN** a maintainer reads `docs/deployment.md`
- **THEN** the document SHALL explain that successful `main` builds produce CI artifacts only
- **AND** it SHALL explain that pushing release tags creates draft GitHub Releases with attached
  verified artifacts
- **AND** it SHALL explain that publishing the GitHub Release triggers production registry publication

#### Scenario: Maintainer follows the release runbook
- **WHEN** a maintainer reads `docs/deployment.md`
- **THEN** the document SHALL describe how to publish a new compiler package release from a `v*` tag
- **AND** it SHALL describe how to publish a new VS Code extension release from a `vscode-v*` tag
- **AND** it SHALL list the verification, draft release inspection, release publication, environment
  approval, and registry confirmation steps for NuGet, npm editor assets, Visual Studio Marketplace,
  and Open VSX

#### Scenario: Maintainer tests pull request artifacts
- **WHEN** a maintainer reads `docs/deployment.md`
- **THEN** the document SHALL describe how to use PR comments and workflow artifacts to download and
  test `.nupkg`, `.snupkg`, npm `.tgz`, and `.vsix` artifacts without preview registry credentials

#### Scenario: Maintainer repairs or rolls forward a release
- **WHEN** a maintainer reads `docs/deployment.md`
- **THEN** the document SHALL describe how to repair a partial registry publish using the same GitHub
  Release assets
- **AND** it SHALL describe the rollback posture for immutable registries, including publishing a
  higher-version fix and unlisting or deprecating bad versions where supported

### Requirement: Release publishing is split into explicit package and extension actions
NX SHALL expose separate release tracks for package-registry publication and VS Code
extension-registry publication. Build and packaging workflows SHALL produce verified artifacts, tag
release workflows SHALL create draft GitHub Releases, and publish workflows SHALL write release assets
to external registries only after the corresponding GitHub Release is published.

#### Scenario: Package and extension release tracks are separate
- **WHEN** a maintainer inspects the release pipeline workflows
- **THEN** the pipeline SHALL provide a package release track for NuGet and editor-assets package
  registries
- **AND** it SHALL provide a separate VS Code extension release track for Visual Studio Marketplace and
  Open VSX publication
- **AND** each track SHALL use a distinct tag pattern and validate the tag before creating or publishing
  a release

#### Scenario: Build workflows do not require production registry credentials
- **WHEN** `Build` or `VS Code Extension` workflow runs verify package artifacts on pull requests or
  `main`
- **THEN** those workflows SHALL complete artifact verification without requiring production registry
  credentials
- **AND** production registry credentials SHALL be used only by explicit publish workflows that target
  the `production` environment after a GitHub Release is published

#### Scenario: Draft releases contain reviewed publish inputs
- **WHEN** a tag-driven release workflow creates a draft GitHub Release
- **THEN** it SHALL attach the verified artifacts that will be published if the release is later
  published
- **AND** it SHALL provide enough release metadata for a maintainer to inspect the source tag, versions,
  and attached artifacts before public registry publication

#### Scenario: Rust tool publishing is out of scope for this release pipeline
- **WHEN** a maintainer reads the package deployment runbook for this release pipeline
- **THEN** the runbook SHALL describe NuGet/editor-assets package publishing and VS Code extension
  publishing
- **AND** it SHALL NOT describe `nxlang`, `nx-lsp`, or Rust crate publication as part of this release
  pipeline
