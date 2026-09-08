## Why

NX has no public place where someone can try the language without cloning the repository. The
DrawnUI fiddle under `sample-apps/drawnui-react` already runs the whole pipeline — editor, compile,
IR, evaluation, drawing — as one deployable service, so promoting it to a public site is the shortest
path to a playground at `https://nxlang.org/playground`. The site will grow beyond DrawnUI later;
this change puts the first cut, which is the fiddle as it stands, on the public address.

## What Changes

- **Move and rename the app.** `sample-apps/drawnui-react` becomes `sites/playground`, the package
  `drawnui-fiddle` becomes `@nx-lang/playground`, and the empty `sample-apps` directory goes away.
  "Fiddle" leaves the code, the UI, the URLs and the documentation.
- **Rebrand the UI as the NX Playground.** The heading names the playground; a subtitle says that
  what it draws today is DrawnUI, so the site reads as a general NX playground that currently has one
  target. The "open in fiddle" affordance becomes an "edit" affordance. Coverage chips stay, worded
  for a visitor rather than for a maintainer.
- **Build for the `/playground` base path.** Gallery at `/playground`, an example's editor at
  `/playground/<id>`, the API under `/playground/api/`, assets under the same prefix. The root `/`
  redirects to `/playground` until nxlang.org has a home page. A future split of the domain across
  services needs no code change because nothing the app serves lives outside its prefix.
- **Add a health endpoint** at `/playground/api/health` so the hosting platform can detect a process
  that is stuck in a synchronous native call and replace it.
- **Set cache headers at the origin** so the edge caches hashed assets and the CanvasKit binary and
  never caches the shell or the API.
- **Deploy to Railway behind Cloudflare.** A GitHub Actions workflow builds and tests each push to
  `main`, then uploads it with `railway up` — the same shape as the other Railway-hosted sites —
  and Railway builds the existing Dockerfile from the repository root, with a health check and
  restart policy in a committed Railway infrastructure-as-code file.
  Cloudflare proxies `nxlang.org`, terminates TLS (Full mode, as Railway requires behind a proxy),
  rate limits the API path,
  and caches assets. The one-time Cloudflare and Railway setup is written down in the deployment
  docs.
- **Retire the `drawnui-fiddle` spec** in favor of a `playground` spec that carries its requirements
  forward under the new name and adds the site's own.
- **BREAKING** for anyone with the old paths in muscle memory: the app directory, the package name,
  the URL scheme (`/fiddle/<id>` is gone) and the API paths all change. Nothing outside this
  repository depended on them.

## Capabilities

### New Capabilities
- `playground`: the public NX Playground site — the gallery and editor carried over from the fiddle,
  its URL scheme under `/playground`, the root redirect, the health endpoint, cache behavior, the
  branding, and its public deployment.

### Modified Capabilities
- `drawnui-fiddle`: every requirement is removed; the `playground` capability supersedes it. The
  spec directory is deleted when this change is archived.

## Impact

- **Code**: `sample-apps/drawnui-react/**` moves to `sites/playground/**`. Touched inside it: the
  server's routing, static serving and redirect; the Vite config's base and proxy; the client router,
  compile client, font loading and the two views; `index.html`; the Dockerfile's copy paths; the
  scripts and tests that name paths or ports; the README and the two docs under `docs/`.
- **Workspace**: `pnpm-workspace.yaml` gains `sites/*` and loses the sample-apps entry; the
  lockfile's importer path changes.
- **Repository docs**: `docs/deployment.md` and `docs/deployment-setup.md` gain the playground's
  deploy runbook and one-time setup.
- **New files**: a Railway infrastructure-as-code file at `.railway/railway.ts` (with the `railway`
  SDK as a root dev dependency); a deploy workflow at `.github/workflows/deploy-playground.yml`; a
  shared base-path module; a watchdog.
- **Infrastructure**: a Railway service and a Cloudflare zone configuration, both set up by hand once
  and described in the setup doc, plus a Railway project token held in the `production` GitHub
  environment for the workflow. The Railway GitHub App is not used.
- **Not touched**: the packages the app depends on (`@nx-lang/sdk-node`, `ir-runtime`,
  `language-http`, `language-client`, `monaco`), the catalog, the examples, and the vendored DrawnUI
  source.
