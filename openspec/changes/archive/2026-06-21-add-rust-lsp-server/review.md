# Review: add-rust-lsp-server

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/editor-language-service/spec.md,
specs/language-server-protocol/spec.md, specs/vscode-extension-publishing/spec.md

**Reviewed code:**
- `crates/nx-language-service/{Cargo.toml,src/lib.rs}`
- `crates/nx-lsp/{Cargo.toml,src/main.rs,src/lib.rs}`
- `src/vscode/src/{extension.ts,serverPath.ts}`
- `src/vscode/test/client/{serverPath.test.ts,packageMetadata.test.ts}`
- `src/vscode/scripts/{build-lsp.mjs,bundle-extension.mjs,check-package-assets.mjs,publish-vsix.mjs,publish-all.mjs}`
- `src/vscode/{package.json,tsconfig.json}`, `.github/workflows/vscode-extension.yml`
- `src/vscode/{README.md,CHANGELOG.md,TODO.md}` (doc updates)

**Verification performed:** `cargo test -p nx-language-service -p nx-lsp` (16 pass),
`cargo fmt --check` (clean), `pnpm test` in `src/vscode` (76 pass), inspected built `out/extension.js`.

**Fix verification performed:** `cargo fmt --check && cargo test -p nx-language-service -p nx-lsp`;
`NX_LSP_PLATFORM=linux-x64 NX_VSCODE_TARGET=linux-x64 pnpm run package:verify`.
Follow-up RF5 refinement verified with `cargo test -p nx-syntax -p nx-language-service -p nx-lsp`.

## Findings

### ✅ Verified - RF1 Extension is bundled as ESM, which the VS Code extension host likely cannot load
- **Severity:** High
- **Evidence:** `scripts/bundle-extension.mjs:13` builds with `--format=esm` and
  `package.json:35` sets `"type": "module"`, so `out/extension.js` is an ES module (confirmed: the
  built file contains top-level `import`/`export`). The VS Code extension host loads the `main`
  entry point with CommonJS `require()`. On the bundled Node target (`--target=node18`,
  engine `^1.90.0`), `require()` of an ESM file throws `ERR_REQUIRE_ESM`, so `activate()` never
  runs and *all* LSP features silently fail (grammar-only remains). This is the kind of failure the
  still-incomplete manual smoke test (task 7.4) would surface.
- **Recommendation:** Emit a CommonJS extension bundle (e.g. esbuild `--format=cjs`, output
  `out/extension.cjs` with `main` pointing at it, or add `out/package.json` containing
  `{"type":"commonjs"}`) so the extension host can `require()` it. Verify activation in a real VS
  Code instance before marking 7.4 done. Note the test suite relies on `"type":"module"` + `.js`
  ESM imports, so prefer scoping CommonJS to the emitted `out/` rather than removing the root
  `type`.
- **Fix:** Changed the esbuild bundle to CommonJS at `out/extension.cjs`, updated `package.json`
  `main` and package metadata tests to point at that file, and updated package asset verification
  and docs to require the CJS runtime.
- **Verification:** Confirmed `scripts/bundle-extension.mjs` now uses `--format=cjs` →
  `out/extension.cjs`; `package.json:36` `main` is `./out/extension.cjs`;
  `check-package-assets.mjs:11` and `packageMetadata.test.ts:16` require/assert the `.cjs` entry.
  Ran `pnpm run compile` and inspected the built `out/extension.cjs`: starts with `"use strict"`,
  0 ESM markers, 287 CJS markers (`require`/`module.exports`). `pnpm test` passes (76). The bundle
  format defect is resolved. (Note: the end-to-end VS Code activation smoke test, task 7.4, is still
  open and remains the right place to confirm runtime activation.)

### ✅ Verified - RF2 Protocol-level tests never exercise the real LSP request/notification handlers
- **Severity:** Medium
- **Evidence:** `crates/nx-lsp/src/lib.rs:455-573` tests only call free functions
  (`server_capabilities`, `snapshot_for_open_documents`, `diagnostics_for_open_documents`, and the
  `to_lsp_*` adapters). None of the `LanguageServer` trait methods (`initialize`, `did_open`,
  `did_change`, `did_close`, `document_symbol`, `hover`, `completion`) are invoked, and no
  tower-lsp `Server`/client round-trip exists. The riskiest logic — debounced publishing, the
  `version` staleness check in `publish_diagnostics_if_current` (`lib.rs:93-124`), the `did_open`
  `language_id != "nx"` filter (`lib.rs:157`), and `did_close` clearing — is therefore untested.
  Task 4.6 calls for "protocol-level tests that exercise initialize, diagnostics publish/clear,
  document symbols, hover, completion, and virtual URI analysis"; the current tests cover the
  projection logic but not the protocol surface.
