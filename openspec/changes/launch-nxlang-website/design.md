## Context

- **Docs**: Astro 4 + Starlight 0.27 in `docs/`, with its own pnpm lockfile, built with
  `base: '/nx'` and published to GitHub Pages by `deploy-docs.yml`. Code blocks are highlighted
  with the shared TextMate grammar at `src/vscode/syntaxes/nx.tmLanguage.json`. The same folder
  holds internal runbooks and proposals that are not part of the site.
- **Playground**: `sites/playground`, a Vite/React app under the `/playground` prefix. Its Node
  server (`server/index.mjs`) does four things: it serves `dist/` with cache headers, redirects `/`,
  answers `/playground/api/health`, and runs a watchdog. Compilation already happens in the browser
  (`add-wasm-sdk`), so none of that needs a process. It is deployed by `railway up` from
  `deploy-playground.yml`, behind Cloudflare, which holds the `nxlang.org` zone.
- **CI**: the `rust` job of `build.yml` runs `pnpm -r build` (the wasm module included) and then
  `pnpm -r test`, so any workspace package's `test` script runs with the compiler available.
- `specs/future.md` already records "A static host, without the Node server" and "Splitting the
  domain across services" as open. This design settles both.

## Goals / Non-Goals

**Goals:**
- One domain, two independently deployed static sites, with no origin process and no Railway.
- Docs that cannot silently rot: code blocks compiled in CI, links checked in the build.
- A Starlight site upgraded and branded, but still recognizably Starlight. We keep its layout,
  search and sidebar rather than design our own.

**Non-Goals:**
- Replacing the playground's DrawnUI gallery, its value output, or the "Open in playground" buttons.
  Those belong to `rebuild-language-playground`, which follows this change. Until then the header's
  Playground link opens the playground as it is.
- Line-precise error annotations in docs blocks, where each `// error` comment would have to match a
  reported line. `invalid` means "at least one error" for now.
- Per-page generated social images, versioned docs, and translations.
- A redirect from `nx-lang.github.io/nx`.

## Decisions

### Starlight stays; the site moves to `sites/website` at the domain root

Web properties in this repository live under `sites/`, and `docs/` goes back to being internal
documentation. The site joins the root pnpm workspace (`sites/*` is already listed), which drops its
second lockfile and lets its tests import `@nx-lang/sdk-wasm` from the workspace. Starlight and Astro
move to their current releases in the same step, because the plugins below need them and the
upgrade touches the same files anyway.

The documentation sits at root paths (`/language-tour/types/`), and the landing page is Starlight's
`splash` template at `/`, not a separate app. Every internal link already in the content is written
that way. The alternative, docs under `/docs/`, would need all 67 of those links rewritten, and would
add a second site structure to maintain in exchange for nothing.

*Alternatives:* VitePress or Docusaurus, which would mean migrating content for no gain. A custom
Astro landing page beside Starlight, which is two layouts to keep consistent. The splash template
with a few custom components is enough.

### Two assets-only Cloudflare Workers, routed by path

| Worker | Routes | Serves |
|---|---|---|
| `nxlang-website` | `nxlang.org/*` | `sites/website/dist`, with `not_found_handling: "404-page"` and `html_handling: "auto-trailing-slash"` |
| `nxlang-playground` | `nxlang.org/playground`, `nxlang.org/playground/*` | `sites/playground/dist`, whose files sit under `playground/` so request paths map straight onto them |

Cloudflare sends each request to the most specific matching route, so the playground's routes win
under its prefix and the website takes everything else. Each Worker's `wrangler.jsonc` sits beside
its site and declares its routes (`zone_name: "nxlang.org"`) and `workers_dev: false`, so no
hostname answers outside the zone. Each deploys on its own: a docs edit never rebuilds Rust, and a
compiler change never redeploys the docs.

*Alternatives:*
- **One Worker for both**: every docs deploy would have to carry the playground's build, so it would
  build Rust.
- **Keep Railway for the playground and route `/` elsewhere**: this keeps a container whose only job
  is serving files, and routing by path to two origins needs a Worker or Origin Rules anyway.
