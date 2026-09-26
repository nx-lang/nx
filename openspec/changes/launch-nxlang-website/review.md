# Review: launch-nxlang-website

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/website/spec.md, specs/playground/spec.md,
specs/vscode-extension-publishing/spec.md  
**Reviewed code:** `.github/workflows/deploy-website.yml`, `.github/workflows/deploy-playground.yml`,
`sites/playground/{_headers,wrangler.jsonc,worker/index.mjs,worker/index.test.mjs,vite.config.ts,base.mjs,package.json,README.md}`,
`sites/website/{astro.config.mjs,starlight.config.mjs,package.json,wrangler.jsonc,README.md}`,
`sites/website/scripts/check-code-blocks{,.test}.mjs` and fixtures, `sites/website/src/components/*.astro`,
`sites/website/src/styles/custom.css`, `sites/website/src/content/docs/{index.mdx,404.md,tutorials/getting-started.md,contributing/index.md}`
and the fragment/invalid blocks across the content, `docs/deployment.md`, `docs/deployment-setup.md`, `docs/README.md`,
`specs/future.md`, package manifests, `NxLang.Sdk.csproj`, `Test-NxSdkPackage.ps1`.  
**Checks run:** `pnpm --filter @nx-lang/website test` (7 unit tests pass; 168 blocks in 23 files, 0 failures) and
`build` (passes); both Workers under `wrangler@4.141.0 dev` (shell routing, 404s and `_headers` behave as specified);
the Getting Started JavaScript sample and the landing snippet evaluated through `@nx-lang/sdk-wasm` +
`@nx-lang/ir-runtime` (both match the values the pages show); the header at 1280 px and 375 px with Playwright.

Tasks 1.1, 7.x, 8.x and 11.x are open and are by-hand or cutover steps; they are not counted as findings.

## Findings

### 🟡 Fixed - RF1 On the landing page and the 404 page, the desktop header shows its links twice and wraps onto a second row.
- **Severity:** High
- **Evidence:** `sites/website/src/components/Header.astro:51-55` gives `.narrow-links` `display: flex` in the
  component's scoped style, which Astro emits unlayered. The class `md:sl-hidden` that is meant to hide it at 50rem
  and wider is Starlight's utility in `@layer starlight.utils`, and unlayered rules beat layered ones regardless of
  specificity. The rule `:root[data-has-sidebar] .narrow-links { display: none }` hides it only on pages with a
  sidebar, so on the splash landing page and the 404 page both `.narrow-links` and `.right-group` render. Playwright
  at 1280 px lists the header's visible links as `NX, Docs, Playground, GitHub, Docs, Playground, GitHub` on `/` and
  `/404.html` (and once on `/language-tour/types/`); a screenshot shows the second set with the theme select on a
  second row under the title and search, with the title pushed up and clipped. This is the first thing a visitor to
  `nxlang.org` sees.
- **Recommendation:** Scope the narrow links' `display: flex` to narrow screens (for example
  `@media (max-width: 49.99rem)`, or hide `.narrow-links` inside `@media (min-width: 50rem)` in the same scoped
  style), so it no longer depends on the utility class winning the cascade. Re-run the 5.2 Playwright check at
  desktop width on `/` and the 404 page, not only on a docs page, and assert each header link appears exactly once.
- **Fix:** `Header.astro` drops `md:sl-hidden` from `.narrow-links` and hides it with `display: none` inside the component's own `@media (min-width: 50rem)`, so it no longer depends on a layered utility winning the cascade; the header comment says why. Playwright against the rebuilt site lists the visible header links as `NX, Docs, Playground, GitHub` exactly once on `/`, `/404.html`, `/language-tour/types/` and an unknown path at 1280 px, and the same on the sidebar-less pages at 375 px (the docs page keeps them in the mobile menu); the 1280 px header is one 64 px row with the theme select beside the links.

