## 1. Language service: build context and UTF-16 positions

- [x] 1.1 Measure whether library modules reach the language service today: write a
  `crates/nx-language-service` test that loads a small library from a `tempfile` directory into a
  `LibraryRegistry`, builds a `ProgramBuildContext`, calls `analyze_workspace_modules` for a
  document using a library component, and records in the change directory (`measured-libraries.md`)
  whether the library's `ModuleArtifact` appears and whether the document's tag resolves — this
  decides D1's fallback branch
- [x] 1.2 Add `build_context: ProgramBuildContext` to `WorkspaceSnapshot` with a
  `with_build_context` builder defaulting to `empty()`, and pass it from `diagnostic_report` and
  `build_workspace_declarations` (`crates/nx-language-service/src/lib.rs:296`, `lib.rs:767`);
  verify `cargo build --workspace` compiles `nx-lsp` unchanged
- [x] 1.3 Add tests for the four `editor-language-service` build-context scenarios — hover on a
  library component, tag and property completions from a library, diagnostics honoring library
  types, and no libraries without a context — using the 1.1 fixture; if 1.1 showed library
  artifacts absent, extend `build_workspace_declarations` to walk the context's library modules
  until these pass (`cargo test -p nx-language-service`)
- [x] 1.4 Rewrite `LineIndex::position_to_byte_offset` and `byte_offset_to_position` to count UTF-16
  code units per D2, clamping inside a surrogate pair and past the line end, and verify the four
  UTF-16 scenarios with tests over `let s = "😀" + name`, including byte offsets staying UTF-8
  (`cargo test -p nx-language-service`)
- [x] 1.5 Add `#[serde(rename_all = "camelCase")]` to every serialized language-service result type
  (`TextPosition`, `EditorRange`, `Hover`, `CompletionList`, `CompletionItem`, `EditorDiagnostic`,
  `RelatedLocation`, `DocumentDiagnostics`, `DiagnosticReport`, `WorkspaceDiagnostic`,
  `DocumentSymbol`) and verify with a serialization test that a `Hover` emits `startByte` and no
  `start_byte`
- [x] 1.6 Add a non-BMP hover test to `crates/nx-lsp` proving an LSP `Position` past an emoji
  resolves and the returned `Range` is UTF-16, and verify `cargo test --workspace` is green

## 2. Protocol package

- [x] 2.1 Create `packages/language-protocol` (ESM, `tsc` build, `exports` with `types`, `files`,
  `publishConfig.access: public`, no `private`) declaring `LanguageDocument`, `TextPosition`,
  `EditorRange`, the four request types, the four answer types, the reserved query-name union, the
  error classes (`UnsupportedQueryError`, `LanguageServiceHttpError`, timeout), and the
  `NxLanguageService` interface; verify `pnpm run build` emits `dist/index.d.ts` with every export
- [x] 2.2 Write `packages/language-protocol/README.md` documenting the document-set model, UTF-16
  encoding, each query, and the reserved names, with one request and one answer example per query;
  verify the examples type-check by compiling them in a `test/examples.test.ts`

## 3. Node SDK language snapshot

- [x] 3.1 Add `nx-language-service` to `bindings/node/native/Cargo.toml` and implement
  `NativeNxLanguageSnapshot` per D3 — constructor from `[{ uri, source, identity?, version? }]` and
  an optional `&NativeNxProgramBuildContext`, methods `hover`, `completions`, `diagnostics`,
  `documentSymbols` returning JSON, and `dispose` — mapping `SnapshotError` to a napi error whose
  message names the offending URI or identity; verify `pnpm run build:native` in `bindings/node`
- [x] 3.2 Add the `NxLanguageSnapshot` TypeScript wrapper in `bindings/node/src` (native declaration
  in `native.ts`, class in `index.ts`, results typed from `@nx-lang/language-protocol`, disposed
  checks via the existing `NxDisposedResourceError` path) and export it; verify `pnpm run build:ts`
- [x] 3.3 Add `bindings/node/test/language-snapshot.test.ts` covering the seven `sdk-node`
  scenarios — logical-URI construction, typed hover and `null`, completions, per-document
  diagnostics, build-context library visibility using a library written to a temp directory,
  invalid input errors, and dispose semantics — and verify `pnpm test` in `bindings/node`
