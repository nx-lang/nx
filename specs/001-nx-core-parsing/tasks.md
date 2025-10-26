---
description: "Implementation tasks for Core NX Parsing and Validation"
---

# Tasks: Core NX Parsing and Validation

**Input**: Design documents from `/specs/001-nx-core-parsing/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/library-api.md

**Tests**: Not explicitly requested in spec - tests included as part of implementation per constitution requirements

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2)
- Include exact file paths in descriptions

## Path Conventions

- **Rust workspace**: `crates/[crate-name]/src/`, `crates/[crate-name]/tests/`
- Four crates: nx-diagnostics, nx-syntax, nx-hir, nx-types
- Dependency flow: nx-diagnostics (leaf) → nx-syntax → nx-hir → nx-types

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Workspace initialization and crate scaffolding per plan.md

- [ ] T001 Verify Rust 1.75+ installation and workspace builds with `cargo build --workspace`
- [ ] T002 Add dependency: ariadne v0.4 to crates/nx-diagnostics/Cargo.toml
- [ ] T003 [P] Add dependency: text-size v1.1+ to crates/nx-diagnostics/Cargo.toml
- [ ] T004 [P] Add dependency: tree-sitter v0.20+ to crates/nx-syntax/Cargo.toml
- [ ] T005 [P] Add dependency: salsa v0.16+ to crates/nx-hir/Cargo.toml
- [ ] T006 [P] Add dependency: la-arena v0.3+ to crates/nx-hir/Cargo.toml
- [ ] T007 [P] Add dependency: smol_str v0.2+ to crates/nx-hir/Cargo.toml
- [ ] T008 [P] Add dependency: rustc-hash v1.1+ to crates/nx-types/Cargo.toml
- [ ] T009 [P] Add dev dependency: insta v1.34+ to workspace Cargo.toml for snapshot testing
- [ ] T010 Create directories: crates/nx-syntax/queries/ for tree-sitter queries
- [ ] T011 Create directories: crates/nx-syntax/tests/fixtures/valid/ and crates/nx-syntax/tests/fixtures/invalid/
- [ ] T012 Run `cargo build --workspace` and `cargo test --workspace` to verify setup

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core nx-diagnostics crate that ALL other components depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T013 Implement Severity enum (Error, Warning, Info) in crates/nx-diagnostics/src/lib.rs
- [ ] T014 Implement TextSpan struct with start, end, len(), contains(), merge() in crates/nx-diagnostics/src/lib.rs
- [ ] T015 Implement Label struct with span, message, style fields in crates/nx-diagnostics/src/lib.rs
- [ ] T016 Implement Diagnostic struct with severity, message, span, labels, notes in crates/nx-diagnostics/src/lib.rs
- [ ] T017 Implement Diagnostic::render() method using Ariadne for beautiful output in crates/nx-diagnostics/src/render.rs
- [ ] T018 Implement Diagnostic::eprint() method to print to stderr with colors in crates/nx-diagnostics/src/render.rs
- [ ] T019 Add unit tests for Diagnostic rendering in crates/nx-diagnostics/src/lib.rs
- [ ] T020 Add integration tests for diagnostic formatting in crates/nx-diagnostics/tests/render_tests.rs
- [ ] T021 Run `cargo test -p nx-diagnostics` to verify diagnostics crate works
- [ ] T022 Run `cargo doc -p nx-diagnostics --open` to verify rustdoc builds

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Parse and Validate NX Files (Priority: P1) 🎯 MVP

**Goal**: Parse .nx files into CST, validate syntax, report errors with source context

**Independent Test**: Run library API `parse_file("example.nx")` and verify valid files parse successfully while invalid files report specific errors with line/column numbers

### Implementation for User Story 1

**Step 1: tree-sitter Grammar (CST Layer)**

- [ ] T023 [US1] Review existing crates/nx-syntax/grammar.js tree-sitter grammar for completeness
- [ ] T024 [US1] Add build.rs script to crates/nx-syntax/ that compiles grammar.js during build
- [ ] T025 [US1] Verify tree-sitter grammar generates parser with `cargo build -p nx-syntax`
- [ ] T026 [US1] Create tree-sitter queries in crates/nx-syntax/queries/highlights.scm for syntax highlighting
- [ ] T027 [P] [US1] Create tree-sitter queries in crates/nx-syntax/queries/locals.scm for scope analysis

**Step 2: CST Wrappers**

- [ ] T028 [US1] Implement SyntaxKind enum with all token and node variants in crates/nx-syntax/src/syntax_kind.rs
- [ ] T029 [US1] Implement SyntaxTree struct wrapping tree-sitter Tree in crates/nx-syntax/src/lib.rs
- [ ] T030 [US1] Implement SyntaxTree::root(), text(), source(), node_at() methods in crates/nx-syntax/src/lib.rs
- [ ] T031 [US1] Implement SyntaxNode wrapper with kind(), text(), span(), children(), child_by_field() in crates/nx-syntax/src/syntax_node.rs
- [ ] T032 [US1] Implement SyntaxNode::is_error() and error node detection in crates/nx-syntax/src/syntax_node.rs
- [ ] T033 [US1] Add AstNode trait for typed CST node casting in crates/nx-syntax/src/ast.rs

**Step 3: Parser API**

- [ ] T034 [US1] Implement ParseResult struct with tree, errors, source_id fields in crates/nx-syntax/src/lib.rs
- [ ] T035 [US1] Implement ParseResult::is_ok(), root() methods in crates/nx-syntax/src/lib.rs
- [ ] T036 [US1] Implement parse_str(source, file_name) function in crates/nx-syntax/src/lib.rs
- [ ] T037 [US1] Implement parse_file(path) function with UTF-8 validation in crates/nx-syntax/src/lib.rs
- [ ] T038 [US1] Add UTF-8 encoding detection and error reporting in crates/nx-syntax/src/lib.rs
- [ ] T039 [US1] Implement Error enum with Io, InvalidUtf8, FileNotFound, Parse variants in crates/nx-syntax/src/lib.rs

**Step 4: Error Recovery and Validation**

- [ ] T040 [US1] Implement post-parse validation for element tag matching in crates/nx-syntax/src/validation.rs
- [ ] T041 [US1] Implement error recovery logic that collects all errors in scope in crates/nx-syntax/src/validation.rs
- [ ] T042 [US1] Convert tree-sitter ERROR nodes to Diagnostic messages in crates/nx-syntax/src/validation.rs
- [ ] T043 [US1] Add helpful error messages with suggestions for common syntax errors in crates/nx-syntax/src/validation.rs

**Step 5: Session API**

- [ ] T044 [US1] Implement ParserSession struct with Salsa database in crates/nx-syntax/src/lib.rs
- [ ] T045 [US1] Implement ParserSession::new(), parse_str(), parse_file() methods in crates/nx-syntax/src/lib.rs
- [ ] T046 [US1] Implement ParserSession::clear_cache() for memory management in crates/nx-syntax/src/lib.rs
- [ ] T047 [US1] Ensure ParserSession is Send + Sync for thread safety in crates/nx-syntax/src/lib.rs

**Step 6: Testing**

- [ ] T048 [P] [US1] Add test fixtures: valid .nx files in crates/nx-syntax/tests/fixtures/valid/
- [ ] T049 [P] [US1] Add test fixtures: invalid .nx files in crates/nx-syntax/tests/fixtures/invalid/
- [ ] T050 [US1] Write parser tests for valid syntax in crates/nx-syntax/tests/parser_tests.rs
- [ ] T051 [US1] Write parser tests for syntax errors in crates/nx-syntax/tests/parser_tests.rs
- [ ] T052 [US1] Write snapshot tests with insta for CST output in crates/nx-syntax/tests/parser_tests.rs
- [ ] T053 [US1] Write tests for UTF-8 validation in crates/nx-syntax/tests/parser_tests.rs
- [ ] T054 [US1] Write tests for concurrent parsing with threads in crates/nx-syntax/tests/parser_tests.rs
- [ ] T055 [US1] Write tests for error recovery within scopes in crates/nx-syntax/tests/parser_tests.rs
- [ ] T056 [US1] Verify performance: parsing >10,000 lines/second in crates/nx-syntax/tests/parser_tests.rs
- [ ] T057 [US1] Run `cargo test -p nx-syntax` to verify all parser tests pass
- [ ] T058 [US1] Run `cargo insta review` to accept snapshot test baselines

**Step 7: Documentation**

- [ ] T059 [P] [US1] Add rustdoc comments to all public APIs in crates/nx-syntax/src/lib.rs
- [ ] T060 [P] [US1] Add usage examples in rustdoc for parse_str, parse_file in crates/nx-syntax/src/lib.rs
- [ ] T061 [US1] Run `cargo doc -p nx-syntax --open` to verify documentation builds

**Checkpoint**: At this point, User Story 1 should be fully functional - can parse .nx files, report errors, and validate syntax independently

---

## Phase 4: User Story 2 - Check Types and Semantics (Priority: P1)

**Goal**: Perform type checking, detect type errors, resolve identifiers, infer types

**Independent Test**: Write .nx files with type errors, run library API `check_file("example.nx")`, verify type checker reports errors with clear messages

### Implementation for User Story 2

**Step 1: HIR Layer (AST)**

- [ ] T062 [P] [US2] Implement Name type using SmolStr in crates/nx-hir/src/lib.rs
- [ ] T063 [P] [US2] Implement SourceId newtype in crates/nx-hir/src/lib.rs
- [ ] T064 [US2] Implement Literal enum (String, Int, Float, Bool, Null) in crates/nx-hir/src/ast/expr.rs
- [ ] T065 [US2] Implement BinOp and UnOp enums in crates/nx-hir/src/ast/expr.rs
- [ ] T066 [US2] Implement Expr enum with all variants (Literal, Ident, BinaryOp, etc.) in crates/nx-hir/src/ast/expr.rs
- [ ] T067 [US2] Implement Stmt enum (Let, Expr) in crates/nx-hir/src/ast/stmt.rs
- [ ] T068 [P] [US2] Implement TypeRef enum (Name, Array, Nullable, Function) in crates/nx-hir/src/ast/types.rs
- [ ] T069 [P] [US2] Implement Param struct with name, ty, span in crates/nx-hir/src/lib.rs
- [ ] T070 [US2] Implement Function struct with name, params, return_type, body in crates/nx-hir/src/lib.rs
- [ ] T071 [US2] Implement Property struct with key, value, span in crates/nx-hir/src/lib.rs
- [ ] T072 [US2] Implement Element struct with tag, properties, children in crates/nx-hir/src/lib.rs
- [ ] T073 [US2] Implement Item enum (Function, TypeAlias, Element) in crates/nx-hir/src/lib.rs
- [ ] T074 [US2] Implement Module struct with arena-based storage in crates/nx-hir/src/lib.rs
- [ ] T075 [US2] Implement Module::items(), find_item() methods in crates/nx-hir/src/lib.rs

**Step 2: CST → HIR Lowering**

- [ ] T076 [US2] Implement lowering from SyntaxNode to Expr in crates/nx-hir/src/lower.rs
- [ ] T077 [US2] Implement lowering from SyntaxNode to Stmt in crates/nx-hir/src/lower.rs
- [ ] T078 [US2] Implement lowering from SyntaxNode to Function in crates/nx-hir/src/lower.rs
- [ ] T079 [US2] Implement lowering from SyntaxNode to Element in crates/nx-hir/src/lower.rs
- [ ] T080 [US2] Implement lowering from SyntaxNode to Module in crates/nx-hir/src/lower.rs
- [ ] T081 [US2] Add error handling for malformed CST during lowering in crates/nx-hir/src/lower.rs
- [ ] T082 [US2] Implement ParseResult::to_hir() method in crates/nx-syntax/src/lib.rs

**Step 3: Salsa Database Setup**

- [ ] T083 [US2] Define Salsa input query: source_text(SourceId) in crates/nx-hir/src/db.rs
- [ ] T084 [US2] Define Salsa derived query: parse(SourceId) → SyntaxTree in crates/nx-hir/src/db.rs
- [ ] T085 [US2] Define Salsa derived query: lower(SourceId) → Module in crates/nx-hir/src/db.rs
- [ ] T086 [US2] Implement NxDatabase trait with query group in crates/nx-hir/src/db.rs
- [ ] T087 [US2] Create default database implementation in crates/nx-hir/src/db.rs

**Step 4: Scope and Symbol Resolution**

- [ ] T088 [US2] Implement Scope struct with parent, symbols map in crates/nx-hir/src/lib.rs
- [ ] T089 [US2] Implement SymbolKind enum (Function, Variable, Parameter, Type) in crates/nx-hir/src/lib.rs
- [ ] T090 [US2] Implement Symbol struct with name, kind, ty, span in crates/nx-hir/src/lib.rs
- [ ] T091 [US2] Implement scope building from Module in crates/nx-hir/src/lib.rs
- [ ] T092 [US2] Implement identifier resolution in scopes in crates/nx-hir/src/lib.rs
- [ ] T093 [US2] Detect undefined identifiers and create diagnostics in crates/nx-hir/src/lib.rs

**Step 5: Type System**

- [ ] T094 [P] [US2] Implement Type enum with all variants in crates/nx-types/src/ty.rs
- [ ] T095 [P] [US2] Implement primitive type constants (String, Int, Float, Boolean, Void) in crates/nx-types/src/primitives.rs
- [ ] T096 [US2] Implement InferenceVar type for type variables in crates/nx-types/src/infer.rs
- [ ] T097 [US2] Implement Constraint struct for unification in crates/nx-types/src/unify.rs
- [ ] T098 [US2] Implement TypeEnvironment with parent scopes in crates/nx-types/src/lib.rs
- [ ] T099 [US2] Implement TypeEnvironment::lookup(), insert(), with_parent() in crates/nx-types/src/lib.rs

**Step 6: Type Inference**

- [ ] T100 [US2] Implement InferenceContext with var_counter, substitutions, constraints in crates/nx-types/src/infer.rs
- [ ] T101 [US2] Implement InferenceContext::fresh_var() to create type variables in crates/nx-types/src/infer.rs
- [ ] T102 [US2] Implement type inference for literals in crates/nx-types/src/infer.rs
- [ ] T103 [US2] Implement type inference for binary operations in crates/nx-types/src/infer.rs
- [ ] T104 [US2] Implement type inference for function calls in crates/nx-types/src/infer.rs
- [ ] T105 [US2] Implement type inference for let bindings in crates/nx-types/src/infer.rs
- [ ] T106 [US2] Implement type inference for function return types in crates/nx-types/src/infer.rs
- [ ] T107 [US2] Implement constraint collection during inference in crates/nx-types/src/infer.rs

**Step 7: Type Unification**

- [ ] T108 [US2] Implement unify(Type, Type) algorithm in crates/nx-types/src/unify.rs
- [ ] T109 [US2] Implement unification for primitive types in crates/nx-types/src/unify.rs
- [ ] T110 [US2] Implement unification for array types in crates/nx-types/src/unify.rs
- [ ] T111 [US2] Implement unification for function types in crates/nx-types/src/unify.rs
- [ ] T112 [US2] Implement unification for nullable types in crates/nx-types/src/unify.rs
- [ ] T113 [US2] Implement occurs check to prevent infinite types in crates/nx-types/src/unify.rs
- [ ] T114 [US2] Generate type error diagnostics from unification failures in crates/nx-types/src/unify.rs

**Step 8: Type Checking API**

- [ ] T115 [US2] Implement TypeCheckResult struct with module, type_env, diagnostics, expr_types in crates/nx-types/src/lib.rs
- [ ] T116 [US2] Implement TypeCheckResult::is_ok(), type_of(), errors() methods in crates/nx-types/src/lib.rs
- [ ] T117 [US2] Implement check_str(source, file_name) function in crates/nx-types/src/lib.rs
- [ ] T118 [US2] Implement check_file(path) function in crates/nx-types/src/lib.rs
- [ ] T119 [US2] Implement TypeCheckSession struct with parser, type environment in crates/nx-types/src/lib.rs
- [ ] T120 [US2] Implement TypeCheckSession::new(), add_file(), check_file(), check_all() in crates/nx-types/src/lib.rs
- [ ] T121 [US2] Implement TypeCheckSession::diagnostics() to collect all errors in crates/nx-types/src/lib.rs
- [ ] T122 [US2] Ensure TypeCheckSession is Send + Sync for thread safety in crates/nx-types/src/lib.rs

**Step 9: Error Detection**

- [ ] T123 [US2] Detect type mismatches in assignments and report with expected vs actual in crates/nx-types/src/lib.rs
- [ ] T124 [US2] Detect type mismatches in function calls and report parameter mismatches in crates/nx-types/src/lib.rs
- [ ] T125 [US2] Detect circular type definitions and report clear errors in crates/nx-types/src/lib.rs
- [ ] T126 [US2] Detect undefined types and suggest similar type names in crates/nx-types/src/lib.rs
- [ ] T127 [US2] Detect nullable type usage without null checks and warn in crates/nx-types/src/lib.rs
- [ ] T128 [US2] Add source context and suggestions to all type error diagnostics in crates/nx-types/src/lib.rs

**Step 10: Testing**

- [ ] T129 [P] [US2] Add test fixtures with valid types in crates/nx-types/tests/fixtures/type_examples.nx
- [ ] T130 [P] [US2] Add test fixtures with type errors in crates/nx-types/tests/fixtures/type_examples.nx
- [ ] T131 [US2] Write tests for type inference in crates/nx-types/tests/type_checker_tests.rs
- [ ] T132 [US2] Write tests for type mismatch detection in crates/nx-types/tests/type_checker_tests.rs
- [ ] T133 [US2] Write tests for undefined identifier detection in crates/nx-types/tests/type_checker_tests.rs
- [ ] T134 [US2] Write tests for circular type detection in crates/nx-types/tests/type_checker_tests.rs
- [ ] T135 [US2] Write tests for function parameter type checking in crates/nx-types/tests/type_checker_tests.rs
- [ ] T136 [US2] Write tests for local type inference in crates/nx-types/tests/type_checker_tests.rs
- [ ] T137 [US2] Write tests for function return type inference in crates/nx-types/tests/type_checker_tests.rs
- [ ] T138 [US2] Verify performance: type checking <2 seconds for 10,000 lines in crates/nx-types/tests/type_checker_tests.rs
- [ ] T139 [US2] Verify memory: <100MB for 10,000-line files in crates/nx-types/tests/type_checker_tests.rs
- [ ] T140 [US2] Write lowering tests for CST → HIR in crates/nx-hir/tests/lowering_tests.rs
- [ ] T141 [US2] Run `cargo test -p nx-hir` to verify HIR crate tests pass
- [ ] T142 [US2] Run `cargo test -p nx-types` to verify type checker tests pass

**Step 11: Documentation**

- [ ] T143 [P] [US2] Add rustdoc comments to all public APIs in crates/nx-hir/src/lib.rs
- [ ] T144 [P] [US2] Add rustdoc comments to all public APIs in crates/nx-types/src/lib.rs
- [ ] T145 [P] [US2] Add usage examples in rustdoc for check_str, check_file in crates/nx-types/src/lib.rs
- [ ] T146 [US2] Run `cargo doc -p nx-hir --open` to verify HIR documentation builds
- [ ] T147 [US2] Run `cargo doc -p nx-types --open` to verify type checker documentation builds

**Checkpoint**: At this point, User Story 2 should be fully functional - can type check .nx files, detect type errors, infer types, and resolve identifiers independently

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect both user stories and overall quality

- [ ] T148 [P] Run `cargo fmt --all` to format all code
- [ ] T149 [P] Run `cargo clippy --workspace` and fix all warnings
- [ ] T150 Run `cargo test --workspace` to verify all tests pass across all crates
- [ ] T151 Run `cargo doc --workspace --open` to verify all documentation builds
- [ ] T152 [P] Add workspace-level README.md documenting the library structure
- [ ] T153 [P] Update quickstart.md with final API examples and usage patterns
- [ ] T154 Performance optimization: Profile parsing with large files and optimize hotspots
- [ ] T155 Performance optimization: Profile type checking with large files and optimize hotspots
- [ ] T156 Memory optimization: Verify memory usage targets are met (<100MB for 10k lines)
- [ ] T157 Code review: Review all public APIs for consistency and ergonomics
- [ ] T158 Security review: Verify no panics in public API, all errors handled gracefully
- [ ] T159 Run full test suite with `RUST_BACKTRACE=1 cargo test --workspace` to catch any panics
- [ ] T160 Validate all success criteria from spec.md are met

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational phase completion
- **User Story 2 (Phase 4)**: Depends on Foundational and User Story 1 completion (needs nx-syntax crate)
- **Polish (Phase 5)**: Depends on both user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P1)**: Can start after User Story 1 completion - Depends on nx-syntax crate

### Within Each User Story

**User Story 1**:
- Grammar work before CST wrappers
- CST wrappers before parser API
- Parser API before error recovery
- Core implementation before session API
- Implementation before testing
- Tests before documentation

**User Story 2**:
- HIR types before lowering
- Lowering before Salsa setup
- Salsa before scope resolution
- Type system before inference
- Inference before unification
- Core implementation before API
- API before error detection
- Implementation before testing
- Tests before documentation

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel (T002-T009)
- Models/types marked [P] can run in parallel within their phases
- Test fixture creation can run in parallel
- Documentation tasks can run in parallel
- Polish tasks marked [P] can run in parallel

---

## Parallel Example: User Story 1

```bash
# Launch grammar and queries together:
Task T024: "Add build.rs script to crates/nx-syntax/"
Task T026: "Create tree-sitter queries in crates/nx-syntax/queries/highlights.scm"
Task T027: "Create tree-sitter queries in crates/nx-syntax/queries/locals.scm"

