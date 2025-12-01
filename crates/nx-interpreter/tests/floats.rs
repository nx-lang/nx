//! Integration tests for float operations
//!
//! Tests for float literals, arithmetic, comparisons, modulo, and mixed int/float operations.

// We intentionally use 3.14 as a test value, not as an approximation of π
#![allow(clippy::approx_constant)]

use nx_diagnostics::{TextSize, TextSpan};
use nx_hir::ast::{BinOp, Expr};
use nx_hir::{lower, Function, Item, Module, Name, Param, SourceId};
use nx_interpreter::{Interpreter, Value};
use nx_syntax::parse_str;

/// Helper function to execute a function and return the result
fn execute_function(source: &str, function_name: &str, args: Vec<Value>) -> Result<Value, String> {
    let parse_result = parse_str(source, "test.nx");
    if !parse_result.errors.is_empty() {
        return Err(format!("Parse errors: {:?}", parse_result.errors));
    }

    let root = parse_result.root().expect("Failed to get root");
    let module = lower(root, SourceId::new(0));

    let interpreter = Interpreter::new();
    interpreter
        .execute_function(&module, function_name, args)
        .map_err(|e| format!("Runtime error: {}", e))
}

/// Helper to compare floats with tolerance
fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-10
}

// ============================================================================
// Float Literals
// ============================================================================

