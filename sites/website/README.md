## NX Website

The site served at https://nxlang.org: the landing page and the NX language documentation, built
with [Astro Starlight](https://starlight.astro.build). The playground is a separate site
(`sites/playground`) served under `/playground` on the same domain.

### Running it

From the repository root:

1. `pnpm install` installs the workspace, this site included.
2. `pnpm --filter @nx-lang/website dev` starts the local server.
3. `pnpm --filter @nx-lang/website build` writes the static site to `sites/website/dist/`.
4. `pnpm --filter @nx-lang/website preview` serves that build.

### Content

Pages live under `src/content/docs/`, one Markdown file per page, and the sidebar is declared in
`starlight.config.mjs`. Links between pages are root-relative (`/language-tour/types/`). The
not-found page is `src/pages/404.astro` rather than a content page, so it stays out of `llms.txt`
and its full-text files.

`public/_headers` holds the cache policy Cloudflare applies to the built files: Astro's hashed
`/_astro/` files are immutable, and everything else keeps the default, revalidated on every use.

### Syntax highlighting

Code blocks fenced with `nx` use the shared VS Code TextMate grammar
(`src/vscode/syntaxes/nx.tmLanguage.json` at the repository root), so the extension and the site
highlight NX the same way.