- **Cloudflare Pages**: Cloudflare is moving Pages' features into Workers static assets, and Pages
  cannot share one hostname by path the way Worker routes can.
- **The playground's Node server serving the docs too**: this couples docs deploys to the Docker/Rust
  build and a container restart.

### The playground's client routing is a Worker script; its headers are a `_headers` file

The playground must serve its shell for `/playground` and `/playground/<id>`, and must answer 404
for a missing asset (the scenario "A missing asset is not the shell"). A `_redirects` rewrite to
`index.html` cannot reliably tell those apart, because a rewrite rule can also catch real files. So
the Worker gets a script of a few lines that runs only when no asset matched (the default, with
`run_worker_first` off):

- a path of exactly one segment under the prefix, or the prefix itself, answers with the shell,
  fetched through the `ASSETS` binding;
- anything else answers 404.

The script is unit-tested like `routes.ts` is today. The cache policy the Node server applied moves
unchanged into `sites/playground/_headers`, which the build copies to `dist/_headers`. It is not in
`public/`, because Vite copies `public/` under the `/playground` prefix, where Cloudflare doesn't
read it and would serve it as a file. A unit test reads it and checks each rule:

- `immutable, max-age=31536000` for `/playground/assets/*`;
- `max-age=86400, must-revalidate` for fonts and images;
- `no-cache` for the shell.

### DNS points at nothing, and the edge answers

Worker routes run only on proxied hostnames, and neither site has an origin. So once both Workers
serve, the apex and `www` records change from CNAMEs to Railway into proxied `AAAA 100::` records,
the documented placeholder for a Worker-only hostname. The existing `www` to apex redirect rule and
Always Use HTTPS run before Workers and stay. The `playground assets` cache rule is deleted, because
Workers static assets are served from Cloudflare's own storage, not through the zone cache.

### Deploys: build, test, `wrangler deploy`, smoke test

- **`deploy-website.yml`** runs on pushes to `main` touching `sites/website/**`, the grammar, or
  itself. It installs only the site's dependencies, runs `astro build` (the link validator runs in
  the build), then `wrangler deploy`. Last, it fetches `/` and one docs page and checks for a
  build-stamped meta tag, so it fails if the old build is still serving.
- **`deploy-playground.yml`** keeps its build-and-test job. Its `paths` list loses what only the
  Docker image copied (`bindings/node/native/**`) and narrows `scripts/**` to
  `scripts/fetch-wasi-sysroot.mjs`, the one script the wasm build reads. Its deploy job changes from
  `railway up` plus deployment polling to `wrangler deploy`, then fetches the shell and the hashed
  compiler module.

A Workers deploy is atomic: the new version serves only once every asset is uploaded. That answers
the question `future.md` asked about what replaces Railway's health-check gate: the tests before the
deploy, the atomic switch, and the smoke test after it. Both workflows authenticate with
`CLOUDFLARE_API_TOKEN` (Workers Scripts Edit, Workers Routes Edit on the zone) and
`CLOUDFLARE_ACCOUNT_ID`, held on the `production` GitHub environment, which is already restricted to
`main`. Rollback is `wrangler rollback` per Worker.

### The code-block check is the website's `test` script

`sites/website/scripts/check-code-blocks.mjs` walks the content files. It parses each Markdown/MDX
file with the same Markdown parser Astro uses, so fences are found the way the site renders them,
and checks every `nx` fence through `@nx-lang/sdk-wasm`'s Node entry, each block as a module of its
own. How a block passes depends on its fence:

| Fence | Passes when |
|---|---|
| unmarked | the block produces no error diagnostics |
| `fragment` | no parser or syntax-validation diagnostics; type errors from names declared elsewhere on the page are ignored |
| `invalid` | at least one error diagnostic |

It runs as `pnpm --filter @nx-lang/website test`, so the CI `rust` job's `pnpm -r test` picks it up
with the freshly built module. Failures print as `file:line: message`.

