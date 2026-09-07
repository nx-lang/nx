## Purpose

Defines the mountable Node handler that answers `language-protocol` queries through the NX Node SDK,
so that any Node host — a plain `http` server, a Hono or Express application — can offer NX language
features by mounting one function, without managing sessions or the language service's lifetime.

## ADDED Requirements

### Requirement: The handler is a Fetch-shaped function a host mounts under a base path
The package SHALL export a factory that returns a handler taking a standard `Request` and returning a
promise of a standard `Response`. The handler SHALL route on the final segment of the request path,
so the host chooses the base path. It SHALL accept only `POST` with a JSON body and SHALL answer
every request with JSON. The package SHALL also export an adapter that presents the same handler
with Node's `http` listener signature.

#### Scenario: Mounted under a host-chosen path
- **WHEN** a host mounts the handler so that `/api/language/hover` reaches it
- **THEN** a `POST` to that path with a hover query body SHALL be answered with a hover answer

#### Scenario: Wrong method or unknown query
- **WHEN** a request uses a method other than `POST`, or names a query segment the handler does not
  define
- **THEN** the handler SHALL answer with a client-error status and a JSON body naming the problem
- **AND** a reserved but unsupported query SHALL be answered with the unsupported-query error the
  protocol defines

#### Scenario: Node listener adapter
- **WHEN** a host passes the handler through the Node adapter and registers the result with
  `http.createServer`
- **THEN** requests SHALL be answered identically to the Fetch-shaped handler

### Requirement: The handler is stateless and safe to run in many instances
The handler SHALL keep no per-client or per-document state between requests. Every answer SHALL be
computed from the request body alone, so that a host running several instances behind a load
balancer needs no session affinity.

#### Scenario: Any instance answers any request
- **WHEN** the same query is sent to two freshly constructed handlers
- **THEN** both SHALL return the same answer

### Requirement: Identical document sets share one analysis
The handler SHALL cache the analysis of a document set keyed by its content, so that a burst of
queries against unchanged text — hover on several positions, then a completion — builds the analysis
once. The cache SHALL be bounded and SHALL never return an analysis built from different content.

#### Scenario: Repeated queries over unchanged text reuse analysis
- **WHEN** several queries arrive whose document sets are byte-identical
- **THEN** the language service snapshot SHALL be constructed once for that content

#### Scenario: Changed text is never served a stale analysis
- **WHEN** a query arrives whose document set differs by one character from a cached one
- **THEN** the answer SHALL be computed from the new text

### Requirement: Oversized and malformed requests are refused without harming the process
The handler SHALL refuse a request whose body exceeds a configured byte limit and a body that is not
a well-formed query, with a client-error status. A failure inside the language service SHALL be
answered with a server-error status and a JSON body, and SHALL NOT terminate the host process.

#### Scenario: Body too large
- **WHEN** a request body exceeds the configured limit
- **THEN** the handler SHALL answer with a payload-too-large status

#### Scenario: Body is not a query
- **WHEN** a request body is not JSON, or is JSON missing the documents or the document URI
- **THEN** the handler SHALL answer with a bad-request status and a message naming what is missing

#### Scenario: Service failure is contained
- **WHEN** the language service throws while answering
- **THEN** the handler SHALL answer with a server-error status and a JSON error body
- **AND** the host process SHALL continue to answer subsequent requests

### Requirement: The handler analyzes against a host-supplied build context
The factory SHALL accept an optional Node SDK program build context. Every query SHALL be analyzed
with the libraries that context makes visible, so a host whose built-in NX modules are loaded through
a library registry gets hover, completions and diagnostics that know those modules.

#### Scenario: Library component is visible through the context
- **WHEN** the handler is constructed with a build context whose registry loaded a library declaring
  a component
- **AND** a hover query lands on a tag naming that component in a request document
- **THEN** the answer SHALL carry that component's signature

#### Scenario: No context means no libraries
- **WHEN** the handler is constructed without a build context
- **THEN** names declared only in a library SHALL be treated as unresolved, as the compiler treats
  them

### Requirement: A prelude document is prepended transparently
The factory SHALL accept an optional prelude — NX source that is placed ahead of the queried
document's text before analysis, for hosts whose context declarations cannot yet be imported without
loss. When a prelude is configured, every position in a query SHALL be shifted into the combined
text, every range in an answer SHALL be shifted back into the queried document's own coordinates, and
a diagnostic whose range falls inside the prelude SHALL be reported as a prelude-origin diagnostic
without a range rather than positioned in the queried document.

#### Scenario: Hover through a prelude
- **WHEN** a prelude declares a component and a hover query lands on a tag naming it in the queried
  document
- **THEN** the answer SHALL carry that component's signature
- **AND** the answer's range SHALL be expressed in the queried document's own lines and columns

#### Scenario: Completions through a prelude
- **WHEN** a completion query lands inside an opening tag of a prelude-declared component
- **THEN** the answer SHALL offer that component's properties

#### Scenario: Prelude-internal diagnostics are not blamed on the document
- **WHEN** a diagnostics query is answered and a diagnostic's range lies inside the prelude
- **THEN** it SHALL be reported with a prelude origin and no range
- **AND** diagnostics positioned in the queried document SHALL carry the document's own coordinates

#### Scenario: A prelude with no trailing newline still separates cleanly
- **WHEN** the prelude source does not end in a newline
- **THEN** the first line of the queried document SHALL still be its own line in the combined text
