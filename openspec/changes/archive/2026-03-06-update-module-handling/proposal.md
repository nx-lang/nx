## Why

The current import syntax does not support string-path imports, namespace aliases, or selective symbol imports in the desired shape. This change standardizes module imports around path-first wildcard imports and explicit selective imports. Additionally, the language needs a `contenttype` directive to support prelude-style imports, and module paths should accept URLs in addition to file paths to enable future remote module support.

## What Changes

- **BREAKING**: Replace wildcard syntax with path-first forms:
  - `import "<path>"` (all exports into scope)
  - `import "<path>" as Foo` (namespace alias)
- Keep selective imports in braces:
  - `import { Stack as LayoutStack } from "<path>"` (selective import alias)
- Allow `<path>` in import statements to be a file path (relative or absolute) OR a URL. URL download/caching is not implemented yet — only syntax and plumbing support.
- Add a new `contenttype` directive: `contenttype "<path>"`. Must appear before any imports. Acts as a prelude, importing everything from the specified path.

## Capabilities

### New Capabilities
- `contenttype-directive`: The `contenttype "<path>"` directive that acts as a prelude, importing all exports from the specified path into the module scope. Must appear as the first statement in a file, before imports.

### Modified Capabilities
- `module-imports`: Update import syntax to use path-first wildcard imports, selective imports, and URL/file string paths.

## Impact

- **Grammar**: `grammar.js` needs significant updates — new `contenttype` rule, path-first wildcard imports, and selective imports
- **Syntax kinds**: New `SyntaxKind` variants for new tokens/nodes (`CONTENTTYPE`, `FROM`, `AS`, selective import nodes, and string literal paths)
- **Tree-sitter queries**: Update highlights/queries to match new grammar structure
- **HIR lowering**: Add import and contenttype lowering (currently imports are skipped)
- **Grammar docs**: Update `nx-grammar.md` and `nx-grammar-spec.md`
- **Language docs**: Update `modules-and-imports.md` and `modules.md`
- **Test fixtures**: Update existing `.nx` files that use old `import qualified.name` syntax
- **Example files**: Update all `.nx` examples to use new syntax
