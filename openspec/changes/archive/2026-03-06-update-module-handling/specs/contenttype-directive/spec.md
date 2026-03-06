## ADDED Requirements

### Requirement: ContentType directive syntax
The parser SHALL recognize a `contenttype` directive with the form `contenttype "<path>"`, where `<path>` is a string literal representing a file path (relative or absolute) or a URL.

#### Scenario: ContentType with relative file path
- **WHEN** a file contains `contenttype "./schemas/html5"`
- **THEN** the parser SHALL produce a CONTENTTYPE_STATEMENT node containing a MODULE_PATH with value `./schemas/html5`

#### Scenario: ContentType with URL
- **WHEN** a file contains `contenttype "https://example.com/schemas/html5.nx"`
- **THEN** the parser SHALL produce a CONTENTTYPE_STATEMENT node containing a MODULE_PATH with value `https://example.com/schemas/html5.nx`

### Requirement: ContentType must appear before imports
The `contenttype` directive, if present, MUST be the first non-comment statement in the file. It SHALL appear before any import statements or definitions.

#### Scenario: ContentType before imports parses successfully
- **WHEN** a file contains `contenttype "./prelude"` followed by `import "./ui" as UI`
- **THEN** the parser SHALL produce a valid module with both a contenttype statement and an import statement

#### Scenario: ContentType after import is a parse error
- **WHEN** a file contains an import statement followed by `contenttype "./prelude"`
- **THEN** the parser SHALL produce a parse error

### Requirement: ContentType is optional
A module SHALL be valid with or without a `contenttype` directive. At most one `contenttype` directive is allowed per file.

#### Scenario: Module without contenttype
- **WHEN** a file contains only import statements and definitions
- **THEN** the parser SHALL produce a valid module with no contenttype statement

#### Scenario: Module with only contenttype
- **WHEN** a file contains only `contenttype "./prelude"` followed by an element
- **THEN** the parser SHALL produce a valid module with a contenttype statement and an element

### Requirement: ContentType in HIR
The HIR Module struct SHALL include an optional content type field that stores the path string from the contenttype directive.

#### Scenario: ContentType lowered to HIR
- **WHEN** a file contains `contenttype "./prelude"`
- **THEN** the HIR Module SHALL have its content_type field set to `Some("./prelude")`

#### Scenario: No contenttype in HIR
- **WHEN** a file has no contenttype directive
- **THEN** the HIR Module SHALL have its content_type field set to `None`
