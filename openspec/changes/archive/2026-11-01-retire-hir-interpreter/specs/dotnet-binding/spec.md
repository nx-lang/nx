## ADDED Requirements

### Requirement: Managed compiler tooling does not execute source or artifacts
The managed SDK SHALL NOT expose source/artifact Evaluate, InitializeComponent, EvaluateComponent
or DispatchComponentActions method families or the legacy opaque-snapshot result wrappers. It
SHALL retain construction, validation, IR generation/explanation, generated DTOs and their
serialization/update helpers. It SHALL NOT add a replacement .NET IR execution binding in this change.
Native export removals SHALL update the generated header, native ABI and managed checks together.

#### Scenario: Managed callers compile source to IR
- **WHEN** a C# caller builds source or a workspace and requests its NX IR
- **THEN** it SHALL receive images for execution by a separately chosen IR runtime
- **AND** building and emission SHALL NOT execute root

#### Scenario: Serialization does not require source execution
- **WHEN** generated DTOs serialize or deserialize canonical NX values
- **THEN** enum strings, discriminators, function references and update helpers SHALL retain their contracts
- **AND** tests SHALL NOT require a removed managed evaluation method

## MODIFIED Requirements

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
  diagnostics, artifact and IR-generation workflows rather than only as a runtime package

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
- **WHEN** source validation fails and diagnostics are returned through the managed API
- **THEN** severity is exposed through a managed enum or equivalent strong type rather than a
  free-form string

### Requirement: Managed code validates native ABI compatibility
The managed NX binding SHALL validate compatibility with the native NX FFI library before relying
on runtime calls. Native load failures and compatibility mismatches SHALL produce actionable
managed exceptions that explain the likely cause and recovery path for both published package
consumption and source-based integration workflows.

#### Scenario: ABI versions match
- **WHEN** the managed binding loads a compatible native library
- **THEN** the runtime proceeds with tooling calls normally

#### Scenario: ABI version mismatch is detected
- **WHEN** the managed binding loads a native library with an incompatible ABI version
- **THEN** the runtime fails before tooling calls
- **AND** it raises a managed exception that identifies the incompatibility

#### Scenario: Native library load fails
- **WHEN** the native library cannot be found or loaded
- **THEN** the managed exception explains that the native NX runtime is missing or incompatible
- **AND** it provides guidance consistent with the documented package workflow and the documented
  source-based integration workflow

### Requirement: Vendored source consumption is a supported workflow
NX SHALL continue to support consumption from repositories that vendor NX as a git submodule or
subtree when a consumer intentionally wants to build from source. The documented workflow SHALL
describe how to build the native SDK library and managed SDK locally and reference them from a
consuming .NET application, but SHALL identify this source-based workflow as an advanced or
contributor-oriented alternative to the primary published NuGet package workflow.

#### Scenario: Consumer uses a project reference
- **WHEN** a repository vendors NX source and references the managed SDK project directly
- **THEN** the documentation describes how to build NX and resolve the native library for local
  tooling use
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
- **WHEN** a package-consuming .NET application compiles a trivial NX program to IR through
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

### Requirement: Managed raw-value and typed-model enum workflows share a single bare-string wire shape
The managed NX binding SHALL represent constant union case values as the bare authored NX case
string across both canonical raw-value workflows and schema-aware typed-model workflows.
Canonical JSON/MessagePack values, typed DTO serialization, and the shared
`NxEnumJsonConverter` / `NxEnumMessagePackFormatter` helpers SHALL produce and consume the same
string representation for a given case. The binding SHALL document and test that the raw and typed
layers share this wire shape rather than presenting it as two distinct contracts.

#### Scenario: Managed JSON raw-value workflow emits a bare authored member string
- **WHEN** a C# caller reads a canonical `JsonElement` containing the constant case `ThemeMode.dark`
- **THEN** the canonical JSON SHALL be the bare string `"dark"` in the slot typed as `ThemeMode`
- **AND** the binding SHALL NOT wrap that raw JSON result in a `"$enum"` / `"$member"` object

#### Scenario: Managed typed MessagePack workflow matches the raw-value wire shape
- **WHEN** a C# caller serializes or deserializes a generated typed DTO that contains
  `ThemeMode.Dark`
- **THEN** the managed typed workflow SHALL use the plain case string `"dark"` for MessagePack
  and JSON
- **AND** the typed DTO wire output SHALL be bit-equivalent to the raw-value wire output for the
  same case at the same slot

#### Scenario: Managed consumer of a raw enum string resolves it through the target type
- **WHEN** a C# caller receives a raw JSON or MessagePack result that contains the bare string
  `"dark"` at a slot whose target typed DTO property is `ThemeMode`
- **THEN** the binding SHALL map that string to `ThemeMode.Dark` through the shared
  `NxEnumJsonConverter<ThemeMode, ThemeModeWireFormat>` / `NxEnumMessagePackFormatter<...>` helpers
- **AND** SHALL reject unknown case strings with the helpers' existing
  `JsonException` / `MessagePackSerializationException` error path

### Requirement: Managed raw-value and typed-model polymorphic record workflows share a single `$type` wire shape
The managed NX binding SHALL represent polymorphic NX records with the same `$type`-discriminated
map contract across both canonical raw-value workflows and schema-aware typed-model
MessagePack workflows. Generated typed DTO serialization and deserialization for polymorphic NX
record families SHALL align with the canonical raw runtime shape rather than a separate
MessagePack-specific union envelope.

