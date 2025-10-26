# Quick Start: Core NX Parsing and Validation

**Feature**: 001-nx-core-parsing
**Date**: 2025-10-26
**For**: Developers implementing or using the NX parsing library

## Overview

This guide helps you quickly get started with the NX parsing and type checking library, whether you're implementing the library or using it in your project.

---

## For Implementers

### Prerequisites

- Rust 1.75+ installed (`rustup show` to check)
- Familiarity with Rust basics
- Understanding of parsing concepts (AST, CST, type systems)

### Development Setup

1. **Clone and navigate to workspace**:
   ```bash
   cd /home/bret/src/nx
   git checkout 001-nx-core-parsing
   ```

2. **Build the workspace**:
   ```bash
   cargo build --workspace
   ```

3. **Run tests**:
   ```bash
   cargo test --workspace
   ```

4. **Check formatting and lints**:
   ```bash
   cargo fmt --all --check
   cargo clippy --workspace
   ```

### Project Structure

```
crates/
├── nx-diagnostics/   # Start here - Error formatting
├── nx-syntax/        # Then - CST + parser
├── nx-hir/          # Then - AST + lowering
└── nx-types/        # Finally - Type system
```

### Implementation Order

Follow this order per `plan.md`:

1. **nx-diagnostics** (Week 1):
   - Implement `Diagnostic`, `Severity`, `Label` types
   - Integrate Ariadne for beautiful error output
   - Add tests for diagnostic rendering

2. **nx-syntax** (Weeks 2-4):
   - Implement tree-sitter grammar in `grammar.js`
   - Create typed `SyntaxNode` wrappers
   - Add CST traversal helpers
   - Write snapshot tests with `insta`

3. **nx-hir** (Weeks 5-7):
   - Define AST node types (Expr, Stmt, Item, etc.)
   - Implement CST → AST lowering
   - Set up Salsa database
   - Add lowering tests

4. **nx-types** (Weeks 8-12):
   - Implement type representation
   - Build type inference engine
   - Add type unification
   - Write comprehensive type checking tests

### Running Individual Crates

```bash
# Build specific crate
cargo build -p nx-syntax

# Test specific crate
cargo test -p nx-syntax

# Run with example
cargo run -p nx-cli -- parse examples/hello.nx
```

### Snapshot Testing

We use `insta` for snapshot tests:

```bash
# Review snapshot changes
cargo insta review

# Accept all snapshot changes
cargo insta accept

# Reject all snapshot changes
cargo insta reject
```

### Common Development Tasks

**Add a new AST node type**:
1. Define type in `crates/nx-hir/src/ast/*.rs`
2. Add lowering logic in `crates/nx-hir/src/lower.rs`
3. Write tests in `crates/nx-hir/tests/lowering_tests.rs`
4. Update snapshot tests

**Fix a parse error**:
1. Add test case in `crates/nx-syntax/tests/fixtures/invalid/`
2. Run tests to confirm failure
3. Update `grammar.js` or error recovery logic
4. Re-run tests and accept new snapshots

**Add a type checking rule**:
1. Add test case in `crates/nx-types/tests/fixtures/`
2. Implement rule in `crates/nx-types/src/infer.rs`
3. Add diagnostic in `crates/nx-types/src/errors.rs`
4. Run tests

---

## For Users

### Adding NX Parser to Your Project

Add to `Cargo.toml`:

```toml
[dependencies]
nx-syntax = { path = "../nx/crates/nx-syntax" }
nx-types = { path = "../nx/crates/nx-types" }
```

### Basic Usage

**Parse a file**:

```rust
use nx_syntax::parse_file;

fn main() -> std::io::Result<()> {
    let result = parse_file("example.nx")?;

    if result.is_ok() {
        println!("Parse succeeded!");
    } else {
        for error in result.errors {
            error.eprint(result.tree.as_ref().unwrap().source());
        }
    }

    Ok(())
}
```

**Type check a file**:

```rust
use nx_types::check_file;

fn main() -> std::io::Result<()> {
    let result = check_file("example.nx")?;

    if result.is_ok() {
        println!("Type check passed!");
    } else {
        for error in result.errors() {
            error.eprint(result.module.as_ref().unwrap().source());
        }
    }

    Ok(())
}
```

**Batch processing**:

