## ADDED Requirements

### Requirement: Node SDK package is source-buildable and Node-only
The repository SHALL expose a Node package named `@nx-lang/sdk-node` for native NX
host/compiler/runtime SDK access, backed by napi-rs / N-API and maintained separately from the pure
TypeScript NX IR runtime.

#### Scenario: Package layout is discoverable
- **WHEN** a contributor inspects the repository
- **THEN** the Node SDK package SHALL live under the bindings area
- **AND** its native Rust crate, TypeScript wrapper, declarations, package metadata, tests, and
  documentation SHALL be discoverable from that package root

#### Scenario: Package is Node-only
- **WHEN** a consumer reads the Node SDK documentation
- **THEN** the documentation SHALL identify the package as Node-only native SDK access for NX host,
  compiler, artifact, diagnostics, and evaluation workflows
- **AND** it SHALL NOT present the package as a browser, WASM, or persisted-IR-only runtime

#### Scenario: TypeScript IR runtime remains distinct
- **WHEN** a consumer wants to evaluate an already persisted NX IR JSON document in JavaScript
- **THEN** the documentation SHALL continue to direct that workflow to the pure TypeScript IR
  runtime rather than the Node SDK package

### Requirement: Node SDK API is high-level and TypeScript-friendly
The Node SDK SHALL expose a high-level JavaScript and TypeScript API for NX workspaces, workspace
modules, library registries, build contexts, reusable program artifacts, IR generation,
evaluation, diagnostics, and errors. Public callers SHALL NOT need to manage raw native pointers or
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
- **AND** byte evaluation results SHALL be returned as `Buffer` or an equivalent Node-compatible
  binary value

### Requirement: Node workspaces support in-memory modules with logical identities
The Node SDK SHALL let callers create in-memory NX workspaces from source modules that have stable
logical identities and string or UTF-8 byte source payloads. Workspace behavior SHALL follow the
existing NX logical identity normalization rules and SHALL NOT require normal workspace modules to
exist on disk.

#### Scenario: Database-backed source builds without temp files
- **WHEN** a Node caller creates a workspace containing `chat-link.nx` and `lib/builtins.nx` from
  in-memory source payloads
- **THEN** NX SHALL treat both as workspace modules with those logical identities
- **AND** the caller SHALL be able to validate or build the workspace without writing temp files

#### Scenario: Duplicate normalized identities are rejected
- **WHEN** a Node caller submits workspace modules named `lib/config.nx` and `lib/./config.nx`
- **THEN** workspace creation, validation, or program build SHALL reject the duplicate normalized
  identity
- **AND** the resulting error or diagnostic SHALL name the normalized identity involved

#### Scenario: Workspace modules satisfy imports before build context lookup
- **WHEN** a workspace module imports another submitted workspace module by logical identity
- **THEN** NX SHALL resolve that import from the submitted workspace modules
- **AND** NX SHALL NOT require the imported module to be loaded from a library registry or disk

### Requirement: Node workspace validation returns structured diagnostics
The Node SDK SHALL expose workspace validation against a supplied program build context and return
structured diagnostics for all user-authored NX validation failures without treating those failures
as native interop errors.

#### Scenario: Valid workspace returns no diagnostics
- **WHEN** a Node caller validates a workspace with valid imports and type-correct NX source
- **THEN** validation SHALL return an empty diagnostics array

#### Scenario: Invalid workspace aggregates module diagnostics
- **WHEN** one workspace module contains a missing import
- **AND** another workspace module contains invalid NX source
- **THEN** validation SHALL return structured diagnostics for both modules in one result
- **AND** each diagnostic SHALL preserve severity, message, and source identity or span metadata
  when available

#### Scenario: Validation uses the supplied build context
- **WHEN** a workspace imports a preloaded library that is visible through the supplied build
  context
- **THEN** validation SHALL resolve that import through the build context
- **AND** the same workspace SHALL report a missing-library diagnostic when validated against a
  build context where that library is not visible

### Requirement: Node program artifacts build from source and workspaces
The Node SDK SHALL expose reusable `NxProgramArtifact` creation from source text and from
`NxWorkspace` plus an explicit entry identity. Workspace program builds SHALL use the supplied
`NxProgramBuildContext` and SHALL surface NX analysis or entrypoint failures as structured
diagnostics.

#### Scenario: Workspace artifact builds for selected entry module
- **WHEN** a Node caller builds a program artifact from a workspace containing `app/main.nx`
- **AND** the caller selects entry identity `app/main.nx`
- **THEN** the build SHALL return a reusable `NxProgramArtifact`
- **AND** the artifact SHALL preserve the selected entry identity for diagnostics, IR generation,
  and evaluation

#### Scenario: Missing workspace entry reports diagnostics
- **WHEN** a Node caller builds a workspace program artifact with entry identity `missing.nx`
- **THEN** the build SHALL fail with an `NxEvaluationError` or equivalent typed error
- **AND** the error SHALL contain structured diagnostics naming `missing.nx`
- **AND** no usable program artifact SHALL be returned