# Launch test fixture creation together:
Task T048: "Add test fixtures: valid .nx files"
Task T049: "Add test fixtures: invalid .nx files"

# Launch documentation tasks together:
Task T059: "Add rustdoc comments to all public APIs"
Task T060: "Add usage examples in rustdoc"
```

## Parallel Example: User Story 2

```bash
# Launch HIR type implementations together:
Task T062: "Implement Name type using SmolStr"
Task T063: "Implement SourceId newtype"
Task T068: "Implement TypeRef enum"

# Launch type system primitives together:
Task T094: "Implement Type enum with all variants"
Task T095: "Implement primitive type constants"

# Launch test fixture creation together:
Task T129: "Add test fixtures with valid types"
Task T130: "Add test fixtures with type errors"

# Launch documentation tasks together:
Task T143: "Add rustdoc comments to nx-hir public APIs"
Task T144: "Add rustdoc comments to nx-types public APIs"
Task T145: "Add usage examples in rustdoc"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (nx-diagnostics) - CRITICAL
3. Complete Phase 3: User Story 1 (Parse and Validate)
4. **STOP and VALIDATE**: Test parsing independently with various .nx files
5. Demo/review if ready

### Full Feature (Both Stories)

1. Complete Setup + Foundational → Foundation ready
2. Complete User Story 1 → Test independently → MVP complete!
3. Complete User Story 2 → Test independently → Full type checking ready!
4. Polish → Production ready

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Developer A: User Story 1 (parsing)
3. Once User Story 1 complete, Developer B: User Story 2 (type checking)
4. Both developers: Polish phase together

---

## Summary

- **Total Tasks**: 160 tasks
- **User Story 1 Tasks**: 39 tasks (T023-T061)
- **User Story 2 Tasks**: 86 tasks (T062-T147)
- **Setup Tasks**: 12 tasks (T001-T012)
- **Foundational Tasks**: 10 tasks (T013-T022)
- **Polish Tasks**: 13 tasks (T148-T160)
- **Parallel Opportunities**: 30+ tasks marked [P]
- **MVP Scope**: Phases 1-3 (Setup + Foundational + User Story 1) = 61 tasks
- **Full Feature Scope**: Phases 1-5 (all tasks) = 160 tasks

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story builds on previous foundations
- Tests integrated into implementation per constitution requirements
- Commit after each task or logical group
- Stop at checkpoints to validate story independently
- Format validation: ALL tasks follow `- [ ] [ID] [Labels] Description` format ✅
