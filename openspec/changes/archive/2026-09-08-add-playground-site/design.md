## Context

See proposal.md — Why. The fiddle at `sample-apps/drawnui-react` is already the shape the site
needs: one Node process serving a Vite-built SPA plus `POST /api/compile` and `POST /api/language/*`,
built by a two-stage Dockerfile from the repository root because the native `@nx-lang/sdk-node`
addon is compiled from source. It honors `PORT`. What it assumes, and what this change removes, is
that it owns the root of its origin: `<base href="/">`, Vite's default base, absolute `/api/...`
fetches, absolute `/fonts/...` font loads, and a router matching `/fiddle/<id>`.

Constraints that shape the design:

- **The compile and language routes call the native binding synchronously on the server's only
  thread.** A call that never returns blocks every visitor, and nothing in-process can interrupt it
  (a worker thread cannot be terminated mid-native-call). The README already says to put rate
  limiting in front before exposing it. This change does not add process isolation.
- **The Docker build context must be the repository root**, since the image copies `packages/`,
  `bindings/node`, `runtime/typescript`, `crates/` and `src/vscode`.
- **nxlang.org is registered at Cloudflare; services run on Railway.** Nothing else answers on the
  domain yet, and the docs site stays on GitHub Pages for now.
- **OpenSpec's delta format cannot rename a capability**, so the old spec is emptied by a REMOVED
  delta and a new one is added.

## Goals / Non-Goals

**Goals:**
- Every address the site serves is under `/playground`, so a later split of the domain across
  services is a Cloudflare change, not a code change.
- One shared source of truth for the prefix, read by the server, the Vite config and the client.
- A hung process is replaced automatically rather than staying up and answering nothing.
- Cache behavior is decided by origin headers, so it holds whichever edge sits in front.
- The one-time hosting and edge setup is written down well enough to redo from scratch.

**Non-Goals:**
- Shareable edited source (URL-encoded or stored). Left for a follow-up; the URL scheme leaves
  `/playground/<id>` free of query and fragment for it.
- Running the compiler in a child process or in the browser. The health-check restart is the
  mitigation for now; the README's structural note stays.
- A home page at `nxlang.org/`. The root redirects.
- Moving the docs site to nxlang.org, PR preview environments, observability beyond Railway logs
  and Cloudflare Web Analytics.
- Any change to the packages the site depends on, the catalog, or the examples' NX.

## Decisions

### D1. The app lives at `sites/playground` as `@nx-lang/playground`
`sites/*` joins the workspace globs and `sample-apps/drawnui-react` leaves them; the empty
`sample-apps` directory is removed. "Sites" rather than "apps" because in NX's own vocabulary an
"app" is likely to mean a client-side program written in NX, and this is a web property. The
package stays `private`; the scoped name is for consistency with the workspace, not for publishing.

*Alternative*: leave the directory and only rebrand. Rejected: a public site is not a sample app,
and every doc that points at it would carry the wrong frame.

