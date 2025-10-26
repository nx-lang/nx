<!--
Sync Impact Report:
Version: 1.1.0 (testing requirements relaxed)
Modified Principles:
  - Principle II: "Test-Driven Development" → "Comprehensive Testing" (relaxed TDD strict ordering)
Added Sections: N/A
Removed Sections: N/A
Templates Updated:
  ✅ .specify/templates/plan-template.md (verified Constitution Check section compatible)
  ✅ .specify/templates/spec-template.md (verified requirements align with principles)
  ✅ .specify/templates/tasks-template.md (verified task phases align with testing requirements)
Follow-up TODOs: None
-->

# NX Language Constitution

## Core Principles

### I. Code Quality First

Code quality is non-negotiable and takes precedence over velocity. Every contribution MUST meet the following standards:

- **Explicit over Implicit**: All types, access modifiers, and intentions MUST be explicitly declared. No `var` unless type is immediately obvious from right-hand side (e.g., `new SomeType()`, obvious LINQ queries).
- **Consistent Formatting**:
  - C#: 4-space indentation, Allman-style braces, CRLF on Windows / LF elsewhere
  - Rust: `cargo fmt` standard formatting, 4-space indentation
  - XML/JSON/XAML: 2-space indentation
- **Always Use Braces**: All control flow statements (if, for, while, etc.) MUST use braces, even for single-line bodies.
- **Naming Conventions**:
  - C#: PascalCase for public members, `_camelCase` for private fields, PascalCase for constants
  - Rust: snake_case for functions/variables, PascalCase for types/traits
  - Treat "UI" as a single word (e.g., `UIComponent`, not `UiComponent`)
- **Documentation**:
  - C#: XML doc comments on all public APIs, wrapped at 120 chars, use `<para>` only for multi-paragraph docs
  - Rust: `///` doc comments on all public APIs following rustdoc conventions
- **Nullable Reference Types**: Enabled by default in C# (`<Nullable>enable</Nullable>`)
- **No Warnings**: Code MUST compile with zero warnings. All analyzer warnings MUST be fixed or explicitly suppressed with justification.

**Rationale**: Explicit, consistent code reduces cognitive load, prevents bugs, and enables effective collaboration across the team and with AI agents.

### II. Comprehensive Testing (NON-NEGOTIABLE)

Testing is mandatory for all code contributions:

- **Testing Requirements**:
  - All new code MUST have corresponding tests
  - Tests MAY be written during implementation (tests-first is encouraged but not required)
  - All tests MUST pass before code can be merged
  - Tests MUST be committed alongside implementation code
- **Test Categories**:
  - **Unit Tests**: Test individual functions/methods in isolation
  - **Integration Tests**: Test component interactions and user journeys
  - **Contract Tests**: Verify API contracts and public interfaces remain stable
- **Coverage Expectations**:
  - Critical paths (error handling, security, data integrity) MUST have comprehensive test coverage
  - Tests MUST be deterministic, fast, and independent
  - Bug fixes MUST include regression tests demonstrating the fix
- **Test Organization**:
  - Place tests in dedicated test directories (`tests/unit/`, `tests/integration/`, `tests/contract/`)
  - One test file per source file for unit tests
  - Integration tests organized by user journey or feature
- **Test Quality**:
  - Tests MUST have clear Given-When-Then structure or equivalent clear arrangement
  - Test names MUST clearly describe what is being tested and expected outcome
  - Tests MUST NOT depend on execution order or shared mutable state
  - Flaky tests MUST be fixed immediately or removed

**Rationale**: Comprehensive testing ensures correctness, documents intent, enables confident refactoring, and prevents regressions. Flexibility in when tests are written during development allows for pragmatic workflows while maintaining quality standards.

### III. User Experience Consistency

Every user-facing feature MUST provide a consistent, predictable experience:

- **Cross-Platform Consistency**: Features MUST work identically on Windows, Linux, and macOS unless platform differences are fundamental and documented.
- **Error Messages**:
  - MUST be clear, actionable, and user-friendly
  - MUST use the `nx-diagnostics` crate (Ariadne) for beautiful, helpful error reporting with source context and suggestions
  - MUST include "what went wrong" and "how to fix it" guidance
