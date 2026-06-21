## Context

The NX VS Code extension currently contributes language metadata, TextMate grammars, and snippets,
but it does not run extension activation code or a language server. The core NX implementation is
already Rust-first: parsing, syntax validation, HIR lowering, source analysis, workspace programs,
symbol resolution, diagnostics, code generation, and runtime assembly all live in Rust crates.

ReachMe also has an NX editing surface that currently benefits from syntax highlighting but not
from server-backed language intelligence. That editor is not guaranteed to use filesystem paths:
documents may come from database-backed business-owner configuration records and should be
addressable by logical identities.

This change should therefore add an editor-facing language service in Rust and expose it through a
standalone LSP server. VS Code is the first client, while ReachMe is a future client that should be
able to reuse the same language behavior through an LSP bridge or direct service integration.

## Goals / Non-Goals

**Goals:**

- Add a Rust language-service layer that exposes editor-oriented analysis without depending on LSP
  protocol types.
- Add a standalone Rust `nx-lsp` server that supports the MVP LSP surface for `.nx` files.
- Integrate the VS Code extension with `nx-lsp` while preserving existing TextMate highlighting and
  snippets.
- Support logical document identities so non-filesystem editors can use the same language-service
  model later.
- Provide automated tests for core language-service queries, LSP request handling, and VS Code
  package verification.

**Non-Goals:**

- Do not implement formatting, rename, find references, or go-to-definition in the MVP.
- Do not replace the TextMate grammar with semantic tokens in this change.
- Do not build the ReachMe product integration in this change.
- Do not require C# or TypeScript implementations of NX semantic analysis.
- Do not require incremental tree-sitter edit handling until full-document reanalysis proves too
  slow for the target file sizes.

## Decisions

### Decision: Use Rust as the LSP implementation language

The LSP server SHALL be implemented in Rust because the parser, diagnostics, workspace analysis,
HIR, and type checker already live in Rust. This avoids FFI overhead and avoids duplicating NX
semantics in C# or TypeScript.

Alternatives considered:

- **C# LSP over `NxLang.Runtime`**: Useful when C# was preferred, but it turns the server into an
  adapter around Rust and makes richer editor queries depend on new managed DTOs.
- **TypeScript LSP**: Natural for VS Code, but it would either duplicate NX semantics or call into
  Rust through native/WASM bindings.

### Decision: Add a reusable `nx-language-service` crate under the LSP

The implementation SHALL add a Rust language-service crate that owns document snapshots, logical
workspace analysis, and editor queries such as diagnostics, document symbols, hovers, and
completions. The LSP crate SHALL translate protocol requests into language-service calls and
translate language-service responses back into LSP data.

This keeps ReachMe options open: a future web editor can communicate with `nx-lsp` over a bridge,
or a ReachMe backend can embed the language-service crate directly without speaking LSP internally.

Alternatives considered:

- **Put all analysis inside `nx-lsp`**: Faster to start, but couples reusable editor intelligence to
  LSP protocol types and VS Code session assumptions.
- **Expose only existing `nx-api` validation**: Good for diagnostics, but insufficient for symbols,
  hovers, and completions.

### Decision: Start with full-document synchronization and snapshot reanalysis

The MVP SHALL use full document synchronization. The server SHALL debounce analysis requests,
associate results with document versions, and avoid publishing stale diagnostics after newer edits.

Alternatives considered:

- **Incremental synchronization and tree-sitter edits immediately**: Better long-term performance,
  but adds complexity before there is evidence that full reanalysis is too slow for typical NX
  documents.
- **Analyze only on save**: Simpler, but too weak for editor diagnostics and completions.

### Decision: Model documents by URI plus normalized NX identity

The language-service layer SHALL separate client document URIs from NX module identities. File
editors can map `file://` URIs to workspace-relative module identities, while non-filesystem
editors can use logical URIs such as `nx://...` and provide stable identities directly.

The MVP VS Code client SHALL use filesystem-backed `.nx` files. The service design SHALL avoid
requiring filesystem canonicalization for all documents, preserving a path to ReachMe logical
workspace integration.