### D2. One base-path module, imported by all three sides
A small module `sites/playground/base.mjs` exports `BASE_PATH = "/playground"` and derived
constants (`API_PREFIX = "/playground/api"`, the shell's `<base href>`, the health path). The server
and `vite.config.ts` (`base: BASE_HREF`) import it; the config also injects the shell's `<base href>`
from it through `transformIndexHtml`, so `index.html` does not spell the prefix either. The client
gets it through Vite's `import.meta.env.BASE_URL`, which Vite fills from `base`, so the bundle does
not need to import a server file; `src/paths.ts` derives the site root and API root from that, and
the pure route table in `src/routes.ts` takes the root as a parameter so it can be unit-tested under
Node without Vite. Fonts move from absolute `/fonts/...` to base-relative; the examples'
`images/baboon.jpg` is already relative and resolves against `<base href>`.

*Alternative*: make the prefix an environment variable. Rejected: nothing needs it to vary, and a
variable read in three places is the drift `server/port.mjs` was created to end.

### D3. Route layout
```
GET  /                          302 → /playground
GET  /playground, /playground/  shell
GET  /playground/<id>           shell (client router resolves <id>; unknown id shows gallery)
GET  /playground/<asset path>   file from dist/, 404 if named with an extension and missing
POST /playground/api/compile    compile
POST /playground/api/language/* language handler (unchanged; the mount prefix moves)
GET  /playground/api/health     { "ok": true }
anything else                   404 JSON
```
The redirect is temporary (302) because the root will become a home page; a permanent redirect
would be cached by browsers past that day. Requests outside the prefix get 404, not the shell, so a
misrouted Cloudflare rule shows up as an obvious error rather than a second copy of the site.

The router regex becomes `^/playground(?:/([A-Za-z0-9-]+))?/?$`. The editor view's "back" link
`href` becomes the gallery path so it still works without JavaScript's handler.

### D4. A stuck process ends itself; the platform restarts it
Railway's health check runs only while a deployment is starting — its docs say the endpoint is
"not used for continuous monitoring" — so it can gate the switch of traffic to a new deployment
but cannot replace a process that hangs later. The service therefore watches itself
(`server/watchdog.mjs`): the main thread writes a heartbeat into shared memory once a second, and a
worker thread, which keeps running while the main thread is inside a native call, reads it back.
When the heartbeat is older than a 20-second deadline the worker kills the process with `SIGKILL`
— nothing gentler can preempt a native call, and a gentler signal would be delivered to the thread
that is not listening. The exit is a failure, and Railway's restart policy starts a fresh process.
The worker's log line is written straight to the descriptor, since a worker's `console` is routed
through the main thread. Staleness is measured on the monotonic clock (`process.hrtime.bigint()`,
one origin for every thread), not the wall clock, so a forward clock step cannot look like a hang.

`/playground/api/health` stays, answered inline on the main thread with no I/O. Railway polls it
before switching traffic to a new deployment, so a build that starts but cannot serve never
replaces a working one; it is also the address a person or an external monitor checks, and because
it is answered from the thread the native calls block, a stuck process cannot lie about being
fine. Railway config: `healthcheckPath: /playground/api/health`, `healthcheckTimeout` of a minute
(the process is serving within seconds; the margin is for a slow container start),
`restartPolicyType: ALWAYS`. Not `ON_FAILURE` with a retry budget: Railway does not document the
budget resetting while a deployment stays live, so a bounded count would leave the service down
after the budget's last hang, which is the outcome the watchdog exists to prevent. The budget is
not what protects against a broken image either; the health check keeps one from going live.

*Alternative*: rely on the platform's health check to restart, as first designed. Rejected once
Railway's docs showed the check is deploy-time only. *Alternative*: a child-process compile with a
kill timer. Better isolation — one visitor's hang would cost one request, not a restart — but a
larger change than the first cut warrants; recorded in `specs/future.md` as the next step.

### D5. Cache policy from origin headers
`server/index.mjs` sets `cache-control` by what it is serving:

| What | Header |
|---|---|
| `dist/assets/*` (Vite content-hashed: JS, CSS, the CanvasKit `.wasm`) | `public, max-age=31536000, immutable` |
| other files under `dist/` (fonts, images, favicon) | `public, max-age=86400` |
| the shell (`index.html`, for every address that resolves to it) | `no-cache` |
| `/api/*` | `no-store` |

Cloudflare honors origin `cache-control` for extensions it considers static, and a Cache Rule for
`/playground/assets/*` makes eligibility explicit for anything it would not cache by default. The
`immutable` line is what makes a deploy safe: a new build changes the hashes, the shell is fetched
fresh, and old assets can stay cached forever.

### D6. GitHub Actions deploys with `railway up`; the service is configured as code
Railway's `railway.json` config-as-code is deprecated (the API refuses to set a config-file path
and points at Infrastructure as Code; existing files stop working on 2026-12-01), so the service is
declared in `.railway/railway.ts` at the repository root — Railway allows one such file per
project, which is why it is not beside the site. It declares the `playground` service with no
source of its own (`empty()`), `build.builder = DOCKERFILE`, `build.dockerfilePath =
sites/playground/Dockerfile`, and the deploy fields from D4. The service's **root directory stays
the repository root** (otherwise the Docker context would be the app directory and the copies
would fail). `railway config plan` previews and `railway config apply` applies it; the `railway`
npm package, a root dev dependency, supplies the DSL. The custom domain cannot be declared in the
file — Railway rejects it — so it is the one Railway setting added by hand.

