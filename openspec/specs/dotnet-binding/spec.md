# dotnet-binding Specification

## Purpose

Define the supported repository layout, API contract, and source-based consumption workflow for the
NX .NET binding.
## Requirements
### Requirement: .NET binding layout and support posture
The repository SHALL expose the managed NX binding under `bindings/dotnet` and SHALL treat it as a
`.NET 10`-only integration. The binding SHALL remain implemented and documented primarily in C#,
and the documentation SHALL state that other .NET languages are expected to work but are not yet
validated beyond C# tests and examples.

#### Scenario: Repository layout is renamed
- **WHEN** a contributor inspects the managed binding in the repository
- **THEN** the binding source, tests, and related documentation live under `bindings/dotnet`

#### Scenario: Support posture is documented
- **WHEN** a consumer reads the .NET binding documentation
- **THEN** the documentation identifies the binding as C#-first
- **AND** it states that other .NET languages are expected to work
- **AND** it states that current validation and examples are limited to C#

#### Scenario: Target framework remains fixed
- **WHEN** a contributor inspects the managed projects for the binding
- **THEN** the binding projects target `.NET 10` only

### Requirement: Managed SDK uses NxLang.Sdk package and project identity
The .NET binding SHALL use `NxLang.Sdk` as its public package, project, and assembly identity. The
previous `NxLang.Runtime` identity SHALL NOT be preserved through a compatibility package, alias
project, or forwarding assembly as part of this change.

#### Scenario: Managed project is renamed to SDK
- **WHEN** a contributor inspects the managed binding source
- **THEN** the primary managed project SHALL be named `NxLang.Sdk`
- **AND** the default assembly name and NuGet package id SHALL be `NxLang.Sdk`

#### Scenario: Previous package identity is not retained
- **WHEN** a consumer or contributor searches the managed package metadata produced by this change
- **THEN** there SHALL NOT be a published or buildable compatibility package named
  `NxLang.Runtime`
- **AND** consumers SHALL reference `NxLang.Sdk` for the managed NX SDK

#### Scenario: Managed tests and documentation use SDK naming
- **WHEN** contributors inspect managed binding tests, README files, package docs, and examples
- **THEN** package/project references SHALL use `NxLang.Sdk`
- **AND** user-facing prose SHALL describe the package as the .NET SDK for NX host, compiler,
  diagnostics, artifact, and evaluation workflows rather than only as a runtime package

### Requirement: Public managed API is CLS-compliant
The managed NX binding SHALL declare explicit CLS compliance and SHALL expose a CLS-compliant
public API surface. Public interop models SHALL avoid non-CLS primitive types, and public domain
concepts that are stable in the native contract SHALL be represented with strong managed types
instead of stringly typed values.

#### Scenario: Assembly declares CLS compliance
- **WHEN** a consumer inspects the managed assembly metadata or source
- **THEN** the assembly declares CLS compliance explicitly

#### Scenario: Public diagnostics use CLS-compliant types
- **WHEN** a consumer uses public diagnostic and span types from the managed API
- **THEN** the public members use CLS-compliant primitive types
- **AND** internal pointer-sized interop details remain hidden from the public API

#### Scenario: Severity is strongly typed
- **WHEN** evaluation fails and diagnostics are returned through the managed API
- **THEN** severity is exposed through a managed enum or equivalent strong type rather than a
  free-form string

### Requirement: Managed code validates native ABI compatibility
The managed NX binding SHALL validate compatibility with the native NX FFI library before relying
on runtime calls. Native load failures and compatibility mismatches SHALL produce actionable
managed exceptions that explain the likely cause and recovery path for both published package
consumption and source-based integration workflows.

#### Scenario: ABI versions match
- **WHEN** the managed binding loads a compatible native library
- **THEN** the runtime proceeds with evaluation calls normally

#### Scenario: ABI version mismatch is detected
- **WHEN** the managed binding loads a native library with an incompatible ABI version
- **THEN** the runtime fails before evaluation
- **AND** it raises a managed exception that identifies the incompatibility

#### Scenario: Native library load fails
- **WHEN** the native library cannot be found or loaded
- **THEN** the managed exception explains that the native NX runtime is missing or incompatible
- **AND** it provides guidance consistent with the documented package workflow and the documented
  source-based integration workflow

