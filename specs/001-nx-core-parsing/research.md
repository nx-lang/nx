# Research: Core NX Parsing and Validation

**Feature**: 001-nx-core-parsing
**Date**: 2025-10-26

## Purpose

This document consolidates research findings that informed the technical decisions in `plan.md`. All unknowns from the Technical Context have been resolved through analysis of the existing `nx-rust-plan.md` and industry best practices.

---

## Parser Technology Selection

### Research Question
What parser technology should we use for NX language parsing?

### Investigation
Evaluated three categories of parser technologies:
1. **Parser generators**: tree-sitter, LALRPOP, Pest
2. **Parser combinators**: Nom, chumsky
3. **Hand-written**: Custom recursive descent

### Findings

**tree-sitter**:
- ✅ Industry standard (VS Code, Neovim, GitHub)
- ✅ Incremental parsing - only re-parse changed regions
- ✅ Error recovery - continues parsing after errors
- ✅ Query system for syntax highlighting
- ✅ C library with Rust bindings
- ✅ Already partially integrated in `nx-syntax` crate
- ⚠️ Learning curve for grammar.js DSL

**LALRPOP**:
- ✅ Pure Rust, good error messages
- ❌ LR parser - poor error recovery
- ❌ No incremental parsing
- ❌ Not designed for editor integration

**Pest/Nom**:
- ✅ Pure Rust, composable
- ❌ Less mature error recovery
- ❌ No built-in editor integration
- ❌ More manual work for incremental parsing

### Decision
**Use tree-sitter v0.20+** - Industry standard, proven error recovery, editor integration built-in.

---

## CST vs AST Architecture

### Research Question
Should we use Concrete Syntax Tree (CST), Abstract Syntax Tree (AST), or both?

### Investigation
Analyzed three architectural patterns:
1. **CST only**: Keep tree-sitter representation throughout
2. **AST only**: Convert directly to simplified tree
3. **Dual-layer**: CST for tooling, AST for semantics

### Findings

**rust-analyzer pattern** (dual-layer):
- Uses tree-sitter CST via rowan (custom CST library)
- Lowers to typed HIR (High-level IR) for semantic analysis
- CST preserves formatting for tools (formatter, refactoring)
- HIR simplifies type checking and code generation

**SWC pattern** (AST only):
- Parses directly to AST
- Loses formatting information
- Simpler architecture
- Harder to build accurate formatters/refactoring tools

**tree-sitter native pattern**:
- Keep tree-sitter CST, build thin typed wrappers
- Avoid custom CST implementation (cstree/rowan)
- Lower to arena-based AST for semantic phases
- Best of both worlds without cstree complexity

### Decision
**Use tree-sitter CST + typed wrappers + Rust HIR** - Preserves formatting for tooling, simplifies semantics, avoids unproven cstree bridge mentioned in nx-rust-plan.md.

---

## Incremental Computation Framework

### Research Question
How do we achieve fast re-analysis for IDE responsiveness?

### Investigation
Evaluated approaches for incremental computation:
1. **No incremental** - recompute everything on change
2. **Manual caching** - build custom invalidation logic
3. **Salsa** - query-based memoization framework

### Findings

**Salsa**:
- ✅ Powers rust-analyzer (proven at scale)
- ✅ Query-based API - define what to compute, framework handles when
- ✅ Automatic dependency tracking
- ✅ Fine-grained invalidation
- ✅ Supports derived queries (query depends on other queries)
- ⚠️ Learning curve for salsa concepts

**Manual caching**:
- ❌ Complex to implement correctly
- ❌ Easy to introduce cache invalidation bugs
- ❌ Reinventing wheel

**No incremental**:
- ❌ Poor IDE performance
- ❌ Wastes computation re-analyzing unchanged files

### Decision
**Use Salsa v0.16+** - Industry-proven, automatic dependency tracking, essential for future LSP server.

---

## Error Reporting Library

### Research Question
What library should we use for beautiful error diagnostics?

### Investigation
Compared diagnostic libraries:
1. **ariadne** - Modern, beautiful output
2. **codespan-reporting** - Older, widely used
3. **miette** - Alternative modern library
4. **Custom** - Build our own formatter

### Findings

**ariadne v0.4**:
- ✅ Beautiful, modern output with source context
- ✅ ANSI colors and Unicode box-drawing
- ✅ Better API than codespan-reporting
- ✅ Integrates with text-size
- ✅ Actively maintained
- ✅ Used by several Rust compilers

**codespan-reporting**:
- ✅ Mature, widely used
- ❌ Older API, less ergonomic
- ❌ More verbose setup

**miette**:
- ✅ Modern, good API
- ⚠️ More focused on application errors vs compiler errors
- ⚠️ Extra features we don't need (error trait integration)

### Decision
**Use Ariadne v0.4** - Modern API, beautiful output, best fit for compiler-style diagnostics.

---

## Type Inference Strategy

### Research Question
What level of type inference should NX support?

### Investigation
Analyzed type inference approaches:
1. **Explicit only** - All types must be annotated
2. **Local inference** - Infer variables/expressions, require function signatures
3. **Full inference** - Infer everything including function signatures

### Findings

**Local inference** (chosen via spec clarifications):
- ✅ Infer types for local variables from initializers
- ✅ Infer expression types from context
- ✅ Infer function return types from body
- ✅ Require explicit function parameter types
- ✅ Used by: Rust, TypeScript, Kotlin, Swift
- ✅ Balances brevity with clarity

