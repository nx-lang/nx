# sdk-wasm Specification

## Purpose

The NX compiler, NX IR codegen and language service as a WebAssembly module, and the
`@nx-lang/sdk-wasm` package that loads it in a browser or in Node: build a program from source, emit
NX IR, and answer editor queries over in-memory documents, with no server and no native addon.

## Requirements

### Requirement: WASM SDK package is source-buildable and host-agnostic
The repository SHALL expose a package named `@nx-lang/sdk-wasm` under the bindings area, backed by a
Rust crate in the same directory that compiles the NX compiler, NX IR codegen and language service
to one WebAssembly module targeting WASI preview 1. The package SHALL load in a modern browser and in
Node without a native addon, and its build SHALL be reproducible from a clean checkout with the
toolchain the repository documents and pins.

#### Scenario: Package layout is discoverable
- **WHEN** a contributor inspects the bindings area
- **THEN** the wasm package's Rust crate, TypeScript wrapper, declarations, package metadata, tests
  and documentation SHALL be discoverable from that package root

#### Scenario: Build is reproducible from a clean checkout
- **WHEN** the package is built on a machine with the documented toolchain and no cached artifacts
- **THEN** the build SHALL fetch the pinned WASI sysroot it needs into a gitignored location
- **AND** it SHALL produce the module and the wrapper without steps performed outside the build

#### Scenario: Module needs only standard WASI imports
- **WHEN** the built module's imports are listed
- **THEN** every import SHALL belong to the WASI preview 1 namespace
- **AND** the module SHALL instantiate under Node's built-in WASI implementation and under a
  browser WASI shim with no host-specific imports

#### Scenario: Package documents its scope
- **WHEN** a consumer reads the package's documentation
- **THEN** it SHALL identify the package as browser and Node access to compilation, NX IR generation
  and the language service
- **AND** it SHALL direct evaluation of persisted NX IR to the TypeScript IR runtime and
  filesystem-backed workflows to the Node SDK

### Requirement: The module's ABI is versioned and self-describing
The module SHALL export an ABI version the wrapper checks at load time, and SHALL exchange every
input and output as UTF-8 bytes in the module's memory through allocation and release functions the
module itself exports, so the wrapper never guesses at memory layout or reuses memory the module
owns.

#### Scenario: Version mismatch is refused
- **WHEN** the wrapper loads a module whose ABI version is not the one it was written against
- **THEN** loading SHALL fail with an error naming both versions
- **AND** no query SHALL be attempted against that module

#### Scenario: Results are released after reading
- **WHEN** the wrapper reads a result the module produced
- **THEN** it SHALL release that result through the module's release function before returning
- **AND** repeated calls SHALL NOT grow the module's memory by the size of the results produced

### Requirement: Program artifacts build from source and emit NX IR
The SDK SHALL build a program artifact from a single NX source text and a file name, and SHALL emit
NX IR JSON and metadata from it. Build failures SHALL be reported as structured diagnostics with the
same shape and property names the Node SDK reports.

#### Scenario: Valid source builds and emits IR
- **WHEN** a caller builds an artifact from source that compiles
- **AND** asks it for NX IR
- **THEN** the result SHALL carry the IR JSON text and the same metadata the Node SDK reports
- **AND** the IR SHALL be byte-identical to what the Node SDK emits for the same source and file
  name

#### Scenario: Invalid source reports diagnostics
- **WHEN** a caller builds an artifact from source that does not compile
- **THEN** the build SHALL throw an SDK evaluation error carrying every diagnostic
- **AND** each diagnostic SHALL carry its severity, code, message and labels with byte and
  line-column spans against the given file name

#### Scenario: Artifact lifecycle is explicit
- **WHEN** a caller disposes an artifact
- **THEN** subsequent operations on it SHALL throw the SDK's disposed-resource error
- **AND** disposing twice SHALL be allowed

### Requirement: Language snapshots answer editor queries over in-memory documents
The SDK SHALL expose a language snapshot constructed from in-memory documents, each with a logical
URI, source text and optional identity and version, that answers hover, completions, diagnostics
and document symbols with the `language-protocol` answer shapes. The snapshot SHALL NOT require any
document to exist on a filesystem.

#### Scenario: Answers match the Node SDK
- **WHEN** the same documents and positions are queried through the wasm SDK and the Node SDK
- **THEN** the hover, completion, diagnostic and document symbol answers SHALL be equal

#### Scenario: Invalid input is reported as an SDK error
- **WHEN** a caller constructs a snapshot with an unparseable URI or two documents sharing an
  identity
- **THEN** construction SHALL throw an SDK evaluation error whose message names the offending URI
  or identity

#### Scenario: Snapshot lifecycle is explicit
- **WHEN** a caller disposes a snapshot
- **THEN** subsequent queries SHALL throw the SDK's disposed-resource error
- **AND** disposing twice SHALL be allowed

### Requirement: A trapped module is reported, not reused
Because the module cannot unwind, a panic or an out-of-bounds access inside it ends the instance.
The SDK SHALL surface that as a crashed-host error on the call that trapped, SHALL refuse every
later call on the same host with the same error, and SHALL let the caller load a fresh host from
the already-compiled module without fetching it again.

