# editor-language-service Specification

## Purpose
Define the Rust editor language-service API for NX documents, including workspace snapshots,
diagnostics, document symbols, hover, completions, and stale-result rejection independent of LSP
protocol types.

## Requirements
### Requirement: Language service owns logical editor workspace snapshots
The system SHALL expose a Rust editor language-service API that accepts NX documents as logical
workspace modules independent of LSP protocol types and independent of filesystem-only paths. Each
document in a snapshot SHALL preserve a client URI, a normalized NX module identity, source text,
and a monotonically increasing document version when supplied by the editor client.

#### Scenario: Filesystem document maps to a workspace identity
- **WHEN** a client submits a `file://` NX document from a workspace folder
- **THEN** the language service SHALL preserve the source text under a normalized NX module identity
  derived from the workspace-relative path
- **AND** diagnostics and editor query results for that module SHALL retain enough information to
  map back to the original client URI

#### Scenario: Virtual document does not require filesystem access
- **WHEN** a client submits a virtual NX document with a logical URI such as `nx://tenant/form.nx`
- **THEN** the language service SHALL analyze the submitted source text without requiring a file
  with that identity to exist on disk
- **AND** diagnostics and editor query results SHALL use the submitted logical identity and URI

### Requirement: Language service returns editor diagnostics from static analysis
The language service SHALL expose diagnostics for a workspace snapshot by reusing NX static
analysis, including syntax validation, lowering, scope resolution, type checking, and workspace
import diagnostics. Returned diagnostics SHALL include severity, message, code when available,
primary range, and any secondary labels that can be represented as related locations.

#### Scenario: Static analysis diagnostics are projected for editors
- **WHEN** a workspace snapshot contains an NX type error
- **THEN** the language service SHALL return an editor diagnostic for the affected document
- **AND** the diagnostic SHALL preserve the NX diagnostic severity, code, message, and source range

#### Scenario: Diagnostics are cleared when a document becomes valid
- **WHEN** a document version previously produced diagnostics
- **AND** a later version of the same document analyzes with no diagnostics
- **THEN** the language service SHALL report an empty diagnostics list for that document version

### Requirement: Language service exposes document symbols
The language service SHALL expose document symbols for top-level NX declarations that can be
identified from the parsed and lowered module, including functions, values, records, unions,
actions, components, and top-level root elements where applicable.

#### Scenario: Top-level declarations become document symbols
- **WHEN** a document declares a component, a record type, and a root function
- **THEN** the language service SHALL return document symbols for each top-level declaration
- **AND** each symbol SHALL include the declaration name, kind, selection range, and enclosing range

### Requirement: Language service exposes hover information
The language service SHALL expose hover information at positions where NX can identify a declaration,
reference, type annotation, component tag, property, or expression with known metadata. Hover
results SHALL be conservative: if the language service cannot determine useful information, it
SHALL return no hover rather than fabricate incomplete semantic data.

#### Scenario: Hover over declaration shows declaration information
- **WHEN** a client requests hover on the name of a function declaration
- **THEN** the language service SHALL return hover content identifying the symbol kind and available
  signature or type information

#### Scenario: Hover on unknown syntax returns no result
- **WHEN** a client requests hover on a position where no useful NX metadata is available
- **THEN** the language service SHALL return no hover result

### Requirement: Language service exposes conservative completions
The language service SHALL expose completion items based on the current document context. MVP
completion sources SHALL include NX keywords, primitive type names, visible top-level declarations,
component or tag names, and component property names when the current syntax context and available
metadata make those completions valid.

#### Scenario: Type position includes primitive and visible type completions
- **WHEN** a client requests completions in an NX type annotation position
- **THEN** the language service SHALL include primitive NX type names
- **AND** it SHALL include visible record, union, enum, component, or action type names that are
  available in the current workspace snapshot

#### Scenario: Component property position includes property completions
- **WHEN** a client requests completions inside an element opening tag for a known component
- **THEN** the language service SHALL include undeclared properties accepted by that component
- **AND** it SHALL NOT include properties already supplied in that opening tag

### Requirement: Language service rejects stale editor results
The language service SHALL associate diagnostics and query results with the snapshot or document
version used to compute them. Editor integrations SHALL be able to determine whether a result is
stale relative to a newer submitted document version.

#### Scenario: Older diagnostic result is superseded by a newer edit
- **WHEN** document version 3 is submitted while analysis for version 2 is still running
- **THEN** the language service SHALL preserve enough version metadata for the integration layer to
  avoid publishing version 2 diagnostics over version 3 state
