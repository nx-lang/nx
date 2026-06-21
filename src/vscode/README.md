# NX Language (VS Code)

Syntax highlighting, language configuration, snippets, and Rust language-server features for the NX language.

## Features

- NX file association (`.nx`)
- TextMate grammar (`source.nx`):
  - Keywords: import, type, let, if/is/else (simple/match/condition-list), for/in, raw
  - Primitive types: string, int, long, float, double, bool, void, object
  - Numbers, strings (single/double), entities, operators
  - Markup elements and attributes, closing/self-closing tags
  - Braced value regions: `{ expr }`, space-delimited `{first second}`, and typed-text `@{first second}`
- Language configuration: comments, bracket/auto-closing, simple indentation rules
- Starter snippets
- Rust `nx-lsp` integration for diagnostics, document symbols, hover, and completions
- `nx.server.path` setting for using a development or custom `nx-lsp` executable

## Getting Started

1. Install and load `nvm` (recommended on WSL):
   ```bash
   curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash
   source ~/.nvm/nvm.sh
   ```
2. Install the Node.js version the VS Code tooling expects (Node 24 LTS or newer):
   ```bash
   nvm install 24 && nvm use 24
   ```
3. From `src/vscode`, enable the package manager version declared by `package.json`, install
   dependencies, and verify the VSIX package when needed:
   ```bash
   corepack enable
   corepack prepare pnpm@10.28.1 --activate
   pnpm install --frozen-lockfile
   pnpm run build:lsp
   pnpm run package:verify
   ```
4. Launch VS Code with the extension loaded and pointing at the repo root (from `src/vscode`):
   ```bash
   code --extensionDevelopmentPath=. ../..
   ```
   OR use the launch config: "Run NX Language Extension".
5. Open `src/vscode/samples` files to see highlighting.

## File Structure

- `package.json` — Extension manifest (publisher: `nx-lang`, id: `nx-language`).
- `syntaxes/nx.tmLanguage.json` — TextMate grammar for NX.
- `language-configuration.json` — Comments, brackets, pairs.
- `snippets/nx.json` — Handy snippets for elements, control-flow, and braced value expressions.
- `src/` — VS Code activation code and LSP client helpers.
- `server/<platform>-<arch>/nx-lsp` — Packaged Rust language server asset.
- `out/` — Compiled extension runtime.
- `samples/` — Example NX files.

## Package Exports

The published package also exposes the language assets for browser consumers:

- `nx-language/grammar` — NX TextMate grammar JSON
- `nx-language/language-configuration` — NX language configuration JSON

This allows web editors and docs tooling to reuse the same highlighting assets without copying them into another repository.

## Roadmap

- Expand grammar coverage from `nx-grammar-spec.md` and `nx-grammar.md`.
- Improve LSP hover and completion precision as richer semantic projections become available.
- Add formatting, rename, references, and go-to-definition in later milestones.

## Packaging and Publishing

Use the local package scripts from `src/vscode` so the same checks run locally and in CI.

```bash
pnpm run test
pnpm run build:lsp
pnpm run package:ls
pnpm run package
```

`package:ls` prints the files that will be included in the extension package. The release VSIX is
limited to `package.json`, `README.md`, `CHANGELOG.md`, `LICENSE`, `language-configuration.json`,
`out/**`, `server/**`, `syntaxes/**`, and `snippets/**` by the `files` allowlist in `package.json`; do not add a
`.vscodeignore` alongside that allowlist because `vsce` switches to ignore-based collection when
one is present, bypassing the `files` allowlist and requiring every development-only path to be
excluded separately.

`pnpm run build:lsp` builds the Rust `nx-lsp` binary in release mode and copies it into
`server/<platform>-<arch>/`. Set `NX_LSP_PROFILE=debug` for a debug server during local
development. `pnpm run package:verify` fails if `out/extension.cjs`,
`out/serverPath.js`, or the expected server asset for the current package target is missing.

To build a native VSIX target, build or copy the matching server binary into the platform directory
that matches the VS Code target, then run `NX_VSCODE_TARGET=<target> pnpm run package`. For cross
builds, set `CARGO_BUILD_TARGET`, `NX_LSP_PLATFORM`, and `NX_VSCODE_TARGET`; for example,
`NX_LSP_PLATFORM=linux-x64 NX_VSCODE_TARGET=linux-x64 CARGO_BUILD_TARGET=x86_64-unknown-linux-gnu pnpm run build:lsp`.

### Release Preparation

1. Update `src/vscode/package.json` to the release version.
2. Add the release notes to `src/vscode/CHANGELOG.md`.
3. Run the package verification:
   ```bash
   pnpm install --frozen-lockfile
   pnpm run build:lsp
   pnpm run package:verify
   ```
4. Create and push a tag that matches the package version:
   ```bash
   git tag vscode-v$(node -p "require('./package.json').version")
   git push origin vscode-v$(node -p "require('./package.json').version")
   ```

The GitHub Actions workflow publishes only from `vscode-v<version>` tags where `<version>` matches
`src/vscode/package.json`.

### Credentials

Configure these GitHub Actions secrets before pushing a release tag:

- `VSCE_PAT` — Visual Studio Marketplace personal access token for publisher `nx-lang`
- `OVSX_PAT` — Open VSX personal access token for namespace `nx-lang`

For local publishing, provide the same values as environment variables:

```bash
export VSCE_PAT=...
export OVSX_PAT=...
```

Do not commit tokens or write them into tracked configuration files.

### Local Publish and Repair

Publish an already-built VSIX to both registries:

```bash
VSIX=nx-language-$(node -p "require('./package.json').version").vsix
pnpm run publish:all -- "$VSIX"
```

If a CI release partially succeeds and only one registry needs a repair, publish the same VSIX to
one registry:

```bash
VSIX=nx-language-$(node -p "require('./package.json').version").vsix
pnpm run publish:vsce -- "$VSIX"
pnpm run publish:ovsx -- "$VSIX"
```

Both commands publish the provided VSIX artifact instead of rebuilding a new package. The publisher
is `nx-lang` and the extension ID is `nx-language`.

## Notes

- The grammar aims for correctness and performance; regexes are kept conservative to avoid backtracking.
- HTML-style comments (`<!-- -->`) are recognized as comments, matching the language spec.

## Limitations

- TextMate text blocks: `text-raw-block` scopes regions like `<tag:text raw> ... </tag>` while `text-typed-block` scopes typed text elements so their bodies stay flat text except for the `@{ … }` braced value delimiter. The grammar still can’t easily host other embedded languages inside these sections, and completely preventing nested NX markup inside raw blocks would require more invasive rule restructuring.
