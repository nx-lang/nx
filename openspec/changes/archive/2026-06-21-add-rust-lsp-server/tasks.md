## 1. Crate Setup And Architecture Decisions

- [x] 1.1 Resolve the Rust LSP framework choice with a minimal stdio spike and update `design.md` with the selected framework.
- [x] 1.2 Add `crates/nx-language-service` to the Cargo workspace with dependencies on the existing NX syntax, HIR, type, API, and diagnostics crates needed for editor analysis.
- [x] 1.3 Add `crates/nx-lsp` to the Cargo workspace as a standalone binary crate with the selected LSP framework dependencies.
- [x] 1.4 Define shared language-service data models for document URI, normalized NX identity, document version, text ranges, diagnostics, symbols, hovers, and completions.

## 2. Language Service Snapshots And Diagnostics

- [x] 2.1 Implement language-service workspace snapshots that accept filesystem and virtual documents without depending on LSP protocol types.
- [x] 2.2 Implement URI-to-NX-identity mapping for workspace-relative `file://` documents and logical non-file URIs.
- [x] 2.3 Reuse existing NX workspace/static analysis to compute diagnostics for a snapshot.
- [x] 2.4 Project NX diagnostics into editor diagnostics with severity, code, message, primary range, and secondary related locations.
- [x] 2.5 Preserve snapshot or document-version metadata so integrations can reject stale diagnostic results.
- [x] 2.6 Add Rust tests for filesystem identities, virtual identities, invalid-to-valid diagnostic clearing, and stale-version metadata.

## 3. Language Service Editor Queries

- [x] 3.1 Implement document symbol extraction for top-level functions, values, records, unions, actions, components, and top-level root elements.
- [x] 3.2 Implement conservative hover results for declarations and syntax positions with known kind, signature, or type metadata.
- [x] 3.3 Implement conservative completions for keywords, primitive types, visible top-level declarations, component/tag names, and supported component property contexts.
- [x] 3.4 Add Rust tests for document symbols, declaration hovers, no-result hovers, type-position completions, and component property completions.

## 4. Rust LSP Server

- [x] 4.1 Implement `nx-lsp --stdio` startup, LSP initialization, graceful shutdown, and MVP capability advertisement.
- [x] 4.2 Implement full-document `didOpen`, `didChange`, and `didClose` synchronization against language-service snapshots.
- [x] 4.3 Implement debounced diagnostics publishing with document-version checks to avoid stale diagnostics.
- [x] 4.4 Implement LSP document symbol, hover, and completion handlers by adapting language-service responses.
- [x] 4.5 Support synchronized virtual NX document URIs without filesystem access.
- [x] 4.6 Add protocol-level tests that exercise initialize, diagnostics publish/clear, document symbols, hover, completion, and virtual URI analysis.

## 5. VS Code Client Integration

- [x] 5.1 Add TypeScript extension activation code and configure the extension `main` entry point while preserving existing language, grammar, and snippet contributions.
- [x] 5.2 Add `vscode-languageclient` runtime dependency and TypeScript build/test scripts for the VS Code extension client.
- [x] 5.3 Implement server path resolution that prefers a configured `nx-lsp` path for development and otherwise locates the packaged server binary.
- [x] 5.4 Start the LSP client for NX documents and wire the document selector to `.nx` files and the `nx` language id.
- [x] 5.5 Add an output channel or equivalent logging surface for server startup and runtime failures while keeping grammar-only highlighting available.
- [x] 5.6 Add VS Code extension tests or unit tests for server path resolution, activation configuration, and failure reporting.

## 6. Packaging, CI, And Documentation

- [x] 6.1 Add a repeatable build step that produces the `nx-lsp` executable for local extension development.
- [x] 6.2 Update the VS Code package allowlist so compiled client runtime files and packaged `nx-lsp` assets are included while development files remain excluded.
- [x] 6.3 Add package verification checks that fail when the compiled client runtime or expected `nx-lsp` server asset is missing.
- [x] 6.4 Update the VS Code CI workflow to build the Rust server before package verification.
- [x] 6.5 Add or document native VSIX target packaging for supported server platforms.
- [x] 6.6 Update the VS Code README, TODO/roadmap notes, and changelog to describe the Rust LSP server, configuration, and packaging behavior.

## 7. Final Verification

- [x] 7.1 Run Rust formatting and tests for the new language-service and LSP crates plus affected existing crates.
- [x] 7.2 Run VS Code extension tests and package verification from `src/vscode`.
- [x] 7.3 Run OpenSpec validation/status for `add-rust-lsp-server` and address any artifact or requirement issues.
- [ ] 7.4 Manually smoke-test the VS Code extension by opening invalid and valid `.nx` files and confirming diagnostics, symbols, hover, and completions.
