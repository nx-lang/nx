## MODIFIED Requirements

### Requirement: Node SDK package is source-buildable and Node-only
The repository SHALL expose a Node package named `@nx-lang/sdk-node` for native NX
host/compiler/runtime SDK access, backed by napi-rs / N-API and maintained separately from the pure
TypeScript NX IR runtime and from the WebAssembly SDK.

#### Scenario: Package layout is discoverable
- **WHEN** a contributor inspects the repository
- **THEN** the Node SDK package SHALL live under the bindings area
- **AND** its native Rust crate, TypeScript wrapper, declarations, package metadata, tests, and
  documentation SHALL be discoverable from that package root

#### Scenario: Package is Node-only
- **WHEN** a consumer reads the Node SDK documentation
- **THEN** the documentation SHALL identify the package as Node-only native SDK access for NX host,
  compiler, artifact, diagnostics, and evaluation workflows
- **AND** it SHALL direct browser and WebAssembly consumers to `@nx-lang/sdk-wasm`

#### Scenario: TypeScript IR runtime remains distinct
- **WHEN** a consumer wants to evaluate an already persisted NX IR JSON document in JavaScript
- **THEN** the documentation SHALL continue to direct that workflow to the pure TypeScript IR
  runtime rather than the Node SDK package
