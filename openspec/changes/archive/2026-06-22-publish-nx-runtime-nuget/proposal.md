## Why

Consumers currently need to vendor NX source, build the Rust FFI library locally, and reference the
managed binding project directly before they can use `NxLang.Runtime`. ReachMe also references the
NX editor support from the repository checkout so its browser-based editor can import the NX
TextMate grammar and language configuration. Those source dependencies are appropriate for
contributors, but they are the wrong default for applications that only need stable runtime and
editor assets.

## What Changes

- Publish `NxLang.Runtime` as a NuGet package that carries the managed .NET binding and the native
  `nx_ffi` runtime binaries needed at execution time.
- Place native runtime binaries in NuGet runtime-specific asset locations so normal .NET restore,
  build, test, and publish workflows stage the correct library without requiring a consumer-side
  Rust toolchain.
- Update .NET binding documentation to make `PackageReference` the primary consumer workflow.
- Generate and publish the NX editor assets as the `@nx-lang/language` npm package that web editor
  consumers can import without depending on the NX source checkout.
- Include the NX TextMate grammar, markdown code-block grammar, language configuration, and snippets
  in the editor-assets package with stable package export paths.
- Publish the full NX VS Code extension, including its compiled client and `nx-lsp` server assets,
  through the VSIX Marketplace/Open VSX workflow.
- Keep source vendoring and local native builds documented as contributor or advanced integration
  workflows, not as the normal application consumption path.
- Do not include the `nxlang` CLI in `NxLang.Runtime`; CLI packaging will be handled by a separate
  follow-up change after runtime and editor-assets packaging are stable.
- Leave consumer-owned `.nx` libraries and domain assets outside NX packaging.

## Capabilities

### New Capabilities
- `editor-assets`: Publish reusable NX editor language assets as `@nx-lang/language` for web
  editors and other JavaScript tooling without requiring an NX source checkout.

### Modified Capabilities
- `dotnet-binding`: `NxLang.Runtime` will support published NuGet consumption with bundled native
  runtime assets, replacing source vendoring as the primary application integration workflow.

## Impact

- Affected code: `bindings/dotnet/src/NxLang.Runtime`, `bindings/dotnet/build`, native artifact
  staging/packaging scripts or targets, `src/vscode`, and package verification tests.
- Affected documentation: `bindings/dotnet/README.md`, `src/vscode/README.md`, and any release or
  consumption guidance for the managed binding and editor assets.
- Affected build/release systems: NX release automation must build `nx-ffi` for supported runtime
  identifiers before packing `NxLang.Runtime`, publish the full VS Code extension through its VSIX
  workflow, and pack/publish `@nx-lang/language` without including VSIX-only runtime files in the
  npm editor-assets package.
- Affected consumers: applications such as ReachMe can replace NX submodule/project references with
  a normal `PackageReference` and replace file-based editor asset dependencies with a registry npm
  package while continuing to own and stage their own `.nx` content.