#### Scenario: A trap is reported on the call that caused it
- **WHEN** a call into the module traps
- **THEN** that call SHALL throw a crashed-host error that names the operation
- **AND** it SHALL NOT return a partial result

#### Scenario: A crashed host stays crashed
- **WHEN** any call is made on a host that has trapped
- **THEN** it SHALL throw the crashed-host error without entering the module

#### Scenario: A replacement host is cheap
- **WHEN** a caller creates a new host from the compiled module after a trap
- **THEN** the new host SHALL answer queries normally
- **AND** creating it SHALL NOT require downloading or recompiling the module

### Requirement: An in-process language service is provided over the snapshot
The SDK SHALL provide an implementation of the protocol's `NxLanguageService` interface that
answers queries in the same process from language snapshots, optionally with a prelude prepended to
the queried document. Its answers SHALL match those of the HTTP handler configured with the same
prelude, including positions shifted into the document's own coordinates and prelude-internal
diagnostics reported without a range, and identical document sets SHALL share one analysis.

#### Scenario: Hover through a prelude
- **WHEN** the service is constructed with a prelude declaring a component
- **AND** a hover query lands on a tag naming it in the queried document
- **THEN** the answer SHALL carry that component's signature
- **AND** its range SHALL be expressed in the queried document's own lines and columns

#### Scenario: Prelude-internal diagnostics are not blamed on the document
- **WHEN** a diagnostics query is answered and a diagnostic's range lies inside the prelude
- **THEN** it SHALL be reported with a prelude origin and no range

#### Scenario: Repeated queries over unchanged text analyze once
- **WHEN** several queries arrive for the same document set
- **THEN** the documents SHALL be analyzed once
- **AND** each answer SHALL carry the version the request that asked for it supplied

#### Scenario: Cancellation is honored
- **WHEN** a caller cancels the signal it passed before the answer is produced
- **THEN** the returned promise SHALL reject with an abort error

### Requirement: The wasm SDK carries parity tests
The package SHALL carry tests that run the module under Node and compare its NX IR and language
answers with the Node SDK's over the same inputs, so a divergence between the two bindings fails the
build rather than reaching a site.

#### Scenario: Parity is checked in the workspace test run
- **WHEN** the workspace's tests run
- **THEN** the wasm SDK's parity tests SHALL run against the module built in that same run
- **AND** a difference in IR text, metadata, or any language answer SHALL fail them

### Requirement: The wasm SDK is published to npm with its module inside
The `@nx-lang/sdk-wasm` package SHALL be publishable and published to the npm registry, and the
published package SHALL contain the built WebAssembly module so that a consumer installs the
compiler with the package and needs no Rust toolchain, checkout or separate download.

#### Scenario: A consumer installs the compiler from the registry
- **WHEN** a JavaScript project installs the published `@nx-lang/sdk-wasm` package
- **THEN** it SHALL be able to import the browser entry, resolve the module's URL through the
  package's `nx.wasm` export, and build a program artifact without any file from an NX checkout

#### Scenario: The published package is what the workspace tests
- **WHEN** the package is packed for publication
- **THEN** the module inside it SHALL be the one the workspace's parity tests ran against for the
  same version

### Requirement: A prelude-aware build classifies diagnostics by origin
The SDK SHALL offer a build that takes a prelude and a document, compiles them as one program, emits
NX IR, and reports every diagnostic with an origin of `source`, `catalog` or `program`: a `source`
diagnostic carries a span shifted into the document's own coordinates, a `catalog` diagnostic points
into the prelude and carries no document position, and a `program` diagnostic has no position. The
build SHALL never blame the document for a fault in the prelude.

#### Scenario: A document error is reported in document coordinates
- **WHEN** the document has an error on its third line and the prelude is six hundred lines long
- **THEN** the diagnostic SHALL have origin `source` and a span whose start line is 3

#### Scenario: A prelude error is not a document error
- **WHEN** the prelude itself does not compile
- **THEN** every resulting diagnostic SHALL have origin `catalog` and no document span, and the
  result SHALL carry no IR

#### Scenario: A clean build yields IR
- **WHEN** the prelude and the document compile
- **THEN** the result SHALL carry the generated NX IR and an empty diagnostic list

#### Scenario: The playground and other hosts share the helper
- **WHEN** the playground compiles an example
- **THEN** its diagnostics and IR SHALL be the SDK helper's, with no second implementation of the
  origin classification in the playground

### Requirement: Emitted NX IR is compact JSON
NX IR produced through the wasm SDK and the FFI SHALL be serialized without indentation or
line breaks between tokens. Its content SHALL be unchanged: parsing compact and pretty-printed
output of the same program SHALL yield equal values.

#### Scenario: The SDK emits compact IR
- **WHEN** a program artifact's NX IR is generated through the wasm SDK
- **THEN** the JSON text SHALL contain no newline characters, and the IR runtime SHALL prepare it
  as before

#### Scenario: The CLI's files stay readable
- **WHEN** the CLI writes an NX IR file
- **THEN** the file SHALL remain pretty-printed