**Explicit only**:
- ✅ Simplest to implement
- ❌ Too verbose for users
- ❌ Poor developer experience

**Full inference** (Haskell-style):
- ❌ Complex type checker
- ❌ Harder to debug type errors
- ❌ Slower type checking
- ❌ Ambiguous error messages

### Decision
**Local type inference with explicit parameters** - Modern language standard, specified in spec clarifications.

---

## Concurrency & Thread Safety

### Research Question
Should the library API be thread-safe? Should it support concurrent parsing?

### Investigation
Evaluated concurrency models:
1. **Single-threaded** - No thread safety
2. **Thread-safe** - `Send + Sync` types
3. **Async/await** - Non-blocking I/O

### Findings

**Thread-safe (Send + Sync)**:
- ✅ Enables parallel file parsing in batch mode
- ✅ Rust ownership prevents data races
- ✅ Salsa supports concurrent queries
- ✅ Required by spec (FR-008a)
- ⚠️ Must be careful with interior mutability

**Single-threaded**:
- ❌ Poor performance for batch operations
- ❌ Violates spec requirement

**Async/await**:
- ⚠️ Not needed for library API (no I/O)
- ⚠️ May add complexity without benefit
- 📌 Consider for future LSP server

### Decision
**Thread-safe with Send + Sync** - Enables parallel parsing, required by spec, natural fit for Rust.

---

## Memory Management for AST Nodes

### Research Question
How should we allocate and manage AST/HIR nodes?

### Investigation
Compared allocation strategies:
1. **Box/Rc per node** - Individual heap allocations
2. **Arena allocation** - Batch allocation with indices
3. **Generational indices** - Versioned arena indices

### Findings

**Arena allocation (la-arena)**:
- ✅ Fast allocation (bump allocator)
- ✅ Fast deallocation (drop entire arena)
- ✅ Stable references via arena indices
- ✅ Used by rust-analyzer successfully
- ✅ Good memory locality
- ✅ Helps meet <100MB target for 10k lines

**Box/Rc per node**:
- ❌ Memory overhead (each allocation has metadata)
- ❌ Fragmentation
- ❌ Slower allocation/deallocation

**Generational indices**:
- ✅ Detects use-after-free bugs
- ⚠️ Extra complexity for our use case
- ⚠️ Arena simpler and sufficient

### Decision
**Use la-arena v0.3+** - Fast, memory-efficient, proven in rust-analyzer.

---

## Testing Strategy

### Research Question
What testing approach should we use for parser and type checker?

### Investigation
Evaluated testing frameworks and patterns:
1. **Unit tests** - Test individual functions
2. **Integration tests** - Test end-to-end flows
3. **Snapshot tests** - Capture and compare output
4. **Property-based tests** - Generate random inputs

### Findings

**Snapshot testing with insta**:
- ✅ Perfect for parser regression tests
- ✅ Captures complex output (AST trees, diagnostics)
- ✅ `cargo insta review` shows diffs
- ✅ Used by rust-analyzer extensively
- ✅ Prevents accidental output changes

**Standard unit/integration tests**:
- ✅ Essential for logic testing
- ✅ Rust's built-in test framework is excellent
- ✅ Fast, deterministic

**Property-based tests** (proptest/quickcheck):
- ⚠️ Good for fuzzing
- ⚠️ Defer to later phase (not critical for MVP)

### Decision
**Use insta for snapshot tests + standard unit/integration tests** - Best of both worlds, catches regressions.

---

## Performance Benchmarking

### Research Question
How do we measure and track performance goals?

### Investigation
Evaluated benchmarking approaches:
1. **Criterion.rs** - Statistical benchmarking
2. **Manual timing** - Simple approach
3. **Built-in Rust bencher** - Unstable feature

### Findings

**Criterion.rs**:
- ✅ Statistical analysis of results
- ✅ Detects performance regressions
- ✅ Beautiful HTML reports
- ✅ Industry standard for Rust

**Manual timing**:
- ❌ No statistical analysis
- ❌ Noisy results
- ❌ Easy to get wrong

### Decision
**Use Criterion.rs** (defer to polish phase) - Proper statistical benchmarking for performance validation.

---

## Summary of Resolved Unknowns

All "NEEDS CLARIFICATION" items from Technical Context have been resolved:

| Item | Resolution |
|------|------------|
| **Language/Version** | Rust 1.75+ |
| **Primary Dependencies** | tree-sitter 0.20+, salsa 0.16+, ariadne 0.4, text-size 1.1+ |
| **Testing** | cargo test, insta for snapshots |
| **Target Platform** | Library (cross-platform: Linux, macOS, Windows) |
| **Performance Goals** | Parsing >10k lines/sec, type checking <2s for 10k lines |
| **Constraints** | Thread-safe, best-effort error recovery, UTF-8 only, local type inference |
| **Scale/Scope** | 4 crates, ~10-15k LOC, full NX grammar support |

---

## References

- [rust-analyzer architecture](https://rust-analyzer.github.io/book/contributing/architecture.html)
- [tree-sitter documentation](https://tree-sitter.github.io/tree-sitter/)
- [Salsa book](https://salsa-rs.github.io/salsa/)
- [Ariadne documentation](https://docs.rs/ariadne/)
- [la-arena documentation](https://docs.rs/la-arena/)
- [insta documentation](https://insta.rs/)
- Existing project document: `nx-rust-plan.md` (primary source)

---

**Research Status**: ✅ Complete - All technical decisions documented and justified.
