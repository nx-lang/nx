# Independent review: package smoke-test CI fix

Scope: the uncommitted change to `tools/packaging/SmokeTest-NxSdkPackage.ps1` only.

## Verdict

**No findings.** I found nothing actionable in this fix.

## What I checked

- **Diff.** The only change is the generated `Program.cs`. It drops the call to the removed
  `NxRuntime.Evaluate<int>`, calls `NxProgramArtifact.Build("let root() = { 42 }", "main.nx")` and
  then `GenerateNxIr()`, and checks that `Identity == "main.nx"`, that `Bytes` is not empty, and
  that `Metadata.FunctionEntrypoints` contains `root`. The rest of the script is unchanged: package
  feed setup, `dotnet add`, `dotnet run --runtime $RuntimeIdentifier --no-self-contained`, and the
  exit-code checks.
- **The script only uses retained APIs, and they match what it calls.**
  - `NxProgramArtifact.Build(string, string?)` (`bindings/dotnet/src/NxLang.Sdk/NxProgramArtifact.cs:45`)
    is public and `IDisposable`, so the `using` declaration is valid.
  - `GenerateNxIr()` (`NxProgramArtifact.cs:258`) returns the entry module's artifact.
  - `NxGeneratedNxIr.Identity` and `NxGeneratedNxIr.Bytes` (`NxGeneratedNxIr.cs`) are public, as is
    `NxIrMetadata.FunctionEntrypoints`, a `string[]` (`NxIrMetadata.cs:48`), so `Array.Exists` is
    valid.
  - The retained public surface of `NxRuntime` is `ValidateWorkspace`, `GenerateNxIr` and
    `ExplainNxIr`. The fix uses no evaluation API, which matches
    `docs/hir-interpreter-retirement.md:15`.
- **The native SDK is still consumed.** `BuildCore` and `GenerateNxIr(NxIrEmitOptions)` both call
  `NxNativeLibrary.EnsureLoaded()` and then native entrypoints (`nx_build_program_artifact`,
  `nx_codegen_nx_ir`). So each RID still has to load
  `runtimes/<rid>/native/<lib>` from the restored package and run native compilation and codegen.
  If the native library is missing or broken, the run throws and exits nonzero, so the job still
  fails as it should. The script still runs from a restored package (feed, then `dotnet add`, then
  `dotnet run --runtime <rid>`), not from repository sources.
- **The assertions match the existing end-to-end test.** The same contract is asserted in
  `bindings/dotnet/tests/NxLang.Sdk.Tests/NxEndToEndTests.cs:93-106` (identity, one `root` function
  entrypoint, a valid image). Passing an explicit `"main.nx"` file name makes the identity check
  deterministic: the default would be `input.nx`. The name has no path separators, so it should not
  vary on Windows.
- **It compiles as a top-level program.** The `dotnet new console` template enables implicit usings,
  so `Array` and `InvalidOperationException` resolve. Your linux-x64 run against the real CI package
  (`target/ci-fix/smoke-linux.txt`) confirms it compiles and runs: `Compiled main.nx: 268 bytes.`,
  then `Verified NX SDK package consumption for linux-x64.`
- **The workflow needs no change.** `.github/workflows/build.yml:131-165` still runs the script once
  per matrix entry (`linux-x64`, `osx-arm64`, `win-x64`) with the same parameters.
- **Specs and docs are consistent.** No active spec or doc requires the smoke test to evaluate `42`.
  `openspec/specs/package-release-automation/spec.md:74` only requires a smoke test. The `42`
  wording appears only in archived change reviews, which are historical.

## Not verified locally

I could not run the macOS and Windows jobs here. The code path is the same managed code on every
platform; only the native asset the package resolves differs. The linux run plus the unchanged
workflow matrix are the evidence for those two platforms.
