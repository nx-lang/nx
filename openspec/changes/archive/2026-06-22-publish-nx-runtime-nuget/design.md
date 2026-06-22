## Context

`NxLang.Runtime` is currently consumable by .NET applications only as a source dependency. A
consumer such as ReachMe vendors NX under `external/nx`, references
`bindings/dotnet/src/NxLang.Runtime/NxLang.Runtime.csproj`, imports
`bindings/dotnet/build/NxLang.Runtime.targets`, and runs `cargo build --release -p nx-ffi` as part
of its own build. This couples application builds to the NX repository layout and Rust toolchain
even when the application only needs the runtime.

NX already has separate managed and native pieces:

- `bindings/dotnet/src/NxLang.Runtime` contains the managed .NET binding.
- `crates/nx-ffi` produces the native C ABI library loaded by the managed binding.
- `bindings/dotnet/build/NxLang.Runtime.targets` stages locally built native libraries for
  source-based consumers and tests.
- `src/vscode` contains the NX TextMate grammar, markdown code-block grammar, language
  configuration, snippets, VS Code extension client, and language-server packaging support.
- The GitHub build workflow already runs on Linux, macOS, and Windows and packs NuGet deployables,
  but release currently ships packages from the Linux deployables artifact.

The desired boundary is that NX publishes the engine/runtime, the full VS Code extension, and
reusable editor language assets, while consumers own any domain-specific `.nx` libraries. ReachMe's
`nx-built-in` files are therefore outside this change.

## Goals / Non-Goals

**Goals:**

- Make `PackageReference Include="NxLang.Runtime"` the primary .NET application consumption model.
- Include the native `nx_ffi` binaries in the NuGet package under RID-specific native asset paths.
- Ensure package restore/build/test/publish workflows stage the native library without invoking
  Cargo in the consumer repository.
- Keep managed and native artifacts version-locked to the same NX source revision.
- Preserve source-based consumption for NX contributors and advanced consumers that intentionally
  build from source.
- Document the migration path for consumers replacing submodule/project-reference integration.
- Publish reusable NX editor assets as the `@nx-lang/language` npm package so web editors can
  import the grammar, language configuration, and snippets from a registry package instead of
  `file:../../external/nx`.
- Publish the full NX VS Code extension, including its compiled client and `nx-lsp` server assets,
  through the existing VSIX Marketplace/Open VSX workflow.
- Verify the editor-assets package contents and public export paths during build/release.

**Non-Goals:**

- Package ReachMe-specific `.nx` files or define a general app-content packaging model.
- Publish the `nxlang` CLI.
- Add a shared Monaco integration helper; application-specific Monaco/Shiki setup remains in the
  consuming application for this change.
- Change the native ABI, managed runtime API shape, or NX language semantics.
- Guarantee RIDs that are not built and tested by the release pipeline.

## Decisions

### Decision 1: Publish one `NxLang.Runtime` package with managed and native assets

`NxLang.Runtime` will carry both the managed assembly and the native `nx_ffi` binaries under
`runtimes/<rid>/native/`. Consumers should not need separate native package references or custom
MSBuild imports. The package will not include the `nxlang` CLI executable.

Alternatives considered:

- Separate `NxLang.Runtime.Native.<rid>` packages: rejected for the first packaging phase because it
  adds dependency/version coordination without improving the consumer experience.
- Source-only package that builds Rust during restore/build: rejected because it preserves the core
  problem and makes consumer builds slower and less reproducible.
- Package only the managed assembly and require manual native deployment: rejected because it keeps
  the failure mode that the package is meant to eliminate.
- Include the `nxlang` CLI in `NxLang.Runtime`: rejected because the CLI is developer/build tooling,
  not part of the .NET runtime dependency.

### Decision 2: Use .NET runtime asset conventions for native library staging

The package will place native outputs in NuGet runtime asset locations such as
`runtimes/linux-x64/native/libnx_ffi.so`, `runtimes/osx-arm64/native/libnx_ffi.dylib`, and
`runtimes/win-x64/native/nx_ffi.dll`. The .NET SDK should then carry the correct native asset into
test, run, and publish outputs for applications targeting a supported RID.

