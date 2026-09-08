## ADDED Requirements

### Requirement: Node SDK exposes the language service over in-memory documents
The Node SDK SHALL expose a language snapshot class constructed from in-memory documents — each with
a logical URI, source text, and optional identity and version — and an optional program build
context. The snapshot SHALL answer hover, completions, diagnostics, and document symbols as typed
JavaScript results whose shapes match the `language-protocol` answer shapes and whose property names
follow the SDK's existing camelCase conventions. The snapshot SHALL NOT require any document to exist
on disk.

#### Scenario: Snapshot builds from logical URIs
- **WHEN** a caller constructs a language snapshot from documents with `nx://` URIs
- **THEN** construction SHALL succeed without touching the filesystem
- **AND** queries SHALL address documents by those URIs

#### Scenario: Hover returns typed content
- **WHEN** a caller requests hover at a position with known metadata
- **THEN** the result SHALL carry markdown contents and a range with zero-based UTF-16 positions and
  byte offsets
- **AND** WHEN the position has no metadata, the result SHALL be `null`

#### Scenario: Completions return typed items
- **WHEN** a caller requests completions at a position
- **THEN** the result SHALL carry items with a label, a kind, and an optional detail

#### Scenario: Diagnostics are reported per document
- **WHEN** a caller requests diagnostics for a snapshot whose documents contain a type error
- **THEN** the result SHALL group diagnostics by document URI and version
- **AND** it SHALL carry workspace-level diagnostics separately

#### Scenario: Build context makes libraries visible
- **WHEN** a caller constructs the snapshot with a build context from a registry that loaded a
  library
- **AND** requests hover on a tag naming a component from that library
- **THEN** the result SHALL carry that component's signature

#### Scenario: Invalid input is reported as an SDK error
- **WHEN** a caller constructs a snapshot with an unparseable URI or two documents sharing an identity
- **THEN** construction SHALL throw an SDK error whose message names the offending URI or identity

#### Scenario: Snapshot lifecycle matches other native resources
- **WHEN** a caller disposes a snapshot
- **THEN** subsequent queries SHALL throw the SDK's disposed-resource error
- **AND** disposing twice SHALL be allowed
