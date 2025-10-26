# NX Language - Rust Implementation

This directory contains the Rust implementation of the NX language, following the architecture described in [../nx-rust-plan.md](../nx-rust-plan.md).

## Current Status

**Phase 0 (Foundation): ✅ Complete**

The Rust workspace foundation has been established with:
- Rust toolchain (v1.75) installed and configured
- Cargo workspace with 5 initial crates
- `nx-diagnostics` crate fully implemented with beautiful error reporting using Ariadne
- Code formatted and linted

**Phase 1 (tree-sitter Grammar + CST): 🚧 In Progress (~60% Complete)**

tree-sitter grammar and parser infrastructure:
- ✅ Complete tree-sitter grammar in [grammar.js](nx-syntax/grammar.js) (542 lines)
- ✅ Parser successfully generated (~525KB parser.c)
- ✅ External scanner stub for text content tokens
- ✅ Build infrastructure with cc-rs integration
- ✅ Basic Rust API: `language()`, `parser()`, `parse()`
- ✅ 9 passing integration tests
- ✅ 8 sample .nx files demonstrating language features
- ⏳ Typed Rust wrappers for CST nodes (pending)
- ⏳ Syntax highlighting queries (pending)
- ⏳ VS Code integration (pending)

All workspace tests passing: **18 tests** (9 nx-diagnostics + 9 nx-syntax)

## Crate Structure

```
crates/
├── nx-diagnostics/   ✅ Complete - Error reporting with Ariadne (9 tests)
├── nx-syntax/        🚧 In Progress - CST + tree-sitter parsing (9 tests)
├── nx-hir/           📝 Phase 2 - AST + semantic model
├── nx-types/         📝 Phase 3 - Type system
└── nx-cli/           📝 Phases 1-6 - Command-line tools
```

## Dependency Graph

```
nx-diagnostics (leaf crate, zero NX deps) ✅
    ↑
nx-syntax (tree-sitter + CST + parsing)
    ↑
nx-hir (AST + lowering from CST)
    ↑
nx-types (type checker + inference)
    ↑
nx-lsp (LSP server using all layers)
nx-cli (CLI tools using all layers)
```

## Getting Started

### Prerequisites

- Rust 1.75+ (installed via rustup)
- cargo, rustfmt, clippy

### Building

```bash
# Build all crates
cargo build --workspace

# Build in release mode
cargo build --workspace --release

# Build specific crate
cargo build -p nx-diagnostics
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run tests for specific crate
cargo test -p nx-diagnostics

# Run tests with output
cargo test --workspace -- --nocapture
```

### Linting and Formatting

```bash
# Run clippy
cargo clippy --workspace

# Format code
cargo fmt --all

# Check formatting without modifying files
cargo fmt --all --check
```

### Running the CLI

```bash
# Debug build
cargo run -p nx-cli

# Release build
cargo run -p nx-cli --release

# Or run the binary directly
./target/release/nx
```

## Next Steps

**Remaining Phase 1 Tasks:**

1. ✅ ~~Port NX grammar to tree-sitter `grammar.js`~~
2. ✅ ~~Generate parser and test against sample `.nx` files~~
3. 🚧 Build typed Rust wrappers over tree-sitter nodes
4. 📝 Write tree-sitter queries for syntax highlighting
5. 📝 Integrate into VS Code extension

**Phase 2: HIR + Semantic Analysis** (next major phase)

See [../nx-rust-plan.md](../nx-rust-plan.md) for the complete implementation roadmap.

## Sample Files

Example NX files for testing are available in [../examples/nx/](../examples/nx/):
- `hello.nx` - Basic markup
- `function.nx` - Function definitions with properties
- `expressions.nx` - Binary, conditional, and interpolation expressions
- `conditionals.nx` - If/match/condition list expressions
- `loops.nx` - For loops with indexing
- `types.nx` - Type definitions and nullable/list types
- `embed.nx` - Embedded content with text types
- `complex.nx` - Real-world todo app example

Test the parser with tree-sitter CLI:
```bash
cd crates/nx-syntax
tree-sitter parse ../../examples/nx/function.nx
```

## Documentation

- [Rust Implementation Plan](../nx-rust-plan.md) - Complete architecture and phases
- [NX Grammar Specification](../nx-grammar-spec.md) - Machine-readable grammar
- [NX Grammar](../nx-grammar.md) - Human-readable grammar
- Individual crate documentation: `cargo doc --open`

## Contributing

See [../CONTRIBUTING.md](../CONTRIBUTING.md) for contribution guidelines.