The existing `NxLang.Runtime.targets` file remains useful for source-based project references and
local test workflows, but package consumers should not import it manually.

Alternatives considered:

- A package `buildTransitive` target that copies native assets manually: useful only if standard
  runtime assets are insufficient, so it should be a fallback rather than the default design.
- Copying all native binaries to every output directory: rejected because it bloats outputs and
  bypasses RID selection.

### Decision 3: Assemble the NuGet package from release-built native artifacts

The release pipeline must build `nx-ffi` for each supported RID before packing the final
`NxLang.Runtime` package. The package should be assembled only after all required native artifacts
are available and validated. A release should not publish a package that advertises a RID without
including the corresponding native library.

Given the current workflow, this likely means adding a packaging job that collects native artifacts
from the OS matrix and produces the final NuGet package from one place, instead of relying on the
Linux build's `deployables` artifact alone.

Alternatives considered:

- Let each OS build publish its own `NxLang.Runtime` package: rejected because NuGet package IDs and
  versions must be unique and each OS package would be incomplete.
- Cross-compile every native library from Linux: rejected for the initial implementation because it
  adds toolchain complexity and may not exercise platform-specific build/link behavior.

### Decision 4: Keep source-based integration supported but demote it in documentation

The README should lead with `PackageReference` usage. The submodule/project-reference workflow
should move to an advanced or contributor section for cases where a consumer deliberately wants to
test unreleased NX changes.

Alternatives considered:

- Remove source-based guidance completely: rejected because it is still valuable while NX is under
  active development and for local binding/native ABI work.
- Keep source-based consumption as the primary documentation path: rejected because it keeps
  consumers pointed at the build model this change is replacing.

### Decision 5: Keep consumer-owned `.nx` resources out of NX runtime packages

`NxLang.Runtime` should not include ReachMe's `nx-built-in` files or any other application-specific
NX programs. Consumers remain responsible for embedding or staging their own NX source assets.

Alternatives considered:

- Add a generic NX built-ins package now: rejected because the immediate built-ins are
  ReachMe-specific and would blur product boundaries.
- Add content-file staging to `NxLang.Runtime`: rejected because the runtime package should contain
  the engine, not app content.

### Decision 6: Publish the VS Code extension and a separate npm editor-assets package from `src/vscode`

The checked-in `src/vscode/package.json` remains the VS Code extension manifest with the unscoped
`nx-language` extension ID. The VSIX workflow packages and publishes the full extension, including
the compiled TypeScript client and the target-specific `nx-lsp` server asset.

The reusable editor-assets package will be generated from a clean staging manifest and published as
`@nx-lang/language`. The staging package will expose stable JSON import paths for:

- `@nx-lang/language/grammar`
- `@nx-lang/language/markdown-codeblock-grammar`
- `@nx-lang/language/language-configuration`
- `@nx-lang/language/snippets`

This supports browser editors such as ReachMe's Monaco/Shiki integration without requiring the NX
repository checkout while keeping VSIX runtime files such as `out/**`, `server/**`, and
`vscode-languageclient` out of the npm editor-assets tarball.

Alternatives considered:

- Publish only a VSIX and make ReachMe extract assets from it: rejected because ReachMe needs normal
  JavaScript package imports, not an IDE extension artifact.
- Move editor assets into a new package directory now: rejected because `src/vscode` already has the
  source assets, tests, and package scripts needed for staged asset package generation.
- Keep the npm package named `nx-language`: rejected before first publication because current usage
  is only a local file dependency, and the scoped `@nx-lang/language` name gives NX a clearer
  package family for future JavaScript packages.
- Rename the checked-in VS Code extension manifest to `@nx-lang/language`: rejected because VSIX
  tooling and marketplace identity require the unscoped `nx-language` extension name.
- Publish a Monaco-specific package now: rejected because ReachMe's current Monaco glue includes
  application choices such as theme selection and lazy setup; reusable editor assets are the stable
  boundary for this phase.