A polymorphic family that is a discriminated union MAY contain constant cases, whose wire form is a
bare string rather than a `$type` map. The managed polymorphic reader SHALL accept a bare string at
a slot whose target type is such a union and SHALL resolve it to that union's constant case, and the
managed polymorphic writer SHALL emit a bare string for that case. This SHALL NOT change the wire
shape of any payload case.

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

#### Scenario: Polymorphic reader accepts a bare string for a constant case
- **WHEN** a C# caller deserializes JSON or MessagePack containing the bare string `"idle"` at a
  slot whose target type is a union with cases `idle` and `failed { message:string }`
- **THEN** the managed typed workflow SHALL resolve the value to that union's `idle` case
- **AND** serializing that value again SHALL produce the bare string `"idle"`
- **AND** an unknown bare string SHALL be rejected through the existing error path

#### Scenario: Raw-to-typed round-trip preserves polymorphic record identity
- **WHEN** a C# caller receives a polymorphic record from raw runtime output and then maps it
  through a typed DTO MessagePack workflow
- **THEN** the resulting value SHALL preserve the same concrete record identity indicated by `$type`
- **AND** the typed and raw workflows SHALL remain wire-compatible for that value

### Requirement: Managed binding supports discriminated union raw and typed workflows
The managed NX binding SHALL preserve discriminated union values through canonical raw-value
workflows and schema-aware generated typed DTO workflows using the canonical `$type` map shape.
Canonical JSON/MessagePack values, typed DTO serialization, and typed DTO
deserialization SHALL agree on the fully scoped case discriminator string and authored field wire
names.

#### Scenario: Managed raw JSON returns union case discriminator
- **WHEN** a C# caller reads canonical JSON for `<LoadState.failed message={"Offline"} />`
- **THEN** the `JsonElement` SHALL contain `$type` with value `LoadState.failed`
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

#### Scenario: Constant and payload cases keep distinct managed shapes
- **WHEN** a C# caller receives raw output for `CardSortMode.closed`, a case of the constant union
  `CardSortMode`, and raw output for the payload case `LoadState.failed`
- **THEN** the managed workflow SHALL expose the constant case as the bare string `"closed"`
- **AND** it SHALL expose the payload case as a map containing `$type: "LoadState.failed"`

### Requirement: A function value is read through a managed function reference type
The managed NX binding SHALL provide a function reference type that a host types a function-valued
property of a rendered element with, exposing the declaring module's identity and the function's
declared name — the two fields of the rendered `Function` record. It SHALL serialize in both output
formats the binding supports, MessagePack and JSON, including when the property carries no value,
and generated C# SHALL type a function-typed member with it rather than with a host delegate, which
neither format can serialize. Calling a function value from .NET is out of scope: a host reads
which function it was handed and passes the record on.

#### Scenario: A rendered function-typed property is read as a function reference
- **WHEN** a C# caller deserializes a canonical List record whose ItemTemplate is a Function record naming module `templates.nx` and function `Row` into a typed element whose property uses the managed function reference type
- **THEN** the property SHALL expose the module `templates.nx` and the name `Row`
- **AND** serializing the typed value as JSON SHALL render `ItemTemplate` as a `Function` record with those
  two fields

#### Scenario: A contract with a function-typed member serializes in both formats
- **WHEN** a typed element with a function-typed property is serialized and deserialized through
  MessagePack and through JSON
- **THEN** both round trips SHALL preserve the module and the name
- **AND** both SHALL succeed when the property is `null` — the .NET reading of an optional property
  the element omits — because a member type that cannot be resolved would otherwise make the whole
  containing contract unserializable

#### Scenario: Generated C# types a function-typed member as the function reference
- **WHEN** C# types are generated for `export external component <DataTable RowTemplate?:<function Item:object Index:int />: string />`
- **THEN** the generated member SHALL be the managed function reference type
- **AND** SHALL NOT be a host delegate type

## REMOVED Requirements

### Requirement: Managed source component convenience is implemented via transient program artifacts
**Reason**: The legacy source/artifact execution binding is removed.
**Migration**: Compile source to IR and execute through an existing IR runtime. No replacement binding is introduced.

### Requirement: Managed runtime APIs expose direct JSON result workflows
**Reason**: The legacy source/artifact execution binding is removed.
**Migration**: Compile source to IR and execute through an existing IR runtime. No replacement binding is introduced.

### Requirement: Managed JSON support replaces debug conversion helpers
**Reason**: The legacy source/artifact execution binding is removed.
**Migration**: Compile source to IR and execute through an existing IR runtime. No replacement binding is introduced.

### Requirement: Managed binding evaluates components with explicit state
**Reason**: The legacy source/artifact execution binding is removed.
**Migration**: Compile source to IR and execute through an existing IR runtime. No replacement binding is introduced.

### Requirement: Managed component evaluation supports source and artifact workflows
**Reason**: The legacy source/artifact execution binding is removed.
**Migration**: Compile source to IR and execute through an existing IR runtime. No replacement binding is introduced.

### Requirement: Managed component evaluation exposes raw JSON and typed workflows
**Reason**: The legacy source/artifact execution binding is removed.
**Migration**: Compile source to IR and execute through an existing IR runtime. No replacement binding is introduced.

### Requirement: Managed dispatch accepts handler invocations and returns rendered output
**Reason**: The legacy source/artifact execution binding is removed.
**Migration**: Compile source to IR and execute through an existing IR runtime. No replacement binding is introduced.
