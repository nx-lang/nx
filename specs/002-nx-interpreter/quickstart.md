# Quickstart Guide: NX Interpreter

**Feature**: 002-nx-interpreter  
**Date**: 2025-10-29  
**Audience**: Developers integrating the NX interpreter

## Overview

The `nx-interpreter` crate provides a simple, production-ready interpreter for executing NX functions. It takes HIR (High-level Intermediate Representation) and executes it with provided parameters, returning computed values or runtime errors.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
nx-interpreter = { path = "../nx-interpreter" }
nx-hir = { path = "../nx-hir" }
nx-syntax = { path = "../nx-syntax" }
```

## Basic Usage

### Step 1: Parse NX Code

```rust
use nx_syntax::parse_str;
use nx_hir::lower;

let source = r#"
let <add a:int b:int /> = { a + b }
"#;

// Parse source to CST
let parse_result = parse_str(source, "example.nx");

// Lower CST to HIR
let module = lower(parse_result.root().unwrap(), parse_result.source_id);
```

### Step 2: Execute Function

```rust
use nx_interpreter::{Interpreter, Value};

// Create interpreter
let interpreter = Interpreter::new();

// Prepare arguments
let args = vec![Value::Int(5), Value::Int(3)];

// Execute function
match interpreter.execute_function(&module, "add", args) {
    Ok(result) => println!("Result: {}", result),  // Prints: Result: 8
    Err(err) => eprintln!("Error: {}", err),
}
```

## Common Patterns

### Pattern 1: Execute with Error Handling

```rust
use nx_interpreter::{Interpreter, Value, RuntimeError, RuntimeErrorKind};

fn run_function(
    module: &Module,
    name: &str,
    args: Vec<Value>
) -> Result<Value, RuntimeError> {
    let interpreter = Interpreter::new();
    interpreter.execute_function(module, name, args)
}

// Usage
match run_function(&module, "divide", vec![Value::Int(10), Value::Int(0)]) {
    Ok(value) => println!("Success: {}", value),
    Err(err) => {
        match err.kind() {
            RuntimeErrorKind::DivisionByZero => {
                eprintln!("Cannot divide by zero!");
            }
            RuntimeErrorKind::FunctionNotFound(name) => {
                eprintln!("Function '{}' not found", name);
            }
            _ => eprintln!("Runtime error: {}", err),
        }
    }
}
```

### Pattern 2: Custom Resource Limits

```rust
use nx_interpreter::{Interpreter, ResourceLimits};

// For testing: lower limits
let test_limits = ResourceLimits {
    recursion_limit: 100,
    operation_limit: 10_000,
};

let result = interpreter.execute_function_with_limits(
    &module,
    "factorial",
    vec![Value::Int(10)],
    test_limits,
);
```

### Pattern 3: Pretty Error Reporting

```rust
use nx_interpreter::{Interpreter, Value};
use nx_syntax::parse_str;

let source = "let <bad /> = { x + 1 }";  // 'x' is undefined
let parse_result = parse_str(source, "example.nx");
let module = lower(parse_result.root().unwrap(), parse_result.source_id);

let interpreter = Interpreter::new();
match interpreter.execute_function(&module, "bad", vec![]) {
    Ok(_) => {}
    Err(err) => {
        // Pretty-print with Ariadne
        let formatted = err.format(parse_result.source_id, source);
        eprintln!("{}", formatted);
        // Output:
        // Error: Undefined variable 'x'
        //   ┌─ example.nx:1:18
        //   │
        // 1 │ let <bad /> = { x + 1 }
        //   │                 ^ undefined variable
    }
}
```

### Pattern 4: Type Checking Before Execution

```rust
use nx_interpreter::{Interpreter, Value, RuntimeErrorKind};

// Values must match function parameter types
let args = vec![
    Value::Int(42),
    Value::String("hello".to_string()),  // Wrong type!
];

match interpreter.execute_function(&module, "add", args) {
    Err(err) if matches!(err.kind(), RuntimeErrorKind::TypeMismatch { .. }) => {
        eprintln!("Type error: {}", err);
    }
    _ => {}
}
```

## Value Types

### Creating Values

```rust
use nx_interpreter::Value;

let int_val = Value::Int(42);
let float_val = Value::Float(3.14);
let string_val = Value::String("hello".to_string());
let bool_val = Value::Boolean(true);
let null_val = Value::Null;
```

### Type Checking

```rust
match value {
    Value::Int(i) => println!("Integer: {}", i),
    Value::Float(f) => println!("Float: {}", f),
    Value::String(s) => println!("String: {}", s),
    Value::Boolean(b) => println!("Boolean: {}", b),
    Value::Null => println!("Null"),
}
```

### Value Operations

```rust
// Get type name
println!("Type: {}", value.type_name());

// Check truthiness
if value.is_truthy() {
    println!("Value is truthy");
}

// Convert to string
println!("Display: {}", value.to_string());
```

## Example Programs

### Example 1: Simple Arithmetic

```rust
let source = r#"
let <calculate x:int y:int /> = {
    let sum = x + y;
    let product = x * y;
    sum + product
}
"#;

let module = parse_and_lower(source);
let args = vec![Value::Int(3), Value::Int(4)];
let result = interpreter.execute_function(&module, "calculate", args)?;
// Result: Int(19)  // sum=7, product=12, total=19
```

### Example 2: Conditionals

```rust
let source = r#"
let <max a:int b:int /> = {
    if a > b {
        a
    } else {
        b
    }
}
"#;

let module = parse_and_lower(source);
let result = interpreter.execute_function(
    &module,
    "max",
    vec![Value::Int(10), Value::Int(5)],
)?;
// Result: Int(10)
```

### Example 3: Loops

```rust
let source = r#"
let <sum_to_n n:int /> = {
    let total = 0;
    let i = 1;
    while i <= n {
        total = total + i;
        i = i + 1;
    }
    total
}
"#;

