# NX Language (VS Code)

Basic syntax highlighting and language configuration for the NX language using a TextMate grammar. No language server yet.

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
- `samples/` — Example NX files.

## Package Exports

The published package also exposes the language assets for browser consumers:

- `nx-language/grammar` — NX TextMate grammar JSON
- `nx-language/language-configuration` — NX language configuration JSON

This allows web editors and docs tooling to reuse the same highlighting assets without copying them into another repository.

## Roadmap

- Expand grammar coverage from `nx-grammar-spec.md` and `nx-grammar.md`.
- Add unit tests for tokenization (e.g., using vscode-tmgrammar-test).
- Add an LSP in a later milestone: diagnostics, hovers, completions, formatting.

## Packaging and Publishing

Use the local package scripts from `src/vscode` so the same checks run locally and in CI.

```bash
pnpm run test
pnpm run package:ls
pnpm run package
```

`package:ls` prints the files that will be included in the extension package. The release VSIX is
limited to `package.json`, `README.md`, `CHANGELOG.md`, `LICENSE`, `language-configuration.json`,
`syntaxes/**`, and `snippets/**` by the `files` allowlist in `package.json`; do not add a
`.vscodeignore` alongside that allowlist because `vsce` switches to ignore-based collection when
one is present, bypassing the `files` allowlist and requiring every development-only path to be
excluded separately.

### Release Preparation

1. Update `src/vscode/package.json` to the release version.
2. Add the release notes to `src/vscode/CHANGELOG.md`.
3. Run the package verification:
   ```bash
   pnpm install --frozen-lockfile
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
