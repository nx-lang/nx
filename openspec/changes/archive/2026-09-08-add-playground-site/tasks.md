## 1. Move and rename

- [x] 1.1 `git mv sample-apps/drawnui-react sites/playground`, remove the empty `sample-apps` directory, and verify `git status` shows renames rather than deletes plus adds
- [x] 1.2 Replace the `sample-apps/drawnui-react` workspace entry with `sites/*` in `pnpm-workspace.yaml`, rename the package to `@nx-lang/playground` with a playground description, run `pnpm install` from the root, and verify the lockfile importer moved and `pnpm -r build` succeeds
- [x] 1.3 Update the Dockerfile's copy paths and comments to `sites/playground`, and verify `docker build -f sites/playground/Dockerfile .` from the repository root completes and `docker run -p 8080:8080` serves the gallery at `http://localhost:8080/playground`
- [x] 1.4 Sweep the moved tree for the old path and the word "fiddle" outside the vendored `src/drawnui/` and `reference/` directories (`grep -rni fiddle sites/playground --exclude-dir=node_modules --exclude-dir=dist --exclude-dir=drawnui --exclude-dir=reference`) and verify no hits remain after tasks 2 through 4 are done

## 2. Base path

- [x] 2.1 Add `sites/playground/base.mjs` exporting the `/playground` prefix, the API prefix and the health path, import it in `vite.config.ts` as `base`, and verify `pnpm run build` emits `dist/index.html` whose asset URLs start with `/playground/`
- [x] 2.2 Inject `<base href="/playground/">` into the shell from `vite.config.ts` (a `transformIndexHtml` plugin reading `base.mjs`, so the prefix is spelled in one place; `index.html` carries only a comment), change the font loads in `src/drawnui-runtime.ts` and the fetch in `src/compile/http.ts` and the language client's endpoint to build from `import.meta.env.BASE_URL`, and verify in the browser that fonts, the baboon image, the CanvasKit binary and compiles all load from under `/playground/`
- [x] 2.3 Rewrite `src/router.ts` so the gallery is `/playground` (with or without trailing slash), an example is `/playground/<id>`, an unknown id resolves to the gallery, and `pathForRoute` emits those paths; verify with unit tests over `routeFromPath` and `pathForRoute` run by `pnpm test`
- [x] 2.4 Rework `server/index.mjs` routing per design D3: `/` answers a 302 to `/playground`, paths outside the prefix answer 404 JSON, static files and the shell are served from under the prefix, compile and language mount under `/playground/api/`; verify with new cases in `server/index.test.mjs` for the redirect, the 404 outside the prefix, the shell at `/playground/<id>`, and a missing asset under the prefix
- [x] 2.5 Point the Vite dev proxy and its probe at `/playground/api`, and verify `pnpm run dev:all` opens the gallery at `http://localhost:5173/playground` with working compiles and hover, and `vite.config.test.mjs` still passes against the prefixed path

## 3. Health and cache headers

- [x] 3.1 Add `GET /playground/api/health` answering `{ "ok": true }` inline on the request thread with `cache-control: no-store`, and verify a test in `server/index.test.mjs` gets 200 JSON and that a non-GET method gets 405
- [x] 3.2 Set `cache-control` per design D5 (immutable for `dist/assets/*`, one day for other static files, `no-cache` for the shell, `no-store` for every `/api/` answer), and verify tests assert each header class against a built `dist/` fixture or a temporary dist directory
- [x] 3.3 Confirm `scripts/check-examples.mjs` and the compile tests are unaffected by the route move, and verify `pnpm test` in `sites/playground` passes end to end
- [x] 3.4 Add `server/watchdog.mjs` — a worker thread that kills the process when the main thread's heartbeat is older than the deadline (Railway's health check is deploy-time only, per design D4) — start it from `server/index.mjs`, and verify a test spawns a process whose main thread blocks and sees it end by `SIGKILL` within the deadline, while a responsive one is left alone

## 4. Rebrand the UI

- [x] 4.1 Rename `src/fiddle/Fiddle.tsx` to `src/editor/EditorView.tsx` (component `EditorView`, props `EditorViewProps`), rename the route kind to `"editor"`, update `App.tsx`, and verify `pnpm run typecheck` passes
- [x] 4.2 Change the gallery heading to "NX Playground" with a subtitle that names DrawnUI as today's target, change the card affordance to "Edit →", set the editor view's back link `href` to the gallery path, update `index.html`'s title and the document titles in `App.tsx`, and verify by loading the gallery and an example and reading the tab titles
- [x] 4.3 Reword the coverage-chip text in `src/examples/index.ts` for a visitor while keeping it derived from the shared capability vocabulary, and verify `pnpm run check-examples` still passes and the chips read correctly in the gallery and editor view
- [x] 4.4 Update the server's startup log line and the app's `README.md`, `docs/CATALOG.md` and `docs/FINDINGS.md` to say playground, and verify task 1.4's sweep is clean

