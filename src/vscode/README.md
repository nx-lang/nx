# NX Language (VS Code)

Basic syntax highlighting and language configuration for the NX language using a TextMate grammar. No language server yet.

## Features

- NX file association (`.nx`)
- TextMate grammar (`source.nx`):
  - Keywords: import, type, let, if/else, switch/case/default, for/in, raw
  - Primitive types: string, int, long, float, double, boolean, void, object
  - Numbers, strings (single/double), entities, operators
  - Markup elements and attributes, closing/self-closing tags
  - Interpolations: `{ expr }`
- Language configuration: comments, bracket/auto-closing, simple indentation rules
- Starter snippets

## Getting Started

1. Open this repository in VS Code.
2. Run the launch config: "Run NX Language Extension".
3. Open `src/vscode/samples/basic.nx` to see highlighting.

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

