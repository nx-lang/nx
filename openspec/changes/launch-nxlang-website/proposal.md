## Why

`https://nxlang.org` redirects to the playground, and the documentation lives somewhere else,
`nx-lang.github.io/nx`, where 67 of its internal links answer 404: they are written from the site
root and the site is built under `/nx`. Five of the pages a newcomer reads first declare records as
`type <User .../>`, which no longer parses, because nothing checks the docs' 165 `nx` code blocks.
The home page says it is under construction, and Getting Started builds the Rust workspace. NX needs
one front door, where the docs lead and the playground is one click away. Nothing about serving it
needs a process any more: the playground compiles in the visitor's browser, and its Node server only
serves files and answers a health check.

## What Changes

- **The documentation becomes the site at `https://nxlang.org`.** The Starlight project moves from
  `docs/` to `sites/website`, joins the root pnpm workspace, and builds at the domain root with no
  base path, so the root-relative links resolve as written. `docs/` keeps the internal runbooks and
  proposals and stops being a site.
- **A landing page at `/`.** It has a hero with an NX snippet beside what it evaluates to, a few
  plain statements of what NX is for, and three ways in: the docs, the playground, and NX drawing
  real interfaces in the DrawnUI fiddle at `https://fiddle.drawnui.net`.
- **One header across the site.** It reads NX · Docs · Playground · GitHub, with search.
- **The docs are checked.** Every `nx` code block compiles in CI unless its fence marks it as a
  fragment (syntax only) or as deliberately invalid (must fail). A broken internal link fails the
  build. The five stale pages, and whatever else the check finds, are fixed.
- **Getting Started is rewritten for someone using NX,** not building it: try it in the playground,
  install the VS Code extension, use NX from .NET or JavaScript. The CLI, which is not published
  yet, and building from source move to Contributing.
- **The VS Code extension is published** to the Visual Studio Marketplace and Open VSX as
  `nx-lang.nx-language`, so Getting Started's editor step is an install, not a build. The release
  pipeline (`vscode-v*` tags, `vscode-extension-publish.yml`) already exists but has never run: the
  registry accounts and tokens don't exist yet. The extension gets an icon, and a README written for
  people installing it, which is also its Marketplace page. The maintainer material in that README
  moves to `src/vscode/CONTRIBUTING.md`.
- **Starlight and Astro move to current releases.** The site gains a logo, favicon, accent color,
  social card, custom 404 page and an `llms.txt`.
- **Both sites are served as static assets from Cloudflare Workers.** `nxlang-website` serves
  `nxlang.org/*`. `nxlang-playground` serves `nxlang.org/playground*`, the more specific route.
  Each deploys from its own GitHub Actions workflow with `wrangler deploy`.
- **BREAKING (hosting): Railway is retired.** The playground's Node server, Dockerfile,
  `.railway/railway.ts`, health route and the `railway up` workflow are deleted. The playground's
  cache policy moves into a `_headers` file. Its client routing, which serves the shell for
  `/playground/<id>`, moves into a Worker script of a few lines. The `/` →
  `/playground` redirect goes, because the website answers `/`.
- **BREAKING (URLs): GitHub Pages is retired.** `deploy-docs.yml` is deleted and the Pages site
  unpublished. Nothing links to the old address that is worth a redirect stub; its internal links
  were broken anyway.
- **Package metadata points home.** Every npm package's `homepage` changes from `https://nx-lang.dev`
  to `https://nxlang.org`, and the README links to the site.

## Capabilities

### New Capabilities
- `website`: the public site at `nxlang.org`. It covers the landing page, the header and navigation,
  the documentation's build and checks (code blocks and links), the Getting Started path, branding,
  the fiddle link, and the site's static hosting and deployment.

### Modified Capabilities
- `vscode-extension-publishing`: adds a requirement that the extension is actually published to
  both registries, under the `nx-lang` publisher and namespace, with a listing written for users.
- `playground`: hosting changes from one Node service on Railway to static assets on a Cloudflare
  Worker. The root redirect, the health route, the single-service and Railway requirements, and the
  origin cache headers are replaced by static equivalents and edge routing. The gallery, editor and
  DrawnUI requirements are untouched here; `rebuild-language-playground` replaces them.

## Impact

- **Moves**: `docs/{astro.config.mjs,starlight.config.mjs,package.json,src,tsconfig.json}` →
  `sites/website/`. The docs' own `pnpm-workspace.yaml` and lockfile go; the site joins the root
  workspace (`sites/*` is already a member).
- **Deleted**: `.github/workflows/deploy-docs.yml`, `.railway/`, `sites/playground/{Dockerfile,server/}`,
  the root `.dockerignore`, and the health watchdog.
- **New**: `sites/website/wrangler.jsonc`, `sites/playground/wrangler.jsonc`, the playground's
  `_headers` and Worker script, the website's `public/_headers`, `.github/workflows/deploy-website.yml`. The
  playground workflow is rewritten from `railway up` to `wrangler deploy`.
- **CI**: the code-block check runs in `build.yml`, where the wasm module is already built. The
  website's own deploy needs no Rust.
- **Infrastructure (by hand, recorded in `docs/deployment-setup.md`)**: a Cloudflare API token and
  account id as `production` environment secrets. Worker routes on the zone. The apex and `www` DNS
  records re-pointed from Railway to a proxied placeholder. The `playground assets` cache rule
  removed. The Railway project deleted once the Workers serve.
- **VS Code extension**: `src/vscode/README.md` rewritten for users, `src/vscode/CONTRIBUTING.md`
  added, an icon added to the manifest, and the first `vscode-v0.1.0` release published. By hand:
  the Marketplace publisher `nx-lang`, the Open VSX namespace `nx-lang`, and the `VSCE_PAT` and
  `OVSX_PAT` secrets on the `production` environment.
- **Docs**: `docs/deployment.md` and `docs/deployment-setup.md` rewritten for the Workers. The
  `specs/future.md` sections "A static host, without the Node server" and "Splitting the domain
  across services" are removed, since this change answers both.
