# Contributing to the NX VS Code extension

How to build, test, package and release the extension in `src/vscode`. The
[README](README.md) is the extension's page on the Visual Studio Marketplace and Open VSX, so it is
written for people installing the extension; everything for people working on it is here. The
repository-wide setup is in the website's
[Contributing](https://nxlang.org/contributing/) page.

## Local Development

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

- `package.json` - VS Code extension manifest (publisher: `nx-lang`, id: `nx-language`).
- `syntaxes/nx.tmLanguage.json` - TextMate grammar for NX.
- `syntaxes/nx.markdown.codeblock.tmLanguage.json` - Markdown fenced-code-block grammar injection.
- `language-configuration.json` - Comments, brackets, pairs.
- `snippets/nx.json` - Handy snippets for elements, control-flow, and braced value expressions.
- `src/` - VS Code activation code and LSP client helpers.
- `server/<platform>-<arch>/nx-lsp` - Packaged Rust language server asset.
- `out/` - Compiled extension runtime.
- `samples/` - Example NX files.
- `images/icon.png` - The Marketplace and Open VSX icon: the website's logo mark
  (`sites/website/src/assets/logo.svg`) at 128×128, since the Marketplace does not accept SVG.

## Packaging and Publishing

Use the local package scripts from `src/vscode` so the same checks run locally and in CI.
Cross-package CI setup is documented in
[docs/deployment-setup.md](../../docs/deployment-setup.md), and the recurring release runbook is in
[docs/deployment.md](../../docs/deployment.md).

```bash
pnpm run test
pnpm run build:lsp
pnpm run package:ls
pnpm run package
```

`package:ls` prints the files that will be included in the VS Code extension package. The release
VSIX is limited to `package.json`, `README.md`, `CHANGELOG.md`, `LICENSE`, `images/icon.png`,
`language-configuration.json`, `out/**`, `server/**`, `syntaxes/**`, and `snippets/**` by the
`files` allowlist in `package.json`; do not add a `.vscodeignore` alongside that allowlist because
`vsce` switches to ignore-based collection when one is present, bypassing the `files` allowlist and
requiring every development-only path to be excluded separately.

`pnpm run build:lsp` builds the Rust `nx-lsp` binary in release mode and copies it into
`server/<platform>-<arch>/`. Set `NX_LSP_PROFILE=debug` for a debug server during local
development. `pnpm run package:verify` fails if `out/extension.cjs`, `out/serverPath.js`, or the
expected server asset for the current package target is missing.

To build a native VSIX target, build or copy the matching server binary into the platform directory
that matches the VS Code target, then run `NX_VSCODE_TARGET=<target> pnpm run package`. For cross
builds, set `CARGO_BUILD_TARGET`, `NX_LSP_PLATFORM`, and `NX_VSCODE_TARGET`; for example,
`NX_LSP_PLATFORM=linux-x64 NX_VSCODE_TARGET=linux-x64 CARGO_BUILD_TARGET=x86_64-unknown-linux-gnu pnpm run build:lsp`.

### Editor Assets Package

Package and verify the npm editor-assets tarball:

```bash
pnpm run package:language
pnpm run verify:package dist/*.tgz
pnpm run smoke:package dist/*.tgz
```

`package:language` builds a clean `@nx-lang/language` tarball from a staging manifest, so VS Code
extension runtime files such as `out/**`, `server/**`, and `vscode-languageclient` stay out of the
reusable editor-assets package.

### Release Preparation

1. Add the release notes to `src/vscode/CHANGELOG.md`.
2. Let CI stage the publishable VSIX version from the `vscode-v<major>.<minor>.<patch>` release tag,
   or set `VSCODE_EXTENSION_VERSION` for a local repair package.
3. Run the package verification:
   ```bash
   pnpm install --frozen-lockfile
   pnpm run build:lsp
   pnpm run version:stage
   pnpm run package:verify
   pnpm run package:language
   ```
4. Push a VS Code extension release tag such as `vscode-v1.2.3`.
5. Review the draft GitHub Release, including the attached VSIX files, manifest, and checksums.
6. Publish the GitHub Release to trigger the separate `Publish VS Code extension` workflow.

Pull requests and `main` builds upload verified VSIX artifacts but do not publish to the Visual
Studio Marketplace or Open VSX. Pull request builds are tested by downloading the VSIX artifact from
the PR comment and installing it directly:

```bash
gh run download <run-id> -R nx-lang/nx -p 'vscode-vsix-*' -D nx-vsix-artifacts
find nx-vsix-artifacts -name '*.vsix' -type f -print0 | xargs -0 -I{} code --install-extension '{}' --force
```

Manual CI repair uses the Publish VS Code extension workflow's `release_tag` dispatch input to
republish VSIX artifacts from an already-published GitHub Release.

### Credentials

Configure these GitHub Actions secrets before enabling production VS Code extension publishing:

- `VSCE_PAT` - Visual Studio Marketplace personal access token for publisher `nx-lang`
- `OVSX_PAT` - Open VSX personal access token for namespace `nx-lang`

Production npm publishing for `@nx-lang/language` uses npm trusted publishing only. Do not configure
an npm publish token for CI.

For local VS Code extension publishing, provide the registry values as environment variables:

```bash
export VSCE_PAT=...
export OVSX_PAT=...
```

Do not commit tokens or write them into tracked configuration files.

### Local Publish and Repair

Publish an already-built VSIX to both registries for a local repair:

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

The normal production path is not local publishing: push a `vscode-v*` tag, inspect the draft GitHub
Release assets, then publish that GitHub Release so CI publishes the reviewed VSIX files.

## Roadmap

- Expand grammar coverage from `nx-grammar-spec.md` and `nx-grammar.md`.
- Improve LSP hover and completion precision as richer semantic projections become available.
- Add formatting, rename, references, and go-to-definition in later milestones.

## Notes

- The grammar aims for correctness and performance; regexes are kept conservative to avoid backtracking.
- HTML-style comments (`<!-- -->`) are recognized as comments, matching the language spec.

## Limitations

- TextMate text blocks: `text-raw-block` scopes regions like `<tag:text raw> ... </tag>` while
  `text-typed-block` scopes typed text elements so their bodies stay flat text except for the
  `@{ ... }` braced value delimiter. The grammar still can't easily host other embedded languages
  inside these sections, and completely preventing nested NX markup inside raw blocks would require
  more invasive rule restructuring.
