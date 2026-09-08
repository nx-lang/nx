# Review: add-playground-site

## Scope
**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md`, `specs/playground/spec.md`,
`specs/drawnui-fiddle/spec.md`
**Reviewed code:** the working tree and staged diff of the move from `sample-apps/drawnui-react` to
`sites/playground` — `base.mjs`, `server/index.mjs`, `server/index.test.mjs`, `server/watchdog.mjs`,
`server/watchdog.test.mjs`, `server/language.mjs`, `vite.config.ts`, `vite.config.test.mjs`,
`index.html`, `src/paths.ts`, `src/routes.ts`, `src/routes.test.mjs`, `src/router.ts`,
`src/compile/http.ts`, `src/drawnui-runtime.ts`, `src/editor/EditorView.tsx`,
`src/editor/NxEditor.tsx`, `src/gallery/Gallery.tsx`, `src/examples/types.ts`, `src/App.tsx`,
`Dockerfile`, `Dockerfile.dockerignore`, `package.json`, `README.md`; at the root
`.railway/railway.ts`, `.github/workflows/deploy-playground.yml`, `package.json`,
`pnpm-workspace.yaml`, `.dockerignore`, `docs/deployment.md`, `docs/deployment-setup.md`,
`specs/future.md`.

**Checks run:** `pnpm test` in `sites/playground` (35 tests pass, 12 examples check),
`pnpm run typecheck` (clean), `actionlint` on the deploy workflow (clean),
`openspec validate add-playground-site --strict` (valid), the task 1.4 `fiddle` sweep (no hits),
and a live probe of `server/index.mjs` against a stand-in `dist/`.

Tasks 5.4, 5.5, 6.1, 6.3–6.5, 8.2 and 8.3 are open; they are the hosting and edge steps plus the
archive step, and nothing below counts them as mismatches.

## Findings

### ✅ Verified - RF1 The shell is served with a one-day cache header when requested by its file name
- **Severity:** Medium
- **Evidence:** `sites/playground/server/index.mjs:165-180` picks `CACHE.static` for any file
  that exists under `dist/` and only switches to `CACHE.shell` on the fallback path. `index.html`
  exists, so `GET /playground/index.html` (and `GET /playground/assets/../index.html`, which
  normalizes to the same file) answers `cache-control: public, max-age=86400`; confirmed against a
  stand-in `dist/`. The spec's "The shell is not cached" scenario says *for any address that
  resolves to it*, and design D5 says the same. A browser or intermediary that fetches the shell by
  name would keep a stale shell naming old asset hashes for a day after a deploy.
- **Recommendation:** After resolving `file`, treat `relative(distRoot, file) === "index.html"` as
  the shell and send `CACHE.shell` (or redirect `/playground/index.html` to `/playground`). Add a
  case to `server/index.test.mjs` for the by-name address.
- **Fix:** `serveSite` now decides the cache class from the resolved file: anything that resolves to `index.html` is `no-cache`, whether reached by route or by name (`/playground/index.html`, `/playground/assets/..%2Findex.html`). New case in `server/index.test.mjs` covers both addresses.
- **Verification:** `serveSite` now derives the cache class from the resolved file (`inDist === "index.html"` → `no-cache`, `assets/` → immutable, else one day). Live probe against a stand-in `dist/`: `/playground/index.html`, `/playground/assets/..%2Findex.html` and `/playground/` all answer 200 with `cache-control: no-cache`; the hashed and plain asset cases still answer their own classes. The new test passes.

### ✅ Verified - RF2 The restart budget the watchdog relies on is finite and, as configured, runs out
- **Severity:** Medium
- **Evidence:** `.railway/railway.ts:31-32` sets `restartPolicyType: "ON_FAILURE"` with
  `restartPolicyMaxRetries: 10`. Design D4 makes the watchdog's `SIGKILL` plus this policy the only
  recovery for a process that hangs after deployment, and Railway does not document the retry
  counter resetting while a deployment stays live. After the tenth hang in one deployment's
  lifetime the service stays down until someone redeploys, which is the outcome D4 exists to
  prevent. The health check already keeps a build that cannot serve from going live, so a bounded
  count is not what protects against a broken image.
- **Recommendation:** Either use `restartPolicyType: "ALWAYS"`, or raise the retry count well
  above any plausible hang rate and say in `docs/deployment.md` what happens when it is exhausted
  (the `watchdog:` log line is already documented; add "if the service is down and the log ends in
  a watchdog line, redeploy"). Confirm Railway's counter semantics when task 5.4 runs.
- **Fix:** `restartPolicyType` is now `ALWAYS` with no retry count in `.railway/railway.ts`; design D4, `docs/deployment-setup.md` and `docs/deployment.md` say why (no budget to exhaust; the health check, not the budget, keeps a broken image from going live), and the deployment doc says what repeated `watchdog:` lines mean. Applied with `railway config apply`; the plan afterwards no longer lists the restart policy, only the two documented quirk lines (source type, watch patterns).
- **Verification:** `.railway/railway.ts` declares `restartPolicyType: "ALWAYS"` with no retry count and a comment giving the reason. Ran `railway config plan` from the repository root: "0 to add, 2 to change" and the two changes are exactly `source.type` and `build.watchPatterns`, so the applied service already carries the policy. Design D4, `docs/deployment-setup.md` and `docs/deployment.md` (repeated `watchdog:` lines) all say the same. One note for whoever runs the plan next: the `railway` SDK re-executes `$_` with `--version` to check the CLI, so invoking the CLI through a wrapper such as `timeout` makes it fail with a misleading "requires Railway CLI 5.42.1" message; run `railway config plan` directly.

### ✅ Resolved - RF3 The workflow's smoke test may be challenged by Bot Fight Mode once task 6.5 is done
- **Severity:** Medium
- **Evidence:** `.github/workflows/deploy-playground.yml:144-157` verifies a deployment with
  `curl` against `https://nxlang.org/playground/api/health` and the gallery, through Cloudflare.
  `docs/deployment-setup.md` turns on Bot Fight Mode, which on the Free plan challenges requests it
  classifies as automated and cannot be exempted per path or per user agent. If the runner's
  requests are challenged, the step sees an HTML challenge instead of `{"ok":true}` and the job
  reports a failed deploy for a deployment that is live, and every subsequent deploy does the same.