## 5. Railway configuration

- [x] 5.1 Declare the service in `.railway/railway.ts` (Railway's `railway.json` is deprecated): no repository source (`empty()`), Dockerfile builder and path, the health check path with a short timeout, and an always-restart policy (no retry budget for the watchdog to exhaust); add the `railway` SDK as a root dev dependency and verify `railway config plan` shows only the intended changes
- [x] 5.2 Create the Railway project and an empty service with the root directory at the repository root (`railway init`, `railway add --service playground`), then apply the configuration with `railway config apply`; verify the service reads back with no repository source and no repository trigger, the Dockerfile path and health check set, and that the only lines the plan keeps reporting are the two documented CLI quirks (source type, watch patterns)
- [x] 5.3 Add `.github/workflows/deploy-playground.yml` per design D6: on pushes to `main` under a `paths` filter matching what the Dockerfile copies, build and test the workspace, `railway up --ci` under `RAILWAY_TOKEN_PRODUCTION` from the `production` GitHub environment, wait for the deployment to report `SUCCESS`, smoke-test `/playground/api/health` and the gallery through Cloudflare, and write a rollback summary on failure; verify `actionlint` passes
- [x] 5.4 Create a Railway project token scoped to `production`, store it as `RAILWAY_TOKEN_PRODUCTION` on the `production` GitHub environment (restricted to `main`), and verify the workflow's first run on `main` builds, deploys and reports healthy; if the Rust build fails on the builder, record the failure and switch to the GHCR alternative from design D6
- [x] 5.5 Check the first run's upload size and build log, and verify the root `.dockerignore` was honored (no `node_modules`, `target` or `sites/playground/reference` in the context)

## 6. Cloudflare configuration

- [x] 6.1 Add the custom domain `nxlang.org` in Railway, create the proxied apex CNAME and the `www` CNAME in Cloudflare, add a redirect rule from `www` to the apex, and verify Railway shows the domain as verified and `https://nxlang.org/playground` serves the gallery
- [x] 6.2 Set SSL/TLS to Full (Railway requires Full, not strict, behind the Cloudflare proxy) and enable Always Use HTTPS, and verify `curl -I http://nxlang.org/playground` answers a redirect to `https://` and the HTTPS page loads without certificate warnings
- [x] 6.3 Add the rate-limiting rule on `/playground/api/*`, and verify a burst above the limit from one client returns 429 at the edge while normal editing stays unaffected
- [x] 6.4 Add the cache rule for `/playground/assets/*`, fonts and images, and verify a second `curl -I` of a hashed asset shows `cf-cache-status: HIT` and the shell shows `DYNAMIC` or `MISS` every time
- [x] 6.5 Enable Bot Fight Mode and Cloudflare Web Analytics, and verify the analytics snippet or proxy-injected beacon reports a page view
  - Bot Fight Mode is on and, on the first live run (2026-09-08), challenged the workflow's smoke test (RF3). Web Analytics is enrolled (automatic injection, EU excluded) and the beacon is injected into every HTML route in SPA mode; it only appears for requests that accept HTML, which is why a bare curl showed none.

## 7. Documentation

- [x] 7.1 Rewrite the site README's run and deploy sections for the `/playground` prefix, the health endpoint, the Railway deploy, and the process-isolation follow-up, and verify a fresh reader can run `pnpm run dev:all` and `pnpm start` from the README alone
- [x] 7.2 Add a "Playground site" section to `docs/deployment.md` (deploy on push to main, rollback via Railway redeploy, how to verify) and to `docs/deployment-setup.md` (Railway service settings, every Cloudflare record and rule with its values, the rate-limit threshold), and verify every rule from tasks 5 and 6 appears there
- [x] 7.3 Note in `specs/future.md` that shareable edited source and child-process compile isolation are the playground's next steps, and verify the section links to the `playground` spec

## 8. Verification and archive

- [x] 8.1 Run the full check from the repository root — `pnpm -r build`, `pnpm -r test`, `pnpm run typecheck` in the site — and verify all pass
- [x] 8.2 From outside the network, verify the migration checks in design: gallery at `https://nxlang.org/playground`, `/` redirects with 302, `/playground/<id>` opens an example directly, an unknown id shows the gallery, `/playground/api/health` answers 200, and a compile from the editor succeeds
- [x] 8.3 When archiving this change, delete `openspec/specs/drawnui-fiddle` after the archive sync empties it, and verify `openspec validate` passes with only the `playground` spec present