Alternatives considered:

- **Use filesystem paths everywhere**: Simplest for VS Code, but blocks database-backed or virtual
  documents.
- **Invent a ReachMe-specific transport now**: Premature for this change; the core identity model is
  enough to avoid boxing the design in.

### Decision: MVP feature set is diagnostics, symbols, hovers, and completions

Diagnostics are the first priority and SHALL include syntax, lowering, scope, type, and workspace
diagnostics. Document symbols, hovers, and completions are included because they exercise the
reusable language-service surface and deliver meaningful value in both VS Code and ReachMe editing
contexts.

Completions SHALL start conservatively with keywords, primitive types, visible top-level
declarations, component/tag names, and component properties where existing metadata supports them.

Alternatives considered:

- **Diagnostics-only MVP**: Smaller, but it fails to validate the reusable editor-query architecture
  and under-delivers for the ReachMe editing scenario.
- **Full IDE feature set**: Too broad for the first server version.

### Decision: Package `nx-lsp` as a native VS Code server asset

The VS Code extension SHALL start a packaged `nx-lsp` binary by default and allow a configured
server path for development or advanced users. Packaging SHOULD use platform-specific VSIX targets
once native server assets are included.

Alternatives considered:

- **Require users to install `nx-lsp` separately**: Easier packaging, but poor first-run experience.
- **Run through Cargo in development and release**: Useful for local debugging, but not appropriate
  for published extension users.

## Risks / Trade-offs

- **Editor-query data is not yet exposed cleanly by existing analysis crates** -> Add narrow
  language-service projection types instead of leaking HIR internals or duplicating parsing logic.
- **Completion and hover quality may be uneven at first** -> Ship conservative, correct answers and
  expand context awareness after the basic query pipeline is proven.
- **Native VSIX packaging adds CI/release complexity** -> Keep the extension client thin, add
  package verification tests, and support a configurable server path for development fallback.
- **ReachMe integration requirements may differ from VS Code** -> Keep the core service independent
  of LSP and filesystem assumptions; defer ReachMe-specific transport until product integration.
- **Full-document reanalysis may become slow on larger workspaces** -> Debounce, cache snapshots,
  measure request latency, and add incremental parsing only when benchmarks justify it.

## Migration Plan

1. Add the Rust language-service and LSP crates to the workspace without changing the existing VS
   Code extension behavior.
2. Implement and test diagnostics, symbols, hovers, and completions through language-service APIs.
3. Implement `nx-lsp --stdio` and protocol-level tests against the binary.
4. Add VS Code activation/client code behind normal `nx` language activation.
5. Update VS Code package contents, local verification scripts, CI, README, and release notes.
6. Package platform-specific server binaries once the client/server integration is stable.

Rollback is straightforward before publishing: remove the VS Code LSP client activation and package
assets while leaving the Rust crates in place for continued development. After publishing, a patch
release can disable server startup by default or fall back to grammar-only behavior if the native
server fails to start.

## Open Questions

- Should arm64 VS Code extension targets be added in the first native LSP release or after x64
  packages have been exercised?
- Should ReachMe eventually talk to `nx-lsp` over a WebSocket bridge, or should a ReachMe backend
  embed `nx-language-service` directly and expose product-specific editor endpoints?

## Resolved Framework Spike

The MVP LSP server SHALL use `tower-lsp`. A small stdio server can advertise the required
initialize capabilities with less protocol dispatch boilerplate than lower-level `lsp-server`, and
the framework keeps request/notification payloads strongly typed through `lsp-types`. The reusable
editor behavior remains isolated in `nx-language-service`, so choosing `tower-lsp` does not couple
the language-service data model to LSP protocol types.

## Resolved VS Code Package Matrix

The first native LSP-enabled VS Code release SHALL publish x64 packages for Linux, macOS, and
Windows. Each package build runs on the matching GitHub Actions runner, builds the native
`nx-lsp` server for that platform, verifies the packaged client and server assets, and passes the
matching `--target` value to `vsce package`.
