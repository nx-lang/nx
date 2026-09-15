## ADDED Requirements

### Requirement: The TypeScript IR runtime is published as an npm package
The TypeScript IR runtime SHALL be publishable and published to the npm registry as
`@nx-lang/ir-runtime`, so that a browser or Node host can evaluate persisted NX IR without an NX
checkout. The published package SHALL carry its type declarations.

#### Scenario: A host installs the runtime from the registry
- **WHEN** a JavaScript project installs the published `@nx-lang/ir-runtime` package
- **THEN** it SHALL be able to prepare an NX IR program and evaluate its `root` entrypoint, with
  TypeScript types available for the exported API

#### Scenario: Runtime and SDK versions agree
- **WHEN** a host installs `@nx-lang/ir-runtime` and `@nx-lang/sdk-wasm` at the same release version
- **THEN** IR emitted by the SDK SHALL satisfy the runtime's format, schema and required-feature
  checks