Deploys come from `.github/workflows/deploy-playground.yml`, on pushes to `main` that touch the
site or something the image copies (a `paths` filter, standing in for Railway's watch patterns):
build and test the workspace on the runner, upload the checkout with `railway up --ci` under a
project token held in the `production` GitHub environment, wait for the deployment to report
success, and smoke-test the health endpoint and the gallery through Cloudflare. This is how the
other Railway-hosted sites deploy, and the playground follows them: the Railway GitHub App is not
installed on the organization and the service has no repository trigger, so `main` is deployed by
the workflow alone, a failing test stops a deploy before Railway builds anything, and the failure
summary names the deployment to roll back to.

*Alternative*: Railway's own GitHub integration — a repository source and a deploy-on-push trigger,
with watch patterns. Needs the Railway GitHub App installed on the organization, deploys without
running the tests, and differs from every other site; not worth a second model for one service.

*Alternative*: GitHub Actions builds to GHCR and Railway pulls the image. More moving parts and a
registry credential to keep; only worth it if Railway's builder proves too slow or too small for the
Rust build. The design leaves that path open — nothing here depends on who runs `docker build`.

### D7. Cloudflare fronts the apex
- DNS: proxied `CNAME nxlang.org → <service>.up.railway.app` (Cloudflare flattens the apex CNAME);
  `www` is a proxied CNAME to the same, with a Redirect Rule sending `www.nxlang.org/*` to the apex.
  Railway's custom-domain screen supplies the exact target and verifies it.
- SSL/TLS: **Full**, not Full (strict). Railway's docs require Full behind the Cloudflare proxy:
  for a proxied domain Railway may fall back to its default `*.up.railway.app` certificate when it
  cannot issue or renew one for the hostname, and strict would answer 526 on every page until that
  cleared. Full still encrypts edge-to-origin traffic; Flexible would loop with Railway's own HTTPS
  redirect. "Always Use HTTPS" on, so `http://nxlang.org/playground` redirects at the edge.
- Rate Limiting Rule on `/playground/api/*`: a per-IP ceiling well above what one editor session
  produces (the client debounces compiles and caches language analysis per document) but far below
  what would starve the single thread. The number is in the setup doc, not the code.
- Cache Rule: eligible for cache on `/playground/assets/*`, `/playground/fonts/*`,
  `/playground/images/*`; respect origin TTL.
- Bot Fight Mode on; Cloudflare Web Analytics on (cookie-less, so no consent banner).
- The service has no Railway-generated public domain, and is not given one: it would answer
  outside Cloudflare, where none of the rules above apply.

### D8. Branding and naming inside the app
- Gallery heading: **NX Playground**. Subtitle: a line that says the playground draws with DrawnUI
  today and that every card is NX compiled and drawn live. The intent is that the heading is the
  site and the subtitle is the current target, so adding a second target later changes the subtitle
  and the gallery, not the identity.
- Document titles: `NX Playground` and `<Example> — NX Playground`.
- The gallery card affordance reads "Edit →"; the editor view keeps "← Gallery".
- Code: `src/fiddle/Fiddle.tsx` becomes `src/editor/EditorView.tsx` beside the Monaco widget
  `NxEditor.tsx`; the route kind `"fiddle"` becomes `"editor"`. Coverage-chip wording stays derived
  from the shared capability vocabulary but is phrased for a visitor ("Static: the original animates
  this" rather than a maintainer's gap note); the vocabulary itself does not change.
- The server's startup line, the README, `docs/CATALOG.md` and `docs/FINDINGS.md` inside the app
  say "playground".

### D9. Spec transition
The `drawnui-fiddle` delta REMOVES every requirement with a pointer to its successor; the new
`playground` spec carries each forward under the site's vocabulary and adds: path prefix, root
redirect, branding, health, cache policy, committed deployment config, edge constraints, and
deployment docs. Archiving leaves an empty `drawnui-fiddle` spec, which the archive task deletes.

## Risks / Trade-offs

- **Railway's builder is slow or short of memory for the Rust build** → watch the first deploy's
  build log; if it fails or takes far too long, switch to D6's alternative (GHCR) without touching
  the app.
- **Railway's build context ignores the root `.dockerignore`** → the build would copy
  `node_modules`, `dist` and `target` from the checkout, which are absent in a clean CI checkout
  anyway, so the image still builds; the root file is the one ignore file for this image (the
  context is the repository root), so verify on the first deploy that the context is small.
- **One slow native call still stalls every visitor until the watchdog fires** → the rate limit
  bounds how often that can be provoked, and the watchdog's deadline plus the restart bounds how
  long it lasts. Accepted for the first cut; process isolation is the follow-up.
- **A restart drops in-flight requests** → the client already treats a compile that does not answer
  within 8 seconds as a reported failure and keeps the last good drawing, so a visitor sees one
  failed compile, not a broken session.
- **Cloudflare cache serves a stale shell after a deploy** → the shell is `no-cache` and not in the
  Cache Rule's paths, so the edge revalidates it every time; only hashed assets are held.
- **Splitting the domain later needs a Host-header override or a Worker** → not needed now; the
  code side (everything under the prefix) is already done by this change, so the later work is
  edge-only. Confirm the Cloudflare plan supports Origin Rules' host override before choosing
  between the two at that time.
- **Base-path mistakes are easy to miss in dev** → the Vite dev server also serves at
  `/playground/` and proxies `/playground/api`, so a hard-coded root path breaks locally before it
  ships. The server tests assert the redirect, the prefix, the 404 outside it and the cache headers,
  against a stand-in `dist/` named by `PLAYGROUND_DIST` so they need no build.

## Migration Plan

1. Land the code change on `main` (move, prefix, health, headers, branding, docs).
2. Create the Railway project and an empty service (`railway init`, `railway add --service
   playground`), apply `.railway/railway.ts` with `railway config apply`, create the project token
   and store it as `RAILWAY_TOKEN_PRODUCTION` on the `production` GitHub environment, and let the
   workflow's first run on `main` build and deploy.
3. Add the custom domain in Railway, then the Cloudflare DNS records, SSL mode, rules and analytics
   per `docs/deployment-setup.md`.
4. Verify from outside: `https://nxlang.org/playground` serves the gallery, `/` redirects,
   `/playground/api/health` answers, asset responses show `cf-cache-status: HIT` on a second
   request, and the API path returns 429 under a burst.
5. Rollback: Railway keeps previous deployments; "redeploy" the last good one from the dashboard.
   Nothing on Cloudflare needs to change for a rollback.

## Open Questions

None remaining. The subtitle copy ("Drawing with DrawnUI today.") is in the gallery, and the
rate-limit threshold (100 requests per 10 seconds per IP on `/playground/api/`) is in
`docs/deployment-setup.md`; both were chosen during implementation and can be changed without
touching the specs or tasks.
