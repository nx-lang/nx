# Implementation Plan: NX Interpreter

**Branch**: `002-nx-interpreter` | **Date**: 2025-10-29 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/002-nx-interpreter/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Create a production-ready interpreter for NX that executes HIR (High-level Intermediate Representation), accepting NX functions and parameters as input and returning computed values. The interpreter prioritizes reliability and security for production use while maintaining reasonable performance for interpreted execution. It will handle expressions, conditionals, loops, function calls, and comprehensive runtime error reporting with resource limits (recursion depth: 1000, operation count: 1M per execution).

## Technical Context

**Language/Version**: Rust 1.80.1  
**Primary Dependencies**: nx-hir (HIR data structures), nx-diagnostics (error reporting), nx-types (type information)  
**Storage**: N/A (in-memory execution only)  
**Testing**: cargo test (unit tests, integration tests, contract tests)  
**Target Platform**: Cross-platform (Linux, macOS, Windows) - library crate  
**Project Type**: Single project (new Rust crate in workspace)  
**Performance Goals**: <100ms for functions with up to 1000 operations, support 1000 recursion depth  
**Constraints**: 
- Recursion depth limit: 1000 calls (stack overflow protection)
- Operation count limit: 1 million operations per execution (infinite loop protection)
- Production-ready: reliability and security prioritized over maximum performance
- Must integrate with existing nx-diagnostics for consistent error reporting

**Scale/Scope**: MVP interpreter supporting:
- All NX primitive types (int, float, string, boolean, null)
- Arithmetic, logical, comparison, string operations
- Conditionals (if/else)
- Loops (for, while) with break/continue
- Function calls (including recursion up to depth limit)
- Comprehensive runtime error reporting with source locations

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Quality Standards
- ✅ **Code Quality First**: Rust with explicit types, cargo fmt formatting, zero warnings required
- ✅ **Comprehensive Testing**: All interpreter logic will have unit tests, integration tests for end-to-end execution, contract tests for public API
- ✅ **User Experience Consistency**: Error messages via nx-diagnostics (Ariadne) for beautiful, helpful error reporting with source context
- ✅ **Cross-Platform**: Library crate works identically on Windows, Linux, macOS

### Architecture Compliance
- ✅ **Dependency Direction**: New `nx-interpreter` crate will depend on nx-hir, nx-types, nx-diagnostics (upward dependencies)
- ✅ **One Primary Type Per File**: Interpreter, ExecutionContext, Value, RuntimeError in separate modules
- ✅ **Access Modifiers**: Explicit (pub for API, private by default for internals)

### Performance & Security
- ✅ **Resource Limits**: Recursion depth (1000), operation count (1M) enforced for production security
- ✅ **Error Handling**: Comprehensive runtime error detection and reporting (division by zero, null operations, parameter mismatches, etc.)
- ✅ **Benchmarking**: Performance tests for execution time will be added (target: <100ms for 1000 operations)

### Complexity Assessment
- ✅ **No Violations**: This is a straightforward interpreter implementation following standard patterns
- ✅ **Existing Patterns**: Follows architecture established by nx-syntax, nx-hir, nx-types crates
- ✅ **Justified Scope**: Essential feature for NX language - ability to execute code is core functionality

**Status**: ✅ All gates passed - proceeding to Phase 0 research

---

## Phase 0: Research ✅ COMPLETE

**Status**: All technical unknowns resolved

See [research.md](./research.md) for detailed findings.

**Key Decisions**:
1. HIR structure examined from nx-hir crate (Module, Expr, Stmt, Function)
2. Value representation: Rust enum with variants (Int, Float, String, Boolean, Null)
3. Execution context: Stack of scopes + call stack
4. Error reporting: RuntimeError integrating with nx-diagnostics/Ariadne
5. Operation counting: Simple counter checked per operation
6. Implementation pattern: Recursive tree-walking interpreter

---

## Phase 1: Design & Contracts ✅ COMPLETE

**Status**: Data model, API contracts, and quickstart complete

**Artifacts Created**:
- [data-model.md](./data-model.md) - Core entities, relationships, validation rules
- [contracts/api.md](./contracts/api.md) - Public API contract with examples
- [quickstart.md](./quickstart.md) - Developer integration guide

**Key Entities Defined**:
1. **Interpreter** - Stateless execution engine
2. **ExecutionContext** - Runtime state (scopes, call stack, resource limits)
3. **Value** - Runtime value representation (enum with 5+ variants)
4. **RuntimeError** - Error type with Ariadne integration
5. **CallFrame** - Call stack frame for debugging

