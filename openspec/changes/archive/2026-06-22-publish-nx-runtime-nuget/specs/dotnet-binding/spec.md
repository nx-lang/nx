## ADDED Requirements

### Requirement: Published NuGet consumption is the primary .NET runtime workflow
The managed NX binding SHALL support application consumption through a published
`NxLang.Runtime` NuGet package. The package SHALL include the managed binding assembly and native
`nx_ffi` runtime assets for every runtime identifier advertised as supported by that package
version. Consumers using the package SHALL NOT need an NX source checkout, direct
`ProjectReference`, imported NX repository targets, or a consumer-side Rust toolchain to build,
test, run, or publish a .NET application that uses the runtime. The package SHALL NOT include the
`nxlang` CLI executable.

#### Scenario: Consumer references runtime package
- **WHEN** a .NET application adds a `PackageReference` to `NxLang.Runtime`
- **THEN** restore SHALL provide the managed NX binding assembly
- **AND** restore SHALL provide the native NX runtime asset for the application's supported runtime
  identifier
- **AND** the application project SHALL NOT need to reference files under `bindings/dotnet` from an
  NX source checkout

#### Scenario: Consumer runs without building native runtime
- **WHEN** a package-consuming .NET application evaluates a trivial NX program through
  `NxLang.Runtime`
- **THEN** the application SHALL load the packaged native `nx_ffi` library
- **AND** the application SHALL NOT invoke Cargo or build `crates/nx-ffi` in the consumer repository

#### Scenario: Package publish output carries native runtime
- **WHEN** a package-consuming .NET executable is published for a runtime identifier supported by
  the `NxLang.Runtime` package
- **THEN** the publish output SHALL contain the native NX runtime library required by that runtime
  identifier
- **AND** the managed binding SHALL be able to load that native library from the published
  application output

#### Scenario: Consumer-owned NX content remains outside runtime package
- **WHEN** a consumer uses application-specific `.nx` source libraries
- **THEN** `NxLang.Runtime` SHALL NOT require those files to be packaged by NX
- **AND** the consumer SHALL remain responsible for embedding, copying, or otherwise supplying that
  application-specific NX content

#### Scenario: Runtime package excludes CLI tooling
- **WHEN** a consumer restores or inspects the `NxLang.Runtime` package
- **THEN** the package SHALL contain the managed binding and native runtime assets needed by .NET
  applications
- **AND** the package SHALL NOT contain the `nxlang` CLI executable
- **AND** CLI packaging SHALL remain outside this runtime package workflow

### Requirement: Runtime package assets are built from one NX version
`NxLang.Runtime` NuGet packages SHALL contain managed and native runtime assets built from the same
NX source revision and version. The package build SHALL fail before publication when a required
supported runtime asset is missing.

#### Scenario: Package contains expected native assets
- **WHEN** the release pipeline produces a `NxLang.Runtime` package
- **THEN** package verification SHALL confirm that the package contains one native `nx_ffi` library
  under `runtimes/<rid>/native/` for each runtime identifier advertised as supported by that package
  version

#### Scenario: Missing supported runtime asset blocks package publication
- **WHEN** a native runtime asset for an advertised supported runtime identifier is not available
  during package assembly
- **THEN** the package build SHALL fail before a `NxLang.Runtime` package is published

#### Scenario: Managed and native assets are version-locked
- **WHEN** a `NxLang.Runtime` package is assembled
- **THEN** the managed assembly and all included native `nx_ffi` libraries SHALL come from the same
  NX source revision
- **AND** the existing managed native ABI validation SHALL remain in place as runtime defense

## MODIFIED Requirements

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

### Requirement: Vendored source consumption is a supported workflow
NX SHALL continue to support consumption from repositories that vendor NX as a git submodule or
subtree when a consumer intentionally wants to build from source. The documented workflow SHALL
describe how to build the native runtime and managed binding locally and reference them from a
consuming .NET application, but SHALL identify this source-based workflow as an advanced or
contributor-oriented alternative to the primary published NuGet package workflow.

#### Scenario: Consumer uses a project reference
- **WHEN** a repository vendors NX source and references the managed binding project directly
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
- **THEN** the documentation SHALL direct the consumer to use the published `NxLang.Runtime` NuGet
  package
- **AND** the workflow SHALL NOT require vendoring NX source or publishing a consumer-local package
