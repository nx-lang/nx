# language-http-service Specification

## Purpose

Defines the mountable Node handler that answers `language-protocol` queries through the NX Node SDK,
so that any Node host — a plain `http` server, a Hono or Express application — can offer NX language
features by mounting one function, without managing sessions or the language service's lifetime.

## Requirements

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

### Requirement: Host context is served as implicitly imported documents
The factory SHALL accept an optional host context: a list of documents, each with a URI and source,
and a list of identities every queried document imports implicitly. The context documents SHALL be
part of every query's document set without the client sending them, and a client document with the
same identity as a context document SHALL be refused as a malformed request. The queried document's
text SHALL be analyzed exactly as the client sent it, so every position in a query and every range
in an answer is in the document's own coordinates with no shifting. A diagnostic located in a
context document SHALL be reported under that document's URI and SHALL NOT be positioned in the
queried document.

#### Scenario: Hover through host context
- **WHEN** a context document declares a component, its identity is implicitly imported, and a hover query lands on a tag naming it in the queried document
- **THEN** the answer SHALL carry that component's signature
- **AND** the answer's range SHALL be in the queried document's own lines and columns

#### Scenario: Completions through host context
- **WHEN** a completion query lands inside an opening tag of a component a context document declares
- **THEN** the answer SHALL offer that component's properties

#### Scenario: A context document's own error is not blamed on the queried document
- **WHEN** a context document contains a type error and a diagnostics query is answered for a client document
- **THEN** the diagnostic SHALL be reported under the context document's URI
- **AND** diagnostics in the queried document SHALL carry that document's own coordinates

#### Scenario: A client cannot replace a context document
- **WHEN** a request's documents include one whose URI equals a context document's
- **THEN** the handler SHALL refuse the request as malformed, naming that URI

#### Scenario: A client cannot replace a context document by naming its identity
- **WHEN** a context document is declared with a URI and no identity of its own, and a request's documents include one under a different URI whose identity is the one that context document's URI derives to
- **THEN** the handler SHALL refuse the request as malformed, naming that identity
