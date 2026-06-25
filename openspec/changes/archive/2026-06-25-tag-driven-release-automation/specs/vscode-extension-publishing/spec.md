## MODIFIED Requirements

### Requirement: VS Code extension versioned release trigger
The automated publishing workflow SHALL publish VS Code extension packages only from trusted release
contexts that produce a registry-valid VSIX version for the artifact being published. The primary
production path SHALL be a `vscode-v<major>.<minor>.<patch>` tag that creates a draft GitHub Release;
publishing that GitHub Release SHALL trigger Marketplace and Open VSX publication from the release
assets.

#### Scenario: VS Code release tag computes publishable extension version
- **WHEN** a maintainer pushes a valid VS Code extension release tag matching
  `vscode-v<major>.<minor>.<patch>`
- **THEN** CI SHALL compute the VSIX version by deriving `major.minor.patch` from the tag
- **AND** the staged extension manifest version SHALL match the VSIX artifact being attached to the
  draft GitHub Release
- **AND** the checked-in development manifest SHALL NOT need to be manually edited for every release
  solely to create the published extension version

#### Scenario: VS Code release tag creates draft release
- **WHEN** a valid VS Code extension release tag is pushed
- **THEN** CI SHALL build, verify, and upload the VSIX artifact for every supported package target
- **AND** CI SHALL create or update a draft GitHub Release for the tag
- **AND** CI SHALL attach the verified VSIX artifacts to the draft GitHub Release
- **AND** CI SHALL NOT publish those VSIX artifacts to Marketplace or Open VSX while the GitHub Release
  remains a draft

#### Scenario: Package release tag does not create VS Code release
- **WHEN** a maintainer pushes a compiler package release tag matching `v<major>.<minor>.<patch>`
- **THEN** the VS Code extension release track SHALL ignore the tag for Marketplace and Open VSX
  publication

#### Scenario: Published VS Code release uses release assets
- **WHEN** a human publishes a non-draft GitHub Release for a valid VS Code extension release tag
- **THEN** the `Publish VS Code extension` workflow SHALL validate the release tag and attached VSIX
  artifacts
- **AND** the workflow SHALL publish those artifacts without rebuilding different package contents

#### Scenario: Extension version is invalid
- **WHEN** a trusted release workflow cannot compute a registry-valid VSIX version
- **THEN** the workflow MUST fail before creating a publishable release or publishing to any registry

#### Scenario: Already-published registry version is skipped
- **WHEN** the `Publish VS Code extension` workflow checks a target registry before publication
- **AND** the verified VSIX artifact version is already published in that registry for the selected
  release channel
- **THEN** the workflow SHALL skip that registry write for that VSIX artifact as an idempotent retry
- **AND** it SHALL continue publishing the same VSIX artifact to any target registry where that version
  is not already present

#### Scenario: Manual repair uses release assets
- **WHEN** a maintainer runs a manual repair publish for the VS Code extension
- **THEN** the workflow SHALL require an explicit GitHub Release tag or release asset set
- **AND** the workflow SHALL validate the release assets before publication
- **AND** the workflow SHALL publish artifacts from that release without rebuilding different package
  contents
- **AND** the workflow SHALL allow same-version repair by skipping registries where the artifact
  version is already published and publishing to registries where that version is missing

### Requirement: VS Code extension registry publishing
The repository SHALL support publishing the same verified VSIX artifact for each package target to
both the Visual Studio Marketplace and Open VSX from trusted CI. Registry publication SHALL happen
only in the dedicated `Publish VS Code extension` workflow after a VS Code GitHub Release is
published, extension tests pass, package verification passes, version checks pass, release assets are
validated, and required environment gates pass.

#### Scenario: Publish to both registries from published release
- **WHEN** the `Publish VS Code extension` workflow targets the `production` environment for a
  published VS Code extension GitHub Release
- **AND** extension tests passed before the release assets were attached
- **AND** package verification passed for each VSIX package target
- **AND** the release asset set passes validation
- **AND** per-registry publication checks pass or identify existing versions as idempotent skips
- **AND** `VSCE_PAT` and `OVSX_PAT` are configured or an equivalent supported registry
  authentication mechanism is available
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
- **AND** any production Marketplace or Open VSX writes SHALL happen only after a VS Code GitHub
  Release is published

### Requirement: VS Code extension release documentation
The VS Code extension documentation SHALL describe the supported release process for maintainers and
SHALL link to the cross-package deployment setup and runbook documentation for CI environment,
registry configuration, artifact testing, and recurring release operations.

#### Scenario: Maintainer prepares a release
- **WHEN** a maintainer reads the VS Code extension publishing documentation
- **THEN** the documentation SHALL describe how to run tests, package the extension, inspect VSIX
  contents, and publish locally for repair scenarios
- **AND** it SHALL describe how CI packages verified VSIX artifacts from pull requests, `main`, and
  VS Code release tags
- **AND** it SHALL describe how a `vscode-v*` tag creates a draft GitHub Release
- **AND** it SHALL describe how publishing that GitHub Release triggers the separate
  `Publish VS Code extension` workflow
- **AND** it SHALL link to `docs/deployment-setup.md` for GitHub environment, registry credential,
  and trusted-publishing setup
- **AND** it SHALL link to `docs/deployment.md` for the ongoing publish, verification, artifact
  testing, and repair runbook

#### Scenario: Maintainer chooses a release channel
- **WHEN** a maintainer reads the VS Code extension publishing documentation
- **THEN** the documentation SHALL explain the configured VS Code extension release channel behavior
- **AND** it SHALL identify the `vscode-v*` tag format and versioning rules needed for regular
  registry publication

#### Scenario: Maintainer tests a PR VSIX
- **WHEN** a maintainer reads the VS Code extension publishing documentation
- **THEN** the documentation SHALL describe how to install a pull request VSIX artifact by using the PR
  comment commands or `code --install-extension`
- **AND** it SHALL explain that installing from VSIX is the supported PR testing path rather than
  publishing PR builds to Marketplace or Open VSX
