## Why

Downstream Node applications need first-class access to NX host/compiler/runtime capabilities
without routing through .NET or shelling out to the CLI. A native Node SDK lets JavaScript and
TypeScript build in-memory workspaces, generate deterministic NX IR, evaluate NX entrypoints, and
capture structured diagnostics through the same Rust capabilities already exposed to .NET.

The existing .NET package name `NxLang.Runtime` is also too narrow for an SDK that supports
compilation, diagnostics, IR generation, workspace artifacts, and evaluation. This change
standardizes the language binding packages on SDK naming: `NxLang.Sdk` for .NET and
`@nx-lang/sdk-node` for the Node version of the JavaScript NX SDK.

## What Changes

- Add a Node package named `@nx-lang/sdk-node`, backed by napi-rs / N-API.
- Rename the .NET package, project, and assembly from `NxLang.Runtime` to `NxLang.Sdk` with no
  backward compatibility package or shim.
- Expose a TypeScript-friendly public API for workspaces, workspace modules, library registries,
  build contexts, reusable program artifacts, IR generation, evaluation, diagnostics, and native
  errors.
- Support in-memory workspace modules with stable logical identities, duplicate detection,
  validation diagnostics, and explicit entry identity selection.
- Support reusable workspace program artifacts that can generate deterministic NX IR JSON and
  metadata, then evaluate supported entrypoints to JSON or bytes where parity with existing native
  host APIs exists.
- Add TypeScript declarations, lifecycle/resource-disposal semantics, local source consumption
  guidance, and a path for future npm/prebuild distribution.
- Add tests proving behavior against existing .NET, Rust, or CLI expectations for workspace
  validation, workspace builds, IR generation, JSON evaluation, diagnostics, duplicate module
  handling, missing entrypoints, and invalid source.

## Capabilities

### New Capabilities
- `sdk-node`: Node-only native SDK for building, validating, generating IR from, and evaluating NX
  source and in-memory workspaces.

### Modified Capabilities

- `dotnet-binding`: Rename the managed SDK package/project/assembly to `NxLang.Sdk` and update
  package, documentation, and test references accordingly.

## Impact

- Adds a new Node SDK package under the repository's bindings area, including native Rust
  glue, TypeScript declarations, package metadata, tests, and documentation.
- Renames the .NET SDK package/project/assembly and updates source-based and NuGet consumption
  guidance to use `NxLang.Sdk`.
- Adds napi-rs / N-API build tooling and local development scripts for source-based consumers.
- Reuses existing Rust `nx-api` and native host concepts rather than replacing the pure TypeScript
  NX IR runtime under `runtime/typescript`.
- Gives downstream Node applications a direct path to compile database-backed and in-memory NX
  modules into deterministic IR/runtime bundles with structured diagnostics and metadata.
- Introduces a breaking package identity change for existing .NET consumers; no compatibility path
  is required for the previous `NxLang.Runtime` package identity.
