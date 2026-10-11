# Deployment Setup

This checklist covers the one-time setup for publishing NX packages and VS Code extension artifacts
from GitHub Actions, and for hosting the website and playground. The ongoing runbook lives in
[deployment.md](deployment.md).

## GitHub Environment

Create one GitHub environment:

- `production`: used by workflows that publish already-reviewed GitHub Release assets to NuGet.org,
  npm, crates.io, the Visual Studio Marketplace, and Open VSX, and by the two workflows that deploy
  `nxlang.org` from `main`.

Recommended protection:

- Add required reviewers before enabling public registry writes.
- Restrict deployments to protected release branches or tags as appropriate for the repository.
- Keep pull request and `main` build workflows artifact-only; they do not need production secrets.
- Audit environment deployment history after each published release.

## Registry Ownership

Set up ownership before enabling publication:

- NuGet.org: reserve or own `NxLang.Sdk`.
- npm: own the `@nx-lang` scope and every package the publish job pushes: `@nx-lang/language`
  (editor assets) and the workspace packages `@nx-lang/language-protocol`, `@nx-lang/language-core`,
  `@nx-lang/language-client`, `@nx-lang/ir-runtime`, `@nx-lang/sdk-wasm`, `@nx-lang/monaco`,
  `@nx-lang/value-view`, `@nx-lang/viewer`, `@nx-lang/previewer` and `@nx-lang/agent`.
  `scripts/pack-packages.mjs` packs every workspace member that is not `private`, so a package
  joins this list by dropping `private`, and leaves it by adding it back.
- crates.io: own the crates `nx-ir`, `nx-value` and `nx-ir-runtime`. A crate name is owned by
  whoever publishes it first and cannot be reserved ahead of that, so the names are taken by the
  first release that includes them. Add a second owner to each afterwards
  (`cargo owner --add <github-user-or-team> nx-ir`), so the crates do not depend on one account.
  `scripts/runtime-crates.mjs` lists the crates that are published; every other member of the
  Cargo workspace is `publish = false`.
- Visual Studio Marketplace: own publisher `nx-lang` and extension `nx-language`.
- Open VSX: own namespace `nx-lang` and extension `nx-language`.

## Trusted Publishing

Prefer trusted publishing where the registry supports it:

- NuGet.org: create a trusted publishing policy for repository `nx-lang/nx`, workflow file
  `package-publish.yml`, and the `production` environment. Set `NUGET_USER` as a production
  environment secret for the NuGet owner account used by `NuGet/login`.
- npm: create a trusted publisher for each package in the list above that matches repository
  `nx-lang/nx`, workflow `.github/workflows/package-publish.yml`, and environment `production`. npm
  only lets a trusted publisher be configured on a package that already exists, so a package's
  first version is published by hand from a release's downloaded `.tgz` with a maintainer's
  token (`npm publish <tgz> --access public`), and the trusted publisher is added right after;
  the publish job then skips that version as already present and publishes the next release with
  provenance.

- crates.io: create a trusted publisher for each of `nx-ir`, `nx-value` and `nx-ir-runtime`, on the
  crate's **Settings → Trusted Publishing** page: repository owner `nx-lang`, repository `nx`,
  workflow file `package-publish.yml`, environment `production`. crates.io, like npm, configures a
  trusted publisher only on a crate that exists, so the first version of each is published with a
  token:
  1. Before the first release that includes the crates, create an API token at
     `https://crates.io/settings/tokens` with the scopes `publish-new` and `publish-update`, and
     store it on `production`:
     ```bash
     gh secret set CRATES_IO_TOKEN --repo nx-lang/nx --env production
     ```
     Run that from your own terminal and paste the token when asked, so that it is never part of a
     command line.
  2. Tag and publish the release as usual. The `publish-crates` job finds no trusted publisher,
     says so in a warning, and publishes the three crates with the token.
  3. Add the three trusted publishers, then delete the secret and revoke the token:
     ```bash
     gh secret delete CRATES_IO_TOKEN --repo nx-lang/nx --env production
     ```
  Later releases exchange GitHub's OIDC token for a short-lived crates.io token
  (`rust-lang/crates-io-auth-action`), which is revoked when the job ends.

  A crate added to the track later needs its first version published by hand. The job would use
  the trusted token, which the three existing crates' publishers grant and which cannot create a
  crate name. So after that release's `publish-crates` job has failed at the new crate, publish it
  from a checkout of the release's tag with a maintainer's token, using the command under *Repair
  A Partial Publish* in [deployment.md](deployment.md), which publishes only if the crates package
  to the release's files; add the crate's trusted publisher; then rerun the job, which skips what
  is there and publishes the crates that depend on the new one.

