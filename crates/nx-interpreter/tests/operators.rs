//! Integration tests for operators
//!
//! Tests for operators including:
//! - Modulo (%)
//! - Not-equal (!=)
//! - Greater-or-equal (>=)
//! - Logical OR (||)
//! - Unary negation (-expr)
//! - Unary NOT (!expr)
//! - Short-circuit evaluation for && and ||
//! - Chained comparison/logical expressions
//!
//! All tests use source parsing, not direct HIR construction.

use nx_hir::{lower, SourceId};
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

// ============================================================================
// Modulo Operator (%)
// ============================================================================

#[test]
fn test_modulo_positive_numbers() {
    let source = r#"
        let mod(a:int, b:int): int = { a % b }
    "#;

    let result = execute_function(source, "mod", vec![Value::Int(17), Value::Int(5)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_modulo_exact_division() {
    let source = r#"
        let mod(a:int, b:int): int = { a % b }
    "#;

    let result = execute_function(source, "mod", vec![Value::Int(15), Value::Int(5)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(0));
}

#[test]
fn test_modulo_smaller_dividend() {
    let source = r#"
        let mod(a:int, b:int): int = { a % b }
    "#;

    let result = execute_function(source, "mod", vec![Value::Int(3), Value::Int(7)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_modulo_with_one() {
    let source = r#"
        let mod(a:int, b:int): int = { a % b }
    "#;

    let result = execute_function(source, "mod", vec![Value::Int(42), Value::Int(1)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(0));
}

#[test]
fn test_modulo_by_zero_error() {
    let source = r#"
        let mod(a:int, b:int): int = { a % b }
    "#;

    let result = execute_function(source, "mod", vec![Value::Int(10), Value::Int(0)]);
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("Division by zero"),
        "Expected division by zero error for modulo"
    );
}

#[test]
fn test_modulo_in_expression() {
    let source = r#"
        let is_even(x:int): bool = { x % 2 == 0 }
    "#;

    assert_eq!(
        execute_function(source, "is_even", vec![Value::Int(4)]).unwrap(),
        Value::Boolean(true)
    );

    assert_eq!(
        execute_function(source, "is_even", vec![Value::Int(7)]).unwrap(),
        Value::Boolean(false)
    );
}

#[test]
fn test_modulo_chained() {
    let source = r#"
        let double_mod(x:int): int = { x % 10 % 3 }
    "#;

    // 17 % 10 = 7, then 7 % 3 = 1
    assert_eq!(
        execute_function(source, "double_mod", vec![Value::Int(17)]).unwrap(),
        Value::Int(1)
    );
}

#[test]
fn test_modulo_with_arithmetic() {
    let source = r#"
        let calc(a:int, b:int): int = { (a + b) % 5 }
    "#;

    // (7 + 8) % 5 = 15 % 5 = 0
    assert_eq!(
        execute_function(source, "calc", vec![Value::Int(7), Value::Int(8)]).unwrap(),
        Value::Int(0)
    );

    // (7 + 3) % 5 = 10 % 5 = 0
    assert_eq!(
        execute_function(source, "calc", vec![Value::Int(7), Value::Int(3)]).unwrap(),
        Value::Int(0)
    );

    // (7 + 1) % 5 = 8 % 5 = 3
    assert_eq!(
        execute_function(source, "calc", vec![Value::Int(7), Value::Int(1)]).unwrap(),
        Value::Int(3)
    );
}

// ============================================================================
// Negative Number Modulo (Truncated Division Semantics)
// NX follows C-family languages (C#, JavaScript, Rust) where the result
// of modulo has the same sign as the dividend (left operand).
// This is "truncated division" semantics, NOT "floored division" (Python).
// ============================================================================

#[test]
fn test_modulo_negative_dividend() {
    // -7 % 3 = -1 (truncated division: -7 = -2*3 + (-1))
    // NOT -7 % 3 = 2 (floored division: -7 = -3*3 + 2)
    let source = r#"
        let mod(a:int, b:int): int = { a % b }
    "#;

    let result = execute_function(source, "mod", vec![Value::Int(-7), Value::Int(3)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(-1));
}

#[test]
fn test_modulo_negative_divisor() {
    // 7 % -3 = 1 (truncated division: 7 = -2*(-3) + 1)
    let source = r#"
        let mod(a:int, b:int): int = { a % b }
    "#;

    let result = execute_function(source, "mod", vec![Value::Int(7), Value::Int(-3)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_modulo_both_negative() {
    // -7 % -3 = -1 (truncated division: -7 = 2*(-3) + (-1))
    let source = r#"
        let mod(a:int, b:int): int = { a % b }
    "#;

    let result = execute_function(source, "mod", vec![Value::Int(-7), Value::Int(-3)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(-1));
}

#[test]
fn test_modulo_negative_exact_division() {
    // -9 % 3 = 0 (exact division)
    let source = r#"
        let mod(a:int, b:int): int = { a % b }
    "#;

    let result = execute_function(source, "mod", vec![Value::Int(-9), Value::Int(3)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(0));
}

#[test]
fn test_modulo_negative_small_dividend() {
    // -2 % 5 = -2 (dividend smaller than divisor in absolute value)
    let source = r#"
        let mod(a:int, b:int): int = { a % b }
    "#;

    let result = execute_function(source, "mod", vec![Value::Int(-2), Value::Int(5)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(-2));
}

// ============================================================================
// Negative Number Division (Truncated Toward Zero)
// NX follows C-family languages where integer division truncates toward zero.
// ============================================================================

#[test]
fn test_division_positive() {
    let source = r#"
        let div(a:int, b:int): int = { a / b }
    "#;

    // 7 / 3 = 2 (truncated)
    let result = execute_function(source, "div", vec![Value::Int(7), Value::Int(3)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_division_negative_dividend() {
    // -7 / 3 = -2 (truncated toward zero, not floored to -3)
    let source = r#"
        let div(a:int, b:int): int = { a / b }
    "#;

    let result = execute_function(source, "div", vec![Value::Int(-7), Value::Int(3)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(-2));
}

#[test]
fn test_division_negative_divisor() {
    // 7 / -3 = -2 (truncated toward zero)
    let source = r#"
        let div(a:int, b:int): int = { a / b }
    "#;

    let result = execute_function(source, "div", vec![Value::Int(7), Value::Int(-3)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(-2));
}

#[test]
fn test_division_both_negative() {
    // -7 / -3 = 2 (two negatives make positive, truncated)
    let source = r#"
        let div(a:int, b:int): int = { a / b }
    "#;

    let result = execute_function(source, "div", vec![Value::Int(-7), Value::Int(-3)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_division_by_zero_error() {
    let source = r#"
        let div(a:int, b:int): int = { a / b }
    "#;

    let result = execute_function(source, "div", vec![Value::Int(10), Value::Int(0)]);
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("Division by zero"),
        "Expected division by zero error"
    );
}

#[test]
fn test_division_exact() {
    let source = r#"
        let div(a:int, b:int): int = { a / b }
    "#;

    // -9 / 3 = -3 (exact)
    let result = execute_function(source, "div", vec![Value::Int(-9), Value::Int(3)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(-3));
}

#[test]
fn test_division_and_modulo_relationship() {
    // Verify: a == (a / b) * b + (a % b) for all combinations
    let source = r#"
        let check(a:int, b:int): bool = { a == (a / b) * b + (a % b) }
    "#;

    // Test various combinations
    let test_cases = vec![
        (7, 3),
        (-7, 3),
        (7, -3),
        (-7, -3),
        (10, 5),
        (-10, 5),
        (17, 4),
        (-17, 4),
    ];

    for (a, b) in test_cases {
        let result = execute_function(source, "check", vec![Value::Int(a), Value::Int(b)])
            .unwrap_or_else(|e| panic!("Failed for ({}, {}): {}", a, b, e));
        assert_eq!(
            result,
            Value::Boolean(true),
            "Division/modulo relationship failed for ({}, {})",
            a,
            b
        );
    }
}

// ============================================================================
// Not-Equal Operator (!=)
// ============================================================================

#[test]
fn test_not_equal_integers_different() {
    let source = r#"
        let <neq a:int b:int /> = { a != b }
    "#;

    let result = execute_function(source, "neq", vec![Value::Int(5), Value::Int(3)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn test_not_equal_integers_same() {
    let source = r#"
        let <neq a:int b:int /> = { a != b }
    "#;

    let result = execute_function(source, "neq", vec![Value::Int(7), Value::Int(7)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_not_equal_booleans() {
    let source = r#"
        let <neq a:bool b:bool /> = { a != b }
    "#;

    let result = execute_function(
        source,
        "neq",
        vec![Value::Boolean(true), Value::Boolean(false)],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn test_not_equal_strings() {
    let source = r#"
        let <neq a:string b:string /> = { a != b }
    "#;

    let result = execute_function(
        source,
        "neq",
        vec![Value::String("hello".into()), Value::String("world".into())],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn test_not_equal_strings_same() {
    let source = r#"
        let <neq a:string b:string /> = { a != b }
    "#;

    let result = execute_function(
        source,
        "neq",
        vec![Value::String("same".into()), Value::String("same".into())],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(false));
}

// ============================================================================
// Greater-or-Equal Operator (>=)
// ============================================================================

#[test]
fn test_greater_or_equal_greater() {
    let source = r#"
        let <gte a:int b:int /> = { a >= b }
    "#;

    let result = execute_function(source, "gte", vec![Value::Int(10), Value::Int(5)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn test_greater_or_equal_equal() {
    let source = r#"
        let <gte a:int b:int /> = { a >= b }
    "#;

    let result = execute_function(source, "gte", vec![Value::Int(7), Value::Int(7)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn test_greater_or_equal_less() {
    let source = r#"
        let <gte a:int b:int /> = { a >= b }
    "#;

    let result = execute_function(source, "gte", vec![Value::Int(3), Value::Int(8)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_greater_or_equal_negative_numbers() {
    let source = r#"
        let <gte a:int b:int /> = { a >= b }
    "#;

    let result = execute_function(source, "gte", vec![Value::Int(-5), Value::Int(-10)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));
}

// ============================================================================
// Logical OR Operator (||)
// ============================================================================

#[test]
fn test_logical_or_both_true() {
    let source = r#"
        let <lor a:bool b:bool /> = { a || b }
    "#;

    let result = execute_function(
        source,
        "lor",
        vec![Value::Boolean(true), Value::Boolean(true)],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn test_logical_or_first_true() {
    let source = r#"
        let <lor a:bool b:bool /> = { a || b }
    "#;

    let result = execute_function(
        source,
        "lor",
        vec![Value::Boolean(true), Value::Boolean(false)],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn test_logical_or_second_true() {
    let source = r#"
        let <lor a:bool b:bool /> = { a || b }
    "#;

    let result = execute_function(
        source,
        "lor",
        vec![Value::Boolean(false), Value::Boolean(true)],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn test_logical_or_both_false() {
    let source = r#"
        let <lor a:bool b:bool /> = { a || b }
    "#;

    let result = execute_function(
        source,
        "lor",
        vec![Value::Boolean(false), Value::Boolean(false)],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(false));
}

// ============================================================================
// Unary Negation (-expr)
// ============================================================================

#[test]
fn test_unary_negation_positive() {
    let source = r#"
        let <neg a:int /> = { -a }
    "#;

    let result =
        execute_function(source, "neg", vec![Value::Int(42)]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(-42));
}

#[test]
fn test_unary_negation_negative() {
    let source = r#"
        let <neg a:int /> = { -a }
    "#;

    let result =
        execute_function(source, "neg", vec![Value::Int(-15)]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(15));
}

#[test]
fn test_unary_negation_zero() {
    let source = r#"
        let <neg a:int /> = { -a }
    "#;

    let result =
        execute_function(source, "neg", vec![Value::Int(0)]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(0));
}

#[test]
fn test_unary_negation_in_expression() {
    let source = r#"
        let <calc a:int b:int /> = { a + -b }
    "#;

    let result = execute_function(source, "calc", vec![Value::Int(10), Value::Int(3)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(7));
}

#[test]
fn test_double_negation() {
    let source = r#"
        let <doubleneg a:int /> = { --a }
    "#;

    let result = execute_function(source, "doubleneg", vec![Value::Int(5)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(5));
}

// ============================================================================
// Unary NOT (!expr)
// ============================================================================

#[test]
fn test_unary_not_true() {
    let source = r#"
        let <not a:bool /> = { !a }
    "#;

    let result = execute_function(source, "not", vec![Value::Boolean(true)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_unary_not_false() {
    let source = r#"
        let <not a:bool /> = { !a }
    "#;

    let result = execute_function(source, "not", vec![Value::Boolean(false)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn test_double_not() {
    let source = r#"
        let <doublenot a:bool /> = { !!a }
    "#;

    let result = execute_function(source, "doublenot", vec![Value::Boolean(true)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));

    let result = execute_function(source, "doublenot", vec![Value::Boolean(false)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_triple_not() {
    let source = r#"
        let <triplenot a:bool /> = { !!!a }
    "#;

    let result = execute_function(source, "triplenot", vec![Value::Boolean(true)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(false));

    let result = execute_function(source, "triplenot", vec![Value::Boolean(false)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn test_not_with_comparison() {
    let source = r#"
        let <notgt a:int b:int /> = { !(a > b) }
    "#;

    // 5 > 3 is true, so !(5 > 3) is false
    let result = execute_function(source, "notgt", vec![Value::Int(5), Value::Int(3)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(false));

    // 2 > 3 is false, so !(2 > 3) is true
    let result = execute_function(source, "notgt", vec![Value::Int(2), Value::Int(3)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn test_not_with_equality() {
    let source = r#"
        let <noteq a:int b:int /> = { !(a == b) }
    "#;

    // 5 == 5 is true, so !(5 == 5) is false
    let result = execute_function(source, "noteq", vec![Value::Int(5), Value::Int(5)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(false));

    // 5 == 3 is false, so !(5 == 3) is true
    let result = execute_function(source, "noteq", vec![Value::Int(5), Value::Int(3)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn test_not_with_and() {
    let source = r#"
        let <notand a:bool b:bool /> = { !(a && b) }
    "#;

    // !(true && true) = false
    let result = execute_function(
        source,
        "notand",
        vec![Value::Boolean(true), Value::Boolean(true)],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(false));

    // !(true && false) = true
    let result = execute_function(
        source,
        "notand",
        vec![Value::Boolean(true), Value::Boolean(false)],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));

    // !(false && true) = true
    let result = execute_function(
        source,
        "notand",
        vec![Value::Boolean(false), Value::Boolean(true)],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn test_not_with_or() {
    let source = r#"
        let <notor a:bool b:bool /> = { !(a || b) }
    "#;

    // !(false || false) = true
    let result = execute_function(
        source,
        "notor",
        vec![Value::Boolean(false), Value::Boolean(false)],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));

    // !(true || false) = false
    let result = execute_function(
        source,
        "notor",
        vec![Value::Boolean(true), Value::Boolean(false)],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_not_in_complex_expression() {
    // Test De Morgan's law: !(a && b) == (!a || !b)
    let source = r#"
        let <demorgans a:bool b:bool /> = { !(a && b) == (!a || !b) }
    "#;

    // All combinations should return true
    let result = execute_function(
        source,
        "demorgans",
        vec![Value::Boolean(true), Value::Boolean(true)],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));

    let result = execute_function(
        source,
        "demorgans",
        vec![Value::Boolean(true), Value::Boolean(false)],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));

    let result = execute_function(
        source,
        "demorgans",
        vec![Value::Boolean(false), Value::Boolean(true)],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));

    let result = execute_function(
        source,
        "demorgans",
        vec![Value::Boolean(false), Value::Boolean(false)],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));
}

// ============================================================================
// Short-Circuit Evaluation
// ============================================================================

// Note: To properly test short-circuit evaluation, we would need side effects.
// Since NX is expression-based and pure, we test that the semantics are correct
// by ensuring the result matches expected short-circuit behavior.

#[test]
fn test_and_short_circuit_false_first() {
    // When first operand is false, second should not matter
    let source = r#"
        let <test a:bool b:bool /> = { a && b }
    "#;

    let result = execute_function(
        source,
        "test",
        vec![Value::Boolean(false), Value::Boolean(true)],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_or_short_circuit_true_first() {
    // When first operand is true, second should not matter
    let source = r#"
        let <test a:bool b:bool /> = { a || b }
    "#;

    let result = execute_function(
        source,
        "test",
        vec![Value::Boolean(true), Value::Boolean(false)],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));
}

// ============================================================================
// Chained and Complex Expressions
// ============================================================================

#[test]
fn test_chained_and() {
    let source = r#"
        let <chain a:bool b:bool c:bool /> = { a && b && c }
    "#;

    let result = execute_function(
        source,
        "chain",
        vec![
            Value::Boolean(true),
            Value::Boolean(true),
            Value::Boolean(true),
        ],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));

    let result = execute_function(
        source,
        "chain",
        vec![
            Value::Boolean(true),
            Value::Boolean(false),
            Value::Boolean(true),
        ],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_chained_or() {
    let source = r#"
        let <chain a:bool b:bool c:bool /> = { a || b || c }
    "#;

    let result = execute_function(
        source,
        "chain",
        vec![
            Value::Boolean(false),
            Value::Boolean(false),
            Value::Boolean(false),
        ],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(false));

    let result = execute_function(
        source,
        "chain",
        vec![
            Value::Boolean(false),
            Value::Boolean(true),
            Value::Boolean(false),
        ],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn test_mixed_and_or() {
    // a || b && c should be a || (b && c) due to precedence
    let source = r#"
        let <mixed a:bool b:bool c:bool /> = { a || b && c }
    "#;

    // false || (true && true) = false || true = true
    let result = execute_function(
        source,
        "mixed",
        vec![
            Value::Boolean(false),
            Value::Boolean(true),
            Value::Boolean(true),
        ],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));

    // false || (true && false) = false || false = false
    let result = execute_function(
        source,
        "mixed",
        vec![
            Value::Boolean(false),
            Value::Boolean(true),
            Value::Boolean(false),
        ],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_comparison_chain() {
    // Test chained comparisons in logical expression
    let source = r#"
        let <inrange x:int lo:int hi:int /> = { x >= lo && x <= hi }
    "#;

    // 5 in range [1, 10]
    let result = execute_function(
        source,
        "inrange",
        vec![Value::Int(5), Value::Int(1), Value::Int(10)],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));

    // 15 not in range [1, 10]
    let result = execute_function(
        source,
        "inrange",
        vec![Value::Int(15), Value::Int(1), Value::Int(10)],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_complex_boolean_expression() {
    // (a > b) || (c == d && e < f)
    let source = r#"
        let <complex a:int b:int c:int d:int e:int f:int /> = { 
            (a > b) || (c == d && e < f) 
        }
    "#;

    // (5 > 3) || (...) = true
    let result = execute_function(
        source,
        "complex",
        vec![
            Value::Int(5),
            Value::Int(3),
            Value::Int(1),
            Value::Int(2),
            Value::Int(1),
            Value::Int(2),
        ],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));

    // (1 > 3) || (2 == 2 && 1 < 5) = false || (true && true) = true
    let result = execute_function(
        source,
        "complex",
        vec![
            Value::Int(1),
            Value::Int(3),
            Value::Int(2),
            Value::Int(2),
            Value::Int(1),
            Value::Int(5),
        ],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));

    // (1 > 3) || (2 == 3 && 1 < 5) = false || (false && true) = false
    let result = execute_function(
        source,
        "complex",
        vec![
            Value::Int(1),
            Value::Int(3),
            Value::Int(2),
            Value::Int(3),
            Value::Int(1),
            Value::Int(5),
        ],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn test_all_comparison_operators() {
    // Test all comparison operators in one module
    let source = r#"
        let eq(a:int, b:int): bool = { a == b }
        let ne(a:int, b:int): bool = { a != b }
        let lt(a:int, b:int): bool = { a < b }
        let le(a:int, b:int): bool = { a <= b }
        let gt(a:int, b:int): bool = { a > b }
        let ge(a:int, b:int): bool = { a >= b }
    "#;

    // Test with 5 and 3
    assert_eq!(
        execute_function(source, "eq", vec![Value::Int(5), Value::Int(3)]).unwrap(),
        Value::Boolean(false)
    );
    assert_eq!(
        execute_function(source, "ne", vec![Value::Int(5), Value::Int(3)]).unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        execute_function(source, "lt", vec![Value::Int(5), Value::Int(3)]).unwrap(),
        Value::Boolean(false)
    );
    assert_eq!(
        execute_function(source, "le", vec![Value::Int(5), Value::Int(3)]).unwrap(),
        Value::Boolean(false)
    );
    assert_eq!(
        execute_function(source, "gt", vec![Value::Int(5), Value::Int(3)]).unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        execute_function(source, "ge", vec![Value::Int(5), Value::Int(3)]).unwrap(),
        Value::Boolean(true)
    );

    // Test with equal values
    assert_eq!(
        execute_function(source, "eq", vec![Value::Int(7), Value::Int(7)]).unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        execute_function(source, "le", vec![Value::Int(7), Value::Int(7)]).unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        execute_function(source, "ge", vec![Value::Int(7), Value::Int(7)]).unwrap(),
        Value::Boolean(true)
    );
}

// ============================================================================
// Short-Circuit Evaluation Tests (using side effects to prove evaluation order)
// ============================================================================

#[test]
fn test_and_short_circuit_avoids_division_by_zero() {
    // If && short-circuits, the division by zero should never happen
    let source = r#"
        let safe_divide(numerator:int, denominator:int): bool = {
            denominator != 0 && (numerator / denominator) > 0
        }
    "#;

    // With denominator = 0, && should short-circuit and NOT evaluate the division
    let result = execute_function(source, "safe_divide", vec![Value::Int(10), Value::Int(0)]);
    assert!(
        result.is_ok(),
        "Short-circuit should prevent division by zero, got error: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap(), Value::Boolean(false));
}

#[test]
fn test_or_short_circuit_avoids_division_by_zero() {
    // If || short-circuits, the division by zero should never happen
    let source = r#"
        let short_or(x:int, denominator:int): bool = {
            x > 5 || (10 / denominator) > 0
        }
    "#;

    // With x > 5 being true, || should short-circuit and NOT evaluate the division
    let result = execute_function(source, "short_or", vec![Value::Int(10), Value::Int(0)]);
    assert!(
        result.is_ok(),
        "Short-circuit should prevent division by zero, got error: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap(), Value::Boolean(true));
}

#[test]
fn test_and_evaluates_both_when_first_is_true() {
    // When first operand is true, && SHOULD evaluate the second
    let source = r#"
        let check(x:int, denominator:int): bool = {
            x > 0 && (10 / denominator) > 0
        }
    "#;

    // With x > 0 being true and denominator = 0, division by zero should occur
    let result = execute_function(source, "check", vec![Value::Int(5), Value::Int(0)]);
    assert!(
        result.is_err(),
        "Should have evaluated RHS and gotten division by zero"
    );
}

#[test]
fn test_or_evaluates_both_when_first_is_false() {
    // When first operand is false, || SHOULD evaluate the second
    let source = r#"
        let check(x:int, denominator:int): bool = {
            x > 100 || (10 / denominator) > 0
        }
    "#;

    // With x > 100 being false and denominator = 0, division by zero should occur
    let result = execute_function(source, "check", vec![Value::Int(5), Value::Int(0)]);
    assert!(
        result.is_err(),
        "Should have evaluated RHS and gotten division by zero"
    );
}

#[test]
fn test_nested_short_circuit() {
    // Test nested short-circuit: (false && x) || (true && y)
    let source = r#"
        let nested(a:int, b:int, c:int): bool = {
            (a > 10 && (1 / 0) > 0) || (b < 5 && c > 0)
        }
    "#;

    // a > 10 is false, so inner && short-circuits (no div by zero)
    // b < 5 is true, c > 0 is true, so result is true
    let result = execute_function(
        source,
        "nested",
        vec![Value::Int(5), Value::Int(3), Value::Int(10)],
    );
    assert!(
        result.is_ok(),
        "Short-circuit should prevent division by zero, got: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap(), Value::Boolean(true));
}