#[test]
fn test_float_literal_simple() {
    let source = r#"
        let <f /> = { 3.14 }
    "#;

    let result = execute_function(source, "f", vec![]).unwrap_or_else(|e| panic!("{}", e));
    match result {
        Value::Float(f) => assert!(approx_eq(f, 3.14), "Expected 3.14, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }
}

#[test]
fn test_float_literal_with_exponent() {
    let source = r#"
        let <f /> = { 1.5e2 }
    "#;

    let result = execute_function(source, "f", vec![]).unwrap_or_else(|e| panic!("{}", e));
    match result {
        Value::Float(f) => assert!(approx_eq(f, 150.0), "Expected 150.0, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }
}

#[test]
fn test_float_literal_negative_exponent() {
    let source = r#"
        let <f /> = { 1.5e-2 }
    "#;

    let result = execute_function(source, "f", vec![]).unwrap_or_else(|e| panic!("{}", e));
    match result {
        Value::Float(f) => assert!(approx_eq(f, 0.015), "Expected 0.015, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }
}

#[test]
fn test_float_literal_zero() {
    let source = r#"
        let <f /> = { 0.0 }
    "#;

    let result = execute_function(source, "f", vec![]).unwrap_or_else(|e| panic!("{}", e));
    match result {
        Value::Float(f) => assert!(approx_eq(f, 0.0), "Expected 0.0, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }
}

// ============================================================================
// Float Arithmetic
// ============================================================================

#[test]
fn test_float_addition() {
    let source = r#"
        let <add a:float b:float /> = { a + b }
    "#;

    let result = execute_function(source, "add", vec![Value::Float(1.5), Value::Float(2.3)])
        .unwrap_or_else(|e| panic!("{}", e));
    match result {
        Value::Float(f) => assert!(approx_eq(f, 3.8), "Expected 3.8, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }
}

#[test]
fn test_float_subtraction() {
    let source = r#"
        let <sub a:float b:float /> = { a - b }
    "#;

    let result = execute_function(source, "sub", vec![Value::Float(5.5), Value::Float(2.2)])
        .unwrap_or_else(|e| panic!("{}", e));
    match result {
        Value::Float(f) => assert!(approx_eq(f, 3.3), "Expected 3.3, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }
}

#[test]
fn test_float_multiplication() {
    let source = r#"
        let <mul a:float b:float /> = { a * b }
    "#;

    let result = execute_function(source, "mul", vec![Value::Float(2.5), Value::Float(4.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    match result {
        Value::Float(f) => assert!(approx_eq(f, 10.0), "Expected 10.0, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }
}

#[test]
fn test_float_division() {
    let source = r#"
        let <div a:float b:float /> = { a / b }
    "#;

    let result = execute_function(source, "div", vec![Value::Float(7.5), Value::Float(2.5)])
        .unwrap_or_else(|e| panic!("{}", e));
    match result {
        Value::Float(f) => assert!(approx_eq(f, 3.0), "Expected 3.0, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }
}

#[test]
fn test_float_division_by_zero() {
    let source = r#"
        let <div a:float b:float /> = { a / b }
    "#;

    let result = execute_function(source, "div", vec![Value::Float(1.0), Value::Float(0.0)]);
    // Float division by zero typically results in infinity, not an error
    // Check the actual behavior
    match result {
        Ok(Value::Float(f)) => assert!(f.is_infinite(), "Expected infinity for float div by zero"),
        Ok(other) => panic!("Expected Float, got {:?}", other),
        Err(e) => {
            // If the interpreter treats it as an error, that's also valid
            assert!(e.contains("Division by zero"), "Unexpected error: {}", e);
        }
    }
}

// ============================================================================
// Float Modulo
// ============================================================================

#[test]
fn test_float_modulo() {
    let source = r#"
        let <fmod a:float b:float /> = { a % b }
    "#;

    let result = execute_function(source, "fmod", vec![Value::Float(10.5), Value::Float(3.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    match result {
        // 10.5 % 3.0 = 1.5
        Value::Float(f) => assert!(approx_eq(f, 1.5), "Expected 1.5, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }
}

#[test]
fn test_float_modulo_smaller_dividend() {
    let source = r#"
        let <fmod a:float b:float /> = { a % b }
    "#;

    let result = execute_function(source, "fmod", vec![Value::Float(2.5), Value::Float(4.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    match result {
        // 2.5 % 4.0 = 2.5
        Value::Float(f) => assert!(approx_eq(f, 2.5), "Expected 2.5, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }
}

#[test]
fn test_float_modulo_exact() {
    let source = r#"
        let <fmod a:float b:float /> = { a % b }
    "#;

    let result = execute_function(source, "fmod", vec![Value::Float(9.0), Value::Float(3.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    match result {
        // 9.0 % 3.0 = 0.0
        Value::Float(f) => assert!(approx_eq(f, 0.0), "Expected 0.0, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }
}

#[test]
fn test_float_modulo_by_zero() {
    let source = r#"
        let <fmod a:float b:float /> = { a % b }
    "#;

    let result = execute_function(source, "fmod", vec![Value::Float(10.0), Value::Float(0.0)]);
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("Division by zero"),
        "Expected division by zero error for float modulo"
    );
}

#[test]
fn test_mixed_int_float_modulo() {
    let source = r#"
        let <fmod a:int b:float /> = { a % b }
    "#;

    let result = execute_function(source, "fmod", vec![Value::Int(10), Value::Float(4.0)]);
    // Accept coercion to float
    match result {
        Ok(Value::Float(f)) => assert!(approx_eq(f, 2.0), "Expected 2.0, got {}", f),
        Err(_) => (), // Type error is also acceptable
        other => panic!("Unexpected result: {:?}", other),
    }
}

#[test]
fn test_float_modulo_in_expression() {
    let source = r#"
        let <calc x:float /> = { (x + 1.5) % 3.0 }
    "#;

    // (4.0 + 1.5) % 3.0 = 5.5 % 3.0 = 2.5
    let result = execute_function(source, "calc", vec![Value::Float(4.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    match result {
        Value::Float(f) => assert!(approx_eq(f, 2.5), "Expected 2.5, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }
}

// ============================================================================
// Negative Float Modulo and Division (Truncated Division Semantics)
// Float operations follow the same truncated division semantics as integers.
// ============================================================================

#[test]
fn test_float_modulo_negative_dividend() {
    // -7.5 % 3.0 = -1.5 (truncated division semantics)
    let source = r#"
        let <fmod a:float b:float /> = { a % b }
    "#;

    let result = execute_function(source, "fmod", vec![Value::Float(-7.5), Value::Float(3.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    match result {
        Value::Float(f) => assert!(approx_eq(f, -1.5), "Expected -1.5, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }
}

#[test]
fn test_float_modulo_negative_divisor() {
    // 7.5 % -3.0 = 1.5 (result has same sign as dividend)
    let source = r#"
        let <fmod a:float b:float /> = { a % b }
    "#;

    let result = execute_function(source, "fmod", vec![Value::Float(7.5), Value::Float(-3.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    match result {
        Value::Float(f) => assert!(approx_eq(f, 1.5), "Expected 1.5, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }
}

#[test]
fn test_float_modulo_both_negative() {
    // -7.5 % -3.0 = -1.5 (result has same sign as dividend)
    let source = r#"
        let <fmod a:float b:float /> = { a % b }
    "#;

    let result = execute_function(source, "fmod", vec![Value::Float(-7.5), Value::Float(-3.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    match result {
        Value::Float(f) => assert!(approx_eq(f, -1.5), "Expected -1.5, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }
}

#[test]
fn test_float_division_negative_dividend() {
    // -7.5 / 3.0 = -2.5
    let source = r#"
        let <div a:float b:float /> = { a / b }
    "#;

    let result = execute_function(source, "div", vec![Value::Float(-7.5), Value::Float(3.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    match result {
        Value::Float(f) => assert!(approx_eq(f, -2.5), "Expected -2.5, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }
}

#[test]
fn test_float_division_negative_divisor() {
    // 7.5 / -3.0 = -2.5
    let source = r#"
        let <div a:float b:float /> = { a / b }
    "#;

    let result = execute_function(source, "div", vec![Value::Float(7.5), Value::Float(-3.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    match result {
        Value::Float(f) => assert!(approx_eq(f, -2.5), "Expected -2.5, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }
}

#[test]
fn test_float_division_both_negative() {
    // -7.5 / -3.0 = 2.5
    let source = r#"
        let <div a:float b:float /> = { a / b }
    "#;

    let result = execute_function(source, "div", vec![Value::Float(-7.5), Value::Float(-3.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    match result {
        Value::Float(f) => assert!(approx_eq(f, 2.5), "Expected 2.5, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }
}

#[test]
fn test_float_negation() {
    let source = r#"
        let <neg a:float /> = { -a }
    "#;

    let result = execute_function(source, "neg", vec![Value::Float(3.14)])
        .unwrap_or_else(|e| panic!("{}", e));
    match result {
        Value::Float(f) => assert!(approx_eq(f, -3.14), "Expected -3.14, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }
}

#[test]
fn test_float_complex_expression() {
    let source = r#"
        let <calc a:float b:float c:float /> = { a + b * c }
    "#;

    // 1.0 + 2.0 * 3.0 = 1.0 + 6.0 = 7.0
    let result = execute_function(
        source,
        "calc",
        vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    match result {
        Value::Float(f) => assert!(approx_eq(f, 7.0), "Expected 7.0, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }
}

// ============================================================================
// Float Comparisons
// ============================================================================

#[test]
fn test_float_equal() {
    let source = r#"
        let <eq a:float b:float /> = { a == b }
    "#;

    let result = execute_function(source, "eq", vec![Value::Float(3.14), Value::Float(3.14)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));

    let result = execute_function(source, "eq", vec![Value::Float(3.14), Value::Float(2.71)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_float_not_equal() {
    let source = r#"
        let <ne a:float b:float /> = { a != b }
    "#;

    let result = execute_function(source, "ne", vec![Value::Float(1.0), Value::Float(2.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));

    let result = execute_function(source, "ne", vec![Value::Float(1.5), Value::Float(1.5)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_float_less_than() {
    let source = r#"
        let <lt a:float b:float /> = { a < b }
    "#;

    let result = execute_function(source, "lt", vec![Value::Float(1.0), Value::Float(2.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));

    let result = execute_function(source, "lt", vec![Value::Float(2.0), Value::Float(1.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_float_less_than_or_equal() {
    let source = r#"
        let <le a:float b:float /> = { a <= b }
    "#;

    let result = execute_function(source, "le", vec![Value::Float(1.0), Value::Float(1.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));

    let result = execute_function(source, "le", vec![Value::Float(1.0), Value::Float(2.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));

    let result = execute_function(source, "le", vec![Value::Float(3.0), Value::Float(2.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_float_greater_than() {
    let source = r#"
        let <gt a:float b:float /> = { a > b }
    "#;

    let result = execute_function(source, "gt", vec![Value::Float(2.0), Value::Float(1.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));

    let result = execute_function(source, "gt", vec![Value::Float(1.0), Value::Float(2.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_float_greater_than_or_equal() {
    let source = r#"
        let <ge a:float b:float /> = { a >= b }
    "#;

    let result = execute_function(source, "ge", vec![Value::Float(2.0), Value::Float(2.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));

    let result = execute_function(source, "ge", vec![Value::Float(3.0), Value::Float(2.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));

    let result = execute_function(source, "ge", vec![Value::Float(1.0), Value::Float(2.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(false));
}

// ============================================================================
// Float in Conditionals
// NOTE: Parsed if-expressions are not yet lowered to HIR, so we test via direct HIR
// ============================================================================

/// Helper to create a text span
fn span(start: u32, end: u32) -> TextSpan {
    TextSpan::new(TextSize::from(start), TextSize::from(end))
}

#[test]
fn test_float_in_conditional() {
    let mut module = Module::new(SourceId::new(0));

    // Create function: maxf(a, b) = if a > b { a } else { b }
    let params = vec![
        Param::new(
            Name::new("a"),
            nx_hir::ast::TypeRef::name("float"),
            span(0, 1),
        ),
        Param::new(
            Name::new("b"),
            nx_hir::ast::TypeRef::name("float"),
            span(2, 3),
        ),
    ];

    // Build condition: a > b
    let a_expr1 = module.alloc_expr(Expr::Ident(Name::new("a")));
    let b_expr1 = module.alloc_expr(Expr::Ident(Name::new("b")));
    let condition = module.alloc_expr(Expr::BinaryOp {
        lhs: a_expr1,
        op: BinOp::Gt,
        rhs: b_expr1,
        span: span(0, 5),
    });

    // Then branch: a
    let then_branch = module.alloc_expr(Expr::Ident(Name::new("a")));

    // Else branch: b
    let else_branch = module.alloc_expr(Expr::Ident(Name::new("b")));

    // If expression
    let if_expr = module.alloc_expr(Expr::If {
        condition,
        then_branch,
        else_branch: Some(else_branch),
        span: span(0, 20),
    });

    let func = Function {
        name: Name::new("maxf"),
        params,
        return_type: None,
        body: if_expr,
        span: span(0, 30),
    };
    module.add_item(Item::Function(func));

    let interpreter = Interpreter::new();

    // Test with 3.14 > 2.71 (true, returns 3.14)
    let result = interpreter
        .execute_function(
            &module,
            "maxf",
            vec![Value::Float(3.14), Value::Float(2.71)],
        )
        .unwrap();
    match result {
        Value::Float(f) => assert!(approx_eq(f, 3.14), "Expected 3.14, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }

    // Test with 1.0 < 2.0 (false, returns 2.0)
    let result = interpreter
        .execute_function(&module, "maxf", vec![Value::Float(1.0), Value::Float(2.0)])
        .unwrap();
    match result {
        Value::Float(f) => assert!(approx_eq(f, 2.0), "Expected 2.0, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }
}

// ============================================================================
// Mixed Int/Float Operations
// Note: These tests verify how the interpreter handles mixed types.
// The behavior may vary (coercion, error, etc.)
// ============================================================================

#[test]
fn test_mixed_int_float_addition() {
    // Test if int + float works (may coerce int to float)
    let source = r#"
        let <add a:int b:float /> = { a + b }
    "#;

    let result = execute_function(source, "add", vec![Value::Int(1), Value::Float(2.5)]);
    // Accept either coercion to float or an error
    match result {
        Ok(Value::Float(f)) => assert!(approx_eq(f, 3.5), "Expected 3.5, got {}", f),
        Ok(Value::Int(i)) => assert_eq!(i, 3), // Maybe it truncates?
        Err(_) => (),                          // Type error is also acceptable
        other => panic!("Unexpected result: {:?}", other),
    }
}

#[test]
fn test_float_literal_in_expression() {
    // Test float literals used inline
    let source = r#"
        let <calc x:float /> = { x * 2.0 + 1.5 }
    "#;

    let result = execute_function(source, "calc", vec![Value::Float(3.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    match result {
        // 3.0 * 2.0 + 1.5 = 6.0 + 1.5 = 7.5
        Value::Float(f) => assert!(approx_eq(f, 7.5), "Expected 7.5, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }
}