### 🟡 Fixed - RF2 The playground's deploy no longer runs when the WASI sysroot pin changes, because `scripts/**` was dropped from its paths.
- **Severity:** Medium
- **Evidence:** `.github/workflows/deploy-playground.yml` removed `scripts/**` from `on.push.paths`, but the compiler
  module the playground ships is built by `bindings/wasm/scripts/build-wasm.mjs`, which imports
  `scripts/fetch-wasi-sysroot.mjs` (line 12) and its pinned version and checksum. A commit that only bumps the sysroot
  changes the shipped `nx-*.wasm` without redeploying, contrary to the playground spec's "a push to `main` that
  touches the site or a package it depends on SHALL build, test and deploy it". design.md also says the workflow
  "keeps its `paths` list".
- **Recommendation:** Restore `scripts/fetch-wasi-sysroot.mjs` (or `scripts/**`) to the paths list, with a comment
  saying the wasm build reads it.
- **Fix:** `deploy-playground.yml` lists `scripts/fetch-wasi-sysroot.mjs` again, with a comment that the wasm build fetches the sysroot by the version and checksum pinned there. It is the only file under `scripts/` that `bindings/wasm/scripts/build-wasm.mjs` imports, so the narrower path is used instead of `scripts/**`.

### 🟡 Fixed - RF3 The playground's cache policy has no automated test now that the server tests are gone.
- **Severity:** Low
- **Evidence:** `sites/playground/server/index.test.mjs` (210 lines, deleted) covered the cache headers. Nothing
  replaces it: `worker/index.test.mjs:8-17` stubs `ASSETS` with a response that sets `cache-control: no-cache`
  itself, so it passes whatever `_headers` says; no test reads `sites/playground/_headers` or checks that the build
  copies it to `dist/_headers`; and the deploy smoke test checks status and content type, not `Cache-Control`. A
  typo in the `/playground/assets/*` or `/playground/index.html` rule, or a new `public/` folder, would ship
  unnoticed. The requirement "Static files declare their cache policy" is only verified by hand (task 6.2).
- **Recommendation:** Add a test that parses `_headers` and asserts `immutable, max-age=31536000` for
  `/playground/assets/*`, `no-cache` for `/playground/index.html`, and a rule for each directory under `public/`.
  Optionally have the smoke test assert `immutable` on the compiler module and `no-cache` on the shell.
- **Fix:** Added `sites/playground/headers.test.mjs`, run by the site's `test` script. It parses `_headers` and asserts `public, max-age=31536000, immutable` for `/playground/assets/*`, `no-cache` for `/playground/index.html`, and `public, max-age=86400, must-revalidate` for `/playground/<dir>/*` for every directory under `public/`. It passes (the full playground suite: 99 tests), and fails naming `public/lottie` when that rule is mistyped. The optional smoke-test assertion on `Cache-Control` was not added.

### 🟡 Fixed - RF4 The website's content-hashed assets are revalidated on every page load.
- **Severity:** Low
- **Evidence:** `sites/website` has no `_headers`, so `dist/_astro/*` (hashed CSS and JS) and the Pagefind bundle are
  served with the static-assets default, `Cache-Control: public, max-age=0, must-revalidate` (seen under
  `wrangler dev` on `/_astro/*.css`), so each docs navigation re-requests them.
  The playground already solves the same problem with `sites/playground/_headers`.
- **Recommendation:** Add `sites/website/public/_headers` with
  `/_astro/*  Cache-Control: public, max-age=31536000, immutable` (Astro emits it at the root of `dist/`, where
  Workers static assets read it). Leave HTML and Pagefind's unhashed index on the default.
- **Fix:** Added `sites/website/public/_headers` with `/_astro/*  Cache-Control: public, max-age=31536000, immutable`; HTML and Pagefind keep the default. The build copies it to `dist/_headers`, and under `wrangler@4.141.0 dev` a `/_astro/*.css` file answers `public, max-age=31536000, immutable`, `/` still answers `max-age=0, must-revalidate`, and `/_headers` itself answers 404. `docs/deployment.md`, `docs/deployment-setup.md` and `sites/website/README.md` mention the file.

