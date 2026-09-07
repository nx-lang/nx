# language-protocol Specification

## Purpose

Defines the transport-independent contract between a browser editor and the NX language service:
the shape of a language query and its answer, the position encoding both sides use, the
`NxLanguageService` interface every client implementation satisfies, and the behavior of the HTTP
client that implements it. Any host that can produce these requests and consume these responses can
offer NX language features, whether the service runs behind an HTTP route, in a worker, or in-process.

## Requirements

### Requirement: A language query carries the documents it is asked against
Every language query SHALL carry the complete set of documents the answer is to be computed from,
each with a logical URI, its full source text, and an optional monotonically increasing version, plus
the URI of the document the query is about. The service SHALL NOT retain document state between
queries, so two identical queries SHALL produce identical answers regardless of what was asked
before.

#### Scenario: A single-document host sends one document
- **WHEN** a host whose editor holds one NX document sends a hover query
- **THEN** the query SHALL contain that one document with a logical URI and its current text
- **AND** the query's document URI SHALL name that same document

#### Scenario: A multi-document host sends its sibling documents
- **WHEN** a host edits one document of a set that import one another
- **AND** it sends a completion query for the document being edited
- **THEN** the query SHALL contain every document of the set, with the edited document carrying the
  editor's unsaved text
- **AND** completions SHALL include declarations the edited document imports from a sibling

#### Scenario: Queries are independent
- **WHEN** a client sends a query, then a different query, then the first query again with identical
  content
- **THEN** the first and third answers SHALL be identical

#### Scenario: A version is echoed back
- **WHEN** a query's document carries a version
- **THEN** the answer SHALL carry that version, so the client can discard an answer that arrives after
  a newer version was sent

### Requirement: Positions and ranges are zero-based UTF-16 code unit offsets
A position in a query or an answer SHALL be a zero-based line number and a zero-based offset within
that line counted in UTF-16 code units. A range SHALL be a start and end position, and MAY carry
start and end byte offsets into the document's UTF-8 text alongside them.

#### Scenario: A position after a non-BMP character
- **WHEN** a line contains an emoji before the position a client asks about
- **THEN** the client SHALL count that emoji as two units when forming the position
- **AND** the service SHALL resolve the position to the character the client's editor shows under the
  cursor

#### Scenario: A range in an answer matches the editor's units
- **WHEN** an answer's range ends after an emoji on the same line
- **THEN** the range's end offset SHALL count that emoji as two units
- **AND** an editor that counts UTF-16 units SHALL highlight exactly the text the service meant

### Requirement: The protocol defines four queries and names the rest
The protocol SHALL define the queries `hover`, `completions`, `diagnostics`, and `documentSymbols`,
each with a request shape and an answer shape that round-trip through JSON without loss. It SHALL
reserve the query names `definition`, `references`, `rename`, `signatureHelp`, `inlayHints`, and
`semanticTokens` for future features. An implementation that does not support a reserved query
SHALL answer it with a distinguishable unsupported-query error rather than a transport failure or a
malformed answer.

#### Scenario: Hover answer shape
- **WHEN** the service has hover content for the queried position
- **THEN** the answer SHALL carry the content as markdown and the range the content applies to
- **AND** WHEN the service has nothing to say, the answer SHALL be an explicit empty result rather
  than an error

#### Scenario: Completion answer shape
- **WHEN** the service has completions for the queried position
- **THEN** each item SHALL carry a label, a kind drawn from the language service's completion kinds,
  and an optional detail string

#### Scenario: Diagnostics answer shape
- **WHEN** a diagnostics query is answered
- **THEN** the answer SHALL carry the diagnostics for the queried document, each with severity,
  message, optional code, a range, and optional related locations
- **AND** it SHALL carry separately any diagnostic not attributable to a document

#### Scenario: Unsupported query is distinguishable
- **WHEN** a client sends a reserved query to an implementation that does not support it
- **THEN** the client SHALL receive an error it can recognize as unsupported-query
- **AND** the error SHALL NOT be reported as a network or server fault

### Requirement: Every client implementation satisfies one service interface
The protocol package SHALL define an `NxLanguageService` interface with one method per supported
query, each accepting the query and an optional cancellation signal and returning a promise of the
answer. Editor integrations SHALL depend on that interface alone, so an HTTP-backed, worker-backed,
or in-process implementation is interchangeable without changing the editor integration.

#### Scenario: Cancellation aborts the query
- **WHEN** a caller cancels the signal it passed before the answer arrives
- **THEN** the returned promise SHALL reject with an abort error
- **AND** it SHALL NOT later resolve with an answer

#### Scenario: Implementations are interchangeable
- **WHEN** an editor integration is constructed with an in-process fake implementing the interface
- **THEN** the integration SHALL behave the same as with the HTTP implementation for identical
  answers

### Requirement: The HTTP client speaks the protocol over a configurable transport
The protocol SHALL have an HTTP client implementation of `NxLanguageService` constructed from a base
URL. The client SHALL accept a custom `fetch` function and a hook that supplies request headers, so a
host can attach authentication, and it SHALL enforce a per-request timeout.

#### Scenario: Requests carry host-supplied headers
- **WHEN** a host constructs the client with a header hook that returns an authorization header
- **THEN** every query request SHALL include that header

#### Scenario: A non-success status becomes a client error
- **WHEN** the server answers a query with a non-success status
- **THEN** the client SHALL reject with an error carrying the status and an excerpt of the response
  body
- **AND** it SHALL NOT resolve with a partial or empty answer

#### Scenario: A stalled request times out
- **WHEN** the server does not answer within the configured timeout
- **THEN** the client SHALL reject with an error identifying the timeout
- **AND** the underlying request SHALL be aborted
