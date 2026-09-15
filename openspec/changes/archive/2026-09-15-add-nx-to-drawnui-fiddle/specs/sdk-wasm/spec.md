## ADDED Requirements

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