**API Surface**:
- `Interpreter::new()` and `execute_function()` - Core execution API
- `execute_function_with_limits()` - Configurable resource limits
- `Value` enum with type checking methods
- `RuntimeError` with beautiful formatting via Ariadne
- `ResourceLimits` for configurable execution bounds

**Agent Context Updated**: ✅ GitHub Copilot context file updated with technology stack

### Constitution Re-Check (Post-Design)

✅ **Code Quality**: Explicit Rust types, modular design (lib.rs, interpreter.rs, context.rs, value.rs, error.rs, eval/)  
✅ **Testing**: Design includes unit/, integration/, and contract/ test structure  
✅ **Error Reporting**: RuntimeError integrates with nx-diagnostics/Ariadne for beautiful errors  
✅ **Architecture**: Clean separation of concerns, upward dependencies only  
✅ **Performance**: Resource limits enforced, efficient data structures chosen  
✅ **Security**: No unsafe code, comprehensive error handling, bounded execution

**Status**: ✅ Design passes all constitution gates

---

## Phase 2: Task Planning ✅ COMPLETE

**Status**: Task breakdown generated with 60 granular tasks

See [tasks.md](./tasks.md) for complete task list.

**Task Summary**:
- **Total Tasks**: 60
- **MVP Tasks**: T001-T025 (Setup + Foundation + User Story 1)
- **Production Ready**: +T026-T035 (Error Handling)
- **Feature Complete**: All 60 tasks

**Tasks by User Story**:
- Setup (Phase 1): 4 tasks
- Foundation (Phase 2): 6 tasks
- User Story 1 - Execute Simple Functions (P1 MVP): 15 tasks
- User Story 4 - Error Handling (P2): 10 tasks
- User Story 2 - Conditionals (P2): 8 tasks
- User Story 3 - Loops (P3): 9 tasks
- Polish: 8 tasks

**Parallel Opportunities**:
- Foundation: 3 tasks parallelizable (Value, ExecutionContext, RuntimeError modules)
- User Story 1: 3 tasks parallelizable (arithmetic, logical, string evaluation)
- User Story 4: 7 tasks parallelizable (individual error kinds)
- User Story 2: 4 integration tests parallelizable
- User Story 3: 5 integration tests parallelizable

**Independent Test Criteria**:
- US1: Execute `let <add a:int b:int /> = { a + b }` → result 8
- US2: Execute `let <max a:int b:int /> = { if a > b { a } else { b } }` → correct branch
- US3: Execute sum loop with n=5 → result 15
- US4: Execute `x / 0` → clear error with source location

---

## Next Steps

**Ready for Implementation**: All planning phases complete

1. **Start with MVP** (T001-T025):
   - Phase 1: Setup (T001-T004)
   - Phase 2: Foundation (T005-T010)
   - Phase 3: User Story 1 (T011-T025)
   
2. **Add Production Error Handling** (T026-T035):
   - Phase 4: User Story 4
   
3. **Expand Features** (T036-T052):
   - Phase 5: User Story 2 (Conditionals)
   - Phase 6: User Story 3 (Loops)
   
4. **Polish** (T053-T060):
   - Phase 7: Recursion, performance, documentation

Each phase delivers independently testable functionality.

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
crates/
└── nx-interpreter/          # New crate for this feature
    ├── Cargo.toml
    ├── src/
    │   ├── lib.rs          # Public API exports
    │   ├── interpreter.rs  # Core Interpreter implementation
    │   ├── context.rs      # ExecutionContext (variables, call stack)
    │   ├── value.rs        # Runtime Value representation
    │   ├── error.rs        # RuntimeError types and handling
    │   └── eval/           # Expression evaluation modules
    │       ├── mod.rs
    │       ├── arithmetic.rs   # Arithmetic operations
    │       ├── logical.rs      # Logical operations
    │       ├── control.rs      # Conditionals and loops
    │       └── functions.rs    # Function calls
    └── tests/
        ├── unit/           # Unit tests per module
        │   ├── interpreter_tests.rs
        │   ├── context_tests.rs
        │   ├── value_tests.rs
        │   └── eval_tests.rs
        ├── integration/    # End-to-end execution tests
        │   ├── simple_functions.rs
        │   ├── conditionals.rs
        │   ├── loops.rs
        │   └── recursion.rs
        └── contract/       # Public API stability tests
            └── api_tests.rs
```

**Structure Decision**: Single project structure with new `nx-interpreter` crate added to the existing Cargo workspace. This follows the established pattern of other nx-* crates (nx-syntax, nx-hir, nx-types). The crate uses a modular design with separate modules for core interpreter logic, execution context, value representation, error handling, and categorized expression evaluation.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| [e.g., 4th project] | [current need] | [why 3 projects insufficient] |
| [e.g., Repository pattern] | [specific problem] | [why direct DB access insufficient] |
