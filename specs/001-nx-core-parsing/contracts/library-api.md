# Library API Contract: Core NX Parsing and Validation

**Feature**: 001-nx-core-parsing
**Date**: 2025-10-26

## Overview

This document defines the public Rust API for the NX parsing and type checking library. The API is designed to be:

- **Thread-safe**: All types are `Send + Sync`
- **Ergonomic**: Builder patterns, clear ownership
- **Performant**: Zero-copy where possible, arena allocation
- **Well-documented**: All public APIs have rustdoc comments

---

## API Layers

The library provides three levels of API:

1. **High-level convenience** - Single function calls for common tasks
2. **Session-based** - Stateful session for batch operations
3. **Low-level** - Direct access to CST/HIR/types for advanced use cases

---

## 1. High-Level Convenience API

### Parse Functions

```rust
/// Parse NX source code into a syntax tree
///
/// # Examples
///
/// ```
/// use nx_syntax::parse_str;
///
/// let source = "let x = 42";
/// let result = parse_str(source, "example.nx");
///
/// assert!(result.tree.is_some());
/// assert_eq!(result.errors.len(), 0);
/// ```
pub fn parse_str(source: &str, file_name: &str) -> ParseResult;

/// Parse NX source from a file path
///
/// # Errors
///
/// Returns `Err` if the file cannot be read or is not valid UTF-8.
pub fn parse_file(path: impl AsRef<Path>) -> io::Result<ParseResult>;
```

### Type Checking Functions

```rust
/// Parse and type-check NX source code
///
/// # Examples
///
/// ```
/// use nx_types::check_str;
///
/// let source = "fn add(a: int, b: int) { a + b }";
/// let result = check_str(source, "example.nx");
///
/// assert!(result.diagnostics.is_empty());
/// ```
pub fn check_str(source: &str, file_name: &str) -> TypeCheckResult;

/// Parse and type-check NX source from a file
///
/// # Errors
///
/// Returns `Err` if the file cannot be read or is not valid UTF-8.
pub fn check_file(path: impl AsRef<Path>) -> io::Result<TypeCheckResult>;
```

---

## 2. Session-Based API

### ParserSession

**Purpose**: Efficient batch parsing of multiple files with shared state

```rust
/// A parsing session for batch operations
///
/// # Thread Safety
///
/// `ParserSession` is `Send + Sync` and can be shared across threads.
///
/// # Examples
///
/// ```
/// use nx_syntax::ParserSession;
/// use rayon::prelude::*;
///
/// let session = ParserSession::new();
/// let files = vec!["a.nx", "b.nx", "c.nx"];
///
/// let results: Vec<_> = files
///     .par_iter()
///     .map(|file| session.parse_file(file))
///     .collect();
/// ```
#[derive(Clone)]
pub struct ParserSession {
    // Internal state (salsa database, caches)
}

impl ParserSession {
    /// Create a new parser session
    pub fn new() -> Self;

    /// Parse source code within this session
    pub fn parse_str(&self, source: &str, file_name: &str) -> ParseResult;

    /// Parse a file within this session
    pub fn parse_file(&self, path: impl AsRef<Path>) -> io::Result<ParseResult>;

    /// Clear cached parse results (for memory management)
    pub fn clear_cache(&mut self);
}
```

### TypeCheckSession

**Purpose**: Efficient batch type checking with shared type environment

```rust
/// A type checking session for batch operations
///
/// # Thread Safety
///
/// `TypeCheckSession` is `Send + Sync` and can be shared across threads.
///
/// # Examples
///
/// ```
/// use nx_types::TypeCheckSession;
///
/// let mut session = TypeCheckSession::new();
///
/// // Add files to session
/// session.add_file("a.nx", include_str!("a.nx"));
/// session.add_file("b.nx", include_str!("b.nx"));
///
/// // Type check all files
/// let results = session.check_all();
/// ```
#[derive(Clone)]
pub struct TypeCheckSession {
    parser: ParserSession,
    // Type environment, symbol tables
}

impl TypeCheckSession {
    /// Create a new type checking session
    pub fn new() -> Self;

    /// Add a source file to the session
    pub fn add_file(&mut self, name: impl Into<String>, source: impl Into<String>);

    /// Type check a single file
    pub fn check_file(&self, name: &str) -> Option<TypeCheckResult>;

    /// Type check all files in the session
    pub fn check_all(&self) -> Vec<(String, TypeCheckResult)>;

