# Deployment Setup

This checklist covers the one-time setup for publishing NX packages and VS Code extension artifacts
from GitHub Actions, and for hosting the playground site. The ongoing runbook lives in
[deployment.md](deployment.md).

## GitHub Environment

Create one GitHub environment:

- `production`: used only by workflows that publish already-reviewed GitHub Release assets to
  NuGet.org, npm, the Visual Studio Marketplace, and Open VSX.

Recommended protection:

- Add required reviewers before enabling public registry writes.
- Restrict deployments to protected release branches or tags as appropriate for the repository.
- Keep pull request and `main` build workflows artifact-only; they do not need production secrets.
- Audit environment deployment history after each published release.

## Registry Ownership

Set up ownership before enabling publication:

- NuGet.org: reserve or own `NxLang.Sdk`.
- npm: own the `@nx-lang` scope and the `@nx-lang/language` package.
- Visual Studio Marketplace: own publisher `nx-lang` and extension `nx-language`.
- Open VSX: own namespace `nx-lang` and extension `nx-language`.

## Trusted Publishing

Prefer trusted publishing where the registry supports it:

- NuGet.org: create a trusted publishing policy for repository `nx-lang/nx`, workflow file
  `package-publish.yml`, and the `production` environment. Set `NUGET_USER` as a production
  environment secret for the NuGet owner account used by `NuGet/login`.
- npm: create a trusted publisher for `@nx-lang/language` that matches repository `nx-lang/nx`,
  workflow `.github/workflows/package-publish.yml`, and environment `production`.

The package publish job requests GitHub OIDC with `id-token: write` only after a package GitHub
Release is published and its release assets are validated.

Visual Studio Marketplace and Open VSX publishing currently use production environment token secrets
in `.github/workflows/vscode-extension-publish.yml`.

## Secrets And Variables

Production environment secrets:

- `NUGET_USER`: NuGet.org account or organization owner used by NuGet trusted publishing.
- `NUGET_API_KEY`: fallback NuGet.org API key when trusted publishing is unavailable.
- `VSCE_PAT`: Visual Studio Marketplace token for publisher `nx-lang`.
- `OVSX_PAT`: Open VSX token for namespace `nx-lang`.

No preview NuGet, preview npm, or pull request registry credentials are required. Pull request
testing uses workflow artifacts and PR comments with download/install commands.

The Publish packages workflow accepts `release_tag` for manual repair. Set it to a published package
GitHub Release tag such as `v1.2.3`; the workflow downloads and republishes the attached release
assets without rebuilding package contents.

The Publish VS Code extension workflow accepts `release_tag` for manual repair. Set it to a
published VS Code GitHub Release tag such as `vscode-v1.2.3`; the workflow validates and republishes
the attached VSIX assets without rebuilding package contents.

Rust tool publication for `nxlang`, `nx-lsp`, and Rust crates is not part of this deployment setup
yet; no crates.io token or Rust binary-release credential is required for this release pipeline.

Never commit registry tokens or write them into tracked configuration files.

## Versioning Setup

The repository uses the restored local .NET tool `minver-cli` through
`tools/versions/Get-ReleaseVersion.ps1`.

Supported release tag formats:

- Package releases: `v<major>.<minor>.<patch>`, for example `v1.2.3`.
- VS Code extension releases: `vscode-v<major>.<minor>.<patch>`, for example `vscode-v1.2.3`.

The first implementation intentionally supports stable `major.minor.patch` release tags only. Pull
request and `main` package artifacts receive unique prerelease versions, while VSIX artifacts use
registry-valid `major.minor.patch` versions and are tested by direct VSIX installation.

## First Enablement

1. Confirm PR workflows upload `deployables-Complete`, `editor-assets-package`, and `vscode-vsix-*`
   artifacts without public registry credentials.
2. Confirm the trusted PR artifact comment workflow posts download/install commands without checking
   out or executing pull request code.
3. Push a test package tag in a disposable repository or dry-run branch and confirm `release.yml`
   creates a draft package GitHub Release with `.nupkg`, `.snupkg`, `.tgz`, manifest, and checksum
   assets.
4. Push a test VS Code tag in a disposable repository or dry-run branch and confirm
   `vscode-release.yml` creates a draft VS Code GitHub Release with VSIX, manifest, and checksum
   assets.
5. Enable `production` after release asset validation, package inspection, smoke tests, and publish
   workflow validation pass.
6. Keep `NUGET_API_KEY` empty when NuGet trusted publishing is working. Production npm publishing
   uses trusted publishing only; do not configure an npm publish token for CI.

## Playground Site Hosting

The playground (`sites/playground`) runs as one Railway service behind Cloudflare, which holds the
`nxlang.org` zone. Everything below is done once by hand; the settings that can live in the
repository do (`.railway/railway.ts`), and this section is the record of the rest.

### Railway service

The service is declared in `.railway/railway.ts` at the repository root and applied with the
Railway CLI, so a fresh account needs only the project, an empty service, and one apply.
(Railway's older `railway.json` config-as-code is deprecated and its API refuses to set a config
file path, which is why the declaration is a TypeScript file at the root rather than a JSON file
beside the site.)

