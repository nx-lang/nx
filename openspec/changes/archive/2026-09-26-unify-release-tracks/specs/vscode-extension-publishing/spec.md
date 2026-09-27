## ADDED Requirements

### Requirement: VS Code extension is released with the NX release

The automated publishing workflow SHALL publish VS Code extension packages only from trusted release
contexts that produce a registry-valid VSIX version for the artifact being published. The production
path SHALL be the NX release tag, `v<major>.<minor>.<patch>`, which creates one draft GitHub Release
for the packages and the extension together; publishing that GitHub Release SHALL trigger
Marketplace and Open VSX publication from the release assets.

#### Scenario: VS Code release tag computes publishable extension version

- **WHEN** a maintainer pushes a valid release tag matching `v<major>.<minor>.<patch>`
- **THEN** CI SHALL compute the VSIX version as `major.minor.patch` from the tag, the same version as
  the release's packages
- **AND** the staged extension manifest version SHALL match the VSIX artifact being published
- **AND** the checked-in development manifest SHALL NOT need to be manually edited for every release
  solely to create the published extension version

#### Scenario: VS Code release tag creates draft release

- **WHEN** a valid release tag is pushed
- **THEN** CI SHALL build, verify, and upload the VSIX artifact for every supported package target
- **AND** CI SHALL attach the verified VSIX artifacts to the release's draft GitHub Release, beside
  the package artifacts
- **AND** CI SHALL NOT publish those VSIX artifacts to Marketplace or Open VSX while the GitHub Release
  remains a draft

#### Scenario: Published VS Code release uses release assets

- **WHEN** a human publishes a non-draft GitHub Release for a valid release tag
- **THEN** the publish workflow SHALL validate the release tag and attached VSIX artifacts
- **AND** the workflow SHALL publish those artifacts without rebuilding different package contents

#### Scenario: Extension version is invalid

- **WHEN** a trusted release workflow cannot compute a registry-valid VSIX version
- **THEN** the workflow MUST fail before creating a publishable release or publishing to any registry

#### Scenario: Already-published registry version is skipped

- **WHEN** the publish workflow checks a target registry before publication
- **AND** the verified VSIX artifact version is already published in that registry for the selected
  release channel and package target
- **THEN** the workflow SHALL skip that registry write for that VSIX artifact as an idempotent retry
- **AND** it SHALL continue publishing the same VSIX artifact to any target registry where that
  version is not already present

#### Scenario: Manual repair uses release assets

- **WHEN** a maintainer runs a manual repair publish for the VS Code extension
- **THEN** the workflow SHALL require an explicit GitHub Release tag or release asset set
- **AND** the workflow SHALL validate the release assets before publication
- **AND** the workflow SHALL publish artifacts from that release without rebuilding different
  package contents
- **AND** the workflow SHALL allow same-version repair by skipping registries where the artifact
  version is already published and publishing to registries where that version is missing

## MODIFIED Requirements

### Requirement: VS Code extension registry publishing

The repository SHALL support publishing the same verified VSIX artifact for each package target to
both the Visual Studio Marketplace and Open VSX from trusted CI. Registry publication SHALL happen
only in the release publish workflow after the release's GitHub Release is published, extension
tests pass, package verification passes, version checks pass, release assets are validated, and
required environment gates pass.

#### Scenario: Publish to both registries from published release

- **WHEN** the publish workflow targets the `production` environment for a published NX GitHub
  Release
- **AND** extension tests passed before the release assets were attached
- **AND** package verification passes for each VSIX package target
- **AND** the release asset set passes validation
- **AND** per-registry publication checks pass or identify existing versions as idempotent skips
- **AND** the Marketplace publishing identity and `OVSX_PAT` are configured
- **THEN** the workflow SHALL publish the verified VSIX artifacts to the Visual Studio Marketplace
- **AND** the workflow SHALL publish the same verified VSIX artifacts to Open VSX

#### Scenario: Publish commands use the packaged artifact

- **WHEN** the automated workflow publishes the VS Code extension
- **THEN** both registry publish commands SHALL use the VSIX artifact attached to the GitHub Release
- **AND** neither registry publish step SHALL rebuild a different package implicitly

#### Scenario: Pull request builds do not publish public VSIX packages

- **WHEN** the VS Code extension workflow runs for a pull request
- **THEN** the workflow SHALL build, verify, and upload VSIX artifacts for inspection
- **AND** the workflow SHALL NOT publish those artifacts to public extension registries from an
  untrusted pull request context

#### Scenario: Pull request VSIX test commands are posted

- **WHEN** pull request VSIX artifacts are uploaded successfully
- **THEN** CI SHALL provide an automated pull request comment with exact commands to download and
  install the VSIX artifacts
- **AND** the comment workflow SHALL NOT execute untrusted pull request code with write-scoped tokens
- **AND** the commands SHALL use the specific artifact-producing workflow run rather than rebuilding
  locally

#### Scenario: Main packaging workflow does not write to registries

- **WHEN** the `VS Code Extension` workflow runs after a merge to `main`
- **THEN** the workflow SHALL build, verify, and upload VSIX artifacts
- **AND** the workflow SHALL NOT require Marketplace or Open VSX credentials
- **AND** any production Marketplace or Open VSX writes SHALL happen only after an NX GitHub Release
  is published