```rust
use nx_types::TypeCheckSession;
use std::fs;

fn main() -> std::io::Result<()> {
    let mut session = TypeCheckSession::new();

    // Load all files
    for entry in fs::read_dir("src")? {
        let entry = entry?;
        let path = entry.path();

        if path.extension() == Some("nx".as_ref()) {
            let name = path.file_name().unwrap().to_str().unwrap();
            let source = fs::read_to_string(&path)?;
            session.add_file(name, source);
        }
    }

    // Check all
    let results = session.check_all();

    // Report errors
    for (name, result) in results {
        if !result.is_ok() {
            println!("Errors in {}:", name);
            for error in result.errors() {
                println!("  {}", error.message);
            }
        }
    }

    Ok(())
}
```

---

## Testing Your Code

### Unit Tests

Add tests inline:

```rust
pub fn parse_str(source: &str) -> ParseResult {
    // Implementation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let result = parse_str("let x = 42");
        assert!(result.is_ok());
    }
}
```

### Integration Tests

Add to `tests/` directory:

```rust
// tests/parse_examples.rs
use nx_syntax::parse_file;

#[test]
fn test_parse_hello_world() {
    let result = parse_file("examples/hello.nx").unwrap();
    assert!(result.is_ok());
}
```

### Snapshot Tests

```rust
use insta::assert_snapshot;

#[test]
fn test_parse_function() {
    let source = "fn add(a: int, b: int) { a + b }";
    let result = parse_str(source);

    assert_snapshot!(format!("{:#?}", result.root()));
}
```

---

## Debugging Tips

### Enable Rust Backtraces

```bash
RUST_BACKTRACE=1 cargo test
```

### Print CST

```rust
let result = parse_str(source);
if let Some(tree) = result.tree {
    println!("{:#?}", tree.root());
}
```

### Print Diagnostics

```rust
for diag in result.errors {
    diag.eprint(source);
}
```

### Use rust-analyzer

Install rust-analyzer extension in VS Code for:
- Inline error checking
- Go to definition
- Auto-completion
- Inline documentation

---

## Performance Tips

### Parallel Parsing

```rust
use rayon::prelude::*;

let files = vec!["a.nx", "b.nx", "c.nx"];
let session = ParserSession::new();

let results: Vec<_> = files
    .par_iter()
    .map(|f| session.parse_file(f))
    .collect();
```

### Reuse Sessions

```rust
// Good - reuses parser state
let session = TypeCheckSession::new();
for file in files {
    session.add_file(file.name, file.source);
}
let results = session.check_all();

// Bad - creates new session each time
for file in files {
    let result = check_str(&file.source, &file.name);
}
```

### Profile Performance

```bash
cargo install cargo-flamegraph
cargo flamegraph --bin nx-cli -- check large_file.nx
```

---

## Common Issues

### "tree-sitter grammar not found"

**Problem**: `grammar.js` not compiled

**Solution**:
```bash
cd crates/nx-syntax
cargo clean
cargo build
```

### "Salsa query panicked"

**Problem**: Query dependency cycle

**Solution**: Check query graph for cycles in `db.rs`

### "Snapshot test failed"

**Problem**: Parser output changed

**Solution**:
```bash
cargo insta review
# Review changes, accept if correct
```

### "Type inference timeout"

**Problem**: Infinite loop in type checker

**Solution**: Add recursion limit or cycle detection

---

## Getting Help

1. **Documentation**: Run `cargo doc --open` to view rustdoc
2. **Examples**: Check `examples/` directory
3. **Tests**: Look at existing tests for usage patterns
4. **Plan**: Review `plan.md` for architecture decisions
5. **Data Model**: Check `data-model.md` for entity relationships

---

## Next Steps

### For Implementers

1. Review `plan.md` for detailed architecture
2. Review `data-model.md` for entity definitions
3. Review `contracts/library-api.md` for API design
4. Start implementing `nx-diagnostics` crate
5. Follow implementation order above

### For Users

1. Add library to your `Cargo.toml`
2. Try basic parsing example above
3. Explore API in `contracts/library-api.md`
4. Check examples in `examples/` directory
5. Refer to rustdoc for detailed API docs

---

## Checklist

**Before Starting Implementation**:
- [ ] Rust 1.75+ installed
- [ ] Workspace builds successfully
- [ ] All tests pass
- [ ] Reviewed `plan.md`
- [ ] Reviewed `data-model.md`
- [ ] Reviewed `contracts/library-api.md`

**Before First Commit**:
- [ ] All tests pass (`cargo test --workspace`)
- [ ] No warnings (`cargo clippy --workspace`)
- [ ] Formatted (`cargo fmt --all`)
- [ ] Documentation builds (`cargo doc`)
- [ ] Added tests for new code

---

**Quick Start Status**: ✅ Complete - Ready for implementation and usage
