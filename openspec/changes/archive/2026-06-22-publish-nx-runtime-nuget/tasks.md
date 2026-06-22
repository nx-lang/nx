## 1. Package Layout

- [x] 1.1 Add explicit NuGet package metadata for `bindings/dotnet/src/NxLang.Runtime/NxLang.Runtime.csproj`, including package ID, description, tags, and package README behavior consistent with the repository packaging conventions.
- [x] 1.2 Add an MSBuild-controlled native package asset root for `NxLang.Runtime` so release automation can include prebuilt `nx_ffi` files under `runtimes/<rid>/native/`.
- [x] 1.3 Update package packing logic to include native runtime files for supported RIDs without requiring package consumers to import `bindings/dotnet/build/NxLang.Runtime.targets`.
- [x] 1.4 Preserve the existing source-project native staging target for local `ProjectReference` and test workflows.
- [x] 1.5 Ensure `NxLang.Runtime` packaging does not include the `nxlang` CLI executable or CLI-specific artifacts.

## 2. Native Artifact Production

- [x] 2.1 Add or update scripts that build `crates/nx-ffi` in release mode and stage the resulting native library into a normalized RID layout.
- [x] 2.2 Map each supported RID to the correct native library file name: `libnx_ffi.so`, `libnx_ffi.dylib`, or `nx_ffi.dll`.
- [x] 2.3 Ensure staged native artifacts are produced from the same source checkout and version used to build `NxLang.Runtime.dll`.
- [x] 2.4 Document the initial supported RID set in the packaging script or configuration so package contents and documentation stay aligned.

## 3. CI And Release Packaging

- [x] 3.1 Update the build workflow to build and upload `nx-ffi` native artifacts for each supported RID.
- [x] 3.2 Add a package assembly step or job that collects the supported native artifacts before packing the final `NxLang.Runtime` NuGet package.
- [x] 3.3 Update deployable artifact staging so release publishing uses the complete assembled package rather than an OS-local incomplete package.
- [x] 3.4 Make package assembly fail when any advertised supported RID native asset is missing.

## 4. Editor Assets Package

- [x] 4.1 Update `src/vscode` package metadata, exports, and package-generation scripts so `@nx-lang/language` exposes the NX TextMate grammar, markdown code-block grammar, language configuration, and snippets through stable import paths while the checked-in manifest remains the full VS Code extension manifest.
- [x] 4.2 Add an editor-assets package generation script or CI step that runs the existing grammar tests and produces an npm tarball from `src/vscode`.
- [x] 4.3 Add package-content verification for the `@nx-lang/language` npm tarball, including all expected files and export paths.
- [x] 4.4 Add an editor-assets consumption smoke test that imports the public JSON exports from the packed package without referencing an NX source checkout path.
- [x] 4.5 Update release automation to publish `@nx-lang/language` to the configured npm registry while preserving the full VS Code extension package/publish workflow.

## 5. Verification

- [x] 5.1 Add package inspection verification that checks the `.nupkg` contains `lib/net10.0/NxLang.Runtime.dll`, every expected `runtimes/<rid>/native/` library, and no `nxlang` CLI executable.
- [x] 5.2 Add a package-consumption smoke test project or script that restores `NxLang.Runtime` from a local package output and evaluates a trivial NX program without an NX source checkout.
- [x] 5.3 Run the package-consumption smoke test on every supported CI operating system or RID host available in the build matrix.
- [x] 5.4 Keep existing managed and native ABI mismatch tests passing after packaging changes.

## 6. Documentation

- [x] 6.1 Update `bindings/dotnet/README.md` so `PackageReference Include="NxLang.Runtime"` is the primary application integration workflow.
- [x] 6.2 Move submodule/project-reference guidance into an advanced or contributor-oriented source-consumption section.
- [x] 6.3 Document that application-owned `.nx` libraries are not packaged by NX and must be embedded or staged by the consuming application.
- [x] 6.4 Update `src/vscode/README.md` to document `@nx-lang/language` npm package consumption alongside the full VS Code extension publishing workflow.
- [x] 6.5 Document migration steps for consumers replacing submodule integration with the published runtime package and replacing file-based editor asset dependencies with `@nx-lang/language`.