### Requirement: Native ABI contract is derived from the Rust FFI definition
The C-facing header for the NX runtime SHALL be generated from the Rust FFI contract so that
exported declarations do not require parallel manual maintenance.

#### Scenario: Header matches exported ABI
- **WHEN** the native FFI contract changes
- **THEN** the generated C header can be refreshed from the Rust source of truth

### Requirement: Vendored source consumption is a supported workflow
NX SHALL continue to support consumption from repositories that vendor NX as a git submodule or
subtree when a consumer intentionally wants to build from source. The documented workflow SHALL
describe how to build the native SDK library and managed SDK locally and reference them from a
consuming .NET application, but SHALL identify this source-based workflow as an advanced or
contributor-oriented alternative to the primary published NuGet package workflow.

#### Scenario: Consumer uses a project reference
- **WHEN** a repository vendors NX source and references the managed SDK project directly
- **THEN** the documentation describes how to build NX and resolve the native library for local
  execution
- **AND** the documentation identifies this as a source-based workflow rather than the primary
  application consumption workflow

#### Scenario: Consumer uses built outputs
- **WHEN** a repository vendors NX source and references the built managed assembly
- **THEN** the documentation describes which managed and native artifacts must be copied or
  referenced together
- **AND** the documentation identifies this as an advanced workflow for consumers that cannot use
  `PackageReference`

#### Scenario: Published packaging is the primary application workflow
- **WHEN** a consumer follows the supported integration path for a normal .NET application
- **THEN** the documentation SHALL direct the consumer to use the published `NxLang.Sdk` NuGet
  package
- **AND** the workflow SHALL NOT require vendoring NX source or publishing a consumer-local package

### Requirement: Managed API exposes reusable library registries and program build contexts
The managed NX binding SHALL expose a disposable `NxLibraryRegistry` API that lets callers load
local NX library roots before any program exists, and a disposable `NxProgramBuildContext` API
created from that registry for building transient `NxProgramArtifact`s from source.

#### Scenario: Managed host preloads a shared library before building any program
- **WHEN** a .NET host creates an `NxLibraryRegistry`
- **AND** loads `../question-flow` into it
- **THEN** the managed API SHALL retain that analyzed library snapshot without requiring an
  `NxProgramArtifact` to exist yet

#### Scenario: Managed caller reuses one loaded library across multiple program builds
- **WHEN** a .NET host creates one `NxLibraryRegistry`
- **AND** loads `../question-flow` into it
- **AND** creates build contexts from that registry
- **AND** builds two `NxProgramArtifact`s from different source strings that each import
  `../question-flow`
- **THEN** both program builds SHALL succeed using that same managed registry-backed workflow

#### Scenario: Managed program build reports missing library from context
- **WHEN** a .NET host builds source that imports `../question-flow` against a build context that
  has not loaded that library
- **THEN** the managed API SHALL surface a build exception describing the missing library load

### Requirement: Managed reusable-library workflow is registry-based
The managed binding SHALL provide reusable library caching through `NxLibraryRegistry` and use
`NxProgramBuildContext` as the build-time selection scope rather than exposing a standalone
reusable library-artifact host API.

#### Scenario: Managed host loads a library for later program builds
- **WHEN** a .NET host wants to cache `../question-flow` for repeated use
- **THEN** the supported managed workflow SHALL be to load that library into an
  `NxLibraryRegistry`
- **AND** later source-based program builds SHALL consume an `NxProgramBuildContext` created from
  that registry

### Requirement: Managed source component convenience is implemented via transient program artifacts
The managed binding SHALL keep source-based component convenience APIs, but implement them by
building transient `NxProgramArtifact`s and then calling the native program-artifact component
entry points rather than depending on separate source-based component FFI entry points.

#### Scenario: Managed source component initialization uses a build context through a transient artifact
- **WHEN** a .NET host calls a source-based component initialization helper with a
  `NxProgramBuildContext`
- **THEN** the managed binding SHALL build a transient `NxProgramArtifact` with that context
- **AND** SHALL initialize the component through the native program-artifact component API

#### Scenario: Managed source component dispatch uses the same source revision through a transient artifact
- **WHEN** a .NET host calls a source-based component dispatch helper
- **THEN** the managed binding SHALL build a transient `NxProgramArtifact` for that source revision
- **AND** SHALL dispatch through the native program-artifact dispatch API