Deploys are driven from GitHub Actions, the same way the other Railway-hosted sites deploy:
`.github/workflows/deploy-playground.yml` builds and tests a push to `main`, then uploads it with
`railway up` under a project token. The Railway GitHub App is **not** installed on the `nx-lang`
organization and the service has **no repository source**, so nothing deploys except through the
workflow. (`railway add --repo` and `railway service source connect` are the other model, Railway's
own GitHub integration; it needs the app installed, deploys without running the tests, and is not
what the other sites use.)

```bash
railway login
railway init --name nxlang --workspace "<your workspace>"      # creates the project and links this directory
railway add --service playground                                 # an empty service; the workflow uploads into it
railway config plan                                              # preview: builder, Dockerfile path, health check, restart policy
railway config apply                                             # apply them
```

`railway config plan` may keep reporting two changes as pending after they have been applied:
`source.type` (`null → "empty"`, which is what a service with no source already is) and
`build.watchPatterns`, which the apply does not clear from a service that once had a repository
source. Neither matters: watch patterns only gate repository-triggered deploys, which a service
with no repository source never has. Treat both as a display quirk of the CLI. Every other line
in the plan is real, so a plan that shows only those two is fully applied.

What the file sets, and why: `DOCKERFILE` builder with `sites/playground/Dockerfile` and the
**root directory left at the repository root**, because the Dockerfile copies `packages/`,
`bindings/node`, `runtime/typescript`, `crates/` and `src/vscode`; `healthcheckPath`
`/playground/api/health` with a 60 s timeout, polled only while a deployment starts; and
`ALWAYS` restarts, which is what brings the process back after the watchdog ends it (no retry
budget, so the service cannot run out of restarts; the health check is what keeps a build that
cannot serve from going live). No variables are required: `PORT` is set by the image (8080) and Railway routes to it.
Which pushes deploy is the workflow's `paths` list, not a Railway watch pattern.

### Railway token for GitHub Actions

The workflow authenticates with a project token scoped to the `production` Railway environment,
held as a secret on the `production` GitHub environment — the same arrangement as the other sites.

1. In the Railway dashboard open the `nxlang` project → Settings → Tokens, and create a project
   token named `github-actions-production` scoped to the `production` environment. Copy it.
2. Store it on the GitHub environment (paste when prompted; never put it in a file in the repo):

   ```bash
   gh secret set RAILWAY_TOKEN_PRODUCTION --repo nx-lang/nx --env production
   ```

3. The `production` GitHub environment is restricted to the `main` branch, so no other ref can
   reach the secret. Check with `gh api repos/nx-lang/nx/environments/production`; set it with:

   ```bash
   gh api -X PUT repos/nx-lang/nx/environments/production \
     --input - <<< '{"deployment_branch_policy":{"protected_branches":false,"custom_branch_policies":true}}'
   gh api -X POST repos/nx-lang/nx/environments/production/deployment-branch-policies \
     -f name=main -f type=branch
   ```

4. Validate the token before relying on it — this lists deployments and nothing else:

   ```bash
   RAILWAY_TOKEN='<the token>' railway deployment list --service playground --environment production --json
   ```

### First deployment

Merge to `main` (or run the workflow from the Actions tab), or deploy a checkout by hand from the
repository root with the CLI linked to the project:

```bash
railway up --service playground --environment production --ci
```

The first build compiles the Rust addon, so it is the slowest one; watch the log (the workflow
streams it, or `railway logs --build`) for two things:

- the upload size printed by `railway up` — it should be a few megabytes, since the upload skips
  gitignored paths (`node_modules`, `dist`, `target`). The root `.dockerignore` — the one ignore
  file for this image, since the build context is the repository root — keeps the same paths and
  the vendored demo pages under `sites/playground/reference` out of the context; if the context is
  hundreds of megabytes anyway, the log names what was sent;
- that the deployment becomes healthy and serves `https://nxlang.org/playground`.

If the Rust build fails on the builder for lack of memory or time, switch to the fallback: the
workflow builds the same Dockerfile on the runner and pushes it to GHCR, and the service's `source`
in `.railway/railway.ts` becomes `image("ghcr.io/nx-lang/playground")` instead of `empty()`.
Nothing in the site depends on who runs `docker build`.

### Custom domain

1. Add the domain — the one Railway setting that cannot live in `.railway/railway.ts`:
   `railway domain nxlang.org --service playground`, or Settings → Networking → Custom Domain in
   the dashboard. Railway shows the CNAME target (`<something>.up.railway.app`) and waits for it to
   resolve.
2. In Cloudflare DNS for `nxlang.org` (the values Railway printed when the domain was added; they
   are specific to the service and change if it is recreated):
   - `CNAME @ → z5ddxlp7.up.railway.app`, **proxied** (orange cloud). Cloudflare flattens the apex
     CNAME.
   - `CNAME www → z5ddxlp7.up.railway.app`, proxied.
   - `TXT _railway-verify → railway-verify=a4584a9bc9142248408b29eaff90550422a6a5350b38ac182c387e3b5c9ec963`,
     which is how Railway proves ownership behind the proxy.
