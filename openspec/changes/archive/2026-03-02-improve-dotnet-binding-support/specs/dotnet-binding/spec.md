## ADDED Requirements

### Requirement: .NET binding layout and support posture
The repository SHALL expose the managed NX binding under `bindings/dotnet` and SHALL treat it as a `.NET 10`-only integration. The binding SHALL remain implemented and documented primarily in C#, and the documentation SHALL state that other .NET languages are expected to work but are not yet validated beyond C# tests and examples.

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
The managed NX binding SHALL declare explicit CLS compliance and SHALL expose a CLS-compliant public API surface. Public interop models SHALL avoid non-CLS primitive types, and public domain concepts that are stable in the native contract SHALL be represented with strong managed types instead of stringly typed values.

#### Scenario: Assembly declares CLS compliance
- **WHEN** a consumer inspects the managed assembly metadata or source
- **THEN** the assembly declares CLS compliance explicitly

#### Scenario: Public diagnostics use CLS-compliant types
- **WHEN** a consumer uses public diagnostic and span types from the managed API
- **THEN** the public members use CLS-compliant primitive types
- **AND** internal pointer-sized interop details remain hidden from the public API

#### Scenario: Severity is strongly typed
- **WHEN** evaluation fails and diagnostics are returned through the managed API
- **THEN** severity is exposed through a managed enum or equivalent strong type rather than a free-form string

### Requirement: Managed code validates native ABI compatibility
The managed NX binding SHALL validate compatibility with the native NX FFI library before relying on runtime calls. Native load failures and compatibility mismatches SHALL produce actionable managed exceptions that explain the likely cause and recovery path.

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
The C-facing header for the NX runtime SHALL be generated from the Rust FFI contract so that exported declarations do not require parallel manual maintenance.

#### Scenario: Header matches exported ABI
- **WHEN** the native FFI contract changes
- **THEN** the generated C header can be refreshed from the Rust source of truth

### Requirement: Vendored source consumption is a supported workflow
NX SHALL support consumption from repositories that vendor NX as a git submodule or subtree. The documented workflow SHALL describe how to build the native runtime and managed binding locally and reference them from a consuming .NET application without requiring publication to NuGet.

#### Scenario: Consumer uses a project reference
- **WHEN** a repository vendors NX source and references the managed binding project directly
- **THEN** the documentation describes how to build NX and resolve the native library for local execution

#### Scenario: Consumer uses built outputs
- **WHEN** a repository vendors NX source and references the built managed assembly
- **THEN** the documentation describes which managed and native artifacts must be copied or referenced together

#### Scenario: Published packaging is not required
- **WHEN** a consumer follows the supported integration path for this phase
- **THEN** the workflow does not require publishing or consuming a NuGet package