Treating each block as a separate module is deliberately strict. Joining a page's blocks would let a
reader copy a snippet that doesn't work on its own. A block that genuinely builds on an earlier one
says so with `fragment`. The meta words are plain Markdown fence metadata, which Expressive Code
ignores when rendering. A task confirms they don't render.

The deploy workflow doesn't run the check, because it needs Rust. Docs reach `main` through pull
requests, where CI runs it.

### Plugins over hand-rolled code

- **`starlight-links-validator`** fails the build on a missing page or heading anchor.
- **`starlight-llms-txt`** emits `/llms.txt`, `/llms-full.txt` and `/llms-small.txt`.
- **Search** is Starlight's built-in Pagefind, which is static and runs in the browser.

### Header and branding

We override Starlight's `Header` component, keeping its search, theme toggle and mobile menu, and add
text links for Docs and Playground next to a GitHub icon from Starlight's `social` config. The
playground doesn't share this component in this change. `rebuild-language-playground` gives it a
matching bar.

Branding means:
- an SVG logo mark and favicon;
- an accent color set through Starlight's `--sl-color-accent*` properties, in both themes;
- one static 1200×630 social card at `/og.png`, referenced from every page through Starlight's
  `head` config.

Code blocks keep dark-plus/light-plus, so they look the way they do in VS Code.

### Content work

- **Rewrite the landing page.** It has a hero whose snippet shows a record, a component and a `for`,
  and shows the evaluated value in a second, hand-written code block (the next change checks it). It
  has feature cards: what NX is for, how it's typed, and where it runs (.NET, JS, the browser). It
  links to Getting Started, `/playground` and `https://fiddle.drawnui.net`. Task 5.1 settled on the
  fiddle's root rather than an NX preset's address.
- **Rewrite Getting Started** in this order:
  1. Try the playground.
  2. Install the VS Code extension from the Marketplace or Open VSX (`nx-lang.nx-language`), which
     this change publishes for the first time (see below).
  3. Use NX from .NET (`dotnet add package NxLang.Sdk`).
  4. Use NX from JavaScript (`npm install @nx-lang/sdk-wasm @nx-lang/ir-runtime`).
  5. Next steps: the language tour.

  The current page's build-from-source steps, and the CLI, move to Contributing.
- **Fix every block the check flags,** starting with the five `type <X .../>` pages. Each block is
  rewritten in current syntax, or marked `fragment` or `invalid` where that is what it is.

### The VS Code extension is published through the release track that already exists

Getting Started can only offer an editor step a user can take, so the extension is published. The
pipeline is in place and specified (`vscode-extension-publishing`): a `vscode-v*` tag builds and
verifies per-platform VSIX files into a draft GitHub Release, and publishing that release runs
`vscode-extension-publish.yml` against both registries. It has never run, because the accounts
behind it don't exist. What this change adds:

- **Accounts, by hand.** A Marketplace publisher `nx-lang`, and an Open VSX namespace `nx-lang`,
  which needs an Eclipse account and the signed publisher agreement. The Open VSX token goes on
  `production` as `OVSX_PAT`.
- **No Marketplace token.** The Marketplace token would have to be a global Azure DevOps personal
  access token, and Azure DevOps retires those on 2026-12-01, two months after this release. So the
  publish workflow signs in through GitHub OIDC as a user-assigned managed identity,
  `nx-vscode-publisher`, and publishes with `vsce publish --azure-credential`. The identity lives in
  an Azure subscription `nx-lang` that Forward Reach pays for; it costs nothing, and it is a member
  of the publisher, so replacing it (for example when the project moves to a community-owned
  account) is a new identity and a member change. An app registration would need no subscription,
  but the Marketplace refuses its publishes. The Marketplace only accepts the identity's Azure
  DevOps id as a member, so a small manual workflow, `marketplace-identity.yml`, prints it, and
  checks with `vsce verify-pat` that the identity may publish.
