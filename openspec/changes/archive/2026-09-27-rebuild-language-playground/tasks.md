Starts after `launch-nxlang-website` is archived: its static hosting, Worker routing, header and
code-block check are assumed.

## 1. Evaluate to annotated NX text in Rust and wasm

- [x] 1.1 Move `crates/nx-cli/src/format.rs` and its tests to `crates/nx-api/src/nx_text.rs`, export `format_nx_text`, and point the CLI at it. Verify `cargo test -p nx-api -p nx-cli` passes with no change to any CLI expected output.
- [x] 1.2 Make the formatter's walk record a node per value as design.md describes (range in UTF-16 offsets, parent, role, name, count), with `format_nx_text` discarding them. Verify unit tests for a one-line record, a multi-line record, a sequence of records, a constant case, `{}`, and a string holding a non-BMP character before a node, each asserting the nodes' ranges against the text.
- [x] 1.3 Add `eval_program_artifact_nx_text` to `nx-api`, returning `NxValueText` with each node's `type` and `declaration` filled in from the artifact's resolved program, and reporting a missing `root`, runtime errors (with spans) and unspellable values (`nx-text-unspellable`) as diagnostics. Add tests for every scenario in the sdk-wasm delta, including declared property types and optionality, a sequence's `User*` and `object*` types, and a prelude type with no declaration. Verify they pass, and that `nxlang run` on each of those sources prints the same text the function returns.
- [x] 1.4 Add `nx_wasm_program_evaluate_nx` to `bindings/wasm/native` and raise `ABI_VERSION` to 3. Add `evaluateNx()` and the `NxValueText` and `NxValueNode` types to `bindings/wasm/src/host.ts` and `api.ts`, with doc comments. Verify the new unit tests in `sdk-wasm.test.ts`, one per scenario in the sdk-wasm delta, pass under Node.
- [x] 1.5 Extend `bindings/wasm/test/parity.test.ts` to compare `evaluateNx().text` with `nxlang run` for every corpus source with a `root`. Record the `nx.wasm` size before and after in the PR. Verify the parity suite passes in `pnpm -r test`.
- [x] 1.6 Update `bindings/wasm/README.md` with `evaluateNx()`, the node shape and the ABI version. Verify the example in it runs under Node.

## 2. The value viewer: `@nx-lang/value-view`

- [x] 2.1 Create `packages/value-view` on the pattern of `packages/monaco`: `package.json` (public, `shiki` and `@nx-lang/language` as peer dependencies, `NxValueText` declared structurally, so a host needs no compiler package to show a value), `tsconfig.json`, `src/index.ts` defining `<nx-value>` with a shadow root, and `verify:package`. Verify `pnpm install`, `pnpm --filter @nx-lang/value-view build` and `verify:package` pass.
- [x] 2.2 Render the text highlighted through Shiki with the published grammar, light and dark through dual themes, with the `highlighter` property to share one. Verify DOM tests (jsdom, under `node --test`) for "The value reads as NX": exact text content, token colors for a record, a supplied highlighter used without loading another, and dark colors.
- [x] 2.3 Add folding from the nodes: toggles on multi-line record, property and sequence nodes, a folded node's first line and ellipsis, and unfolding from either. Verify DOM tests for every scenario in "Long values fold".
- [x] 2.4 Add hover and keyboard focus: the innermost node, the `describe` hook, the default descriptions, and the markdown subset (fenced `nx` highlighted, paragraphs, inline code). Verify DOM tests for every scenario in "Hover explains a value".
- [x] 2.5 Add `nx-value-navigate`, the `truncated` notice, the `stale` style and badge, and the copy control. Verify DOM tests for every scenario in "The element reports where a value is declared" and "The element marks cut, stale and copied values", and the "Host styles don't reach in" scenario.
- [x] 2.6 Write `packages/value-view/README.md`: what it shows, a plain-page example, a React example, the properties, the hook and the event. Verify a test page built from the plain-page example shows a value in Playwright.
- [x] 2.7 Add `@nx-lang/value-view` to the package lists in `docs/deployment.md` and `docs/deployment-setup.md`. Verify `node scripts/pack-packages.mjs` packs it. Note in the PR that its first npm publish needs the token step `docs/deployment-setup.md` describes, before its trusted publisher can be added.