#### Scenario: Source artifact builds against a supplied build context
- **WHEN** a Node caller builds a source program artifact that imports a preloaded library
- **AND** the supplied build context exposes that library
- **THEN** the build SHALL succeed through the registry-backed host pipeline

### Requirement: Node native resources have explicit lifecycle semantics
The Node SDK SHALL define lifecycle semantics for native resources including library registries,
program build contexts, and program artifacts. Long-lived resources SHALL support explicit
disposal, and using a disposed resource SHALL fail predictably.

#### Scenario: Program artifact remains usable after build context disposal
- **WHEN** a Node caller builds a program artifact from a workspace using a build context
- **AND** the caller disposes the build context
- **THEN** evaluating the program artifact or generating IR from it SHALL still succeed

#### Scenario: Disposed artifact rejects further operations
- **WHEN** a Node caller disposes a program artifact
- **AND** later calls `evaluateJson`, `evaluateBytes`, or `generateNxIr` on that artifact
- **THEN** the call SHALL fail with a typed disposed-resource error

#### Scenario: Finalizers are a backstop, not the primary contract
- **WHEN** a Node process allows a native resource wrapper to be garbage collected
- **THEN** the binding SHALL release the underlying native resource eventually
- **AND** documentation SHALL still present explicit disposal as the supported lifecycle for
  server-side reuse

### Requirement: Node SDK generates deterministic NX IR JSON and metadata
The Node SDK SHALL expose deterministic NX IR generation from a reusable program artifact and from
source convenience APIs by delegating to the shared Rust IR emission pipeline.

#### Scenario: Artifact emits IR JSON and metadata
- **WHEN** a Node caller generates NX IR from a valid program artifact containing `root()`
- **THEN** the result SHALL include deterministic IR JSON text
- **AND** the result SHALL include structured metadata such as runtime ABI, program fingerprint,
  entrypoints, and references exposed by the shared IR generator

#### Scenario: Equivalent workspace inputs produce equivalent IR
- **WHEN** two Node callers build program artifacts from equivalent workspace module identities,
  source payloads, entry identities, and build contexts
- **THEN** generated NX IR JSON SHALL be equivalent for cache-key purposes
- **AND** generated metadata SHALL identify the same program fingerprint

#### Scenario: Invalid artifact does not emit partial IR
- **WHEN** NX analysis prevents creation of a valid program artifact
- **THEN** the Node SDK SHALL surface structured diagnostics from the build failure
- **AND** it SHALL NOT return partial NX IR JSON

### Requirement: Node SDK evaluates supported entrypoints to JSON or bytes
The Node SDK SHALL expose evaluation APIs for the entrypoint behavior supported by the underlying
native host, including `root()` JSON and byte evaluation parity with the existing .NET SDK.
Evaluation failures SHALL surface structured diagnostics.

#### Scenario: Program artifact evaluates root to JSON
- **WHEN** a Node caller evaluates a valid program artifact whose `root()` returns a record value
- **THEN** `evaluateJson` SHALL return the normalized JSON-compatible NX value

#### Scenario: Program artifact evaluates root to bytes
- **WHEN** a Node caller evaluates a valid program artifact through the byte API
- **THEN** the result SHALL contain the canonical byte representation produced by the native NX
  runtime for the requested output format

#### Scenario: Missing root entrypoint reports diagnostics
- **WHEN** a Node caller evaluates a program artifact that has no public `root()` entrypoint
- **THEN** evaluation SHALL fail with a typed NX evaluation error
- **AND** the error SHALL contain structured diagnostics describing the missing entrypoint

#### Scenario: Unsupported named entrypoint does not fall back to global lookup
- **WHEN** a Node caller requests evaluation of an entrypoint not supported by the current native
  host API
- **THEN** the binding SHALL fail with an actionable typed error or diagnostic
- **AND** it SHALL NOT execute an arbitrary same-named declaration through JavaScript-side lookup

### Requirement: Node diagnostics and errors are ergonomic and stable
The Node SDK SHALL expose stable diagnostic and error types that preserve the structured data
emitted by Rust host APIs while feeling natural to JavaScript and TypeScript consumers.

#### Scenario: NX diagnostics preserve structured fields
- **WHEN** validation, build, IR generation, or evaluation produces NX diagnostics
- **THEN** each diagnostic exposed to Node SHALL include severity and message
- **AND** it SHALL include code, labels, help, note, source identity, and span fields when the Rust
  diagnostic provides them

#### Scenario: NX evaluation errors carry diagnostics
- **WHEN** a Node operation fails because NX source is invalid or an entrypoint cannot be evaluated
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
- **THEN** they SHALL cover deterministic NX IR generation, metadata shape, JSON evaluation, byte
  evaluation where supported, missing `root()` diagnostics, and comparison against existing Rust,
  .NET SDK, or CLI behavior for the same NX inputs

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
