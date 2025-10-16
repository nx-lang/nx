# NX Language Rust Implementation Plan

## Executive Summary

This document outlines the complete architecture and implementation plan for the NX language using Rust as the core implementation language, with FFI bindings for cross-platform access.

**Key Decisions:**
1. **Rust + tree-sitter + FFI** for core implementation
2. **CST (tree-sitter/cstree) + AST (HIR)** dual-layer approach
3. **Pure Rust LSP server** (with TypeScript/C# wrapper as fallback if needed)
4. **Separate crates** in flat workspace (not combined into nx-core)
5. **`nx-hir` naming** for AST + semantic layer
6. **UniFFI + napi-rs** for cross-language FFI bindings

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Repository Structure](#repository-structure)
3. [Rust Crate Organization](#rust-crate-organization)
4. [Technology Stack](#technology-stack)
5. [FFI Public API](#ffi-public-api)
6. [Implementation Phases](#implementation-phases)
7. [Test Organization](#test-organization)
8. [Build & Development Workflow](#build--development-workflow)
9. [Migration from Current Structure](#migration-from-current-structure)

---

## Architecture Overview

### Two-Layer Strategy (Rust-Analyzer Pattern)

```
Source Code
    ↓
[tree-sitter parser] → CST (Green Tree) ← Editor integration, syntax highlighting
    ↓
[CST → AST converter] → AST (HIR) ← Type checker, semantic analysis
    ↓
[Rust LSP Server] ← IDE features (completions, diagnostics, hover)
    ↓
[Code generation / Interpreter] (Future)
```

### Why CST + AST (Not Either/Or)?

**CST (Concrete Syntax Tree):**
- Preserves everything: whitespace, comments, formatting, errors
- Perfect for tooling: formatters, LSP (rename, refactor), error recovery
- Incremental re-parsing via tree-sitter
- Error resilience: represents incomplete/malformed code

**AST (Abstract Syntax Tree):**
- Type checking: simplified structure for semantic analysis
- Code generation: cleaner representation for transpilation/compilation
- Optimization: easier to transform and optimize

---

## Repository Structure

### Complete Directory Layout

```
nx/                                 # Repository root
├── .github/                        # GitHub workflows, actions
│   ├── workflows/
│   │   ├── ci.yml                  # Rust: build, test, clippy, fmt
│   │   ├── deploy-docs.yml         # Deploy documentation site
│   │   ├── publish-nuget.yml       # Publish .NET packages
│   │   ├── publish-npm.yml         # Publish npm packages
│   │   └── release.yml             # Create releases, build binaries
│   └── actions/                    # Custom GitHub actions
│
├── .vscode/                        # VS Code workspace settings
│   ├── settings.json
│   ├── launch.json                 # Debug configs for Rust/TypeScript
│   └── tasks.json
│
├── .claude/                        # Claude Code configuration
│
├── crates/                         # Rust workspace (core implementation)
│   ├── nx-diagnostics/             # Error formatting (ariadne)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── diagnostic.rs
│   │   │   └── render.rs
│   │   └── tests/
│   │       └── render_tests.rs
│   │
│   ├── nx-syntax/                  # CST + tree-sitter + parser
│   │   ├── Cargo.toml
│   │   ├── build.rs                # Compiles tree-sitter grammar
│   │   ├── grammar.js              # tree-sitter grammar (shared with editors)
│   │   ├── src/
│   │   │   ├── lib.rs              # Unit tests: #[cfg(test)] mod tests
│   │   │   ├── syntax_kind.rs      # Token/node types
│   │   │   ├── syntax_node.rs      # CST nodes (cstree wrapper)
│   │   │   ├── ast.rs              # AST-like API over CST
│   │   │   └── validation.rs       # Post-parse validation
│   │   ├── queries/                # tree-sitter queries (for highlighting)
│   │   │   ├── highlights.scm
│   │   │   ├── locals.scm
│   │   │   └── injections.scm
│   │   └── tests/                  # Integration tests
│   │       ├── common/
│   │       │   └── mod.rs          # Shared test utilities
│   │       ├── fixtures/
│   │       │   ├── valid/
│   │       │   │   ├── hello.nx
│   │       │   │   └── functions.nx
│   │       │   └── invalid/
│   │       │       └── syntax_error.nx
│   │       ├── snapshots/          # insta snapshots (auto-gen)
│   │       └── parser_tests.rs
│   │
│   ├── nx-hir/                     # AST + semantic model
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs              # Unit tests inline
│   │   │   ├── ast/                # AST definitions
│   │   │   │   ├── mod.rs
│   │   │   │   ├── expr.rs
│   │   │   │   ├── stmt.rs
│   │   │   │   └── types.rs
│   │   │   ├── lower.rs            # CST → AST lowering
│   │   │   └── db.rs               # Salsa queries (Phase 5+)
│   │   └── tests/
│   │       └── lowering_tests.rs
│   │
│   ├── nx-types/                   # Type system
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs              # Unit tests inline
│   │   │   ├── ty.rs               # Type representation
│   │   │   ├── infer.rs            # Type inference
│   │   │   ├── unify.rs            # Unification
│   │   │   └── primitives.rs
│   │   └── tests/
│   │       ├── fixtures/
│   │       │   └── type_examples.nx
│   │       └── type_checker_tests.rs
│   │
│   ├── nx-lsp/                     # LSP server (Phase 5)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── handlers/
│   │       │   ├── mod.rs
│   │       │   ├── completion.rs
│   │       │   ├── hover.rs
│   │       │   └── diagnostics.rs
│   │       ├── analysis.rs
│   │       └── document.rs
│   │
│   └── nx-cli/                     # CLI tools
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── parse.rs
│           ├── check.rs
│           └── format.rs
│
├── bindings/                       # FFI bindings (Phase 4)
│   ├── nx-ffi/                     # Core FFI layer
│   │   ├── Cargo.toml
│   │   ├── cbindgen.toml
│   │   ├── uniffi.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── nx.udl              # UniFFI interface definition
│   │
│   ├── dotnet/                     # .NET bindings
│   │   ├── NX.Native/              # C# P/Invoke wrapper
│   │   │   ├── NX.Native.csproj
│   │   │   ├── Parser.cs
│   │   │   ├── TypeChecker.cs
│   │   │   └── Interop/
│   │   │       └── NativeMethods.cs
│   │   ├── NX.Native.Tests/        # C# unit tests
│   │   │   └── ParserTests.cs
│   │   └── build.sh                # Build native lib + NuGet
│   │
│   ├── node/                       # Node.js bindings (napi-rs)
│   │   ├── package.json
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   └── lib.rs              # napi bindings
│   │   ├── index.d.ts              # TypeScript definitions (auto-gen)
│   │   └── __test__/
│   │       └── parser.spec.ts
│   │
│   └── wasm/                       # Browser WASM
│       ├── Cargo.toml
│       ├── package.json
│       ├── src/
│       │   └── lib.rs              # wasm-bindgen bindings
│       └── examples/
│           └── playground.html     # WASM playground example
│
├── editors/                        # Editor integrations
│   ├── vscode/                     # VS Code extension
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   ├── src/
│   │   │   ├── extension.ts        # Extension entry point
│   │   │   ├── client.ts           # LSP client
│   │   │   └── syntax.ts           # tree-sitter integration
│   │   ├── syntaxes/
│   │   │   └── nx.tmLanguage.json  # TextMate grammar (fallback/legacy)
│   │   ├── snippets/
│   │   │   └── nx.json             # Code snippets
│   │   ├── samples/                # Example .nx files
│   │   │   ├── tally-survey.nx
│   │   │   └── survey.nx
│   │   ├── test/
│   │   │   └── grammar/
│   │   │       └── basic.test.ts
│   │   └── tree-sitter-nx.wasm     # Compiled tree-sitter grammar
│   │
│   ├── emacs/                      # (Future) Emacs support
│   │   └── nx-mode.el
│   │
│   └── vim/                        # (Future) Vim/Neovim support
│       └── syntax/nx.vim
│
├── docs/                           # Documentation site (Astro)
│   ├── package.json
│   ├── astro.config.mjs
│   ├── tsconfig.json
│   ├── src/
│   │   ├── content/
│   │   │   └── docs/
│   │   │       ├── overview/
│   │   │       │   ├── what-is-nx.md
│   │   │       │   ├── design-goals.md
│   │   │       │   └── comparison.md
│   │   │       ├── reference/
│   │   │       │   ├── syntax/
│   │   │       │   │   ├── modules.md
│   │   │       │   │   ├── functions.md
│   │   │       │   │   ├── expressions.md
│   │   │       │   │   ├── types.md
│   │   │       │   │   └── elements.md
│   │   │       │   └── api/        # FFI API docs
│   │   │       │       ├── dotnet.md
│   │   │       │       ├── nodejs.md
│   │   │       │       └── wasm.md
│   │   │       ├── guides/
│   │   │       │   ├── getting-started.md
│   │   │       │   ├── using-in-dotnet.md
│   │   │       │   └── using-in-nodejs.md
│   │   │       └── internals/
│   │   │           ├── architecture.md
│   │   │           ├── grammar.md
│   │   │           └── contributing.md
│   │   ├── components/
│   │   └── layouts/
│   └── public/
│       └── playground/             # Interactive NX playground
│           └── index.html          # Uses WASM bindings
│
├── src/                            # (Existing) Legacy/additional C# code
│   └── NX/                         # C# library (may deprecate or use for runtime)
│       ├── NX.csproj
│       └── Calculator.cs
│
├── test/                           # (Existing) C# tests
│   └── NX.Tests/
│       ├── NX.Tests.csproj
│       └── CalculatorTests.cs
│
├── examples/                       # Example NX projects
│   ├── hello-world/
│   │   └── main.nx
│   ├── ui-components/
│   │   ├── button.nx
│   │   └── card.nx
│   └── config-dsl/
│       └── server-config.nx
│
├── benchmarks/                     # Performance benchmarks
│   ├── Cargo.toml
│   └── benches/
│       ├── parser_bench.rs
│       └── type_checker_bench.rs
│
├── tools/                          # (Existing) Build tooling
│
├── scripts/                        # Build/maintenance scripts
│   ├── build-all.sh                # Build Rust + .NET + npm packages
│   ├── test-all.sh                 # Run all tests
│   ├── generate-grammar.sh         # Generate tree-sitter parser
│   └── publish-packages.sh         # Publish to crates.io, NuGet, npm
│
├── .gitignore
├── .editorconfig
├── CLAUDE.md                       # (Existing) Claude instructions
├── AGENTS.md                       # (Existing) AI agent guidelines
├── CONTRIBUTING.md
├── LICENSE
├── README.md                       # Main readme (language overview, quick start)
│
├── Cargo.toml                      # Rust workspace manifest (virtual)
├── Cargo.lock                      # Shared Rust lockfile
├── rust-toolchain.toml             # Rust version pinning
│
├── NX.sln                          # (Existing) .NET solution (for C# code)
├── Directory.Build.props           # (Existing) .NET build config
├── Directory.Packages.props        # (Existing) .NET packages
├── global.json                     # (Existing) .NET SDK version
│
├── package.json                    # Root package.json (workspace for docs/editors)
├── pnpm-workspace.yaml             # pnpm workspace config (if using pnpm)
│
├── nx-grammar.md                   # (Existing) Human-readable grammar
├── nx-grammar-spec.md              # (Existing) Machine-readable grammar spec
├── nx-planning.md                  # (Existing) Original implementation planning
├── nx-planning-future.md           # (Existing) Future features
└── nx-rust-plan.md                 # This document
```

---

## Rust Crate Organization

### Initial 5 Crates (Phase 1-3)

```
crates/
├── nx-diagnostics/     # ~500 LOC - Error formatting (leaf, no deps)
├── nx-syntax/          # ~3-5k LOC - CST + parser + tree-sitter
├── nx-hir/             # ~2-4k LOC - AST + lowering
├── nx-types/           # ~3-5k LOC - Type system
└── nx-cli/             # ~500 LOC - CLI entry point
```

### Dependency Graph

```
nx-diagnostics (leaf crate, zero deps)
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

### Rationale for Separate Crates

**Why NOT combine into `nx-core`:**

1. ✅ **Parallel compilation** - Crates compile independently
2. ✅ **Clear boundaries** - Each crate has focused responsibility
3. ✅ **Reusability** - Other tools can use just diagnostics or syntax
4. ✅ **Faster incremental builds** - Only recompile what changed
5. ✅ **Testing isolation** - Test each layer independently
6. ✅ **Follows rust-analyzer pattern** - Industry best practice

**Precedent:**
- rust-analyzer: 32+ separate crates
- SWC: `swc_ecma_parser`, `swc_ecma_ast`, `swc_ecma_utils` all separate

---

## Technology Stack

### Core Parsing & CST

1. **`tree-sitter`** (v0.20+)
   - Parser generator and incremental parsing
   - Used for: Grammar definition, syntax highlighting, fast re-parsing

2. **`cstree`** (v0.12+)
   - Lossless syntax tree (improved fork of rowan)
   - Features: Persistent trees, thread-safe, string interning
   - Used for: CST representation, preserving formatting

3. **`text-size`** (v1.1+)
   - Text range types for source positions
   - Used for: Source locations, LSP range conversions

### Type System & Semantics

4. **`salsa`** (v0.16+)
   - Incremental computation framework (Phase 5+)
   - Used for: Query-based caching, on-demand computation, IDE responsiveness

### LSP Implementation (Pure Rust)

5. **`tower-lsp`** (v0.20+)
   - Async LSP server framework
   - Used for: LSP protocol implementation

6. **`lsp-types`** (v0.95+)
   - LSP protocol type definitions
   - Used with: tower-lsp

7. **`tokio`** (v1+)
   - Async runtime
   - Used by: tower-lsp for async operations

8. **`dashmap`** (v5.5+)
   - Concurrent hashmap
   - Used for: File cache in LSP server

### FFI Bindings

9. **`uniffi-rs`** (v0.25+)
   - Multi-language bindings generator (.NET, Python, Swift, Kotlin)
   - Auto-generates from `.udl` interface definition

10. **`napi-rs`** (v2.14+)
    - Node.js N-API bindings with TypeScript type generation
    - Used for: High-quality Node.js/npm package

11. **`wasm-bindgen`** (v0.2+)
    - WebAssembly bindings for browsers
    - Used with: wasm-pack for npm WASM package

### Error Reporting

12. **`ariadne`**
    - Beautiful diagnostic messages with source snippets
    - Modern API, better than codespan-reporting

### Utilities

13. **`insta`**
    - Snapshot testing for parser/CST regression tests
    - Essential for parser testing (rust-analyzer uses extensively)

14. **`smol_str`** (v0.2+)
    - Efficient small string storage

15. **`la-arena`** (v0.3+)
    - Indexed arena for AST nodes

16. **`rustc-hash`** (v1.1+)
    - Fast hashmaps

---

## FFI Public API

### Design Principles

1. **Object-Based API** - Opaque handles rather than raw data structures
2. **Coarse-Grained Operations** - Parse entire files, not individual tokens
3. **Safe Error Handling** - All fallible operations return `Result<T, Error>`
4. **Zero-Copy Where Possible** - Use string slices for input

### Three Layers of Abstraction

**High-level (Convenience Functions):**
```rust
parse_string(source) -> ParseResult
check_string(source) -> TypeCheckResult
format_string(source, options) -> FormatResult
```

**Mid-level (Object-Based):**
```rust
Parser::parse(source) -> SyntaxTree
TypeChecker::check(module) -> TypeCheckResult
```

**Low-level (CST Traversal):**
```rust
SyntaxTree::root() -> SyntaxNode
SyntaxNode::children() -> Vec<SyntaxNode>
```

### Core API (UniFFI IDL)

```idl
// Core parser interface
interface Parser {
  constructor();
  [Throws=NxError]
  SyntaxTree parse([ByRef] string source, string? file_path);
};

// Syntax tree (CST representation)
interface SyntaxTree {
  SyntaxNode root();
  string source_text();
  string to_json();
  sequence<Diagnostic> syntax_errors();
  [Throws=NxError]
  Module to_ast();
};

// Type checker interface
interface TypeChecker {
  constructor();
  [Throws=NxError]
  TypeCheckResult check(Module module);
};

// Formatter interface
interface Formatter {
  constructor();
  [Throws=NxError]
  string format([ByRef] string source, FormatOptions? options);
};

// Diagnostic (error or warning)
dictionary Diagnostic {
  DiagnosticSeverity severity;
  string message;
  TextRange range;
  string? code;
  sequence<DiagnosticRelatedInfo> related_info;
  sequence<CodeAction> suggested_fixes;
};
```

### Usage Examples

**C# (.NET):**
```csharp
using NX.Native;

var parser = new Parser();
var tree = parser.Parse(source, "greeting.nx");

foreach (var error in tree.SyntaxErrors())
{
    Console.WriteLine($"{error.Severity}: {error.Message}");
}

var module = tree.ToAst();
var checker = new TypeChecker();
var result = checker.Check(module);
```

**TypeScript/Node.js:**
```typescript
import { Parser, TypeChecker } from '@nx-lang/parser';

const parser = new Parser();
const tree = parser.parse(source, 'greeting.nx');

for (const error of tree.syntaxErrors()) {
  console.log(`${error.severity}: ${error.message}`);
}

const module = tree.toAst();
const checker = new TypeChecker();
const result = checker.check(module);
```

**Browser/WASM:**
```typescript
import init, { parse_string, check_string } from '@nx-lang/wasm';

await init();

const parseResult = parse_string(source);
const checkResult = check_string(source);
```

---

## Implementation Phases

### Phase 1: tree-sitter Grammar + CST (3-4 weeks)

**Goal:** Parse NX source into a lossless CST

**Tasks:**
1. Port NX grammar from `nx-grammar-spec.md` to tree-sitter `grammar.js`
2. Generate parser and test against existing `.nx` samples
3. Create `cstree` wrapper for CST representation
4. Write tree-sitter queries for syntax highlighting
5. Integrate into VS Code extension (replace TextMate grammar)

**Deliverables:**
- `nx-syntax` crate with tree-sitter grammar
- VS Code extension with tree-sitter highlighting
- CST API with typed node accessors
- Test suite: parse all sample `.nx` files without errors

**Dependencies:**
```toml
# crates/nx-syntax/Cargo.toml
[dependencies]
tree-sitter = "0.20"
cstree = "0.12"
text-size = "1.1"

[build-dependencies]
cc = "1.0"  # Compile tree-sitter C code
```

---

### Phase 2: AST Layer + Validation (2-3 weeks)

**Goal:** Convert CST → AST, validate semantic rules

**Tasks:**
1. Define AST types in `nx-hir` crate (based on `nx-grammar-spec.md`)
2. Implement CST → AST lowering with error recovery
3. Add post-parse validation (matching element tags, unique properties)
4. Create diagnostic infrastructure with `ariadne`

**Deliverables:**
- `nx-hir` crate with complete AST definitions
- CST → AST converter with error recovery
- Validation error reporting with source snippets
- CLI tool: `nx parse <file>` (print AST as JSON)

**Dependencies:**
```toml
# crates/nx-hir/Cargo.toml
[dependencies]
nx-syntax = { path = "../nx-syntax" }
nx-diagnostics = { path = "../nx-diagnostics" }
smol_str = "0.2"
la-arena = "0.3"

# crates/nx-diagnostics/Cargo.toml
[dependencies]
ariadne = "0.4"
text-size = "1.1"
```

---

### Phase 3: Type System (4-5 weeks)

**Goal:** Type checking and inference

**Tasks:**
1. Implement type representation in `nx-types` crate
   - Primitives: string, int, long, float, double, boolean, void, object
   - Sequences: `T[]`
   - Nullable: `T?`
   - Functions: `(T1, T2) => T3`
   - User-defined types and type aliases
2. Build type checker for expressions, elements, functions
3. Implement type inference
4. Add nullable type support
5. Generate helpful type errors with suggestions

**Deliverables:**
- `nx-types` crate with complete type system
- Type checker integrated into CLI
- CLI tool: `nx check <file>` (type check + formatted errors)
- Comprehensive type inference tests

**Dependencies:**
```toml
# crates/nx-types/Cargo.toml
[dependencies]
nx-hir = { path = "../nx-hir" }
nx-diagnostics = { path = "../nx-diagnostics" }
rustc-hash = "1.1"
```

---

### Phase 4: FFI Bindings (3-4 weeks, can parallelize)

**Goal:** Expose parser/type checker to .NET and Node.js as a library

**Use Cases:**
- Embedding NX parser in C# applications
- Node.js build tools that need to parse NX
- Web playgrounds (WASM)

**Approach:**
- UniFFI for .NET/multi-language bindings
- napi-rs for high-quality Node.js/TypeScript support
- wasm-bindgen for browser WASM

**Tasks:**
1. Design FFI-safe API (C-compatible types, coarse-grained operations)
2. Implement `nx-ffi` crate with C exports
3. Generate .NET bindings via UniFFI
4. Create `napi-rs` bindings for Node.js
5. Build WASM package with `wasm-bindgen`
6. Package as NuGet and npm

**Deliverables:**
- `NX.Native` NuGet package (.NET)
- `@nx-lang/parser` npm package (Node.js)
- `@nx-lang/wasm` npm package (Browser)
- API documentation and examples

**Dependencies:**
```toml
# bindings/nx-ffi/Cargo.toml
[dependencies]
nx-syntax = { path = "../../crates/nx-syntax" }
nx-hir = { path = "../../crates/nx-hir" }
nx-types = { path = "../../crates/nx-types" }
uniffi = "0.25"

# bindings/node/Cargo.toml
[dependencies]
nx-ffi = { path = "../nx-ffi" }
napi = { version = "2.14", features = ["napi8"] }
napi-derive = "2.14"

# bindings/wasm/Cargo.toml
[dependencies]
nx-ffi = { path = "../nx-ffi" }
wasm-bindgen = "0.2"
```

---

### Phase 5: LSP Server (Pure Rust) (3-4 weeks)

**Goal:** Full IDE support with optimal performance

**Primary Approach: Pure Rust LSP**

**Rationale:**
- ✅ Zero FFI overhead (1-5ms latency vs 10-50ms with wrapper)
- ✅ Best performance (rust-analyzer: 2-5ms completion latency)
- ✅ Seamless incremental computation with Salsa
- ✅ Memory efficiency (shared data structures, no serialization)
- ✅ Industry precedent (rust-analyzer, gopls, clangd)

**Tasks:**
1. Implement LSP server using `tower-lsp` (async, tokio-based)
2. Add document state management (in-memory file cache)
3. Implement core LSP features:
   - Diagnostics (syntax + type errors)
   - Hover (type info and documentation)
   - Completions (context-aware, <10ms target)
   - Go to definition
   - Find references
   - Rename refactoring
   - Formatting
4. Add incremental computation with `salsa`
5. Integrate into VS Code extension (LSP client in TypeScript)
6. Performance optimization and benchmarking

**Deliverables:**
- `nx-lsp` binary (standalone LSP server)
- VS Code extension with LSP client
- Full IntelliSense support
- Performance target: <10ms completions, <100ms for other operations

**Fallback Evaluation Point:**
- ✅ If Rust LSP works well → Continue with pure Rust
- ⚠️ If blocked by Rust complexity → Evaluate TypeScript/C# wrapper
  - Benchmark FFI overhead with concrete data
  - Keep hot path (completions, parsing) in Rust
  - Only wrap cold path (project management)

**Dependencies:**
```toml
# crates/nx-lsp/Cargo.toml
[dependencies]
tower-lsp = "0.20"
lsp-types = "0.95"
tokio = { version = "1", features = ["full"] }
salsa = "0.16"
dashmap = "5.5"
nx-syntax = { path = "../nx-syntax" }
nx-hir = { path = "../nx-hir" }
nx-types = { path = "../nx-types" }
nx-diagnostics = { path = "../nx-diagnostics" }
```

---

### Phase 6: Formatter & Polish (2 weeks)

**Goal:** Production-ready tooling

**Tasks:**
1. Implement code formatter (use CST to preserve structure/comments)
2. Performance benchmarking (LSP latency, parser throughput)
3. Comprehensive documentation (rustdoc, user guides)
4. Publish to crates.io
5. CI/CD for multi-platform builds

**Deliverables:**
- `nx format` command
- Performance benchmarks vs baseline
- Published crates and packages
- Complete documentation

---

## Test Organization

### Rust Test Conventions

Rust has two distinct types of tests with different locations:

#### 1. Unit Tests → In `src/` alongside code

**Location:** Same file as code, in `#[cfg(test)] mod tests`

```rust
// src/parser.rs

pub fn parse(source: &str) -> Result<Tree, Error> {
    // implementation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let result = parse("let x = 1");
        assert!(result.is_ok());
    }
}
```

**Key Points:**
- ✅ Tests private functions (not exposed in public API)
- ✅ Small, focused tests for individual modules
- ✅ Compiled only when running `cargo test`

#### 2. Integration Tests → In `tests/` directory

**Location:** `tests/` at crate root (sibling to `src/`)

```
crates/nx-syntax/
├── Cargo.toml
├── src/
│   └── lib.rs
└── tests/
    ├── parser_tests.rs     # Separate crate, tests public API
    └── validation_tests.rs
```

**Key Points:**
- ✅ Each file is compiled as a separate crate
- ✅ Can only test public API
- ✅ Tests multiple modules working together

#### 3. Test Utilities → `tests/common/` subdirectory

```
tests/
├── common/
│   └── mod.rs          # Shared test utilities
├── parser_tests.rs
└── validation_tests.rs
```

**Why `common/` works:** Cargo ignores subdirectories in `tests/`

#### 4. Snapshot Tests → `tests/snapshots/` (insta crate)

```rust
use insta::assert_snapshot;

#[test]
fn parse_hello_world() {
    let source = "let <Hello/> = <div>Hello!</div>";
    let tree = parse(source).unwrap();

    assert_snapshot!(tree.debug_string());
}
```

**Benefits:**
- ✅ Automatically captures complex output (AST trees)
- ✅ `cargo insta review` shows diffs
- ✅ Perfect for parser/compiler testing

### Test Organization Summary

| Test Type | Location | Tests | Can Access |
|-----------|----------|-------|------------|
| **Unit tests** | `src/*.rs` in `#[cfg(test)]` modules | Individual functions | Private functions |
| **Integration tests** | `tests/*.rs` | Public API, end-to-end | Only public exports |
| **Test utilities** | `tests/common/mod.rs` | Shared helpers | N/A |
| **Snapshot tests** | `tests/*.rs` with `insta` | Complex output | Public API |
| **Fixtures** | `tests/fixtures/` | Sample input files | Loaded by tests |

---

## Build & Development Workflow

### Rust Development

```bash
# Build all Rust crates
cargo build --workspace

# Test all crates
cargo test --workspace

# Run specific crate
cargo run -p nx-cli -- parse examples/hello-world/main.nx

# Lint and format
cargo clippy --workspace
cargo fmt --workspace

# Run tests for specific crate
cargo test -p nx-syntax

# Review snapshot changes
cargo insta review
```

### .NET Development

```bash
# Build C# projects (existing)
dotnet build NX.sln

# Run C# tests
dotnet test
```

### Node.js/Documentation

```bash
# Install all workspace deps
pnpm install

# Build VS Code extension
cd editors/vscode && pnpm build

# Run docs dev server
cd docs && pnpm dev
```

### All-in-One

```bash
# Build everything
./scripts/build-all.sh

# Run all tests (Rust + .NET + TypeScript)
./scripts/test-all.sh
```

---

## Migration from Current Structure

### What Stays

```
✅ docs/                   # Already good location
✅ .github/                # GitHub workflows
✅ src/NX/                 # Existing C# code
✅ test/NX.Tests/          # Existing C# tests
✅ NX.sln                  # .NET solution
✅ nx-grammar*.md          # Language specs at root
✅ README.md, LICENSE, etc.
```

### What to Create

```
📦 Create new:
  crates/                  # NEW: Rust workspace
  bindings/                # NEW: FFI bindings
  editors/                 # NEW: Editor integrations
  examples/                # NEW: Example projects
  benchmarks/              # NEW: Performance tests
  scripts/                 # NEW: Build scripts
```

### What to Migrate

```
🔀 Migrate:
  src/vscode/ → editors/vscode/
    - Move entire VS Code extension
    - Update package.json paths
    - Update .vscodeignore
```

### What to Update/Create

```
📝 Update:
  README.md               # Add Rust/FFI sections
  CONTRIBUTING.md         # Add Rust development guide
  .gitignore              # Add Rust/Cargo patterns

📝 Create:
  Cargo.toml              # Rust workspace manifest
  rust-toolchain.toml     # Rust version (1.75)
  pnpm-workspace.yaml     # npm workspace config
```

---

## Key Architectural Decisions Summary

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Parser** | tree-sitter | Industry standard, incremental, editor integration |
| **CST Library** | `cstree` | Lossless, persistent, thread-safe (better than rowan) |
| **AST Strategy** | CST + separate AST | CST for tooling, AST for semantics (rust-analyzer pattern) |
| **LSP Implementation** | **Pure Rust** | Zero overhead, best performance, seamless Salsa integration |
| **LSP Fallback** | TypeScript wrapper (if needed) | Only if Rust proves blocking; evaluate at end of Phase 5 |
| **FFI Strategy** | `uniffi-rs` + `napi-rs` | uniffi for .NET/multi-lang, napi for Node.js quality |
| **Crate Organization** | Separate crates (5-10) | Parallel compilation, clear boundaries, rust-analyzer pattern |
| **HIR Naming** | `nx-hir` | Matches rust-analyzer, signals semantic model not just syntax |
| **Incremental** | `salsa` (Phase 5) | Essential for IDE performance, proven in rust-analyzer |
| **Error Display** | `ariadne` | Modern, beautiful diagnostics |
| **Testing** | `insta` snapshots | Catch regressions in parser output |

---

## Root Configuration Files

### `Cargo.toml` (Virtual Manifest)

```toml
[workspace]
resolver = "2"
members = [
    "crates/nx-diagnostics",
    "crates/nx-syntax",
    "crates/nx-hir",
    "crates/nx-types",
    "crates/nx-cli",
    # Phase 4+
    # "crates/nx-lsp",
    # "bindings/nx-ffi",
    # "bindings/node",
    # "bindings/wasm",
]

[workspace.package]
version = "0.1.0"
authors = ["Your Name <you@example.com>"]
edition = "2021"
rust-version = "1.75"
license = "MIT OR Apache-2.0"
repository = "https://github.com/yourusername/nx"
homepage = "https://nx-lang.dev"

[workspace.dependencies]
# Shared dependencies
ariadne = "0.4"
cstree = "0.12"
text-size = "1.1"
tree-sitter = "0.20"
smol_str = "0.2"
la-arena = "0.3"
rustc-hash = "1.1"

# Testing
insta = "1.34"

[profile.release]
lto = "thin"
codegen-units = 1
```

### `package.json` (npm Workspace)

```json
{
  "name": "nx-workspace",
  "private": true,
  "workspaces": [
    "docs",
    "editors/vscode"
  ],
  "scripts": {
    "build": "pnpm -r build",
    "dev": "pnpm --filter docs dev",
    "test": "pnpm -r test"
  },
  "devDependencies": {
    "typescript": "^5.3.0"
  },
  "engines": {
    "node": ">=18",
    "pnpm": ">=8"
  }
}
```

### `rust-toolchain.toml`

```toml
[toolchain]
channel = "1.75"
components = ["rustfmt", "clippy"]
```

---

## Success Criteria

1. ✅ **Parse all sample NX files** in `src/vscode/samples/` without errors
2. ✅ **VS Code syntax highlighting** faster than current TextMate grammar
3. ✅ **Type checking** produces helpful, actionable error messages
4. ✅ **LSP completions** <10ms latency (measured in VS Code)
5. ✅ **LSP diagnostics** <100ms latency for typical files (<500 LOC)
6. ✅ **FFI bindings** usable from C#/Node.js with <10 lines of code
7. ✅ **Test coverage** >85% for parser, type checker, LSP handlers
8. ✅ **Documentation** complete for all public APIs

---

## Timeline

**Estimated Total:** 17-22 weeks (4-5 months) for Phases 1-6

- Phase 1: tree-sitter Grammar + CST (3-4 weeks)
- Phase 2: AST Layer + Validation (2-3 weeks)
- Phase 3: Type System (4-5 weeks)
- Phase 4: FFI Bindings (3-4 weeks, can parallelize)
- Phase 5: LSP Server (3-4 weeks)
- Phase 6: Formatter & Polish (2 weeks)

---

## References

- [rust-analyzer architecture](https://rust-analyzer.github.io/book/contributing/architecture.html)
- [SWC project structure](https://github.com/swc-project/swc)
- [tree-sitter documentation](https://tree-sitter.github.io/tree-sitter/)
- [UniFFI user guide](https://mozilla.github.io/uniffi-rs/)
- [napi-rs documentation](https://napi.rs/)
- [tower-lsp documentation](https://docs.rs/tower-lsp/)
- [The Rust Programming Language - Test Organization](https://doc.rust-lang.org/book/ch11-03-test-organization.html)

---

## Appendix: Why Rust?

**Performance:**
- Native code performance (no GC pauses)
- Critical for LSP responsiveness (<10ms completions)
- rust-analyzer benchmarks: 2-5ms completion latency

**Cross-Platform:**
- Compiles to native code on all platforms
- WASM support for web (no runtime required)
- Works on embedded, mobile, desktop

**Ecosystem:**
- tree-sitter: industry standard for modern editors
- tower-lsp: production-ready LSP framework
- salsa: proven incremental computation (rust-analyzer)
- Rich FFI options: uniffi, napi-rs, wasm-bindgen

**Industry Precedent:**
- rust-analyzer (Rust LSP) - gold standard
- SWC (JavaScript/TypeScript compiler)
- Deno (TypeScript runtime)
- Zed editor (written in Rust)

---

*This plan represents the complete architectural vision for the NX language implementation in Rust. It balances performance, maintainability, and cross-platform support while following industry best practices from rust-analyzer, SWC, and other successful Rust language projects.*