- **Publishing per platform.** The release has one VSIX per platform, all at the same version. The
  publish script used to skip a VSIX when the registry already listed its version, which would have
  published the first platform and skipped the other two. It now passes `--skip-duplicate`, which
  both registries apply per version and target.
- **A listing for users.** Both registries show the extension's README as its page. Today that
  README opens with contributor setup (nvm, pnpm, building the language server), so it is rewritten
  for someone installing the extension: features, settings, and a link to `nxlang.org`. The
  maintainer material moves to `src/vscode/CONTRIBUTING.md`. The same README is also the
  `@nx-lang/language` npm package's README, so its section on the editor-assets package stays,
  below the extension's. The manifest gains a 128×128 PNG icon made from the site's logo mark; the
  Marketplace doesn't accept SVG icons.
- **The first release is `vscode-v0.1.0`,** the manifest's current version, following the runbook
  in `docs/deployment.md`. The tag is pushed by a maintainer.

Until the release is out, Getting Started says the extension is coming and points to Contributing.
Its editor step becomes an install only after the listing is live, so the site never names an
extension that can't be installed. The cutover doesn't wait for it.

*Alternative:* say "coming soon" and point to building from source. That leaves the first editor
step of Getting Started as a clone and a Rust build, which is what this change sets out to remove.

## Risks / Trade-offs

- **[An assets-only Worker can't claim a zone route, or the more specific route doesn't win the way
  the table assumes]** → The first task deploys both Workers with routes on a scratch path pattern
  and checks precedence before cutover. The fallback is a single website Worker script that, under
  `/playground`, forwards to the playground Worker through a service binding. Nothing else in this
  design changes.
- **[Cloudflare Web Analytics' injected beacon may not reach Worker-served HTML]** → Check for
  `cloudflareinsights` in the HTML after cutover. If it's missing, add the beacon snippet to both
  sites' `<head>`. It is cookie-less, so no consent banner is needed.
- **[The code-block check fails on dozens of blocks on day one]** → Expected. That is the point of
  the check. Fixing the blocks is scoped as its own task group, and the check lands together with
  the fixes, so `main` never goes red.
- **[The Starlight major upgrade breaks the content collection config or the Expressive Code
  options]** → The upgrade is its own task, verified by a clean build before any content changes.
- **[Open VSX namespace ownership takes time]** → Creating the namespace is immediate, but the
  verified-owner badge needs a request to the Open VSX maintainers. Publishing doesn't wait on it.
  If the Marketplace publisher can't be created as `nx-lang`, the extension's `publisher` field and
  its id change everywhere it's named, so this is settled first.
- **[Losing Railway loses nothing we use today]** → True as of this change. If NX later needs a
  server (stored shares, a language service over HTTP), it can be a Worker with storage or a new
  service behind a route. The routing already supports either.

## Migration Plan

1. Before any deploy, prove route precedence with the throwaway spike (task 1.1), and serve both
   builds locally with `wrangler dev`: shell routing, the 404s, the headers.
2. **Cutover:** each `wrangler.jsonc` declares its routes, so the first deploy of each Worker creates
   them. The deploy workflows need `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`, so until those
   secrets exist a merge to `main` deploys nothing (the deploy jobs fail at the upload). With the
   secrets set, running both workflows is the cutover. DNS still points at Railway, but routes
   intercept at the edge, so Railway just stops receiving traffic. **Rollback:** delete the two
   Workers' routes in the dashboard. Railway is still running and serves again at once.
3. Turn `workers_dev` off. Delete the `playground assets` cache rule. Check the analytics beacon.
4. After a quiet week, change the apex and `www` records to proxied `AAAA 100::`, remove the Railway
   custom domain, and delete the Railway project and the `RAILWAY_TOKEN_PRODUCTION` secret. The
   repository's `.railway/`, Dockerfile and server are already deleted in the PR.
5. Unpublish GitHub Pages and delete the `github-pages` environment. The PR deletes
   `deploy-docs.yml`, since `docs/` no longer holds the site.
6. Independently, publish the VS Code extension (`vscode-v0.1.0`), then change Getting Started's
   editor step to an install.
