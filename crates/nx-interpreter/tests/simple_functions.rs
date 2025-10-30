//! Integration tests for simple function execution (User Story 1)
//!
//! These tests verify end-to-end execution of NX functions with basic operations.

use nx_hir::{lower, SourceId};
use nx_interpreter::{Interpreter, Value};
use nx_syntax::parse_str;
use smol_str::SmolStr;

/// Helper function to execute a function and return the result
fn execute_function(source: &str, function_name: &str, args: Vec<Value>) -> Result<Value, String> {
    // Parse the source code
    let parse_result = parse_str(source, "test.nx");
    if !parse_result.errors.is_empty() {
        return Err(format!("Parse errors: {:?}", parse_result.errors));
    }

    // Lower to HIR
    let root = parse_result.root().expect("Failed to get root");
    let module = lower(root, SourceId::new(0));

    // Execute the function
    let interpreter = Interpreter::new();
    interpreter
        .execute_function(&module, function_name, args)
        .map_err(|e| format!("Runtime error: {}", e))
}

// ============================================================================
// T023: Arithmetic Function Tests
// ============================================================================

#[test]
fn test_add_function() {
    let source = r#"
        let <add a:int b:int /> = { a + b }
    "#;

    let result = execute_function(source, "add", vec![Value::Int(5), Value::Int(3)]);
    assert!(result.is_ok(), "Function execution failed: {:?}", result);
    assert_eq!(result.unwrap(), Value::Int(8));
}

#[test]
fn test_subtract_function() {
    let source = r#"
        let <sub a:int b:int /> = { a - b }
    "#;

    let result = execute_function(source, "sub", vec![Value::Int(10), Value::Int(3)]);
    assert!(result.is_ok(), "Function execution failed: {:?}", result);
    assert_eq!(result.unwrap(), Value::Int(7));
}

#[test]
fn test_multiply_function() {
    let source = r#"
        let <mul a:int b:int /> = { a * b }
    "#;

    let result = execute_function(source, "mul", vec![Value::Int(4), Value::Int(5)]);
    assert!(result.is_ok(), "Function execution failed: {:?}", result);
    assert_eq!(result.unwrap(), Value::Int(20));
}

#[test]
fn test_divide_function() {
    let source = r#"
        let <div a:int b:int /> = { a / b }
    "#;

    let result = execute_function(source, "div", vec![Value::Int(15), Value::Int(3)]);
    assert!(result.is_ok(), "Function execution failed: {:?}", result);
    assert_eq!(result.unwrap(), Value::Int(5));
}

#[test]
fn test_mixed_arithmetic() {
    let source = r#"
        let <calc a:int b:int c:int /> = { a + b * c }
    "#;

    let result = execute_function(
        source,
        "calc",
        vec![Value::Int(2), Value::Int(3), Value::Int(4)],
    );
    assert!(result.is_ok(), "Function execution failed: {:?}", result);
    // 2 + (3 * 4) = 2 + 12 = 14
    assert_eq!(result.unwrap(), Value::Int(14));
}

// ============================================================================
// T024: String Concatenation Tests
// ============================================================================

#[test]
fn test_string_concat() {
    let source = r#"
        let <concat a:string b:string /> = { a + b }
    "#;

    let result = execute_function(
        source,
        "concat",
        vec![
            Value::String(SmolStr::new("hello")),
            Value::String(SmolStr::new(" world")),
        ],
    );
    assert!(result.is_ok(), "Function execution failed: {:?}", result);
    assert_eq!(result.unwrap(), Value::String(SmolStr::new("hello world")));
}

// ============================================================================
// T025: Variable Binding Tests
// ============================================================================

#[test]
fn test_variable_binding() {
    let source = r#"
        let <compute x:int /> = {
            let double = x + x
            double
        }
    "#;

    let result = execute_function(source, "compute", vec![Value::Int(5)]);
    assert!(result.is_ok(), "Function execution failed: {:?}", result);
    assert_eq!(result.unwrap(), Value::Int(10));
}

#[test]
fn test_multiple_variables() {
    let source = r#"
        let <calc a:int b:int /> = {
            let sum = a + b
            let product = a * b
            sum + product
        }
    "#;

    let result = execute_function(source, "calc", vec![Value::Int(3), Value::Int(4)]);
    assert!(result.is_ok(), "Function execution failed: {:?}", result);
    // sum = 7, product = 12, result = 19
    assert_eq!(result.unwrap(), Value::Int(19));
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_function_not_found() {
    let source = r#"
        let <add a:int b:int /> = { a + b }
    "#;

    let result = execute_function(source, "nonexistent", vec![]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Function not found"));
}

#[test]
fn test_parameter_count_mismatch() {
    let source = r#"
        let <add a:int b:int /> = { a + b }
    "#;

    let result = execute_function(source, "add", vec![Value::Int(5)]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("parameter"));
}

#[test]
fn test_division_by_zero() {
    let source = r#"
        let <div a:int b:int /> = { a / b }
    "#;

    let result = execute_function(source, "div", vec![Value::Int(10), Value::Int(0)]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Division by zero"));
}
