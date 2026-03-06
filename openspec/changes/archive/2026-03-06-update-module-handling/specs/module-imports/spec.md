## ADDED Requirements

### Requirement: Wildcard import (global)
The parser SHALL support wildcard imports with the syntax `import "<path>"`. All public definitions from the imported module are brought into scope globally and can be referenced directly by name.

#### Scenario: Wildcard import without alias
- **WHEN** a file contains `import "./ui/controls"`
- **THEN** the parser SHALL produce an IMPORT_STATEMENT containing a WILDCARD_IMPORT node with no alias and a MODULE_PATH with value `./ui/controls`

#### Scenario: Wildcard import with URL path
- **WHEN** a file contains `import "https://cdn.example.com/lib.nx"`
- **THEN** the parser SHALL produce an IMPORT_STATEMENT with a WILDCARD_IMPORT node with no alias and a MODULE_PATH with value `https://cdn.example.com/lib.nx`

### Requirement: Namespace import
The parser SHALL support namespace imports with the syntax `import "<path>" as <Namespace>`, where `<Namespace>` is an identifier. All public definitions are available under the namespace and MUST be referenced as `<Namespace>.<identifier>`.

#### Scenario: Namespace import
- **WHEN** a file contains `import "./ui/controls" as UI`
- **THEN** the parser SHALL produce an IMPORT_STATEMENT containing a WILDCARD_IMPORT node with namespace `UI` and a MODULE_PATH with value `./ui/controls`

### Requirement: Selective imports
The parser SHALL support selective imports with the syntax `import { <Name1>, <Name2> } from "<path>"`, where each name is an identifier.

#### Scenario: Single selective import
- **WHEN** a file contains `import { Button } from "./ui/controls"`
- **THEN** the parser SHALL produce an IMPORT_STATEMENT containing a SELECTIVE_IMPORT_LIST with one SELECTIVE_IMPORT for `Button`

#### Scenario: Multiple selective imports
- **WHEN** a file contains `import { Button, Input, Label } from "./ui/controls"`
- **THEN** the parser SHALL produce an IMPORT_STATEMENT containing a SELECTIVE_IMPORT_LIST with three SELECTIVE_IMPORT entries

### Requirement: Selective imports with aliases
The parser SHALL support aliasing individual selective imports with the syntax `import { <Name> as <Alias> } from "<path>"`.

#### Scenario: Selective import with alias
- **WHEN** a file contains `import { Stack as LayoutStack } from "./layout"`
- **THEN** the parser SHALL produce an IMPORT_STATEMENT with a SELECTIVE_IMPORT_LIST containing a SELECTIVE_IMPORT with name `Stack` and alias `LayoutStack`

#### Scenario: Mixed aliased and non-aliased imports
- **WHEN** a file contains `import { Stack as LayoutStack, Button } from "./ui"`
- **THEN** the parser SHALL produce two SELECTIVE_IMPORT entries: one with alias and one without

### Requirement: Module path supports file paths and URLs
The `<path>` in an import statement SHALL accept relative file paths, absolute file paths, and URLs as string literals.

#### Scenario: Relative file path
- **WHEN** a file contains `import { Foo } from "./foo"`
- **THEN** the parser SHALL accept the path `./foo`

#### Scenario: Absolute file path
- **WHEN** a file contains `import { Foo } from "/lib/foo"`
- **THEN** the parser SHALL accept the path `/lib/foo`

#### Scenario: URL path
- **WHEN** a file contains `import { Foo } from "https://example.com/foo.nx"`
- **THEN** the parser SHALL accept the URL path

### Requirement: Selective import requires from clause
Selective imports MUST include a `from "<path>"` clause. The old `import qualified.name` syntax is removed.

#### Scenario: Import without from clause is a parse error
- **WHEN** a file contains `import ui.components`
- **THEN** the parser SHALL produce a parse error

### Requirement: Imports in HIR
The HIR Module struct SHALL include an imports list. Each import SHALL store the module path, import kind (wildcard or selective), optional alias (for wildcard), and individual name/alias pairs (for selective imports).

#### Scenario: Wildcard import without alias lowered to HIR
- **WHEN** a file contains `import "./ui"`
- **THEN** the HIR Module SHALL contain an import with path `./ui`, kind wildcard, and alias `None`

#### Scenario: Namespace import lowered to HIR
- **WHEN** a file contains `import "./ui" as UI`
- **THEN** the HIR Module SHALL contain an import with path `./ui`, kind wildcard, and alias `Some("UI")`

#### Scenario: Selective imports lowered to HIR
- **WHEN** a file contains `import { Button, Stack as LayoutStack } from "./ui"`
- **THEN** the HIR Module SHALL contain an import with path `./ui`, kind selective, and entries `[("Button", None), ("Stack", Some("LayoutStack"))]`

### Requirement: Multiple import statements
A module SHALL support zero or more import statements, all appearing after the optional contenttype directive and before definitions.

#### Scenario: Multiple imports
- **WHEN** a file contains two import statements from different paths
- **THEN** the parser SHALL produce a valid module with both import statements

#### Scenario: No imports
- **WHEN** a file contains only definitions and an element
- **THEN** the parser SHALL produce a valid module with an empty imports list
