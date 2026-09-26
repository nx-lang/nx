## ADDED Requirements

### Requirement: The extension is published for NX users
The VS Code extension SHALL be published to the Visual Studio Marketplace under publisher `nx-lang`
and to Open VSX under namespace `nx-lang`, as extension `nx-language`, through the existing
`vscode-v*` release track. Its listing SHALL be written for someone installing it: the extension's
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

## MODIFIED Requirements

### Requirement: VS Code extension publishing credentials

The publishing workflow SHALL keep registry credentials outside source control and fail safely when
credentials are missing for a publish job. Registry credentials SHALL be scoped through GitHub
environments when publication targets `production`. The workflow SHALL authenticate to the Visual
Studio Marketplace as a Microsoft Entra ID managed identity that is a member of the `nx-lang`
publisher, signed in through GitHub's OIDC token, rather than with a personal access token, because
Azure DevOps retires global personal access tokens on 2026-12-01. It SHALL authenticate to Open VSX
with the `OVSX_PAT` token.

#### Scenario: Required production CI credentials are missing

- **WHEN** the `Publish VS Code extension` workflow targets the `production` environment for VS Code
  extension publication
- **AND** any of `AZURE_CLIENT_ID`, `AZURE_TENANT_ID` or `OVSX_PAT` is not configured
- **THEN** the workflow MUST fail before publishing to either registry

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
