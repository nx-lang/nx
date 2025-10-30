# Data Model: NX Interpreter

**Feature**: 002-nx-interpreter  
**Date**: 2025-10-29  
**Purpose**: Define core entities, their relationships, and validation rules

## Core Entities

### 1. Interpreter

**Purpose**: Main execution engine that interprets HIR and returns results

**Fields**:
- (Stateless - operates on provided Module and ExecutionContext)

**Methods**:
```rust
pub fn execute_function(
    &self, 
    module: &Module,
    function_name: &str,
    args: Vec<Value>
) -> Result<Value, RuntimeError>

fn eval_expr(
    &self,
    expr: &Expr,
    module: &Module,
    ctx: &mut ExecutionContext
) -> Result<Value, RuntimeError>

fn eval_stmt(
    &self,
    stmt: &Stmt,
    module: &Module,
    ctx: &mut ExecutionContext
) -> Result<(), RuntimeError>
```

**Validation Rules**:
- Function name must exist in module
- Parameter count must match function signature
- Recursion depth must not exceed 1000
- Operation count must not exceed configured limit

**Relationships**:
- Uses `Module` from nx-hir as input
- Creates and manages `ExecutionContext`
- Returns `Value` or `RuntimeError`

---

### 2. ExecutionContext

**Purpose**: Runtime state including variable bindings, call stack, and resource limits

**Fields**:
```rust
pub struct ExecutionContext {
    /// Stack of variable scopes (innermost scope at end)
    scopes: Vec<HashMap<String, Value>>,
    
    /// Call stack for function invocations
    call_stack: Vec<CallFrame>,
    
    /// Counter for operations executed
    operation_count: usize,
    
    /// Maximum operations allowed (default: 1_000_000)
    operation_limit: usize,
    
    /// Maximum recursion depth (default: 1000)
    recursion_limit: usize,
}
```

**Methods**:
```rust
pub fn new() -> Self
pub fn with_limits(operation_limit: usize, recursion_limit: usize) -> Self
pub fn push_scope(&mut self)
pub fn pop_scope(&mut self)
pub fn define_variable(&mut self, name: String, value: Value)
pub fn lookup_variable(&self, name: &str) -> Result<Value, RuntimeError>
pub fn update_variable(&mut self, name: &str, value: Value) -> Result<(), RuntimeError>
pub fn push_call_frame(&mut self, frame: CallFrame) -> Result<(), RuntimeError>
pub fn pop_call_frame(&mut self)
pub fn check_operation_limit(&mut self) -> Result<(), RuntimeError>
pub fn current_depth(&self) -> usize
```

**Validation Rules**:
- Variable lookup must find name in scope stack (innermost to outermost)
- Call stack depth must not exceed `recursion_limit`
- Operation count must not exceed `operation_limit`
- Cannot pop scope if only one scope remains (global scope)

**State Transitions**:
1. New → Ready (after construction)
2. Ready → Executing (when eval_expr/eval_stmt called)
3. Executing → Ready (after successful completion)
4. Executing → Error (on RuntimeError)

---

### 3. Value

**Purpose**: Runtime representation of NX values with type information

**Definition**:
```rust
pub enum Value {
    /// Integer value
    Int(i64),
    
    /// Floating-point value
    Float(f64),
    
    /// String value
    String(String),
    
    /// Boolean value
    Boolean(bool),
    
    /// Null/undefined value
    Null,
    
    /// Array value (for future expansion)
    Array(Vec<Value>),
    
    /// Function value (for higher-order functions in future)
    Function {
        params: Vec<String>,
        body: ExprId,
    },
}
```

**Methods**:
```rust
pub fn type_name(&self) -> &'static str
pub fn is_truthy(&self) -> bool
pub fn to_string(&self) -> String
pub fn to_int(&self) -> Result<i64, RuntimeError>
pub fn to_float(&self) -> Result<f64, RuntimeError>
pub fn to_bool(&self) -> Result<bool, RuntimeError>
```

**Validation Rules**:
- Type conversions must be explicit and safe
- Arithmetic operations only on Int/Float
- Logical operations only on Boolean
- String concatenation only on String
- Null cannot be used in arithmetic or logical operations (raises RuntimeError)

**Type Compatibility**:
- Int ⟷ Float: Allowed with explicit conversion
- String + any type: Converts to string and concatenates
- Boolean operations: Only on Boolean values
- Comparison: Same types only (int/int, float/float, string/string, bool/bool)

---

### 4. RuntimeError

**Purpose**: Represents runtime execution errors with source location context

