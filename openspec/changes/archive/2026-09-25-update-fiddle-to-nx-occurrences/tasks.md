Paths without a repository prefix are in `~/src/DrawnUi.FiddleEngine`; `nx:` marks this repository.
Until section 8, every fiddle command runs side by side (`--nx ../nx`). Fiddle commits are local to
`nx-occurrences` until 8.6, and NX commits are local to `refine-sequences-and-optionals` until 8.1.

## 1. Baseline

- [x] 1.1 In the fiddle, `git fetch origin` and create `nx-occurrences` from `origin/main`. Run `npm install`, because `node_modules` still holds drawnui-react preview.9. Verify `git status` is clean, `git merge-base --is-ancestor origin/main HEAD` succeeds, and `node_modules/drawnui-react` is the pinned version.
- [x] 1.2 Compare `npm view drawnui-react version` with the pin. If npm is ahead, move the pin, `npm install`, `npm run catalog` (still 0.3.0) and commit on its own. Verify `npm run test:nx` passes on 0.3.0 and `catalog-meta.json` records the pinned version.
- [x] 1.3 While `node_modules` still holds 0.3.0, write a schema-4 share string for the Cards preset to `nx/test/fixtures/schema4-cards.txt`, and its source beside it, the way the runtime tests build shares. Record how it was made in `fixtures/README.md`. Verify its image's schema cell reads 4. (It was Welcome at first. Welcome's source compiles unchanged under 0.4, so 4.3 could not show migration errors. Cards was rebuilt in a 0.3.0 worktree at `58ca16c`.)
- [x] 1.4 Build this checkout (`nx:` `pnpm install && pnpm -r build`, plus the wasm SDK build so `bindings/wasm` holds a fresh `nx.wasm`). Verify `nx:` `pnpm -r test` is green, so a later fiddle failure is not a stale NX build.
- [x] 1.5 Run `npm run test:nx -- --nx ../nx` and record which failures are expected: catalog spelling, presets, the stale schema-4 catalog image. Verify nothing fails for another reason, or note it under section 7.

## 2. Catalog in the new spelling

- [x] 2.1 Port the `occurrence-cardinality` changes from `nx:sites/playground/scripts/generate-catalog.mjs` (`git diff v0.3.0..HEAD` there) into `nx/generate-catalog.mjs`: `x?: T` for props and record fields, `+` for arrays and `TItem`, `content Children?: DrawnNode+`, and the unparenthesized optional `ItemTemplate`. Rewrite the generated header comment, which still says nulls are dropped. Verify `catalog.test.ts`, which reads the generated catalog back, asserts `MaxLines?: float64`, `Children?: DrawnNode+`, `ItemsSource?: TItem+` and `ItemTemplate?: <function Item:TItem Index:int />: DrawnNode`.
- [x] 2.2 Regenerate with `npm run catalog -- --nx ../nx`. Verify `drawnui.nx` has no `[]` and no type-slot `?`, `drawnui.nxir` is schema 5, and a second run gives a byte-identical result.
- [x] 2.3 Update `nx/CATALOG.md`: the new spellings, the versions it is generated from (it still names preview.9), and the playground comparison. Verify `grep -n 'preview.9\|\[\]?' nx/CATALOG.md` finds nothing stale.
- [x] 2.4 Commit, noting the catalog image's size before and after.

## 3. Presets, tests and docs

- [x] 3.1 Rewrite the NX presets in `src/DrawnUi.Fiddle/FiddlePresetsNx.cs`: `content Children:DrawnNode+`, `colors: string+`, and the ternary as `if … else`. Change nothing else unless the compiler demands it. Verify every preset compiles in `runtime.test.ts` and still dispatches where it did.
- [x] 3.2 Rewrite the NX sources embedded in `nx/test/*.test.ts` and the examples in `README-NX.md` in the new spelling. Verify `readme.test.ts` and `instances.test.ts` pass.
- [x] 3.3 Commit.

## 4. Renderer and shares

