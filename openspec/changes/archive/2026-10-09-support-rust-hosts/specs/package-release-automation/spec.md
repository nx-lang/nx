## MODIFIED Requirements

### Requirement: Packages and the VS Code extension are released together
NX SHALL release its packages and its VS Code extension as one release: one
`v<major>.<minor>.<patch>` tag, one version, one draft GitHub Release and one publication. Build and
packaging workflows SHALL produce verified artifacts, the tag release workflow SHALL create the draft
GitHub Release, and the publish workflow SHALL write release assets to external registries only after
that GitHub Release is published.

#### Scenario: One tag releases every artifact
- **WHEN** a maintainer inspects the release pipeline workflows
- **THEN** the pipeline SHALL provide one release track, triggered by `v<major>.<minor>.<patch>` tags,
  that covers NuGet, npm, crates.io, the Visual Studio Marketplace and Open VSX
- **AND** it SHALL validate the tag before creating or publishing a release

#### Scenario: Packages and extension share a version
- **WHEN** a release is built from a `v<major>.<minor>.<patch>` tag
- **THEN** the NuGet package, every npm package, every published crate and every VSIX file SHALL
  carry the version `<major>.<minor>.<patch>`

#### Scenario: One release is the latest
- **WHEN** a maintainer publishes a release
- **THEN** that GitHub Release SHALL carry every artifact of the version, so the repository's latest
  release covers the packages and the extension alike

#### Scenario: Build workflows do not require production registry credentials
- **WHEN** `Build` or `VS Code Extension` workflow runs verify package artifacts on pull requests or
  `main`
- **THEN** those workflows SHALL complete artifact verification without requiring production registry
  credentials
- **AND** production registry credentials SHALL be used only by the publish workflow, targeting the
  `production` environment after a GitHub Release is published

#### Scenario: Draft releases contain reviewed publish inputs
- **WHEN** the tag release workflow creates a draft GitHub Release
- **THEN** it SHALL attach the verified artifacts that will be published if the release is later
  published
- **AND** it SHALL provide enough release metadata for a maintainer to inspect the source tag, version
  and attached artifacts before public registry publication

#### Scenario: Rust tool publishing is out of scope for this release pipeline
- **WHEN** a maintainer reads the deployment runbook for this release pipeline
- **THEN** the runbook SHALL describe NuGet, npm, runtime crate and VS Code extension publishing
- **AND** it SHALL NOT describe publication of the Rust tools `nxlang` and `nx-lsp`, or of any crate
  other than the runtime crates, as part of this release pipeline, beyond `nx-lsp` shipping inside
  the VSIX files

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
- **AND** CI SHALL package and verify the runtime crates' `.crate` artifacts
- **AND** CI SHALL upload the tested package artifacts for maintainer and reviewer inspection
- **AND** CI SHALL NOT publish to public, preview, or test package registries from the pull request
  build

#### Scenario: Pull request package test commands are posted
- **WHEN** pull request package artifacts are uploaded successfully
- **THEN** CI SHALL provide an automated pull request comment with exact commands to download and test
  the NuGet, npm and runtime crate artifacts
- **AND** the comment workflow SHALL NOT execute untrusted pull request code with write-scoped tokens
- **AND** the commands SHALL use the specific artifact-producing workflow run rather than rebuilding
  locally

#### Scenario: Main package builds are artifact-only
- **WHEN** a trusted `main` Build workflow run completes successfully
- **THEN** CI SHALL upload verified package artifacts for inspection and repair use
- **AND** CI SHALL NOT publish production packages to NuGet.org, npm or crates.io solely because the
  `main` build succeeded
- **AND** CI SHALL NOT publish package artifacts to preview or test package registries

#### Scenario: Package release tag creates draft release
- **WHEN** a maintainer pushes a valid release tag matching `v<major>.<minor>.<patch>`
- **THEN** CI SHALL build, verify, and smoke-test the compiler/runtime and editor-assets package
  artifacts, and build and verify a VSIX file for every supported extension target, from that tag
- **AND** CI SHALL create or update one draft GitHub Release for the tag, titled `NX <version>`
- **AND** CI SHALL attach the verified NuGet, npm, runtime crate and VSIX artifacts to that draft
  GitHub Release
- **AND** CI SHALL NOT publish those artifacts to external registries while the GitHub Release
  remains a draft

#### Scenario: VS Code tag does not create package release
- **WHEN** a maintainer pushes a tag matching `vscode-v<major>.<minor>.<patch>`, the retired
  extension-only tag format
- **THEN** no release workflow SHALL build, create a release for, or publish anything from that tag

#### Scenario: Published package release triggers production publishing
- **WHEN** a human publishes a non-draft GitHub Release for a valid release tag
- **THEN** the publish workflow SHALL target the `production` environment
- **AND** CI SHALL publish the attached NuGet and npm artifacts to NuGet.org and npm, the runtime
  crates to crates.io, and the attached VSIX files to the Visual Studio Marketplace and Open VSX