### Requirement: Managed runtime APIs expose direct JSON result workflows
The managed NX binding SHALL allow C# callers to request JSON output directly from evaluation and
component lifecycle calls. The managed JSON workflow SHALL support both pass-through raw bytes and
parsed `JsonElement` results without requiring a post-processing MessagePack-to-JSON conversion
step.

#### Scenario: C# caller requests raw JSON bytes for pass-through
- **WHEN** a C# caller evaluates NX source through the managed raw-byte API and requests JSON output
- **THEN** the binding SHALL request JSON from the native runtime for that call
- **AND** SHALL return UTF-8 JSON bytes suitable for forwarding to a client

#### Scenario: C# evaluation reads JSON as JsonElement
- **WHEN** a C# caller evaluates `let root() = { { answer: 42 } }` through the managed JSON
  workflow
- **THEN** the binding SHALL return the result as `JsonElement`
- **AND** the caller SHALL be able to read property `answer` as `42`

#### Scenario: C# component lifecycle reads JSON results with JsonElement payloads
- **WHEN** a C# caller initializes or dispatches a component through the managed JSON workflow
- **THEN** initialization SHALL return `NxComponentInitResult<JsonElement>`
- **AND** dispatch SHALL return `NxComponentDispatchResult<JsonElement>`
- **AND** the opaque `StateSnapshot` bytes SHALL remain available for later dispatch calls

### Requirement: Managed JSON support replaces debug conversion helpers
The managed NX binding SHALL expose JSON by requesting it from the runtime call itself rather than
by converting previously returned MessagePack bytes through public helper APIs.

#### Scenario: Debug conversion helpers are not part of the managed JSON workflow
- **WHEN** a consumer inspects the managed runtime API surface for JSON result support
- **THEN** the supported JSON path SHALL be direct JSON-returning runtime calls and raw-byte format
  selection
- **AND** `NxRuntime` SHALL NOT require public `ValueBytesToJson`,
  `DiagnosticsBytesToJson`, `ComponentInitResultBytesToJson`, or
  `ComponentDispatchResultBytesToJson` methods

### Requirement: Managed raw-value and typed-model enum workflows share a single bare-string wire shape
The managed NX binding SHALL represent enum values as the bare authored NX member string across
both raw `NxValue` runtime-result workflows and schema-aware typed-model workflows. JSON and
MessagePack output from raw runtime calls, typed DTO serialization, and the shared
`NxEnumJsonConverter` / `NxEnumMessagePackFormatter` helpers SHALL produce and consume the same
string representation for a given enum member. The binding SHALL document and test that the raw
and typed layers share this wire shape rather than presenting it as two distinct enum contracts.

#### Scenario: Managed JSON raw-value workflow emits a bare authored member string
- **WHEN** a C# caller evaluates NX source to `JsonElement` and the result is an enum value such as
  `ThemeMode.dark`
- **THEN** the returned JSON SHALL be the bare string `"dark"` in the slot typed as `ThemeMode`
- **AND** the binding SHALL NOT wrap that raw JSON result in a `"$enum"` / `"$member"` object

#### Scenario: Managed typed MessagePack workflow matches the raw-value wire shape
- **WHEN** a C# caller serializes or deserializes a generated typed DTO that contains
  `ThemeMode.Dark`
- **THEN** the managed typed workflow SHALL use the plain member string `"dark"` for MessagePack
  and JSON
- **AND** the typed DTO wire output SHALL be bit-equivalent to the raw-value wire output for the
  same enum member at the same slot

#### Scenario: Managed consumer of a raw enum string resolves it through the target type
- **WHEN** a C# caller receives a raw JSON or MessagePack result that contains the bare string
  `"dark"` at a slot whose target typed DTO property is `ThemeMode`
- **THEN** the binding SHALL map that string to `ThemeMode.Dark` through the shared
  `NxEnumJsonConverter<ThemeMode, ThemeModeWireFormat>` / `NxEnumMessagePackFormatter<...>` helpers
- **AND** SHALL reject unknown member strings with the helpers' existing
  `JsonException` / `MessagePackSerializationException` error path