    /// Get diagnostics for all files
    pub fn diagnostics(&self) -> Vec<Diagnostic>;
}
```

---

## 3. Low-Level API

### ParseResult

```rust
/// Result of parsing NX source code
pub struct ParseResult {
    /// The parsed syntax tree (None if fatal parse error)
    pub tree: Option<SyntaxTree>,

    /// Parse errors and warnings
    pub errors: Vec<Diagnostic>,

    /// Source file identifier
    pub source_id: SourceId,
}

impl ParseResult {
    /// Check if parsing succeeded (no errors)
    pub fn is_ok(&self) -> bool {
        self.errors.iter().all(|d| d.severity != Severity::Error)
    }

    /// Get the root syntax node
    pub fn root(&self) -> Option<SyntaxNode> {
        self.tree.as_ref().map(|t| t.root())
    }

    /// Lower CST to HIR module
    pub fn to_hir(&self) -> Option<Module>;
}
```

### SyntaxTree

```rust
/// Immutable syntax tree from tree-sitter
pub struct SyntaxTree {
    // Internal tree-sitter tree
}

impl SyntaxTree {
    /// Get the root node
    pub fn root(&self) -> SyntaxNode;

    /// Get source text for a span
    pub fn text(&self, span: TextSpan) -> &str;

    /// Get the full source code
    pub fn source(&self) -> &str;

    /// Find node at byte offset
    pub fn node_at(&self, offset: usize) -> Option<SyntaxNode>;
}
```

### SyntaxNode

```rust
/// Typed wrapper around tree-sitter node
#[derive(Copy, Clone)]
pub struct SyntaxNode<'tree> {
    // Internal tree-sitter node handle
}

impl<'tree> SyntaxNode<'tree> {
    /// Get the node kind
    pub fn kind(&self) -> SyntaxKind;

    /// Get source text
    pub fn text(&self) -> &'tree str;

    /// Get source span
    pub fn span(&self) -> TextSpan;

    /// Get child nodes
    pub fn children(&self) -> impl Iterator<Item = SyntaxNode<'tree>>;

    /// Get named child by field name
    pub fn child_by_field(&self, field: &str) -> Option<SyntaxNode<'tree>>;

    /// Check if node is an error node
    pub fn is_error(&self) -> bool;

    /// Convert to typed AST node (via casting)
    pub fn cast<T: AstNode>(&self) -> Option<T>;
}
```

### Module (HIR)

```rust
/// High-level intermediate representation of a module
pub struct Module {
    // Arena-backed AST nodes
}

impl Module {
    /// Get all top-level items
    pub fn items(&self) -> &[Item];

    /// Find item by name
    pub fn find_item(&self, name: &str) -> Option<&Item>;

    /// Get module scope
    pub fn scope(&self) -> &Scope;
}
```

### TypeCheckResult

```rust
/// Result of type checking
pub struct TypeCheckResult {
    /// The parsed module (None if parse failed)
    pub module: Option<Module>,

    /// Type environment (symbol → type mappings)
    pub type_env: TypeEnvironment,

    /// Type checking diagnostics
    pub diagnostics: Vec<Diagnostic>,

    /// Type information for all expressions
    pub expr_types: ExprTypeMap,
}

impl TypeCheckResult {
    /// Check if type checking succeeded (no errors)
    pub fn is_ok(&self) -> bool {
        self.diagnostics.iter().all(|d| d.severity != Severity::Error)
    }

    /// Get the type of an expression
    pub fn type_of(&self, expr: ExprId) -> Option<&Type>;

    /// Get all type errors
    pub fn errors(&self) -> Vec<&Diagnostic> {
        self.diagnostics.iter()
            .filter(|d| d.severity == Severity::Error)
            .collect()
    }
}
```

### Diagnostic

```rust
/// A diagnostic message (error, warning, or info)
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Severity level
    pub severity: Severity,

    /// Primary message
    pub message: String,

    /// Source location
    pub span: TextSpan,

    /// Additional labeled spans
    pub labels: Vec<Label>,

    /// Help/note messages
    pub notes: Vec<String>,
}

impl Diagnostic {
    /// Render diagnostic with Ariadne
    pub fn render(&self, source: &str) -> String;

    /// Print diagnostic to stderr with colors
    pub fn eprint(&self, source: &str);
}
```

### TextSpan

```rust
/// A span of text in a source file
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TextSpan {
    /// Start byte offset
    pub start: u32,

    /// End byte offset (exclusive)
    pub end: u32,
}

impl TextSpan {
    /// Create a new span
    pub fn new(start: u32, end: u32) -> Self;

    /// Get span length
    pub fn len(&self) -> u32 {
        self.end - self.start
    }

