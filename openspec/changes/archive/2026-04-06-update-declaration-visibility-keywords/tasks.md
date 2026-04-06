## 1. Grammar and Documentation

- [x] 1.1 Update [nx-grammar.md](/home/bret/src/nexara/external/nx/nx-grammar.md) to remove the `internal` visibility keyword, document default internal visibility, and add `export` for externally visible declarations
- [x] 1.2 Update [nx-grammar-spec.md](/home/bret/src/nexara/external/nx/nx-grammar-spec.md) and related language docs to describe `private`, default internal visibility, `export`, and the non-library program rule
- [x] 1.3 Review the grammar docs against the OpenSpec delta so the published syntax and visibility semantics stay aligned before implementation lands

## 2. Parser and Syntax Tree

- [x] 2.1 Update `crates/nx-syntax/grammar.js` and related keyword handling to parse `export` as a top-level visibility modifier and remove `internal` from valid source syntax
- [x] 2.2 Regenerate tree-sitter artifacts and refresh syntax metadata in generated parser files, syntax kinds, and CST helpers for the revised visibility keywords
- [x] 2.3 Update parser and CST tests to cover `private`, default internal visibility, and `export`

## 3. HIR and Semantic Visibility

- [x] 3.1 Update the HIR visibility model in `crates/nx-hir/src/lib.rs` so omitted modifiers lower to semantic `internal` visibility and `export` lowers explicitly
- [x] 3.2 Update lowering in `crates/nx-hir/src/lower.rs` to preserve the new source-to-visibility mapping for omitted modifiers and explicit `export`
- [x] 3.3 Extend HIR and scope tests for file-local `private`, default internal visibility, and explicit `export`

## 4. Resolution and Export Metadata

- [x] 4.1 Update library export-table construction and import resolution so only `export` declarations are visible to importing consumer libraries
- [x] 4.2 Ensure same-library and same-program resolution continue to see declarations with default internal visibility across source files
- [x] 4.3 Add integration coverage for private isolation, library-local default visibility, and explicit export visibility

## 5. Editor, Fixtures, and Verification

- [x] 5.1 Update VS Code grammar assets, repository fixtures, and examples to remove stale `internal` references and recognize `export`
- [x] 5.2 Update language reference content under `docs/` to reflect the new visibility model consistently
- [x] 5.3 Run the relevant syntax, HIR, interpreter, and editor test suites and fix regressions introduced by the visibility keyword change