### Requirement: Managed raw-value and typed-model polymorphic record workflows share a single `$type` wire shape
The managed NX binding SHALL represent polymorphic NX records with the same `$type`-discriminated
map contract across both raw `NxValue` runtime-result workflows and schema-aware typed-model
MessagePack workflows. Generated typed DTO serialization and deserialization for polymorphic NX
record families SHALL align with the canonical raw runtime shape rather than a separate
MessagePack-specific union envelope.

#### Scenario: Typed MessagePack polymorphic record serialization matches raw runtime shape
- **WHEN** a C# caller serializes a generated typed DTO value for `SearchRequested` through
  MessagePack
- **THEN** the payload SHALL encode the record as a map containing `$type: "SearchRequested"` and
  the declared record fields
- **AND** the payload SHALL NOT use a MessagePack `Union` discriminator envelope

#### Scenario: Typed MessagePack polymorphic record deserialization accepts canonical `$type` map values
- **WHEN** a C# caller deserializes MessagePack bytes produced from canonical raw runtime output for
  a polymorphic record family
- **THEN** the managed typed workflow SHALL resolve the concrete CLR type from the `$type` field
- **AND** SHALL populate declared fields using their authored NX wire names

#### Scenario: Raw-to-typed round-trip preserves polymorphic record identity
- **WHEN** a C# caller receives a polymorphic record from raw runtime output and then maps it
  through a typed DTO MessagePack workflow
- **THEN** the resulting value SHALL preserve the same concrete record identity indicated by `$type`
- **AND** the typed and raw workflows SHALL remain wire-compatible for that value

### Requirement: Managed binding exposes reusable enum serialization helpers for typed DTOs
`NxLang.Sdk` SHALL expose public generic enum serialization helpers in
`NxLang.Nx.Serialization` so generated C# enums and hand-written managed enums can share the same
typed DTO serialization path for `System.Text.Json` and MessagePack. The helper contract SHALL use
an explicit wire-format mapping type rather than inferring authored NX member strings from CLR enum
member names.

#### Scenario: SDK exposes shared helper types for generated enums
- **WHEN** a C# caller or generated file references shared NX enum serialization infrastructure
- **THEN** `NxLang.Sdk` SHALL expose public `INxEnumWireFormat<TEnum>`
- **AND** SHALL expose public `NxEnumJsonConverter<TEnum, TWire>`
- **AND** SHALL expose public `NxEnumMessagePackFormatter<TEnum, TWire>`
- **AND** the helper contract SHALL allow the caller-provided wire-format type to map
  `DealStage.PendingReview` to `"pending_review"` explicitly

#### Scenario: Managed binding enum uses the shared helper path
- **WHEN** a caller serializes or deserializes `NxSeverity.Warning` through the managed typed DTO
  workflow
- **THEN** the managed binding SHALL use the shared enum helper infrastructure to emit and parse the
  plain member string `"warning"`
- **AND** SHALL NOT require dedicated `NxSeverityJsonConverter` or
  `NxSeverityMessagePackFormatter` support types

### Requirement: Managed binding exposes byte-oriented workspace APIs
The managed NX binding SHALL expose `NxWorkspace` and `NxWorkspaceModule` public APIs for
workspace-backed validation and program artifact construction. `NxWorkspaceModule` SHALL store the
logical identity as a string and source content as `ReadOnlyMemory<byte>`.

#### Scenario: C# caller constructs workspace from UTF-8 bytes
- **WHEN** a C# caller creates `NxWorkspaceModule("chat-link-config.nx", sourceBytes)`
- **AND** creates an `NxWorkspace` containing that module
- **THEN** the managed binding SHALL preserve the identity string and source byte payload for the
  native workspace call

#### Scenario: Managed API offers string convenience without replacing byte model
- **WHEN** a C# caller has source text as a string
- **THEN** the managed binding MAY offer a convenience factory or overload that encodes the source
  text as UTF-8
- **AND** the primary workspace module representation SHALL remain byte-oriented

### Requirement: Managed binding validates workspace arguments before FFI
The managed binding SHALL validate null workspaces, null modules, null build contexts, and empty
module or entry identities before invoking native workspace FFI.