All three publish jobs in `package-publish.yml` request GitHub OIDC with `id-token: write`, and run
only after a GitHub Release is published; each validates the release assets before any registry
write.

The Visual Studio Marketplace also takes GitHub OIDC: the `publish-extension` job signs in as an
Azure managed identity that is a member of the `nx-lang` publisher (see
[Visual Studio Marketplace publishing identity](#visual-studio-marketplace-publishing-identity)).
Open VSX has no equivalent, so it uses the `OVSX_PAT` token secret.

## Secrets And Variables

Production environment secrets:

- `NUGET_USER`: NuGet.org account or organization owner used by NuGet trusted publishing.
- `NUGET_API_KEY`: fallback NuGet.org API key when trusted publishing is unavailable.
- `AZURE_CLIENT_ID` and `AZURE_TENANT_ID`: the Marketplace publishing identity's client and tenant
  ids. They identify the identity rather than grant anything; GitHub's OIDC token does that.
- `OVSX_PAT`: Open VSX token for namespace `nx-lang`.
- `CRATES_IO_TOKEN`: crates.io API token, present only until the runtime crates' first versions are
  published and their trusted publishers exist (see [Trusted publishing](#trusted-publishing)).
- `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`: deploy the website and playground Workers
  (see [Website and playground hosting](#website-and-playground-hosting)).

No preview NuGet, preview npm, or pull request registry credentials are required. Pull request
testing uses workflow artifacts and PR comments with download/install commands.

The Publish release workflow (`package-publish.yml`) uses all of these but the Cloudflare pair, and
accepts `release_tag` for manual repair. Set it to a published GitHub Release tag such as `v1.2.3`;
the workflow downloads, validates and republishes the attached release assets, packages and VSIX
files alike, without rebuilding them.

Rust tool publication for `nxlang` and `nx-lsp` is not part of this deployment setup yet, and no
Rust binary-release credential is required for this release pipeline. Of the Rust crates only the
three runtime crates are published.

Never commit registry tokens or write them into tracked configuration files.

## Visual Studio Marketplace Publishing Identity

The Marketplace retires global personal access tokens on 2026-12-01, so the extension is published
with a Microsoft Entra ID token for a user-assigned managed identity, which GitHub Actions signs in
as through OIDC. It must be a managed identity: an app registration signs in, but the Marketplace
refuses its publish with `InvalidAccessException`.

The identity lives in the Azure subscription `nx-lang` (Pay-As-You-Go, sponsored by Forward Reach,
with a $1 monthly budget alert). A managed identity costs nothing. No Azure DevOps organization is
needed.

1. In the Azure portal, **Managed Identities → Create**: subscription `nx-lang`, resource group
   `nx-lang`, name `nx-vscode-publisher`, any region.
2. On the identity, **Settings → Federated credentials → Add credential**: scenario "GitHub Actions
   deploying Azure resources", organization `nx-lang`, repository `nx`, entity **Environment**,
   environment `production`. An environment matches every release; a tag entity would match only
   one tag.
3. Store the identity's ids from its **Overview** on `production`:

   ```bash
   gh secret set AZURE_CLIENT_ID --repo nx-lang/nx --env production --body <client-id>
   gh secret set AZURE_TENANT_ID --repo nx-lang/nx --env production --body <tenant-id>
   ```

4. Run **Check Marketplace identity** (`marketplace-identity.yml`) from the Actions tab. Its
   summary shows the identity's Azure DevOps id, the only id the Marketplace accepts for a member.
   Its publish check fails until step 5 is done.
5. At `https://marketplace.visualstudio.com/manage/publishers/nx-lang`, **Members → Add**, paste
   that id, role **Contributor**. The Marketplace lists the member as `<tenant id>\<object id>`.
6. Run **Check Marketplace identity** again. Its last step, `vsce verify-pat --azure-credential
   nx-lang`, passes once the identity may publish.

To replace the identity, repeat these steps and remove the old member.

## Versioning Setup

The repository uses the restored local .NET tool `minver-cli` through
`tools/versions/Get-ReleaseVersion.ps1`.

Releases use one tag format, `v<major>.<minor>.<patch>`, for example `v1.2.3`, and every package
and VSIX in the release takes that version. The retired extension-only format, `vscode-v*`, releases
nothing.

Only stable `major.minor.patch` release tags are supported. Pull request and `main` package
artifacts receive unique prerelease versions, while VSIX artifacts use registry-valid
`major.minor.patch` versions and are tested by direct VSIX installation.

## First Enablement

1. Confirm PR workflows upload `deployables-Complete`, `editor-assets-package`, `npm-packages`,
   `rust-crates`, and `vscode-vsix-*` artifacts without public registry credentials.
2. Confirm the trusted PR artifact comment workflow posts download/install commands without checking
   out or executing pull request code.
3. Push a test tag in a disposable repository or dry-run branch and confirm `release.yml` creates a
   draft GitHub Release with `.nupkg`, `.snupkg`, one `.tgz` per npm package, one `.crate` per
   runtime crate, one `.vsix` per extension target, manifest, and checksum assets.
4. Enable `production` after release asset validation, package inspection, smoke tests, and publish
   workflow validation pass.
5. Keep `NUGET_API_KEY` empty when NuGet trusted publishing is working. Production npm publishing
   uses trusted publishing only; do not configure an npm publish token for CI. Remove
   `CRATES_IO_TOKEN` once the three crates have trusted publishers.

## Website And Playground Hosting

`nxlang.org` is a Cloudflare zone served by two Workers with static assets and no origin:
`nxlang-website` on `nxlang.org/*`, and `nxlang-playground` on `nxlang.org/play` and
`nxlang.org/play/*`. Each Worker's name, routes and assets settings are in its site's
`wrangler.jsonc`, and a deploy creates the Worker and its routes. Everything below is done once by
hand, and this section is the record of it.

### API token and secrets

1. In the Cloudflare dashboard, **Manage Account → Account API Tokens → Create Token → Start from
   scratch**, named `nxlang deploy`, with no expiration and no IP filter (GitHub's runners have no
   fixed addresses), and two permission policies. An account token belongs to the account rather
   than to a person, so the deploys don't depend on anyone's login.
   - the entire account that holds the zone: **Workers Scripts Write** (a Worker is an account
     resource, so this can't be narrowed to the zone);
   - the zone `nxlang.org`: **Workers Routes Write** and **Zone Read**.
2. Store it and the account id (the dashboard's right-hand column on the zone overview) on the
   `production` environment:

   ```bash
   gh secret set CLOUDFLARE_API_TOKEN --repo nx-lang/nx --env production
   gh secret set CLOUDFLARE_ACCOUNT_ID --repo nx-lang/nx --env production
   gh secret list --repo nx-lang/nx --env production     # lists both
   ```

### DNS

Worker routes run only on proxied hostnames, and neither Worker has an origin, so both records
point at the reserved placeholder address:

| Type | Name | Content | Proxy |
|---|---|---|---|
| `AAAA` | `@` | `100::` | Proxied |
| `AAAA` | `www` | `100::` | Proxied |

### Zone settings

| Setting | Value | Why |
|---|---|---|
| SSL/TLS → Edge Certificates → Always Use HTTPS | **On** | `http://` requests redirect at the edge, before any Worker runs. |
| Security → Bots → Bot Fight Mode | **Off** | It challenges the deploy workflows' smoke tests from GitHub's runners (a managed challenge, 403, on every attempt), and the Free plan can't exempt a path or user agent. Every response is a static file, so there is nothing to protect. |
| Analytics → Web Analytics | **Enable, excluding visitor data in the EU** | Cookie-less, so no consent banner. The EU exclusion is the same choice as the account's other sites. The edge injects the beacon into HTML responses; confirm it with the check below. |

### Redirect rule: `www` to apex

- Name: `www to apex`
- When: Hostname equals `www.nxlang.org`
- Then: Dynamic redirect, expression `concat("https://nxlang.org", http.request.uri.path)`,
  status 301, preserve query string.

No cache rule is needed. Workers static assets are served from Cloudflare's own storage, and the
cache headers come from the files: `sites/playground/_headers` makes hashed assets immutable for a
year and the shell `no-cache`, and `sites/website/public/_headers` makes the website's hashed
`/_astro/` files immutable.

### First deployment

Run **Deploy Website** and **Deploy Playground** from the Actions tab. Each creates its Worker and
its routes, and its smoke test fetches the new build through `https://nxlang.org`.

### Verify the setup

```bash
curl -sI http://nxlang.org/language-tour/types/ | grep -i location     # https://nxlang.org/language-tour/types/
curl -sI https://www.nxlang.org/play | grep -i location                 # https://nxlang.org/play
curl -sI https://nxlang.org/                                            # 200, the landing page
curl -sI https://nxlang.org/play                                        # 200, the playground's shell
curl -s -H 'accept: text/html' https://nxlang.org/ | grep -c cloudflareinsights            # 1
curl -s -H 'accept: text/html' https://nxlang.org/play | grep -c cloudflareinsights        # 1
#   the edge injects the Web Analytics beacon only into responses to requests that accept HTML,
#   so a bare curl shows none and proves nothing
```

Open `https://nxlang.org` and `https://nxlang.org/play` in a browser, search the docs, and
open and edit a playground example. Web Analytics should show the visits within a few minutes.
