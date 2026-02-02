# NX Language - Core Parsing and Type Checking

A Rust-based parser and type checker for the NX language, a modern XML-like syntax with embedded expressions.

## Overview

This workspace provides comprehensive parsing and type checking infrastructure for NX files:

- **Fast parsing** using tree-sitter (>10,000 lines/second)
- **Rich diagnostics** with beautiful error messages via Ariadne
- **Type inference** and compatibility checking
- **Incremental compilation** support via Salsa

## Architecture

The workspace is organized into four crates following a dependency hierarchy:

```
nx-diagnostics (foundation)
    ↓
nx-syntax (parsing)
    ↓
nx-hir (lowering & symbol resolution)
    ↓
nx-types (type checking & inference)
```

### Crates

- **[nx-diagnostics](crates/nx-diagnostics/)** - Error reporting with source spans and beautiful formatting
- **[nx-syntax](crates/nx-syntax/)** - Tree-sitter based parser producing Concrete Syntax Trees (CST)
- **[nx-hir](crates/nx-hir/)** - High-level Intermediate Representation with symbol resolution
- **[nx-types](crates/nx-types/)** - Type inference and checking with compatibility-based type system

## Quick Start

### Parsing NX Files

```rust
use nx_syntax::parse_file;

let result = parse_file("example.nx")?;
if result.is_ok() {
    println!("Parsed successfully!");
} else {
    for error in result.errors() {
        error.eprint();
    }
}
```

### Type Checking

```rust
use nx_types::check_str;

let source = r#"
let <Button text:string /> = <button>{text}</button>
"#;

let result = check_str(source, "example.nx");
if result.is_ok() {
    println!("Type checking passed!");
} else {
    for error in result.errors() {
        eprintln!("Error: {}", error.message());
    }
}
```

### Batch Processing

```rust
use nx_types::TypeCheckSession;

let mut session = TypeCheckSession::new();
session.add_file("button.nx", "let <Button /> = <button />");
session.add_file("app.nx", "let <App /> = <Button />");

for (name, result) in session.check_all() {
    println!("{}: {} errors", name, result.errors().len());
}
```

## Features

### Parsing (nx-syntax)
- ✅ XML-like element syntax with attributes
- ✅ Embedded expressions with `{...}`
- ✅ Type annotations (`:type`)
- ✅ Error recovery within scopes
- ✅ UTF-8 validation and encoding detection

### Type System (nx-types)
- ✅ Primitive types: `i32`, `i64`/`int`, `f32`, `f64`/`float`, `string`, `boolean`, `void`
- ✅ Compound types: arrays (`T[]`), functions, nullable (`T?`)
- ✅ Compatibility-based type checking
- ✅ Type inference for expressions
- ✅ Structural typing for elements

### Diagnostics (nx-diagnostics)
- ✅ Source span tracking with line/column
- ✅ Multi-label error messages
- ✅ Color-coded severity levels
- ✅ Beautiful formatting via Ariadne

## Development

### Prerequisites

- Rust 1.75 or later
- Node.js (for tree-sitter grammar development)

### Building

```bash
cargo build --workspace
```

### Testing

```bash
# Run all tests (197 tests)
cargo test --workspace

# Run tests for specific crate
cargo test -p nx-syntax
cargo test -p nx-types
```

### Documentation

```bash
# Build and open documentation
cargo doc --workspace --open

# Build documentation without dependencies
cargo doc --workspace --no-deps
```

### Code Quality

```bash
# Format code
cargo fmt --all

# Run linter
cargo clippy --workspace
```

## Performance

- **Parsing**: >10,000 lines/second
- **Type checking**: <100ms for typical files
- **Memory**: <100MB for 10,000+ line files
- **Incremental**: Full incremental compilation support

## Testing

The workspace includes comprehensive test coverage:

- **Unit tests**: 150+ tests across all crates
- **Integration tests**: 26 tests for end-to-end workflows
- **Snapshot tests**: Using `insta` for CST validation
- **Doc tests**: Embedded examples in documentation

## Project Status

✅ **Phase 1-4 Complete** (197 tests passing)
- Core parsing and validation
- Type system and inference
- Symbol resolution and HIR
- Comprehensive test coverage

🚧 **Phase 5 In Progress**
- Performance optimization
- Enhanced documentation
- Security hardening

## License

Copyright © 2024-2025. All rights reserved.

## Contributing

This is currently a private project. For questions or issues, please contact the maintainers.

## Documentation

- [Specification](specs/001-nx-core-parsing/spec.md) - Feature requirements and success criteria
- [Implementation Plan](specs/001-nx-core-parsing/plan.md) - Technical architecture and design
- [Tasks](specs/001-nx-core-parsing/tasks.md) - Detailed implementation checklist
- [API Documentation](target/doc/nx_types/index.html) - Generated rustdoc

## Related Work

- [tree-sitter](https://tree-sitter.github.io/) - Incremental parsing library
- [Salsa](https://github.com/salsa-rs/salsa) - Incremental computation framework
- [Ariadne](https://github.com/zesterer/ariadne) - Beautiful diagnostic reporting