3. Railway marks the domain verified once the records resolve (`railway domain status <id>
   --service playground`), then issues its certificate.
4. Leave the service without a Railway-generated domain (`railway domain list --service playground`
   shows only `nxlang.org`). A bare `railway domain` would create one, and it would answer outside
   Cloudflare, where the rate limit and the rest of the rules below do not apply.

### Cloudflare zone settings

| Setting | Value | Why |
|---|---|---|
| SSL/TLS → Overview → encryption mode | **Full** (not strict) | Railway's docs: "If you have proxying enabled on Cloudflare (the orange cloud), you MUST set your SSL/TLS settings to Full -- Full (Strict) will not work as intended." For proxied domains Railway "may not always be able to issue a certificate for the domain" and then serves its default `*.up.railway.app` certificate, which strict would reject with a 526 on every page. Full still encrypts edge-to-origin traffic; Flexible would loop with Railway's own HTTPS redirect. |
| SSL/TLS → Edge Certificates → Always Use HTTPS | **On** | `http://nxlang.org/playground` redirects at the edge before reaching Railway. |
| Security → Bots → Bot Fight Mode | **On** | Cheap protection for a single-threaded origin. |
| Analytics → Web Analytics | **Enable, excluding visitor data in the EU**, for `nxlang.org` (proxy-injected beacon) | Cookie-less, so no consent banner, and the EU exclusion removes the remaining ePrivacy question at the cost of not seeing EU visitors — the same choice as the account's other sites. No snippet in the site. Dashboard only: the API token permission for it is not available on the zone-scoped token used for the rest. |

### Cloudflare rules

**Redirect rule — `www` to apex**

- Name: `www to apex`
- When: Hostname equals `www.nxlang.org`
- Then: Dynamic redirect, expression `concat("https://nxlang.org", http.request.uri.path)`,
  status 301, preserve query string.

**Rate limiting rule — API path**

- Name: `playground api`
- When: URI Path starts with `/playground/api/`
- Characteristics: IP (the API form is `ip.src` plus `cf.colo.id`, which the Free plan requires)
- Rate: **100 requests per 10 seconds** — an editing session produces a few requests per second
  at most (compiles are debounced, language analysis is cached per document), while a client at
  this ceiling could take a fifth of the origin's one thread.
- Action: Block, for the plan's mitigation timeout (10 seconds on Free).

Excess requests are answered 429 at the edge and never reach Railway. Raise the threshold if the
editor grows chattier; the number lives here, not in the site.

**Cache rule — site assets**

- Name: `playground assets`
- When: URI Path starts with `/playground/assets/` OR starts with `/playground/fonts/` OR starts
  with `/playground/images/`
- Then: Cache eligibility **Eligible for cache**; Edge TTL **Use cache-control header from
  origin**; Browser TTL **Respect origin**.

The origin sends `immutable` for a year on hashed assets (which includes the CanvasKit `.wasm`,
which Cloudflare would not cache by extension alone) and a day on fonts and images. The shell
(`/playground`, `/playground/<id>`) and everything under `/playground/api/` are outside the rule and
carry `no-cache` / `no-store`, so the edge revalidates or bypasses them every time.

All of the above except Web Analytics can be applied with the Cloudflare API from a token holding
Zone Read, DNS Edit, Zone Settings Edit, Zone WAF Edit, Cache Rules Edit, Single Redirect Edit and
Bot Management Edit on the zone; the rules are single-rule entrypoint rulesets in the
`http_request_dynamic_redirect`, `http_ratelimit` and `http_request_cache_settings` phases.

### Verify the setup

```bash
curl -sI http://nxlang.org/playground | grep -i location        # https://nxlang.org/playground
curl -sI https://nxlang.org/                                     # 302 → /playground
curl -s  https://nxlang.org/playground/api/health                # {"ok":true}
curl -sI https://nxlang.org/playground/assets/<hashed asset>     # twice: second shows cf-cache-status: HIT
curl -sI https://nxlang.org/playground                           # cf-cache-status: DYNAMIC (never HIT)
curl -s -H 'accept: text/html' https://nxlang.org/playground | grep -c cloudflareinsights   # 1: the beacon is injected
#   the edge injects the Web Analytics beacon only into responses to requests that accept HTML,
#   so a bare curl shows none and proves nothing
seq 1 200 | xargs -P 40 -I{} curl -s -o /dev/null -w "%{http_code}\n" -X POST \
  -H 'content-type: application/json' -d '{"source":""}' https://nxlang.org/playground/api/compile | sort | uniq -c
#   mostly 200, then 429 once the rate limit engages; the requests must be parallel, since one
#   curl after another from outside the datacenter stays under 100 in any 10 seconds
```

Open `https://nxlang.org/playground` in a browser without certificate warnings, open an example,
edit it, and confirm hover answers. Cloudflare's Web Analytics should show the visit within a few
minutes.
