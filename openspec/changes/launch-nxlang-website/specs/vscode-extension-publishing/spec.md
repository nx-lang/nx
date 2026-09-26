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