- [x] 4.1 Add `values.test.ts` and `runtime.test.ts` cases where an `if` with no `else` that took no branch, and an optional component prop the caller left out, reach a control property and a content property. Verify the control gets no such property and no `null`, `undefined` or `[]`, as "Unset properties keep DrawnUI's defaults" requires. Fix `values.ts` if it fails.
- [x] 4.2 Rename or reword what is left of `null` in `nx/src/*.ts` comments and types where it now describes NX's empty value rather than JavaScript's `null`. Verify with `tsc` through `npm run test:nx -- --nx ../nx`.
- [x] 4.3 Add a runtime test that mounts the schema-4 fixture from 1.3. Verify it draws the failure label naming schema 4 and schema 5, creates no compiler host, and that the source stored beside it compiles to `L{line}:` errors, not a crash.
- [x] 4.4 Verify every preset's share module is still under the 128 KB budget. Record the largest in the commit message, then commit.

## 5. Docs and the whole suite

- [x] 5.1 Update `README.md` and `AGENTS.md` wherever they name 0.3.0 behaviour or spelling. (`README-NX.md` gained the schema-mismatch paragraph and the new share sizes. `AGENTS.md`'s "pinned at `0.3.0`" is still true until the repin, so 8.3 changes it.) Verify `grep -rn '\[\]' README*.md AGENTS.md docs/` finds no NX type spelling.
- [x] 5.2 Run `npm run runtime -- --nx ../nx` and `npm run test:nx -- --nx ../nx`. Verify both pass and the summary line names the checkout.
- [x] 5.3 Browser run, headless through `window.fiddle` (dev host on 5041, backend on 5299). Verify:
  - every NX preset draws;
  - Welcome and Controls answer taps;
  - completion and hover show the `?`-on-name spelling for a catalog property;
  - a share round-trips through `/app#id=` and plays in `/p/` without the compiler module;
  - the schema-4 fixture posted as a share shows the mismatch label in `/p/` and its source in the editor.

## 6. NX playground on the published drawnui-react

Paths in this section are under `nx:sites/playground` unless they say otherwise.

- [x] 6.1 Before anything moves, take a headless screenshot of every gallery entry and of one editor view on the current vendored preview.4, saved in the scratchpad (Playwright from the npx cache; port 5174 is taken). Verify there is one screenshot per entry in `src/examples/examples.json`.
- [x] 6.2 Read `applyProps` in `src/react/reconciler.ts` (it was `registry.ts` at preview.4) at upstream tag `v<pin>` (`git -C ~/src/DrawnUi.React fetch --tags`, then `git show`) and note everything it does beyond assigning props by name. Verify the note lists each behaviour with where it will be reproduced, or says there is none.
- [x] 6.3 Add `"drawnui-react"` to `package.json`, pinned exactly to the fiddle's version, and run `pnpm install` at the repository root. Verify `pnpm install --frozen-lockfile` passes, and check whether `playwright-core` was installed or warned about; if it was, handle it as design.md says.
- [x] 6.4 Port the fiddle's `nx/generate-catalog.mjs`, as finished in section 2, into `scripts/generate-catalog.mjs`, keeping the playground's output paths, its header, and `catalog-meta.json` with a `version` field. Run `pnpm run generate-catalog` twice. Verify:
  - the output is byte-identical between runs;
  - a `diff` against the fiddle's `nx/catalog/drawnui.nx` shows nothing beyond the header, or each remaining difference is recorded in both `CATALOG.md` files.
- [x] 6.5 Add a playground test that fails when `catalog/catalog-meta.json` records a version other than the pinned `drawnui-react`, naming both versions and `pnpm run generate-catalog`. Verify it fails with the pin temporarily changed and passes as committed.
- [x] 6.6 Move every DrawnUI import to `drawnui-react` or `drawnui-react/core`: `drawnui-runtime.ts`, `main.tsx`, `editor/*`, `gallery/*` and `render/*`. Rewrite `render/materialize.ts` to construct controls from `drawnui-react/core` by tag name, and to apply props as 6.2 found. Verify `pnpm run typecheck` passes and `materialize.test.mjs` covers each behaviour 6.2 listed.
- [x] 6.7 Delete `src/drawnui/` and its entries in `tsconfig.json`. Verify `grep -rn "drawnui/" src scripts` finds no vendored import and `pnpm run build` succeeds.
- [x] 6.8 Rework `scripts/sync-drawnui.mjs`:
  - stop copying `src`;
  - take `public/*` and `reference/demo-pages` from tag `v<pin>` with `git archive`;
  - fail naming `git fetch --tags` when the tag is missing;
  - write `docs/UPSTREAM.md` with the tag and commit.

  Run it. Verify the upstream checkout's `HEAD` and `git status` are unchanged, and review the asset diff.
- [x] 6.9 Run `pnpm test`, which includes `check-examples`. Fix any example the new DrawnUI or catalog breaks, with the smallest edit. Name any demo page new since preview.4 in the site's documentation, with why it has no NX example. Verify `pnpm test` passes.
- [x] 6.10 Update the docs and build files:
  - `README.md`: dependencies, the sync, and the vendored-tree rows;
  - `docs/CATALOG.md`: derivation from the package, and "Edits to the vendored DrawnUI source" becomes the no-patch rule;
  - the Dockerfile, if its install step needs it.

  Verify `grep -rni vendored README.md docs/` finds only asset provenance, and the Docker image builds (without Docker: `pnpm --filter @nx-lang/playground build` from a clean install).
- [x] 6.11 Browser run against the built site (`PORT=5199 node server/index.mjs`, which is what 6.1 shot, so the two compare like for like). Verify:
  - every gallery entry draws, and differs from its 6.1 screenshot only where an upstream change explains it;
  - a templated list scrolls and binds its cells;
  - taps dispatch;
  - completion and hover in the editor show the regenerated catalog.
- [x] 6.12 In the fiddle, reduce `nx/CATALOG.md`'s "Differences from the NX playground's catalog" to what 6.4 left, and commit. In this repository, commit the playground work as one commit. Verify both repositories' `git status` show only unrelated files.

## 7. NX issues found along the way

- [x] 7.1 For each NX defect sections 1–6 turned up, apply design.md's "fix now" test. Fix the qualifying ones on `nx:` `refine-sequences-and-optionals`, each with a test in this repository. Verify `nx:` `cargo test --workspace` and `pnpm -r test` stay green and the fiddle's or the playground's failure is gone (the fiddle's through `--nx ../nx`).
- [x] 7.2 Record each deferred defect in `nx:specs/future.md` with the repro, the code location and the workaround, and put the workaround in the fiddle or the playground with a comment naming the entry's heading. Verify each workaround's comment and its entry match.
- [x] 7.3 If none were found, say so in this task's checkbox note. Otherwise commit the NX fixes, and verify `nx:` `git status` shows only this change's files.
  - Fixed in `fa04fa9`, each with a test:
    - a property completion's detail lost the `?` (now `name?: T`, as hover shows it);
    - a `name: T?` declaration was also reported as required at every use.
  - Also in `fa04fa9`: the playground test that still used the removed ternary, and rustfmt and clippy drift.
  - Deferred to `specs/future.md` ("An old `T[]` property is reported twice"). No workaround was needed, so the fiddle has no comment for it.

