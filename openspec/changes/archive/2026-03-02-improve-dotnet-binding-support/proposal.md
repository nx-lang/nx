## Why

NX already has a working Rust FFI layer and a managed wrapper, but the .NET integration is still positioned as an ad hoc C# binding that depends on manual native library discovery and undocumented consumption patterns. Before broader adoption, the repository needs a clearer .NET support story, a cleaner binding layout, and a supported workflow for consumers who vendor NX as a git submodule or subtree.

## What Changes

- Rename `bindings/csharp` to `bindings/dotnet` while keeping C# as the primary implementation and documentation language.
- Define the .NET binding as a supported managed wrapper over the native NX FFI layer, with an explicit boundary between managed API, native loading, and Rust ABI exports.
- Add explicit CLS compliance to the managed API and tighten public surface documentation so other .NET languages are expected to work, while clarifying that validation and examples remain C#-first for now.
- Improve the managed/native contract with a versioned ABI handshake, generated native header output, and clearer failure semantics for native loading and interop errors.
- Establish a supported source-based consumption workflow for repositories that include NX as a git submodule or subtree.
- Keep the managed binding .NET 10-only for now.
- Prefer project/source reference and built outputs over published NuGet distribution for now; local packaging may be considered later but is out of scope for this change.
- **BREAKING**: Rename the top-level binding directory and related paths from `csharp` to `dotnet`.

## Capabilities

### New Capabilities
- `dotnet-binding`: Provide a supported .NET integration surface for invoking the NX runtime from managed code, including layout, API contract, native loading, documentation, and source-based consumption guidance.

### Modified Capabilities
- None.

## Impact

- Affected code: `bindings/csharp` (renamed to `bindings/dotnet`), `crates/nx-ffi`, `bindings/c`, build/test scripts, and .NET-facing documentation.
- Affected APIs: managed .NET API shape, binding paths, native loading behavior, and exported FFI compatibility checks.
- Affected consumers: repositories that vendor NX source and build the runtime locally for C# applications.
