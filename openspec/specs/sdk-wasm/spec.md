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
The SDK SHALL build a program artifact from a workspace of in-memory modules, each with a logical
identity, its source text and an optional version string, together with an entry identity and an
optional list of identities every other module imports implicitly. Building from a single source
text and a file name SHALL remain available as the one-module case. From a built artifact the SDK
SHALL emit NX IR for the modules the caller names, or for the entry alone by default, returning one
artifact per module as bytes with its metadata, and SHALL include the debug section only when
asked. Build failures SHALL be reported as structured diagnostics with the same shape and property
names the Node SDK reports, each against the identity of the module it belongs to.

#### Scenario: Valid source builds and emits IR
- **WHEN** a caller builds an artifact from a single source that compiles
- **AND** asks it for NX IR
- **THEN** the result SHALL carry one artifact's bytes and the same metadata the Node SDK reports
- **AND** the bytes SHALL be identical to what the Node SDK emits for the same source, file name
  and options

#### Scenario: The bytes are the caller's
- **WHEN** a caller receives an artifact's bytes and the SDK's result has been released
- **THEN** the bytes SHALL remain readable
- **AND** the IR runtime SHALL prepare them

#### Scenario: A snippet is emitted without the catalog it uses
- **WHEN** a caller builds a workspace of `drawnui` at version `9` and `input.nx`, with `drawnui`
  implicitly imported and `input.nx` as the entry
- **AND** asks for NX IR
- **THEN** the result SHALL carry one artifact, for `input.nx`, whose module table names `drawnui`
  with version `9`
- **AND** the artifact SHALL be free of `drawnui`'s declarations and of any debug section

#### Scenario: A catalog is emitted on its own
- **WHEN** a caller builds a workspace of `drawnui` alone with `drawnui` as the entry
- **AND** asks for NX IR
- **THEN** the result SHALL carry one artifact for `drawnui` whose module table holds only itself

#### Scenario: The SDK emits debug data on request
- **WHEN** a caller asks the SDK for NX IR with the debug section
- **THEN** the artifact SHALL carry spans and source text, and SHALL otherwise equal the artifact
  emitted without them

#### Scenario: Invalid source reports diagnostics
- **WHEN** a caller builds a workspace in which `input.nx` does not compile
- **THEN** the build SHALL throw an SDK evaluation error carrying every diagnostic
- **AND** each diagnostic SHALL carry its severity, code, message and labels with byte and
  line-column spans against `input.nx`, never shifted by another module's text

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

### Requirement: An in-process language service sees host documents through implicit imports
The SDK SHALL provide an implementation of the protocol's `NxLanguageService` interface that
answers queries in the same process from language snapshots. The service SHALL accept documents
the host always includes alongside the queried document set and a list of identities every queried
document imports implicitly, so a host's context declarations are visible to hover, completion and
diagnostics without appearing in the queried document's text. Its answers SHALL match those of a
snapshot built from the same documents, and identical document sets SHALL share one analysis.

#### Scenario: Hover through an implicit import
- **WHEN** the service is constructed with a `drawnui` document declaring a component and `drawnui`
  as an implicit import
- **AND** a hover query lands on a tag naming that component in the queried document
- **THEN** the answer SHALL carry that component's signature
- **AND** its range SHALL be expressed in the queried document's own lines and columns

#### Scenario: Completions through an implicit import
- **WHEN** a completion query lands inside an opening tag of an implicitly imported component
- **THEN** the answer SHALL offer that component's properties

#### Scenario: A fault in a host document is not the queried document's
- **WHEN** a diagnostics query is answered and a diagnostic belongs to a host-supplied document
- **THEN** it SHALL be reported against that document's URI, not the queried document's

#### Scenario: Repeated queries over unchanged text analyze once
- **WHEN** several queries arrive for the same document set
- **THEN** the documents SHALL be analyzed once
- **AND** each answer SHALL carry the version the request that asked for it supplied

#### Scenario: Cancellation is honored
- **WHEN** a caller cancels the signal it passed before the answer is produced
- **THEN** the returned promise SHALL reject with an abort error

### Requirement: The SDK explains an NX IR artifact
The SDK SHALL render an NX IR image it is given as the same readable text the CLI's `ir explain`
prints for that image, so an artifact can be read on the page that holds it. A malformed image
SHALL be reported as an SDK error carrying the diagnostic rather than as a trap.

#### Scenario: A page reads an artifact it holds
- **WHEN** a page holds the bytes of an artifact and asks the SDK to explain them
- **THEN** it SHALL receive the text the CLI would print for that artifact

#### Scenario: A damaged artifact is reported
- **WHEN** a page asks the SDK to explain bytes that are not a valid image
- **THEN** the SDK SHALL throw an error naming what is malformed
- **AND** the module SHALL remain usable