- **CLI Design**:
  - Follow UNIX philosophy: text in, text out
  - Support both machine-readable (JSON) and human-readable output formats
  - Use stdin/args for input, stdout for success output, stderr for errors
  - Provide `--help` that clearly documents all options with examples
- **Performance Expectations**:
  - Parsing: >10k lines/second for typical NX files
  - LSP responsiveness: <100ms for completion requests, <200ms for diagnostics
  - Interactive commands: <1 second response time for user actions
- **Documentation**:
  - Every feature MUST have user-facing documentation with examples
  - Breaking changes MUST be documented in CHANGELOG with migration guide
  - API documentation MUST include code examples

**Rationale**: Consistent UX reduces learning curve, builds user trust, and ensures NX is approachable for developers of all skill levels.

## Implementation Standards

### Code Organization

- **One Primary Type Per File**: Each file SHOULD contain one main type; nested types are acceptable if tightly coupled
- **Namespace = Folder Structure**: Namespace hierarchy MUST match directory structure
- **Dependency Direction**: Dependencies MUST flow upward in the architecture:
  - `nx-diagnostics` (leaf crate, zero NX dependencies)
  - `nx-syntax` depends on `nx-diagnostics`
  - `nx-hir` depends on `nx-syntax`
  - `nx-types` depends on `nx-hir`
  - `nx-cli` and `nx-lsp` depend on all layers
- **Access Modifiers**: Always explicit; prefer most restrictive appropriate level (private by default)

### Refactoring Policy

- **No Backward Compatibility Burden**: We are pre-1.0; prefer simpler new code over complex backward-compatible code
- **Refactor Fearlessly**: With comprehensive tests, aggressive refactoring is encouraged to improve design
- **Update Tests**: When modifying code, update related tests to reflect new behavior
- **Follow Existing Patterns**: Maintain architectural consistency unless proposing a deliberate change

### Performance & Scalability

- **Benchmarking**: Performance-critical code MUST have benchmarks
- **Profiling**: Before optimizing, profile to identify actual bottlenecks
- **Lazy Evaluation**: Use Salsa's incremental computation for expensive operations
- **Zero-Copy Where Possible**: Prefer string slices (`&str`) and references over clones

## Quality Gates

Every contribution MUST pass these gates before merging:

1. **Build**: `cargo build --workspace` succeeds with zero warnings
2. **Format**: `cargo fmt --all --check` passes
3. **Lint**: `cargo clippy --workspace` passes with zero warnings
4. **Tests**: `cargo test --workspace` passes with 100% success rate
5. **Documentation**: `cargo doc --workspace` builds without errors
6. **Manual Review**: Code reviewed by at least one maintainer for:
   - Adherence to constitution principles
   - Test coverage adequacy
   - Documentation completeness
   - Architectural fit

### Complexity Justification

If a feature violates simplicity principles (e.g., adds significant complexity), it MUST include written justification in the PR:

- **Why is this complexity necessary?**
- **What simpler alternatives were considered and why were they rejected?**
- **What is the long-term maintenance cost?**

Unjustified complexity will be rejected.

## Governance

### Amendment Process

1. Proposed amendments MUST be documented in a PR with rationale
2. Amendments require maintainer approval
3. Version bump follows semantic versioning:
   - **MAJOR**: Breaking changes to core principles or governance removal
   - **MINOR**: New principles added or materially expanded guidance
   - **PATCH**: Clarifications, wording improvements, non-semantic fixes
4. When constitution changes, dependent templates MUST be updated for consistency

### Compliance Review

- All PRs MUST be checked against this constitution
- Maintainers may request constitution compliance before technical review
- Repeated violations may result in contributor education or PR rejection
- Constitution takes precedence over convenience

### Living Document

This constitution is a living document. As the project evolves, principles may be refined, but the core commitment to quality, testing, and user experience remains immutable.

**Version**: 1.1.0 | **Ratified**: 2025-10-25 | **Last Amended**: 2025-10-25
