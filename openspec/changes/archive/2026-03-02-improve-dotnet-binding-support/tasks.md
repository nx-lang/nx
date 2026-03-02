## 1. Repository layout and binding metadata

- [x] 1.1 Rename `bindings/csharp` to `bindings/dotnet` and update solution files, project references, scripts, and documentation paths.
- [x] 1.2 Keep the managed binding projects targeting `.NET 10` only and update binding documentation to describe the package as C#-first with expected but unvalidated compatibility for other .NET languages.
- [x] 1.3 Reorganize the managed binding so internal interop and native-loading code are clearly separated from the public runtime API.

## 2. CLS-compliant managed API

- [x] 2.1 Add explicit assembly-level CLS compliance to the managed binding.
- [x] 2.2 Update public diagnostic and related model types to use CLS-compliant public members instead of non-CLS primitives.
- [x] 2.3 Replace stringly typed managed diagnostic severity with a strong managed type that matches the stable native diagnostic model.
- [x] 2.4 Update public API documentation and tests to reflect the CLS-compliant managed surface.

## 3. Native ABI contract and validation

- [x] 3.1 Add a native ABI version export to `crates/nx-ffi` and expose it through the C header.
- [x] 3.2 Add managed startup validation for the native ABI version and improve exceptions for missing, incompatible, or unloadable native libraries.
- [x] 3.3 Introduce header generation for `bindings/c/nx.h` from the Rust FFI contract and document how it is refreshed.

## 4. Source-based consumption workflow

- [x] 4.1 Document the primary vendored-source workflow using a `ProjectReference` to the managed binding plus locally built native artifacts.
- [x] 4.2 Document the secondary fallback workflow that references built managed assemblies and the required native library outputs together.
- [x] 4.3 Add helper build logic, scripts, or targets to stage native outputs for local execution in the supported source-based workflows.
- [x] 4.4 Update C# tests to validate the documented loading and execution path without adding non-C# smoke tests.