- **Recommendation:** Add at least one test that drives the handlers (e.g. construct
  `NxLanguageServer`, call `did_open`/`did_change`/`did_close` and assert debounce + staleness
  behavior), or an end-to-end stdio test via `LspService`/`Server`. At minimum cover the
  stale-version drop path and `did_close` diagnostic clearing.
- **Fix:** Added handler-level tests for `initialize`, `didOpen`, `didChange`, `didClose`,
  diagnostics publish/clear, stale-version debounce suppression, and document symbol/hover/
  completion request delegation through an initialized `LspService`; also covered the non-`nx`
  `didOpen` ignore path.
- **Verification:** Confirmed five new `#[tokio::test]` cases in `crates/nx-lsp/src/lib.rs:762-`:
  `initialize_handler_advertises_mvp_surface`, `handlers_publish_current_diagnostics_and_clear_on_close`
  (drives `did_open`/`did_close` through a real `LspService` and reads published diagnostics off the
  client socket), `did_open_ignores_non_nx_documents`, `debounced_diagnostics_drop_stale_versions`
  (exercises the version staleness drop path via `did_open`+`did_change`), and
  `request_handlers_delegate_to_language_service` (calls the `document_symbol`/`hover`/`completion`
  handlers). The riskiest paths called out in the finding are now covered. `cargo test -p nx-lsp`
  passes (10 tests, up from 4).

### ✅ Verified - RF3 Publishing produces a single non-targeted VSIX that only works on the build runner's platform
- **Severity:** Medium
- **Evidence:** The `package` script is `vsce package --no-dependencies` with no `--target`
  (`package.json:45`), and the publish workflow runs only on `ubuntu-latest`, building one
  `linux-x64` server binary and publishing one universal VSIX
  (`.github/workflows/vscode-extension.yml:53-152`). At runtime `resolveServerPath` looks for
  `server/<platform>-<arch>/nx-lsp` (`src/serverPath.ts:39-51`), so macOS/Windows users installing
  the published extension find no binary and fall back to grammar-only. The
  `vscode-extension-publishing` spec requires "platform-specific package targets … each published
  native package SHALL pair the extension client with a server binary built for the corresponding
  target platform." Native targeting is documented (README:95-98) but not implemented in CI.
- **Recommendation:** Either implement a CI matrix that builds per-platform binaries and runs
  `vsce package --target <target>` per platform (with `package:check-assets` per target), or
  explicitly scope the first release to a single declared platform and record that deferral in the
  spec/design so the requirement and implementation agree. The design open question about the
  platform matrix is still unresolved.
- **Fix:** Added target-aware VSIX packaging via `NX_VSCODE_TARGET`, made asset checks and LSP
  binary copying target-aware, updated the publish workflow to build and publish `linux-x64`,
  `darwin-x64`, and `win32-x64` packages on matching runners, and recorded the initial x64 package
  matrix in `design.md`.
- **Verification:** Confirmed the new `scripts/package-vsix.mjs` passes `--target <NX_VSCODE_TARGET>`
  to `vsce package`; `build-lsp.mjs` and `check-package-assets.mjs` resolve the platform from
  `NX_LSP_PLATFORM`/`NX_VSCODE_TARGET` (including `win32-` → `nx-lsp.exe`); and the publish job in
  `.github/workflows/vscode-extension.yml:56-72` now runs a `linux-x64`/`darwin-x64`/`win32-x64`
  matrix on `ubuntu-latest`/`macos-13`/`windows-latest`, each setting both env vars so the packaged
  binary matches the target — satisfying the `vscode-extension-publishing` spec scenario. Locally ran
  `pnpm run build:lsp` then `pnpm run package:check-assets` for `linux-x64`: assets verified. Arm64
  remains an explicitly deferred follow-up in `design.md`, which is acceptable for this change.

### ✅ Verified - RF4 `nx-language-service` declares an unused `nx-diagnostics` dependency
- **Severity:** Low
- **Evidence:** `crates/nx-language-service/Cargo.toml:12` depends on `nx-diagnostics`, but the
  crate source never references `nx_diagnostics` (the diagnostic types it uses — `NxDiagnostic`,
  `NxDiagnosticLabel`, `NxSeverity` — are imported from `nx_api` in `src/lib.rs:3-6`).
- **Recommendation:** Remove the `nx-diagnostics` dependency, or use it directly if the API
  re-export is incidental.
- **Fix:** Removed the unused `nx-diagnostics` dependency from `crates/nx-language-service`.
- **Verification:** Confirmed `crates/nx-language-service/Cargo.toml` no longer lists
  `nx-diagnostics`; `cargo test -p nx-language-service` still builds and passes (13 tests).