#### Scenario: Empty module identity is rejected in managed code
- **WHEN** a C# caller creates or submits a workspace module with an empty identity
- **THEN** the managed binding SHALL throw a managed argument exception before invoking native code

#### Scenario: Null build context is rejected in managed code
- **WHEN** a C# caller invokes workspace validation with a null `NxProgramBuildContext`
- **THEN** the managed binding SHALL throw a managed argument exception before invoking native code

### Requirement: Managed workspace validation returns diagnostics
The managed binding SHALL expose a workspace validation API that returns an
`IReadOnlyList<NxDiagnostic>` translated through the existing managed diagnostic model.

#### Scenario: Valid managed workspace returns empty diagnostics list
- **WHEN** a C# caller validates a valid `NxWorkspace`
- **THEN** the managed validation API SHALL return an empty diagnostics list

#### Scenario: Invalid managed workspace returns structured diagnostics
- **WHEN** a C# caller validates a workspace containing type-invalid NX source
- **THEN** the managed validation API SHALL return `NxDiagnostic` values whose label files preserve
  the normalized workspace identities reported by native NX

### Requirement: Managed workspace builds pin buffers only for the native call
The managed binding SHALL pin workspace module descriptors, identity bytes, source bytes, and entry
identity bytes only for the duration of the native workspace call. A returned
`NxProgramArtifact` SHALL remain valid after those managed buffers are unpinned or collected.

#### Scenario: Workspace artifact remains executable after build buffers are released
- **WHEN** a C# caller builds an `NxProgramArtifact` from an `NxWorkspace`
- **AND** the managed workspace source buffers are no longer pinned after the build call returns
- **THEN** evaluating the returned artifact SHALL still succeed

### Requirement: Managed binding supports discriminated union raw and typed workflows
The managed NX binding SHALL preserve discriminated union values through raw runtime-result
workflows and schema-aware generated typed DTO workflows using the canonical `$type` map shape.
JSON and MessagePack output from raw runtime calls, typed DTO serialization, and typed DTO
deserialization SHALL agree on the fully scoped case discriminator string and authored field wire
names.

#### Scenario: Managed raw JSON returns union case discriminator
- **WHEN** a C# caller evaluates NX source that returns `<LoadState.failed message={"Offline"} />`
- **AND** requests JSON output through the managed runtime API
- **THEN** the returned `JsonElement` SHALL contain `$type` with value `LoadState.failed`
- **AND** it SHALL contain field `message` with value `"Offline"`

