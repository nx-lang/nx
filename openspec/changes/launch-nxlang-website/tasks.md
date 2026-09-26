## 1. Hosting spike

- [ ] 1.1 In a scratch directory outside the repository, make two assets-only Workers, one with an `index.html` at its root and one with `playground/index.html`. Give them routes on the `nxlang.org` zone for a throwaway prefix (`nxlang.org/spike/*` and `nxlang.org/spike/pg*`) and deploy with a personal token. Verify with `curl` that the more specific route answers under `/spike/pg` and the other answers elsewhere, even though DNS still points at Railway. Then delete both Workers and their routes. If precedence does not hold, switch the design to the service-binding fallback in design.md before going on.

## 2. Move and upgrade the docs site

- [x] 2.1 `git mv` the Starlight project (`astro.config.mjs`, `starlight.config.mjs`, `package.json`, `src/`, `tsconfig.json`) from `docs/` to `sites/website/`. Rename the package `@nx-lang/website` (private). Delete `docs/pnpm-workspace.yaml`, `docs/pnpm-lock.yaml`, `docs/dist` and `docs/node_modules`, fix the grammar path, and `pnpm install` at the root. Verify `pnpm --filter @nx-lang/website build` produces `dist/` and `docs/` holds only internal Markdown.
- [x] 2.2 Set `site: 'https://nxlang.org'` and remove `base`. Verify the build output links `/reference/syntax/types/` rather than `/nx/...`, and that a root-relative content link such as `/reference/syntax/types#ranges` resolves in `astro preview`.
- [x] 2.3 Upgrade Astro and Starlight to their current releases, following the Starlight upgrade notes for the content config (`src/content.config.ts` loaders), Expressive Code options and the sidebar. Change no content in this task. Verify a clean build and that an `nx` block still renders with NX highlighting in both themes.
- [x] 2.4 Update `sites/website/README.md` (was `docs/README.md`) and `docs/README.md` so each describes what its folder now holds. Verify `grep -rn "docs/src/content\|--dir docs" --exclude-dir=node_modules --exclude-dir=archive .` finds no stale path outside `openspec/changes/archive`.

## 3. Checks

- [x] 3.1 Add `starlight-links-validator` to the build. Verify a deliberately broken link and a broken heading anchor each fail `astro build` with the page named, then remove them.
- [x] 3.2 Write `sites/website/scripts/check-code-blocks.mjs` as design.md describes: every `nx` fence in `src/content/**/*.{md,mdx}`, checked as unmarked, `fragment` or `invalid` through `@nx-lang/sdk-wasm`'s Node entry, reporting `file:line: message`, with a nonzero exit on any failure. Wire it as the package's `test` script, with `@nx-lang/sdk-wasm` as a `workspace:*` devDependency. Add unit tests over small fixture Markdown for each of the four scenarios in "Documentation code blocks are checked". Verify the fixtures pass and fail as the spec says.
- [x] 3.3 Confirm Expressive Code ignores the `fragment` and `invalid` fence words. Verify a built page shows no stray text or frame title for such a block.
- [x] 3.4 Run the check over the real content and save the list of failures. Do not wire it into CI yet (see 4.4).

## 4. Content

- [x] 4.1 Rewrite the five pages that declare `type <X .../>` (What is NX, Design Goals, Building Your First Component, and the others 3.4 lists) in current syntax, checking each rewritten block with `nxlang run` or the check script. Verify the check reports no failures on those pages.
- [x] 4.2 Work through the rest of 3.4's list. Rewrite each stale block, or mark it `fragment` when it builds on an earlier block and `invalid` when it shows a rejected form. Verify the check passes over the whole site.
- [x] 4.3 Rewrite Getting Started as design.md orders it (playground, VS Code, .NET, JavaScript, next steps), and move the build-from-source and CLI material into Contributing. Verify the page's commands match the published package names (`npm view @nx-lang/sdk-wasm name`, NuGet `NxLang.Sdk`), and that no step before the Contributing pointer mentions Rust, Cargo or cloning. The editor step is finished in 11.5, once the extension is published.
- [x] 4.4 Now that the check passes, confirm the CI `rust` job's `pnpm -r test` runs it. Verify with a local `pnpm -r test` that it appears in the output.

