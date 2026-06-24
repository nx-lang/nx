## MODIFIED Requirements

### Requirement: VS Code extension versioned release trigger

The automated publishing workflow SHALL publish VS Code extension packages only from trusted CI
contexts that produce a registry-valid VSIX version for the artifact being published. The primary
production trigger SHALL be a trusted `main` build targeting the `production` environment; manual or
tag-based publishing MAY remain available as a repair path for an already-built artifact.

#### Scenario: Main build computes a publishable extension version

- **WHEN** a trusted `main` build prepares the VS Code extension for publication
- **THEN** the workflow SHALL compute or stage a `major.minor.patch` extension version for the VSIX
  artifact
- **AND** the staged extension manifest version SHALL match the VSIX artifact being published
- **AND** the checked-in development manifest SHALL NOT need to be manually edited for every main
  merge solely to create a unique CI package version

#### Scenario: Extension version is not publishable

- **WHEN** a trusted publish workflow cannot compute a registry-valid VSIX version
- **OR** the computed VSIX version is already published to the target registry for the selected
  release channel
- **THEN** the workflow MUST fail before publishing to any VS Code extension registry

#### Scenario: Manual repair uses an explicit artifact

- **WHEN** a maintainer runs a manual repair publish for the VS Code extension
- **THEN** the workflow SHALL require an explicit VSIX artifact or artifact-producing workflow run
- **AND** the workflow SHALL publish that artifact without rebuilding different package contents

### Requirement: VS Code extension registry publishing

The repository SHALL support publishing the same verified VSIX artifact for each package target to
both the Visual Studio Marketplace and Open VSX from trusted CI. Registry publication SHALL happen
only after extension tests, package verification, version checks, and required environment gates
pass.

#### Scenario: Publish to both registries from production CI

- **WHEN** a trusted `main` build targets the `production` environment
- **AND** extension tests pass
- **AND** package verification passes for each VSIX package target
- **AND** version checks pass
- **AND** `VSCE_PAT` and `OVSX_PAT` are configured or an equivalent supported registry
  authentication mechanism is available
- **THEN** the workflow SHALL publish the verified VSIX artifacts to the Visual Studio Marketplace
- **AND** the workflow SHALL publish the same verified VSIX artifacts to Open VSX

#### Scenario: Publish commands use the packaged artifact

- **WHEN** the automated workflow publishes the VS Code extension
- **THEN** both registry publish commands SHALL use the VSIX artifact produced by the package step
- **AND** neither registry publish step SHALL rebuild a different package implicitly

#### Scenario: Pull request builds do not publish public VSIX packages

- **WHEN** the VS Code extension workflow runs for a pull request
- **THEN** the workflow SHALL build, verify, and upload VSIX artifacts for inspection
- **AND** the workflow SHALL NOT publish those artifacts to public extension registries from an
  untrusted pull request context

### Requirement: VS Code extension publishing credentials

The publishing workflow SHALL keep registry credentials outside source control and fail safely when
credentials are missing for a publish job. Registry credentials SHALL be scoped through GitHub
environments when publication targets `preview` or `production`.

#### Scenario: Required production CI credentials are missing

- **WHEN** a trusted `main` build targets the `production` environment for VS Code extension
  publication
- **AND** either `VSCE_PAT` or `OVSX_PAT` is required but not configured
- **THEN** the workflow MUST fail before publishing to either registry

#### Scenario: Pull request credentials are unavailable

- **WHEN** the VS Code extension workflow runs from an untrusted pull request context
- **THEN** registry credentials SHALL NOT be exposed to the job
- **AND** the workflow SHALL limit itself to verification and artifact upload behavior

#### Scenario: Local credentials are supplied through environment variables

- **WHEN** a maintainer follows the documented local publishing commands
- **THEN** the commands SHALL read Marketplace and Open VSX credentials from environment variables
- **AND** the documentation MUST NOT instruct maintainers to commit tokens or write them into
  tracked configuration files

### Requirement: VS Code extension release documentation

The VS Code extension documentation SHALL describe the supported release process for maintainers and
SHALL link to the cross-package deployment setup and runbook documentation for CI environment,
registry configuration, and recurring release operations.

#### Scenario: Maintainer prepares a release

- **WHEN** a maintainer reads the VS Code extension publishing documentation
- **THEN** the documentation SHALL describe how to run tests, package the extension, inspect VSIX
  contents, and publish locally for repair scenarios
- **AND** it SHALL describe how CI publishes verified VSIX artifacts from trusted `main` builds
- **AND** it SHALL link to `docs/deployment-setup.md` for GitHub environment, registry credential,
  and trusted-publishing setup
- **AND** it SHALL link to `docs/deployment.md` for the ongoing publish, verification, and repair
  runbook

#### Scenario: Maintainer chooses a release channel

- **WHEN** a maintainer reads the VS Code extension publishing documentation
- **THEN** the documentation SHALL explain the configured VS Code extension release channel behavior
- **AND** it SHALL identify any versioning rules needed for regular or pre-release registry
  publication
