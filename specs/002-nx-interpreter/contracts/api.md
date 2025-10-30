# API Contract: nx-interpreter Public Interface

**Feature**: 002-nx-interpreter  
**Date**: 2025-10-29  
**Purpose**: Define public API contract for the interpreter crate

## Overview

The `nx-interpreter` crate provides a simple, production-ready interpreter for NX code. It accepts HIR (High-level Intermediate Representation) from `nx-hir` and executes functions with provided parameters.

## Public API

### Core Types

#### `Interpreter`

Main entry point for code execution.

```rust
pub struct Interpreter {
    // Internal state (private)
}

impl Interpreter {
    /// Create a new interpreter instance.
    pub fn new() -> Self;
    
    /// Execute a function by name from the given module.
    ///
    /// # Arguments
    /// * `module` - The HIR module containing the function
    /// * `function_name` - Name of the function to execute
    /// * `args` - Arguments to pass to the function
    ///
    /// # Returns
    /// * `Ok(Value)` - The computed result value
    /// * `Err(RuntimeError)` - Runtime error with diagnostics
    ///
    /// # Example
    /// ```rust
    /// use nx_interpreter::{Interpreter, Value};
    /// use nx_hir::Module;
    /// 
    /// let interpreter = Interpreter::new();
    /// let args = vec![Value::Int(5), Value::Int(3)];
    /// let result = interpreter.execute_function(&module, "add", args)?;
    /// assert_eq!(result, Value::Int(8));
    /// ```
    pub fn execute_function(
        &self,
        module: &Module,
        function_name: &str,
        args: Vec<Value>,
    ) -> Result<Value, RuntimeError>;
    
    /// Execute a function with custom resource limits.
    ///
    /// # Arguments
    /// * `module` - The HIR module containing the function
    /// * `function_name` - Name of the function to execute
    /// * `args` - Arguments to pass to the function
    /// * `limits` - Custom resource limits for execution
    ///
    /// # Returns
    /// * `Ok(Value)` - The computed result value
    /// * `Err(RuntimeError)` - Runtime error with diagnostics
    pub fn execute_function_with_limits(
        &self,
        module: &Module,
        function_name: &str,
        args: Vec<Value>,
        limits: ResourceLimits,
    ) -> Result<Value, RuntimeError>;
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}
```

#### `Value`

Runtime representation of NX values.

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Integer value
    Int(i64),
    
    /// Floating-point value  
    Float(f64),
    
    /// String value
    String(String),
    
    /// Boolean value
    Boolean(bool),
    
    /// Null value
    Null,
}

impl Value {
    /// Get the type name of this value.
    pub fn type_name(&self) -> &'static str;
    
    /// Check if value is truthy (for conditionals).
    pub fn is_truthy(&self) -> bool;
    
    /// Convert value to display string.
    pub fn to_string(&self) -> String;
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}
```

#### `RuntimeError`

Runtime execution error with diagnostic information.

```rust
#[derive(Debug, Clone)]
pub struct RuntimeError {
    // Internal fields (private)
}

impl RuntimeError {
    /// Get the error kind.
    pub fn kind(&self) -> &RuntimeErrorKind;
    
    /// Get the error message.
    pub fn message(&self) -> &str;
    
    /// Get the source location where error occurred.
    pub fn location(&self) -> TextSpan;
    
    /// Get the call stack at time of error.
    pub fn call_stack(&self) -> &[CallFrame];
    
    /// Format error with source code context using Ariadne.
    ///
    /// # Arguments
    /// * `source_id` - Source file identifier
    /// * `source` - Source code text
    ///
    /// # Returns
    /// Formatted error string with source context and annotations
    pub fn format(&self, source_id: SourceId, source: &str) -> String;
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

impl std::error::Error for RuntimeError {}
```

#### `RuntimeErrorKind`

Categories of runtime errors.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeErrorKind {
    /// Division by zero
    DivisionByZero,
    
    /// Null value used in operation
    NullOperation,
    
    /// Variable not found
    UndefinedVariable(String),
    
    /// Type mismatch
    TypeMismatch {
        expected: String,
        got: String,
    },
    
    /// Wrong parameter count
    ParameterCountMismatch {
        expected: usize,
        got: usize,
    },
    
    /// Recursion limit exceeded
    StackOverflow,
    
    /// Operation limit exceeded
    OperationLimitExceeded,
    
    /// Missing return statement
    MissingReturn,
    
    /// Function not found
    FunctionNotFound(String),
}
```

#### `ResourceLimits`

Configurable resource limits for execution.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceLimits {
    /// Maximum recursion depth (default: 1000)
    pub recursion_limit: usize,
    
    /// Maximum operation count (default: 1_000_000)
    pub operation_limit: usize,
}

impl ResourceLimits {
    /// Create limits with default values.
    pub fn new() -> Self;
    
    /// Create unlimited limits (use with caution!).
    pub fn unlimited() -> Self;
    
    /// Create limits for testing (lower values).
    pub fn for_testing() -> Self;
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self::new()
    }
}
```

#### `CallFrame`

Represents a function call in the call stack.

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct CallFrame {
    // Internal fields (private)
}

impl CallFrame {
    /// Get the function name.
    pub fn function_name(&self) -> &str;
    
