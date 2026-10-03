## ADDED Requirements

### Requirement: The wasm SDK reaches standard libraries with no registry
Every wasm SDK entry point that analyzes or builds NX source SHALL resolve standard libraries from
the source the module carries: a single-source build, a workspace build, workspace validation, a
library load, a language snapshot and the in-process language service. No registry, build context,
document or option SHALL be needed for an `import "@nx/<name>"` to resolve, and the root of a
standard library SHALL be accepted wherever the SDK accepts implicit imports. A program that links a
standard library SHALL emit that library's module image by its identity through the same emit call
that emits any library module, with metadata carrying the identity and an image that records the
library's version, which an IR runtime answers when it prepares the image. The
answers SHALL equal the Node SDK's for the same inputs.

#### Scenario: A single source imports the agent library
- **WHEN** a caller builds an artifact from the source `import "@nx/agent"` followed by `let root() = { <Agent name="support">Be brief.</Agent> }`, with no build context
- **THEN** the build SHALL succeed
- **AND** `evaluateNx()` SHALL return `<Agent instructions="Be brief." name="support" />`

#### Scenario: Tenant source uses the agent library through a host's libraries
- **WHEN** a host loads `libraries/chat-link`, whose module contains `import "@nx/agent"`, into a registry
- **AND** builds a tenant workspace against a build context from it with implicit imports `["libraries/chat-link", "@nx/agent"]`
- **THEN** the tenant source SHALL use both libraries' declarations with no import line

#### Scenario: The library image is emitted beside the entry
- **WHEN** a caller asks that artifact for NX IR for the entry and `@nx/agent/agent.nx`
- **THEN** the result SHALL carry two artifacts, and the second's metadata SHALL name `@nx/agent/agent.nx`
- **AND** the second image, prepared by `@nx-lang/ir-runtime`, SHALL answer the library's version, equal to the version the entry image records for it
- **AND** `@nx-lang/ir-runtime` SHALL prepare both, link the entry against the library image and evaluate `root`

#### Scenario: Parity with the Node SDK
- **WHEN** the same workspace importing `@nx/agent` is built through the wasm SDK and the Node SDK
- **THEN** the emitted entry and library images SHALL be byte-identical and the diagnostics equal

#### Scenario: No new export is needed
- **WHEN** the built module's exports are compared with those of the release before this change
- **THEN** this change SHALL have added no export and changed no exchanged shape, so it alone SHALL NOT require a new ABI version
