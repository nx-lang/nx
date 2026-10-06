## ADDED Requirements

### Requirement: Library registries resolve standard libraries without a load
Every `LibraryRegistry`, including an empty one, SHALL resolve an import of a standard library to
that library's process-wide analyzed snapshot without the host loading anything. A library whose
modules import a standard library SHALL record that root in its dependency metadata like any other
dependency, SHALL load whether it comes from a directory or from memory, and SHALL preserve the
semantic bindings to the standard library's declarations that IR generation needs. A standard
library SHALL NOT be reported as a missing dependency, SHALL NOT take part in a dependency cycle
with a host library, and SHALL be the same snapshot in every registry of the process. A registry
SHALL refuse to load a host library whose root lies under the reserved `@nx/` root.

#### Scenario: A host library that imports the agent library loads into an empty registry
- **WHEN** a host creates a registry and loads `libraries/chat-link` from memory, whose module contains `import "@nx/agent"` and `export type AssistantConfig = { enabled?:boolean agent?:Agent }`
- **THEN** the load SHALL succeed
- **AND** the registry SHALL record `@nx/agent` as a dependency of `libraries/chat-link`

#### Scenario: Directory and in-memory loads agree
- **WHEN** the same host library that imports `@nx/agent` is loaded once from a directory and once from memory
- **THEN** both loads SHALL succeed and a program built against each SHALL report equal diagnostics

#### Scenario: Two registries share the standard library
- **WHEN** two registries in one process each load a library that imports `@nx/agent`
- **THEN** both SHALL bind to the same analyzed `@nx/agent` snapshot
- **AND** `@nx/agent/agent.nx` images emitted through programs built from each SHALL be byte-identical

#### Scenario: A program selects the standard library through a host library
- **WHEN** a workspace module imports only `libraries/chat-link`, which imports `@nx/agent`, and contains `let root() = { <AssistantConfig enabled=true /> }`
- **THEN** the built program's library closure SHALL include `@nx/agent`
- **AND** emitting every module SHALL include `@nx/agent/agent.nx`

#### Scenario: A host library under the reserved root is refused
- **WHEN** a host loads a library with root `@nx/agent` from memory
- **THEN** the load SHALL fail with a diagnostic naming the root and saying `@nx/` is reserved
