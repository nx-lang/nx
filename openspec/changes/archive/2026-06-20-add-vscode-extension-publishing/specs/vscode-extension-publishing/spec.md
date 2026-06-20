## ADDED Requirements

### Requirement: VS Code extension package verification

The repository SHALL provide a repeatable way to package the NX VS Code extension into a VSIX and
inspect the package contents before publishing.

#### Scenario: Local package verification

- **WHEN** a maintainer runs the documented package verification command from `src/vscode`
- **THEN** the command SHALL run the extension test suite before producing or validating the VSIX
- **AND** the generated VSIX contents SHALL be visible to the maintainer before any publish command
  is run

#### Scenario: Development files are excluded from the VSIX

- **WHEN** the VS Code extension is packaged for release
- **THEN** the VSIX SHALL include the extension manifest, README, changelog, license, language
  configuration, TextMate grammars, and snippets
- **AND** the VSIX SHALL exclude tests, samples, lockfiles, workspace metadata, local editor
  settings, generated VSIX files, and dependency directories

### Requirement: VS Code extension versioned release trigger

The automated publishing workflow SHALL publish only from `vscode-v<version>` tags whose version
matches `src/vscode/package.json`.

#### Scenario: Tag version matches package version

- **WHEN** a tag named `vscode-v0.1.0` triggers the VS Code extension workflow
- **AND** `src/vscode/package.json` declares version `0.1.0`
- **THEN** the workflow SHALL allow the package and publish steps to continue after tests pass

#### Scenario: Tag version does not match package version

- **WHEN** a tag named `vscode-v0.1.1` triggers the VS Code extension workflow
- **AND** `src/vscode/package.json` declares version `0.1.0`
- **THEN** the workflow MUST fail before publishing to any registry

### Requirement: VS Code extension registry publishing

The repository SHALL support publishing the same verified VSIX artifact to both the Visual Studio
Marketplace and Open VSX.

#### Scenario: Publish to both registries from CI

- **WHEN** a matching `vscode-v<version>` tag triggers the VS Code extension workflow
- **AND** extension tests pass
- **AND** package verification passes
- **AND** `VSCE_PAT` and `OVSX_PAT` are configured
- **THEN** the workflow SHALL publish the verified VSIX to the Visual Studio Marketplace
- **AND** the workflow SHALL publish the same verified VSIX to Open VSX

#### Scenario: Publish commands use the packaged artifact

- **WHEN** the automated workflow publishes the VS Code extension
- **THEN** both registry publish commands SHALL use the VSIX artifact produced by the package step
- **AND** neither registry publish step SHALL rebuild a different package implicitly

### Requirement: VS Code extension publishing credentials

The publishing workflow SHALL keep registry credentials outside source control and fail safely when
credentials are missing.

#### Scenario: Required CI credentials are missing

- **WHEN** a matching `vscode-v<version>` tag triggers the VS Code extension workflow
- **AND** either `VSCE_PAT` or `OVSX_PAT` is not configured
- **THEN** the workflow MUST fail before publishing to either registry

#### Scenario: Local credentials are supplied through environment variables

- **WHEN** a maintainer follows the documented local publishing commands
- **THEN** the commands SHALL read Marketplace and Open VSX credentials from environment variables
- **AND** the documentation MUST NOT instruct maintainers to commit tokens or write them into
  tracked configuration files

### Requirement: VS Code extension release documentation

The VS Code extension documentation SHALL describe the supported release process for maintainers.

#### Scenario: Maintainer prepares a release

- **WHEN** a maintainer reads the VS Code extension publishing documentation
- **THEN** the documentation SHALL describe how to update the extension version and changelog
- **AND** it SHALL describe how to install the expected package manager, run tests, package the
  extension, inspect the VSIX, configure registry credentials, and publish locally or by pushing a
  `vscode-v<version>` tag