## 8. Release and repin (each step on the user's go-ahead)

- [x] 8.1 Get the NX branch onto `main` (push `refine-sequences-and-optionals`, open and merge its PR). Verify `origin/main` contains `6f56e76`, the section 6 playground commit and the section 7 fixes, and that nxlang.org/playground deploys and draws its gallery.
- [x] 8.2 Tag `v0.4.0` per `nx:docs/deployment.md` § "Publish A Package Release", verify the draft release's tarballs, manifest and checksums, and publish it. Verify `npm view @nx-lang/sdk-wasm@0.4.0 version` answers and NuGet lists `NxLang.Sdk` 0.4.0.
- [x] 8.3 In the fiddle, merge `origin/main` again, repin the four `@nx-lang/*` packages to 0.4.0 (and `AGENTS.md`'s pin sentence), and run `npm install --prefer-online`. Verify `npm run catalog` without `--nx` reproduces `drawnui.nxir` byte for byte.
- [x] 8.4 Run `npm run runtime` and `npm run test:nx` without `--nx`. Verify both pass; the one expected skip is the checkout-only debug-trap test.
- [x] 8.5 Commit the repin.
- [x] 8.6 Push `nx-occurrences` and open its PR against `DrawnUi/DrawnUi.FiddleEngine` `main`. The description summarizes the spelling change, the catalog, the preset edits, and the note that deploying it retires existing NX share links, which then show the schema-mismatch label. Verify the PR exists and CI, if any, is green.

## 9. Wrap up

- [x] 9.1 Sync `nx:openspec/specs` and archive the change. Verify `openspec validate --all --strict` passes.
