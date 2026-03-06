## Context

The current import syntax is `import qualified.name` (e.g., `import ui.components`), using dot-separated identifiers with no aliasing, selective imports, or string paths. The grammar, SyntaxKind enum, and tree-sitter queries all reflect this simple form. Import statements are parsed into the CST but skipped entirely during HIR lowering — there is no cross-module resolution.

The desired syntax introduces path-first wildcard imports and explicit selective imports with `from`.

Several tokens already exist in SyntaxKind: `STAR`, `LBRACE`, `RBRACE`, `STRING_LITERAL`. New keywords needed: `from`, `as`, `contenttype`.

## Goals / Non-Goals

**Goals:**
- Implement path-first wildcard imports (`import "<path>"` / `import "<path>" as Alias`)
- Support string paths (file paths and URLs) in import statements
- Add `contenttype` directive as a prelude mechanism
- Update grammar, parser, SyntaxKind, tree-sitter queries, HIR lowering, grammar docs, language docs, and tests
- Plumb URL paths through the system (accept and store them)

**Non-Goals:**
- URL download/caching implementation (deferred)
- Full cross-module resolution or linking
- Module federation or dynamic imports
- Semantic validation of import paths (existence checking)

## Decisions

### 1. Import Syntax Forms

Four import forms:

```nx
import "<path>"
import "<path>" as Foo
import { Name1, Name2 } from "<path>"
import { Name1 as Alias1, Name2 } from "<path>"
```

- `import "<path>"` brings all public definitions into scope globally — they can be referenced directly by name.
- `import "<path>" as Namespace` brings all public definitions under a namespace — references must use `Namespace.<identifier>`.
- `import { ... } from "<path>"` brings specific definitions into scope.
- `import { Name as Alias } from "<path>"` brings a specific definition into scope under an alias.

**Rationale**: Path-first wildcard imports are concise and remove `*` punctuation from the common case. The alias still sits close to the wildcard path (`import "<path>" as Namespace`), while selective imports keep inline renaming for individual symbols.

### 2. Module Path Format

The `<path>` in import and contenttype statements is a string literal (quoted). It can be:
- A relative file path: `"./ui/controls"` or `"../shared/tokens"`
- An absolute file path: `"/lib/core"`
- A URL: `"https://example.com/nx-modules/ui.nx"`

No file extension is required. Path resolution rules (how to find the actual file) are out of scope for this change — only the syntax and data plumbing are being added.

**Rationale**: String paths (vs dot-separated identifiers) allow file paths and URLs naturally. This matches TypeScript/JavaScript conventions.

### 3. ContentType Directive

```nx
contenttype "https://example.com/schemas/html5.nx"
```

- Must be the first non-comment statement in a file (before imports)
- Semantically equivalent to `import "<path>"` — brings all exports from the specified path into scope
- Uses the existing `STRING_LITERAL` token for the path

**Rationale**: A dedicated directive (vs a special import) makes it visually distinct and enforceable as "must come first." It signals the document's schema/prelude at a glance.

### 4. Grammar Implementation

The `module_definition` rule changes from:
```
repeat(import_statement), repeat(definitions), optional(element)
```
to:
```
optional(contenttype_statement), repeat(import_statement), repeat(definitions), optional(element)
```

The `import_statement` rule changes from:
```
'import', qualified_name
```
to:
```
'import', wildcard_import
| 'import', selective_import_list, 'from', string_literal
```

Where:
- `wildcard_import`: `module_path [optional: 'as' identifier]`
- `selective_import_list`: `'{' selective_import (',' selective_import)* '}'`
- `selective_import`: `identifier ['as' identifier]`

New `contenttype_statement`: `'contenttype', string_literal`

### 5. New SyntaxKind Entries

New node types:
- `CONTENTTYPE_STATEMENT` — the contenttype directive node
- `WILDCARD_IMPORT` — `"<path>"` or `"<path>" as Name` node
- `SELECTIVE_IMPORT_LIST` — `{ Name1, Name2 as Alias }` node
- `SELECTIVE_IMPORT` — individual `Name` or `Name as Alias` within the list
- `MODULE_PATH` — the string path in import/contenttype (wraps STRING_LITERAL for semantic distinction)

New keyword tokens:
- `FROM` — `from` keyword
- `AS` — `as` keyword
- `CONTENTTYPE` — `contenttype` keyword

Tokens already existing and reused: `LBRACE`, `RBRACE`, `STRING_LITERAL`, `COMMA`.

### 6. HIR Lowering

Add basic lowering for import and contenttype statements into the HIR `Module` struct:
- Add `content_type: Option<String>` field to `Module`
- Add `imports: Vec<Import>` field to `Module`
- `Import` struct holds the path string and import kind (wildcard with optional alias, or selective list with optional aliases)

This stores parsed import data without implementing resolution.

### 7. Backward Compatibility

This is a **breaking change** to the import syntax. The old `import qualified.name` form is removed entirely. All existing `.nx` files and test fixtures must be updated.

The migration is straightforward since no module resolution exists yet — the old imports were parsed but never used at runtime.

## Risks / Trade-offs

- **Breaking all existing imports** → Low risk since import resolution isn't implemented; existing imports are decorative. All test fixtures and examples need updating.
- **URL path support without validation** → URLs will be accepted syntactically but cannot be resolved. Code that encounters a URL path should handle it gracefully (store it, don't crash). → Document this as a known limitation.
- **`as` and `from` become reserved keywords** → Could conflict with identifiers named `as` or `from`. → Acceptable trade-off; these are standard keywords in similar languages. The grammar's `word` rule and tree-sitter's keyword extraction should handle this.