### Decision 7: Defer CLI packaging to a follow-up change

The `nxlang` CLI will not be included in the `NxLang.Runtime` NuGet package and will not be
published as part of this change. CLI distribution should be proposed separately after runtime NuGet
packaging and `@nx-lang/language` publishing are stable.

Alternatives considered:

- Include `nxlang` in `NxLang.Runtime`: rejected because it would make every .NET runtime consumer
  download developer tooling they do not need and would mix runtime deployment with build-tool
  distribution.
- Add CLI publishing to this change: rejected because runtime packaging and editor asset publishing
  already cover the submodule-removal path for ReachMe; CLI distribution has its own target
  artifact choices and release concerns.

## Risks / Trade-offs

- [RID coverage gaps] A consumer on an unsupported RID will restore the managed package but fail to
  load `nx_ffi`. → Mitigation: document supported RIDs, test package restore/run per supported RID,
  and make native load errors name the missing runtime asset clearly.
- [Incomplete release package] The release pipeline could pack before all native artifacts are
  present. → Mitigation: add explicit package verification that inspects the `.nupkg` for every
  supported RID and runs a package-consumption smoke test.
- [ABI mismatch] Managed and native assets from different revisions could be packed together.
  → Mitigation: assemble the package from one checked-out revision and keep the existing managed ABI
  validation as runtime defense in depth.
- [Build workflow complexity] Packaging now spans Rust, .NET, and multiple operating systems.
  → Mitigation: keep native artifact collection isolated in scripts/targets and avoid changing
  runtime APIs as part of the packaging work.
- [Consumer migration churn] Existing source-based consumers need project-file cleanup.
  → Mitigation: provide explicit README migration guidance from `ProjectReference` plus imported
  targets to `PackageReference`.
- [Package boundary drift between VSIX and npm assets] The VS Code extension and npm editor-assets
  package share source files but have different runtime contents. → Mitigation: keep the checked-in
  manifest as the VSIX manifest, generate the `@nx-lang/language` package from a clean staging
  manifest, and verify the packed npm tarball contains only the expected reusable assets.
- [Editor export breakage] Web consumers rely on stable package export paths for JSON imports.
  → Mitigation: verify the packed npm tarball exposes the expected grammar, markdown grammar,
  language configuration, and snippets paths.
- [Runtime package includes tooling by accident] Adding more distribution work could blur the
  boundary between runtime and developer tooling. → Mitigation: verify `NxLang.Runtime` package
  contents and keep `nxlang` CLI packaging in a separate follow-up change.

## Migration Plan

1. Add package metadata and native asset packing support to `NxLang.Runtime`.
2. Add release/build steps that produce `nx-ffi` native libraries for each supported RID and stage
   them into the package layout.
3. Add editor-asset package generation from `src/vscode` and verify the packed npm tarball contains
   the expected files and export paths.
4. Add package verification that confirms the `.nupkg` contains the managed assembly and expected
   native assets.
5. Add a package-consumption smoke test that restores the package from a local feed and evaluates a
   trivial NX program without NX source checkout or Cargo.
6. Add an editor-assets consumption smoke test that installs or packs the npm package and imports
   the public grammar/configuration exports without an NX source checkout.
7. Update `bindings/dotnet/README.md` so `PackageReference` is the primary workflow and source
   vendoring is advanced/contributor guidance.
8. Update `src/vscode/README.md` so npm editor-assets consumption is documented separately from the
   full VS Code extension packaging and publishing workflow.
9. Publish prerelease packages and migrate ReachMe by replacing the submodule/project reference with
   `PackageReference`, replacing the `file:../../external/nx/src/vscode` dependency with the npm
   `@nx-lang/language` package, and leaving ReachMe's `.nx` assets in ReachMe.

Rollback is straightforward before public publication: keep source-based consumption as-is and stop
publishing the package. After publication, rollback should publish a fixed package version rather
than mutating an already-published package.

## Open Questions

- Which RID set is required for the first public package beyond the CI-hosted platforms?
- Should package-consumption smoke tests run only in CI or also as a local script used before
  release?