### Requirement: VS Code extension publishing credentials

The publishing workflow SHALL keep registry credentials outside source control and fail safely when
credentials are missing for a publish job. Registry credentials SHALL be scoped through GitHub
environments when publication targets `production`. The workflow SHALL authenticate to the Visual
Studio Marketplace as a Microsoft Entra ID managed identity that is a member of the `nx-lang`
publisher, signed in through GitHub's OIDC token, rather than with a personal access token, because
Azure DevOps retires global personal access tokens on 2026-12-01. It SHALL authenticate to Open VSX
with the `OVSX_PAT` token.

#### Scenario: Required production CI credentials are missing

- **WHEN** the publish workflow targets the `production` environment for a release
- **AND** any of `AZURE_CLIENT_ID`, `AZURE_TENANT_ID` or `OVSX_PAT` is not configured
- **THEN** the extension publish job MUST fail before publishing to the Visual Studio Marketplace or
  Open VSX

#### Scenario: Pull request credentials are unavailable

- **WHEN** the VS Code extension workflow runs from an untrusted pull request context
- **THEN** registry credentials SHALL NOT be exposed to the job
- **AND** the workflow SHALL limit itself to verification and artifact upload behavior
- **AND** the managed identity SHALL trust GitHub's OIDC token only for jobs in the `production`
  environment

#### Scenario: Local credentials are supplied through environment variables

- **WHEN** a maintainer follows the documented local publishing commands
- **THEN** the commands SHALL sign in to the Marketplace through the maintainer's Azure CLI session
  and read the Open VSX token from an environment variable
- **AND** the documentation MUST NOT instruct maintainers to commit tokens or write them into
  tracked configuration files

### Requirement: VS Code extension release documentation

The VS Code extension documentation SHALL describe the supported release process for maintainers and
SHALL link to the cross-package deployment setup and runbook documentation for CI environment,
registry configuration, artifact testing, and recurring release operations.

#### Scenario: Maintainer prepares a release

- **WHEN** a maintainer reads the VS Code extension publishing documentation
- **THEN** the documentation SHALL describe how to run tests, package the extension, inspect VSIX
  contents, and publish locally for repair scenarios
- **AND** it SHALL describe how CI packages verified VSIX artifacts from pull requests, `main`, and
  release tags
- **AND** it SHALL describe how a `v*` release tag creates one draft GitHub Release for the packages
  and the extension
- **AND** it SHALL describe how publishing that GitHub Release publishes the extension along with the
  packages
- **AND** it SHALL link to `docs/deployment-setup.md` for GitHub environment, registry credential,
  and trusted-publishing setup
- **AND** it SHALL link to `docs/deployment.md` for the ongoing publish, verification, artifact
  testing, and repair runbook

#### Scenario: Maintainer chooses a release channel

- **WHEN** a maintainer reads the VS Code extension publishing documentation
- **THEN** the documentation SHALL explain that the extension is released with the packages, at the
  package version
- **AND** it SHALL identify the `v*` tag format and versioning rules needed for regular registry
  publication

#### Scenario: Maintainer tests a PR VSIX

- **WHEN** a maintainer reads the VS Code extension publishing documentation
- **THEN** the documentation SHALL describe how to install a pull request VSIX artifact by using the PR
  comment commands or `code --install-extension`
- **AND** it SHALL explain that installing from VSIX is the supported PR testing path rather than
  publishing PR builds to Marketplace or Open VSX

### Requirement: The extension is published for NX users
The VS Code extension SHALL be published to the Visual Studio Marketplace under publisher `nx-lang`
and to Open VSX under namespace `nx-lang`, as extension `nx-language`, with each NX release
from its `v*` tag. Its listing SHALL be written for someone installing it: the extension's
README, which both registries show as its page, SHALL describe what the extension does and how to
configure it, and SHALL link to `https://nxlang.org`. Instructions for building, packaging and
releasing the extension SHALL live in a separate maintainer document, which the README links to.

#### Scenario: Installed from the Marketplace
- **WHEN** a user runs `code --install-extension nx-lang.nx-language`, or installs NX Language from
  the Extensions view in VS Code
- **THEN** the latest published release SHALL install, and a `.nx` file SHALL open with NX
  highlighting and language-server diagnostics

#### Scenario: Installed from Open VSX
- **WHEN** a user of an editor that reads Open VSX, such as VSCodium, searches for NX Language
- **THEN** the same release SHALL be offered, under the `nx-lang` namespace

#### Scenario: The listing is for users
- **WHEN** a user opens the extension's page on either registry
- **THEN** it SHALL show the extension's icon and a description of its features and settings
- **AND** it SHALL link to `https://nxlang.org`
- **AND** it SHALL NOT open with instructions for building the extension or installing a Node
  toolchain

#### Scenario: Maintainer instructions have a home
- **WHEN** a maintainer looks for how to build, package or release the extension
- **THEN** `src/vscode/CONTRIBUTING.md` SHALL describe it, and the README SHALL link to that file

## REMOVED Requirements

### Requirement: VS Code extension versioned release trigger
**Reason**: The extension no longer has its own `vscode-v*` release tag. It is released from the NX
release tag at the package version; see "VS Code extension is released with the NX release".
**Migration**: Push a `v<major>.<minor>.<patch>` tag to release the extension along with the
packages.
