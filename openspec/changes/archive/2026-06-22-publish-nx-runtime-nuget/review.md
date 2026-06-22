# Review: publish-nx-runtime-nuget

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/dotnet-binding/spec.md, specs/editor-assets/spec.md

**Reviewed code:**
- `bindings/dotnet/src/NxLang.Runtime/NxLang.Runtime.csproj` (package metadata, native asset packing, validation target)
- `bindings/dotnet/src/NxLang.Runtime/Interop/NxNativeLibrary.cs` (error message)
- `bindings/dotnet/README.md`, `src/vscode/README.md` (docs)
- `src/vscode/package.json` (scoped name, exports, scripts)
- `src/vscode/scripts/verify-editor-package.mjs`, `src/vscode/scripts/smoke-editor-package.mjs`
- `tools/packaging/NxRuntimeRids.ps1`, `Stage-NxRuntimeNativeArtifact.ps1`, `Test-NxRuntimePackage.ps1`, `SmokeTest-NxRuntimePackage.ps1`
- `.github/workflows/build.yml`, `release.yml`, `vscode-extension.yml`

## Findings

### ✅ Resolved - RF1 Renaming the manifest `name` to scoped `@nx-lang/language` likely breaks the still-active `vsce package` step
- **Severity:** Medium
- **Evidence:** `src/vscode/package.json:2` now sets `"name": "@nx-lang/language"`. The same manifest is still consumed by `vscode-extension.yml`, whose `Package .vsix` step was updated to run `pnpm run package:vsix` → `vsce package` ([vscode-extension.yml:72](.github/workflows/vscode-extension.yml#L72), [package.json](src/vscode/package.json)). `vsce` validates the extension `name` against an unscoped identifier pattern and rejects values containing `@`/`/`. That workflow triggers on `pull_request` for `src/vscode/**` ([vscode-extension.yml:4-6](.github/workflows/vscode-extension.yml#L4-L6)), so this change's own PR will exercise it. `design.md:142-145` explicitly anticipated this ("the npm package manifest should be generated or split rather than forcing the npm editor-assets package and VSIX manifest to share one name"), but the implementation kept one shared manifest and only renamed the script.
- **Recommendation:** Confirm whether `vsce package` still succeeds with the scoped name. If it fails (expected), keep the VS Code extension manifest unscoped and generate the scoped npm editor-assets package from a separate manifest or staging directory.
- **Fix:** Superseded by the combined publishing direction. The checked-in manifest now remains the unscoped VS Code extension manifest (`nx-language`), and `@nx-lang/language` is generated from a separate staging manifest in `scripts/package-language.mjs`.
- **Verification:** Resolved by design and implementation shape. `vsce` sees the unscoped extension manifest, while the npm editor-assets tarball gets the scoped package identity during staged packing.

### ✅ Verified - RF2 `src/vscode/dist/` pack output is not git-ignored
- **Severity:** Low
- **Evidence:** `package:language` runs `pnpm pack --pack-destination dist` ([package.json:43](src/vscode/package.json)), producing `src/vscode/dist/*.tgz`. There is no `src/vscode/.gitignore`, and the root `.gitignore` only ignores `docs/dist/` ([.gitignore:277](.gitignore#L277)). A locally produced tarball can be accidentally committed.
- **Recommendation:** Add a `src/vscode/.gitignore` (or a root rule) ignoring `dist/`.
- **Fix:** Added `src/vscode/.gitignore` to ignore `dist/` tarballs and locally generated `*.vsix` files.
- **Verification:** Confirmed. `src/vscode/.gitignore` exists with `dist/` and `*.vsix` entries, covering both the `package:language` pack output and the new `package-vsix.mjs` VSIX output.

### ✅ Verified - RF3 Native staging maps RID from OS only, ignoring architecture
- **Severity:** Low
- **Evidence:** `Stage-NxRuntimeNativeArtifact.ps1:13-21` selects `win-x64`/`osx-arm64`/`linux-x64` purely from `$IsWindows`/`$IsMacOS`/else. This is correct for the current build matrix (`macos-14` is arm64), but it silently mislabels artifacts if the script ever runs on an x64 macOS runner (e.g. `macos-13`) or an arm64 Linux/Windows host — producing a package whose `runtimes/osx-arm64/native/` actually contains an x64 binary, which the package verification cannot detect.
- **Recommendation:** Derive the architecture (e.g. from `uname -m` / `RuntimeInformation.OSArchitecture`) and either build the matching RID or fail fast when the host architecture doesn't match the requested RID.
- **Fix:** Updated `Stage-NxRuntimeNativeArtifact.ps1` to infer RIDs from both host platform and `RuntimeInformation.OSArchitecture`, and to fail fast when an explicit RID does not match the current host platform or architecture.
- **Verification:** Confirmed. Auto-detection now throws on a non-X64 Windows/Linux or non-Arm64 macOS host ([Stage-NxRuntimeNativeArtifact.ps1:13-34](../../../tools/packaging/Stage-NxRuntimeNativeArtifact.ps1#L13-L34)), and an explicitly-passed RID is validated against both host platform (lines 57-59) and host architecture via `expectedArchitectureByRid` (lines 36-64) before any cargo build or copy. This eliminates the silent mislabeling path — an x64 macOS runner can no longer stage a binary as `osx-arm64`.

### ✅ Verified - RF4 Dead `*.symbols.nupkg` filter in package selection
- **Severity:** Low
- **Evidence:** Both the verify and smoke steps select the package with `Where-Object Name -NotLike '*.symbols.nupkg'` ([build.yml](.github/workflows/build.yml)). The repo sets `SymbolPackageFormat=snupkg` ([Directory.Build.props:36](Directory.Build.props#L36)), so symbol packages are `*.snupkg` (already excluded by the `NxLang.Runtime.*.nupkg` glob). The filter never matches anything and is misleading.
- **Recommendation:** Drop the filter or change it to `*.snupkg` to reflect the actual symbol format.
- **Fix:** Removed the dead `*.symbols.nupkg` filters from the package verification and smoke-test selection steps.
- **Verification:** Confirmed. Both selection steps in `build.yml` are now `Get-ChildItem ...NxLang.Runtime.*.nupkg | Select-Object -First 1`. The `*.nupkg` glob does not match the `.snupkg` symbol package, so selection remains correct without the misleading filter.

## Questions
- Design Open Question (unresolved by this change): is the initial RID set `linux-x64; osx-arm64; win-x64` sufficient for the first public package, or are additional RIDs (e.g. `linux-arm64`, `win-arm64`, `osx-x64`) expected? The supported set is currently hardcoded in three places (csproj `NxRuntimeSupportedRids`, `NxRuntimeRids.ps1`, README) which stay in sync today but will need coordinated edits to extend.

## Summary
Strong, well-structured implementation that satisfies the runtime packaging scenarios: the NuGet package carries the managed assembly plus RID-specific native assets, a `BeforeTargets="GenerateNuspec"` validation target fails the pack when any advertised RID asset is missing, and the CI `build → package → smoke-test` flow assembles the package from one revision and verifies/smoke-tests it per OS. After the latest pull, the editor work now intentionally combines full VS Code extension publishing with a separate staged `@nx-lang/language` npm package so the VSIX can include LSP runtime assets while the npm package exposes only reusable editor assets.

## New Findings Discovered During 2026-06-22 13:48 Review

### ✅ Verified - RF5 Build workflow conflict can regress the refreshed .NET solution path
- **Severity:** Medium
- **Evidence:** Pulling latest introduced `bindings/dotnet/NxLang.sln` as the build entry point and removed the root `NX.sln`. The upstream side of the build conflict uses `dotnet build bindings/dotnet/NxLang.sln` (`.github/workflows/build.yml:48-52`), while the stashed publishing side still uses `dotnet build -t:build --no-restore` from the repository root (`.github/workflows/build.yml:54-58`). Resolving by taking the stashed command would leave `dotnet build` with no root project or solution.
- **Recommendation:** Keep the publishing change's native artifact staging and package assembly flow, but update the managed build/format commands to use `bindings/dotnet/NxLang.sln` consistently with the pulled repo layout.
- **Fix:** Resolved `build.yml` by keeping `Stage-NxRuntimeNativeArtifact.ps1` and the package assembly jobs, while changing the managed build and format commands to target `bindings/dotnet/NxLang.sln`.
- **Verification:** Confirmed. The `build` job now runs `dotnet build bindings/dotnet/NxLang.sln -t:build ...` and `dotnet format bindings/dotnet/NxLang.sln --verify-no-changes`, both targeting the solution; the new `package` job packs `NxLang.Runtime.csproj` directly. No remaining root-relative `dotnet build`/`dotnet format` invocation, so there is no project-less build command.

### ✅ Verified - RF6 Editor-assets scripts now need to merge with the VS Code extension publishing scripts
- **Severity:** Medium
- **Evidence:** The pulled `vscode-extension.yml` publish job expects `pnpm run package:verify` to produce a VSIX and set `steps.package.outputs.vsix_path` / `vsix_name` before upload and registry publishing (`.github/workflows/vscode-extension.yml:132-168`). The stashed editor-assets side of `src/vscode/package.json` replaces `package`, omits `package:verify`, and changes `publish:vsce` / `publish:ovsx` back to raw registry CLIs (`src/vscode/package.json:53-69`). The stashed workflow conflict also replaces the `id: package` output-producing step with a plain `Package .vsix` step (`.github/workflows/vscode-extension.yml:153-156`), which would leave the later upload/publish steps with empty outputs.
- **Recommendation:** Preserve the pulled VSIX workflow contract (`package`, `package:verify`, `package:ls`, `publish:vsce`, `publish:ovsx`, `publish:all`) and add the npm editor-assets commands under distinct names such as `package:language`, `verify:package`, and `smoke:package`. Do not replace the output-producing VSIX package step in `vscode-extension.yml`.
- **Fix:** Preserved the full VSIX script/workflow contract and added `package:language`, `verify:package`, and `smoke:package` as separate npm editor-assets commands. The VS Code workflow keeps its output-producing `Verify and package VSIX` step.
- **Verification:** Confirmed. `src/vscode/package.json` retains the full VSIX contract (`package` → `package-vsix.mjs`, `package:verify`, `package:ls` → `vsce ls`, `publish:vsce`/`publish:ovsx` → `publish-vsix.mjs`, `publish:all`) and adds the three distinctly-named npm commands. `vscode-extension.yml` is unmodified by this change (working tree clean vs HEAD), so its output-producing package step is untouched.

### ✅ Verified - RF7 The scoped-name VSIX workaround is incomplete after LSP packaging landed
- **Severity:** Medium
- **Evidence:** The previous RF1 fix stages a temporary unscoped VSIX manifest, but the stashed `package-vsix.mjs` only copies README, changelog, license, language configuration, syntaxes, and snippets (`src/vscode/scripts/package-vsix.mjs:43-56`). Latest main added an LSP-enabled VS Code package contract requiring `out/extension.cjs`, `out/serverPath.js`, and `server/<target>/nx-lsp` (`openspec/specs/vscode-extension-publishing/spec.md`), and the pulled manifest now includes `main`, activation events, `out/**`, and `server/**`. Also, `package:verify` runs `package:ls` before `package`, and the pulled `package:ls` invokes `vsce ls` directly against the checked-in manifest (`src/vscode/package.json:53-57`), so a scoped `@nx-lang/language` manifest can still break verification before `package-vsix.mjs` runs.
- **Recommendation:** If the checked-in manifest remains scoped for npm, make all VSIX operations (`package`, `package:ls`, and verification) use the same generated unscoped manifest and include the LSP client/server assets. Otherwise split the npm package manifest from the VS Code extension manifest so VSIX tooling continues to see `nx-language`.
- **Fix:** Removed the scoped checked-in manifest approach. The checked-in manifest remains `nx-language`, `package-vsix.mjs` packages the full extension including `out/**` and `server/**`, and `package-language.mjs` generates the scoped npm manifest separately.
- **Verification:** Confirmed. `src/vscode/package.json:2` is `"name": "nx-language"` (valid unscoped vsce identity) with `files` including `out/**` and `server/**`; `package-vsix.mjs` runs `vsce package` against that checked-in manifest in place (no scoped temp manifest), so `package:ls`/`package`/verification all see `nx-language`. The scoped `@nx-lang/language` identity now exists only inside `package-language.mjs`'s staged manifest, eliminating the earlier scoped-vs-VSIX conflict.

### ✅ Verified - RF8 The npm editor-assets package can inherit VSIX-only assets and metadata from the refreshed manifest
- **Severity:** Medium
- **Evidence:** The latest VS Code extension work expanded the shared `src/vscode/package.json` manifest with `main`, activation events, `vscode-languageclient`, and `files` entries for `out/**` and `server/**` (`src/vscode/package.json:41-83`). The publishing change's `package:language` still runs `pnpm pack --pack-destination dist` directly from `src/vscode` (`src/vscode/package.json:61-64`). If `out/` or `server/` exists from a local or CI LSP build, the `@nx-lang/language` npm tarball can include VSIX runtime files or native `nx-lsp` binaries, even though the editor-assets spec only requires reusable JSON language assets and explicitly excludes VS Code extension publication from this change.
- **Recommendation:** Generate the npm editor-assets package from a clean staging manifest/directory or otherwise give `package:language` an npm-specific file list containing only the grammar, markdown grammar, language configuration, snippets, README/license/changelog, and export metadata.
- **Fix:** Added `scripts/package-language.mjs`, which copies only README, changelog, license, language configuration, syntaxes, and snippets into a temporary staging directory and writes an `@nx-lang/language` manifest before packing.
- **Verification:** Confirmed. `package-language.mjs:14-21,42-43` copies only the six asset paths into a fresh `mkdtemp` staging dir and packs from there, so `out/**`/`server/**`/`vscode-languageclient` from the checked-in manifest can never enter the tarball regardless of local build state. `verify-editor-package.mjs:30-40` independently fails the build if any `package/out/`, `package/server/`, or `package/src/` entry appears, and the CI `editor-assets` job runs that verification on every build.

## New Findings Discovered During 2026-06-22 14:46 Review

### ✅ Verified - RF9 The `@nx-lang/language` npm version is static `0.1.0`, so release republish fails and the package version cannot track the runtime version
- **Severity:** Medium
- **Evidence:** `scripts/package-language.mjs:30` sets the staged package version from `manifest.version`, which is the hardcoded `"version": "0.1.0"` in [src/vscode/package.json:5](src/vscode/package.json#L5). Nothing in the editor-assets CI job ([build.yml editor-assets job](.github/workflows/build.yml)) or the release step bumps it, while the NuGet package version is computed by Nerdbank.GitVersioning (the build/package jobs use `fetch-depth: 0` and `tools/variables/_define.ps1` so nbgv "can do its work"). The release workflow runs `npm publish ${{ runner.temp }}/editor-assets/*.tgz` unconditionally whenever `NPM_TOKEN` is defined ([release.yml:103-109](.github/workflows/release.yml#L103-L109)). Consequences: (1) the npm registry rejects an already-published version with E403, so the *second* release (or any `workflow_dispatch` re-ship) fails at the publish step; (2) the editor-assets package version permanently reads `0.1.0` and is decoupled from the NX runtime/source revision it was generated from, so consumers cannot tell which NX version a grammar tarball matches. This is the npm analogue of the version-lock guarantee the NuGet side gets from nbgv.
- **Recommendation:** Derive the staged npm version from the release version (e.g. read the nbgv-computed version, or pass it into `package-language.mjs`) so the tarball version advances with each release and matches the runtime revision. Alternatively, gate the publish so a duplicate version is a no-op rather than a hard failure, and document the manual bump requirement.
- **Fix:** Updated `package-language.mjs` to use `NPM_PACKAGE_VERSION`/NBGV npm version data and fall back to `dotnet nbgv get-version -v NpmPackageVersion` locally. The `editor-assets` CI job now restores local .NET tools, computes `NPM_PACKAGE_VERSION`, and exports it before packing.
- **Verification:** Confirmed. `package-language.mjs:23-39,46-50` now derives the staged version from `NPM_PACKAGE_VERSION`/`NBGV_NpmPackageVersion` or `dotnet nbgv get-version -v NpmPackageVersion`, with a clear error if none is available — the hardcoded `0.1.0` is gone from the publish path. The `editor-assets` job uses `fetch-depth: 0`, runs `./init.ps1 -NoPrerequisites -NoRestore` (which still performs `dotnet tool restore` — that step is gated by `-NoToolRestore`, not `-NoRestore`, per init.ps1:102), and sets `NPM_PACKAGE_VERSION` from nbgv before `package:language`. `nbgv` 3.9.50 is in `.config/dotnet-tools.json` and `NpmPackageVersion` is a built-in nbgv variable, so each release produces a distinct, runtime-tracking npm version rather than a duplicate `0.1.0`.

### ✅ Verified - RF10 `SmokeTest-NxRuntimePackage.ps1` still auto-detects the RID from OS only, unlike the RF3-hardened staging script
- **Severity:** Low
- **Evidence:** When `-RuntimeIdentifier` is omitted, `tools/packaging/SmokeTest-NxRuntimePackage.ps1:9-17` selects `win-x64`/`osx-arm64`/`linux-x64` purely from `$IsWindows`/`$IsMacOS`/else, with no architecture check — the exact OS-only pattern RF3 replaced in `Stage-NxRuntimeNativeArtifact.ps1` with host-architecture validation. CI always passes `-RuntimeIdentifier ${{ matrix.rid }}` ([build.yml smoke-test-package job](.github/workflows/build.yml)), so this only affects local/default invocation: on an x64 macOS host it would request `osx-arm64` and the subsequent `dotnet run --runtime osx-arm64` would fail loudly rather than mislabel anything, but the inconsistency is a latent footgun once more RIDs/arches are supported.
- **Recommendation:** Mirror the RF3 fix — infer the RID from both `$IsWindows`/`$IsMacOS` and `RuntimeInformation.OSArchitecture`, and throw a clear error on an unsupported host architecture instead of silently assuming x64/arm64.
- **Fix:** Moved host RID inference and host/RID validation into `NxRuntimeRids.ps1`, then updated both `Stage-NxRuntimeNativeArtifact.ps1` and `SmokeTest-NxRuntimePackage.ps1` to use the shared architecture-aware helpers.
- **Verification:** Confirmed. `NxRuntimeRids.ps1:39-84` adds `Get-NxRuntimeHostRuntimeIdentifier` (matches host platform **and** `OSArchitecture`, throwing on an unsupported host) and `Assert-NxRuntimeHostMatchesRuntimeIdentifier` (validates an explicit RID against both host platform and architecture). `SmokeTest-NxRuntimePackage.ps1:11-14` now calls these instead of the old OS-only `if/elseif` block, and `Stage-NxRuntimeNativeArtifact.ps1:13-18` shares the same helpers, so both scripts have identical architecture-aware behavior. `Test-NxRuntimePackage.ps1` still consumes `Get-NxRuntimeNativeLibraryName`/`$NxRuntimeSupportedRids`, which are unchanged — no regression from the refactor.

## Questions (2026-06-22 14:46 Review)
- Answered by RF9 fix: per-release versioning of `@nx-lang/language` is automated from the NBGV npm package version rather than manually bumped in `src/vscode/package.json`.

## Summary (2026-06-22 14:46 Review)
Re-reviewed all artifacts and the full staged diff. The runtime-packaging path (csproj native asset packing + `BeforeTargets="GenerateNuspec"` validation, `build → package → smoke-test-package` CI flow, `Test`/`SmokeTest` scripts, version-locking via single-revision checkout) and the editor-assets staging path (clean `package-language.mjs` manifest, `verify`/`smoke` scripts, RID coverage) are solid and satisfy the specs. RF1–RF8 remain addressed. RF9 and RF10 are now fixed: editor-assets package versions are derived from the NBGV npm package version, and smoke-test RID auto-detection uses the same architecture-aware helper as native staging.

## Verification (2026-06-22 15:0X Verification)
Verified all six previously-`Fixed` findings (RF5–RF10) against the current files; every fix is correct and complete, so all are now `✅ Verified`. No findings were reopened and no new issues were discovered during verification. With RF1–RF4 already verified/resolved, all ten findings are now closed and the change is ready to archive.