- [x] 3.4 Add a parity test that serializes one result of each kind from a real snapshot and asserts
  its key set equals the keys the protocol types declare (derived from a checked-in fixture object
  typed against the protocol), so a serde rename fails here; verify it passes and that removing
  `rename_all` from one Rust type makes it fail
- [x] 3.5 Document `NxLanguageSnapshot` in `bindings/node/README.md` with a hover example and the
  build-context note; verify the README's example runs as a script against the built package

## 4. HTTP handler package

- [x] 4.1 Create `packages/language-http` with `createNxLanguageHandler(options)` per D5: path
  routing on the final segment, `POST`-only, JSON parsing with a `maxBodyBytes` limit, request
  validation naming the missing field, unsupported-query answers for reserved names, `onError`
  reporting, and a JSON 500 for service throws; verify with `node --test` cases for each status path
- [x] 4.2 Implement the content-keyed LRU of `NxLanguageSnapshot`s (default size 8) with an
  injectable snapshot factory, and verify with a test that byte-identical document sets construct
  one snapshot and a one-character change constructs another
- [x] 4.3 Implement `buildContext` pass-through and verify with a test that a handler built from a
  registry that loaded a temp-directory library answers hover on the library's component, and a
  handler without one does not
- [x] 4.4 Implement `prelude` per D5 — position shift in, range shift out, prelude-origin diagnostics
  without a range, trailing-newline normalization, columns untouched — as an exported
  `preludeOffsets`/`shiftRange` helper pair plus the handler wiring; verify the four prelude
  scenarios with tests, including a completion inside a prelude component's tag
- [x] 4.5 Implement `toNodeListener(handler)` and verify with a test that starts an `http` server on
  an ephemeral port and answers a hover through it identically to calling the handler directly
- [x] 4.6 Write `packages/language-http/README.md` with a plain-`http` mount, a Hono mount, and the
  statelessness and single-thread notes; verify the plain-`http` snippet is the one the fiddle
  server uses (task 7.3)

## 5. HTTP client package

- [x] 5.1 Create `packages/language-client` with `createHttpLanguageService({ baseUrl, fetch?,
  headers?, timeoutMs? })` implementing `NxLanguageService`: JSON `POST` per query, header hook
  applied per request, `AbortSignal` forwarded and combined with the timeout, non-2xx rejected as
  `LanguageServiceHttpError` with status and a 200-character body excerpt, unsupported-query bodies
  mapped to `UnsupportedQueryError`; verify with `node --test` cases using an injected `fetch`
- [x] 5.2 Add an end-to-end test that mounts the handler from task 4 on an ephemeral port and drives
  it through the client for all four queries, proving the two packages agree on the wire; verify it
  passes
- [x] 5.3 Write `packages/language-client/README.md` showing construction with an authorization
  header hook; verify the snippet compiles in the package's test build

## 6. Monaco integration package

- [x] 6.1 Create `packages/monaco` with `registerNxLanguage(monaco, options)` per D6: language and
  configuration from `@nx-lang/language`, Shiki highlighter with configurable themes defaulting to
  `github-light`/`github-dark`, `shikiToMonaco`, a per-`monaco` `WeakMap` registration returning an
  `IDisposable`; peer dependencies on `monaco-editor`, `shiki`, `@shikijs/monaco`; verify
  `pnpm run build` and a unit test with a fake `monaco` namespace proving two calls register the
  language once and dispose removes providers
- [x] 6.2 Implement the hover provider — one-based to zero-based conversion, `model.getValue()` at
  request time, the workspace callback with the single-model default (`model.uri.toString()`,
  `getVersionId()`), cancellation token to `AbortSignal`, markdown contents with the returned
  range, `null` on empty, `onError` on rejection — and verify with fake-namespace tests against an
  in-process fake `NxLanguageService` covering each scenario
- [x] 6.3 Implement the completion provider with trigger characters `<` and `:`, kind mapping for
  all six protocol kinds, word-range insertion, and the same cancellation and error behavior;
  verify with fake-namespace tests, including that a duplicate registration yields each item once
- [x] 6.4 Implement and export `toMonacoMarkers(diagnostics, owner)` with severity mapping, one-based
  conversion, zero-width widening, and omission of range-less diagnostics; verify with unit tests
  for each rule
- [x] 6.5 Write `packages/monaco/README.md` with a direct `monaco.editor.create` example and an
  `@monaco-editor/react` `beforeMount`/`onMount` example including a multi-document workspace
  callback; verify both snippets type-check in the package's test build

## 7. Fiddle adopts the packages

