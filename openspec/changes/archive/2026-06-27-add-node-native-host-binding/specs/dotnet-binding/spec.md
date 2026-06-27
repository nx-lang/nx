## RENAMED Requirements

FROM: `Published NuGet consumption is the primary .NET runtime workflow`
TO: `Published NuGet consumption is the primary .NET SDK workflow`

FROM: `Runtime package assets are built from one NX version`
TO: `SDK package assets are built from one NX version`

## ADDED Requirements

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

## MODIFIED Requirements

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
