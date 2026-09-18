## ADDED Requirements

### Requirement: The SDK explains an NX IR artifact
The SDK SHALL render an NX IR image it is given as the same readable text the CLI's `ir explain`
prints for that image, so an artifact can be read on the page that holds it. A malformed image
SHALL be reported as an SDK error carrying the diagnostic rather than as a trap.

#### Scenario: A page reads an artifact it holds
- **WHEN** a page holds the bytes of an artifact and asks the SDK to explain them
- **THEN** it SHALL receive the text the CLI would print for that artifact

#### Scenario: A damaged artifact is reported
- **WHEN** a page asks the SDK to explain bytes that are not a valid image
- **THEN** the SDK SHALL throw an error naming what is malformed
- **AND** the module SHALL remain usable

## MODIFIED Requirements

### Requirement: Program artifacts build from source and emit NX IR
The SDK SHALL build a program artifact from a workspace of in-memory modules, each with a logical
identity, its source text and an optional version string, together with an entry identity and an
optional list of identities every other module imports implicitly. Building from a single source
text and a file name SHALL remain available as the one-module case. From a built artifact the SDK
SHALL emit NX IR for the modules the caller names, or for the entry alone by default, returning one
artifact per module as bytes with its metadata, and SHALL include the debug section only when
asked. Build failures SHALL be reported as structured diagnostics with the same shape and property
names the Node SDK reports, each against the identity of the module it belongs to.

#### Scenario: Valid source builds and emits IR
- **WHEN** a caller builds an artifact from a single source that compiles
- **AND** asks it for NX IR
- **THEN** the result SHALL carry one artifact's bytes and the same metadata the Node SDK reports
- **AND** the bytes SHALL be identical to what the Node SDK emits for the same source, file name
  and options

#### Scenario: The bytes are the caller's
- **WHEN** a caller receives an artifact's bytes and the SDK's result has been released
- **THEN** the bytes SHALL remain readable
- **AND** the IR runtime SHALL prepare them

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

#### Scenario: The SDK emits debug data on request
- **WHEN** a caller asks the SDK for NX IR with the debug section
- **THEN** the artifact SHALL carry spans and source text, and SHALL otherwise equal the artifact
  emitted without them

#### Scenario: Invalid source reports diagnostics
- **WHEN** a caller builds a workspace in which `input.nx` does not compile
- **THEN** the build SHALL throw an SDK evaluation error carrying every diagnostic
- **AND** each diagnostic SHALL carry its severity, code, message and labels with byte and
  line-column spans against `input.nx`, never shifted by another module's text

#### Scenario: Artifact lifecycle is explicit
- **WHEN** a caller disposes an artifact
- **THEN** subsequent operations on it SHALL throw the SDK's disposed-resource error
- **AND** disposing twice SHALL be allowed

## REMOVED Requirements

### Requirement: Emitted NX IR is compact JSON
**Reason**: An NX IR artifact is a binary image; there is no compact or pretty text to choose
between, and the CLI's files are read with `nxlang ir explain`.
**Migration**: A caller that parsed the `json` text reads the `bytes` instead and prepares them with
the IR runtime; the debug section is still omitted unless asked, under *Program artifacts build from
source and emit NX IR*.