    /// Check if span contains a position
    pub fn contains(&self, pos: u32) -> bool {
        self.start <= pos && pos < self.end
    }

    /// Merge two spans
    pub fn merge(&self, other: TextSpan) -> TextSpan {
        TextSpan::new(
            self.start.min(other.start),
            self.end.max(other.end),
        )
    }
}
```

---

## Error Handling

All fallible operations return `Result`:

```rust
pub type Result<T> = std::result::Result<T, Error>;

/// Library error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid UTF-8 in source file")]
    InvalidUtf8,

    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Parse error: {0}")]
    Parse(String),
}
```

---

## Thread Safety Guarantees

All public types implement `Send + Sync`:

```rust
// Auto-implemented via internal design
unsafe impl Send for ParserSession {}
unsafe impl Sync for ParserSession {}

unsafe impl Send for TypeCheckSession {}
unsafe impl Sync for TypeCheckSession {}

// Syntax types are Copy (thread-safe by definition)
impl Copy for SyntaxNode<'_> {}
```

**Concurrent Usage Example**:

```rust
use rayon::prelude::*;

let session = ParserSession::new();
let files = vec!["a.nx", "b.nx", "c.nx"];

// Parse in parallel
let results: Vec<_> = files
    .par_iter()
    .map(|file| session.parse_file(file))
    .collect();
```

---

## Performance Characteristics

| Operation | Time Complexity | Space Complexity |
|-----------|----------------|------------------|
| Parse (cold) | O(n) | O(n) |
| Parse (incremental) | O(changed) | O(n) |
| Type check | O(n * log n) | O(n) |
| Lookup symbol | O(log n) | O(1) |
| Render diagnostic | O(lines) | O(1) |

Where `n` = source code size in bytes/tokens.

---

## Examples

### Basic Parsing

```rust
use nx_syntax::parse_str;

fn main() {
    let source = r#"
        fn greet(name: string) {
            "Hello, " + name
        }
    "#;

    let result = parse_str(source, "example.nx");

    if result.is_ok() {
        println!("Parse succeeded!");
        let root = result.root().unwrap();
        println!("Root kind: {:?}", root.kind());
    } else {
        for error in result.errors {
            error.eprint(source);
        }
    }
}
```

### Type Checking

```rust
use nx_types::check_str;

fn main() {
    let source = r#"
        fn add(a: int, b: int) {
            a + b
        }

        fn main() {
            let result = add(1, 2);
            result
        }
    "#;

    let result = check_str(source, "example.nx");

    if result.is_ok() {
        println!("Type checking passed!");
    } else {
        println!("Type errors:");
        for diag in result.errors() {
            diag.eprint(source);
        }
    }
}
```

### Batch Processing

```rust
use nx_types::TypeCheckSession;
use std::fs;

fn main() -> std::io::Result<()> {
    let mut session = TypeCheckSession::new();

    // Load all .nx files
    for entry in fs::read_dir("src")? {
        let entry = entry?;
        let path = entry.path();

        if path.extension() == Some("nx".as_ref()) {
            let name = path.file_name().unwrap().to_str().unwrap();
            let source = fs::read_to_string(&path)?;
            session.add_file(name, source);
        }
    }

    // Type check all files
    let results = session.check_all();

    for (name, result) in results {
        println!("Checking {}:", name);
        if !result.is_ok() {
            for diag in result.errors() {
                println!("  {}", diag.message);
            }
        }
    }

    Ok(())
}
```

---

## API Stability

**Pre-1.0 Policy**: The API is unstable and may change between minor versions. Breaking changes will be noted in CHANGELOG.

**Post-1.0 Policy**: The API will follow semantic versioning. Breaking changes will only occur in major versions.

---

## Testing the API

All public APIs have:

1. **Unit tests**: Test individual functions
2. **Integration tests**: Test end-to-end workflows
3. **Doc tests**: Examples in rustdoc must compile and pass

**Example doc test**:

```rust
/// Parse NX source code
///
/// # Examples
///
/// ```
/// use nx_syntax::parse_str;
///
/// let result = parse_str("let x = 42", "test.nx");
/// assert!(result.is_ok());
/// ```
pub fn parse_str(source: &str, file_name: &str) -> ParseResult {
    // Implementation
}
```

---

## Next Steps

1. Implement core types in `nx-diagnostics` crate
2. Implement CST layer in `nx-syntax` crate
3. Implement HIR layer in `nx-hir` crate
4. Implement type system in `nx-types` crate
5. Add comprehensive tests and documentation

---

**API Contract Status**: ✅ Complete - Ready for implementation