## 5. Landing page, header and brand

- [x] 5.1 Replace `index.md` with a `splash` landing page: the one-sentence description, a hero snippet with its evaluated value, primary "Get started" and secondary "Try the playground" actions, feature cards, and a section linking `https://fiddle.drawnui.net` as where NX draws with DrawnUI. Settle the fiddle link target (design.md's open question). Verify the snippet passes the code-block check, and the value shown matches `nxlang run` on it.
- [x] 5.2 Override Starlight's `Header` to add Docs and Playground links beside search, the theme toggle and a GitHub social link. Verify on desktop and at 375 px width, with Playwright, that each link is reachable, including from the mobile menu.
- [x] 5.3 Add the logo mark, favicon, accent color (light and dark) and `/og.png` social card, with `og:image` set to its absolute URL through `head`. Verify the built HTML of three pages carries `og:title`, `og:description` and `og:image`, and that the image answers 200.
- [x] 5.4 Add a custom 404 page with the header and a link to the docs. Verify `dist/404.html` exists and carries the header.
- [x] 5.5 Add `starlight-llms-txt`. Verify `dist/llms.txt` links every sidebar section and the full-text file exists.

## 6. Playground as static files

- [x] 6.1 Move the build output so files land under `dist/playground/` (Vite `outDir`), keeping `base` from `base.mjs`. Verify `dist/playground/index.html` and `dist/playground/assets/nx-*.wasm` exist.
- [x] 6.2 Write `_headers` (at the site root, copied to `dist/_headers` by the build) carrying the cache policy from `server/index.mjs` (hashed assets immutable for a year, fonts and images revalidated daily, shell `no-cache`). Verify with `wrangler dev`, using `curl -sI`, for one path of each kind.
- [x] 6.3 Write the playground Worker script (the shell for the prefix and single-segment paths, 404 for everything else unmatched) and its unit tests, which cover `/playground`, `/playground/`, `/playground/cards`, `/playground/assets/missing.js` and `/playground/api/health`. Verify the tests pass, and the same five paths behave the same under `wrangler dev`.
- [x] 6.4 Add `sites/playground/wrangler.jsonc` (assets directory, script, routes `nxlang.org/playground` and `nxlang.org/playground/*`, `workers_dev: true` for now). Verify `wrangler deploy --dry-run` succeeds.
- [x] 6.5 Add `sites/website/wrangler.jsonc` (assets directory `dist`, `not_found_handling: "404-page"`, `html_handling: "auto-trailing-slash"`, route `nxlang.org/*`, `workers_dev: true` for now). Verify `wrangler deploy --dry-run` succeeds.

## 7. Deploy workflows

- [x] 7.1 Create a Cloudflare API token (Workers Scripts Edit on the account, Workers Routes Edit on the zone). Store it and the account id as `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` on the `production` GitHub environment with `gh secret set`. Verify `gh secret list --env production` lists both.
- [ ] 7.2 Write `.github/workflows/deploy-website.yml` (paths, filtered install, build, `wrangler deploy`, smoke test for the build stamp on `/` and one docs page). Add a `<meta name="nx-build">` carrying the commit to the site head. Verify with its first `workflow_dispatch` run, which is the cutover (design.md, Migration Plan).
- [ ] 7.3 Rewrite the deploy job of `deploy-playground.yml` from Railway to `wrangler deploy` plus a smoke test of the shell and the hashed compiler module. Remove the Railway env and steps. Verify with its first `workflow_dispatch` run, which is the cutover.

## 8. Cutover

- [ ] 8.1 Deploy both Workers with their routes. Verify every scenario in the `website` spec's "The site is served at the domain root" and in the playground's "Site is served under the playground path" with `curl` against `https://nxlang.org`, plus a Playwright run that opens an example and sees it draw.
- [ ] 8.2 Set `workers_dev: false` in both configs and redeploy. Delete the `playground assets` cache rule. Check `curl -s -H 'accept: text/html' https://nxlang.org/ | grep -c cloudflareinsights`, and if it is 0, add the beacon snippet to both sites. Verify the check answers 1 on both sites.
- [ ] 8.3 Change the apex and `www` DNS records to proxied `AAAA 100::`. Remove the Railway custom domain, then delete the Railway project and `RAILWAY_TOKEN_PRODUCTION`. Verify 8.1's checks again.
- [ ] 8.4 Unpublish GitHub Pages and delete the `github-pages` environment (`.github/workflows/deploy-docs.yml` is deleted in the PR). Verify `https://nx-lang.github.io/nx/` no longer serves the docs.

## 9. Remove what is no longer used

- [x] 9.1 Delete `sites/playground/server/`, `sites/playground/Dockerfile`, `.railway/`, the root `.dockerignore`, and the playground's `start` script. Rewrite `routes.test.mjs` or other tests that exercised the server's redirect. Verify `pnpm -r test` passes and `git grep -n "railway\|Dockerfile\|api/health"` finds only history and `openspec/changes/archive`.
- [x] 9.2 Change every `homepage` of `https://nx-lang.dev` to `https://nxlang.org` (`packages/*`, `bindings/{node,wasm}`, `runtime/typescript`, and `src/vscode` from its GitHub URL). Do the same for any NuGet `PackageProjectUrl`, and add the site link to the README. Verify `git grep -n "nx-lang.dev"` is empty and `pnpm run verify:packages` passes.

## 10. Documentation

- [x] 10.1 Rewrite the Playground Site sections of `docs/deployment.md` and `docs/deployment-setup.md` as "Website and playground": both workflows, rollback with `wrangler rollback`, and the one-time setup (token, secrets, routes, DNS placeholder records, zone settings, the redirect rule). Remove every Railway and cache-rule instruction. Verify against the "The site's hosting is documented" scenario by reading the setup section as a fresh account would.
- [x] 10.2 Update `sites/playground/README.md` for the static deploy: `pnpm run preview` or `wrangler dev` in place of `pnpm start`, and no server. Verify each command in it runs.
- [x] 10.3 Delete the `specs/future.md` sections "A static host, without the Node server" and "Splitting the domain across services". Update the "Playground" preface there, which still describes Railway. Verify `grep -n "Railway" specs/future.md` finds nothing that describes the current site.
- [x] 10.4 Run `openspec validate launch-nxlang-website --strict` and the full `pnpm -r test`. Verify both pass.

## 11. Publish the VS Code extension

- [ ] 11.1 Make the listing: a 128×128 PNG icon from the site's logo mark at
  `src/vscode/images/icon.png`, set as `icon` in `package.json` with a matching `galleryBanner`.
  Rewrite `src/vscode/README.md` for users (what it does, settings such as `nx.server.path`, a link
  to `https://nxlang.org`), keeping the `@nx-lang/language` editor-assets section below. Move the
  local development, file structure, packaging, credentials and publishing sections to
  `src/vscode/CONTRIBUTING.md`, and link it from the README and from the website's Contributing
  page. Verify `pnpm run package:verify` in `src/vscode` passes, `vsce ls` lists the icon, and
  `node scripts/verify-editor-package.mjs` still passes for `@nx-lang/language`.
- [ ] 11.2 Create the registry accounts (by hand): the Marketplace publisher `nx-lang`, with an Azure
  DevOps token scoped to Marketplace (Manage); an Eclipse account with the Open VSX publisher
  agreement signed, and the namespace `nx-lang` (`npx ovsx create-namespace nx-lang -p <token>`).
  Verify `https://marketplace.visualstudio.com/publishers/nx-lang` and
  `https://open-vsx.org/api/nx-lang` answer.
- [ ] 11.3 Store the tokens as `VSCE_PAT` and `OVSX_PAT` on the `production` environment with
  `gh secret set`. Verify `gh secret list --env production` lists both.
- [ ] 11.4 Release `vscode-v0.1.0` by the runbook in `docs/deployment.md` (a maintainer pushes the
  tag): inspect the draft release, publish it, and approve the deployment. Verify both registries
  report 0.1.0, and that `code --install-extension nx-lang.nx-language` in a clean profile installs
  it and opens a `.nx` file with highlighting and diagnostics.
- [ ] 11.5 Once 11.4 has published the extension, change Getting Started's editor step to install the extension: links to both registry
  pages, the identifier and the `code --install-extension` command, with no "not published yet"
  note. Verify it against "The editor extension is installed, not built", and that both links
  answer 200.
