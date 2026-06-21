# language-server-protocol Specification

## Purpose
Define the Rust `nx-lsp` Language Server Protocol server, VS Code client integration, and runtime
behavior for NX editor features over stdio.

## Requirements
### Requirement: Rust NX language server initializes over LSP
The repository SHALL provide a standalone Rust `nx-lsp` server that speaks Language Server Protocol
over stdio for editor clients. During initialization, the server SHALL advertise full-document text
synchronization, diagnostics through publish diagnostics, document symbols, hover, and completion
capabilities for NX documents.

#### Scenario: LSP initialize advertises MVP capabilities
- **WHEN** an LSP client sends `initialize` to `nx-lsp`
- **THEN** the server SHALL respond successfully
- **AND** the response SHALL advertise full-document text synchronization, document symbol, hover,
  and completion capabilities

### Requirement: Language server synchronizes open NX documents
The language server SHALL maintain an in-memory snapshot of open NX documents from
`textDocument/didOpen`, `textDocument/didChange`, and `textDocument/didClose` notifications. The
MVP server SHALL accept full-document change events and SHALL debounce analysis after edits.

#### Scenario: Open document publishes diagnostics
- **WHEN** an LSP client opens an invalid `.nx` document
- **THEN** the server SHALL analyze the document through the Rust language service
- **AND** it SHALL publish diagnostics for that document URI

#### Scenario: Changed document replaces diagnostics
- **WHEN** an LSP client changes an invalid document to valid NX source
- **THEN** the server SHALL analyze the newer document version
- **AND** it SHALL publish an empty diagnostics list for that document URI

#### Scenario: Closed document clears diagnostics
- **WHEN** an LSP client closes an NX document that previously had diagnostics
- **THEN** the server SHALL publish an empty diagnostics list for that document URI
- **AND** it SHALL remove the open-document source text from the live snapshot

### Requirement: Language server supports logical document URIs
The language server SHALL accept NX documents whose URI scheme is not `file` when the client
submits source text through LSP document synchronization. The server SHALL map those documents to
logical NX identities without requiring filesystem access.

#### Scenario: Virtual URI document is analyzed
- **WHEN** an LSP client opens a document with URI `nx://tenant/form.nx`
- **AND** the document language identifier is `nx`
- **THEN** the server SHALL analyze the submitted text as an NX module
- **AND** it SHALL publish diagnostics back to the same virtual URI

### Requirement: Language server adapts editor query requests
The language server SHALL implement document symbol, hover, and completion requests by delegating
to the Rust language-service APIs and translating results into LSP response types.

#### Scenario: Document symbol request returns NX declarations
- **WHEN** an LSP client requests `textDocument/documentSymbol` for an open NX document
- **THEN** the server SHALL return symbols produced by the language service for that document

#### Scenario: Hover request returns language-service hover content
- **WHEN** an LSP client requests `textDocument/hover` over known NX syntax
- **THEN** the server SHALL return the hover content produced by the language service

#### Scenario: Completion request returns language-service completions
- **WHEN** an LSP client requests `textDocument/completion` in a supported NX context
- **THEN** the server SHALL return the completion items produced by the language service

### Requirement: VS Code extension starts the Rust language server
The VS Code extension SHALL activate for NX documents and start an LSP client connected to the Rust
`nx-lsp` server. The extension SHALL preserve the existing TextMate grammar and snippets while LSP
features are layered on top.

#### Scenario: Opening an NX file starts LSP features
- **WHEN** a user opens a `.nx` file in VS Code with the NX extension installed
- **THEN** the extension SHALL start or reuse an `nx-lsp` server process
- **AND** the editor SHALL receive diagnostics, document symbols, hovers, and completions through
  the LSP client when the server is available

#### Scenario: Configured server path is used for development
- **WHEN** the user configures an explicit `nx-lsp` server path
- **THEN** the VS Code extension SHALL start the configured server path instead of the packaged
  server binary

### Requirement: Server startup failures are visible to users
The VS Code extension SHALL expose server startup and runtime failures through a VS Code output
channel or equivalent diagnostic surface so users and maintainers can diagnose failed LSP startup
without losing baseline TextMate highlighting.

#### Scenario: Packaged server cannot start
- **WHEN** the VS Code extension cannot start the configured or packaged `nx-lsp` server
- **THEN** the extension SHALL report the failure in an output channel or user-visible diagnostic
  surface
- **AND** the existing TextMate grammar-based highlighting SHALL remain available