## 3. Remove DrawnUI from the playground

- [x] 3.1 Delete:
  - `catalog/`, `reference/`, `docs/`;
  - `public/{shaders,anims,lottie,fonts,images}`;
  - `src/render/`, `src/gallery/`, `src/compile/catalog.ts`, `src/drawnui-runtime.ts`;
  - the DrawnUI scripts (`sync-drawnui`, `generate-catalog`, `expand-components`, `emit-example-ir`, `compile-example`) and their tests;
  - the DrawnUI examples;
  - the `virtual:nx-catalog-artifact` plugin in `vite.config.ts`.

  Drop `drawnui-react`, `canvaskit-wasm`, `react-reconciler` and `@nx-lang/ir-runtime` from `package.json`. Verify `pnpm install` succeeds and `git grep -in "drawnui\|canvaskit\|catalog" sites/playground` finds only the fiddle link.
- [x] 3.2 Remove the fonts/images rule from `_headers`, and any font preloads from `index.html`. Verify `pnpm --filter @nx-lang/playground typecheck` passes, which by now fails only where later sections have not rewritten the code yet. Note those places.

## 4. Move the playground to `/play`

- [x] 4.1 In `sites/playground`, change the prefix from `/playground` to `/play` in `wrangler.jsonc` (both routes), `base.mjs`, `vite.config.ts` (the `dist/play` output), `_headers`, `worker/index.mjs`, `src/paths.ts`, and every test that names it. Verify `git grep -n "/playground" sites/playground` finds nothing, and that the worker and route tests pass.
- [x] 4.2 In `sites/website`, point `SiteLinks.astro`, the landing page's actions and link card, and the tutorials at `/play`, and change the link validator's exclusion in `astro.config.mjs` to `/play` and `/play/**`. Verify the website build passes its link check, and `git grep -n "/playground" sites/website` finds nothing.
- [x] 4.3 Update `SITE_URL` and the environment URL in `.github/workflows/deploy-playground.yml`, the route table, smoke commands and rollback note in `docs/deployment.md` (the rollback caveat from design.md's Migration Plan), and the routes and checks in `docs/deployment-setup.md`. Verify `git grep -n "nxlang.org/playground" -- ":!openspec"` finds nothing.

## 5. Evaluate in the worker

- [x] 5.1 Change the worker protocol (`src/worker/protocol.ts`, `session.ts`, `nx.worker.ts`) and the compile seam (`src/compile/*`) from compile-to-IR to evaluate-to-annotated-text, with the outcome union and the 100,000-character cap from design.md, which also cuts the nodes. Keep the deadline and recovery code. Update `session.test.mjs`, `worker.test.mjs` and `channel.test.mjs` for the new answers. Add tests where a recursive `root` is answered with the recursion limit as a runtime error and the host is not replaced, and where a capped value keeps only nodes within the cut. Verify `pnpm --filter @nx-lang/playground test` passes those.
- [x] 5.2 Build the language service without the implicit catalog import. Verify hover on a user-declared type and completion inside an element function's tag work in a worker test.

## 6. Address and sharing

- [x] 6.1 Write `src/share/codec.ts` (encode and decode per design.md) and generate `src/share/fixture.json` with `node:zlib`, where the fixture source is a few lines holding non-ASCII text. Verify codec tests that decode the fixture, round-trip random sources, and reject bad base64url, bad DEFLATE and bad UTF-8.
- [x] 6.2 Rewrite `routes.ts` and `router.ts`: an example id or default from the path, the `#code=` fragment winning, `replaceState` on the edit pause, `pushState` on choosing an example, and the not-found notices. Verify `routes.test.mjs` covers every scenario in "Addresses under the playground prefix" and "Source travels in the address" that doesn't need a browser.
- [x] 6.3 Add the Share button (clipboard plus confirmation). Verify with Playwright that Share, then opening the copied address in a new page, shows the same source and output.

## 7. Examples

- [x] 7.1 Write 10 to 20 examples (`examples.json`, `nx/<id>.nx`, `nx/<id>.out.nx`), one or two per topic in "Examples get a visitor started", each with a `docs` page path, and `hello` as the default. Include a sequence of multi-line records, so hover and folding show on examples. Verify every `.out.nx` equals `nxlang run` on its source.
- [x] 7.2 Rewrite `scripts/check-examples.mjs` as design.md describes, including `--update`, the count check and the docs-page existence check. Verify it passes. Verify it fails with both texts shown when one `.out.nx` is edited, and fails naming the example when a `docs` path is wrong.

## 8. The view

- [x] 8.1 Rewrite `App.tsx` and `EditorView.tsx`: the copied header, the toolbar (the examples `<select>` with an `<optgroup>` per topic and an "Edited" state, the docs link, Share, the fiddle link), the source pane, and the output pane. Verify with Playwright the scenarios in "Examples get a visitor started" and "The playground carries the site header and names itself" that need a browser.
- [x] 8.2 Make the output pane an `<nx-value>` that loads its own highlighter (the editor's cannot be shared: coloring switches its theme under the Monaco bridge), gets `truncated` and `stale` from the outcome, and shows the no-root, runtime-error and unspellable messages in its place. Verify with Playwright each value scenario in "The output pane shows the value as NX text" (record, sequence, `{}`, constant case, no root, runtime error with a marked span, unspellable value, large value, stale after a broken edit).
- [x] 8.3 Wire the output's `describe` hook to the language service's hover at the node's declaration, falling back to the default while the output is stale, and `nx-value-navigate` to selecting the declaration in the source pane. Verify with Playwright the hover, stale-hover, navigation and folding scenarios of the output pane requirement.
- [x] 8.4 Add the narrow layout. Verify with Playwright at 375 × 800 that both panes are reachable, the examples drop-down works, and `document.documentElement.scrollWidth` equals the viewport width.
- [x] 8.5 Set document titles per "The playground carries the site header and names itself". Verify in the Playwright run for an example and for a fragment.

## 9. Website: Open in playground and checked output

- [x] 9.1 Add the Expressive Code plugin that puts "Open in playground" on unmarked and `invalid` `nx` blocks, linking to `/play#code=…`. Import the encoder from a website-local copy of the codec, tested against the shared fixture. Verify a build shows the link on those blocks and not on `fragment` or `output` blocks. Verify with Playwright that clicking one on a built page opens the playground with that exact text.
- [x] 9.2 Teach the code-block check `nx output`, per "Documented output is checked", comparing against `evaluateNx().text`. Add fixture tests for matching, stale and orphaned output. Verify they pass.
- [x] 9.3 Turn the landing page's hand-written value into an `nx output` block, and add `nx output` after a handful of tour examples where seeing the value helps (records, unions, `for`). Verify the check passes over the site.

## 10. Documentation and specs housekeeping

- [x] 10.1 Rewrite `sites/playground/README.md`: what the playground is, prerequisites, running it under `/play`, adding an example and its expected output, the output pane's use of `@nx-lang/value-view`, and where deployment is documented. Verify each command in it runs.
- [x] 10.2 In `specs/future.md`, delete "Removing DrawnUI from the playground", "Shareable edited source" and "NX IR size". Move the still-relevant part of "The catalog as a library artifact" into the fiddle's section. Record the ir-runtime end-to-end coverage gap from design.md there too, and a table view for sequences of records as a `value-view` follow-up. Verify `grep -n "sites/playground" specs/future.md` finds nothing that describes code which no longer exists.
- [ ] 10.3 Tell the fiddle's maintainers, in the fiddle repo's `README-NX.md` or in an issue, that the NX playground no longer carries a DrawnUI catalog, so "the playground comparison" in its `CATALOG.md` is stale, and that `@nx-lang/value-view` exists if the fiddle wants to show values. Verify the note exists.
- [x] 10.4 Run `openspec validate rebuild-language-playground --strict`, `cargo test --workspace` and `pnpm -r test`. Verify all pass.

## 11. Ship

- [ ] 11.1 Merge, and let `deploy-playground.yml` and `deploy-website.yml` deploy. Verify in the Cloudflare dashboard that the `nxlang-playground` Worker's routes are `nxlang.org/play` and `nxlang.org/play/*` only, removing any `/playground` route the deploy left behind. Verify on `https://nxlang.org`:
  - open `/play`, choose three examples, hover a type name in the output, and fold a record;
  - press "Open in playground" from a tour page;
  - share an edited source and open it in a fresh browser context;
  - load at phone width;
  - `/playground` answers 404 with the site's not-found page.
