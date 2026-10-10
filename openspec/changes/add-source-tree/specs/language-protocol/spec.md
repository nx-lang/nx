## RENAMED Requirements

- FROM: `### Requirement: The protocol defines four queries and names the rest`
- TO: `### Requirement: The protocol defines five queries and names the rest`

## MODIFIED Requirements

### Requirement: The protocol defines five queries and names the rest
The protocol SHALL define the queries `hover`, `completions`, `diagnostics`, `documentSymbols`, and
`sourceTree`, each with a request shape and an answer shape that round-trip through JSON without
loss. It SHALL reserve the query names `definition`, `references`, `rename`, `signatureHelp`,
`inlayHints`, and `semanticTokens` for future features. An implementation that does not support a
reserved query SHALL answer it with a distinguishable unsupported-query error rather than a
transport failure or a malformed answer.

#### Scenario: Hover answer shape
- **WHEN** the service has hover content for the queried position
- **THEN** the answer SHALL carry the content as markdown and the range the content applies to
- **AND** WHEN the service has nothing to say, the answer SHALL be an explicit empty result rather
  than an error

#### Scenario: Completion answer shape
- **WHEN** the service has completions for the queried position
- **THEN** each item SHALL carry a label, a kind drawn from the language service's completion kinds,
  an optional detail string, and optional documentation as markdown

#### Scenario: Diagnostics answer shape
- **WHEN** a diagnostics query is answered
- **THEN** the answer SHALL carry the diagnostics for the queried document, each with severity,
  message, optional code, a range, and optional related locations
- **AND** it SHALL carry separately any diagnostic not attributable to a document

#### Scenario: Source tree answer shape
- **WHEN** a source tree query is answered
- **THEN** the answer SHALL carry the document's URI, identity and version, its nodes and its
  declaration table as the `source-tree` capability defines them

#### Scenario: Unsupported query is distinguishable
- **WHEN** a client sends a reserved query to an implementation that does not support it
- **THEN** the client SHALL receive an error it can recognize as unsupported-query
- **AND** the error SHALL NOT be reported as a network or server fault