- **Recommendation:** When 6.5 is applied, run the workflow (or the same two `curl` calls from a
  cloud runner) and confirm the answers are the origin's. If they are challenged, either smoke-test
  the Railway-issued hostname for the origin checks and treat a Cloudflare 403 as "edge reachable",
  or drop Bot Fight Mode in favor of the rate limit alone, and record the choice in the setup doc.
- **Observed (2026-09-08, run 34191751574):** confirmed. Railway's deployment went SUCCESS and the site answers from a residential address, but all 12 smoke-test attempts from the GitHub runner got a Cloudflare managed challenge (`Just a moment...`, HTTP 403) for both the health path and the gallery, so the job reported a failed deploy for a live deployment. Bot Fight Mode is on (`fight_mode: true` on the zone); the Free plan offers no per-path or per-agent exemption. Remedy pending the owner's choice between dropping Bot Fight Mode (the rate limit stays) and keeping it with the smoke test reduced to "the edge answers for the host".
- **Fix:** Bot Fight Mode switched off for the zone (2026-09-08). The rate limit on `/playground/api/*` is the origin's protection and is verified live; Bot Fight Mode challenged only clients the rate limit does not need to stop, including the smoke test, and cannot be exempted on the Free plan. The workflow is unchanged; the setup doc records the setting as Off with the reason, and the corresponding open question is removed from `specs/future.md`.
- **Status:** Not fixable before the first run: whether Bot Fight Mode challenges the runner is only observable live, and both remedies (smoke-test the origin hostname, or drop Bot Fight Mode) are a call to make on that evidence. Task 5.4 covers the first run; if the smoke test sees a challenge page, apply one of the two remedies and record it in the setup doc. Note there is now no Railway-generated hostname to fall back to (RF10), so the fallback would be to skip the through-Cloudflare check for the health body and keep only the status code.