### 🟡 Fixed - RF5 The not-found page is included in `llms-full.txt` and `llms-small.txt`.
- **Severity:** Low
- **Evidence:** After a build, `dist/llms-full.txt:73` and `dist/llms-small.txt:33` contain `# Page not found` and
  its text ("It may have moved when the documentation moved to nxlang.org"). The page is already kept out of search
  (`pagefind: false`) and the sitemap, but `starlight-llms-txt` in `sites/website/astro.config.mjs:15` has no
  `exclude`.
- **Recommendation:** Pass `exclude: ['404']` to `starlightLlmsTxt`, and check both files no longer carry it.
- **Fix:** `starlight-llms-txt`'s `exclude` applies only to `llms-small.txt` (0.12.0's `llms-full.txt` filters on `draft` alone, and `draft` would drop the page from the build), so the 404 moved out of the docs collection instead: `src/content/docs/404.md` is now `src/pages/404.astro`, the same frontmatter passed to `StarlightPage`, with `disable404Route: true` in `starlight.config.mjs`. After a build, `Page not found` appears in neither `llms-full.txt` nor `llms-small.txt`; `dist/404.html` still carries the header and the `nx-build` stamp, stays out of Pagefind and the sitemap, and `wrangler dev` answers an unknown path with 404 and that page. `sites/website/README.md` says where the page lives and why.

### 🟡 Fixed - RF6 The change's artifacts no longer match what was built.
- **Severity:** Low
- **Evidence:**
  - proposal.md ("the playground's `public/_headers`"), design.md ("moves unchanged into `public/_headers`") and
    task 6.2 ("Write `public/_headers`") name a file that is `sites/playground/_headers`, copied to `dist/_headers`
    by the `cacheHeaders` plugin (`sites/playground/vite.config.ts:67-83`), for the good reason given there.
  - design.md says `deploy-playground.yml` "keeps its `paths` list"; it changed (see RF2).
  - design.md's Open Questions still asks which fiddle page to link, which task 5.1 settled (the root).
  - Task 4.3's verify clause is broken by an inserted sentence: "…NuGet `NxLang.Sdk`). The editor step is finished
    in 11.5, once the extension is published, and that no step before the Contributing pointer mentions Rust…".
- **Recommendation:** Update the three `_headers` mentions to the real path and why; update the paths sentence
  after RF2; record the fiddle decision and drop the question; restore 4.3's verify clause and put the 11.5 note in
  its own sentence.
- **Fix:** proposal.md and design.md name `sites/playground/_headers` (and the website's `public/_headers`), and design.md says why it isn't in `public/` and that a test checks it; task 6.2 names the real path. design.md's paths sentence now says what changed in the list and why (after RF2). The fiddle decision is recorded in Content work and the Open Questions section is gone, since its other question (social card and accent color) was answered by task 5.3. Task 4.3's verify clause is whole again, with the 11.5 note as its own sentence. `openspec validate --strict` passes.

## Questions
- `openspec/changes/archive/2026-09-25-update-fiddle-to-nx-occurrences/tasks.md` is staged with 8.1–8.6 checked
  off. That belongs to another change. Is it meant to ride along in this PR?
- `docs/README.md` is untracked (`??`) while its old content is staged as the rename to `sites/website/README.md`,
  so a commit of the staged tree would leave `docs/` with no README. It needs a `git add` before committing.

## Summary
- The static hosting is sound: both Worker configs, the playground's shell routing and 404s, and its `_headers`
  behave as the specs require under `wrangler@4.141.0 dev`, and both deploy workflows gate on build and tests
  before an atomic deploy and verify the new build afterwards. The code-block check is well built and its fixtures
  cover all four spec scenarios; the content passes it, the fragments hide only names declared elsewhere, and
  Getting Started's samples evaluate to what the page says. Railway, Pages and `nx-lang.dev` are fully gone.
- Fix RF1 before cutover, since it breaks the header on the front page. RF2 is a one-line fix to the workflow;
  the rest is test coverage, caching and artifact tidying.
