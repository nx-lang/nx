## REMOVED Requirements

### Requirement: A prelude document is prepended transparently
**Reason**: Prepending host text to the queried document and shifting every position was a
workaround for context declarations that could not be imported. Implicit imports replaced it — the
playground already uses them — and the language now has a prelude of its own, so keeping a second,
unrelated feature under the same name would mislead.
**Migration**: Pass the host's declarations as a context document and name its identity as an
implicit import, as the requirement "Host context is served as implicitly imported documents"
specifies. Positions need no shifting, because the queried document's text is analyzed as written.

## ADDED Requirements

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
