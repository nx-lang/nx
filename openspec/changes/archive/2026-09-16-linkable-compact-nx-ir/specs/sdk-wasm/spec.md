## MODIFIED Requirements

### Requirement: Program artifacts build from source and emit NX IR
The SDK SHALL build a program artifact from a workspace of in-memory modules, each with a logical
identity, its source text and an optional version string, together with an entry identity and an
optional list of identities every other module imports implicitly. Building from a single source
text and a file name SHALL remain available as the one-module case. From a built artifact the SDK
SHALL emit NX IR for the modules the caller names, or for the entry alone by default, returning one
artifact per module with its metadata, and SHALL include the debug section only when asked. Build
failures SHALL be reported as structured diagnostics with the same shape and property names the
Node SDK reports, each against the identity of the module it belongs to.

#### Scenario: Valid source builds and emits IR
- **WHEN** a caller builds an artifact from a single source that compiles
- **AND** asks it for NX IR
- **THEN** the result SHALL carry one artifact's JSON text and the same metadata the Node SDK reports
- **AND** the artifact SHALL be byte-identical to what the Node SDK emits for the same source, file
  name and options

#### Scenario: A snippet is emitted without the catalog it uses
- **WHEN** a caller builds a workspace of `drawnui` at version `9` and `input.nx`, with `drawnui`
  implicitly imported and `input.nx` as the entry
- **AND** asks for NX IR
- **THEN** the result SHALL carry one artifact, for `input.nx`, whose module table names `drawnui`
  with version `9`
- **AND** the artifact SHALL be free of `drawnui`'s declarations and of any debug section

#### Scenario: A catalog is emitted on its own
- **WHEN** a caller builds a workspace of `drawnui` alone with `drawnui` as the entry
- **AND** asks for NX IR
- **THEN** the result SHALL carry one artifact for `drawnui` whose module table holds only itself

#### Scenario: Invalid source reports diagnostics
- **WHEN** a caller builds a workspace in which `input.nx` does not compile
- **THEN** the build SHALL throw an SDK evaluation error carrying every diagnostic
- **AND** each diagnostic SHALL carry its severity, code, message and labels with byte and
  line-column spans against `input.nx`, never shifted by another module's text

#### Scenario: Artifact lifecycle is explicit
- **WHEN** a caller disposes an artifact
- **THEN** subsequent operations on it SHALL throw the SDK's disposed-resource error
- **AND** disposing twice SHALL be allowed

### Requirement: Emitted NX IR is compact JSON
NX IR produced through the wasm SDK and the FFI SHALL be serialized without indentation or line
breaks between tokens and SHALL omit the debug section unless the caller asks for it. Its content
SHALL be otherwise unchanged: parsing compact and pretty-printed output of the same program with
the same debug choice SHALL yield equal values.

#### Scenario: The SDK emits compact IR
- **WHEN** a program artifact's NX IR is generated through the wasm SDK
- **THEN** the JSON text SHALL contain no newline characters and no debug section, and the IR
  runtime SHALL prepare it

#### Scenario: The SDK emits debug data on request
- **WHEN** a caller asks the wasm SDK for NX IR with the debug section
- **THEN** the artifact SHALL carry spans and source text, and SHALL otherwise equal the artifact
  emitted without them

#### Scenario: The CLI's files stay readable
- **WHEN** the CLI writes an NX IR file
- **THEN** the file SHALL remain pretty-printed

## REMOVED Requirements

### Requirement: A prelude-aware build classifies diagnostics by origin
**Reason**: The prelude compiled a host's context declarations and the document as one module,
which put the whole context into every emitted artifact and forced every diagnostic through a
coordinate shift. Implicit imports make the context its own module, so the document's diagnostics
are already in its own coordinates and a fault in the context is reported against the context.
**Migration**: Build a workspace with the context as a module and the document as the entry, naming
the context in the implicit-import list. A diagnostic against the context's identity is what the
`catalog` origin used to mean; one against the document's identity is what `source` meant.

### Requirement: An in-process language service is provided over the snapshot
**Reason**: The service prepended a prelude to the queried document and shifted every position,
which is the concatenation the rest of this change retires; host context now reaches the service as
documents of its own with an implicit import.
**Migration**: Construct the service with the host's documents and the implicit-import list instead
of a prelude. Answers are already in the queried document's coordinates; a diagnostic in a host
document is reported against that document's URI, which is what the prelude origin used to mean.

## ADDED Requirements

### Requirement: An in-process language service sees host documents through implicit imports
The SDK SHALL provide an implementation of the protocol's `NxLanguageService` interface that
answers queries in the same process from language snapshots. The service SHALL accept documents
the host always includes alongside the queried document set and a list of identities every queried
document imports implicitly, so a host's context declarations are visible to hover, completion and
diagnostics without appearing in the queried document's text. Its answers SHALL match those of a
snapshot built from the same documents, and identical document sets SHALL share one analysis.

#### Scenario: Hover through an implicit import
- **WHEN** the service is constructed with a `drawnui` document declaring a component and `drawnui`
  as an implicit import
- **AND** a hover query lands on a tag naming that component in the queried document
- **THEN** the answer SHALL carry that component's signature
- **AND** its range SHALL be expressed in the queried document's own lines and columns

#### Scenario: Completions through an implicit import
- **WHEN** a completion query lands inside an opening tag of an implicitly imported component
- **THEN** the answer SHALL offer that component's properties

#### Scenario: A fault in a host document is not the queried document's
- **WHEN** a diagnostics query is answered and a diagnostic belongs to a host-supplied document
- **THEN** it SHALL be reported against that document's URI, not the queried document's

#### Scenario: Repeated queries over unchanged text analyze once
- **WHEN** several queries arrive for the same document set
- **THEN** the documents SHALL be analyzed once
- **AND** each answer SHALL carry the version the request that asked for it supplied

#### Scenario: Cancellation is honored
- **WHEN** a caller cancels the signal it passed before the answer is produced
- **THEN** the returned promise SHALL reject with an abort error
