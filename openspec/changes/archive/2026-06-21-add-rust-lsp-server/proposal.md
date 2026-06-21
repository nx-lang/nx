## Why

NX editing currently stops at TextMate syntax highlighting in VS Code and the ReachMe business owner
UI. A Rust language server can reuse the existing parser, lowering, type analysis, workspace
analysis, and diagnostics directly, giving both VS Code and future web-based NX editors richer,
consistent language intelligence without duplicating NX semantics in TypeScript or C#.

## What Changes

- Add a reusable Rust editor-language-service layer that exposes diagnostics, document symbols,
  hover information, and completion data over logical NX workspaces.
- Add a standalone Rust `nx-lsp` language server that speaks Language Server Protocol over stdio for
  VS Code and keeps its analysis model independent of local filesystem-only assumptions.
- Update the VS Code extension from a declarative grammar-only package to an extension runtime that
  starts `nx-lsp`, synchronizes `.nx` documents, and surfaces LSP diagnostics and language features.
- Preserve the existing TextMate grammar and snippets as the baseline highlighting and editing
  experience.
- Prepare the server architecture for later ReachMe browser/editor integration through logical
  document identities and a transport-independent analysis core.

## Capabilities

### New Capabilities

- `editor-language-service`: Reusable Rust editor-analysis APIs for NX diagnostics, document
  symbols, hovers, completions, and logical workspace snapshots.
- `language-server-protocol`: The standalone Rust NX LSP server, its supported protocol surface,
  document synchronization behavior, and editor-facing language features.

### Modified Capabilities

- `vscode-extension-publishing`: VS Code extension packaging and verification must account for the
  compiled extension client runtime and platform-specific Rust LSP server assets.

## Impact

- Adds new Rust workspace crates for editor analysis and the LSP binary.
- Reuses `nx-syntax`, `nx-hir`, `nx-types`, `nx-api`, `nx-diagnostics`, and existing workspace
  source-provider behavior.
- Adds VS Code extension activation/client code, runtime dependencies, package allowlist changes,
  tests, and CI packaging updates.
- Adds native binary packaging/release considerations for VS Code targets.
- Establishes a path for ReachMe to consume the same language-service behavior later through a
  browser/editor LSP bridge or a direct Rust service integration.