### ✅ Verified - RF4 The root `.dockerignore` still names the old app path, and the docs assume there is none
- **Severity:** Low
- **Evidence:** `.dockerignore:13` excludes `sample-apps/drawnui-react/dist`, a path that no longer
  exists; task 1.4's sweep was scoped to the moved tree so the repo-wide reference survived.
  `sites/playground/Dockerfile.dockerignore` sits beside the Dockerfile and, under BuildKit,
  replaces the root file for this build, yet `docs/deployment-setup.md` ("move that file's
  contents to a root `.dockerignore`") and design D6's risk note both read as if no root file
  existed. The two files also disagree: the root one excludes `.git`, `docs`, `bin` and `obj`,
  the Dockerfile one excludes `reference`.
- **Recommendation:** Replace the stale line with `sites/playground/dist` (or delete it, since
  `**/dist` already covers it), and either fold the Dockerfile-specific ignores into the root file
  and delete `Dockerfile.dockerignore`, or keep both and say in the setup doc which one Railway
  honors and why there are two.
- **Fix:** `sites/playground/Dockerfile.dockerignore` is deleted and the root `.dockerignore` is the one ignore file: stale `sample-apps/drawnui-react/dist` line removed (covered by `**/dist`), `**/target` and `sites/playground/reference` added (the old `reference` pattern was relative to the context root and never matched the site's directory). `docs/deployment-setup.md`, design's risk note and task 5.5 now describe the root file.
- **Verification:** `sites/playground/Dockerfile.dockerignore` is gone (`git status` shows the original deleted). The root `.dockerignore` no longer mentions `sample-apps`, adds `**/target` and `sites/playground/reference`, and explains at the top that it is the one ignore file for the image build. A repo-wide grep for `Dockerfile.dockerignore` and `sample-apps` finds only the change's own proposal/design/tasks prose describing the move. Design's risk line and `docs/deployment-setup.md` now refer to the root file.

### ✅ Verified - RF5 Design and tasks contradict the implementation on two points
- **Severity:** Low
- **Evidence:** Design "Non-Goals" lists "a GitHub Actions deploy workflow", while D6, tasks 5.3
  and `.github/workflows/deploy-playground.yml` add exactly that. Task 2.2 says to set
  `<base href="/playground/">` in `index.html`, while D2 and `vite.config.ts:93-98` inject it
  through `transformIndexHtml` and `index.html:6` carries only a comment. Both were resolved the
  right way in code; the artifacts were not brought along.
- **Recommendation:** Remove the workflow from the design's Non-Goals and reword task 2.2 to
  describe the injection, so the archived change does not contradict its own design.
- **Fix:** Removed the deploy workflow from the design's Non-Goals; task 2.2 now describes the `transformIndexHtml` injection from `vite.config.ts` and that `index.html` carries only a comment.
- **Verification:** The Non-Goals list now reads "Moving the docs site to nxlang.org, PR preview environments, observability beyond…" with no mention of a deploy workflow. Task 2.2 describes the `transformIndexHtml` plugin reading `base.mjs` and the comment-only `index.html`, which matches `vite.config.ts` and `index.html`. `openspec validate --strict` still passes.

### ✅ Verified - RF6 The watchdog measures staleness with the wall clock
- **Severity:** Low
- **Evidence:** `sites/playground/server/watchdog.mjs:37-39` and `:53` compare `Date.now()`
  across threads. A forward clock step larger than the deadline (an NTP correction on a fresh
  container, a laptop resuming from sleep during development) makes a healthy process look stuck
  for one tick and kills it. The stand-in is monotonic and process-wide.
- **Recommendation:** Store and compare `process.hrtime.bigint()` (nanoseconds since an arbitrary
  process-wide origin, comparable between worker threads) instead of epoch milliseconds.
- **Fix:** `server/watchdog.mjs` stores and compares `process.hrtime.bigint()` (monotonic, one origin per process) instead of `Date.now()`; the staleness is converted to milliseconds for the deadline and the log line. Design D4 and the site README mention the clock. Both watchdog tests pass.
- **Verification:** `server/watchdog.mjs` stores `process.hrtime.bigint()` in the shared `BigInt64Array` on both threads and computes staleness as `(now() - beat) / 1_000_000n` milliseconds before comparing with the deadline; no `Date.now()` remains in the file. Both watchdog tests pass (stuck main thread killed by `SIGKILL` in ~2 s at a 1.5 s deadline; responsive one exits 0). Design D4 and README line 149 name the monotonic clock.

### ✅ Verified - RF7 `HEAD` on the health path answers 405
- **Severity:** Low
- **Evidence:** `sites/playground/server/index.mjs:129-132` accepts only `GET`; a live probe of
  `HEAD /playground/api/health` returned 405. Railway polls with `GET` so the deploy gate works,
  but external monitors and Cloudflare's own health checks commonly use `HEAD`, and Node already
  drops the body for a `HEAD` response so nothing else needs to change.
- **Recommendation:** Accept `HEAD` alongside `GET` in `handleHealth`, and add it to the health
  test.
- **Fix:** `handleHealth` accepts `HEAD` alongside `GET`; the health test now probes with `HEAD` and asserts 200, an empty body and `no-store`.
- **Verification:** Live probe: `HEAD /playground/api/health` answers `200` with `cache-control: no-store` and no body (Node drops it; `content-length: 11` is still advertised, which is correct for `HEAD`). `POST` still answers 405. The updated health test passes.

### ✅ Verified - RF8 The rollback summary loses the failed deployment's id, and the wait loop has no exit for a superseded deployment
- **Severity:** Low
- **Evidence:** `.github/workflows/deploy-playground.yml:111-112` runs `railway up --ci`; when
  the Railway build fails, that step fails, the "Wait" step never runs, and the failure summary at
  `:178` prints "Attempted deployment: not created" although Railway did create a deployment and
  marked it `FAILED`. Separately, the status `case` at `:131-136` has no branch for `SKIPPED` (the
  status Railway gives a deployment superseded by a newer one), so a concurrent deploy makes the
  loop run its full ten minutes before failing.
- **Recommendation:** Look up the newest deployment in a step that runs `if: always()` after
  `railway up` (or parse the id from `railway up`'s output) so the summary always names it, and add
  `SKIPPED` to the terminal statuses.
- **Fix:** The wait step now runs whenever the job was not cancelled (`if: !cancelled() && steps.before.outcome == 'success'`); when `railway up` failed it looks up the newest deployment once, records its id for the summary if Railway created one, and fails with that status (or says no deployment was created). `SKIPPED` was added to the terminal statuses. `actionlint` passes.
- **Verification:** The "Upload" step has `id: up`; the "Wait" step runs under `if: !cancelled() && steps.before.outcome == 'success'` with `UPLOADED` derived from `steps.up.outcome`. On a failed upload it looks up the newest deployment once, writes its id to `GITHUB_OUTPUT` when it differs from `BEFORE`, and exits 1 either way, so the failure summary's `steps.after.outputs.id` is populated when Railway created a deployment. `SKIPPED` is in the terminal-status `case`. The smoke-test and success-summary steps have no `if`, so they still skip after a failure. `actionlint` passes. The logic is verified by reading only; the workflow has not yet run on a runner (task 5.4).

### ✅ Verified - RF9 The route tests need a newer Node than the package declares
- **Severity:** Low
- **Evidence:** `sites/playground/src/routes.test.mjs:7` imports `./routes.ts` and relies on
  Node's built-in type stripping, which is unflagged only from Node 22.18 / 23.6. `package.json`
  declares `"node": ">=22.0.0"` and the README says "Node 22+", so a contributor on 22.0–22.17
  gets `ERR_UNKNOWN_FILE_EXTENSION` from `pnpm test`. CI and the Docker image use Node 24 and 22.x
  from nodesource respectively, so the pipeline is unaffected.
- **Recommendation:** Raise `engines.node` and the README to `>=22.18`, or compile the route
  table's tests through the same path the server tests use.
- **Fix:** `engines.node` raised to `>=22.18.0` in `sites/playground/package.json` and the root `package.json` (the root's `pnpm -r test` runs the same route tests); the README prerequisite now says Node 22.18+ and why.
- **Verification:** Both `package.json` files declare `"node": ">=22.18.0"`; README line 21 says "Node 22.18+" and gives the type-stripping reason. The Dockerfile's nodesource `setup_22.x` installs current 22.x (above 22.18) and CI uses 24, so nothing else needed to move.

### ✅ Verified - RF10 The edge rules can be bypassed through the Railway-issued hostname
- **Severity:** Low
- **Evidence:** Design D7 and the spec's "API requests are rate limited" scenario make Cloudflare
  the bound on how often the single thread can be provoked, but a Railway service also answers on
  its generated `*.up.railway.app` domain, which Cloudflare does not front. Nothing in
  `.railway/railway.ts` or `docs/deployment-setup.md` removes that domain or notes that the rate
  limit does not apply to it.
- **Recommendation:** After the custom domain verifies (task 6.1), remove the Railway-generated
  public domain from the service and add that step to the "Custom domain" section of the setup
  doc; or state explicitly that the bypass is accepted for the first cut.
- **Fix:** Checked live: `railway status --json` shows `serviceDomains: []` for the service, so no bypass exists today. The "Custom domain" section of `docs/deployment-setup.md` gained a step saying to leave the service without a generated domain (`railway domain list` shows only `nxlang.org`; a bare `railway domain` would create one), design D7 says the same, and `.railway/railway.ts` carries the note beside the replicas line.
- **Verification:** Ran `railway domain list --service playground`: the only domain is `nxlang.org` (custom, ACTIVE); no generated `*.up.railway.app` domain exists. `docs/deployment-setup.md` step 4 under "Custom domain" says to leave it that way and warns that a bare `railway domain` would create one; `.railway/railway.ts` lines 39-40 carry the same note.

### ✅ Verified - RF11 The `<base href>` injection and the cache headers for the by-name shell have no test
- **Severity:** Low
- **Evidence:** D2 makes `siteBase()` in `vite.config.ts:93-98` the only place the shell's
  `<base href>` is written, and the examples' relative `images/baboon.jpg` depends on it at nested
  addresses such as `/playground/images`. Task 2.1 verified it by hand; `vite.config.test.mjs`
  already boots a dev server and could assert it in one request. RF1's header case is likewise
  untested.
- **Recommendation:** Add a `GET /playground/` case to `vite.config.test.mjs` asserting the
  response contains `<base href="/playground/">`, and the RF1 case to `server/index.test.mjs`.
- **Fix:** `vite.config.test.mjs` fetches `/playground/` from the dev server and asserts `<base href="/playground/">` is in the shell; the by-name shell header case is in `server/index.test.mjs` (RF1). `pnpm test` in the site: 37 tests pass.
- **Verification:** `vite.config.test.mjs` has "the shell carries the site's `<base href>`…" fetching `/playground/` from the real dev server and matching `<base href="/playground/"`; `server/index.test.mjs` has "the shell is not cached when it is asked for by name either" over both addresses. Ran `pnpm test`: 37 tests pass and all 12 examples check.

## Questions
- Task 6.2 (SSL mode and Always Use HTTPS) is marked done while 6.1 (domain, DNS records, `www`
  redirect) is not, yet `docs/deployment-setup.md` records concrete values for the CNAME target and
  the `_railway-verify` TXT record. Which of the Cloudflare steps have actually been applied, so
  the open tasks reflect the real state?
  - **Answer:** Every Cloudflare step is applied — DNS records, the `_railway-verify` TXT, SSL
    Full, Always Use HTTPS, the `www` redirect, the rate limit, the cache rule, Bot Fight Mode and
    Web Analytics — and Railway reports the domain verified. Tasks 6.1, 6.3, 6.4 and 6.5 stay open
    only because their verification half (a 429 under burst, `cf-cache-status: HIT`, a beacon page
    view, the gallery at the domain) needs a live deployment; 6.2's verification (the redirect)
    was possible without one, which is why it alone is ticked.
- The `validate` job in `deploy-playground.yml` is the first workflow in this repository to run
  `pnpm -r build` on a runner, which compiles the native addon. It has no explicit Rust setup step
  and relies on the runner's rustup honoring `rust-toolchain.toml` (1.91.1). Has that job been run
  once (via `workflow_dispatch`) before it gates the first deploy in task 5.4?
  - **Answer:** Not yet: the workflow exists only in the uncommitted working tree, and
    `workflow_dispatch` needs it on a branch first. Its first run will be the first deploy. The
    runner's rustup does honor `rust-toolchain.toml` (it installs the pinned toolchain on first
    use), and `Swatinem/rust-cache` keeps that from repeating; if the job fails on the toolchain,
    adding `dtolnay/rust-toolchain` is a one-line fix.

## Summary
- The code half of the change is complete and matches the design: one prefix module feeds the
  server, the Vite config and the client; the address scheme, redirect, 404 outside the prefix,
  health route, cache classes and watchdog are all implemented and covered by tests that pass; the
  rebrand is thorough (the `fiddle` sweep is clean, including the docs); the Railway declaration
  and the deploy workflow follow D6 and lint clean; the docs describe every edge rule with values.
- Two Medium findings are about the shell's cache header when fetched by file name (RF1) and the
  restart budget the watchdog depends on (RF2). RF3 is a risk to verify when Bot Fight Mode is
  switched on. The rest are small consistency and hardening items.
- The open tasks are all hosting and edge steps that need the Railway token and Cloudflare
  dashboard; nothing in the reviewed code blocks them.

### Verification pass (2026-09-07 21:35)
- Verified RF1, RF2, RF4, RF5, RF6, RF7, RF8, RF9, RF10 and RF11: the site's 37 tests and 12
  example checks pass, `actionlint` and `openspec validate --strict` pass, live probes confirm
  the shell's `no-cache` by name and `HEAD` on health, `railway config plan` shows only the two
  documented quirk lines, and `railway domain list` shows only `nxlang.org`.
- RF3 stays open by design: it can only be observed on the first live run (task 5.4), and the
  finding's Status note records the two remedies.
- No new findings.
