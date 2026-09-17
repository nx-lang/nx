//! Integration tests for float operations
//!
//! Tests for float literals, arithmetic, comparisons, modulo, and mixed int/float operations.

// We intentionally use 3.14 as a test value, not as an approximation of π
#![allow(clippy::approx_constant)]

use nx_diagnostics::{TextSize, TextSpan};
use nx_hir::ast::{BinOp, Expr};
use nx_hir::{lower, Function, Item, LoweredModule, Name, Param, SourceId};
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
        let <add a:float64 b:float64 /> = { a + b }
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
        let <sub a:float64 b:float64 /> = { a - b }
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
        let <mul a:float64 b:float64 /> = { a * b }
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
        let <div a:float64 b:float64 /> = { a / b }
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
        let <div a:float64 b:float64 /> = { a / b }
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
        let <fmod a:float64 b:float64 /> = { a % b }
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
        let <fmod a:float64 b:float64 /> = { a % b }
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
        let <fmod a:float64 b:float64 /> = { a % b }
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
        let <fmod a:float64 b:float64 /> = { a % b }
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
        let <fmod a:int b:float64 /> = { a % b }
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
        let <calc x:float64 /> = { (x + 1.5) % 3.0 }
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
        let <fmod a:float64 b:float64 /> = { a % b }
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
        let <fmod a:float64 b:float64 /> = { a % b }
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
        let <fmod a:float64 b:float64 /> = { a % b }
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
        let <div a:float64 b:float64 /> = { a / b }
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
        let <div a:float64 b:float64 /> = { a / b }
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
        let <div a:float64 b:float64 /> = { a / b }
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
        let <neg a:float64 /> = { -a }
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
        let <calc a:float64 b:float64 c:float64 /> = { a + b * c }
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
        let <eq a:float64 b:float64 /> = { a == b }
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
        let <ne a:float64 b:float64 /> = { a != b }
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
        let <lt a:float64 b:float64 /> = { a < b }
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
        let <le a:float64 b:float64 /> = { a <= b }
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
        let <gt a:float64 b:float64 /> = { a > b }
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
        let <ge a:float64 b:float64 /> = { a >= b }
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
    let mut module = LoweredModule::new(SourceId::new(0));

    // Create function: maxf(a, b) = if a > b { a } else { b }
    let params = vec![
        Param::new(
            Name::new("a"),
            nx_hir::ast::TypeRef::name("float64"),
            span(0, 1),
        ),
        Param::new(
            Name::new("b"),
            nx_hir::ast::TypeRef::name("float64"),
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
        visibility: nx_hir::Visibility::Export,
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
        let <add a:int b:float64 /> = { a + b }
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
        let <calc x:float64 /> = { x * 2.0 + 1.5 }
    "#;

    let result = execute_function(source, "calc", vec![Value::Float(3.0)])
        .unwrap_or_else(|e| panic!("{}", e));
    match result {
        // 3.0 * 2.0 + 1.5 = 6.0 + 1.5 = 7.5
        Value::Float(f) => assert!(approx_eq(f, 7.5), "Expected 7.5, got {}", f),
        other => panic!("Expected Float, got {:?}", other),
    }
}

// ============================================================================
// Implicit numeric widening
// ============================================================================

/// Executes a function of the module as type analysis leaves it.
///
/// <para>Whether a `+` concatenates, which operands are rendered as text, and the width a literal
/// takes are all decided by the type checker and written into the module, so a test of any of
/// them has to evaluate the analyzed module rather than the freshly lowered one.</para>
fn execute_checked(source: &str, function_name: &str, args: Vec<Value>) -> Result<Value, String> {
    let checked = nx_types::check_str(source, "test.nx");
    if !checked.errors().is_empty() {
        let messages: Vec<_> = checked
            .errors()
            .iter()
            .map(|diagnostic| diagnostic.message().to_string())
            .collect();
        return Err(format!("Type errors: {:?}", messages));
    }

    let module = checked.lowered_module.expect("lowered module");
    Interpreter::new()
        .execute_function(&module, function_name, args)
        .map_err(|e| format!("Runtime error: {}", e))
}

fn record_field(value: Value, field: &str) -> Value {
    match value {
        Value::Record { fields, .. } => fields
            .get(field)
            .cloned()
            .unwrap_or_else(|| panic!("no field {field}")),
        other => panic!("Expected a record, got {:?}", other),
    }
}

#[test]
fn test_int_plus_float64_is_float64() {
    let source = "let f(n:int, x:float64) = { n + x }";
    let result = execute_checked(source, "f", vec![Value::Int(1), Value::Float(1.5)]).unwrap();
    assert_eq!(result, Value::Float(2.5));
}

#[test]
fn test_int_compared_with_float64_compares_numerically() {
    let equal = "let f(n:int, x:float64) = { n == x }";
    let result = execute_checked(equal, "f", vec![Value::Int(2), Value::Float(2.0)]).unwrap();
    assert_eq!(result, Value::Boolean(true));

    let less = "let f(n:int, x:float64) = { n < x }";
    let result = execute_checked(less, "f", vec![Value::Int(2), Value::Float(2.5)]).unwrap();
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn test_integer_division_of_mixed_widths_stays_integer() {
    let source = "let f(n:int32, m:int) = { n / m }";
    let result = execute_checked(source, "f", vec![Value::Int32(7), Value::Int(2)]).unwrap();
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_an_int_bound_at_a_float64_site_evaluates_as_a_float() {
    let source = r#"
        type Box = { width:float64 }
        let f(n:int) = { <Box width={n} /> }
    "#;
    let result = execute_checked(source, "f", vec![Value::Int(3)]).unwrap();
    assert_eq!(record_field(result, "width"), Value::Float(3.0));
}

#[test]
fn test_an_int_bound_at_a_float64_component_property_evaluates_as_a_float() {
    let source = r#"
        external component <B v:float64 />
        let f(n:int) = { <B v={n} /> }
    "#;
    let result = execute_checked(source, "f", vec![Value::Int(3)]).unwrap();
    assert_eq!(record_field(result, "v"), Value::Float(3.0));
}

#[test]
fn test_widening_applies_to_each_element_of_a_list() {
    let source = r#"
        type Series = { values:float64[] }
        let f(n:int, m:int32) = { <Series values={n m} /> }
    "#;
    let result = execute_checked(source, "f", vec![Value::Int(1), Value::Int32(2)]).unwrap();
    assert_eq!(
        record_field(result, "values"),
        Value::Array(vec![Value::Float(1.0), Value::Float(2.0)])
    );
}

#[test]
fn test_widening_applies_at_a_nullable_site() {
    let source = r#"
        type Box = { width:float64? }
        let f(n:int) = { <Box width={n} /> }
    "#;
    let result = execute_checked(source, "f", vec![Value::Int(3)]).unwrap();
    assert_eq!(record_field(result, "width"), Value::Float(3.0));
}

#[test]
fn test_a_host_number_takes_the_width_of_its_parameter() {
    // A host has only `int` and `float64` to pass, so an entry call's argument takes the width
    // its parameter declares, on the terms a literal written there would.
    let source = r#"
        let next(n:int32) = { n + 1 }
        let twice(w:float32) = { w * 2 }
        let halves(ws:float32?[]) = { ws }
    "#;
    assert_eq!(
        execute_checked(source, "next", vec![Value::Int(7)]).unwrap(),
        Value::Int32(8)
    );
    assert_eq!(
        execute_checked(source, "twice", vec![Value::Float(1.5)]).unwrap(),
        Value::Float32(3.0)
    );
    assert_eq!(
        execute_checked(source, "twice", vec![Value::Int(2)]).unwrap(),
        Value::Float32(4.0)
    );
    let items = Value::Array(vec![Value::Float(0.5), Value::Null]);
    assert_eq!(
        execute_checked(source, "halves", vec![items]).unwrap(),
        Value::Array(vec![Value::Float32(0.5), Value::Null])
    );

    let too_big = execute_checked(source, "next", vec![Value::Int(3_000_000_000)]).unwrap_err();
    assert!(too_big.contains("out of range for int32"), "{too_big}");
    let inexact = execute_checked(source, "twice", vec![Value::Int(16_777_217)]).unwrap_err();
    assert!(inexact.contains("not exact as a float32"), "{inexact}");
}

#[test]
fn test_a_nullable_int_widens_at_a_nullable_float64_site() {
    let source = r#"
        let widen(n:int?): float64? = { n }
        let widen_all(ns:int?[]): float64?[] = { ns }
    "#;
    assert_eq!(
        execute_checked(source, "widen", vec![Value::Int(3)]).unwrap(),
        Value::Float(3.0)
    );
    assert_eq!(
        execute_checked(source, "widen", vec![Value::Null]).unwrap(),
        Value::Null
    );
    let items = Value::Array(vec![Value::Int(3), Value::Null]);
    assert_eq!(
        execute_checked(source, "widen_all", vec![items]).unwrap(),
        Value::Array(vec![Value::Float(3.0), Value::Null])
    );
}

#[test]
fn test_widening_applies_at_a_declared_return_type_and_a_typed_parameter() {
    let source = r#"
        let widen(n:int32): int = { n }
        let half(x:float64) = { x / 2 }
        let f(n:int) = { half(n) }
    "#;
    let widened = execute_checked(source, "widen", vec![Value::Int32(7)]).unwrap();
    assert_eq!(widened, Value::Int(7));

    let halved = execute_checked(source, "f", vec![Value::Int(3)]).unwrap();
    assert_eq!(halved, Value::Float(1.5));
}

#[test]
fn test_an_int_branch_joined_with_a_float64_branch_evaluates_as_a_float() {
    // The join is a `float64`, so dividing it by an `int` divides as floats.
    let source = r#"
        let pick(b:boolean, n:int, x:float64) = { if b { n } else { x } }
        let f(b:boolean, n:int, x:float64, d:int) = { pick(b, n, x) / d }
    "#;
    let args = vec![
        Value::Boolean(true),
        Value::Int(3),
        Value::Float(1.5),
        Value::Int(2),
    ];
    assert_eq!(
        execute_checked(source, "f", args).unwrap(),
        Value::Float(1.5)
    );
}

#[test]
fn test_an_int_element_of_a_float64_list_evaluates_as_a_float() {
    let source = "let f(n:int, x:float64) = { n x }";
    let result = execute_checked(source, "f", vec![Value::Int(7), Value::Float(1.5)]).unwrap();
    assert_eq!(
        result,
        Value::Array(vec![Value::Float(7.0), Value::Float(1.5)])
    );
}

#[test]
fn test_a_narrower_match_arm_evaluates_as_the_join() {
    let source = r#"
        let arm(k:int, m:int32, n:int, x:float64) = {
          if k is {
            1 => m
            2 => n
            else => x
          }
        }
        let f(k:int, d:int) = { arm(k, 3, 3, 1.5) / d }
    "#;
    let run = |k: i64| execute_checked(source, "f", vec![Value::Int(k), Value::Int(2)]).unwrap();
    assert_eq!(run(1), Value::Float(1.5));
    assert_eq!(run(2), Value::Float(1.5));
    assert_eq!(run(3), Value::Float(0.75));
}

#[test]
fn test_a_joined_list_or_nullable_branch_widens_item_by_item() {
    let lists = r#"
        let ints(n:int) = { n n }
        let floats(x:float64) = { x x }
        let f(b:boolean, n:int, x:float64) = { if b { ints(n) } else { floats(x) } }
    "#;
    let args = vec![Value::Boolean(true), Value::Int(3), Value::Float(1.5)];
    assert_eq!(
        execute_checked(lists, "f", args).unwrap(),
        Value::Array(vec![Value::Float(3.0), Value::Float(3.0)])
    );

    let nullables = "let f(b:boolean, n:int?, x:float64?) = { if b { n } else { x } }";
    let present = vec![Value::Boolean(true), Value::Int(3), Value::Null];
    assert_eq!(
        execute_checked(nullables, "f", present).unwrap(),
        Value::Float(3.0)
    );
    let absent = vec![Value::Boolean(true), Value::Null, Value::Float(1.5)];
    assert_eq!(
        execute_checked(nullables, "f", absent).unwrap(),
        Value::Null
    );
}

#[test]
fn test_a_branch_joined_with_a_nullable_float_through_null_evaluates_as_a_float() {
    let source = r#"
        let f(a:boolean, b:boolean, n:int, x:float64) = {
          if a { n } else { if b { x } else { null } }
        }
    "#;
    let run = |a: bool, b: bool| {
        let args = vec![
            Value::Boolean(a),
            Value::Boolean(b),
            Value::Int(3),
            Value::Float(1.5),
        ];
        execute_checked(source, "f", args).unwrap()
    };
    assert_eq!(run(true, false), Value::Float(3.0));
    assert_eq!(run(false, true), Value::Float(1.5));
    assert_eq!(run(false, false), Value::Null);
}

#[test]
fn test_a_join_of_one_integer_width_is_left_alone() {
    let source = r#"
        let pick(b:boolean, n:int, m:int) = { if b { n } else { m } }
        let f(n:int, m:int, d:int) = { pick(true, n, m) / d }
    "#;
    let args = vec![Value::Int(3), Value::Int(4), Value::Int(2)];
    assert_eq!(execute_checked(source, "f", args).unwrap(), Value::Int(1));
}

#[test]
fn test_a_literal_is_evaluated_at_the_width_its_site_chose() {
    let source = r#"
        type Sizes = { small:int32 ratio:float32 }
        let f() = { <Sizes small=1 ratio=1.5 /> }
    "#;
    let result = execute_checked(source, "f", vec![]).unwrap();
    assert_eq!(record_field(result.clone(), "small"), Value::Int32(1));
    assert_eq!(record_field(result, "ratio"), Value::Float32(1.5));
}

#[test]
fn test_a_constant_expression_takes_its_site_width_after_folding() {
    let source = r#"
        type Sizes = { small:int32 ratio:float32 half:float32 wide:float64 }
        let f() = { <Sizes small={2 + 3} ratio={1.5 * 2} half={7 / 2} wide={7 / 2} /> }
    "#;
    let result = execute_checked(source, "f", vec![]).unwrap();
    assert_eq!(record_field(result.clone(), "small"), Value::Int32(5));
    assert_eq!(record_field(result.clone(), "ratio"), Value::Float32(3.0));
    // `7 / 2` is integer division wherever it is written; only the result takes the site's width.
    assert_eq!(record_field(result.clone(), "half"), Value::Float32(3.0));
    assert_eq!(record_field(result, "wide"), Value::Float(3.0));
}

#[test]
fn test_a_constant_operand_takes_the_other_operands_width() {
    let int32 = "let f(n:int32) = { n * (2 + 3) }";
    assert_eq!(
        execute_checked(int32, "f", vec![Value::Int32(4)]).unwrap(),
        Value::Int32(20)
    );

    let float32 = "let f(w:float32) = { w * -(0.5 * 3) }";
    assert_eq!(
        execute_checked(float32, "f", vec![Value::Float32(2.0)]).unwrap(),
        Value::Float32(-3.0)
    );
}

#[test]
fn test_a_folded_real_is_rounded_once_at_its_site() {
    let source = "let f(): float32 = { 0.1 * 3 }";
    assert_eq!(
        execute_checked(source, "f", vec![]).unwrap(),
        Value::Float32((0.1_f64 * 3.0) as f32)
    );
}

#[test]
fn test_a_constant_expression_that_cannot_take_its_site_is_reported() {
    let cases = [
        (
            "let f(): int32 = { 1000000 * 3000 }",
            "3000000000 is out of range for int32",
        ),
        (
            "let f(): float32 = { 16777216 + 1 }",
            "16777217 is not exactly representable as float32",
        ),
        (
            "let f(): float32 = { 1 / 0 }",
            "the constant expression divides by zero",
        ),
        (
            "let f(): float64 = { 9223372036854775807 + 1 }",
            "the constant expression overflows int",
        ),
        (
            "let f(n:int32) = { n > 1000000 * 3000 }",
            "3000000000 is out of range for int32",
        ),
        // A real constant takes no integer type, folded or not.
        (
            "let f(): int32 = { 3.0 * 2 }",
            "expects int32, found float64",
        ),
    ];
    for (source, needle) in cases {
        let error = execute_checked(source, "f", vec![]).unwrap_err();
        assert!(error.contains(needle), "{source}: {error}");
    }
}

#[test]
fn test_a_constant_expression_without_a_narrow_site_evaluates_at_run_time() {
    // Nothing gives these a width, so they are left for evaluation, which reports the division.
    assert_eq!(
        execute_checked("let f() = { 7 / 2 }", "f", vec![]).unwrap(),
        Value::Int(3)
    );
    for source in [
        "let f() = { 1 / 0 }",
        "let f(): int = { 1 / 0 }",
        "let f(): float64 = { 1.0 / 0 }",
    ] {
        let error = execute_checked(source, "f", vec![]).unwrap_err();
        assert!(error.contains("Runtime error"), "{source}: {error}");
    }
}
