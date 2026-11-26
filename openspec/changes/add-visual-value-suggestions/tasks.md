## 1. Suggestions in the language service

- [x] 1.1 Add `suggestions: bool` to `SourceTreeOptions`, `suggestion_key` to `SourceProperty` (properties, state and case properties) and to entries for this document's parameterless `let`s, and `suggestions: Vec<SourceSuggestion>` (name, key, group, scope, type, declaration, doc, self, preview) to `SourceTree`, all skipped when empty, in `crates/nx-language-service/src/source_tree.rs`. Declarations added only for a suggestion bring no keys.
- [x] 1.2 Compute suggestion keys from declared types (alias of a primitive → the alias; record, union or action through aliases → the type; none otherwise), keyed by module identity and name, in a new `suggestions.rs`.
- [x] 1.3 Gather candidates: visible environment values, top-level parameters and state, this document's parameterless `let`s, peer files' non-private `let`s, imported modules' visible `let`s spelled through import or selective-import aliases, and record fields to four levels; keep only names the top-level scope resolves without a diagnostic (no ambiguous or shadowed ones, and a top-level name a parameter or state field shadows lists that declaration in `self`); match by key, alias chain, record inheritance and a case element's union; list in `self` the declarations each entry reads transitively (reusing hover's read walk); order by group and declaration order; send only keys the answer uses.
- [x] 1.4 Static previews: follow names, `let`s, `env.` defaults, construction attributes and field defaults, and `if`/`is` on an environment value with a bare-case default, up to 16 steps, ending at literals, cases or colors. Factor color discovery so it runs on any module's syntax with that module's imports, and give previews the node's swatch text and outside-sRGB mark.
- [x] 1.5 Tests in `source_tree_tests.rs` (or `suggestions_tests.rs`) for every scenario of the value-suggestions spec, including `@nx/design` tokens, an import alias, a user alias of `Spacing`, an untyped color, a component property's scope, `self`, ordering and each preview kind, and that the answer is unchanged without the option.

## 2. Source edits

- [x] 2.1 Add the `name` value kind to the Rust source-edit request and spell it braced at an unbraced attribute or `let` value and bare inside braces or as a sequence item, refusing keywords, `true`/`false`, non-identifier parts and a lone `env`.
- [x] 2.2 Widen the `setValue` target to whole values written as names and elements, dropping a name's braces when a non-name value replaces it at an attribute or `let`; let a `name` replace a hex literal, constructor call or color element whole, keep `color` channel edits on calls and elements, write a `color` over a name as hex; keep refusing other expressions.
- [x] 2.3 Tests in `source_edit_tests.rs` for every new scenario of the source-edits delta, and that the existing ones still pass.

## 3. Protocol and TypeScript

- [x] 3.1 Add the option, `suggestions`, `suggestionKey` and the `name` edit value to `packages/language-protocol`, and pass them through `language-core`, the wasm and Node snapshots, the HTTP handler and client.
- [x] 3.2 Extend the wasm SDK's language-service parity tests to source trees with `suggestions` and to `name` edits.

## 4. Graphics types

- [x] 4.1 Import `@nx/design` as `design` in `@nx/graphics` and retype `Stroke.width`, `Text.fontSize`, `Text.fontFamily`, `Rectangle.rx`, `Rectangle.ry` and every `opacity` with the design aliases, keeping constraints and defaults.
- [x] 4.2 Check the library stays free of diagnostics, that existing graphics tests and examples pass unchanged, and regenerate corpus images; record any result change.

## 5. Viewer

- [x] 5.1 Read `suggestions` and `suggestionKey` from the tree; filter by key, scope and `self`.
- [x] 5.2 Add the named-values button after a field's value (written and unset), shown under the pointer, when selected or open, not a Tab stop, with the host's `named-values` icon or a built-in hexagon; open the popover on click, Ctrl+Space and Alt+Escape.
- [x] 5.3 Build the popover: combobox search over a grouped listbox, preview column, `Default` row, marked current row, filtering by words, keyboard, sticky group labels, own scrolling, the doc and exact-value lines and the defaults footer.
- [x] 5.4 Fire `nx-edit` with a `name` value on choosing (setValue or addProperty), the default row's removal, and nothing for the current row.
- [x] 5.5 Read `env.` reads, parameters, state fields and member chains as tokens, with swatches from color previews; a click on a token opens its popover; add Go to for names starting in this document and Replace with value in the token's menu (colors only inside sRGB, written as hex).
- [x] 5.6 Viewer tests for each scenario of the viewer delta, in both themes and at phone width. (Themes and the scenarios are covered by `packages/viewer/test/named.test.ts` and the playground browser check; the playground's Visual is unusable at phone width for every example, recorded in `specs/future.md`.)

## 6. Playground

- [x] 6.1 Ask for `suggestions` in Visual, draw the `named-values` icon, and add a token example to `/play/visual-test-cases`.
- [x] 6.2 Add page tests for choosing a token and undoing it, and screenshots of the popover in light and dark for the PR.

## 7. Documentation

- [x] 7.1 Document `suggestions` and the `name` edit value in the language-service reference, as unstable.
- [x] 7.2 Document the named-values popover in the viewer docs, and add a "Picking tokens in Visual" section to the design-tokens tutorial, with the advice to declare token types (`let gap: Spacing = 12`) and to import `@nx/graphics` or `@nx/design` under an alias when using both, since each exports a `Color`.
- [x] 7.3 Record the follow-ups (suggestions from modules not yet imported, local bindings, list items, previews that follow provides, element-typed properties) in `specs/future.md`.

## 8. Completion

- [ ] 8.1 Run the Rust and TypeScript test suites, `cargo fmt` and `cargo clippy`, then the review loop, then validate strictly, sync and archive.