let module = parse_and_lower(source);
let result = interpreter.execute_function(&module, "sum_to_n", vec![Value::Int(10)])?;
// Result: Int(55)  // 1+2+3+...+10
```

### Example 4: Recursion

```rust
let source = r#"
let <fibonacci n:int /> = {
    if n <= 1 {
        n
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
}
"#;

let module = parse_and_lower(source);
let result = interpreter.execute_function(&module, "fibonacci", vec![Value::Int(10)])?;
// Result: Int(55)
```

## Error Handling

### Common Runtime Errors

```rust
// 1. Function not found
let result = interpreter.execute_function(&module, "nonexistent", vec![]);
// Error: RuntimeErrorKind::FunctionNotFound("nonexistent")

// 2. Wrong parameter count
let result = interpreter.execute_function(&module, "add", vec![Value::Int(1)]);
// Error: RuntimeErrorKind::ParameterCountMismatch { expected: 2, got: 1 }

// 3. Division by zero
let source = "let <bad /> = { 10 / 0 }";
// Error: RuntimeErrorKind::DivisionByZero

// 4. Undefined variable
let source = "let <bad /> = { unknown_var }";
// Error: RuntimeErrorKind::UndefinedVariable("unknown_var")

// 5. Stack overflow
let source = "let <infinite /> = { infinite() }";
let limits = ResourceLimits { recursion_limit: 10, operation_limit: 1_000_000 };
// Error: RuntimeErrorKind::StackOverflow

// 6. Infinite loop
let source = "let <infinite /> = { while true { } }";
// Error: RuntimeErrorKind::OperationLimitExceeded
```

### Error Information

```rust
match interpreter.execute_function(&module, "bad", args) {
    Err(err) => {
        println!("Error kind: {:?}", err.kind());
        println!("Error message: {}", err.message());
        println!("Source location: {:?}", err.location());
        println!("Call stack depth: {}", err.call_stack().len());
        
        // Pretty format with source
        println!("{}", err.format(source_id, source));
    }
    Ok(_) => {}
}
```

## Performance Tips

### 1. Reuse Interpreter

```rust
// ✅ Good: Reuse interpreter instance
let interpreter = Interpreter::new();
for (name, args) in test_cases {
    let result = interpreter.execute_function(&module, name, args)?;
    // ...
}

// ❌ Bad: Create new interpreter each time
for (name, args) in test_cases {
    let interpreter = Interpreter::new();  // Unnecessary allocation
    // ...
}
```

### 2. Set Appropriate Limits

```rust
// For testing: Use lower limits
let test_limits = ResourceLimits::for_testing();

// For production: Use defaults or custom
let prod_limits = ResourceLimits::new();  // Default: 1000 recursion, 1M ops
```

### 3. Avoid Deep Recursion

```rust
// ✅ Good: Iterative version
let <sum_to_n_iter n:int /> = {
    let total = 0;
    let i = 1;
    while i <= n {
        total = total + i;
        i = i + 1;
    }
    total
}

// ⚠️ Slower: Recursive version
let <sum_to_n_rec n:int /> = {
    if n <= 0 { 0 } else { n + sum_to_n_rec(n - 1) }
}
```

## Testing

### Unit Testing Functions

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use nx_interpreter::{Interpreter, Value};

    #[test]
    fn test_add_function() {
        let source = "let <add a:int b:int /> = { a + b }";
        let module = parse_and_lower(source);
        let interpreter = Interpreter::new();
        
        let result = interpreter
            .execute_function(&module, "add", vec![Value::Int(5), Value::Int(3)])
            .unwrap();
        
        assert_eq!(result, Value::Int(8));
    }

    #[test]
    fn test_division_by_zero() {
        let source = "let <divide a:int b:int /> = { a / b }";
        let module = parse_and_lower(source);
        let interpreter = Interpreter::new();
        
        let result = interpreter.execute_function(
            &module,
            "divide",
            vec![Value::Int(10), Value::Int(0)],
        );
        
        assert!(matches!(
            result,
            Err(err) if matches!(err.kind(), RuntimeErrorKind::DivisionByZero)
        ));
    }
}
```

## Next Steps

- **Read the API Contract**: See `contracts/api.md` for full API reference
- **Review Data Model**: See `data-model.md` for entity details
- **Explore Examples**: See `examples/` directory for more code samples
- **Run Tests**: `cargo test -p nx-interpreter`

## Troubleshooting

### Problem: "Function not found"

**Solution**: Ensure the function is defined in the module before calling.

```rust
// Check if function exists
if module.find_item("my_function").is_some() {
    // Function exists, safe to call
}
```

### Problem: "Parameter count mismatch"

**Solution**: Verify the function signature matches argument count.

```rust
// Get function from module
if let Some(Item::Function(func)) = module.find_item("add") {
    println!("Function {} expects {} parameters", func.name, func.params.len());
}
```

### Problem: "Stack overflow on recursion"

**Solution**: Either increase recursion limit or refactor to iterative approach.

```rust
let limits = ResourceLimits {
    recursion_limit: 2000,  // Increase limit
    operation_limit: 1_000_000,
};
```

### Problem: "Operation limit exceeded"

**Solution**: Check for infinite loops or increase operation limit.

```rust
let limits = ResourceLimits {
    recursion_limit: 1000,
    operation_limit: 10_000_000,  // Increase limit
};
```

## Support

- **Documentation**: See `docs/` directory
- **Issues**: File on GitHub repository
- **Examples**: See `crates/nx-interpreter/tests/integration/`
