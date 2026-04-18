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
managed exceptions that explain the likely cause and recovery path.

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
- **AND** it provides guidance consistent with the documented source-based integration workflow

### Requirement: Native ABI contract is derived from the Rust FFI definition
The C-facing header for the NX runtime SHALL be generated from the Rust FFI contract so that
exported declarations do not require parallel manual maintenance.

#### Scenario: Header matches exported ABI
- **WHEN** the native FFI contract changes
- **THEN** the generated C header can be refreshed from the Rust source of truth

### Requirement: Vendored source consumption is a supported workflow
NX SHALL support consumption from repositories that vendor NX as a git submodule or subtree. The
documented workflow SHALL describe how to build the native runtime and managed binding locally and
reference them from a consuming .NET application without requiring publication to NuGet.

#### Scenario: Consumer uses a project reference
- **WHEN** a repository vendors NX source and references the managed binding project directly
- **THEN** the documentation describes how to build NX and resolve the native library for local
  execution

#### Scenario: Consumer uses built outputs
- **WHEN** a repository vendors NX source and references the built managed assembly
- **THEN** the documentation describes which managed and native artifacts must be copied or
  referenced together

#### Scenario: Published packaging is not required
- **WHEN** a consumer follows the supported integration path for this phase
- **THEN** the workflow does not require publishing or consuming a NuGet package

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

### Requirement: Managed raw-value and typed-model enum workflows remain distinct
The managed NX binding SHALL preserve canonical raw `NxValue` enum payloads for raw runtime result
workflows, while schema-aware typed model workflows SHALL use strong managed enums encoded as plain
member strings. The binding SHALL document and test that raw JSON/MessagePack payloads and typed
DTO serialization are intentionally different layers rather than interchangeable enum contracts.

#### Scenario: Managed JSON raw-value workflow preserves canonical enum identity
- **WHEN** a C# caller evaluates NX source to `JsonElement` and the result is an enum value such as
  `ThemeMode.dark`
- **THEN** the returned JSON SHALL expose the canonical raw enum object with
  `"$enum": "ThemeMode"` and `"$member": "dark"`
- **AND** the binding SHALL NOT collapse that raw JSON result to the bare string `"dark"`

#### Scenario: Managed typed MessagePack workflow uses strong enums as plain member strings
- **WHEN** a C# caller serializes or deserializes a generated typed DTO that contains
  `ThemeMode.Dark`
- **THEN** the managed typed workflow SHALL use the plain member string `dark` for MessagePack and
  JSON
- **AND** the typed DTO workflow SHALL NOT require the canonical raw enum object shape
