# Research: NX Interpreter

**Feature**: 002-nx-interpreter  
**Date**: 2025-10-29  
**Purpose**: Resolve technical unknowns and establish implementation patterns

## Research Questions

### 1. HIR Structure and Traversal

**Question**: What is the structure of the HIR from nx-hir, and how should it be traversed for interpretation?

**Research Findings**:
- HIR (High-level Intermediate Representation) is the output from nx-hir crate
- Need to examine nx-hir crate structure to understand:
  - HIR node types (expressions, statements, declarations)
  - How functions are represented in HIR
  - How to access function parameters and body
  - Type information embedded in HIR nodes

**Decision**: Examine existing nx-hir crate implementation
- Read `crates/nx-hir/src/lib.rs` for public API
- Read HIR data structures and node types
- Understand how nx-types integrates type information into HIR

**Rationale**: Must understand input format before designing interpreter

**Alternatives Considered**:
- Build custom IR: Rejected - would duplicate work and lose type information
- Use AST directly: Rejected - HIR is more structured and type-checked

---

### 2. Runtime Value Representation

**Question**: How should runtime values be represented in memory?

**Research Findings**:
- Need to support NX primitive types: int, float, string, boolean, null
- May need to support arrays and objects in future
- Values need type information for runtime checks
- Values must be cloneable for variable bindings

**Decision**: Use Rust enum with variants for each type
```rust
pub enum Value {
    Int(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
    // Future: Array, Object, Function
}
```

**Rationale**: 
- Enums provide type safety and pattern matching
- Aligned with Rust best practices for variant types
- Easy to extend with new types
- Small enough to clone efficiently

**Alternatives Considered**:
- Trait objects: Rejected - more complex, heap allocation overhead
- Untagged union: Rejected - unsafe, loses Rust's type safety benefits

---

### 3. Execution Context Management

**Question**: How should variable bindings and call stack be managed?

**Research Findings**:
- Need to track variables in current scope
- Need call stack for function calls and recursion tracking
- Need to support nested scopes (blocks within functions)
- Must enforce recursion depth limit (1000 calls)

**Decision**: Two-level context structure
```rust
pub struct ExecutionContext {
    // Variable scopes: stack of maps (innermost scope at end)
    scopes: Vec<HashMap<String, Value>>,
    
    // Call stack: track function calls for recursion limit
    call_stack: Vec<CallFrame>,
}

pub struct CallFrame {
    function_name: String,
    source_location: SourceLocation,
}
```

**Rationale**:
- Stack of scopes handles nested blocks correctly
- Separate call stack enables recursion depth enforcement
- HashMap lookups are O(1) for variable access
- Simple to push/pop scopes and frames

**Alternatives Considered**:
- Single flat map: Rejected - doesn't handle nested scopes
- Tree structure: Rejected - overkill for linear scope nesting
- Global state: Rejected - not thread-safe, harder to test

---

### 4. Error Reporting Integration

**Question**: How should runtime errors integrate with nx-diagnostics?

**Research Findings**:
- nx-diagnostics uses Ariadne for beautiful error reporting
- Errors need source locations from HIR nodes
- Should follow same patterns as compile-time errors
- Need clear error messages with context

**Decision**: Create RuntimeError type that integrates with nx-diagnostics
```rust
pub struct RuntimeError {
    kind: RuntimeErrorKind,
    message: String,
    location: SourceLocation,
}

pub enum RuntimeErrorKind {
    DivisionByZero,
    NullOperation,
    UndefinedVariable,
    TypeMismatch,
    ParameterCountMismatch,
    StackOverflow,
    OperationLimitExceeded,
    MissingReturn,
}
```

**Rationale**:
- Follows existing diagnostic patterns in nx-diagnostics
- Enum provides type-safe error categorization
- Source location enables Ariadne to show code context
- Extensible for future error types

**Alternatives Considered**:
- String-based errors: Rejected - loses structure and type safety
- Panic on errors: Rejected - not recoverable, bad for production
- Result without context: Rejected - poor user experience

---

### 5. Operation Counting for Infinite Loop Protection

**Question**: How should operation counting work to enforce the 1M operation limit?

**Research Findings**:
- Need to count every HIR instruction executed
- Must be efficient (checked on every operation)
- Should be configurable for testing
- Exceeding limit should produce clear error

**Decision**: Counter in ExecutionContext, increment on every eval step
```rust
pub struct ExecutionContext {
    // ... other fields ...
    operation_count: usize,
    operation_limit: usize, // Default: 1_000_000
}

impl ExecutionContext {
    fn check_operation_limit(&mut self) -> Result<(), RuntimeError> {
        self.operation_count += 1;
        if self.operation_count > self.operation_limit {
            Err(RuntimeError::operation_limit_exceeded())
        } else {
            Ok(())
        }
    }
}
```

**Rationale**:
- Simple counter with O(1) increment and check
- No performance impact (single integer comparison)
- Configurable limit enables testing and different use cases
- Fails fast when limit exceeded

**Alternatives Considered**:
- Time-based limit: Rejected - not deterministic, platform-dependent
- Instruction-level hooks: Rejected - too complex, performance overhead
- No limit: Rejected - violates security requirement

---

### 6. Best Practices for Interpreter Implementation

**Question**: What are the established patterns for tree-walking interpreters in Rust?

**Research Findings**:
- Visitor pattern is common for AST/HIR traversal
- Recursive evaluation for expression trees
- Result<T, E> for error propagation
- Pattern matching on node types

**Decision**: Use recursive evaluator with pattern matching
```rust
impl Interpreter {
    pub fn eval_expr(&mut self, expr: &HirExpr, ctx: &mut ExecutionContext) 
        -> Result<Value, RuntimeError> 
    {
        ctx.check_operation_limit()?;
        
        match expr {
            HirExpr::Literal(lit) => self.eval_literal(lit),
            HirExpr::Binary(op, lhs, rhs) => self.eval_binary(op, lhs, rhs, ctx),
            HirExpr::Variable(name) => ctx.lookup_variable(name),
            // ... other expression types
        }
    }
}
```

**Rationale**:
- Follows "Crafting Interpreters" and Rust interpreter patterns
- Pattern matching is idiomatic Rust and exhaustive
- Result type enables clean error propagation
- Recursive structure matches HIR tree structure

**Alternatives Considered**:
- Bytecode VM: Rejected - overkill for MVP, adds complexity
- Stack machine: Rejected - harder to debug, more implementation work
- Non-recursive iterative: Rejected - more complex, loses clarity

---

## Summary

All technical unknowns have been resolved with concrete decisions:

1. **HIR Integration**: Will read nx-hir structures directly, examine existing crate for API
2. **Value Representation**: Rust enum with variants for each type
3. **Execution Context**: Stack of variable scopes + call stack for recursion tracking
4. **Error Reporting**: RuntimeError type integrating with nx-diagnostics/Ariadne
5. **Operation Counting**: Simple counter in context, checked on every eval step
6. **Implementation Pattern**: Recursive tree-walking interpreter with pattern matching

These decisions enable progression to Phase 1 (data model and contracts).
