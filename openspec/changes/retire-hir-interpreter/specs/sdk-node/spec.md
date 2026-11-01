## ADDED Requirements

### Requirement: Node compiler SDK leaves execution to the IR runtime
The Node compiler SDK SHALL NOT expose source or program-artifact JSON/byte evaluation methods.
It SHALL continue to emit IR consumable by the standalone TypeScript IR runtime.

#### Scenario: A Node host executes compiled source
- **WHEN** a host builds source and emits its module images through the Node SDK
- **THEN** it SHALL be able to execute those images through `@nx-lang/ir-runtime`
- **AND** SHALL NOT require an SDK source-evaluation method

## MODIFIED Requirements

### Requirement: Node SDK API is high-level and TypeScript-friendly
The Node SDK SHALL expose a high-level JavaScript and TypeScript API for NX workspaces, workspace
modules, library registries, build contexts, reusable program artifacts, IR generation,
diagnostics, and errors. Public callers SHALL NOT need to manage raw native pointers or
unstable native handle identifiers.

#### Scenario: TypeScript declarations describe public models
- **WHEN** a TypeScript consumer imports `@nx-lang/sdk-node`
- **THEN** declarations SHALL describe public options, result objects, diagnostics, errors, and
  resource-owning classes without requiring `any` for normal workflows

#### Scenario: Raw native handles are hidden
- **WHEN** a JavaScript consumer creates a workspace, build context, or program artifact
- **THEN** the public object SHALL NOT expose a raw pointer, numeric handle, or BigInt handle as the
  primary way to call NX

#### Scenario: Byte-oriented APIs use Node-compatible buffers
- **WHEN** a caller submits source bytes or requests byte output
- **THEN** the public API SHALL accept `Buffer` or `Uint8Array` inputs where bytes are appropriate
- **AND** emitted IR images SHALL be returned as `Buffer` or an equivalent Node-compatible
  binary value

### Requirement: Node diagnostics and errors are ergonomic and stable
The Node SDK SHALL expose stable diagnostic and error types that preserve the structured data
emitted by Rust host APIs while feeling natural to JavaScript and TypeScript consumers.

#### Scenario: NX diagnostics preserve structured fields
- **WHEN** validation, build, or IR generation produces NX diagnostics
- **THEN** each diagnostic exposed to Node SHALL include severity and message
- **AND** it SHALL include code, labels, help, note, source identity, and span fields when the Rust
  diagnostic provides them

#### Scenario: NX evaluation errors carry diagnostics
- **WHEN** a Node operation fails because NX source is invalid or IR generation fails
- **THEN** the thrown error SHALL be distinguishable from native load or interop failures
- **AND** it SHALL expose the diagnostics array without requiring callers to parse an error string

#### Scenario: Native load failures are actionable
- **WHEN** the native Node module cannot be loaded or its ABI is incompatible
- **THEN** the error SHALL identify the native binding problem
- **AND** it SHALL provide guidance consistent with local source consumption and future package
  distribution workflows

### Requirement: Node SDK package includes parity tests and packaging guidance
The Node SDK implementation SHALL include automated tests and documentation that prove the package
can be consumed from local source and behaves consistently with existing Rust, .NET SDK, or CLI
behavior for equivalent host operations.

#### Scenario: Tests cover validation and build parity
- **WHEN** the Node SDK tests run
- **THEN** they SHALL cover valid workspace validation, invalid workspace validation, workspace
  program builds, duplicate module handling, missing entry identities, and invalid source
  diagnostics

#### Scenario: Tests cover IR generation and evaluation parity
- **WHEN** the Node SDK tests run
- **THEN** they SHALL cover deterministic NX IR generation, metadata shape and invalid source diagnostics
- **AND** tests SHALL execute emitted images through the standalone IR runtime and compare with
  explicit expected values and native IR runtime results, without an SDK evaluation method

#### Scenario: Local source consumption is documented
- **WHEN** a downstream repository vendors or references NX source
- **THEN** the Node SDK documentation SHALL explain how to install dependencies, build the native
  module locally, import the TypeScript/JavaScript API, and run tests without requiring a published
  npm package

#### Scenario: Future npm and prebuild distribution path is documented
- **WHEN** maintainers prepare package distribution
- **THEN** package documentation or build metadata SHALL identify the intended napi-rs prebuild
  workflow, supported Node/platform targets, and how published artifacts relate to the local
  source-built workflow

## REMOVED Requirements

### Requirement: Node SDK evaluates supported entrypoints to JSON or bytes
**Reason**: The legacy source/artifact execution binding is removed.
**Migration**: Compile source to IR and execute through an existing IR runtime. No replacement binding is introduced.
