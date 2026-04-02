## 1. Grammar and Spec Definitions

- [x] 1.1 Update [nx-grammar.md](/home/bret/src/nx/nx-grammar.md) to remove `contenttype`, rename `ModulePath` to `LibraryPath`, add qualified selective aliases, and document `internal` plus public-by-default visibility
- [x] 1.2 Update [nx-grammar-spec.md](/home/bret/src/nx/nx-grammar-spec.md) to match the revised grammar, AST node names, and declaration visibility metadata
- [x] 1.3 Review the updated grammar docs against the OpenSpec deltas to ensure the normative syntax and visibility rules match before code changes start

## 2. Parser and Syntax Tree

- [x] 2.1 Update `crates/nx-syntax/grammar.js` to remove `contenttype`, rename `module_path` to `library_path`, and parse selective `as QualifiedName` aliases plus top-level visibility modifiers
- [x] 2.2 Regenerate the tree-sitter artifacts and refresh syntax metadata in `crates/nx-syntax/src/parser.c`, `crates/nx-syntax/src/grammar.json`, `crates/nx-syntax/src/node-types.json`, and related generated files
- [x] 2.3 Update `crates/nx-syntax/src/syntax_kind.rs`, CST helpers, and query files so the new library-path and visibility nodes/tokens are exposed consistently and `contenttype` is removed
- [x] 2.4 Update parser and CST tests in `crates/nx-syntax/tests/` and `crates/nx-syntax/src/syntax_node.rs` for library imports, qualified selective aliases, `internal`, public-by-default declarations, and parse errors for removed or invalid forms

## 3. HIR and Visibility Modeling

- [x] 3.1 Update `crates/nx-hir/src/lib.rs` to remove `content_type`, rename import path data to library paths, store selective qualifier prefixes instead of free-form aliases, and add a `Visibility` model to top-level items
- [x] 3.2 Update `crates/nx-hir/src/lower.rs` to lower the new import forms and declaration visibility, reject invalid selective aliases, and stop lowering `contenttype`
- [x] 3.3 Extend HIR and scope tests to cover library-path imports, qualified selective aliases, `private` file scope, `internal` library scope, and default public visibility

## 4. Library Resolution and Diagnostics

- [x] 4.1 Implement local library loading for directory imports by recursively collecting `.nx` files under a canonical library root and caching the aggregated declaration index
- [x] 4.2 Update import resolution and name lookup to distinguish unqualified imports, namespace imports, and selective qualifier-prefix imports while filtering exports by visibility
- [x] 4.3 Add duplicate-library-import detection based on normalized local paths and emit clear compile errors before downstream resolution
- [x] 4.4 Add deferred ambiguity diagnostics that trigger only when an ambiguous unqualified imported name is used and include both source libraries plus remediation suggestions
- [x] 4.5 Accept Git and HTTP library paths syntactically but emit explicit "not yet supported" diagnostics during semantic resolution

## 5. Interpreter and Runtime Integration

- [x] 5.1 Thread the new library-resolution model through the interpreter or compilation entry points so imported public and internal declarations are available where resolution currently assumes a single module
- [x] 5.2 Ensure runtime/type resolution paths respect visibility boundaries and imported qualifier prefixes for records, enums, components, and functions
- [x] 5.3 Add integration coverage for multi-file local libraries, nested library directories, duplicate imports, ambiguous names, and visibility-restricted declarations

## 6. Editor, Documentation, and Fixtures

- [x] 6.1 Update VS Code grammar assets, samples, snippets, and grammar tests under `src/vscode/` to remove `contenttype`, use library-directory imports, and recognize `internal`
- [x] 6.2 Update language documentation under `docs/src/content/docs/` to describe libraries as recursive directories, selective qualifier prefixes, and the three visibility levels
- [x] 6.3 Update repository fixtures, examples, and archived references that must stay buildable under the new syntax and visibility rules

## 7. Verification

- [x] 7.1 Run the relevant syntax, HIR, interpreter, and VS Code test suites and fix regressions introduced by the import and visibility overhaul
- [x] 7.2 Add or update end-to-end cases that prove the requested behavior: local directory libraries, `import { Foo as Prefix.Foo }`, duplicate import errors, deferred ambiguity errors, and `private`/`internal`/public visibility boundaries
