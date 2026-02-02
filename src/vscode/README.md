# NX Language (VS Code)

Basic syntax highlighting and language configuration for the NX language using a TextMate grammar. No language server yet.

## Features

- NX file association (`.nx`)
- TextMate grammar (`source.nx`):
  - Keywords: import, type, let, if/is/else (simple/match/condition-list), for/in, raw
  - Primitive types: string, int, long, float, double, bool, void, object
  - Numbers, strings (single/double), entities, operators
  - Markup elements and attributes, closing/self-closing tags
  - Interpolations: `{ expr }`
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
3. From `src/vscode`, install dependencies and build the VSIX package when needed:
   ```bash
   npm install
   npm run package
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
- `snippets/nx.json` — Handy snippets for elements, control-flow, and interpolation.
- `samples/` — Example NX files.

## Roadmap

- Expand grammar coverage from `nx-grammar-spec.md` and `nx-grammar.md`.
- Add unit tests for tokenization (e.g., using vscode-tmgrammar-test).
- Add an LSP in a later milestone: diagnostics, hovers, completions, formatting.

## Packaging and Publishing

You can package with `vsce` or publish to Open VSX with `ovsx`.

Example commands (install tools globally or as dev dependencies):

```
vsce package
ovsx publish
```

Set the publisher to `nx-lang` and the extension ID to `nx-language` (already configured).

## Notes

- The grammar aims for correctness and performance; regexes are kept conservative to avoid backtracking.
- HTML-style comments (`<!-- -->`) are recognized as comments, matching the language spec.

## Limitations

- TextMate text blocks: `text-raw-block` scopes regions like `<tag:text raw> ... </tag>` while `text-typed-block` scopes typed text elements so their bodies stay flat text except for the `@{ … }` interpolation delimiter. The grammar still can’t easily host other embedded languages inside these sections, and completely preventing nested NX markup inside raw blocks would require more invasive rule restructuring.
