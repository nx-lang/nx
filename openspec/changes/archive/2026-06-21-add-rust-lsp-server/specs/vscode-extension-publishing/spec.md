## ADDED Requirements

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
