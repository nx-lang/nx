## 1. Rust query

- [x] 1.1 Add the serializable types to `crates/nx-language-service`: `SourceTree`, `SourceNode`, `SourceRole`, `SourceFlag`, `SourceDeclaration`, `SourceProperty`, using `EditorRange` for ranges and `camelCase` serialization; verify `cargo doc -p nx-language-service` documents each field
- [x] 1.2 Implement `WorkspaceSnapshot::source_tree(&DocumentUri)` as a walk of the tree-sitter tree producing nodes in source order with parent indices, one role per construct listed in the spec, and `unparsed` nodes for error regions; verify with a test per role in a new `source_tree_tests.rs`
- [x] 1.3 Annotate nodes with resolved types and declaration indices by mapping each syntax node to its lowered counterpart, and build the declaration table (properties with inherited ones first, defaults as source text, state fields, cases, bases, doc comments) for declarations of this module, other modules and the standard library; verify with the spec's scenarios `The tree agrees with hover`, `A library type is described` and `A reference leads to its value`
- [x] 1.4 Compute keys as name paths with positional items, and test that no two nodes share a key and that inserting a declaration leaves other declarations' keys unchanged; verify with `An insertion elsewhere does not move a key` and `Items are keyed by position`

## 2. Coverage

- [x] 2.1 Add a conformance test that computes the source tree of every `.nx` file under `examples/`, `specs/`, `sites/` and `crates/` and checks the token rule, reporting the file, line and token on failure; verify it passes, and that removing the `comment` role from the builder makes it fail
- [x] 2.2 Record the answer size and time for `specs/ir-conformance/question-flow/main.nx` in the test output; verify it is under 100 ms in release

## 3. TypeScript exposure

- [ ] 3.1 Export the query from the wasm language snapshot in `bindings/wasm`, add the TypeScript types to `types.ts`, and move the ABI version; verify the wasm SDK tests pass and an older module is refused with the version mismatch error
- [ ] 3.2 Add `sourceTree` to `LANGUAGE_QUERIES`, the request and answer maps and the service interface in `@nx-lang/language-protocol`; implement it in `@nx-lang/language-core` and route it in `@nx-lang/language-http`; mark it unstable in each README; verify each package's tests pass
- [ ] 3.3 Add a parity test that the TypeScript and Rust answers are equal after JSON round-trip for the wasm SDK's language-service corpus; verify it passes

## 4. Specs

- [ ] 4.1 Run `openspec validate add-source-tree --strict`; verify it passes