### ✅ Verified - RF5 Unmapped/label-less diagnostics are attributed to an arbitrary document
- **Severity:** Low
- **Evidence:** `crates/nx-language-service/src/lib.rs:408-410` falls back to document index `0`
  when a diagnostic's primary label has no matching identity (`...or_else(|| self.documents.first().map(|_| 0))`).
  In a multi-document snapshot the document order comes from `documents.values()` over a
  `HashMap` (`crates/nx-lsp/src/lib.rs:319-325`), so "document 0" is non-deterministic and a
  label-less or unmatched diagnostic can be reported against an unrelated file. The VS Code MVP is
  effectively single-document so impact is limited today, but the language service is explicitly
  designed for multi-module workspaces.
- **Recommendation:** Drop diagnostics that cannot be mapped to a document in the snapshot (or
  collect them under a stable, well-defined "workspace" bucket) rather than defaulting to an
  arbitrary document.
- **Fix:** Removed the arbitrary document fallback, added zero-width document-start labels for
  document-scoped parser diagnostics such as `source-too-large`, and added a language-service
  diagnostic report that separates unmapped or truly label-less diagnostics into workspace
  diagnostics. The LSP now logs workspace diagnostics instead of publishing them against a random
  document, and regression tests cover document/workspace separation plus LSP log-message
  formatting.
- **Verification:** Confirmed `crates/nx-language-service/src/lib.rs:421-463` removed the
  `documents.first().map(|_| 0)` fallback; `project_diagnostic` now returns
  `ProjectedDiagnostic::Workspace` when there is no primary label or the primary label's identity is
  not in the snapshot, collected into the new `DiagnosticReport.workspace` bucket while
  `diagnostics()` still returns only document-mapped diagnostics. The LSP side
  (`workspace_diagnostic_message`/`to_lsp_message_type`) logs workspace diagnostics instead of
  publishing them, covered by `workspace_diagnostic_messages_include_code_and_unmapped_labels`.
  `cargo test -p nx-language-service -p nx-lsp` passes (13 + 10).

### ✅ Verified - RF6 Completion is triggered on the space character, producing noisy popups
- **Severity:** Low
- **Evidence:** `crates/nx-lsp/src/lib.rs:45` registers trigger characters `[":", "<", " "]`.
  Space-triggered completion fires the general keyword/declaration list after virtually every word,
  which most editors intentionally avoid. The space trigger appears intended to re-fire property
  completion inside tags, but it applies globally.
- **Recommendation:** Remove `" "` from `trigger_characters` (rely on `<` and explicit invocation
  for tag/property contexts), or gate space-triggered results to the in-tag property context only.
- **Fix:** Removed the space completion trigger and updated the capability test to assert only
  `":"` and `"<"` are advertised.
- **Verification:** Confirmed `crates/nx-lsp/src/lib.rs:45` now registers
  `trigger_characters: Some(vec![":", "<"])` with no space; capability tests pass.

## Questions
- RF1: Has the extension actually been loaded in a running VS Code instance yet? Task 7.4 is the
  only incomplete task and is exactly where an ESM activation failure would appear — was activation
  ever observed working?
  - **Status:** The activation bundle issue has a code fix, but the manual VS Code smoke test is
    still not completed in this fix pass.
- RF3: Is a single-platform (linux-x64) first release the intended scope, or should the platform
  matrix land before publishing? (This is one of the design's open questions.)
  - **Status:** The package matrix landed for Linux, macOS, and Windows x64 targets. Arm64 remains
    an explicit follow-up question in `design.md`.

## Summary
The language-service and LSP crates are clean, well-documented, and well-factored: the
protocol-independent core in `nx-language-service` is properly separated from the `tower-lsp`
adapter, identity/URI normalization is careful, and version-based staleness handling is sound.
Rust tests, formatting, and the VS Code grammar/client unit tests all pass. All six findings
(RF1–RF6) have been verified fixed: re-ran `cargo fmt --check` (clean) and
`cargo test -p nx-language-service -p nx-lsp` (13 + 10 pass), rebuilt the extension bundle and
confirmed it is now CommonJS (`out/extension.cjs`, 0 ESM markers), and ran `pnpm test` (76 pass) plus
`pnpm run build:lsp` + `pnpm run package:check-assets` (assets verified) in `src/vscode`.

One item remains outside this verification pass: task 7.4, the manual VS Code activation smoke test,
is still open. RF1 fixed the bundle-format root cause, but runtime activation in a real VS Code
instance should still be confirmed before archiving.
```
