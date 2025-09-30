# NX VS Code Extension – TODO

This tracks near-term enhancements and future work. The LSP server will be implemented in C#.

## Grammar Coverage
- [x] Parse attribute values that can embed inline elements, e.g., `prop=<Start/>` and `content=<:uitext>…</>`.
- [ ] Improve mixed content highlighting: interleave text, elements, and interpolations more accurately.
- [ ] Add explicit patterns for elements control forms: `ElementsIfExpression`, `ElementsSwitchExpression`, `ElementsForExpression`.
- [ ] Recognize `raw` embed mode distinctly and treat inner content as unparsed text.
- [ ] Consider injection of other grammars for typed embeds (e.g., `:markdown`, `:uitext`) via TextMate injections or `contentName` to piggyback existing scopes.
- [ ] Broaden identifier rules for `QualifiedMarkupName` vs `QualifiedName` where hyphens are allowed only in markup identifiers.
- [ ] Expand numeric literals per spec (underscores, exponents) and guard against ambiguity.
- [ ] Add scopes for type modifiers (`?`, `...`) and ensure they don’t collide with spread `...` syntax in attributes.

## Folding / Indentation
- [ ] Improve indentation rules for `if … /if`, `switch … /switch`, `for … /for` blocks.
- [ ] Add folding markers for block pairs (`if/for/switch` and tag open/close), including nested tags.
- [ ] Validate behavior with long lines and mixed-content blocks.

## Snippets
- [ ] Add snippets for common property patterns (typed defaults, spreads `...props`).
- [ ] Add concise control-flow snippets (inline value `if`, `switch` cases with multiple patterns).

## Tests
- [ ] Introduce tokenization tests (e.g., `vscode-tmgrammar-test` or `vscode-textmate` + `oniguruma`).
- [ ] Derive representative cases from `nx-example-scratchpad.md` and `nx-grammar-spec.md`.
- [ ] Add negative tests for malformed tags and unterminated blocks.
- [ ] Add a JSON validation step for grammar/config (`jq` or `jsonlint`).

## Tooling / Packaging
- [ ] Add npm scripts: `package` (vsce), `publish:vsce`, `publish:ovsx`, `test:grammar`.
- [ ] Keep package lean (whitelist already added). Optionally include a tiny sample if desired.
- [ ] Add an icon (`icon.png`) and branding.
- [ ] Document Publisher setup for Marketplace and Open VSX (tokens, 2FA).

## CI
- [ ] GitHub Actions: validate JSON, run grammar tests, package on tag.
- [ ] Dual publish (Marketplace via `vsce`, Open VSX via `ovsx`) gated by secrets.
- [ ] Cache Node/npm for faster CI runs.

## Documentation
- [ ] Expand README with language overview, scopes list, and screenshots (light/dark themes).
- [ ] Add a “Contributing” section for grammar/test iteration.
- [ ] Link to `nx-grammar.md` and `nx-grammar-spec.md` as sources of truth.

## WSL / Dev Experience
- [ ] Verify debug launch works across Windows, WSL, and macOS. Consider a separate non-WSL config.
- [ ] Add `.vscode/tasks.json` helpers (package, test) and recommended extensions for contributors.

## LSP (C#) Roadmap – later milestone
- [ ] Choose server framework (e.g., OmniSharp.Extensions.LanguageServer) and set up a C# project.
- [ ] Define protocol surface: diagnostics, hovers, completion, document symbols, formatting.
- [ ] Implement a minimal server (no features) and a Node client stub in the extension to spawn the C# server.
- [ ] Follow repo C# guidelines (nullable enabled, Allman braces, explicit access modifiers).
- [ ] Ensure any JSON-RPC handlers adhere to internal conventions (e.g., method attributes as required by repo guidance).
- [ ] Add launch configurations for server + client debugging (attach to dotnet, run Extension Host).
- [ ] Wire WSL support for server process (spawn via `dotnet` within WSL).

## Performance & Quality
- [ ] Profile grammar on large `.nx` files; tune regexes to avoid catastrophic backtracking.
- [ ] Add benchmarks or a corpus for manual comparison across themes.
- [ ] Collect user feedback; track false positives/negatives in highlighting.

---

If you want, I can start by adding test scaffolding and npm scripts next.