### Requirement: Program artifacts evaluate root to annotated NX text
A program artifact built by the wasm SDK SHALL evaluate its entry module's `root` and return an
`NxValueText`: the value as NX text, identical to what `nxlang run` prints for the same source, and
a list of nodes annotating that text. The one difference from `nxlang run` SHALL be how deep calls
may nest: at most 200, where the command line allows 1,000, because a browser runs the module on a
native stack the module cannot size. A program that nests deeper fails with the interpreter's
recursion-limit error. Each node SHALL give the UTF-16 range of one value, property or
sequence in the text, its enclosing node, its role, its type spelled in NX, and, when the entry
module declares it, the span of its declaration. An artifact that cannot be evaluated SHALL fail
with an `NxEvaluationError` carrying diagnostics, and the artifact SHALL remain usable afterwards.

#### Scenario: A record evaluates to its NX spelling
- **WHEN** a host builds `type User = { id:string name:string }` followed by
  `<User id="1" name="Ada" />` and calls `evaluateNx()` on the artifact
- **THEN** the result's `text` SHALL be `<User id="1" name="Ada" />`

#### Scenario: Nodes annotate the text
- **WHEN** a host evaluates that same source
- **THEN** the result SHALL hold a `record` node of type `User` covering the whole text, whose
  declaration is the span of `type User`
- **AND** it SHALL hold, as that node's children, `property` nodes named `id` and `name` of type
  `string`, each covering its `name="…"` text and declared at its property

#### Scenario: A sequence carries its count
- **WHEN** `root` returns a sequence of three records of type `User`
- **THEN** the result SHALL hold a `sequence` node of type `User*` covering the whole text with a
  count of 3, and one `record` node per item as its children

#### Scenario: A property's type is its declared type
- **WHEN** a record's type declares `subtitle?:string` and the value omits it or supplies it
- **THEN** any `property` node for `subtitle` SHALL have the type `string` and be marked optional

#### Scenario: Offsets count UTF-16 code units
- **WHEN** a string before a node holds a character outside the Basic Multilingual Plane
- **THEN** the node's offsets SHALL count that character as two units, so that they index the
  JavaScript string `text` directly

#### Scenario: No root
- **WHEN** a host calls `evaluateNx()` on an artifact whose entry module has no `root`
- **THEN** it SHALL throw an `NxEvaluationError` whose diagnostic says there is no `root`

#### Scenario: A runtime error carries its span
- **WHEN** evaluating `root` fails at run time
- **THEN** `evaluateNx()` SHALL throw an `NxEvaluationError` whose diagnostic carries the message and
  the span in the entry module

#### Scenario: Recursion past the module's limit but within the command line's
- **WHEN** `root` calls a function that recurses 250 deep and then returns
- **THEN** `evaluateNx()` SHALL throw an `NxEvaluationError` saying the recursion depth of 200 was
  exceeded, where `nxlang run` prints the value

#### Scenario: Runaway recursion is a diagnostic, not a trap
- **WHEN** `root` recurses without end through a plain function
- **THEN** `evaluateNx()` SHALL throw an `NxEvaluationError` for the interpreter's recursion limit
- **AND** the host SHALL remain usable

#### Scenario: A value with no NX spelling
- **WHEN** `root` evaluates to a value holding an action handler, or a sequence nested directly in a
  sequence
- **THEN** `evaluateNx()` SHALL throw an `NxEvaluationError` saying which part has no spelling
- **AND** SHALL NOT return partial text

#### Scenario: Parity with the command line
- **WHEN** the wasm SDK's parity tests run
- **THEN** `evaluateNx()` SHALL return, for every source in their corpus that has a `root`, a `text`
  equal to what `nxlang run` prints for it (none of them nests calls past the module's limit)

#### Scenario: The ABI version moves with the new export
- **WHEN** the wrapper for this version loads a module built before this export existed
- **THEN** loading SHALL fail with the version mismatch error, naming both versions

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

### Requirement: A built program artifact reports its warnings
A program artifact that builds SHALL report the diagnostics its build produced without failing —
warnings, info and hints — in the same shape a build failure's diagnostics take, each against the
identity of the module it belongs to. A program with none SHALL report an empty list.

#### Scenario: A warning from a build that succeeds is reported
- **WHEN** a caller builds an artifact from the source `/// See [Missing].` followed by
  `let root() = { 42 }`
- **THEN** the build SHALL succeed and the artifact SHALL evaluate to `42`
- **AND** the artifact's diagnostics SHALL be one `unresolved-doc-link` warning labeled at line 1

#### Scenario: A clean build reports no diagnostics
- **WHEN** a caller builds an artifact from `let root() = { 42 }`
- **THEN** the artifact's diagnostics SHALL be empty