#### Scenario: Managed typed MessagePack deserializes union case from `$type`
- **WHEN** a C# caller deserializes MessagePack bytes containing a map with `$type:
  "LoadState.failed"` and field `message`
- **AND** the target generated DTO type is the generated `LoadState` root
- **THEN** the managed typed workflow SHALL instantiate the generated `LoadState.failed` case DTO
- **AND** it SHALL populate the `message` property from the authored wire field

#### Scenario: Managed typed serialization matches raw union output
- **WHEN** a C# caller serializes a generated typed DTO value for the `LoadState.failed` case
  through JSON or MessagePack
- **THEN** the payload SHALL encode the value as a map containing `$type: "LoadState.failed"`
- **AND** the payload SHALL include the declared case fields using their authored NX wire names
- **AND** the payload SHALL NOT use a MessagePack union envelope

#### Scenario: Managed enum workflow remains separate
- **WHEN** a C# caller receives raw output for `CardSortMode.closed` and raw output for
  `LoadState.idle`
- **THEN** the managed enum workflow SHALL expose the enum value as the bare string `"closed"`
- **AND** the managed union workflow SHALL expose the union case as a map containing `$type:
  "LoadState.idle"`

### Requirement: Managed binding evaluates components with explicit state
The managed NX binding SHALL expose `EvaluateComponent` APIs that evaluate a named component with
caller-provided props and caller-provided current state, then return the rendered component body.
These APIs SHALL NOT expose or require `StateSnapshot`, SHALL NOT dispatch actions, and SHALL NOT
return effect actions.

#### Scenario: Managed typed component evaluation returns rendered element
- **WHEN** a C# caller evaluates `SearchBox` from source containing `component <SearchBox placeholder:string = "Find docs" /> = { state { query:string } <TextInput value={query} placeholder={placeholder} /> }` with state `{ query = "docs" }`
- **THEN** `NxRuntime.EvaluateComponent<SearchBoxProps, SearchBoxState, TextInputElement>` SHALL
  return a `TextInputElement` whose `Value` is `"docs"`
- **AND** the returned `TextInputElement.Placeholder` SHALL be `"Find docs"`

#### Scenario: Managed component evaluation does not return lifecycle fields
- **WHEN** a C# caller evaluates a component through `EvaluateComponent`
- **THEN** the managed result SHALL be the rendered element type requested by the caller
- **AND** the API SHALL NOT require a prior `StateSnapshot`
- **AND** the API SHALL NOT return a `StateSnapshot`
- **AND** the API SHALL NOT return an effect action collection

#### Scenario: Managed component evaluation rejects invalid state input
- **WHEN** a C# caller evaluates a component whose state declaration requires `query:string`
- **AND** the supplied state payload omits `query`
- **THEN** the managed binding SHALL throw `NxEvaluationException` with diagnostics from the native
  runtime instead of silently rendering with missing state

### Requirement: Managed component evaluation supports source and artifact workflows
The managed binding SHALL expose component evaluation overloads for both `NxProgramArtifact` and
source strings. Source-string overloads SHALL build a transient `NxProgramArtifact` using either the
default build context or a caller-provided `NxProgramBuildContext`, then invoke the artifact-first
native evaluation entry point.

#### Scenario: Managed program-artifact evaluation reuses imported libraries
- **WHEN** a C# caller builds a `NxProgramArtifact` from source that imports a library through a
  preconfigured `NxProgramBuildContext`
- **AND** the caller evaluates a component from that artifact
- **THEN** the managed binding SHALL evaluate the component using the already-built artifact and its
  selected library snapshots

#### Scenario: Managed source evaluation uses caller build context
- **WHEN** a C# caller evaluates a component from source using an explicit `NxProgramBuildContext`
- **THEN** the managed binding SHALL build a transient `NxProgramArtifact` with that context
- **AND** SHALL evaluate the component through the native program-artifact component evaluation API

#### Scenario: Managed source evaluation reports static diagnostics
- **WHEN** a C# caller evaluates a component from source that contains static diagnostics
- **THEN** the managed binding SHALL throw `NxEvaluationException` containing those diagnostics
- **AND** SHALL NOT invoke component body evaluation

### Requirement: Managed component evaluation exposes raw JSON and typed workflows
The managed binding SHALL support component evaluation results as typed MessagePack DTOs, raw bytes
with caller-selected output format, and parsed `JsonElement` values. JSON and MessagePack outputs
SHALL represent the rendered component body directly rather than wrapping it in a lifecycle result
object.

#### Scenario: Managed JSON component evaluation returns rendered JsonElement
- **WHEN** a C# caller evaluates `SearchBox` and requests JSON output
- **THEN** `NxRuntime.EvaluateComponentJson` SHALL return a `JsonElement` representing the rendered
  component body
- **AND** the JSON value SHALL NOT contain `rendered`, `state_snapshot`, or `effects` wrapper fields

#### Scenario: Managed raw-byte component evaluation returns selected format
- **WHEN** a C# caller evaluates `SearchBox` through a raw-byte component evaluation API and selects
  JSON output
- **THEN** the binding SHALL return UTF-8 JSON bytes for the rendered component body
- **AND** selecting MessagePack output SHALL return MessagePack bytes for the same rendered value

#### Scenario: Managed typed component evaluation preserves generated DTO wire rules
- **WHEN** a C# caller evaluates a component whose rendered body contains polymorphic records or enum
  values
- **THEN** typed DTO deserialization SHALL use the existing `$type` record and bare-string enum
  contracts shared by root evaluation and component initialization

### Requirement: Managed binding emits NX IR from program artifacts
The managed .NET binding SHALL expose artifact-first APIs that emit NX IR JSON and structured
metadata from an `NxProgramArtifact`. The managed API SHALL reuse the native artifact-first IR
emission path, SHALL preserve the program fingerprint and runtime ABI metadata, and SHALL surface
IR emission diagnostics through the existing managed diagnostic exception path.

#### Scenario: Managed artifact emits IR
- **WHEN** a .NET caller builds a valid `NxProgramArtifact`
- **AND** the caller requests NX IR output from that artifact
- **THEN** the managed API SHALL return NX IR JSON
- **AND** it SHALL return metadata identifying the program fingerprint, IR schema version, runtime
  ABI, and exported function/component entrypoints

#### Scenario: Managed artifact IR emission surfaces diagnostics
- **WHEN** a .NET caller requests NX IR output for an artifact that cannot be represented by the
  supported IR feature set
- **THEN** the managed API SHALL throw an `NxEvaluationException`
- **AND** the exception SHALL contain the native IR emission diagnostics

#### Scenario: Managed source convenience uses transient artifact
- **WHEN** a .NET caller requests NX IR output from source text using a build context convenience
  API
- **THEN** the managed binding SHALL build a transient `NxProgramArtifact` with that context
- **AND** it SHALL emit IR through the same artifact-first path as direct artifact calls

### Requirement: Published NuGet consumption is the primary .NET SDK workflow
The managed NX SDK SHALL support application consumption through a published `NxLang.Sdk` NuGet
package. The package SHALL include the managed SDK assembly and native `nx_ffi` SDK assets for
every runtime identifier advertised as supported by that package version. Consumers using the
package SHALL NOT need an NX source checkout, direct `ProjectReference`, imported NX repository
targets, or a consumer-side Rust toolchain to build, test, run, or publish a .NET application that
uses the SDK. The package SHALL NOT include the `nxlang` CLI executable.

#### Scenario: Consumer references SDK package
- **WHEN** a .NET application adds a `PackageReference` to `NxLang.Sdk`
- **THEN** restore SHALL provide the managed NX SDK assembly
- **AND** restore SHALL provide the native NX SDK asset for the application's supported runtime
  identifier
- **AND** the application project SHALL NOT need to reference files under `bindings/dotnet` from an
  NX source checkout

#### Scenario: Consumer runs without building native SDK library
- **WHEN** a package-consuming .NET application evaluates a trivial NX program through
  `NxLang.Sdk`
- **THEN** the application SHALL load the packaged native `nx_ffi` library
- **AND** the application SHALL NOT invoke Cargo or build `crates/nx-ffi` in the consumer repository

#### Scenario: Package publish output carries native SDK asset
- **WHEN** a package-consuming .NET executable is published for a runtime identifier supported by
  the `NxLang.Sdk` package
- **THEN** the publish output SHALL contain the native NX SDK library required by that runtime
  identifier
- **AND** the managed SDK SHALL be able to load that native library from the published application
  output

#### Scenario: Consumer-owned NX content remains outside SDK package
- **WHEN** a consumer uses application-specific `.nx` source libraries
- **THEN** `NxLang.Sdk` SHALL NOT require those files to be packaged by NX
- **AND** the consumer SHALL remain responsible for embedding, copying, or otherwise supplying that
  application-specific NX content

#### Scenario: SDK package excludes CLI tooling
- **WHEN** a consumer restores or inspects the `NxLang.Sdk` package
- **THEN** the package SHALL contain the managed SDK and native SDK assets needed by .NET
  applications
- **AND** the package SHALL NOT contain the `nxlang` CLI executable
- **AND** CLI packaging SHALL remain outside this SDK package workflow

### Requirement: SDK package assets are built from one NX version
`NxLang.Sdk` NuGet packages SHALL contain managed SDK and native SDK assets built from the same
NX source revision and version. The package build SHALL fail before publication when a required
supported native SDK asset is missing.

#### Scenario: Package contains expected native assets
- **WHEN** the release pipeline produces a `NxLang.Sdk` package
- **THEN** package verification SHALL confirm that the package contains one native `nx_ffi` library
  under `runtimes/<rid>/native/` for each runtime identifier advertised as supported by that package
  version

#### Scenario: Missing supported native SDK asset blocks package publication
- **WHEN** a native SDK asset for an advertised supported runtime identifier is not available
  during package assembly
- **THEN** the package build SHALL fail before a `NxLang.Sdk` package is published

#### Scenario: Managed and native assets are version-locked
- **WHEN** a `NxLang.Sdk` package is assembled
- **THEN** the managed assembly and all included native `nx_ffi` libraries SHALL come from the same
  NX source revision
- **AND** the existing managed native ABI validation SHALL remain in place as runtime defense
