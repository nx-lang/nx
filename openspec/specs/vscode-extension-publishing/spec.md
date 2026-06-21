# vscode-extension-publishing Specification

## Purpose
Define the VS Code extension packaging, verification, registry publishing, and maintainer release
documentation for the NX language extension.
## Requirements
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

### Requirement: VS Code extension package includes LSP runtime assets
The VS Code extension package SHALL include the compiled TypeScript extension client runtime and
the Rust `nx-lsp` server asset required for the package target. LSP runtime assets SHALL be included
without including development-only source files, tests, local editor settings, dependency caches, or
generated package artifacts.

#### Scenario: LSP-enabled package contains client runtime
- **WHEN** the VS Code extension is packaged after LSP client integration
- **THEN** the VSIX SHALL include the compiled extension client JavaScript needed by the `main`
  extension entry point
- **AND** it SHALL include package metadata needed for VS Code to activate the NX LSP client

#### Scenario: LSP-enabled package contains server binary
- **WHEN** the VS Code extension is packaged for a platform target that supports the Rust language
  server
- **THEN** the VSIX SHALL include the `nx-lsp` executable for that target
- **AND** the extension SHALL be able to locate that executable at runtime without requiring a
  separate user installation

### Requirement: VS Code package verification validates LSP assets
The local and CI package verification workflow SHALL verify that LSP-enabled packages contain the
expected extension runtime and server assets before publishing.

#### Scenario: Package verification detects missing server asset
- **WHEN** package verification runs for an LSP-enabled VS Code package
- **AND** the expected `nx-lsp` executable is missing from the packaged contents
- **THEN** package verification MUST fail before publishing

#### Scenario: Package verification still runs grammar tests
- **WHEN** package verification runs after LSP integration
- **THEN** the verification workflow SHALL continue to run the existing grammar and extension tests
- **AND** it SHALL add the LSP asset checks rather than replacing the existing checks

### Requirement: VS Code publishing supports native package targets
The VS Code extension publishing workflow SHALL support platform-specific package targets when a
release includes native `nx-lsp` binaries. Each published native package SHALL pair the extension
client with a server binary built for the corresponding target platform.

#### Scenario: Native package target uses matching server binary
- **WHEN** CI packages the VS Code extension for a native target
- **THEN** the packaged `nx-lsp` executable SHALL match that target platform
- **AND** the package verification step SHALL inspect the target package contents before publishing
