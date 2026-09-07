## Purpose

Defines the one Monaco integration every NX web editor uses: language registration and highlighting
from the published NX editor assets, hover and completion providers over an `NxLanguageService`,
supply of sibling documents by the host, and mapping of protocol diagnostics to editor markers — so
that the DrawnUI fiddle, ReachMe's config editor, and any later host render and behave identically.

## ADDED Requirements

### Requirement: NX is registered and highlighted from the published editor assets
The integration SHALL register the `nx` language with Monaco using the language configuration from
the published `@nx-lang/language` package and SHALL highlight NX from that package's TextMate grammar
through Shiki. It SHALL accept the Shiki theme names to load and SHALL default to a light and a dark
theme so a host without a theme preference gets both.

#### Scenario: Highlighting follows the published grammar
- **WHEN** the integration is registered and a model in the `nx` language is displayed
- **THEN** tokens SHALL be colored according to the published NX grammar and the active Shiki theme
- **AND** no grammar file SHALL exist in the host application

#### Scenario: Hover code blocks are highlighted the same way
- **WHEN** a hover answer contains a fenced code block tagged `nx`
- **THEN** the rendered hover SHALL highlight that block with the same grammar as the editor buffer

#### Scenario: Highlighting works without a service
- **WHEN** the integration is registered with no language service
- **THEN** highlighting and language configuration SHALL be active
- **AND** no hover or completion provider SHALL be registered

### Requirement: Registration is idempotent
Registering the integration more than once against the same Monaco instance SHALL NOT register the
language, the highlighter, or any provider a second time. The registration SHALL return a disposable
that removes the providers it added.

#### Scenario: Two registrations, one set of providers
- **WHEN** a host calls the registration twice, as a React component that mounts twice will
- **AND** the user requests completions
- **THEN** each completion item SHALL appear once

#### Scenario: Disposal removes providers
- **WHEN** the host disposes the registration
- **THEN** hover and completion SHALL no longer be offered for `nx` models

### Requirement: Hover and completion are answered by the language service
When constructed with an `NxLanguageService`, the integration SHALL register a hover provider and a
completion provider for the `nx` language. Each SHALL convert Monaco's one-based line and column to
the protocol's zero-based UTF-16 position, send the current model text, convert answer ranges back,
and honor Monaco's cancellation token by cancelling the service call.

#### Scenario: Hover renders the service's markdown
- **WHEN** the user hovers a position the service has content for
- **THEN** Monaco SHALL show that markdown over the range the service reported

#### Scenario: Hover is silent where the service is
- **WHEN** the service answers a hover with an empty result
- **THEN** Monaco SHALL show no hover

#### Scenario: Completions use the language service's trigger characters
- **WHEN** the user types `<` or `:` in an `nx` model
- **THEN** the completion provider SHALL be invoked
- **AND** each item's kind SHALL be mapped from the protocol's completion kinds to Monaco's

#### Scenario: Unsaved text is what gets asked about
- **WHEN** the user edits and immediately hovers
- **THEN** the query SHALL carry the model's text at the moment of the hover, not a stale copy

#### Scenario: Cancellation propagates
- **WHEN** Monaco cancels a hover because the pointer moved
- **THEN** the service call SHALL be aborted

#### Scenario: A failing service does not break the editor
- **WHEN** a service call rejects
- **THEN** the provider SHALL return no result
- **AND** the failure SHALL be reported through the host's error hook rather than thrown into Monaco

### Requirement: The host supplies the workspace the query is asked against
By default a query SHALL carry the queried model alone, with its Monaco URI as the logical URI and
its version as the document version. A host MAY supply a callback that returns the full document set
for a model, so an editor over a multi-file configuration can include its sibling files.

#### Scenario: Default workspace is the model
- **WHEN** no workspace callback is supplied
- **THEN** each query SHALL contain exactly one document, with the model's URI and current text

#### Scenario: Host supplies siblings
- **WHEN** a host's workspace callback returns the model's document and two siblings
- **THEN** the query SHALL contain all three
- **AND** the query's document URI SHALL be the model's

### Requirement: Protocol diagnostics map to editor markers
The integration SHALL export a function that converts protocol diagnostics for a document into Monaco
markers: severity mapped to Monaco's, ranges converted to one-based columns, and a zero-width range
widened by one column so an insertion-point diagnostic remains visible. Diagnostics without a range
SHALL be omitted from markers.

#### Scenario: Severities and ranges map
- **WHEN** a diagnostic with warning severity and a non-empty range is converted
- **THEN** the marker SHALL have warning severity and the same span in one-based coordinates

#### Scenario: Insertion points stay visible
- **WHEN** a diagnostic's range is empty
- **THEN** the marker SHALL be one column wide at that position
