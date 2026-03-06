## 1. Grammar Updates

- [x] 1.1 Add `from`, `as`, and `contenttype` keywords to grammar.js word list / keyword handling
- [x] 1.2 Replace the `import_statement` rule in grammar.js with new forms: wildcard import (`"path"` with optional `as Alias`) and selective import list (`{ Name, Name as Alias } from "path"`)
- [x] 1.3 Add `contenttype_statement` rule to grammar.js: `contenttype string_literal`
- [x] 1.4 Update `module_definition` rule to include optional `contenttype_statement` before imports
- [x] 1.5 Regenerate tree-sitter parser (run `tree-sitter generate`)

## 2. SyntaxKind Updates

- [x] 2.1 Add new node kinds to SyntaxKind enum: `CONTENTTYPE_STATEMENT`, `WILDCARD_IMPORT`, `SELECTIVE_IMPORT_LIST`, `SELECTIVE_IMPORT`, `MODULE_PATH`
- [x] 2.2 Add new keyword tokens to SyntaxKind enum: `FROM`, `AS`, `CONTENTTYPE`
- [x] 2.3 Add tree-sitter name-to-SyntaxKind mappings for all new kinds
- [x] 2.4 Update `is_token()` / `is_keyword()` classifications for new entries

## 3. Tree-Sitter Queries

- [x] 3.1 Update highlights.scm to highlight `from`, `as`, and `contenttype` as keywords
- [x] 3.2 Update any other tree-sitter queries (locals.scm, tags.scm) that reference import_statement

## 4. HIR Lowering

- [x] 4.1 Add `Import` struct to HIR with path, kind (wildcard/selective), alias, and selective entries
- [x] 4.2 Add `content_type: Option<String>` field to HIR `Module` struct
- [x] 4.3 Add `imports: Vec<Import>` field to HIR `Module` struct
- [x] 4.4 Implement `lower_import_statement` in lower.rs to extract import data from CST nodes
- [x] 4.5 Implement `lower_contenttype_statement` in lower.rs to extract the path string
- [x] 4.6 Update `lower_module` to call the new lowering functions instead of skipping imports

## 5. Grammar Documentation

- [x] 5.1 Update `nx-grammar.md` ImportStatement and ModuleDefinition rules to reflect new syntax
- [x] 5.2 Update `nx-grammar-spec.md` with new AST node types and grammar rules

## 6. Language Documentation

- [x] 6.1 Update `docs/src/content/docs/language-tour/modules-and-imports.md` with new syntax and contenttype
- [x] 6.2 Update `docs/src/content/docs/reference/syntax/modules.md` with new syntax and contenttype

## 7. Test Fixtures and Examples

- [x] 7.1 Update existing test fixture `.nx` files to use new import syntax (e.g., `import ui.components` → `import "./ui/components" as UiComponents`)
- [x] 7.2 Update example `.nx` files in `examples/nx/` and `src/vscode/samples/` to use new import syntax
- [x] 7.3 Add new parser test fixtures for wildcard imports, selective imports, aliased imports, and contenttype
- [x] 7.4 Add parser test fixtures for error cases (import without from, contenttype after import)

## 8. Unit Tests

- [x] 8.1 Add unit tests in syntax_node.rs for new import statement CST structure
- [x] 8.2 Add unit tests in parser_tests.rs for all import forms and contenttype
- [x] 8.3 Add HIR lowering tests for imports and contenttype
- [x] 8.4 Verify existing tests pass with updated fixtures