- [x] 7.1 Replace `sample-apps/drawnui-react/src/editor/nxLanguage.ts` with a call to
  `registerNxLanguage` in `NxEditor.tsx` using `createHttpLanguageService({ baseUrl: "/api/language" })`,
  a fixed model URI `nx://fiddle/fiddle.nx`, and `github-dark`; delete the file, remove
  `vscode-textmate` and `vscode-oniguruma`, add `shiki` and `@shikijs/monaco`; verify
  `pnpm run typecheck` and that the source pane is highlighted in `pnpm run dev:all`
- [x] 7.2 Move `server/compile.mjs`'s prefix arithmetic onto the exported prelude helpers from
  `@nx-lang/language-http` so compile and language share one shift implementation; verify
  `server/compile.test.mjs` still passes
- [x] 7.3 Mount the handler in `server/index.mjs` at `/api/language/*` via `toNodeListener`, with
  the catalog as `prelude`; add `server/language.test.mjs` proving hover on a catalog component
  answers in visitor coordinates and completions inside a catalog tag offer its properties; verify
  `pnpm test`
- [x] 7.4 Add the `/api/language` proxy beside `/api/compile` in `vite.config.ts` and its test in
  `vite.config.test.mjs`; verify `pnpm test`
- [x] 7.5 Verify the `drawnui-fiddle` scenarios by hand in `pnpm run dev:all`: hover a catalog tag
  shows a highlighted NX signature, hover a visitor declaration answers, completions inside a
  catalog tag list unsupplied properties, and stopping the compile server leaves editing and
  highlighting working; record the check in the change directory as `manual-check.md`
- [x] 7.6 Update `sample-apps/drawnui-react/README.md`: the pipeline diagram gains the language
  route, the layout table drops the bridge and names the packages, and the single-thread note covers
  both routes; verify the README's commands match `package.json`

## 8. Root pnpm workspace and package readiness

- [x] 8.1 Add a root `pnpm-workspace.yaml` listing `packages/*`, `runtime/typescript`,
  `bindings/node`, and `sample-apps/drawnui-react`, a root `package.json` with `packageManager`
  pinned to the pnpm version `src/vscode` uses, and a root `pnpm-lock.yaml`; delete the three
  `package-lock.json` files; convert the `file:` dependencies in `bindings/node` and the fiddle and
  every intra-package dependency under `packages/` to `workspace:*`; verify
  `pnpm install --frozen-lockfile && pnpm -r build` from the repository root succeeds and that
  `pnpm run --dir src/vscode typecheck` still resolves the nested workspace unchanged
- [x] 8.2 Fix any dependency pnpm's strict `node_modules` exposes as undeclared in the existing
  three packages, declaring each in the package that imports it; verify `pnpm -r test` passes from
  the root
- [x] 8.3 Add one `scripts/verify-package.mjs` at the repository root (`pnpm pack` to a temp dir,
  assert the packed manifest has no `workspace:` specifier left, install the tarball into a scratch
  project, import every export) wired as `pnpm run verify:package` in each new package, and a root
  script that runs all four;
  verify each passes and that a missing `dist` entry fails it
- [x] 8.4 Update `sample-apps/drawnui-react/Dockerfile` to copy the root manifests, `packages/`, and
  the existing directories, enable pnpm through corepack, run one
  `pnpm install --frozen-lockfile` at the root, build with `pnpm -r build`, and
  produce the runtime stage with `pnpm deploy --prod` for the fiddle; verify `docker build` from the
  repository root succeeds and `docker run` answers `POST /api/language/hover`
- [x] 8.5 Update `bindings/node/README.md`, the fiddle README, and `runtime/typescript` docs for the
  workspace bootstrap (corepack, one `pnpm install` at the root) and verify the documented commands
  work from a clean checkout
- [x] 8.6 Record the follow-ups in `specs/future.md`: publication pipeline changes for the four
  packages, `@nx-lang/ir-runtime`, and `@nx-lang/sdk-node` prebuilds; folding `src/vscode` into the
  root workspace with `@nx-lang/language` extracted to `packages/language`, the staging script
  deleted, and VSIX packaging verified under the root workspace; ReachMe's migration steps (adding
  the four directories to its workspace, Hono mount with its registry context, `nx-language` to
  `@nx-lang/language`, submodule removal once published); and NXE12/NXE13 as the condition for
  deleting the prelude option; verify the section links the three specs this change adds
