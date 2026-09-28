## ADDED Requirements

### Requirement: The wasm SDK loads shared libraries once and builds against them
The wasm SDK SHALL expose a library registry on a host that loads in-memory libraries, and build
contexts created from it that workspace builds and workspace validation accept. Libraries loaded
into a registry SHALL be analyzed once and reused by every build and validation that uses a build
context from that registry, until the registry is disposed. Registries and build contexts SHALL
follow the SDK's explicit lifecycle and crashed-host rules.

#### Scenario: Many tenant builds reuse one analyzed library
- **WHEN** a host loads `libraries/chat-link` and `libraries/question-flow` into a registry
- **AND** builds 100 different tenant workspaces against a build context from that registry
- **THEN** each build SHALL resolve the libraries through the snapshots loaded once
- **AND** the libraries SHALL NOT be re-analyzed per build

#### Scenario: Disposed registries are refused
- **WHEN** a caller disposes a registry and then creates a build context from it
- **THEN** the call SHALL throw the SDK's disposed-resource error

#### Scenario: A crashed host invalidates its registries
- **WHEN** a call on a host traps
- **THEN** every registry and build context on that host SHALL report the crashed-host error
- **AND** a fresh host from the same compiled module SHALL be able to load the libraries again

### Requirement: Library modules emit NX IR images
A program built against loaded libraries SHALL be able to emit an NX IR image for any library
module in its module table, named by an identity that combines the library root and the
library-relative module identity, and recording the library's version. Each library module image
SHALL be identical whichever tenant program it was emitted from, so a host can prepare it once and
link many entry modules against it.

#### Scenario: A library module image is independent of the tenant program
- **WHEN** two different tenant workspaces are built against the same loaded `libraries/question-flow`
- **AND** each emits the image for `libraries/question-flow/QuestionFlow.nx`
- **THEN** the two images SHALL be byte-identical

#### Scenario: Entry images link to library images
- **WHEN** a caller emits the entry module and every library module it links against
- **THEN** the IR runtime SHALL prepare each image and link the entry against the library images
  by the identities and versions recorded in the entry's module table