- **AND** CI SHALL use only credentials or trusted-publishing policies scoped to the `production`
  environment

### Requirement: Production publishing uses verified immutable package artifacts
NX SHALL publish only package artifacts that were built, inspected, verified, and attached to the
corresponding GitHub Release before the publish step. Production publishing MUST fail before registry
writes when release asset validation fails or when the package version is not publishable.

A registry that takes a file SHALL be sent the attached file. crates.io takes none: its client
packages a crate again from the sources. The publish job SHALL therefore package the runtime crates
from the release's tag, SHALL publish a crate only when every crate it packaged is byte for byte
the file attached to the release, and SHALL fail without a registry write when one is not. That is
the one place the track builds anything at publish time, and what it builds is held to the reviewed
artifact before the upload and, by the registry's recorded checksum, after it.

#### Scenario: Publish jobs consume GitHub Release assets
- **WHEN** a production publish job runs after a GitHub Release is published
- **THEN** it SHALL download the NuGet, npm, runtime crate and VSIX artifacts attached to that GitHub
  Release
- **AND** it SHALL validate that the release tag, artifact versions, and expected artifact set match
  the release
- **AND** it SHALL publish the NuGet, npm and VSIX artifacts without rebuilding package contents in
  the publish job
- **AND** it SHALL publish the runtime crates only after packaging them from the release's tag and
  finding each identical, byte for byte, to the attached `.crate` file

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
  the same release asset rather than rebuilding, and for crates.io by packaging from the same tag
  again and publishing only what matches the same release assets

## ADDED Requirements

### Requirement: Rust runtime crates ship on the package release track
The package release track SHALL publish to crates.io the crates a Rust host needs to run compiled
NX, which are the image format crate `nx-ir`, the host value crate `nx-value` and the runtime crate
`nx-ir-runtime`, and no other crate. Pull request and `main` builds SHALL package each of the three
and verify that the packaged crate builds, without publishing. A release tag SHALL package them at
the tag's version, each depending on the others at exactly that version, and SHALL attach the
packaged `.crate` files to the draft GitHub Release with their checksums in the release manifest.
Publishing the release SHALL publish the three to crates.io from the `production` environment in
dependency order, SHALL verify that each crate the registry then serves is byte for byte the
attached file, and SHALL skip a version the registry already has as an idempotent retry. Every
other crate of the workspace SHALL be marked so that it cannot be published.

#### Scenario: Pull request builds package the runtime crates
- **WHEN** a pull request build runs
- **THEN** CI SHALL package `nx-ir`, `nx-value` and `nx-ir-runtime`, SHALL build each from its
  packaged contents alone, and SHALL upload the `.crate` files for inspection without publishing

#### Scenario: A packaged crate that depends on the checkout fails the build
- **WHEN** a change makes one of the three crates read a file outside its own directory at build
  time, or depend on a crate that is not published
- **THEN** the pull request build SHALL fail in the packaging step

#### Scenario: A release tag attaches the runtime crates
- **WHEN** a maintainer pushes a valid release tag
- **THEN** CI SHALL attach one `.crate` file per runtime crate to the draft GitHub Release, each
  carrying the tag's version
- **AND** the packaged manifest of `nx-ir-runtime` SHALL depend on `nx-ir` and `nx-value` at exactly
  that version

#### Scenario: Publishing a release publishes the runtime crates
- **WHEN** a human publishes the GitHub Release
- **THEN** the publish workflow SHALL publish `nx-ir` and `nx-value` before `nx-ir-runtime`
- **AND** a failure of the crates.io publication SHALL NOT stop NuGet, npm or extension publication,
  nor they it

#### Scenario: The published crate is the reviewed file
- **WHEN** the publish workflow has published a crate
- **THEN** it SHALL compare the checksum crates.io records for that version with the checksum of the
  attached `.crate` file and SHALL fail when they differ

#### Scenario: A host builds the runtime from crates.io alone
- **WHEN** a Rust project outside this repository depends on `nx-ir-runtime` at a published version
- **THEN** it SHALL build without a checkout of this repository
- **AND** its build SHALL NOT compile a crate of the NX compiler

#### Scenario: Another workspace crate cannot be published by mistake
- **WHEN** a maintainer runs the publish command for a workspace crate other than the three
- **THEN** the command SHALL refuse because the crate is marked unpublishable

#### Scenario: crates.io publishing is documented
- **WHEN** a maintainer reads `docs/deployment-setup.md` and `docs/deployment.md`
- **THEN** the setup document SHALL describe the crates.io trusted-publisher policy of each crate,
  and the first publication of a new crate name that has to precede it
- **AND** the runbook SHALL list crates.io among the registries to confirm after a release and SHALL
  describe repairing a failed crates.io publication from the same release