**Definition**:
```rust
pub struct RuntimeError {
    /// Error category
    kind: RuntimeErrorKind,
    
    /// Human-readable error message
    message: String,
    
    /// Source location where error occurred
    location: TextSpan,
    
    /// Call stack at time of error
    call_stack: Vec<CallFrame>,
}

pub enum RuntimeErrorKind {
    /// Division by zero
    DivisionByZero,
    
    /// Null value used in operation
    NullOperation,
    
    /// Variable not found in scope
    UndefinedVariable,
    
    /// Type mismatch at runtime
    TypeMismatch,
    
    /// Wrong number of parameters
    ParameterCountMismatch { expected: usize, got: usize },
    
    /// Stack overflow (recursion limit exceeded)
    StackOverflow,
    
    /// Infinite loop protection (operation limit exceeded)
    OperationLimitExceeded,
    
    /// Function missing return statement
    MissingReturn,
    
    /// Function not found
    FunctionNotFound,
    
    /// Array index out of bounds
    IndexOutOfBounds,
    
    /// Invalid operation
    InvalidOperation,
}
```

**Methods**:
```rust
pub fn new(kind: RuntimeErrorKind, message: String, location: TextSpan) -> Self
pub fn with_call_stack(mut self, call_stack: Vec<CallFrame>) -> Self
pub fn kind(&self) -> &RuntimeErrorKind
pub fn message(&self) -> &str
pub fn location(&self) -> TextSpan
pub fn format_error(&self, source: &str) -> String  // Uses nx-diagnostics/Ariadne
```

**Validation Rules**:
- Must have non-empty message
- Must have valid source location
- Call stack must be captured at error creation time

**Integration**:
- Uses `nx_diagnostics::TextSpan` for source locations
- Formats errors using Ariadne for beautiful output
- Integrates with existing diagnostic infrastructure

---

### 5. CallFrame

**Purpose**: Represents a single function call on the call stack

**Definition**:
```rust
pub struct CallFrame {
    /// Function name
    function_name: String,
    
    /// Source location of call site
    call_site: TextSpan,
    
    /// Parameters passed to function
    params: Vec<(String, Value)>,
}
```

**Methods**:
```rust
pub fn new(function_name: String, call_site: TextSpan) -> Self
pub fn with_params(mut self, params: Vec<(String, Value)>) -> Self
pub fn function_name(&self) -> &str
pub fn call_site(&self) -> TextSpan
```

**Validation Rules**:
- Function name must be non-empty
- Call site must be valid source location

---

## Entity Relationships

```text
┌─────────────┐
│ Interpreter │ (stateless execution engine)
└──────┬──────┘
       │ uses
       ├────────────────┐
       │                │
       ▼                ▼
┌──────────┐     ┌────────────────┐
│  Module  │     │ExecutionContext│
│ (nx-hir) │     │ (runtime state)│
└──────────┘     └────────┬───────┘
                          │ contains
                          ├──────────────┬─────────────┐
                          ▼              ▼             ▼
                   ┌──────────┐   ┌───────────┐ ┌──────────┐
                   │  Value   │   │CallFrame  │ │HashMap   │
                   │(variants)│   │(call info)│ │(vars)    │
                   └──────────┘   └───────────┘ └──────────┘

Errors: RuntimeError ← produced by any eval operation
```

### Data Flow

1. **Function Execution**:
   ```
   User → Interpreter.execute_function(module, name, args)
        → Create ExecutionContext
        → Find function in Module
        → Validate parameter count
        → Bind parameters to new scope
        → Eval function body
        → Return Value or RuntimeError
   ```

2. **Expression Evaluation**:
   ```
   eval_expr(expr, module, ctx)
        → Check operation limit
        → Match on expr type:
           - Literal → Convert to Value
           - Ident → Lookup in context
           - BinaryOp → Eval lhs, eval rhs, apply operation
           - Call → Recursive execute_function
           - If → Eval condition, choose branch
           - Block → Eval statements, return expr
        → Return Value or RuntimeError
   ```

3. **Error Propagation**:
   ```
   RuntimeError created
        → Capture call stack from context
        → Capture source location from HIR
        → Propagate up via Result<T, RuntimeError>
        → Format using nx-diagnostics at top level
   ```

## Performance Considerations

- **Value Cloning**: Values are cloned when stored/retrieved from context. Keep values small.
- **Scope Stack**: Linear search from innermost to outermost. Typical depth: 3-5 levels.
- **Operation Counting**: O(1) increment per operation. Negligible overhead.
- **Call Stack**: Vec push/pop. O(1) amortized. Max depth 1000 limits memory.
- **Expression Arena**: Module owns arena. No allocation during interpretation.

## Security Considerations

- **Resource Limits**:
  - Recursion depth: 1000 (prevents stack overflow)
  - Operation count: 1M (prevents infinite loops)
  - Both limits are configurable for testing

- **Memory Safety**:
  - No unsafe code in interpreter
  - All operations type-checked
  - Bounds checking on array access

- **Error Handling**:
  - All errors return Result, never panic
  - Comprehensive error messages
  - No information leakage in error messages
