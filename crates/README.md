# NX Language - Rust Implementation

This directory contains the Rust implementation of the NX language, following the architecture described in [../nx-rust-plan.md](../nx-rust-plan.md).

## Current Status

**Phase 0 (Foundation): ✅ Complete**

The Rust workspace foundation has been established with:
- Rust toolchain (v1.75) installed and configured
- Cargo workspace with 5 initial crates
- `nx-diagnostics` crate fully implemented with beautiful error reporting using Ariadne
- All tests passing (9 tests)
- Code formatted and linted

## Crate Structure

```
crates/
├── nx-diagnostics/   ✅ Complete - Error reporting with Ariadne
├── nx-syntax/        🚧 Phase 1 - CST + tree-sitter parsing (next)
├── nx-hir/           🚧 Phase 2 - AST + semantic model
├── nx-types/         🚧 Phase 3 - Type system
└── nx-cli/           🚧 Phases 1-6 - Command-line tools
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

**Phase 1: tree-sitter Grammar + CST (3-4 weeks)**

1. Port NX grammar to tree-sitter `grammar.js`
2. Generate parser and test against sample `.nx` files
3. Build typed Rust wrappers over tree-sitter nodes
4. Write tree-sitter queries for syntax highlighting
5. Integrate into VS Code extension

See [../nx-rust-plan.md](../nx-rust-plan.md) for the complete implementation roadmap.

## Documentation

- [Rust Implementation Plan](../nx-rust-plan.md) - Complete architecture and phases
- [NX Grammar Specification](../nx-grammar-spec.md) - Machine-readable grammar
- [NX Grammar](../nx-grammar.md) - Human-readable grammar
- Individual crate documentation: `cargo doc --open`

## Contributing

See [../CONTRIBUTING.md](../CONTRIBUTING.md) for contribution guidelines.
