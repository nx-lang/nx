## NX Docs Site

This directory contains the Astro Starlight documentation site for the NX language.

### Getting Started

1. `npm install` – install the documentation dependencies (run from `docs/`).
2. `npm run dev` – start the local docs server.
3. `npm run build` – create the production build in `docs/dist/`.

### Syntax Highlighting

Code blocks fenced with `nx` use the shared VS Code TextMate grammar (`src/vscode/syntaxes/nx.tmLanguage.json`) via Astro's Shiki integration. Update that grammar in one place and both the extension and docs stay in sync.

### Content Layout

Documentation lives directly under this folder (e.g. `overview/`, `tutorials/`, `reference/`). Add new pages by creating markdown files that align with the sidebar structure declared in `starlight.config.mjs`.
