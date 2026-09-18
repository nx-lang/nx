# Review: add-nx-to-drawnui-fiddle

## Scope

**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md`, `verification.md`,
`specs/fiddle-nx-language/spec.md`, `specs/sdk-wasm/spec.md`,
`specs/package-release-automation/spec.md`, `specs/monaco-language-integration/spec.md`,
`specs/typescript-ir-runtime/spec.md`

**Reviewed code (NX repository, `~/src/nx`):** `crates/nx-codegen/src/{ir,lib,tests}.rs`,
`crates/nx-cli/src/main.rs`, `crates/nx-ffi/src/lib.rs`, `bindings/wasm/native/src/lib.rs`,
`bindings/node/native/src/lib.rs`, `bindings/wasm/src/{prelude,api}.ts`,
`bindings/wasm/test/{prelude,parity,sdk-wasm,trap}.test.ts`, `bindings/wasm/{package.json,README.md}`,
`runtime/typescript/{package.json,README.md}`, `packages/monaco/{package.json,README.md}`,
`packages/monaco/test/amd-namespace.test.ts`, `packages/language-http/package.json`,
`scripts/{workspace-packages,pack-packages,publish-packages,verify-package}.mjs`,
`sites/playground/src/compile/{catalog.ts,types.ts,catalog.test.mjs}`,
`.github/workflows/{build,release,package-publish}.yml`, `docs/deployment*.md`, `specs/future.md`,
`bindings/dotnet/tests/NxLang.Sdk.Tests/NxEndToEndTests.cs`, `src/vscode/scripts/package-language.mjs`

**Reviewed code (fiddle repository, `~/src/DrawnUi.FiddleEngine`):**
`nx/{generate-catalog.mjs,nx-packages.mjs,CATALOG.md}`, `nx/catalog/*`,
`nx/src/{main.ts,runtime.ts,values.ts,draw.tsx}`, `nx/test/{run.mjs,support.ts,runtime.test.ts,values.test.ts}`,
`nx/tsconfig.json`, `dev/build-nx-runtime.mjs`, `src/DrawnUi.Fiddle/wwwroot/{fiddle-nx.js,fiddle-react.js,fiddle-intellisense.js}`,
`src/DrawnUi.Fiddle/Languages/NxLanguage.cs`, `src/DrawnUi.Fiddle/FiddlePresetsNx.cs`,
`src/DrawnUi.Fiddle/ServiceCollectionExtensions.cs`, `src/Fiddle.DevHost/{wwwroot/index.html,Fiddle.DevHost.csproj,Properties/launchSettings.json}`,
`package.json`, `package-lock.json`, `.gitignore`, `AGENTS.md`, `README.md`, `docs/ADDING-A-LANGUAGE.md`

**Executed during review:** `node nx/generate-catalog.mjs` (output byte-identical to the committed
catalog — reproducibility scenario holds), `npm run test:nx -- --nx ../nx` (15/15 pass, including
trap recovery and all four presets), `pnpm exec vitest run test/prelude.test.ts` (6/6),
`pnpm -C packages/monaco test` (15/15, the 0.52 AMD-namespace test included),
`pnpm -C bindings/wasm run verify:package` (green), `pnpm pack` of `@nx-lang/sdk-wasm`
(664 KB, carries `dist/nx.wasm`, excludes `dist/nx-debug-trap.wasm`).

## Findings

### ✅ Verified - RF1 The new `specs/future.md` section is inserted mid-section and orphans two playground follow-ups
- **Severity:** Medium
- **Evidence:** `specs/future.md:740` adds `## The DrawnUI Fiddle: What add-nx-to-drawnui-fiddle Left
  For Later` between `### The catalog as a library artifact` (`:731`) and `### A static host, without
  the Node server` (`:780`). The last two `###` sections of the file (`:780` and
  `### Splitting the domain across services`, `:789`) belonged to `## Playground: What
  add-playground-site Left For Later` (`:704`); they now read as fiddle follow-ups, which they are
  not (they are about the playground's Node server and domain routing).
- **Recommendation:** Move the whole new `##` block to the end of the file (after `:797`), so the
  playground section keeps its own subsections.
- **Fix:** Moved the whole `## The DrawnUI Fiddle` block to the end of `specs/future.md`, after
  `### Splitting the domain across services`. The playground section keeps its own five subsections
  and the fiddle section owns its three.
- **Verification:** Confirmed. `specs/future.md` now runs `## Playground` (`:704`) with its five
  subsections (`:713`, `:722`, `:731`, `:740`, `:749`), then `## The DrawnUI Fiddle` (`:760`) with its
  three (`:770`, `:779`, `:790`). Each `###` sits under the `##` it belongs to.

### ✅ Verified - RF2 The release track publishes a different package set than the spec names, and silently un-publishes `@nx-lang/language-http`
- **Severity:** Medium
- **Evidence:** `specs/package-release-automation/spec.md` names five workspace packages
  (`sdk-wasm`, `ir-runtime`, `language-core`, `language-protocol`, `monaco`).
  `scripts/workspace-packages.mjs:16-47` instead publishes *every* non-private workspace member,
  which adds `@nx-lang/language-client`, and `packages/language-http/package.json` was changed in
  this change to `"private": true` with its `publishConfig`, `prepack` and `verify:package` removed.
  Neither the proposal, the design, nor a task mentions language-http or language-client;
  `docs/deployment-setup.md` and `docs/deployment.md` now document the six-package set as fact.
  `verification.md` says "seven packages packed", which does not match the six publishable members
  either.
- **Recommendation:** Decide deliberately and record it: either restore `@nx-lang/language-http` as
  publishable, or add a line to the proposal/spec saying it is withdrawn from the track (and why),
  and name `@nx-lang/language-client` in the spec's package list. Correct the count in
  `verification.md`.
- **Fix:** Recorded the decision rather than reversing it. `@nx-lang/language-http` depends on
  `@nx-lang/sdk-node`, which stays private, so a consumer could not install it: the proposal now says
  it is withdrawn from the track until the Node SDK is published, and the
  `package-release-automation` spec names `@nx-lang/language-client` in the publishable set and gains
  a scenario for a member a consumer could not install. `verification.md` now says six publishable
  workspace packages, which is what `pnpm -r run verify:package` covers (`src/vscode` is not a
  workspace member and packs on its own).
- **Verification:** Confirmed, and the stated reason is factual: `packages/language-http` depends on
  `@nx-lang/sdk-node`, which is `private: true`, so a consumer could not install it. The proposal
  records the withdrawal and its reason, the spec's package list names all six publishable members
  and gains the "a member a consumer could not install" scenario, and `verification.md` says six. I
  also checked the converse across the tree: no other publishable package depends on a private one
  (`language-client`, `language-core`, `monaco` and `sdk-wasm` reach only `language-protocol` /
  `language-core`), so the new scenario matches the workspace as it stands.

### ✅ Verified - RF3 `npm run runtime -- --out <dir>` writes the NX bundle to the wrong place and leaves the React runtime behind
- **Severity:** Medium
- **Evidence:** `package.json` (fiddle) defines `"runtime": "node dev/build-react-runtime.mjs && node
  dev/build-nx-runtime.mjs"`. npm appends `--` arguments to the end of the whole script string, so
  `--out X` reaches only the second command. The two scripts also read `--out` differently:
  `dev/build-react-runtime.mjs:21` treats it as `<host>/wwwroot/react`, `dev/build-nx-runtime.mjs:129`
  as `<host>/wwwroot/nx`. `npm run runtime -- --out other/wwwroot/react` therefore writes
  `nx-runtime.js` and `nx.wasm` into `.../react` and leaves the DrawnUi.React runtime in the dev
  host. The `fiddle-nx-language` spec scenario "Another host is targeted … both runtimes SHALL be
  written there and nowhere else" is not satisfied by any single command; `AGENTS.md` works around it
  by telling the reader to run the two scripts separately.
- **Recommendation:** Take one `--out <host>/wwwroot` in `npm run runtime` and have each build derive
  its own `react/` or `nx/` subdirectory from it, forwarding the flag to both commands (e.g. a small
  `dev/build-runtimes.mjs` that parses once and invokes both).
- **Fix:** Added `dev/build-runtimes.mjs`, which parses one `--out <host>/wwwroot`, derives
  `react/` and `nx/` from it, and forwards every other argument to both builds; `npm run runtime` now
  runs that. `AGENTS.md` and both build scripts' usage comments describe the single flag. Verified:
  `npm run runtime -- --out <scratch>/wwwroot --nx ../nx` wrote `react/drawnui-react.js` (plus
  canvaskit and fonts) and `nx/nx-runtime.js` plus `nx.wasm` under the scratch host, and touched
  nothing in the dev host.
- **Verification:** Confirmed by running it. `npm run runtime -- --out <scratch>/wwwroot --nx ../nx`
  wrote `<scratch>/wwwroot/react/{drawnui-react.js,canvaskit.wasm,types.json,fonts/}` and
  `<scratch>/wwwroot/nx/{nx-runtime.js,nx.wasm}`, and the dev host's own two runtimes kept their
  mtimes to the second — nothing was written there. `dev/build-runtimes.mjs` strips `--out` and its
  value and forwards everything else, so `--nx ../nx` reached both builds (the NX build reported
  "@nx-lang from /home/bret/src/nx"). The spec's "another host is targeted … and nowhere else"
  scenario is now satisfied by one command.

### ✅ Verified - RF4 `fiddle-nx.js` caches the runtime load against the wrong root when `configure` runs before the first `prepare(ROOT)`
- **Severity:** Medium
- **Evidence:** `src/DrawnUi.Fiddle/wwwroot/fiddle-nx.js:85-104`. `prepare(root)` sets the
  module-level `moduleRoot` and memoizes `ready`; `compiler()` calls `prepare(moduleRoot)` — that is,
  it can only ever pass the value `prepare` itself last stored, defaulting to `''`. The editor calls
  `configure()` (via `fiddleSetEditorLanguage`, `fiddle-react.js:478`) on the language switch, before
  the first `fiddleReactRun` reaches `language.prepare(ROOT)` (`fiddle-react.js:322`). So in a host
  where `ROOT` is not `''` — the documented purpose of the parameter, "a published app points this at
  its shared runtime copy" (`fiddle-react.js:10`) — the bundle is fetched from the page base and the
  memoized promise makes the later `prepare(ROOT)` a no-op. Harmless in the fiddle itself and in the
  player path (where `opts.root` is set before `prepare` is reached), so the dev-host verification
  could not have caught it.
- **Recommendation:** Have the registration learn the root from the engine rather than from itself —
  for example, keep `ROOT` in a getter `fiddle-react.js` exposes, or pass it to `configure()` — and
  throw if `prepare` is later called with a root that differs from the one already loaded.
- **Fix:** `fiddle-react.js` exposes `window.fiddleReactRuntimeRoot()`, and `fiddle-nx.js` reads it
  in `prepare` when the engine has not passed a root — which is the `configure()` path — instead of
  carrying its own default. `prepare` now records the root it loaded from and rejects a later call
  naming a different one, so a mismatch is an error rather than a silently wrong fetch.
  `docs/ADDING-A-LANGUAGE.md` documents the getter next to `prepare(root)`.
- **Verification:** Confirmed. `fiddle-react.js:162` exposes `window.fiddleReactRuntimeRoot()` over the
  engine's own `ROOT`, and `fiddle-nx.js:28-53` reads it when `prepare` is called without a root — the
  `configure()` path — records `loadedRoot`, and rejects a later call naming a different root.
  `compiler()` now builds the module URL from `loadedRoot` rather than from a variable only `prepare`
  could set. Both call sites agree in every path I traced: the editor passes `ROOT` (`''`) and the
  getter answers `''`; the player sets `ROOT = opts.root` before `prepare(ROOT)`, so the explicit
  argument and the getter are the same value. The failure path clears both `ready` and `loadedRoot`,
  so a failed load still retries. Documented in `docs/ADDING-A-LANGUAGE.md:129-131`.

### ✅ Verified - RF5 `publish-packages.mjs` loses its directory argument when `--version` is omitted
- **Severity:** Low
- **Evidence:** `scripts/publish-packages.mjs:233-240`. With no `--version`, `versionIndex` is `-1`,
  so the guard `index !== versionIndex + 1` excludes index `0` — the directory. `node
  scripts/publish-packages.mjs release-assets` and `node scripts/publish-packages.mjs dist
  --dry-run` both print the usage line and exit 2. Both workflow call sites pass `--version`, so it is
  latent, but the script's own usage string advertises the form that fails.
- **Recommendation:** `const dir = args.find((arg, index) => !arg.startsWith("--") && !(versionIndex
  >= 0 && index === versionIndex + 1));`
- **Fix:** `const dir = args.find((arg, index) => !arg.startsWith("--") && !(versionIndex >= 0 &&
  index === versionIndex + 1));`. Verified: `node scripts/publish-packages.mjs <dir> --dry-run` now
  reaches the tarball scan instead of printing the usage line.
- **Verification:** Confirmed by running all three argument forms against a directory holding one
  tarball: `<dir> --dry-run`, `--dry-run <dir>` and `<dir> --version 0.1.0 --dry-run` each reached the
  publish plan ("would publish @nx-lang/sdk-wasm@0.1.0 … dry run over 1 package(s)"). The first two
  printed the usage line and exited 2 before the fix.

### ✅ Verified - RF6 `verify-package.mjs` emits duplicate imports when a package has more than one asset export
- **Severity:** Low
- **Evidence:** `scripts/verify-package.mjs:130-134`. The probe is built by mapping over `assets` and
  emitting `import { statSync } from "node:fs"; import { createRequire } from "node:module";` inside
  *each* entry. Two asset subpaths in one package produce two top-level declarations of `statSync`
  and `createRequire`, and Node rejects the module with a SyntaxError before any entry point is
  checked — a verification failure that looks like a packaging failure. Only `@nx-lang/sdk-wasm` has
  an asset export today (`./nx.wasm`), so the current run is green (confirmed: "imported 1 entry
  point(s)").
- **Recommendation:** Emit the two imports once, ahead of the generated statements, and let each asset
  contribute only its `statSync(require.resolve(...))` line.
- **Fix:** The probe is built as a list of lines: `statSync` and `createRequire` are imported once
  ahead of everything, with one `packageRequire` instance, and each asset contributes only its
  `statSync(packageRequire.resolve(...))` line. Verified: `pnpm -C bindings/wasm run verify:package`
  is green ("imported 1 entry point(s)").
- **Verification:** Confirmed. `scripts/verify-package.mjs` now builds the probe as a line list with
  `statSync`, `createRequire` and one `packageRequire` emitted once ahead of everything, and each
  asset contributing only its `statSync(packageRequire.resolve(...))` line — so two asset subpaths can
  no longer redeclare either binding. `pnpm -C bindings/wasm run verify:package` re-run: green,
  "imported 1 entry point(s)".

### ✅ Verified - RF7 The fiddle pins the NX packages as `optionalDependencies` and the lockfile has recorded them as unresolvable
- **Severity:** Low
- **Evidence:** `package.json` (fiddle) lists `@nx-lang/{ir-runtime,language,monaco,sdk-wasm}` under
  `optionalDependencies`, diverging from design D10 ("pins … as devDependencies, mirroring how
  `drawnui-react` is consumed"). `package-lock.json:468-479` holds them as bare `{"optional": true}`
  stubs with no version, resolved URL or integrity, because they are not on the registry yet
  (task 2.6 is the one unchecked task). Consequences: `npm ci` installs none of them and the failure
  surfaces only as `nxPackageDir`'s "is not installed" error deep in the runtime build; and the lock
  must be refreshed once the packages publish. `AGENTS.md` records the intent to move them to
  `devDependencies` afterwards.
- **Recommendation:** Keep this as an explicit follow-up attached to task 2.6 — move the four to
  `devDependencies` and re-run `npm install` to re-lock — and add the same note to the fiddle's
  `AGENTS.md` release checklist so the temporary shape does not become permanent.
- **Fix:** Recorded as the reviewer recommends, since the move itself has to wait for 2.6. Added task
  2.7 (move the four pins to `devDependencies` and re-run `npm install` so the lock records resolved
  versions; verify `npm ci` installs them and `npm run runtime` builds without `--nx`), and the
  fiddle's `AGENTS.md` now says the lockfile holds them as unresolved optional stubs until then.
- **Verification:** Confirmed as a recorded follow-up, which is what the finding asked for. Task 2.7
  is present and unchecked, ordered after 2.6, and names both halves of the work (the
  `optionalDependencies` → `devDependencies` move and the `npm install` that replaces the
  `{"optional": true}` stubs) with its own verification step. The fiddle's `AGENTS.md:24-27` says the
  lockfile holds them unresolved until then.

### ✅ Verified - RF8 Warnings from a successful NX compile can never reach the visitor
- **Severity:** Low
- **Evidence:** `bindings/wasm/src/prelude.ts:104-111` returns `{ ir, diagnostics: [] }` on success,
  and the underlying SDK only surfaces diagnostics by throwing (`bindings/wasm/src/host.ts:63,92`).
  So `nx/src/runtime.ts:112-117` computes `warnings` from a list that is always empty whenever `ir`
  is non-null: the engine's warning channel (which TSX uses to print type complaints into the
  console panel, `fiddle-react.js:325-329`) is dead for NX. Squiggles still come from the language
  service, so nothing is lost to the visitor — but the code reads as though warnings flow, and the
  preset test asserts `result.warnings` is empty as if that were a check.
- **Recommendation:** Say so where it is decided — a line in `runtime.ts`'s `compile` noting that a
  successful build carries no diagnostics by construction — or drop the `warnings` computation on the
  success path.
- **Fix:** Said it where it is decided. `NxPreludeBuildResult.diagnostics` documents that the host
  reports diagnostics by throwing, so a build that returned IR reported nothing and carries no
  warnings; `nx/src/runtime.ts` computes `errors`/`warnings` only on the failure path and returns
  `warnings: []` on success with a comment pointing at the language service for what the author
  should still see.
- **Verification:** Confirmed. `NxPreludeBuildResult.diagnostics` in `bindings/wasm/src/prelude.ts`
  now documents that the host reports diagnostics by throwing, so a build that returned IR carries no
  warnings, and `nx/src/runtime.ts` computes `errors`/`warnings` only inside the `result.ir === null`
  branch and returns `warnings: []` on success with a comment pointing at the language service. The
  code no longer reads as though warnings flow.

### ✅ Verified - RF9 Diagnostics failures are swallowed twice with no trace
- **Severity:** Low
- **Evidence:** `fiddle-nx.js:126-132` returns `[]` for any error from `compiler()` or
  `diagnostics()`, and `NxLanguage.DiagnosticsAsync` (`src/DrawnUi.Fiddle/Languages/NxLanguage.cs:42-49`)
  catches again and returns empty. A failed runtime load, a crashed host or a language-service fault
  is therefore indistinguishable from a clean snippet: no squiggles, no console line, and the
  HotReload gate opens as if the code were fine.
- **Recommendation:** `console.warn('nx: diagnostics failed', e)` in the `fiddle-nx.js` catch. The C#
  catch can stay silent once the JS side says something.
- **Fix:** `console.warn('nx: diagnostics failed', e)` in the `fiddle-nx.js` catch. The C# catch stays
  silent.
- **Verification:** Confirmed. `fiddle-nx.js:86-91` warns before returning `[]`, with a comment saying
  why silence would be misread. The C# catch in `NxLanguage.DiagnosticsAsync` stays silent, as
  recommended.

### 🔴 Open - RF10 The dev-host port change from 5040 to 5041 is unrelated to this change
- **Severity:** Low
- **Evidence:** `src/Fiddle.DevHost/Properties/launchSettings.json:8`,
  `src/Fiddle.DevHost/Fiddle.DevHost.csproj:5`, `AGENTS.md:21`, `docs/ADDING-A-LANGUAGE.md:177`.
  Nothing in the proposal, design or tasks calls for it, and nothing in the NX integration depends on
  the port. It looks like a local port conflict being carried into a shared upstream repository.
- **Recommendation:** Split it into its own commit with its own reason, or revert it and set the port
  locally, so the NX change stays reviewable as one thing.
- **Status:** Left as it is, and worth knowing why: 5040 is genuinely occupied on this machine — a
  second dev host asked for it during verification and failed with `Failed to bind to address
  http://127.0.0.1:5040: address already in use`, from a listener `ss` does not show (a Windows-side
  process behind WSL2's localhost forwarding). So the bump is not arbitrary, but it is still local to
  one machine and unrelated to NX. Nothing in either repository is committed yet, so the reviewer's
  first recommendation is still available: commit the four port lines
  (`launchSettings.json`, `Fiddle.DevHost.csproj`, `AGENTS.md`, `docs/ADDING-A-LANGUAGE.md`)
  separately from the NX change, or drop them and run the host with
  `dotnet run --project src/Fiddle.DevHost --urls http://localhost:5041` locally.
- **Verification:** Stays open, correctly — nothing was claimed fixed. The status note's factual claim
  checks out: binding `127.0.0.1:5040` fails with `EADDRINUSE` on this machine, so the bump is not
  arbitrary. It is still a local condition carried into a shared repository, and since neither
  repository has committed yet, the split-commit-or-drop decision is still available at commit time.
  Nothing here blocks the change.

### ✅ Verified - RF11 The per-component `content` property in `catalog-meta.json` is generated but never read
- **Severity:** Low
- **Evidence:** `nx/catalog/catalog-meta.json` records `components[tag].content` (18 tags carry
  `"Children"`, 16 carry `null`), and `nx/src/values.ts:68` exports it — but `coerceProps`
  (`values.ts:122-131`) and `childrenOf` (`values.ts:134-140`) both use the single global
  `meta.contentProperty` instead, and nothing else reads `components[type].content`. Two mechanisms
  exist for one decision; they agree only because every content property happens to be `Children`
  today.
- **Recommendation:** Pick one. Either drive `childrenOf`/`coerceProps` from
  `components[type].content` (which is what makes the field worth generating), or stop emitting it
  and keep the global.
- **Fix:** Picked the per-component field. `childrenOf` and `coerceProps` take the control's own
  content property, `draw.tsx` passes `components[type].content` (it already had the lookup in hand),
  and the global `contentProperty` is gone from the generated metadata and from `values.ts`. Verified:
  the catalog still regenerates byte-identically across two runs, and `npm run test:nx -- --nx ../nx`
  is 15/15.
- **Verification:** Confirmed, and the stronger of the two options was taken. `catalog-meta.json`'s
  keys are now `version, nodeRoot, unions, records, components` — the global `contentProperty` is gone
  — `coerceProps(value, content)` and `childrenOf(value, content)` take the control's own property,
  and `draw.tsx:106-118` passes `component.content` from the lookup it already had, with the
  `component === undefined` guard still routing to the authored-component and placeholder paths.
  Re-ran the generator: the catalog regenerates byte-identically. `npm run test:nx -- --nx ../nx`
  (run as part of the NX runtime build): 15/15.

### ✅ Verified - RF12 `NxLanguage` re-declares the `IJSRuntime` its base class already holds
- **Severity:** Low
- **Evidence:** `src/DrawnUi.Fiddle/Languages/NxLanguage.cs:22-24` adds `private readonly IJSRuntime
  _js;` and passes the same instance to `base(js)`, which stores its own `private readonly IJSRuntime
  _js` (`ReactSurfaceLanguage.cs:17-19`). Two fields, one object; a third React-surface language with
  diagnostics would copy the pattern.
- **Recommendation:** Expose it once on the base — `protected IJSRuntime Js { get; }` — and delete the
  subclass field.
- **Fix:** `ReactSurfaceLanguage` exposes `protected IJSRuntime Js { get; }` and uses it for its own
  two calls; `NxLanguage` drops its field and calls through `Js`.
- **Verification:** Confirmed. `ReactSurfaceLanguage` exposes `protected IJSRuntime Js { get; }`
  (`:18`, assigned at `:20`) and uses it for `RunAsync` and `ShareArtifactAsync`; `NxLanguage` holds no
  field and calls through `Js`. No `_js` remains anywhere under `src/DrawnUi.Fiddle/Languages/`.
  `dotnet build src/Fiddle.DevHost`: succeeded, 0 warnings, 0 errors.

## Questions

- The design's own open question is still open: the private drawfiddle.com backend's maximum share
  artifact size. The largest preset's artifact measures 1,073 KB (`verification.md`, and the table in
  the fiddle's `README.md`), so this decides whether NX shares can be opened publicly before the
  catalog-as-library follow-up. Who owns that answer, and when?
- Was making `@nx-lang/language-http` private a deliberate withdrawal from publishing, and is
  publishing `@nx-lang/language-client` intended? (see RF2)
- Task 2.6 (cut and publish a release) is the only unchecked task, and everything downstream of it —
  the fiddle's registry build, the `optionalDependencies` → `devDependencies` move (RF7), and the
  first-publish-by-hand procedure now documented in `docs/deployment-setup.md` — is waiting on it. Is
  the plan to land both repositories first and release after, or to release before merging the fiddle
  side?

## Summary

- The implementation matches the proposal, the design and the specs closely, and it is unusually well
  covered for a two-repository change: the catalog regenerates byte-identically, every preset is
  compiled against the catalog in the build, trap recovery is tested against a real trapping module,
  the Monaco integration is tested against a 0.52-shaped AMD namespace, and the packaging path is
  verified by packing and installing into a scratch project. Everything I re-ran during this review
  was green.
- The findings are all in the seams rather than the core: one documentation-structure mistake (RF1),
  one undeclared scope change in the publishing set (RF2), two build/host-plumbing bugs that the
  dev-host verification could not see because the fiddle itself uses the default values (RF3, RF4),
  and eight small latent or tidiness items. Nothing here blocks the change; RF1–RF4 are worth fixing
  before it lands.
- The one substantive risk the change carries forward is the share-artifact size (~0.9–1.1 MB per
  share), which is recorded honestly in the README, the design's open question and `specs/future.md`,
  and depends on an answer from outside both repositories.

## Verification pass — 2026-09-14 16:12

- All eleven findings marked fixed verify: RF1–RF9, RF11 and RF12 are now ✅ Verified. RF10 stays
  🔴 Open by the author's own decision, with an accurate status note; it blocks nothing.
- Four fixes were checked by running them rather than by reading: `npm run runtime -- --out
  <scratch>/wwwroot --nx ../nx` wrote both runtimes under the scratch host and left the dev host's
  files untouched (RF3); all three argument forms of `publish-packages.mjs` now reach the publish plan
  (RF5); `pnpm -C bindings/wasm run verify:package` is green (RF6); `dotnet build src/Fiddle.DevHost`
  succeeds with no warnings (RF12). The catalog still regenerates byte-identically and the fiddle's
  NX suite is 15/15 after the metadata change (RF11).
- Two fixes chose to record a decision rather than reverse one (RF2, RF7), which is the right call in
  both cases: `@nx-lang/language-http` genuinely cannot be published while `@nx-lang/sdk-node` is
  private, and the `optionalDependencies` move genuinely cannot happen before task 2.6. Both are now
  written where someone will find them — the proposal and the release spec for one, task 2.7 and the
  fiddle's `AGENTS.md` for the other.
- No new findings. The open questions from the review pass are unchanged: the backend's artifact
  limit, and the sequencing of task 2.6 against merging the two repositories.