    /// Get the call site location.
    pub fn call_site(&self) -> TextSpan;
}
```

## Usage Examples

### Example 1: Execute Simple Function

```rust
use nx_interpreter::{Interpreter, Value};
use nx_syntax::parse_str;
use nx_hir::lower;

// Parse NX code
let source = "let <add a:int b:int /> = { a + b }";
let parse_result = parse_str(source, "example.nx");
let module = lower(parse_result.root().unwrap(), parse_result.source_id);

// Execute function
let interpreter = Interpreter::new();
let args = vec![Value::Int(5), Value::Int(3)];
let result = interpreter.execute_function(&module, "add", args)?;

assert_eq!(result, Value::Int(8));
```

### Example 2: Handle Runtime Errors

```rust
use nx_interpreter::{Interpreter, Value, RuntimeErrorKind};

let source = "let <divide a:int b:int /> = { a / b }";
let module = /* ... parse and lower ... */;

let interpreter = Interpreter::new();
let args = vec![Value::Int(10), Value::Int(0)];
let result = interpreter.execute_function(&module, "divide", args);

match result {
    Ok(value) => println!("Result: {}", value),
    Err(err) => {
        match err.kind() {
            RuntimeErrorKind::DivisionByZero => {
                eprintln!("Error: {}", err.format(source_id, source));
            }
            _ => eprintln!("Unexpected error: {}", err),
        }
    }
}
```

### Example 3: Custom Resource Limits

```rust
use nx_interpreter::{Interpreter, Value, ResourceLimits};

let interpreter = Interpreter::new();

// Lower limits for testing
let limits = ResourceLimits {
    recursion_limit: 100,
    operation_limit: 10_000,
};

let args = vec![Value::Int(5)];
let result = interpreter.execute_function_with_limits(
    &module,
    "factorial",
    args,
    limits,
);
```

### Example 4: Recursion with Depth Limit

```rust
use nx_interpreter::{Interpreter, Value, RuntimeErrorKind};

let source = r#"
let <factorial n:int /> = {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}
"#;

let module = /* ... parse and lower ... */;
let interpreter = Interpreter::new();
let args = vec![Value::Int(5)];

match interpreter.execute_function(&module, "factorial", args) {
    Ok(Value::Int(result)) => println!("5! = {}", result),
    Err(err) if matches!(err.kind(), RuntimeErrorKind::StackOverflow) => {
        eprintln!("Recursion too deep!");
    }
    Err(err) => eprintln!("Error: {}", err),
}
```

## Stability Guarantees

### Semver Contract

This crate follows semantic versioning:

- **Breaking changes** (major version bump):
  - Removing public types or methods
  - Changing method signatures
  - Changing enum variant names or adding non-exhaustive variants
  
- **Non-breaking changes** (minor version bump):
  - Adding new public types or methods
  - Adding new enum variants (with `#[non_exhaustive]`)
  - Expanding functionality
  
- **Patch changes**:
  - Bug fixes
  - Performance improvements
  - Documentation updates

### API Stability

- ✅ **Stable**: `Interpreter`, `Value`, `RuntimeError`, `RuntimeErrorKind`, `ResourceLimits`
- ⚠️ **Unstable**: Internal modules (subject to change without notice)

## Error Handling Contract

### Guaranteed Behaviors

1. **No Panics**: The interpreter MUST NOT panic on well-formed HIR input
2. **Result Types**: All fallible operations return `Result<T, RuntimeError>`
3. **Error Context**: All errors include source location and call stack
4. **Deterministic**: Same input always produces same output or error

### Error Reporting

All errors MUST include:
- Clear, user-friendly message
- Source location (file, line, column)
- Call stack for debugging
- Error category (RuntimeErrorKind)

Errors integrate with `nx-diagnostics` for beautiful formatting with Ariadne.

## Performance Contract

### Guarantees

- **Operation Limit**: Execution halts after 1M operations (configurable)
- **Recursion Limit**: Call stack limited to 1000 frames (configurable)
- **Memory**: Linear in code size, bounded by resource limits
- **Time Complexity**:
  - Variable lookup: O(scope_depth) - typically 3-5 levels
  - Function call: O(1) overhead + O(body)
  - Expression eval: O(tree_size)

### Performance Targets

- Functions with <1000 operations: <100ms
- Recursive functions to depth 100: <50ms
- Variable lookups: <1μs

## Dependencies

### Required Crates

- `nx-hir`: HIR data structures (Module, Expr, Stmt)
- `nx-diagnostics`: Error reporting and text spans
- `nx-types`: Type information (already in HIR)

### Optional Dependencies

- (none)

## Thread Safety

- ✅ `Interpreter`: Send + Sync (stateless)
- ✅ `Value`: Send + Sync (all data types are thread-safe)
- ⚠️ `Module`: Send + !Sync (from nx-hir, contains internal arenas)
- ✅ `RuntimeError`: Send + Sync (fully owned data)

**Recommendation**: Create one `Interpreter` per thread if needed, share `Module` immutably.

## Testing Contract

### Test Categories

1. **Unit Tests**: All public methods have unit tests
2. **Integration Tests**: End-to-end function execution
3. **Contract Tests**: API stability and backward compatibility
4. **Property Tests**: Randomized testing for edge cases (if applicable)

### Required Test Coverage

- ✅ All `RuntimeErrorKind` variants triggered
- ✅ All `Value` types supported
- ✅ All expression types evaluated
- ✅ Resource limits enforced
- ✅ Error messages are clear and actionable
